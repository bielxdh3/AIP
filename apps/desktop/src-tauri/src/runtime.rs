use std::{
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
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
        health_request, parse_health_response, shutdown_request, MAX_MESSAGE_BYTES,
        PROTOCOL_VERSION,
    },
};

const HANDSHAKE_ID: &str = "phase0-health";

#[derive(Clone)]
pub struct RuntimeController {
    status: Arc<Mutex<RuntimeStatus>>,
    stop: Arc<AtomicBool>,
    worker: Arc<Mutex<Option<JoinHandle<()>>>>,
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
            source_root,
        }
    }

    pub fn snapshot(&self) -> RuntimeStatus {
        lock(&self.status).clone()
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
        let status = Arc::clone(&self.status);
        let stop = Arc::clone(&self.stop);
        let source_root = self.source_root.clone();
        *worker = Some(thread::spawn(move || {
            run_runtime_process(status, stop, source_root);
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
        if let Some(handle) = lock(&self.worker).take() {
            let _ = handle.join();
        }
    }
}

fn run_runtime_process(
    status: Arc<Mutex<RuntimeStatus>>,
    stop: Arc<AtomicBool>,
    source_root: PathBuf,
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
        set_status(
            &status,
            RuntimeState::Unavailable,
            None,
            "python_unavailable",
        );
        return;
    };
    let Some(mut stdin) = child.stdin.take() else {
        let _ = child.kill();
        set_status(
            &status,
            RuntimeState::Unavailable,
            None,
            "runtime_stdio_unavailable",
        );
        return;
    };
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        set_status(
            &status,
            RuntimeState::Unavailable,
            None,
            "runtime_stdio_unavailable",
        );
        return;
    };

    let Ok(request) = health_request(HANDSHAKE_ID) else {
        let _ = child.kill();
        set_status(
            &status,
            RuntimeState::Unavailable,
            None,
            "protocol_encoding_failed",
        );
        return;
    };
    if writeln!(stdin, "{request}")
        .and_then(|_| stdin.flush())
        .is_err()
    {
        let _ = child.kill();
        set_status(
            &status,
            RuntimeState::Unavailable,
            None,
            "runtime_handshake_failed",
        );
        return;
    }

    let (sender, receiver) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let result = reader
            .by_ref()
            .take((MAX_MESSAGE_BYTES + 1) as u64)
            .read_line(&mut line)
            .map(|read| (read, line));
        let _ = sender.send((result, reader));
    });

    let deadline = Instant::now() + Duration::from_secs(3);
    let handshake = loop {
        if stop.load(Ordering::SeqCst) || Instant::now() >= deadline {
            break None;
        }
        match receiver.recv_timeout(Duration::from_millis(75)) {
            Ok(result) => break Some(result),
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break None,
        }
    };

    let Some((handshake, _stdout_reader)) = handshake else {
        let _ = child.kill();
        let _ = child.wait();
        set_status(
            &status,
            RuntimeState::Unavailable,
            None,
            "runtime_handshake_failed",
        );
        return;
    };
    let handshake_valid = handshake.ok().is_some_and(|(read, line)| {
        read > 0
            && line.len() <= MAX_MESSAGE_BYTES
            && parse_health_response(line.trim_end(), HANDSHAKE_ID).is_ok()
    });
    if !handshake_valid {
        let _ = child.kill();
        let _ = child.wait();
        set_status(
            &status,
            RuntimeState::Unavailable,
            None,
            "runtime_handshake_failed",
        );
        return;
    }

    set_status(
        &status,
        RuntimeState::Ready,
        Some(PROTOCOL_VERSION),
        "runtime_ready",
    );

    loop {
        if stop.load(Ordering::SeqCst) {
            if let Ok(request) = shutdown_request("phase0-shutdown") {
                let _ = writeln!(stdin, "{request}").and_then(|_| stdin.flush());
            }
            let shutdown_deadline = Instant::now() + Duration::from_secs(2);
            while Instant::now() < shutdown_deadline {
                if child.try_wait().ok().flatten().is_some() {
                    break;
                }
                thread::sleep(Duration::from_millis(50));
            }
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
            set_status(&status, RuntimeState::Stopped, None, "runtime_stopped");
            return;
        }

        match child.try_wait() {
            Ok(Some(_)) | Err(_) => {
                set_status(
                    &status,
                    RuntimeState::Crashed,
                    None,
                    "runtime_process_ended",
                );
                return;
            }
            Ok(None) => thread::sleep(Duration::from_millis(100)),
        }
    }
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
}
