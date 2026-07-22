use std::{
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    process::{ChildStdout, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::{
    domain::{can_transition_runtime, RuntimeState, RuntimeStatus},
    protocol::{
        health_request, parse_health_response, parse_runtime_output, shutdown_request,
        RuntimeOutput, MAX_MESSAGE_BYTES, PROTOCOL_VERSION,
    },
};

const HANDSHAKE_ID: &str = "phase1-health";

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeNotice {
    Output(RuntimeOutput),
    Disconnected { detail_code: &'static str },
}

enum RuntimeCommand {
    Send(String),
    Stop,
}

enum ReaderItem {
    Line(String),
    Invalid,
    Closed,
}

#[derive(Clone)]
pub struct RuntimeController {
    status: Arc<Mutex<RuntimeStatus>>,
    stop: Arc<AtomicBool>,
    worker: Arc<Mutex<Option<JoinHandle<()>>>>,
    command_sender: Arc<Mutex<Option<mpsc::Sender<RuntimeCommand>>>>,
    subscribers: Arc<Mutex<Vec<mpsc::Sender<RuntimeNotice>>>>,
    source_root: PathBuf,
}

impl RuntimeController {
    pub fn new(source_root: PathBuf, safe_mode: bool) -> Self {
        let status = if safe_mode {
            RuntimeStatus {
                state: RuntimeState::SafeMode,
                protocol_version: None,
                detail_code: "safe_mode_active",
            }
        } else {
            RuntimeStatus::stopped()
        };
        Self {
            status: Arc::new(Mutex::new(status)),
            stop: Arc::new(AtomicBool::new(false)),
            worker: Arc::new(Mutex::new(None)),
            command_sender: Arc::new(Mutex::new(None)),
            subscribers: Arc::new(Mutex::new(Vec::new())),
            source_root,
        }
    }

    pub fn snapshot(&self) -> RuntimeStatus {
        lock(&self.status).clone()
    }

    pub fn subscribe(&self) -> mpsc::Receiver<RuntimeNotice> {
        let (sender, receiver) = mpsc::channel();
        lock(&self.subscribers).push(sender);
        receiver
    }

    pub fn send(&self, message: String) -> Result<(), &'static str> {
        if message.len() > MAX_MESSAGE_BYTES {
            return Err("protocol_encoding_failed");
        }
        let sender = lock(&self.command_sender)
            .clone()
            .ok_or("runtime_unavailable")?;
        sender
            .send(RuntimeCommand::Send(message))
            .map_err(|_| "runtime_unavailable")
    }

    pub fn start(&self) {
        let mut worker = lock(&self.worker);
        if worker.as_ref().is_some_and(|handle| !handle.is_finished()) {
            return;
        }
        if let Some(previous) = worker.take() {
            let _ = previous.join();
        }

        self.stop.store(false, Ordering::SeqCst);
        set_status(
            &self.status,
            RuntimeState::Starting,
            None,
            "runtime_starting",
        );
        let (command_sender, command_receiver) = mpsc::channel();
        *lock(&self.command_sender) = Some(command_sender);
        let status = Arc::clone(&self.status);
        let stop = Arc::clone(&self.stop);
        let source_root = self.source_root.clone();
        let subscribers = Arc::clone(&self.subscribers);
        let stored_sender = Arc::clone(&self.command_sender);
        *worker = Some(thread::spawn(move || {
            run_runtime_process(status, stop, source_root, command_receiver, subscribers);
            *lock(&stored_sender) = None;
        }));
    }

    pub fn enter_safe_mode(&self) {
        self.stop_and_join();
        set_status(
            &self.status,
            RuntimeState::SafeMode,
            None,
            "safe_mode_active",
        );
    }

    pub fn leave_safe_mode(&self) {
        set_status(&self.status, RuntimeState::Stopped, None, "runtime_stopped");
        self.start();
    }

    pub fn shutdown(&self) {
        self.stop_and_join();
    }

    fn stop_and_join(&self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(sender) = lock(&self.command_sender).clone() {
            let _ = sender.send(RuntimeCommand::Stop);
        }
        if let Some(handle) = lock(&self.worker).take() {
            let _ = handle.join();
        }
        *lock(&self.command_sender) = None;
    }
}

fn run_runtime_process(
    status: Arc<Mutex<RuntimeStatus>>,
    stop: Arc<AtomicBool>,
    source_root: PathBuf,
    command_receiver: mpsc::Receiver<RuntimeCommand>,
    subscribers: Arc<Mutex<Vec<mpsc::Sender<RuntimeNotice>>>>,
) {
    let mut command = Command::new("python");
    let inherited_environment = ["PATH", "PATHEXT", "SYSTEMROOT", "WINDIR"]
        .into_iter()
        .filter_map(|key| std::env::var_os(key).map(|value| (key, value)))
        .collect::<Vec<_>>();
    command
        .env_clear()
        .envs(inherited_environment)
        .arg("-m")
        .arg("aip_runtime")
        .arg("--stdio")
        .env("PYTHONPATH", source_root)
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUNBUFFERED", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(target_os = "windows")]
    command.creation_flags(0x0800_0000);

    let Ok(mut child) = command.spawn() else {
        unavailable(&status, &subscribers, "python_unavailable");
        return;
    };
    let Some(mut stdin) = child.stdin.take() else {
        let _ = child.kill();
        unavailable(&status, &subscribers, "runtime_stdio_unavailable");
        return;
    };
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        unavailable(&status, &subscribers, "runtime_stdio_unavailable");
        return;
    };

    let (line_sender, line_receiver) = mpsc::channel();
    let reader = thread::spawn(move || read_runtime_lines(stdout, line_sender));
    let Ok(request) = health_request(HANDSHAKE_ID) else {
        let _ = child.kill();
        unavailable(&status, &subscribers, "protocol_encoding_failed");
        let _ = reader.join();
        return;
    };
    if write_message(&mut stdin, &request).is_err() {
        let _ = child.kill();
        unavailable(&status, &subscribers, "runtime_handshake_failed");
        let _ = reader.join();
        return;
    }

    let deadline = Instant::now() + Duration::from_secs(3);
    let handshake_valid = loop {
        if stop.load(Ordering::SeqCst) || Instant::now() >= deadline {
            break false;
        }
        match line_receiver.recv_timeout(Duration::from_millis(75)) {
            Ok(ReaderItem::Line(line)) => break parse_health_response(&line, HANDSHAKE_ID).is_ok(),
            Ok(ReaderItem::Invalid | ReaderItem::Closed)
            | Err(mpsc::RecvTimeoutError::Disconnected) => {
                break false;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
        }
    };
    if !handshake_valid {
        let _ = child.kill();
        let _ = child.wait();
        let _ = reader.join();
        unavailable(&status, &subscribers, "runtime_handshake_failed");
        return;
    }

    set_status(
        &status,
        RuntimeState::Ready,
        Some(PROTOCOL_VERSION),
        "runtime_ready",
    );
    broadcast(
        &subscribers,
        RuntimeNotice::Output(RuntimeOutput::HealthReady {
            id: HANDSHAKE_ID.into(),
        }),
    );

    loop {
        while let Ok(command) = command_receiver.try_recv() {
            match command {
                RuntimeCommand::Send(message) => {
                    if write_message(&mut stdin, &message).is_err() {
                        return crashed(
                            &mut child,
                            reader,
                            &status,
                            &subscribers,
                            "runtime_write_failed",
                        );
                    }
                }
                RuntimeCommand::Stop => {
                    return stop_child(&mut child, &mut stdin, reader, &status);
                }
            }
        }
        if stop.load(Ordering::SeqCst) {
            return stop_child(&mut child, &mut stdin, reader, &status);
        }
        match line_receiver.recv_timeout(Duration::from_millis(40)) {
            Ok(ReaderItem::Line(line)) => match parse_runtime_output(&line) {
                Ok(output) => broadcast(&subscribers, RuntimeNotice::Output(output)),
                Err(()) => {
                    return crashed(
                        &mut child,
                        reader,
                        &status,
                        &subscribers,
                        "runtime_protocol_invalid",
                    );
                }
            },
            Ok(ReaderItem::Invalid) => {
                return crashed(
                    &mut child,
                    reader,
                    &status,
                    &subscribers,
                    "runtime_protocol_invalid",
                );
            }
            Ok(ReaderItem::Closed) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                return crashed(
                    &mut child,
                    reader,
                    &status,
                    &subscribers,
                    "runtime_process_ended",
                );
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if child.try_wait().ok().flatten().is_some() {
            let _ = reader.join();
            set_status(
                &status,
                RuntimeState::Crashed,
                None,
                "runtime_process_ended",
            );
            broadcast(
                &subscribers,
                RuntimeNotice::Disconnected {
                    detail_code: "runtime_process_ended",
                },
            );
            return;
        }
    }
}

fn read_runtime_lines(stdout: ChildStdout, sender: mpsc::Sender<ReaderItem>) {
    let mut reader = BufReader::new(stdout);
    loop {
        let mut raw = Vec::new();
        let result = reader
            .by_ref()
            .take((MAX_MESSAGE_BYTES + 2) as u64)
            .read_until(b'\n', &mut raw);
        match result {
            Ok(0) => {
                let _ = sender.send(ReaderItem::Closed);
                return;
            }
            Ok(_) if raw.len() > MAX_MESSAGE_BYTES + 1 || !raw.ends_with(b"\n") => {
                let _ = sender.send(ReaderItem::Invalid);
                return;
            }
            Ok(_) => {
                raw.pop();
                if raw.last() == Some(&b'\r') {
                    raw.pop();
                }
                let Ok(line) = String::from_utf8(raw) else {
                    let _ = sender.send(ReaderItem::Invalid);
                    return;
                };
                if sender.send(ReaderItem::Line(line)).is_err() {
                    return;
                }
            }
            Err(_) => {
                let _ = sender.send(ReaderItem::Closed);
                return;
            }
        }
    }
}

fn write_message(stdin: &mut impl Write, message: &str) -> std::io::Result<()> {
    writeln!(stdin, "{message}")?;
    stdin.flush()
}

fn stop_child(
    child: &mut std::process::Child,
    stdin: &mut impl Write,
    reader: JoinHandle<()>,
    status: &Arc<Mutex<RuntimeStatus>>,
) {
    if let Ok(request) = shutdown_request("phase1-shutdown") {
        let _ = write_message(stdin, &request);
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if child.try_wait().ok().flatten().is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(40));
    }
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
    let _ = reader.join();
    set_status(status, RuntimeState::Stopped, None, "runtime_stopped");
}

fn crashed(
    child: &mut std::process::Child,
    reader: JoinHandle<()>,
    status: &Arc<Mutex<RuntimeStatus>>,
    subscribers: &Arc<Mutex<Vec<mpsc::Sender<RuntimeNotice>>>>,
    detail_code: &'static str,
) {
    let _ = child.kill();
    let _ = child.wait();
    let _ = reader.join();
    set_status(status, RuntimeState::Crashed, None, detail_code);
    broadcast(subscribers, RuntimeNotice::Disconnected { detail_code });
}

fn unavailable(
    status: &Arc<Mutex<RuntimeStatus>>,
    subscribers: &Arc<Mutex<Vec<mpsc::Sender<RuntimeNotice>>>>,
    detail_code: &'static str,
) {
    set_status(status, RuntimeState::Unavailable, None, detail_code);
    broadcast(subscribers, RuntimeNotice::Disconnected { detail_code });
}

fn broadcast(subscribers: &Arc<Mutex<Vec<mpsc::Sender<RuntimeNotice>>>>, notice: RuntimeNotice) {
    lock(subscribers).retain(|subscriber| subscriber.send(notice.clone()).is_ok());
}

fn set_status(
    status: &Arc<Mutex<RuntimeStatus>>,
    state: RuntimeState,
    protocol_version: Option<u32>,
    detail_code: &'static str,
) {
    let mut current = lock(status);
    if current.state == state || can_transition_runtime(current.state, state) {
        *current = RuntimeStatus {
            state,
            protocol_version,
            detail_code,
        };
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::domain::RuntimeState;

    use super::RuntimeController;

    #[test]
    fn safe_mode_never_starts_a_runtime() {
        let controller = RuntimeController::new(PathBuf::from("unused"), true);
        assert_eq!(controller.snapshot().state, RuntimeState::SafeMode);
        controller.shutdown();
        assert_eq!(controller.snapshot().state, RuntimeState::SafeMode);
    }

    #[test]
    fn unavailable_session_rejects_commands() {
        let controller = RuntimeController::new(PathBuf::from("unused"), false);
        assert_eq!(controller.send("{}".into()), Err("runtime_unavailable"));
    }
}
