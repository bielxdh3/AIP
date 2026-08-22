use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use serde_json::json;
use uuid::Uuid;

use crate::{
    gateway::{
        GatewaySessionProof, GATEWAY_FIXTURE_AGENT_ID, GATEWAY_FIXTURE_APP_VERSION,
        GATEWAY_FIXTURE_AUTH_PROOF_METADATA, GATEWAY_FIXTURE_CLIENT_ID,
        GATEWAY_FIXTURE_EXTERNAL_ACCOUNT_METADATA, GATEWAY_FIXTURE_RECOVERY_TARGET,
        GATEWAY_FIXTURE_TRANSFER_INTEGRITY_HASH,
    },
    gateway_transport::{decode, encode, sign_frame, start_secure, Session, WireFrame, PROTOCOL},
    gateway_transport_handler,
};

fn temp_db() -> PathBuf {
    std::env::temp_dir().join(format!("aip-gateway-roundtrip-{}", Uuid::now_v7()))
}
fn frame(kind: &str, counter: u64, payload: String) -> WireFrame {
    WireFrame {
        protocol: PROTOCOL.into(),
        kind: kind.into(),
        client_id: GATEWAY_FIXTURE_CLIENT_ID.into(),
        session_id: Some("gateway-session".into()),
        nonce: format!("gateway-nonce-{counter}"),
        counter,
        payload,
        mac: String::new(),
    }
}
fn signed(key: &[u8], mut f: WireFrame) -> WireFrame {
    f.mac = sign_frame(key, &f);
    f
}
fn exchange(addr: std::net::SocketAddr, f: &[u8]) -> Vec<u8> {
    let mut s = TcpStream::connect(addr).unwrap();
    s.set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .unwrap();
    s.write_all(f).unwrap();
    let mut out = Vec::new();
    let _ = BufReader::new(s).read_until(b'\n', &mut out);
    out
}

