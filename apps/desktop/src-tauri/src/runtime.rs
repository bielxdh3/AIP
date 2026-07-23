use std::{
    collections::VecDeque,
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    process::{ChildStderr, ChildStdout, Command, ExitStatus, Stdio},
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
const DIAGNOSTIC_PREFIX: &str = "AIP_RUNTIME_DIAGNOSTIC ";
const MAX_DIAGNOSTIC_LINE_BYTES: usize = 96;
const MAX_DIAGNOSTIC_CODES: usize = 16;

const PYTHON_DIAGNOSTIC_CODES: &[&str] = &[
    "ollama_cancel_close_failed",
    "ollama_stream_cancelled",
    "ollama_stream_failed",
    "runtime_diagnostic_rejected",
    "runtime_request_exception",
    "runtime_server_exception",
    "runtime_shutdown_requested",
    "runtime_stdin_eof",
    "runtime_stdout_write_failed",
    "runtime_worker_exception",
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RuntimeDiagnostics {
    pub last_lifecycle_code: Option<&'static str>,
    pub exit_code: Option<i32>,
    pub stderr_codes: VecDeque<String>,
}

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
    diagnostics: Arc<Mutex<RuntimeDiagnostics>>,
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
            diagnostics: Arc::new(Mutex::new(RuntimeDiagnostics::default())),
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

    #[cfg(test)]
    pub(crate) fn diagnostics(&self) -> RuntimeDiagnostics {
        lock(&self.diagnostics).clone()
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
        *lock(&self.diagnostics) = RuntimeDiagnostics::default();
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
        let diagnostics = Arc::clone(&self.diagnostics);
        let stored_sender = Arc::clone(&self.command_sender);
        *worker = Some(thread::spawn(move || {
            run_runtime_process(
                status,
                stop,
                source_root,
                command_receiver,
                subscribers,
                diagnostics,
            );
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
    diagnostics: Arc<Mutex<RuntimeDiagnostics>>,
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
        .stderr(Stdio::piped());
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
    let Some(stderr) = child.stderr.take() else {
        let _ = child.kill();
        let _ = child.wait();
        unavailable(&status, &subscribers, "runtime_stdio_unavailable");
        return;
    };

    let (line_sender, line_receiver) = mpsc::channel();
    let reader = thread::spawn(move || read_runtime_lines(stdout, line_sender));
    let diagnostic_state = Arc::clone(&diagnostics);
    let stderr_reader = thread::spawn(move || read_runtime_stderr(stderr, diagnostic_state));
    let Ok(request) = health_request(HANDSHAKE_ID) else {
        let _ = child.kill();
        unavailable(&status, &subscribers, "protocol_encoding_failed");
        let _ = reader.join();
        let _ = stderr_reader.join();
        return;
    };
    if write_message(&mut stdin, &request).is_err() {
        let _ = child.kill();
        unavailable(&status, &subscribers, "runtime_handshake_failed");
        let _ = reader.join();
        let _ = stderr_reader.join();
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
        let _ = stderr_reader.join();
        record_lifecycle(&diagnostics, "runtime_handshake_failed");
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
                            stderr_reader,
                            &status,
                            &subscribers,
                            &diagnostics,
                            "runtime_stdin_closed",
                        );
                    }
                }
                RuntimeCommand::Stop => {
                    return stop_child(
                        &mut child,
                        &mut stdin,
                        reader,
                        stderr_reader,
                        &status,
                        &diagnostics,
                    );
                }
            }
        }
        if stop.load(Ordering::SeqCst) {
            return stop_child(
                &mut child,
                &mut stdin,
                reader,
                stderr_reader,
                &status,
                &diagnostics,
            );
        }
        match line_receiver.recv_timeout(Duration::from_millis(40)) {
            Ok(ReaderItem::Line(line)) => match parse_runtime_output(&line) {
                Ok(output) => broadcast(&subscribers, RuntimeNotice::Output(output)),
                Err(()) => {
                    return crashed(
                        &mut child,
                        reader,
                        stderr_reader,
                        &status,
                        &subscribers,
                        &diagnostics,
                        "runtime_protocol_decode_failed",
                    );
                }
            },
            Ok(ReaderItem::Invalid) => {
                return crashed(
                    &mut child,
                    reader,
                    stderr_reader,
                    &status,
                    &subscribers,
                    &diagnostics,
                    "runtime_protocol_decode_failed",
                );
            }
            Ok(ReaderItem::Closed) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                let detail_code = if child_exited_within(&mut child, Duration::from_millis(200)) {
                    "runtime_process_exit_unexpected"
                } else {
                    "runtime_stdout_closed"
                };
                return crashed(
                    &mut child,
                    reader,
                    stderr_reader,
                    &status,
                    &subscribers,
                    &diagnostics,
                    detail_code,
                );
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if let Some(exit_status) = child.try_wait().ok().flatten() {
            let _ = reader.join();
            let _ = stderr_reader.join();
            record_exit(&diagnostics, Some(exit_status));
            record_lifecycle(&diagnostics, "runtime_process_exit_unexpected");
            report_crash_diagnostics(&diagnostics);
            set_status(
                &status,
                RuntimeState::Crashed,
                None,
                "runtime_process_exit_unexpected",
            );
            broadcast(
                &subscribers,
                RuntimeNotice::Disconnected {
                    detail_code: "runtime_process_exit_unexpected",
                },
            );
            return;
        }
    }
}

fn child_exited_within(child: &mut std::process::Child, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if child.try_wait().ok().flatten().is_some() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(10));
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

fn read_runtime_stderr(stderr: ChildStderr, diagnostics: Arc<Mutex<RuntimeDiagnostics>>) {
    let mut reader = BufReader::new(stderr);
    loop {
        let mut raw = Vec::new();
        let result = reader
            .by_ref()
            .take((MAX_DIAGNOSTIC_LINE_BYTES + 2) as u64)
            .read_until(b'\n', &mut raw);
        match result {
            Ok(0) => return,
            Ok(_) if raw.len() > MAX_DIAGNOSTIC_LINE_BYTES + 1 || !raw.ends_with(b"\n") => {
                record_stderr_code(&diagnostics, "runtime_diagnostic_rejected");
            }
            Ok(_) => {
                raw.pop();
                if raw.last() == Some(&b'\r') {
                    raw.pop();
                }
                let Some(code) = parse_diagnostic_line(&raw) else {
                    continue;
                };
                record_stderr_code(&diagnostics, code);
            }
            Err(_) => return,
        }
    }
}

fn parse_diagnostic_line(raw: &[u8]) -> Option<&'static str> {
    let line = std::str::from_utf8(raw).ok()?;
    let candidate = line.strip_prefix(DIAGNOSTIC_PREFIX)?;
    PYTHON_DIAGNOSTIC_CODES
        .iter()
        .copied()
        .find(|code| *code == candidate)
}

