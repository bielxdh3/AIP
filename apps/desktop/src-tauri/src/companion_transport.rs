//! Bounded, opt-in local transport core. It deliberately has no database or UI authority.
#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{BufRead, BufReader, Write},
    net::{IpAddr, SocketAddr, TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

pub const PROTOCOL: &str = "aip-companion-v1";
pub const MAX_FRAME: usize = 16_384;
pub const MAX_PAYLOAD: usize = 8_192;
const MAX_FIELD: usize = 512;

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("invalid bind address")]
    BindPolicy,
    #[error("invalid frame")]
    Frame,
    #[error("oversized frame")]
    Oversized,
    #[error("protocol version mismatch")]
    Version,
    #[error("authentication failed")]
    Auth,
    #[error("replay rejected")]
    Replay,
    #[error("revoked")]
    Revoked,
    #[error("io error")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WireFrame {
    pub protocol: String,
    pub kind: String,
    pub client_id: String,
    pub session_id: Option<String>,
    pub nonce: String,
    pub counter: u64,
    pub payload: String,
    pub mac: String,
}

fn field_ok(v: &str, max: usize) -> bool {
    !v.is_empty() && v.len() <= max && v.is_char_boundary(v.len())
}
pub fn canonical(f: &WireFrame) -> Vec<u8> {
    [
        f.protocol.as_str(),
        f.kind.as_str(),
        f.client_id.as_str(),
        f.session_id.as_deref().unwrap_or(""),
        f.nonce.as_str(),
        &f.counter.to_string(),
        f.payload.as_str(),
    ]
    .join("\u{1f}")
    .into_bytes()
}
fn hmac(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut k = [0u8; 64];
    if key.len() > 64 {
        k[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut i = [0x36u8; 64];
    let mut o = [0x5cu8; 64];
    for n in 0..64 {
        i[n] ^= k[n];
        o[n] ^= k[n];
    }
    let mut a = Sha256::new();
    a.update(i);
    a.update(data);
    let inner = a.finalize();
    let mut b = Sha256::new();
    b.update(o);
    b.update(inner);
    b.finalize().into()
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
fn valid_hex(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn constant_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |x, (l, r)| x | (l ^ r)) == 0
}

pub fn encode(f: &WireFrame) -> Result<Vec<u8>, TransportError> {
    validate(f)?;
    let mut out = serde_json::to_vec(f).map_err(|_| TransportError::Frame)?;
    out.push(b'\n');
    if out.len() > MAX_FRAME {
        return Err(TransportError::Oversized);
    }
    Ok(out)
}
pub fn decode(line: &[u8]) -> Result<WireFrame, TransportError> {
    if line.len() > MAX_FRAME || !line.ends_with(b"\n") {
        return Err(TransportError::Oversized);
    }
    let f: WireFrame =
        serde_json::from_slice(&line[..line.len() - 1]).map_err(|_| TransportError::Frame)?;
    validate(&f)?;
    Ok(f)
}
fn validate(f: &WireFrame) -> Result<(), TransportError> {
    if f.protocol != PROTOCOL {
        return Err(TransportError::Version);
    }
    if !field_ok(&f.kind, MAX_FIELD)
        || !field_ok(&f.client_id, MAX_FIELD)
        || !field_ok(&f.nonce, 128)
        || f.payload.len() > MAX_PAYLOAD
        || !valid_hex(&f.mac)
    {
        return Err(TransportError::Frame);
    }
    if let Some(s) = &f.session_id {
        if s.len() > MAX_FIELD {
            return Err(TransportError::Frame);
        }
    }
    Ok(())
}

#[derive(Clone)]
pub struct Session {
    key: Vec<u8>,
    counter: u64,
    nonce: Option<String>,
    revoked: bool,
}
impl Session {
    pub fn new(key: &[u8]) -> Self {
        Self {
            key: key.to_vec(),
            counter: 0,
            nonce: None,
            revoked: false,
        }
    }
    pub fn revoke(&mut self) {
        self.revoked = true
    }
    pub fn rotate(&mut self, key: &[u8]) {
        self.key = key.to_vec();
        self.counter = 0;
        self.nonce = None;
        self.revoked = false
    }
    pub fn challenge(&self, value: &str) -> String {
        hex(&hmac(&self.key, value.as_bytes()))
    }
    pub fn authenticate(&mut self, f: &WireFrame) -> Result<(), TransportError> {
        if self.revoked {
            return Err(TransportError::Revoked);
        }
        if f.counter <= self.counter || self.nonce.as_deref() == Some(&f.nonce) {
            return Err(TransportError::Replay);
        }
        let expected = hmac(&self.key, &canonical(f));
        let supplied = hex_decode(&f.mac)?;
        if !constant_eq(&expected, &supplied) {
            return Err(TransportError::Auth);
        }
        self.counter = f.counter;
        self.nonce = Some(f.nonce.clone());
        Ok(())
    }
}
fn hex_decode(s: &str) -> Result<Vec<u8>, TransportError> {
    if !valid_hex(s) {
        return Err(TransportError::Auth);
    }
    Ok((0..32)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap())
        .collect())
}

pub fn validate_bind(ip: IpAddr, private_confirmed: bool) -> Result<(), TransportError> {
    match ip {
        IpAddr::V4(v) if v == std::net::Ipv4Addr::LOCALHOST => Ok(()),
        IpAddr::V4(v) if private_confirmed && (v.is_private()) => Ok(()),
        _ => Err(TransportError::BindPolicy),
    }
}
pub type Handler = Arc<dyn Fn(WireFrame) -> Option<WireFrame> + Send + Sync + 'static>;
pub struct TransportHandle {
    stop: Arc<Mutex<bool>>,
    join: Option<thread::JoinHandle<()>>,
    pub addr: SocketAddr,
}
impl TransportHandle {
    pub fn stop(mut self) {
        *self.stop.lock().unwrap() = true;
        let _ = TcpStream::connect(self.addr);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}
pub fn start(
    addr: SocketAddr,
    private_confirmed: bool,
    key: Vec<u8>,
    handler: Handler,
) -> Result<TransportHandle, TransportError> {
    validate_bind(addr.ip(), private_confirmed)?;
    let listener = TcpListener::bind(addr)?;
    listener.set_nonblocking(true)?;
    let actual = listener.local_addr()?;
    let stop = Arc::new(Mutex::new(false));
    let flag = Arc::clone(&stop);
    let join = thread::spawn(move || {
        while !*flag.lock().unwrap() {
            match listener.accept() {
                Ok((stream, _)) => {
                    let h = Arc::clone(&handler);
                    let k = key.clone();
                    thread::spawn(move || {
                        let _ = serve(stream, k, h);
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10))
                }
                Err(_) => break,
            }
        }
    });
    Ok(TransportHandle {
        stop,
        join: Some(join),
        addr: actual,
    })
}
fn serve(mut stream: TcpStream, key: Vec<u8>, handler: Handler) -> Result<(), TransportError> {
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    let mut line = Vec::new();
    BufReader::new(stream.try_clone()?).read_until(b'\n', &mut line)?;
    let mut session = Session::new(&key);
    let f = decode(&line)?;
    session.authenticate(&f)?;
    if let Some(response) = handler(f) {
        stream.write_all(&encode(&response)?)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    fn frame(mac: String) -> WireFrame {
        WireFrame {
            protocol: PROTOCOL.into(),
            kind: "hello".into(),
            client_id: "android".into(),
            session_id: None,
            nonce: "abc".into(),
            counter: 1,
            payload: "{}".into(),
            mac,
        }
    }
    #[test]
    fn canonical_and_json() {
        let k = b"key";
        let mut f = frame("".into());
        f.mac = hex(&hmac(k, &canonical(&f)));
        assert_eq!(decode(&encode(&f).unwrap()).unwrap(), f);
        let mut s = Session::new(k);
        s.authenticate(&f).unwrap();
    }
    #[test]
    fn auth_replay_rotation() {
        let k = b"key";
        let mut f = frame("".into());
        f.mac = hex(&hmac(k, &canonical(&f)));
        let mut s = Session::new(k);
        s.authenticate(&f).unwrap();
        assert!(matches!(s.authenticate(&f), Err(TransportError::Replay)));
        s.rotate(b"new");
        s.revoke();
        assert!(matches!(s.authenticate(&f), Err(TransportError::Revoked)));
    }
    #[test]
    fn bounds_and_policy() {
        assert!(validate_bind(IpAddr::V4(Ipv4Addr::LOCALHOST), false).is_ok());
        assert!(validate_bind(IpAddr::V4(Ipv4Addr::UNSPECIFIED), true).is_err());
        assert!(decode(b"{}\n").is_err());
        let mut f = frame("0".repeat(64));
        f.payload = "x".repeat(MAX_PAYLOAD + 1);
        assert!(encode(&f).is_err());
    }
    #[test]
    fn loopback_exchange() {
        let key = b"key".to_vec();
        let h: Handler = Arc::new(|f| {
            Some(WireFrame {
                kind: "ok".into(),
                ..f
            })
        });
        let t = start("127.0.0.1:0".parse().unwrap(), false, key.clone(), h).unwrap();
        let mut s = TcpStream::connect(t.addr).unwrap();
        let mut f = frame("".into());
        f.mac = hex(&hmac(&key, &canonical(&f)));
        s.write_all(&encode(&f).unwrap()).unwrap();
        let mut out = Vec::new();
        BufReader::new(s).read_until(b'\n', &mut out).unwrap();
        assert_eq!(decode(&out).unwrap().kind, "ok");
        t.stop();
    }
}