#[test]
fn gateway_authority_round_trip_and_fail_closed() {
    let path = temp_db();
    let database = crate::database::Database::initialize(&path).unwrap();
    let safe = Arc::new(AtomicBool::new(false));
    let key = b"gateway-integration-key".to_vec();
    let handler = gateway_transport_handler(database.clone(), Arc::clone(&safe));
    let mut transport =
        start_secure("127.0.0.1:0".parse().unwrap(), false, key.clone(), handler).unwrap();
    let mut verifier = Session::new(&key);
    let mut counter = 1;
    let mut call = |kind: &str, payload: serde_json::Value| -> WireFrame {
        let request = signed(
            &key,
            frame(kind, counter, serde_json::to_string(&payload).unwrap()),
        );
        let response = decode(&exchange(transport.addr, &encode(&request).unwrap())).unwrap();
        verifier.authenticate(&request).unwrap();
        verifier.authenticate(&response).unwrap();
        counter += 2;
        response
    };
    assert_eq!(
        call("protocol", json!({"agentId": GATEWAY_FIXTURE_AGENT_ID})).kind,
        "protocol_result"
    );
    assert_eq!(
        call("accounts", json!({"agentId": GATEWAY_FIXTURE_AGENT_ID})).kind,
        "accounts_result"
    );
    let prepared_response = call(
        "transfer_prepare",
        json!({"agentId":GATEWAY_FIXTURE_AGENT_ID,"ownerUserId":crate::database::OWNER_ID,"destinationAccountMetadata":GATEWAY_FIXTURE_EXTERNAL_ACCOUNT_METADATA,"integrityHash":GATEWAY_FIXTURE_TRANSFER_INTEGRITY_HASH,"idempotencyKey":"gateway-prepare","temporaryChat":false}),
    );
    assert_eq!(
        prepared_response.kind, "transfer_prepare_result",
        "{}",
        prepared_response.payload
    );
    let prepared: crate::gateway::GatewayTransfer =
        serde_json::from_str(&prepared_response.payload).unwrap();
    let approved: crate::gateway::GatewayTransfer = serde_json::from_str(&call("transfer_approve", json!({"agentId":GATEWAY_FIXTURE_AGENT_ID,"ownerUserId":crate::database::OWNER_ID,"transferId":prepared.id,"approved":true,"idempotencyKey":"gateway-approve","temporaryChat":false})).payload).unwrap();
    assert_eq!(
        approved.status,
        crate::gateway::GatewayTransferStatus::Approved
    );
    let session: crate::gateway::GatewaySession = serde_json::from_str(&call("session_connect", json!({"agentId":GATEWAY_FIXTURE_AGENT_ID,"ownerUserId":crate::database::OWNER_ID,"transferId":approved.id,"clientId":GATEWAY_FIXTURE_CLIENT_ID,"appVersion":GATEWAY_FIXTURE_APP_VERSION,"protocolVersion":1,"authProofMetadata":GATEWAY_FIXTURE_AUTH_PROOF_METADATA,"messageNonceMetadata":"gateway-session-message","replayCounter":1,"idempotencyKey":"gateway-session","temporaryChat":false})).payload).unwrap();
    let mut proof = GatewaySessionProof {
        session_id: session.id.clone(),
        transfer_id: approved.id.clone(),
        client_id: GATEWAY_FIXTURE_CLIENT_ID.into(),
        session_nonce_metadata: session.session_nonce_metadata.clone(),
        auth_proof_metadata: GATEWAY_FIXTURE_AUTH_PROOF_METADATA.into(),
        app_version: GATEWAY_FIXTURE_APP_VERSION.into(),
        protocol_version: 1,
        message_nonce_metadata: "gateway-session-message".into(),
        replay_counter: 1,
    };
    proof.replay_counter = 2;
    proof.message_nonce_metadata = "gateway-recovery-message".into();
    let recovery_response = call(
        "recovery_request",
        json!({"agentId":GATEWAY_FIXTURE_AGENT_ID,"ownerUserId":crate::database::OWNER_ID,"proof":proof.clone(),"recoveryKind":"mobile_administrative","targetMetadata":GATEWAY_FIXTURE_RECOVERY_TARGET,"idempotencyKey":"gateway-recovery","temporaryChat":false}),
    );
    assert_eq!(
        recovery_response.kind, "recovery_request_result",
        "{}",
        recovery_response.payload
    );
    let recovery: crate::gateway::GatewayRecovery =
        serde_json::from_str(&recovery_response.payload).unwrap();
    proof.replay_counter = 3;
    proof.message_nonce_metadata = "gateway-recovery-approve-message".into();
    let approved_recovery: crate::gateway::GatewayRecovery = serde_json::from_str(&call("recovery_approve", json!({"agentId":GATEWAY_FIXTURE_AGENT_ID,"ownerUserId":crate::database::OWNER_ID,"proof":proof,"recoveryId":recovery.id,"approved":true,"idempotencyKey":"gateway-recovery-approve","temporaryChat":false})).payload).unwrap();
    assert_eq!(
        approved_recovery.status,
        crate::gateway::GatewayRecoveryStatus::Approved
    );
    call(
        "session_revoke",
        json!({"agentId":GATEWAY_FIXTURE_AGENT_ID,"ownerUserId":crate::database::OWNER_ID,"sessionId":session.id,"reason":"fixture","idempotencyKey":"gateway-session-revoke","temporaryChat":false}),
    );
    proof.replay_counter = 4;
    proof.message_nonce_metadata = "gateway-revoked-session".into();
    let revoked_session = call(
        "session_reconnect",
        json!({"agentId":GATEWAY_FIXTURE_AGENT_ID,"ownerUserId":crate::database::OWNER_ID,"proof":proof,"idempotencyKey":"gateway-reconnect-revoked","temporaryChat":false}),
    );
    assert_eq!(revoked_session.kind, "error");
    call(
        "transfer_revoke",
        json!({"agentId":GATEWAY_FIXTURE_AGENT_ID,"ownerUserId":crate::database::OWNER_ID,"transferId":approved.id,"reason":"fixture","idempotencyKey":"gateway-transfer-revoke","temporaryChat":false}),
    );
    let bad = signed(
        &key,
        frame(
            "protocol",
            counter,
            json!({"agentId":GATEWAY_FIXTURE_AGENT_ID}).to_string(),
        ),
    );
    let mut wrong = bad.clone();
    wrong.mac = "0".repeat(64);
    assert!(exchange(transport.addr, &encode(&wrong).unwrap()).is_empty());
    assert!(!exchange(transport.addr, &encode(&bad).unwrap()).is_empty());
    assert!(exchange(transport.addr, &encode(&bad).unwrap()).is_empty());
    let unknown = signed(
        &key,
        frame(
            "protocol",
            counter + 1,
            json!({"agentId":GATEWAY_FIXTURE_AGENT_ID,"unknown":true}).to_string(),
        ),
    );
    let mut raw = serde_json::to_vec(&unknown).unwrap();
    raw.pop();
    raw.extend_from_slice(b",\"unknown\":true}\n");
    assert!(exchange(transport.addr, &raw).is_empty());
    let temporary = signed(
        &key,
        frame(
            "transfer_prepare",
            counter + 2,
            json!({"agentId":GATEWAY_FIXTURE_AGENT_ID,"ownerUserId":crate::database::OWNER_ID,"destinationAccountMetadata":GATEWAY_FIXTURE_EXTERNAL_ACCOUNT_METADATA,"integrityHash":GATEWAY_FIXTURE_TRANSFER_INTEGRITY_HASH,"idempotencyKey":"gateway-temporary","temporaryChat":true}).to_string(),
        ),
    );
    assert_eq!(
        decode(&exchange(transport.addr, &encode(&temporary).unwrap()))
            .unwrap()
            .kind,
        "error"
    );
    safe.store(true, Ordering::Release);
    assert!(decode(&exchange(
        transport.addr,
        &encode(&signed(
            &key,
            frame(
                "protocol",
                counter + 4,
                json!({"agentId":GATEWAY_FIXTURE_AGENT_ID}).to_string()
            )
        ))
        .unwrap()
    ))
    .is_ok());
    let mut closed = TcpStream::connect(transport.addr).unwrap();
    transport.stop();
    let _ = closed.write_all(b"x\n");
    let mut one = [0u8; 1];
    assert!(closed.read(&mut one).unwrap_or(0) == 0);
    let _ = fs::remove_dir_all(path);
}