fn record_stderr_code(diagnostics: &Arc<Mutex<RuntimeDiagnostics>>, code: &str) {
    let mut current = lock(diagnostics);
    if current.stderr_codes.len() == MAX_DIAGNOSTIC_CODES {
        current.stderr_codes.pop_front();
    }
    current.stderr_codes.push_back(code.to_string());
}

fn record_lifecycle(diagnostics: &Arc<Mutex<RuntimeDiagnostics>>, code: &'static str) {
    lock(diagnostics).last_lifecycle_code = Some(code);
}

fn record_exit(diagnostics: &Arc<Mutex<RuntimeDiagnostics>>, exit_status: Option<ExitStatus>) {
    lock(diagnostics).exit_code = exit_status.and_then(|status| status.code());
}

fn report_crash_diagnostics(diagnostics: &Arc<Mutex<RuntimeDiagnostics>>) {
    let snapshot = lock(diagnostics).clone();
    let lifecycle = snapshot.last_lifecycle_code.unwrap_or("runtime_crashed");
    let exit = snapshot
        .exit_code
        .map_or_else(|| "none".to_string(), |code| code.to_string());
    let stderr = if snapshot.stderr_codes.is_empty() {
        "none".to_string()
    } else {
        snapshot
            .stderr_codes
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(",")
    };
    eprintln!("AIP_RUNTIME_EXIT lifecycle={lifecycle} exit={exit} stderr={stderr}");
}

fn write_message(stdin: &mut impl Write, message: &str) -> std::io::Result<()> {
    writeln!(stdin, "{message}")?;
    stdin.flush()
}

fn stop_child(
    child: &mut std::process::Child,
    stdin: &mut impl Write,
    reader: JoinHandle<()>,
    stderr_reader: JoinHandle<()>,
    status: &Arc<Mutex<RuntimeStatus>>,
    diagnostics: &Arc<Mutex<RuntimeDiagnostics>>,
) {
    record_lifecycle(diagnostics, "runtime_shutdown_requested");
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
    let exit_status = child.wait().ok();
    let _ = reader.join();
    let _ = stderr_reader.join();
    record_exit(diagnostics, exit_status);
    set_status(status, RuntimeState::Stopped, None, "runtime_stopped");
}

fn crashed(
    child: &mut std::process::Child,
    reader: JoinHandle<()>,
    stderr_reader: JoinHandle<()>,
    status: &Arc<Mutex<RuntimeStatus>>,
    subscribers: &Arc<Mutex<Vec<mpsc::Sender<RuntimeNotice>>>>,
    diagnostics: &Arc<Mutex<RuntimeDiagnostics>>,
    detail_code: &'static str,
) {
    record_lifecycle(diagnostics, detail_code);
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let exit_status = child.wait().ok();
    let _ = reader.join();
    let _ = stderr_reader.join();
    record_exit(diagnostics, exit_status);
    report_crash_diagnostics(diagnostics);
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
    use std::{
        collections::VecDeque,
        fs,
        path::{Path, PathBuf},
        sync::{mpsc, Arc, Mutex},
        thread,
        time::{Duration, Instant},
    };

    use crate::domain::RuntimeState;
    use crate::protocol::{
        cancellation_request, discovery_request, generation_request, health_request, PromptMessage,
        RuntimeOutput,
    };
    use uuid::Uuid;

    use super::{RuntimeController, RuntimeNotice};

    const FIXTURE_RUNTIME: &str = r#"
import json
import sys

active = None

def write(value):
    sys.stdout.write(json.dumps(value, separators=(",", ":")) + "\n")
    sys.stdout.flush()

def event(request, kind, **extra):
    value = {
        "protocolVersion": 1,
        "event": kind,
        "requestId": request["id"],
        "agentId": request["params"]["agentId"],
        "conversationId": request["params"]["conversationId"],
        "assistantMessageId": request["params"]["assistantMessageId"],
    }
    value.update(extra)
    write(value)

for raw in sys.stdin:
    request = json.loads(raw)
    method = request["method"]
    if method == "runtime.health":
        write({"protocolVersion": 1, "id": request["id"], "result": {"name": "aip-runtime", "status": "ready", "protocolVersion": 1}})
    elif method == "runtime.shutdown":
        write({"protocolVersion": 1, "id": request["id"], "result": {"status": "stopping"}})
        raise SystemExit(0)
    elif method == "provider.discover":
        sys.stderr.write("AIP_RUNTIME_DIAGNOSTIC runtime_server_exception\n")
        sys.stderr.flush()
        raise SystemExit(7)
    elif method == "generation.start":
        write({"protocolVersion": 1, "id": request["id"], "result": {"status": "accepted"}})
        event(request, "generation.started", sequence=0)
        model = request["params"]["model"]
        if model == "wait:latest":
            active = request
        elif model == "failure:latest":
            event(request, "generation.failed", sequence=0, errorCode="provider_stream_failed")
        else:
            event(request, "generation.chunk", sequence=1, content="Synthetic reply")
            event(request, "generation.complete", sequence=1)
    elif method == "generation.cancel":
        write({"protocolVersion": 1, "id": request["id"], "result": {"status": "cancelling"}})
        if active is not None and active["id"] == request["params"]["requestId"]:
            sys.stderr.write("AIP_RUNTIME_DIAGNOSTIC ollama_stream_cancelled\n")
            sys.stderr.flush()
            event(active, "generation.cancelled", sequence=0)
            active = None
"#;

    fn fixture_source_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("aip-runtime-fixture-{}", Uuid::now_v7()));
        let package = root.join("aip_runtime");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("__init__.py"), "").unwrap();
        fs::write(package.join("__main__.py"), FIXTURE_RUNTIME).unwrap();
        root
    }

    fn wait_for_state(controller: &RuntimeController, expected: RuntimeState) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if controller.snapshot().state == expected {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("runtime did not reach expected synthetic state");
    }

    fn wait_for_event(
        receiver: &mpsc::Receiver<RuntimeNotice>,
        request_id: &str,
        event_type: &str,
    ) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let Ok(notice) = receiver.recv_timeout(Duration::from_millis(100)) else {
                continue;
            };
            if let RuntimeNotice::Output(RuntimeOutput::Event(event)) = notice {
                if event.request_id.as_deref() == Some(request_id) && event.event_type == event_type
                {
                    return;
                }
            }
        }
        panic!("runtime did not emit expected synthetic event");
    }

    fn generation(id: &str, model: &str) -> String {
        generation_request(
            id,
            "agent",
            "conversation",
            &format!("assistant-{id}"),
            model,
            15,
            &[PromptMessage {
                role: "user",
                content: "Synthetic input".into(),
            }],
        )
        .unwrap()
    }

    fn cleanup(controller: RuntimeController, root: &Path) {
        controller.shutdown();
        let _ = fs::remove_dir_all(root);
    }

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

    #[test]
    fn persistent_child_survives_completion_cancellation_and_provider_failure() {
        let root = fixture_source_root();
        let controller = RuntimeController::new(root.clone(), false);
        let receiver = controller.subscribe();
        controller.start();
        wait_for_state(&controller, RuntimeState::Ready);

        controller
            .send(generation("complete", "complete:latest"))
            .unwrap();
        wait_for_event(&receiver, "complete", "generation.complete");
        assert_eq!(controller.snapshot().state, RuntimeState::Ready);

        controller
            .send(generation("cancelled", "wait:latest"))
            .unwrap();
        wait_for_event(&receiver, "cancelled", "generation.started");
        controller
            .send(cancellation_request("cancel-command", "cancelled").unwrap())
            .unwrap();
        wait_for_event(&receiver, "cancelled", "generation.cancelled");
        assert_eq!(controller.snapshot().state, RuntimeState::Ready);

        controller
            .send(generation("failed", "failure:latest"))
            .unwrap();
        wait_for_event(&receiver, "failed", "generation.failed");
        controller
            .send(health_request("health-after-failure").unwrap())
            .unwrap();
        assert_eq!(controller.snapshot().state, RuntimeState::Ready);
        assert!(controller
            .diagnostics()
            .stderr_codes
            .contains(&"ollama_stream_cancelled".to_string()));

        cleanup(controller, &root);
    }

    #[test]
    fn unexpected_exit_is_diagnosed_once_and_explicit_restart_recovers() {
        let root = fixture_source_root();
        let controller = RuntimeController::new(root.clone(), false);
        let receiver = controller.subscribe();
        controller.start();
        wait_for_state(&controller, RuntimeState::Ready);
        controller
            .send(discovery_request("crash").unwrap())
            .unwrap();
        wait_for_state(&controller, RuntimeState::Crashed);

        let diagnostics = controller.diagnostics();
        assert_eq!(
            diagnostics.last_lifecycle_code,
            Some("runtime_process_exit_unexpected")
        );
        assert_eq!(diagnostics.exit_code, Some(7));
        assert_eq!(
            diagnostics.stderr_codes,
            VecDeque::from(["runtime_server_exception".to_string()])
        );
        assert_eq!(
            receiver
                .try_iter()
                .filter(|notice| matches!(notice, RuntimeNotice::Disconnected { .. }))
                .count(),
            1
        );

        controller.start();
        wait_for_state(&controller, RuntimeState::Ready);
        assert!(controller
            .send(health_request("health-after-restart").unwrap())
            .is_ok());

        cleanup(controller, &root);
    }

    #[test]
    fn stderr_parser_accepts_only_bounded_stable_codes() {
        assert_eq!(
            super::parse_diagnostic_line(b"AIP_RUNTIME_DIAGNOSTIC runtime_worker_exception"),
            Some("runtime_worker_exception")
        );
        assert_eq!(
            super::parse_diagnostic_line(b"private conversation content"),
            None
        );
        assert_eq!(
            super::parse_diagnostic_line(b"AIP_RUNTIME_DIAGNOSTIC private_conversation_content"),
            None
        );
        let diagnostics = Arc::new(Mutex::new(super::RuntimeDiagnostics::default()));
        for _ in 0..(super::MAX_DIAGNOSTIC_CODES + 5) {
            super::record_stderr_code(&diagnostics, "runtime_worker_exception");
        }
        assert_eq!(
            diagnostics.lock().unwrap().stderr_codes.len(),
            super::MAX_DIAGNOSTIC_CODES
        );
    }
}
