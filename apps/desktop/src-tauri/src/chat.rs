use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    io::{BufRead, BufReader, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    database::{now_millis, ContextMessage, Database, MessageAttempt},
    domain::{
        MessageAuthor, MessageStatus, PhaseOneEvent, PhaseOneState, ProviderSnapshot,
        ProviderState, QueueEntrySnapshot, RuntimeState, SendMessageResult,
        MAX_ASSISTANT_OUTPUT_BYTES, MAX_CONTEXT_BYTES, MAX_HISTORY_MESSAGES, MAX_QUEUE_LENGTH,
        MAX_USER_MESSAGE_BYTES,
    },
    orchestration::{
        ConnectivityState, HealthSnapshot, HealthState, OrchestrationError, OrchestrationManager,
        RequestPriority, RoutingMode, RoutingPolicy, RoutingRequest,
    },
    overlays,
    protocol::{
        cancellation_request, discovery_request, generation_request, show_model_request,
        valid_provider_model_id, PromptMessage, RuntimeOutput, PROTOCOL_VERSION,
    },
    runtime::{RuntimeController, RuntimeNotice},
};

const EVENT_NAME: &str = "phase-one-event";
const MAX_REQUEST_TRACE_ENTRIES: usize = 24;
const MAX_RETAINED_REQUEST_TRACES: usize = 16;
const MAX_PENDING_REQUEST_TRACES: usize = 16;
const MAX_IGNORED_TEMPORARY_REQUESTS: usize = 32;
const MAX_TRACE_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_TRACE_CODE_BYTES: usize = 128;
const CANCELLATION_GRACE_PERIOD: std::time::Duration = std::time::Duration::from_secs(5);
const DISCOVERY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);
const RUNTIME_ACCEPT_TIMEOUT: Duration = Duration::from_secs(15);
const GENERATION_RESPONSE_TIMEOUT: Duration = Duration::from_secs(90);
const GENERATION_WATCHDOG_INTERVAL: Duration = Duration::from_millis(250);
const MAX_TITLE_OUTPUT_BYTES: usize = 512;
const MAX_TITLE_CONTEXT_CHARS: usize = 2_000;

#[derive(Debug, Clone, PartialEq, Eq)]
struct GenerationJob {
    request_id: String,
    agent_id: String,
    conversation_id: String,
    branch_id: String,
    assistant_message_id: String,
    model_ref: String,
    temporary: bool,
}

#[derive(Debug, Default)]
struct TemporaryChatStore {
    conversations: HashMap<String, TemporaryConversation>,
    // Conversion removes the in-memory conversation while the React surface may still be
    // mounted. Keep a tombstone so a late temporary-state read cannot silently recreate it.
    converted_agents: HashSet<String>,
}

#[derive(Debug, Clone)]
struct TemporaryConversation {
    conversation: crate::domain::PhaseOneConversation,
    messages: Vec<crate::domain::ConversationMessage>,
    model_override_ref: Option<String>,
}

#[derive(Debug)]
struct TitleGeneration {
    agent_id: String,
    conversation_id: String,
    assistant_message_id: String,
    output: String,
    last_sequence: u64,
    output_bytes: usize,
    cancellation_requested: bool,
    dispatched_at: Instant,
    last_progress_at: Instant,
    runtime_accepted: bool,
    generation_started: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveGeneration {
    job: GenerationJob,
    last_sequence: u64,
    output_bytes: usize,
    cancellation_requested: bool,
    dispatched_at: Instant,
    last_progress_at: Instant,
    runtime_accepted: bool,
    generation_started: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CancellationDecision {
    Requested,
    AlreadyRequested,
    NotActive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ChunkDecision {
    Accepted(GenerationJob),
    Ignored,
    OutputLimitExceeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestTerminalState {
    Completed,
    Cancelled,
    FailedProvider,
    FailedOther,
}

fn request_terminal_state(
    event_type: &str,
    error_code: Option<&str>,
) -> Option<RequestTerminalState> {
    match event_type {
        "generation.complete" => Some(RequestTerminalState::Completed),
        "generation.cancelled" => Some(RequestTerminalState::Cancelled),
        "generation.failed" => Some(if error_code.is_some_and(is_provider_error_code) {
            RequestTerminalState::FailedProvider
        } else {
            RequestTerminalState::FailedOther
        }),
        _ => None,
    }
}

fn is_provider_error_code(code: &str) -> bool {
    code == "model_unavailable" || code.starts_with("provider_")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RequestTraceEntry {
    code: String,
    sequence: Option<u64>,
    terminal_code: Option<String>,
    timestamp_ms: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedTraceRecord {
    request_id: String,
    agent_id: String,
    conversation_id: String,
    branch_id: String,
    model_ref: String,
    code: String,
    sequence: Option<u64>,
    terminal_code: Option<String>,
    timestamp_ms: i64,
}

#[derive(Debug, Clone)]
struct RequestTraceMetadata {
    agent_id: String,
    conversation_id: String,
    branch_id: String,
    model_ref: String,
    temporary: bool,
}

#[derive(Debug, Clone, Default)]
struct RequestTraceStore {
    order: VecDeque<String>,
    entries: HashMap<String, VecDeque<RequestTraceEntry>>,
    metadata: HashMap<String, RequestTraceMetadata>,
    // Events that arrive before enqueue/register are kept separately. They become part of the
    // bounded request trace if registration succeeds, but transient failures cannot evict a
    // durable request from `order`.
    pending_order: VecDeque<String>,
    pending_entries: HashMap<String, VecDeque<RequestTraceEntry>>,
    ignored_temporary_order: VecDeque<String>,
    ignored_temporary_requests: HashSet<String>,
}

#[derive(Clone)]
struct TracePersistence {
    sender: mpsc::Sender<RequestTraceStore>,
}

impl TracePersistence {
    fn new(path: PathBuf) -> Self {
        let (sender, receiver) = mpsc::channel::<RequestTraceStore>();
        thread::spawn(move || {
            while let Ok(snapshot) = receiver.recv() {
                snapshot.persist(&path);
            }
        });
        Self { sender }
    }

    fn enqueue(&self, snapshot: RequestTraceStore) {
        let _ = self.sender.send(snapshot);
    }
}

fn should_persist_trace_code(code: &str) -> bool {
    !matches!(
        code,
        "chunk_persisted" | "generation.chunk" | "chunk_ignored"
    )
}

impl RequestTraceStore {
    fn record(
        &mut self,
        request_id: &str,
        code: &str,
        sequence: Option<u64>,
        terminal_code: Option<&str>,
    ) {
        if self.ignored_temporary_requests.contains(request_id) {
            return;
        }
        let entry = RequestTraceEntry {
            code: code.to_string(),
            sequence,
            terminal_code: terminal_code.map(str::to_string),
            timestamp_ms: now_millis(),
        };
        if self.metadata.contains_key(request_id) {
            self.ensure_registered_slot(request_id);
            self.append_entry(request_id, entry);
        } else {
            self.append_pending(request_id, entry);
        }
    }

    fn load(path: &PathBuf) -> Self {
        let Ok(file) = fs::File::open(path) else {
            return Self::default();
        };
        let Ok(file_length) = file.metadata().map(|metadata| metadata.len()) else {
            return Self::default();
        };
        let mut reader = BufReader::new(file);
        if file_length > MAX_TRACE_FILE_BYTES {
            let offset = file_length.saturating_sub(MAX_TRACE_FILE_BYTES);
            if reader.seek(SeekFrom::Start(offset)).is_err() {
                return Self::default();
            }
            // The seek may land in the middle of a record. Discard that partial line and
            // retain the newest bounded suffix, which is the only part that can survive the
            // store's request/entry retention limits.
            let mut partial = String::new();
            let _ = reader.read_line(&mut partial);
        }

        let mut store = Self::default();
        for line in reader.lines() {
            let Ok(line) = line else {
                continue;
            };
            if line.len() > MAX_TRACE_CODE_BYTES * 8 {
                continue;
            }
            let Ok(record) = serde_json::from_str::<PersistedTraceRecord>(&line) else {
                continue;
            };
            store.record_loaded(record);
        }
        store
    }

    fn record_loaded(&mut self, record: PersistedTraceRecord) {
        if !valid_trace_field(&record.request_id, 200)
            || !valid_trace_field(&record.agent_id, 200)
            || !valid_trace_field(&record.conversation_id, 200)
            || !valid_trace_field(&record.branch_id, 200)
            || !valid_trace_field(&record.model_ref, 200)
            || !valid_trace_code(&record.code)
            || !record
                .terminal_code
                .as_deref()
                .is_none_or(valid_trace_field_value)
        {
            return;
        }
        if !self.entries.contains_key(&record.request_id) {
            if self.order.len() == MAX_RETAINED_REQUEST_TRACES {
                if let Some(oldest) = self.order.pop_front() {
                    self.entries.remove(&oldest);
                    self.metadata.remove(&oldest);
                }
            }
            self.order.push_back(record.request_id.clone());
            self.entries
                .insert(record.request_id.clone(), VecDeque::new());
            self.metadata.insert(
                record.request_id.clone(),
                RequestTraceMetadata {
                    agent_id: record.agent_id.clone(),
                    conversation_id: record.conversation_id.clone(),
                    branch_id: record.branch_id.clone(),
                    model_ref: record.model_ref.clone(),
                    // Temporary traces are never written, so every record read from disk is
                    // safe to retain as a non-temporary historical trace.
                    temporary: false,
                },
            );
        } else if self
            .metadata
            .get(&record.request_id)
            .is_none_or(|metadata| {
                metadata.agent_id != record.agent_id
                    || metadata.conversation_id != record.conversation_id
                    || metadata.branch_id != record.branch_id
                    || metadata.model_ref != record.model_ref
            })
        {
            // A request id is the correlation key. Reject records that try to change its
            // immutable metadata instead of mixing unrelated content into one trace.
            return;
        }

        let trace = self
            .entries
            .get_mut(&record.request_id)
            .expect("loaded request trace exists");
        if trace.len() == MAX_REQUEST_TRACE_ENTRIES {
            trace.pop_front();
        }
        trace.push_back(RequestTraceEntry {
            code: record.code,
            sequence: record.sequence,
            terminal_code: record.terminal_code,
            timestamp_ms: record.timestamp_ms,
        });
    }

    fn register(&mut self, job: &GenerationJob) {
        if job.temporary {
            self.pending_entries.remove(&job.request_id);
            self.pending_order
                .retain(|request_id| request_id != &job.request_id);
            if self.ignored_temporary_order.len() == MAX_IGNORED_TEMPORARY_REQUESTS {
                if let Some(oldest) = self.ignored_temporary_order.pop_front() {
                    self.ignored_temporary_requests.remove(&oldest);
                }
            }
            self.ignored_temporary_order
                .push_back(job.request_id.clone());
            self.ignored_temporary_requests
                .insert(job.request_id.clone());
            return;
        }
        self.metadata.insert(
            job.request_id.clone(),
            RequestTraceMetadata {
                agent_id: job.agent_id.clone(),
                conversation_id: job.conversation_id.clone(),
                branch_id: job.branch_id.clone(),
                model_ref: job.model_ref.clone(),
                temporary: job.temporary,
            },
        );
        if let Some(entries) = self.pending_entries.remove(&job.request_id) {
            self.pending_order
                .retain(|request_id| request_id != &job.request_id);
            self.ensure_registered_slot(&job.request_id);
            for entry in entries {
                self.append_entry(&job.request_id, entry);
            }
        }
    }

    fn ensure_registered_slot(&mut self, request_id: &str) {
        if self.entries.contains_key(request_id) {
            return;
        }
        if self.order.len() == MAX_RETAINED_REQUEST_TRACES {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
                self.metadata.remove(&oldest);
            }
        }
        self.order.push_back(request_id.to_string());
        self.entries.insert(request_id.to_string(), VecDeque::new());
    }

    fn append_entry(&mut self, request_id: &str, entry: RequestTraceEntry) {
        let trace = self
            .entries
            .get_mut(request_id)
            .expect("request trace exists");
        if trace.len() == MAX_REQUEST_TRACE_ENTRIES {
            trace.pop_front();
        }
        trace.push_back(entry);
    }

    fn append_pending(&mut self, request_id: &str, entry: RequestTraceEntry) {
        if !self.pending_entries.contains_key(request_id) {
            if self.pending_order.len() == MAX_PENDING_REQUEST_TRACES {
                if let Some(oldest) = self.pending_order.pop_front() {
                    self.pending_entries.remove(&oldest);
                }
            }
            self.pending_order.push_back(request_id.to_string());
            self.pending_entries
                .insert(request_id.to_string(), VecDeque::new());
        }
        let trace = self
            .pending_entries
            .get_mut(request_id)
            .expect("pending request trace exists");
        if trace.len() == MAX_REQUEST_TRACE_ENTRIES {
            trace.pop_front();
        }
        trace.push_back(entry);
    }

    fn persist(&self, path: &PathBuf) {
        let Some(parent) = path.parent() else {
            return;
        };
        if fs::create_dir_all(parent).is_err() {
            return;
        }
        let temporary = path.with_extension("ndjson.tmp");
        let Ok(mut file) = fs::File::create(&temporary) else {
            return;
        };
        for request_id in &self.order {
            let Some(entries) = self.entries.get(request_id) else {
                continue;
            };
            let Some(metadata) = self.metadata.get(request_id) else {
                continue;
            };
            if metadata.temporary {
                continue;
            }
            for entry in entries {
                let record = serde_json::json!({
                    "requestId": request_id,
                    "agentId": metadata.agent_id,
                    "conversationId": metadata.conversation_id,
                    "branchId": metadata.branch_id,
                    "modelRef": metadata.model_ref,
                    "code": entry.code,
                    "sequence": entry.sequence,
                    "terminalCode": entry.terminal_code,
                    "timestampMs": entry.timestamp_ms,
                });
                if serde_json::to_writer(&mut file, &record).is_err()
                    || file.write_all(b"\n").is_err()
                {
                    return;
                }
            }
        }
        let _ = fs::remove_file(path);
        let _ = fs::rename(temporary, path);
    }

    #[cfg(test)]
    fn entries(&self, request_id: &str) -> Vec<RequestTraceEntry> {
        self.entries
            .get(request_id)
            .map(|entries| entries.iter().cloned().collect())
            .unwrap_or_default()
    }
}

fn valid_trace_field(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || "_-.".contains(character)
                || character == ':'
                || character == '/'
        })
}

fn valid_trace_field_value(value: &str) -> bool {
    valid_trace_field(value, MAX_TRACE_CODE_BYTES)
}

fn valid_trace_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_TRACE_CODE_BYTES
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
}

#[derive(Debug, Default)]
struct GenerationQueue {
    pending: VecDeque<GenerationJob>,
    active: Option<ActiveGeneration>,
}

impl GenerationQueue {
    fn enqueue(&mut self, job: GenerationJob) -> Result<(), &'static str> {
        if self.len() >= MAX_QUEUE_LENGTH {
            return Err("queue_full");
        }
        if self.contains(&job.request_id) {
            return Err("duplicate_request");
        }
        self.pending.push_back(job);
        Ok(())
    }

    fn activate_next(&mut self) -> Option<GenerationJob> {
        if self.active.is_some() {
            return None;
        }
        let job = self.pending.pop_front()?;
        self.active = Some(ActiveGeneration {
            job: job.clone(),
            last_sequence: 0,
            output_bytes: 0,
            cancellation_requested: false,
            dispatched_at: Instant::now(),
            last_progress_at: Instant::now(),
            runtime_accepted: false,
            generation_started: false,
        });
        Some(job)
    }

    fn cancel_queued(&mut self, request_id: &str) -> Option<GenerationJob> {
        let index = self
            .pending
            .iter()
            .position(|job| job.request_id == request_id)?;
        self.pending.remove(index)
    }

    fn finish_active(&mut self, request_id: &str) -> Option<GenerationJob> {
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.job.request_id == request_id)
        {
            return self.active.take().map(|active| active.job);
        }
        None
    }

    fn take_temporary_active(
        &mut self,
        agent_id: &str,
        conversation_id: &str,
    ) -> Option<GenerationJob> {
        let request_id = self.active.as_ref().and_then(|active| {
            (active.job.temporary
                && active.job.agent_id == agent_id
                && active.job.conversation_id == conversation_id)
                .then(|| active.job.request_id.clone())
        })?;
        self.finish_active(&request_id)
    }

    fn active_request(&self) -> Option<&str> {
        self.active
            .as_ref()
            .map(|active| active.job.request_id.as_str())
    }

    fn request_cancellation(&mut self, request_id: &str) -> CancellationDecision {
        let Some(active) = self.active.as_mut() else {
            return CancellationDecision::NotActive;
        };
        if active.job.request_id != request_id {
            return CancellationDecision::NotActive;
        }
        if active.cancellation_requested {
            return CancellationDecision::AlreadyRequested;
        }
        active.cancellation_requested = true;
        CancellationDecision::Requested
    }

    fn cancellation_is_pending(&self, request_id: &str) -> bool {
        self.active.as_ref().is_some_and(|active| {
            active.job.request_id == request_id && active.cancellation_requested
        })
    }

    fn matches_event(&self, event: &PhaseOneEvent) -> bool {
        self.active.as_ref().is_some_and(|active| {
            event.request_id.as_deref() == Some(active.job.request_id.as_str())
                && event.agent_id.as_deref() == Some(active.job.agent_id.as_str())
                && event.conversation_id.as_deref() == Some(active.job.conversation_id.as_str())
                && event.assistant_message_id.as_deref()
                    == Some(active.job.assistant_message_id.as_str())
        })
    }

    fn accepts_started(&self, event: &PhaseOneEvent) -> bool {
        self.matches_event(event) && event.sequence == Some(0)
    }

    fn mark_runtime_accepted(&mut self, request_id: &str) -> bool {
        let Some(active) = self.active.as_mut() else {
            return false;
        };
        if active.job.request_id != request_id {
            return false;
        }
        active.runtime_accepted = true;
        active.last_progress_at = Instant::now();
        true
    }

    fn mark_generation_started(&mut self, event: &PhaseOneEvent) -> bool {
        if !self.accepts_started(event) {
            return false;
        }
        let active = self
            .active
            .as_mut()
            .expect("started event has active generation");
        // The event is authoritative even if the response envelope is delivered later.
        active.runtime_accepted = true;
        active.generation_started = true;
        active.last_progress_at = Instant::now();
        true
    }

    fn expired_request(&self, now: Instant) -> Option<(String, &'static str)> {
        let active = self.active.as_ref()?;
        let code = if !active.runtime_accepted
            && now.duration_since(active.dispatched_at) >= RUNTIME_ACCEPT_TIMEOUT
        {
            "runtime_accept_timeout"
        } else if now.duration_since(active.last_progress_at) >= GENERATION_RESPONSE_TIMEOUT {
            if active.generation_started {
                "generation_response_timeout"
            } else {
                "generation_start_timeout"
            }
        } else {
            return None;
        };
        Some((active.job.request_id.clone(), code))
    }

    fn accepts_terminal(&self, event: &PhaseOneEvent) -> bool {
        self.active.as_ref().is_some_and(|active| {
            if !self.matches_event(event) {
                return false;
            }
            let Some(sequence) = event.sequence else {
                return false;
            };
            if active.cancellation_requested {
                return event.event_type == "generation.cancelled"
                    && sequence >= active.last_sequence;
            }
            sequence == active.last_sequence
        })
    }

    fn accept_chunk(
        &mut self,
        request_id: &str,
        sequence: u64,
        content_bytes: usize,
    ) -> ChunkDecision {
        let Some(active) = self.active.as_mut() else {
            return ChunkDecision::Ignored;
        };
        if active.job.request_id != request_id || sequence != active.last_sequence + 1 {
            return ChunkDecision::Ignored;
        }
        if active.cancellation_requested {
            // A chunk can already be in flight when cancellation is requested. Advance the
            // sequence without persisting content so its following cancellation terminal is
            // still attributable to this request.
            active.last_sequence = sequence;
            return ChunkDecision::Ignored;
        }
        let next_size = active.output_bytes.saturating_add(content_bytes);
        if next_size > MAX_ASSISTANT_OUTPUT_BYTES {
            return ChunkDecision::OutputLimitExceeded;
        }
        active.last_sequence = sequence;
        active.output_bytes = next_size;
        active.last_progress_at = Instant::now();
        ChunkDecision::Accepted(active.job.clone())
    }

    fn contains(&self, request_id: &str) -> bool {
        self.active_request() == Some(request_id)
            || self.pending.iter().any(|job| job.request_id == request_id)
    }

    fn len(&self) -> usize {
        self.pending.len() + usize::from(self.active.is_some())
    }

    fn snapshots(&self) -> Vec<QueueEntrySnapshot> {
        let mut snapshots = Vec::with_capacity(self.len());
        if let Some(active) = &self.active {
            snapshots.push(snapshot(
                &active.job,
                0,
                true,
                active.cancellation_requested,
            ));
        }
        snapshots.extend(
            self.pending
                .iter()
                .enumerate()
                .map(|(index, job)| snapshot(job, index + 1, false, false)),
        );
        snapshots
    }

    fn clear(&mut self) -> Vec<GenerationJob> {
        let mut jobs = Vec::with_capacity(self.len());
        if let Some(active) = self.active.take() {
            jobs.push(active.job);
        }
        jobs.extend(self.pending.drain(..));
        jobs
    }

    fn discard_temporary_pending(&mut self, agent_id: &str, conversation_id: &str) {
        self.pending.retain(|job| {
            !(job.temporary && job.agent_id == agent_id && job.conversation_id == conversation_id)
        });
    }
}

fn snapshot(
    job: &GenerationJob,
    position: usize,
    active: bool,
    cancellation_requested: bool,
) -> QueueEntrySnapshot {
    QueueEntrySnapshot {
        request_id: job.request_id.clone(),
        agent_id: job.agent_id.clone(),
        conversation_id: job.conversation_id.clone(),
        assistant_message_id: job.assistant_message_id.clone(),
        position,
        active,
        cancellation_requested,
    }
}

struct ChatInner {
    events: Arc<dyn ChatEventSink>,
    database: Database,
    runtime: RuntimeController,
    orchestration: Arc<Mutex<OrchestrationManager>>,
    safe_mode: Arc<AtomicBool>,
    provider: Mutex<ProviderSnapshot>,
    discovery_requests: Mutex<HashSet<String>>,
    model_detail_requests: Mutex<HashMap<String, String>>,
    send_lock: Mutex<()>,
    scheduler_lock: Mutex<()>,
    queue: Mutex<GenerationQueue>,
    title_generations: Mutex<HashMap<String, TitleGeneration>>,
    deferred_title_generations: Mutex<VecDeque<GenerationJob>>,
    request_traces: Mutex<RequestTraceStore>,
    request_trace_path: Option<PathBuf>,
    trace_persistence: Option<TracePersistence>,
    temporary_chats: Mutex<TemporaryChatStore>,
    cancellation_recovery: AtomicBool,
}

trait ChatEventSink: Send + Sync {
    fn emit(&self, event: PhaseOneEvent);
}

struct TauriEventSink {
    app: AppHandle,
}

impl ChatEventSink for TauriEventSink {
    fn emit(&self, event: PhaseOneEvent) {
        if let Some(agent_id) = event.agent_id.as_deref() {
            let _ = self.app.emit_to("main", EVENT_NAME, event.clone());
            if let Some(label) = overlays::window_label(agent_id) {
                let _ = self.app.emit_to(label, EVENT_NAME, event.clone());
            }
            if let Some(label) = overlays::bubble_window_label(agent_id) {
                let _ = self.app.emit_to(label, EVENT_NAME, event);
            }
        } else {
            let _ = self.app.emit(EVENT_NAME, event);
        }
    }
}

#[cfg(test)]
struct NoopEventSink;

#[cfg(test)]
impl ChatEventSink for NoopEventSink {
    fn emit(&self, _event: PhaseOneEvent) {}
}

#[derive(Clone)]
pub struct ChatCoordinator {
    inner: Arc<ChatInner>,
}

impl ChatCoordinator {
    pub fn new(
        app: AppHandle,
        database: Database,
        runtime: RuntimeController,
        safe_mode: Arc<AtomicBool>,
        orchestration: Arc<Mutex<OrchestrationManager>>,
    ) -> Self {
        let request_trace_path = app
            .path()
            .app_local_data_dir()
            .ok()
            .map(|path| path.join("diagnostics/generation-trace.ndjson"));
        Self::new_with_sink(
            Arc::new(TauriEventSink { app }),
            database,
            runtime,
            safe_mode,
            orchestration,
            request_trace_path,
        )
    }

    fn new_with_sink(
        events: Arc<dyn ChatEventSink>,
        database: Database,
        runtime: RuntimeController,
        safe_mode: Arc<AtomicBool>,
        orchestration: Arc<Mutex<OrchestrationManager>>,
        request_trace_path: Option<PathBuf>,
    ) -> Self {
        let receiver = runtime.subscribe();
        let request_traces = request_trace_path
            .as_ref()
            .map(RequestTraceStore::load)
            .unwrap_or_default();
        let trace_persistence = request_trace_path.clone().map(TracePersistence::new);
        let coordinator = Self {
            inner: Arc::new(ChatInner {
                events,
                database,
                runtime,
                orchestration,
                safe_mode,
                provider: Mutex::new(ProviderSnapshot::checking()),
                discovery_requests: Mutex::new(HashSet::new()),
                model_detail_requests: Mutex::new(HashMap::new()),
                send_lock: Mutex::new(()),
                scheduler_lock: Mutex::new(()),
                queue: Mutex::new(GenerationQueue::default()),
                title_generations: Mutex::new(HashMap::new()),
                deferred_title_generations: Mutex::new(VecDeque::new()),
                request_traces: Mutex::new(request_traces),
                request_trace_path,
                trace_persistence,
                temporary_chats: Mutex::new(TemporaryChatStore::default()),
                cancellation_recovery: AtomicBool::new(false),
            }),
        };
        let listener = coordinator.clone();
        thread::spawn(move || {
            while let Ok(notice) = receiver.recv() {
                listener.handle_notice(notice);
            }
        });
        let watchdog = coordinator.clone();
        thread::spawn(move || watchdog.generation_watchdog_loop());
        coordinator
    }

    #[cfg(test)]
    fn new_for_test(
        database: Database,
        runtime: RuntimeController,
        safe_mode: Arc<AtomicBool>,
        orchestration: Arc<Mutex<OrchestrationManager>>,
    ) -> Self {
        Self::new_with_sink(
            Arc::new(NoopEventSink),
            database,
            runtime,
            safe_mode,
            orchestration,
            None,
        )
    }

    pub fn state(&self, agent_id: &str) -> Result<PhaseOneState, &'static str> {
        let agent = self
            .inner
            .database
            .agent(agent_id)
            .map_err(|_| "operation_unavailable")?;
        let conversation = self
            .inner
            .database
            .active_conversation(agent_id)
            .map_err(|_| "operation_unavailable")?;
        let messages = self
            .inner
            .database
            .messages(agent_id, &conversation.id)
            .map_err(|_| "operation_failed")?;
        let branches = self
            .inner
            .database
            .branches(agent_id, &conversation.id)
            .map_err(|_| "operation_failed")?;
        let turn_variants = self
            .inner
            .database
            .turn_variants(agent_id, &conversation.id)
            .map_err(|_| "operation_failed")?;
        let active_branch_id = self
            .inner
            .database
            .active_branch_id(agent_id, &conversation.id)
            .map_err(|_| "operation_failed")?;
        let settings = self
            .inner
            .database
            .settings(agent_id)
            .map_err(|_| "operation_failed")?;
        let provider = lock(&self.inner.provider).clone();
        let default_model_ref = settings.selected_model_ref;
        let selected_model_ref = conversation
            .model_override_ref
            .clone()
            .or(default_model_ref.clone());
        let model_override_ref = conversation.model_override_ref.clone();
        let effective_model_source = if model_override_ref.is_some() {
            "conversation_override"
        } else {
            "agent_default"
        };
        let selected_model_available = selected_model_ref.as_ref().is_some_and(|selected| {
            provider
                .models
                .iter()
                .any(|model| &model.model_ref == selected)
        });
        let auto_candidate_available = self.auto_candidate_available();
        let queue = lock(&self.inner.queue).snapshots();
        let simulated_state = self
            .inner
            .database
            .simulated_state(agent_id)
            .map_err(|_| "operation_failed")?;
        let blocked = self.send_blocked_code(
            &provider,
            selected_model_ref.as_deref(),
            selected_model_available,
            model_override_ref.is_some(),
            queue.len(),
            simulated_state.suspended,
            auto_candidate_available,
        );
        Ok(PhaseOneState {
            agent,
            conversation,
            messages,
            branches,
            turn_variants,
            active_branch_id: Some(active_branch_id),
            provider,
            selected_model_ref,
            default_model_ref,
            model_override_ref,
            effective_model_source: effective_model_source.into(),
            selected_model_available,
            keep_alive_minutes: settings.keep_alive_minutes,
            queue,
            can_send: blocked.is_none(),
            send_blocked_code: blocked.map(str::to_string),
        })
    }

    pub fn temporary_chat_active(&self, agent_id: &str) -> bool {
        lock(&self.inner.temporary_chats)
            .conversations
            .contains_key(agent_id)
    }

    pub fn refresh_models(&self) -> Result<(), &'static str> {
        if self.inner.safe_mode.load(Ordering::SeqCst) {
            return Err("operation_unavailable");
        }
        if self.inner.runtime.snapshot().state != RuntimeState::Ready {
            let snapshot = ProviderSnapshot::unavailable("runtime_unavailable");
            self.sync_orchestration(&snapshot);
            *lock(&self.inner.provider) = snapshot;
            self.emit_refresh(None);
            return Err("runtime_unavailable");
        }
        if !lock(&self.inner.discovery_requests).is_empty() {
            return Ok(());
        }
        let snapshot = ProviderSnapshot::checking();
        self.sync_orchestration(&snapshot);
        *lock(&self.inner.provider) = snapshot;
        let request_id = format!("discover-{}", uuid::Uuid::now_v7());
        lock(&self.inner.discovery_requests).insert(request_id.clone());
        let request = discovery_request(&request_id).map_err(|_| "operation_failed")?;
        if let Err(error) = self.inner.runtime.send(request) {
            lock(&self.inner.discovery_requests).remove(&request_id);
            let snapshot = ProviderSnapshot::unavailable(error);
            self.sync_orchestration(&snapshot);
            *lock(&self.inner.provider) = snapshot;
            self.emit_refresh(None);
            return Err(error);
        }
        let coordinator = self.clone();
        thread::spawn(move || {
            thread::sleep(DISCOVERY_TIMEOUT);
            coordinator.expire_discovery(&request_id);
        });
        self.emit_refresh(None);
        Ok(())
    }

    pub fn provider_snapshot(&self) -> ProviderSnapshot {
        lock(&self.inner.provider).clone()
    }

    pub fn temporary_state(&self, agent_id: &str) -> Result<PhaseOneState, &'static str> {
        let mut state = self.state(agent_id)?;
        let conversation = {
            let mut chats = lock(&self.inner.temporary_chats);
            ensure_temporary_chat(&mut chats, agent_id)?
                .conversation
                .clone()
        };
        state.conversation = conversation.clone();
        state.messages = lock(&self.inner.temporary_chats)
            .conversations
            .get(agent_id)
            .map(|chat| chat.messages.clone())
            .unwrap_or_default();
        state.branches.clear();
        state.active_branch_id = None;
        let temporary_model_override = lock(&self.inner.temporary_chats)
            .conversations
            .get(agent_id)
            .and_then(|chat| chat.model_override_ref.clone());
        state.model_override_ref = temporary_model_override.clone();
        state.selected_model_ref = temporary_model_override.or(state.default_model_ref.clone());
        state.effective_model_source = if state.model_override_ref.is_some() {
            "temporary_override".into()
        } else {
            "agent_default".into()
        };
        state.selected_model_available =
            state.selected_model_ref.as_ref().is_some_and(|selected| {
                state
                    .provider
                    .models
                    .iter()
                    .any(|model| &model.model_ref == selected)
            });
        let simulated_state = self
            .inner
            .database
            .simulated_state(agent_id)
            .map_err(|_| "operation_failed")?;
        state.send_blocked_code = self
            .send_blocked_code(
                &state.provider,
                state.selected_model_ref.as_deref(),
                state.selected_model_available,
                state.model_override_ref.is_some(),
                state.queue.len(),
                simulated_state.suspended,
                self.auto_candidate_available(),
            )
            .map(str::to_string);
        state.can_send = state.send_blocked_code.is_none();
        Ok(state)
    }

    pub fn start_temporary(&self, agent_id: &str) -> Result<(), &'static str> {
        let _send_guard = lock(&self.inner.send_lock);
        let mut chats = lock(&self.inner.temporary_chats);
        chats.converted_agents.remove(agent_id);
        let _ = ensure_temporary_chat(&mut chats, agent_id)?;
        Ok(())
    }

    pub fn select_model(&self, agent_id: &str, model_ref: &str) -> Result<(), &'static str> {
        let provider = lock(&self.inner.provider);
        if !provider
            .models
            .iter()
            .any(|model| model.model_ref == model_ref)
        {
            return Err("model_unavailable");
        }
        drop(provider);
        self.inner
            .database
            .set_selected_model(agent_id, model_ref)
            .map_err(|_| "operation_failed")?;
        if let Some(provider_model_id) = model_ref.strip_prefix("ollama:") {
            let request_id = format!("show-{}", uuid::Uuid::now_v7());
            if let Ok(request) = show_model_request(&request_id, provider_model_id) {
                lock(&self.inner.model_detail_requests)
                    .insert(request_id.clone(), model_ref.to_string());
                if self.inner.runtime.send(request).is_err() {
                    lock(&self.inner.model_detail_requests).remove(&request_id);
                }
            }
        }
        self.emit_refresh(None);
        Ok(())
    }

    pub fn set_keep_alive(&self, agent_id: &str, minutes: u32) -> Result<(), &'static str> {
        self.inner
            .database
            .set_keep_alive(agent_id, minutes)
            .map_err(|_| "invalid_keep_alive")?;
        self.emit_refresh(None);
        Ok(())
    }

    pub fn set_conversation_override(
        &self,
        agent_id: &str,
        conversation_id: &str,
        model_ref: Option<&str>,
    ) -> Result<(), &'static str> {
        if let Some(model) = model_ref {
            if !lock(&self.inner.provider)
                .models
                .iter()
                .any(|candidate| candidate.model_ref == model)
            {
                return Err("model_unavailable");
            }
        }
        self.inner
            .database
            .set_conversation_override(agent_id, conversation_id, model_ref)
            .map_err(|_| "operation_failed")?;
        self.emit_refresh(Some(agent_id));
        Ok(())
    }

    pub fn set_temporary_model(
        &self,
        agent_id: &str,
        model_ref: Option<&str>,
    ) -> Result<(), &'static str> {
        if let Some(model) = model_ref {
            if !lock(&self.inner.provider)
                .models
                .iter()
                .any(|candidate| candidate.model_ref == model)
            {
                return Err("model_unavailable");
            }
        }
        let _ = self.temporary_state(agent_id)?;
        let mut chats = lock(&self.inner.temporary_chats);
        let chat = chats
            .conversations
            .get_mut(agent_id)
            .ok_or("operation_unavailable")?;
        chat.model_override_ref = model_ref.map(str::to_string);
        self.emit_refresh(Some(agent_id));
        Ok(())
    }

    #[allow(dead_code)]
    pub fn send_message(
        &self,
        agent_id: &str,
        conversation_id: &str,
        content: &str,
    ) -> Result<SendMessageResult, &'static str> {
        self.send_message_with_policy(agent_id, conversation_id, content, RoutingPolicy::default())
    }

    pub fn send_message_with_policy(
        &self,
        agent_id: &str,
        conversation_id: &str,
        content: &str,
        policy: RoutingPolicy,
    ) -> Result<SendMessageResult, &'static str> {
        let _send_guard = lock(&self.inner.send_lock);
        if content.is_empty() || content.len() > MAX_USER_MESSAGE_BYTES {
            return Err("invalid_message");
        }
        let state = self.state(agent_id)?;
        if state.conversation.id != conversation_id {
            return Err("operation_unavailable");
        }
        if let Some(code) = state.send_blocked_code.as_deref() {
            let auto_can_resolve = matches!(
                policy.mode,
                RoutingMode::Auto | RoutingMode::Quality | RoutingMode::Speed
            ) && state.model_override_ref.is_none();
            if !auto_can_resolve
                || !matches!(
                    code,
                    "model_not_selected" | "selected_model_unavailable" | "no_candidate"
                )
            {
                return Err(match code {
                    "queue_full" => "queue_full",
                    "safe_mode_active" => "safe_mode_active",
                    "agent_suspended" => "agent_suspended",
                    "model_not_selected" | "selected_model_unavailable" => "model_unavailable",
                    "no_candidate" => "no_candidate",
                    _ => "runtime_unavailable",
                });
            }
        }
        let request_id = uuid::Uuid::now_v7().to_string();
        self.trace(&request_id, "chat.send.received", None, None);
        let model_ref = self.reserve_route(
            &request_id,
            state.selected_model_ref.as_deref(),
            state.model_override_ref.as_deref(),
            &policy,
        )?;
        self.trace(&request_id, "chat.route.reserved", None, None);
        let attempt = self
            .inner
            .database
            .create_message_attempt_with_request_id(
                agent_id,
                conversation_id,
                content,
                &model_ref,
                &request_id,
            )
            .map_err(|_| {
                self.release_route(&request_id);
                "operation_failed"
            })?;
        let job = job_from_attempt(agent_id, conversation_id, &model_ref, &attempt);
        let enqueue_result = self.enqueue_job(&job);
        if let Err(code) = enqueue_result {
            let _ = self.finish_job(&job, MessageStatus::Failed, Some(code));
            return Err(code);
        }
        self.trace(&attempt.request_id, "request_enqueued", None, None);
        self.trace(&attempt.request_id, "chat.queue.enqueued", None, None);
        let result = SendMessageResult {
            request_id: attempt.request_id,
            conversation_id: conversation_id.to_string(),
            user_message_id: attempt.user_message_id,
            assistant_message_id: attempt.assistant_message_id,
        };
        self.emit_refresh(Some(agent_id));
        self.dispatch_next();
        Ok(result)
    }

    pub fn regenerate_message(
        &self,
        agent_id: &str,
        conversation_id: &str,
        assistant_message_id: &str,
        model_ref: Option<&str>,
        request_id: &str,
    ) -> Result<SendMessageResult, &'static str> {
        let model_ref = model_ref
            .map(str::to_string)
            .or_else(|| {
                self.inner
                    .database
                    .message_model_ref(assistant_message_id)
                    .ok()
            })
            .ok_or("model_unavailable")?;
        self.send_branch_attempt(
            agent_id,
            conversation_id,
            &model_ref,
            request_id,
            |database, model, request_id| {
                database.create_regeneration_attempt(
                    agent_id,
                    conversation_id,
                    assistant_message_id,
                    model,
                    request_id,
                )
            },
        )
    }

    pub fn edit_message(
        &self,
        agent_id: &str,
        conversation_id: &str,
        user_message_id: &str,
        content: &str,
    ) -> Result<SendMessageResult, &'static str> {
        if content.is_empty() || content.len() > MAX_USER_MESSAGE_BYTES {
            return Err("invalid_message");
        }
        let request_id = uuid::Uuid::now_v7().to_string();
        let model_ref = self
            .state(agent_id)?
            .selected_model_ref
            .ok_or("model_unavailable")?;
        self.send_branch_attempt(
            agent_id,
            conversation_id,
            &model_ref,
            &request_id,
            |database, model, request_id| {
                database.create_edited_attempt(
                    agent_id,
                    conversation_id,
                    user_message_id,
                    content,
                    model,
                    request_id,
                )
            },
        )
    }

    fn send_branch_attempt<F>(
        &self,
        agent_id: &str,
        conversation_id: &str,
        model_ref: &str,
        request_id: &str,
        create_attempt: F,
    ) -> Result<SendMessageResult, &'static str>
    where
        F: FnOnce(&Database, &str, &str) -> Result<MessageAttempt, crate::database::DatabaseError>,
    {
        let _send_guard = lock(&self.inner.send_lock);
        let state = self.state(agent_id)?;
        if state.conversation.id != conversation_id {
            return Err("operation_unavailable");
        }
        let explicit_retry_model_available = explicit_retry_can_bypass_selection_block(
            state.send_blocked_code.as_deref(),
            model_ref,
            &lock(&self.inner.provider),
        );
        if state.send_blocked_code.is_some() && !explicit_retry_model_available {
            return Err("runtime_unavailable");
        }
        self.reserve_route(
            request_id,
            Some(model_ref),
            Some(model_ref),
            &RoutingPolicy {
                mode: RoutingMode::Manual,
                ..RoutingPolicy::default()
            },
        )?;
        let attempt = match create_attempt(&self.inner.database, model_ref, request_id) {
            Ok(attempt) => attempt,
            Err(_) => {
                self.release_route(request_id);
                return Err("operation_failed");
            }
        };
        let job = job_from_attempt(agent_id, conversation_id, model_ref, &attempt);
        let enqueue_result = self.enqueue_job(&job);
        if let Err(code) = enqueue_result {
            if code == "duplicate_request" {
                return Ok(SendMessageResult {
                    request_id: attempt.request_id,
                    conversation_id: conversation_id.into(),
                    user_message_id: attempt.user_message_id,
                    assistant_message_id: attempt.assistant_message_id,
                });
            }
            let _ = self.finish_job(&job, MessageStatus::Failed, Some(code));
            return Err(code);
        }
        self.trace(&attempt.request_id, "request_enqueued", None, None);
        self.emit_refresh(Some(agent_id));
        self.dispatch_next();
        Ok(SendMessageResult {
            request_id: attempt.request_id,
            conversation_id: conversation_id.into(),
            user_message_id: attempt.user_message_id,
            assistant_message_id: attempt.assistant_message_id,
        })
    }

    pub fn select_branch(
        &self,
        agent_id: &str,
        conversation_id: &str,
        branch_id: &str,
    ) -> Result<(), &'static str> {
        self.inner
            .database
            .set_active_branch(agent_id, conversation_id, branch_id)
            .map_err(|_| "operation_failed")?;
        self.emit_refresh(Some(agent_id));
        Ok(())
    }

    #[allow(dead_code)]
    pub fn send_temporary_message(
        &self,
        agent_id: &str,
        content: &str,
    ) -> Result<SendMessageResult, &'static str> {
        self.send_temporary_message_with_policy(agent_id, content, RoutingPolicy::default())
    }

    pub fn send_temporary_message_with_policy(
        &self,
        agent_id: &str,
        content: &str,
        policy: RoutingPolicy,
    ) -> Result<SendMessageResult, &'static str> {
        // Capture the session before waiting for the send lock. Conversion takes this same
        // lock and clears the temporary store; a sender that was already waiting must not
        // recreate a hidden temporary conversation after conversion succeeds.
        let temporary_conversation_id = lock(&self.inner.temporary_chats)
            .conversations
            .get(agent_id)
            .map(|chat| chat.conversation.id.clone());
        let _send_guard = lock(&self.inner.send_lock);
        if let Some(expected_conversation_id) = temporary_conversation_id.as_deref() {
            if !temporary_session_is_active(
                &lock(&self.inner.temporary_chats),
                agent_id,
                expected_conversation_id,
            ) {
                return Err("operation_unavailable");
            }
        }
        if content.is_empty() || content.len() > MAX_USER_MESSAGE_BYTES {
            return Err("invalid_message");
        }
        let state = self.temporary_state(agent_id)?;
        if let Some(code) = state.send_blocked_code.as_deref() {
            let auto_can_resolve = matches!(
                policy.mode,
                RoutingMode::Auto | RoutingMode::Quality | RoutingMode::Speed
            ) && state.model_override_ref.is_none();
            if !auto_can_resolve
                || !matches!(
                    code,
                    "model_not_selected" | "selected_model_unavailable" | "no_candidate"
                )
            {
                return Err(match code {
                    "queue_full" => "queue_full",
                    "safe_mode_active" => "safe_mode_active",
                    "agent_suspended" => "agent_suspended",
                    "model_not_selected" | "selected_model_unavailable" => "model_unavailable",
                    "no_candidate" => "no_candidate",
                    _ => "runtime_unavailable",
                });
            }
        }
        let now = now_millis();
        let request_id = uuid::Uuid::now_v7().to_string();
        self.trace(&request_id, "chat.send.received", None, None);
        let model_ref = self.reserve_route(
            &request_id,
            state.selected_model_ref.as_deref(),
            state.model_override_ref.as_deref(),
            &policy,
        )?;
        self.trace(&request_id, "chat.route.reserved", None, None);
        let user_message_id = uuid::Uuid::now_v7().to_string();
        let assistant_message_id = uuid::Uuid::now_v7().to_string();
        let conversation = state.conversation.clone();
        {
            let mut chats = lock(&self.inner.temporary_chats);
            let chat = chats
                .conversations
                .get_mut(agent_id)
                .ok_or("operation_unavailable")?;
            chat.messages.push(crate::domain::ConversationMessage {
                id: user_message_id.clone(),
                conversation_id: conversation.id.clone(),
                agent_id: agent_id.into(),
                author: MessageAuthor::User,
                content: content.into(),
                model_ref: None,
                status: MessageStatus::Complete,
                created_at: now,
                completed_at: Some(now),
                error_code: None,
                branch_id: conversation.id.clone(),
                turn_group_id: user_message_id.clone(),
            });
            chat.messages.push(crate::domain::ConversationMessage {
                id: assistant_message_id.clone(),
                conversation_id: conversation.id.clone(),
                agent_id: agent_id.into(),
                author: MessageAuthor::Agent,
                content: String::new(),
                model_ref: Some(model_ref.clone()),
                status: MessageStatus::Pending,
                created_at: now + 1,
                completed_at: None,
                error_code: None,
                branch_id: conversation.id.clone(),
                turn_group_id: user_message_id.clone(),
            });
        }
        let job = GenerationJob {
            request_id: request_id.clone(),
            agent_id: agent_id.into(),
            conversation_id: conversation.id.clone(),
            branch_id: format!("{}:main", conversation.id),
            assistant_message_id: assistant_message_id.clone(),
            model_ref,
            temporary: true,
        };
        let enqueue_result = self.enqueue_job(&job);
        if let Err(code) = enqueue_result {
            let _ = self.finish_temporary(&job, MessageStatus::Failed, Some(code));
            return Err(code);
        }
        self.trace(&request_id, "temporary_request_enqueued", None, None);
        self.trace(&request_id, "chat.queue.enqueued", None, None);
        self.emit_refresh(Some(agent_id));
        self.dispatch_next();
        Ok(SendMessageResult {
            request_id,
            conversation_id: conversation.id,
            user_message_id,
            assistant_message_id,
        })
    }

    pub fn cancel(&self, request_id: &str) -> Result<(), &'static str> {
        let mut queue = lock(&self.inner.queue);
        if let Some(job) = queue.cancel_queued(request_id) {
            drop(queue);
            self.finish_job(&job, MessageStatus::Cancelled, None)
                .map_err(|_| "operation_failed")?;
            self.emit_terminal(&job, "generation.cancelled", None);
            return Ok(());
        }
        match queue.request_cancellation(request_id) {
            CancellationDecision::NotActive => return Err("generation_not_active"),
            CancellationDecision::AlreadyRequested => return Ok(()),
            CancellationDecision::Requested => {}
        }
        drop(queue);
        self.emit_refresh(None);
        let cancel_id = format!("cancel-{}", uuid::Uuid::now_v7());
        let request =
            cancellation_request(&cancel_id, request_id).map_err(|_| "operation_failed")?;
        if let Err(code) = self.inner.runtime.send(request) {
            self.fail_active(request_id, code);
            return Err(code);
        }
        self.trace(request_id, "cancel_sent", None, None);
        let coordinator = self.clone();
        let request_id = request_id.to_string();
        thread::spawn(move || {
            thread::sleep(CANCELLATION_GRACE_PERIOD);
            coordinator.recover_stalled_cancellation(&request_id);
        });
        Ok(())
    }

    pub fn enter_safe_mode(&self, error_code: &'static str) {
        self.inner.safe_mode.store(true, Ordering::SeqCst);
        let _scheduler_guard = lock(&self.inner.scheduler_lock);
        self.cancel_all_locked(error_code);
    }

    fn cancel_all_locked(&self, error_code: &'static str) {
        let mut queue = lock(&self.inner.queue);
        if let Some(request_id) = queue.active_request().map(str::to_string) {
            let cancel_id = format!("cancel-{}", uuid::Uuid::now_v7());
            if let Ok(request) = cancellation_request(&cancel_id, &request_id) {
                let _ = self.inner.runtime.send(request);
            }
        }
        let jobs = queue.clear();
        drop(queue);
        for job in jobs {
            let _ = self.finish_job(&job, MessageStatus::Cancelled, Some(error_code));
            self.emit_terminal(&job, "generation.cancelled", Some(error_code));
        }

        // Title generations are metadata work outside the main FIFO. Safe mode shuts down the
        // runtime without guaranteeing a terminal event, so clear both active and deferred title
        // state here to prevent a later HealthReady from blocking dispatch forever.
        let title_request_ids = {
            let mut requests = lock(&self.inner.title_generations);
            let ids = requests.keys().cloned().collect::<Vec<_>>();
            requests.clear();
            ids
        };
        lock(&self.inner.deferred_title_generations).clear();
        for request_id in title_request_ids {
            let cancel_id = format!("cancel-{}", uuid::Uuid::now_v7());
            if let Ok(request) = cancellation_request(&cancel_id, &request_id) {
                let _ = self.inner.runtime.send(request);
            }
            self.trace(
                &request_id,
                "title_generation_cancelled",
                None,
                Some(error_code),
            );
        }
    }

    pub fn reset_temporary(&self, agent_id: &str) -> Result<(), &'static str> {
        let conversation_id = lock(&self.inner.temporary_chats)
            .conversations
            .get(agent_id)
            .map(|chat| chat.conversation.id.clone());
        let Some(conversation_id) = conversation_id else {
            return Ok(());
        };
        let mut queue = lock(&self.inner.queue);
        let cancelled_active = queue.take_temporary_active(agent_id, &conversation_id);
        queue.discard_temporary_pending(agent_id, &conversation_id);
        drop(queue);
        clear_temporary_chat(&mut lock(&self.inner.temporary_chats), agent_id);
        if let Some(job) = cancelled_active {
            let cancel_id = format!("cancel-{}", uuid::Uuid::now_v7());
            if let Ok(request) = cancellation_request(&cancel_id, &job.request_id) {
                let _ = self.inner.runtime.send(request);
            }
        }
        self.emit_refresh(Some(agent_id));
        self.dispatch_next();
        Ok(())
    }

    pub fn continue_temporary(
        &self,
        agent_id: &str,
    ) -> Result<crate::domain::PhaseOneConversation, &'static str> {
        let _send_guard = lock(&self.inner.send_lock);
        let conversation_id = lock(&self.inner.temporary_chats)
            .conversations
            .get(agent_id)
            .map(|chat| chat.conversation.id.clone())
            .ok_or("operation_unavailable")?;
        if lock(&self.inner.queue)
            .snapshots()
            .iter()
            .any(|entry| entry.agent_id == agent_id && entry.conversation_id == conversation_id)
        {
            return Err("generation_active");
        }
        let conversation = continue_temporary_in_database(
            &self.inner.database,
            &self.inner.temporary_chats,
            agent_id,
        )?;
        Ok(conversation)
    }

    pub fn retry_runtime(&self) {
        if !self.inner.safe_mode.load(Ordering::SeqCst) {
            lock(&self.inner.discovery_requests).clear();
            self.inner.runtime.start();
            let snapshot = ProviderSnapshot::checking();
            self.sync_orchestration(&snapshot);
            *lock(&self.inner.provider) = snapshot;
            self.emit_refresh(None);
        }
    }

    fn expire_discovery(&self, request_id: &str) {
        if lock(&self.inner.discovery_requests).remove(request_id) {
            let snapshot = provider_error_snapshot("provider_timeout");
            self.sync_orchestration(&snapshot);
            *lock(&self.inner.provider) = snapshot;
            self.trace(
                request_id,
                "discovery_timed_out",
                None,
                Some("provider_timeout"),
            );
            self.emit_refresh(None);
        }
    }

    fn recover_stalled_cancellation(&self, request_id: &str) {
        let job = {
            let mut queue = lock(&self.inner.queue);
            if !queue.cancellation_is_pending(request_id) {
                return;
            }
            queue.finish_active(request_id)
        };
        let Some(job) = job else {
            return;
        };
        self.trace(
            request_id,
            "cancellation_watchdog",
            None,
            Some("generation_cancel_timeout"),
        );
        let _ = self.finish_job(
            &job,
            MessageStatus::Cancelled,
            Some("generation_cancel_timeout"),
        );
        self.emit_terminal(
            &job,
            "generation.cancelled",
            Some("generation_cancel_timeout"),
        );
        self.inner
            .cancellation_recovery
            .store(true, Ordering::SeqCst);
        lock(&self.inner.discovery_requests).clear();
        // A cancellation recovery is a runtime lifecycle event, not provider health evidence.
        self.inner.runtime.shutdown();
        self.inner.runtime.start();
        self.emit_refresh(Some(&job.agent_id));
        self.dispatch_next();
    }

    fn auto_candidate_available(&self) -> bool {
        let request = RoutingRequest {
            request_id: "state-auto".into(),
            model_ref: None,
            capability: crate::orchestration::ModelCapability::TextGeneration,
            priority: RequestPriority::ActiveConversation,
            additional_ram_mb: 0,
            max_latency_ms: None,
            created_at_ms: now_millis() as u64,
        };
        lock(&self.inner.orchestration)
            .rank_candidates_with_policy(&request, &RoutingPolicy::default())
            .is_ok()
    }

    fn reserve_route(
        &self,
        request_id: &str,
        selected_model: Option<&str>,
        explicit_model: Option<&str>,
        policy: &RoutingPolicy,
    ) -> Result<String, &'static str> {
        if policy.mode == RoutingMode::Manual && selected_model.is_none() {
            return Err("model_unavailable");
        }
        let policy = routing_policy_with_selection(policy, selected_model, explicit_model);
        let request = RoutingRequest {
            request_id: request_id.to_string(),
            model_ref: requested_model_ref(policy.mode, selected_model, explicit_model),
            capability: crate::orchestration::ModelCapability::TextGeneration,
            priority: RequestPriority::ActiveConversation,
            additional_ram_mb: 0,
            max_latency_ms: None,
            created_at_ms: now_millis() as u64,
        };
        lock(&self.inner.orchestration)
            .reserve_with_policy(request, policy, |_| Ok(()))
            .map(|reservation| reservation.model_ref)
            .map_err(orchestration_error_code)
    }

    fn release_route(&self, request_id: &str) {
        let _ = lock(&self.inner.orchestration).complete(request_id);
    }

    fn sync_orchestration(&self, provider: &ProviderSnapshot) {
        let health = provider_health(provider.state);
        let model_refs = provider
            .models
            .iter()
            .map(|model| model.model_ref.clone())
            .collect::<Vec<_>>();
        let _ = lock(&self.inner.orchestration).sync_local_provider(&model_refs, health, health);
    }

    #[allow(clippy::too_many_arguments)]
    fn send_blocked_code(
        &self,
        provider: &ProviderSnapshot,
        selected_model: Option<&str>,
        selected_available: bool,
        explicit_override: bool,
        queue_length: usize,
        suspended: bool,
        auto_candidate_available: bool,
    ) -> Option<&'static str> {
        send_blocked_code_for_state(
            self.inner.safe_mode.load(Ordering::SeqCst),
            self.inner.runtime.snapshot().state,
            provider,
            selected_model,
            selected_available,
            explicit_override,
            queue_length,
            suspended,
            auto_candidate_available,
        )
    }

    fn handle_notice(&self, notice: RuntimeNotice) {
        match notice {
            RuntimeNotice::Output(RuntimeOutput::Provider { id, mut snapshot }) => {
                if lock(&self.inner.discovery_requests).remove(&id) {
                    self.sync_orchestration(&snapshot);
                    snapshot.refreshed_at = Some(now_millis());
                    *lock(&self.inner.provider) = snapshot;
                    self.emit_refresh(None);
                }
            }
            RuntimeNotice::Output(RuntimeOutput::Error { id, code }) => {
                if lock(&self.inner.discovery_requests).remove(&id) {
                    let snapshot = provider_error_snapshot(&code);
                    self.sync_orchestration(&snapshot);
                    *lock(&self.inner.provider) = snapshot;
                    self.emit_refresh(None);
                } else if lock(&self.inner.model_detail_requests)
                    .remove(&id)
                    .is_none()
                {
                    let title_failed = {
                        let _scheduler_guard = lock(&self.inner.scheduler_lock);
                        lock(&self.inner.title_generations).remove(&id).is_some()
                    };
                    if title_failed {
                        self.trace(&id, "title_generation_failed", None, Some(&code));
                        self.dispatch_next();
                    } else {
                        self.trace(&id, "request_error", None, Some(&code));
                        self.fail_active(&id, &code);
                    }
                }
            }
            RuntimeNotice::Output(RuntimeOutput::ModelDetails {
                id,
                provider_model_id,
                capabilities,
            }) => {
                let Some(model_ref) = lock(&self.inner.model_detail_requests).remove(&id) else {
                    return;
                };
                let mut provider = lock(&self.inner.provider);
                if let Some(model) = provider.models.iter_mut().find(|model| {
                    model.model_ref == model_ref && model.provider_model_id == provider_model_id
                }) {
                    model.capabilities = capabilities;
                }
                drop(provider);
                self.emit_refresh(None);
            }
            RuntimeNotice::Output(RuntimeOutput::Event(event)) => {
                self.handle_generation_event(event)
            }
            RuntimeNotice::Trace {
                request_id,
                event,
                error_code,
            } => self.trace(&request_id, event, None, error_code.as_deref()),
            RuntimeNotice::Disconnected { detail_code } => {
                lock(&self.inner.discovery_requests).clear();
                let cancellation_recovery = self
                    .inner
                    .cancellation_recovery
                    .swap(false, Ordering::SeqCst);
                let current_provider = lock(&self.inner.provider).clone();
                let snapshot = provider_after_runtime_disconnect(
                    current_provider,
                    detail_code,
                    cancellation_recovery,
                );
                self.sync_orchestration(&snapshot);
                *lock(&self.inner.provider) = snapshot;
                if !cancellation_recovery {
                    self.fail_all("runtime_interrupted");
                }
                self.emit_refresh(None);
            }
            RuntimeNotice::Output(RuntimeOutput::HealthReady { .. }) => {
                self.inner
                    .cancellation_recovery
                    .store(false, Ordering::SeqCst);
                let _ = self.refresh_models();
                self.dispatch_next();
            }
            RuntimeNotice::Output(RuntimeOutput::Accepted { id }) => {
                let queue_accepted = lock(&self.inner.queue).mark_runtime_accepted(&id);
                let title_accepted = self.mark_title_runtime_accepted(&id);
                if queue_accepted || title_accepted {
                    self.trace(&id, "runtime_accepted", None, None);
                    self.trace(&id, "generation.accepted", None, None);
                } else {
                    self.trace(&id, "runtime_accepted_ignored", None, None);
                }
            }
        }
    }

    fn handle_generation_event(&self, event: PhaseOneEvent) {
        if self.handle_title_event(&event) {
            return;
        }
        if !lock(&self.inner.queue).matches_event(&event) {
            if let Some(request_id) = event.request_id.as_deref() {
                self.trace(
                    request_id,
                    "event_ignored",
                    event.sequence,
                    event.error_code.as_deref(),
                );
            }
            return;
        }
        let Some(request_id) = event.request_id.clone() else {
            return;
        };
        match event.event_type.as_str() {
            "generation.started" => {
                if !lock(&self.inner.queue).mark_generation_started(&event) {
                    self.trace(&request_id, "started_ignored", event.sequence, None);
                    return;
                }
                self.trace(&request_id, "generation_started", event.sequence, None);
                self.trace(&request_id, "generation.started", event.sequence, None);
                self.emit(event)
            }
            "generation.chunk" => {
                let Some(content) = event.content.as_deref() else {
                    return;
                };
                let Some(sequence) = event.sequence else {
                    return;
                };
                let mut queue = lock(&self.inner.queue);
                let decision = queue.accept_chunk(&request_id, sequence, content.len());
                drop(queue);
                let job = match decision {
                    ChunkDecision::Accepted(job) => job,
                    ChunkDecision::Ignored => {
                        self.trace(&request_id, "chunk_ignored", Some(sequence), None);
                        return;
                    }
                    ChunkDecision::OutputLimitExceeded => {
                        self.fail_active(&request_id, "provider_output_too_large");
                        return;
                    }
                };
                if self.append_job_chunk(&job, content).is_ok() {
                    if sequence == 1 {
                        self.trace(&request_id, "first_chunk", Some(sequence), None);
                    }
                    self.trace(&request_id, "chunk_persisted", Some(sequence), None);
                    self.trace(&request_id, "generation.chunk", Some(sequence), None);
                    self.emit(event);
                } else {
                    self.trace(&request_id, "persistence_failed", Some(sequence), None);
                    self.fail_active(&request_id, "persistence_failed");
                }
            }
            "generation.complete" => {
                if !lock(&self.inner.queue).accepts_terminal(&event) {
                    self.trace(&request_id, "terminal_ignored", event.sequence, None);
                    return;
                }
                self.trace(&request_id, "terminal_received", event.sequence, None);
                self.trace(&request_id, "generation.completed", event.sequence, None);
                self.finish_runtime_terminal(
                    &request_id,
                    RequestTerminalState::Completed,
                    None,
                    event,
                )
            }
            "generation.failed" => {
                if !lock(&self.inner.queue).accepts_terminal(&event) {
                    self.trace(
                        &request_id,
                        "terminal_ignored",
                        event.sequence,
                        event.error_code.as_deref(),
                    );
                    return;
                }
                let error_code = event
                    .error_code
                    .clone()
                    .unwrap_or_else(|| "provider_failed".into());
                self.trace(
                    &request_id,
                    "terminal_received",
                    event.sequence,
                    Some(&error_code),
                );
                self.trace(
                    &request_id,
                    "generation.failed",
                    event.sequence,
                    Some(&error_code),
                );
                self.finish_runtime_terminal(
                    &request_id,
                    request_terminal_state("generation.failed", Some(&error_code))
                        .expect("failed events have a terminal state"),
                    Some(&error_code),
                    event,
                );
            }
            "generation.cancelled" => {
                if !lock(&self.inner.queue).accepts_terminal(&event) {
                    self.trace(&request_id, "terminal_ignored", event.sequence, None);
                    return;
                }
                self.trace(&request_id, "terminal_received", event.sequence, None);
                self.trace(&request_id, "generation.cancelled", event.sequence, None);
                self.finish_runtime_terminal(
                    &request_id,
                    RequestTerminalState::Cancelled,
                    None,
                    event,
                )
            }
            _ => {}
        }
    }

    fn handle_title_event(&self, event: &PhaseOneEvent) -> bool {
        let Some(request_id) = event.request_id.as_deref() else {
            return false;
        };
        let mut completed = None;
        let mut failed = false;
        let mut cancellation_requested = false;
        let now = Instant::now();
        {
            let _scheduler_guard = lock(&self.inner.scheduler_lock);
            let mut requests = lock(&self.inner.title_generations);
            let Some(request) = requests.get_mut(request_id) else {
                return false;
            };
            if event.agent_id.as_deref() != Some(request.agent_id.as_str())
                || event.conversation_id.as_deref() != Some(request.conversation_id.as_str())
                || event.assistant_message_id.as_deref()
                    != Some(request.assistant_message_id.as_str())
            {
                return true;
            }
            if request.cancellation_requested {
                if matches!(
                    event.event_type.as_str(),
                    "generation.complete" | "generation.failed" | "generation.cancelled"
                ) {
                    requests.remove(request_id);
                    failed = true;
                }
            } else {
                match event.event_type.as_str() {
                    "generation.started" if event.sequence == Some(0) => {
                        request.generation_started = true;
                        request.last_progress_at = now;
                    }
                    "generation.chunk" => {
                        if let Some((sequence, content)) =
                            event.sequence.zip(event.content.as_deref())
                        {
                            let next_size = request.output_bytes.saturating_add(content.len());
                            if sequence != request.last_sequence + 1
                                || next_size > MAX_TITLE_OUTPUT_BYTES
                            {
                                request.cancellation_requested = true;
                                cancellation_requested = true;
                            } else {
                                request.last_sequence = sequence;
                                request.output_bytes = next_size;
                                request.output.push_str(content);
                                request.last_progress_at = now;
                            }
                        } else {
                            request.cancellation_requested = true;
                            cancellation_requested = true;
                        }
                    }
                    "generation.complete" => {
                        if event.sequence == Some(request.last_sequence) {
                            completed = requests.remove(request_id);
                        } else {
                            request.cancellation_requested = true;
                            cancellation_requested = true;
                        }
                    }
                    "generation.failed" | "generation.cancelled" => {
                        requests.remove(request_id);
                        failed = true;
                    }
                    _ => {}
                }
            }
        }
        if cancellation_requested {
            self.cancel_title_generation(request_id);
        } else if let Some(request) = completed {
            if let Some(title) = sanitize_generated_title(&request.output) {
                let committed = self.inner.database.commit_generated_title(
                    &request.agent_id,
                    &request.conversation_id,
                    &title,
                );
                if matches!(committed, Ok(true)) {
                    self.emit_refresh(Some(&request.agent_id));
                    self.emit_conversation_list_changed(
                        &request.agent_id,
                        &request.conversation_id,
                    );
                }
            }
            self.dispatch_next();
        } else if failed {
            self.trace(
                request_id,
                "title_generation_failed",
                event.sequence,
                event.error_code.as_deref(),
            );
            self.dispatch_next();
        }
        true
    }

    fn mark_title_runtime_accepted(&self, request_id: &str) -> bool {
        let _scheduler_guard = lock(&self.inner.scheduler_lock);
        let mut requests = lock(&self.inner.title_generations);
        let Some(request) = requests.get_mut(request_id) else {
            return false;
        };
        request.runtime_accepted = true;
        request.last_progress_at = Instant::now();
        true
    }

    fn cancel_title_generation(&self, request_id: &str) {
        let cancel_id = format!("cancel-{}", uuid::Uuid::now_v7());
        let Ok(request) = cancellation_request(&cancel_id, request_id) else {
            self.recover_stalled_title_cancellation(request_id);
            return;
        };
        if let Err(code) = self.inner.runtime.send(request) {
            self.trace(
                request_id,
                "title_generation_cancel_failed",
                None,
                Some(code),
            );
            self.recover_stalled_title_cancellation(request_id);
            return;
        }
        self.trace(request_id, "title_generation_cancel_sent", None, None);
        let coordinator = self.clone();
        let request_id = request_id.to_string();
        thread::spawn(move || {
            thread::sleep(CANCELLATION_GRACE_PERIOD);
            coordinator.recover_stalled_title_cancellation(&request_id);
        });
    }

    fn recover_stalled_title_cancellation(&self, request_id: &str) {
        let _scheduler_guard = lock(&self.inner.scheduler_lock);
        let mut requests = lock(&self.inner.title_generations);
        if !requests
            .get(request_id)
            .is_some_and(|request| request.cancellation_requested)
        {
            return;
        }
        requests.remove(request_id);
        drop(requests);
        self.trace(
            request_id,
            "title_generation_cancellation_watchdog",
            None,
            Some("generation_cancel_timeout"),
        );
        self.inner
            .cancellation_recovery
            .store(true, Ordering::SeqCst);
        self.inner.runtime.shutdown();
        if !self.inner.safe_mode.load(Ordering::SeqCst) {
            self.inner.runtime.start();
        }
    }

    fn expired_title_generation(&self, now: Instant) -> Option<(String, &'static str)> {
        let _scheduler_guard = lock(&self.inner.scheduler_lock);
        lock(&self.inner.title_generations)
            .iter()
            .find_map(|(request_id, request)| {
                title_generation_timeout(request, now).map(|code| (request_id.clone(), code))
            })
    }

    fn fail_timed_out_title_generation(&self, request_id: &str, code: &'static str) {
        let _scheduler_guard = lock(&self.inner.scheduler_lock);
        if lock(&self.inner.title_generations)
            .remove(request_id)
            .is_none()
        {
            return;
        }
        self.trace(
            request_id,
            "title_generation_watchdog_timeout",
            None,
            Some(code),
        );
        self.inner
            .cancellation_recovery
            .store(true, Ordering::SeqCst);
        self.trace(request_id, "runtime_recovery_restart", None, Some(code));
        self.inner.runtime.shutdown();
        if self.inner.safe_mode.load(Ordering::SeqCst) {
            self.inner
                .cancellation_recovery
                .store(false, Ordering::SeqCst);
        } else if !self.inner.safe_mode.load(Ordering::SeqCst) {
            self.inner.runtime.start();
        }
    }

    fn finish_runtime_terminal(
        &self,
        request_id: &str,
        terminal: RequestTerminalState,
        error_code: Option<&str>,
        event: PhaseOneEvent,
    ) {
        let status = match terminal {
            RequestTerminalState::Completed => MessageStatus::Complete,
            RequestTerminalState::Cancelled => MessageStatus::Cancelled,
            RequestTerminalState::FailedProvider | RequestTerminalState::FailedOther => {
                MessageStatus::Failed
            }
        };
        self.finish_active(request_id, status, error_code, event);
    }

    fn finish_active(
        &self,
        request_id: &str,
        status: MessageStatus,
        error_code: Option<&str>,
        event: PhaseOneEvent,
    ) {
        let Some(job) = lock(&self.inner.queue).finish_active(request_id) else {
            return;
        };
        let persisted = self.finish_job(&job, status, error_code);
        if persisted.is_ok() {
            if status == MessageStatus::Complete
                && !job.temporary
                && !self.inner.safe_mode.load(Ordering::SeqCst)
            {
                let _ = self.inner.database.refresh_conversation_summary_for_branch(
                    &job.agent_id,
                    &job.conversation_id,
                    &job.branch_id,
                );
                let _ = self
                    .inner
                    .database
                    .create_explicit_memory_candidate_for_branch(
                        &job.agent_id,
                        &job.conversation_id,
                        &job.branch_id,
                        &job.assistant_message_id,
                    );
                self.schedule_title_generation_or_defer(&job);
            }
            self.trace(request_id, "terminal_persisted", event.sequence, error_code);
            self.trace(request_id, "database.finalized", event.sequence, error_code);
            self.trace(request_id, "chat.finalized", event.sequence, error_code);
            self.emit(event);
            self.trace(request_id, "frontend.state_changed", None, error_code);
        } else {
            self.trace(request_id, "persistence_failed", event.sequence, None);
            self.trace(
                request_id,
                "chat.finalized.failed",
                event.sequence,
                Some("persistence_failed"),
            );
            self.emit_terminal(&job, "generation.failed", Some("persistence_failed"));
        }
        self.dispatch_next();
    }

    fn fail_active(&self, request_id: &str, code: &str) {
        let Some(job) = lock(&self.inner.queue).finish_active(request_id) else {
            return;
        };
        self.trace(request_id, "queue_finalized", None, Some(code));
        self.trace(request_id, "request_failed", None, Some(code));
        self.trace(request_id, "chat.finalized.failed", None, Some(code));
        let _ = self.finish_job(&job, MessageStatus::Failed, Some(code));
        self.emit_terminal(&job, "generation.failed", Some(code));
        self.dispatch_next();
    }

    fn fail_timed_out_generation(&self, request_id: &str, code: &'static str) {
        let _scheduler_guard = lock(&self.inner.scheduler_lock);
        let Some(job) = lock(&self.inner.queue).finish_active(request_id) else {
            return;
        };
        self.trace(request_id, "queue_finalized", None, Some(code));
        self.trace(request_id, "request_failed", None, Some(code));
        self.trace(request_id, "chat.finalized.failed", None, Some(code));
        let _ = self.finish_job(&job, MessageStatus::Failed, Some(code));
        self.emit_terminal(&job, "generation.failed", Some(code));

        // A timed-out provider call may still be inside the Python worker. Restart the
        // sidecar before advancing FIFO so the next request cannot receive generation_busy.
        self.inner
            .cancellation_recovery
            .store(true, Ordering::SeqCst);
        self.trace(request_id, "runtime_recovery_restart", None, Some(code));
        self.inner.runtime.shutdown();
        if self.inner.safe_mode.load(Ordering::SeqCst) {
            self.inner
                .cancellation_recovery
                .store(false, Ordering::SeqCst);
        } else if !self.inner.safe_mode.load(Ordering::SeqCst) {
            self.inner.runtime.start();
        }
    }

    fn fail_all(&self, code: &str) {
        let _scheduler_guard = lock(&self.inner.scheduler_lock);
        let jobs = lock(&self.inner.queue).clear();
        lock(&self.inner.title_generations).clear();
        lock(&self.inner.deferred_title_generations).clear();
        for job in jobs {
            self.trace(&job.request_id, "runtime.exited", None, Some(code));
            self.trace(&job.request_id, "request_failed", None, Some(code));
            let _ = self.finish_job(&job, MessageStatus::Failed, Some(code));
            self.emit_terminal(&job, "generation.failed", Some(code));
        }
    }

    fn schedule_title_generation(&self, job: &GenerationJob) {
        if self.inner.safe_mode.load(Ordering::SeqCst) {
            return;
        }
        let provider_model_id = match job.model_ref.strip_prefix("ollama:") {
            Some(model) if valid_provider_model_id(model) => model,
            _ => return,
        };
        let Ok(Some((user_content, assistant_content))) = self
            .inner
            .database
            .first_title_context_for_branch(&job.agent_id, &job.conversation_id, &job.branch_id)
        else {
            return;
        };
        if lock(&self.inner.title_generations)
            .values()
            .any(|request| request.conversation_id == job.conversation_id)
        {
            return;
        }
        let Ok(settings) = self.inner.database.settings(&job.agent_id) else {
            return;
        };
        let request_id = format!("title-{}", uuid::Uuid::now_v7());
        let assistant_message_id = format!("title-message-{}", uuid::Uuid::now_v7());
        let context = format!(
            "Mensagem do usuário:\n{}\n\nPrimeira resposta do agente:\n{}",
            bounded_title_context(&user_content),
            bounded_title_context(&assistant_content),
        );
        let messages = [
            PromptMessage {
                role: "system",
                content: "Gere apenas um título curto em português para esta conversa. Use 3 a 7 palavras, sem aspas, sem prefixo e com no máximo 64 caracteres. Não explique nada além do título.".into(),
            },
            PromptMessage {
                role: "user",
                content: context,
            },
        ];
        let Ok(request) = generation_request(
            &request_id,
            &job.agent_id,
            &job.conversation_id,
            &assistant_message_id,
            provider_model_id,
            settings.keep_alive_minutes,
            &messages,
        ) else {
            return;
        };
        let Ok(true) = self
            .inner
            .database
            .mark_generated_title_attempted(&job.agent_id, &job.conversation_id)
        else {
            return;
        };
        lock(&self.inner.title_generations).insert(
            request_id.clone(),
            TitleGeneration {
                agent_id: job.agent_id.clone(),
                conversation_id: job.conversation_id.clone(),
                assistant_message_id,
                output: String::new(),
                last_sequence: 0,
                output_bytes: 0,
                cancellation_requested: false,
                dispatched_at: Instant::now(),
                last_progress_at: Instant::now(),
                runtime_accepted: false,
                generation_started: false,
            },
        );
        if self.inner.runtime.send(request).is_err() {
            lock(&self.inner.title_generations).remove(&request_id);
            return;
        }
        self.trace(&request_id, "title_generation_sent", None, None);
    }

    fn schedule_title_generation_or_defer(&self, job: &GenerationJob) {
        let _scheduler_guard = lock(&self.inner.scheduler_lock);
        if self.inner.safe_mode.load(Ordering::SeqCst) {
            return;
        }
        if lock(&self.inner.queue).len() > 0 {
            let mut deferred = lock(&self.inner.deferred_title_generations);
            if !deferred
                .iter()
                .any(|candidate| candidate.conversation_id == job.conversation_id)
            {
                deferred.push_back(job.clone());
            }
            return;
        }
        self.schedule_title_generation(job);
    }

    fn dispatch_next(&self) {
        let _scheduler_guard = lock(&self.inner.scheduler_lock);
        self.dispatch_next_locked();
    }

    fn enqueue_job(&self, job: &GenerationJob) -> Result<(), &'static str> {
        let _scheduler_guard = lock(&self.inner.scheduler_lock);
        if self.inner.safe_mode.load(Ordering::SeqCst) {
            return Err("safe_mode_active");
        }
        let result = lock(&self.inner.queue).enqueue(job.clone());
        if result.is_ok() {
            lock(&self.inner.request_traces).register(job);
        }
        result
    }

    fn dispatch_next_locked(&self) {
        loop {
            if !lock(&self.inner.title_generations).is_empty() {
                return;
            }
            if self.inner.runtime.snapshot().state != RuntimeState::Ready
                || self.inner.safe_mode.load(Ordering::SeqCst)
            {
                return;
            }
            let queue_has_active_generation = {
                let queue = lock(&self.inner.queue);
                queue.active.is_some()
            };
            if queue_has_active_generation {
                return;
            }
            let Some(job) = lock(&self.inner.queue).activate_next() else {
                let Some(title_job) = lock(&self.inner.deferred_title_generations).pop_front()
                else {
                    return;
                };
                self.schedule_title_generation(&title_job);
                continue;
            };
            self.trace(&job.request_id, "queue_activated", None, None);
            self.trace(&job.request_id, "chat.queue.activated", None, None);
            self.trace(&job.request_id, "runtime_ready", None, None);
            let dispatch = self.build_generation_request(&job).and_then(|request| {
                self.trace(&job.request_id, "dispatching", None, None);
                self.mark_job_streaming(&job)?;
                self.inner.runtime.send(request)
            });
            if let Err(code) = dispatch {
                self.trace(&job.request_id, "queue_dispatch_failed", None, Some(code));
                let _ = lock(&self.inner.queue).finish_active(&job.request_id);
                let _ = self.finish_job(&job, MessageStatus::Failed, Some(code));
                self.emit_terminal(&job, "generation.failed", Some(code));
                continue;
            }
            self.trace(&job.request_id, "request_written", None, None);
            self.trace(&job.request_id, "generation.start.sent", None, None);
            self.trace(
                &job.request_id,
                "protocol.generation_start_written",
                None,
                None,
            );
            self.emit_refresh(Some(&job.agent_id));
            return;
        }
    }

    fn build_generation_request(&self, job: &GenerationJob) -> Result<String, &'static str> {
        if job.temporary {
            return self.build_temporary_generation_request(job);
        }
        build_generation_request_from_database(&self.inner.database, job)
    }

    fn build_temporary_generation_request(
        &self,
        job: &GenerationJob,
    ) -> Result<String, &'static str> {
        let agent = self
            .inner
            .database
            .agent(&job.agent_id)
            .map_err(|_| "operation_unavailable")?;
        let settings = self
            .inner
            .database
            .settings(&job.agent_id)
            .map_err(|_| "persistence_failed")?;
        let messages = lock(&self.inner.temporary_chats)
            .conversations
            .get(&job.agent_id)
            .filter(|chat| chat.conversation.id == job.conversation_id)
            .map(|chat| {
                chat.messages
                    .iter()
                    .filter(|message| message.status == MessageStatus::Complete)
                    .map(|message| ContextMessage {
                        author: message.author,
                        content: message.content.clone(),
                    })
                    .collect::<Vec<_>>()
            })
            .ok_or("operation_unavailable")?;
        let provider_model_id = job
            .model_ref
            .strip_prefix("ollama:")
            .filter(|model| valid_provider_model_id(model))
            .ok_or("model_unavailable")?;
        generation_request(
            &job.request_id,
            &job.agent_id,
            &job.conversation_id,
            &job.assistant_message_id,
            provider_model_id,
            settings.keep_alive_minutes,
            &assemble_context(&agent, messages),
        )
        .map_err(|_| "protocol_encoding_failed")
    }

    fn mark_job_streaming(&self, job: &GenerationJob) -> Result<(), &'static str> {
        if job.temporary {
            return self.update_temporary_message(job, MessageStatus::Streaming, None, None);
        }
        self.inner
            .database
            .mark_streaming(&job.assistant_message_id, &job.request_id)
            .map_err(|_| "persistence_failed")
    }

    fn append_job_chunk(&self, job: &GenerationJob, content: &str) -> Result<(), &'static str> {
        if job.temporary {
            return self.update_temporary_message(
                job,
                MessageStatus::Streaming,
                Some(content),
                None,
            );
        }
        self.inner
            .database
            .append_assistant_chunk(&job.assistant_message_id, &job.request_id, content)
            .map_err(|_| "persistence_failed")
    }

    fn finish_job(
        &self,
        job: &GenerationJob,
        status: MessageStatus,
        error_code: Option<&str>,
    ) -> Result<(), &'static str> {
        self.release_route(&job.request_id);
        if job.temporary {
            return self.finish_temporary(job, status, error_code);
        }
        self.inner
            .database
            .finish_assistant(
                &job.assistant_message_id,
                &job.request_id,
                status,
                error_code,
            )
            .map_err(|_| "persistence_failed")
    }

    fn finish_temporary(
        &self,
        job: &GenerationJob,
        status: MessageStatus,
        error_code: Option<&str>,
    ) -> Result<(), &'static str> {
        self.update_temporary_message(job, status, None, error_code)
    }

    fn update_temporary_message(
        &self,
        job: &GenerationJob,
        status: MessageStatus,
        chunk: Option<&str>,
        error_code: Option<&str>,
    ) -> Result<(), &'static str> {
        let mut chats = lock(&self.inner.temporary_chats);
        let chat = chats
            .conversations
            .get_mut(&job.agent_id)
            .filter(|chat| chat.conversation.id == job.conversation_id)
            .ok_or("persistence_failed")?;
        let message = chat
            .messages
            .iter_mut()
            .find(|message| {
                message.id == job.assistant_message_id
                    && message.status != MessageStatus::Complete
                    && message.status != MessageStatus::Failed
                    && message.status != MessageStatus::Cancelled
            })
            .ok_or("persistence_failed")?;
        if let Some(chunk) = chunk {
            message.content.push_str(chunk);
        }
        message.status = status;
        if matches!(
            status,
            MessageStatus::Complete | MessageStatus::Failed | MessageStatus::Cancelled
        ) {
            message.completed_at = Some(now_millis());
            message.error_code = error_code.map(str::to_string);
        }
        Ok(())
    }

    fn emit_terminal(&self, job: &GenerationJob, event_type: &str, error_code: Option<&str>) {
        self.emit(PhaseOneEvent {
            protocol_version: PROTOCOL_VERSION,
            event_type: event_type.into(),
            request_id: Some(job.request_id.clone()),
            agent_id: Some(job.agent_id.clone()),
            conversation_id: Some(job.conversation_id.clone()),
            assistant_message_id: Some(job.assistant_message_id.clone()),
            sequence: None,
            content: None,
            error_code: error_code.map(str::to_string),
        });
    }

    fn emit_refresh(&self, agent_id: Option<&str>) {
        self.emit(PhaseOneEvent {
            protocol_version: PROTOCOL_VERSION,
            event_type: "state.changed".into(),
            request_id: None,
            agent_id: agent_id.map(str::to_string),
            conversation_id: None,
            assistant_message_id: None,
            sequence: None,
            content: None,
            error_code: None,
        });
    }

    fn emit_conversation_list_changed(&self, agent_id: &str, conversation_id: &str) {
        self.emit(PhaseOneEvent {
            protocol_version: PROTOCOL_VERSION,
            event_type: "conversation-list.changed".into(),
            request_id: None,
            agent_id: Some(agent_id.to_string()),
            conversation_id: Some(conversation_id.to_string()),
            assistant_message_id: None,
            sequence: None,
            content: None,
            error_code: None,
        });
    }

    fn emit(&self, event: PhaseOneEvent) {
        self.inner.events.emit(event);
    }

    fn trace(
        &self,
        request_id: &str,
        code: &str,
        sequence: Option<u64>,
        terminal_code: Option<&str>,
    ) {
        let snapshot = {
            let mut traces = lock(&self.inner.request_traces);
            traces.record(request_id, code, sequence, terminal_code);
            should_persist_trace_code(code).then(|| traces.clone())
        };
        let Some(snapshot) = snapshot else {
            return;
        };
        if let Some(persistence) = self.inner.trace_persistence.as_ref() {
            persistence.enqueue(snapshot);
        } else if let Some(path) = self.inner.request_trace_path.as_ref() {
            snapshot.persist(path);
        }
    }

    fn generation_watchdog_loop(&self) {
        loop {
            thread::sleep(GENERATION_WATCHDOG_INTERVAL);
            if let Some((request_id, code)) = self.expired_title_generation(Instant::now()) {
                self.fail_timed_out_title_generation(&request_id, code);
                continue;
            }
            let expired = lock(&self.inner.queue).expired_request(Instant::now());
            let Some((request_id, code)) = expired else {
                continue;
            };
            self.trace(&request_id, "generation_watchdog_timeout", None, Some(code));
            self.fail_timed_out_generation(&request_id, code);
        }
    }
}

fn title_generation_timeout(request: &TitleGeneration, now: Instant) -> Option<&'static str> {
    if request.cancellation_requested {
        return None;
    }
    let (last_progress_at, timeout, code) =
        if request.runtime_accepted || request.generation_started {
            (
                request.last_progress_at,
                GENERATION_RESPONSE_TIMEOUT,
                "generation_response_timeout",
            )
        } else {
            (
                request.dispatched_at,
                RUNTIME_ACCEPT_TIMEOUT,
                "runtime_accept_timeout",
            )
        };
    (now.duration_since(last_progress_at) >= timeout).then_some(code)
}

fn bounded_title_context(content: &str) -> String {
    content.chars().take(MAX_TITLE_CONTEXT_CHARS).collect()
}

fn sanitize_generated_title(raw: &str) -> Option<String> {
    let mut title = raw.trim().replace(['\r', '\n', '\t'], " ");
    if let Some(stripped) = title
        .strip_prefix("Título:")
        .or_else(|| title.strip_prefix("título:"))
    {
        title = stripped.trim().into();
    }
    let title_chars = title.chars().collect::<Vec<_>>();
    if title_chars.len() >= 2
        && ((title_chars.first() == Some(&'"') && title_chars.last() == Some(&'"'))
            || (title_chars.first() == Some(&'“') && title_chars.last() == Some(&'”')))
    {
        title = title_chars[1..title_chars.len() - 1]
            .iter()
            .collect::<String>()
            .trim()
            .into();
    }
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.is_empty() {
        return None;
    }
    Some(title.chars().take(64).collect())
}

fn build_generation_request_from_database(
    database: &Database,
    job: &GenerationJob,
) -> Result<String, &'static str> {
    let agent = database
        .agent(&job.agent_id)
        .map_err(|_| "operation_unavailable")?;
    let settings = database
        .settings(&job.agent_id)
        .map_err(|_| "persistence_failed")?;
    let context = database
        .context_messages_for_branch(
            &job.agent_id,
            &job.conversation_id,
            &job.branch_id,
            MAX_HISTORY_MESSAGES,
        )
        .map_err(|_| "persistence_failed")?;
    let messages = assemble_context(&agent, context);
    let provider_model_id = job
        .model_ref
        .strip_prefix("ollama:")
        .filter(|model| valid_provider_model_id(model))
        .ok_or("model_unavailable")?;
    generation_request(
        &job.request_id,
        &job.agent_id,
        &job.conversation_id,
        &job.assistant_message_id,
        provider_model_id,
        settings.keep_alive_minutes,
        &messages,
    )
    .map_err(|_| "protocol_encoding_failed")
}

fn job_from_attempt(
    agent_id: &str,
    conversation_id: &str,
    model_ref: &str,
    attempt: &MessageAttempt,
) -> GenerationJob {
    GenerationJob {
        request_id: attempt.request_id.clone(),
        agent_id: agent_id.to_string(),
        conversation_id: conversation_id.to_string(),
        branch_id: attempt.branch_id.clone(),
        assistant_message_id: attempt.assistant_message_id.clone(),
        model_ref: model_ref.to_string(),
        temporary: false,
    }
}

fn temporary_conversation(agent_id: &str) -> crate::domain::PhaseOneConversation {
    crate::domain::PhaseOneConversation {
        id: format!("temporary-{agent_id}-{}", uuid::Uuid::now_v7()),
        agent_id: agent_id.to_string(),
        title: "Conversa temporária".into(),
        model_override_ref: None,
        is_pinned: false,
    }
}

fn clear_temporary_chat(store: &mut TemporaryChatStore, agent_id: &str) {
    store.conversations.remove(agent_id);
}

fn ensure_temporary_chat<'a>(
    store: &'a mut TemporaryChatStore,
    agent_id: &str,
) -> Result<&'a mut TemporaryConversation, &'static str> {
    if store.converted_agents.contains(agent_id) {
        return Err("operation_unavailable");
    }
    Ok(store
        .conversations
        .entry(agent_id.to_string())
        .or_insert_with(|| TemporaryConversation {
            conversation: temporary_conversation(agent_id),
            messages: Vec::new(),
            model_override_ref: None,
        }))
}

fn temporary_session_is_active(
    store: &TemporaryChatStore,
    agent_id: &str,
    conversation_id: &str,
) -> bool {
    store
        .conversations
        .get(agent_id)
        .is_some_and(|chat| chat.conversation.id == conversation_id)
}

fn continue_temporary_in_database(
    database: &Database,
    temporary_chats: &Mutex<TemporaryChatStore>,
    agent_id: &str,
) -> Result<crate::domain::PhaseOneConversation, &'static str> {
    let mut chats = lock(temporary_chats);
    if !chats.conversations.contains_key(agent_id) {
        return Err("operation_unavailable");
    }
    let conversation = database
        .create_conversation_and_activate(agent_id, "Nova conversa")
        .map_err(|error| error.code())?;
    chats.converted_agents.insert(agent_id.to_string());
    clear_temporary_chat(&mut chats, agent_id);
    Ok(conversation)
}

fn assemble_context(
    agent: &crate::domain::ProvisionalAgent,
    messages: Vec<ContextMessage>,
) -> Vec<PromptMessage> {
    let agent_name = &agent.name;
    let profile_key = &agent.profile_key;
    let instruction = format!(
        "Você é {agent_name}, um agente provisório local do perfil {profile_key}. Responda em português por padrão. Não afirme ter executado ações externas."
    );
    let instruction = format!(
        "{instruction} Persistent identity: species: {}; pronouns: {}; fictive age: {} ({}); personality: {}; validated traits (0 to 100): {}.",
        agent.species,
        agent.pronouns,
        agent.fictive_age,
        agent.age_category,
        agent.personality_summary,
        agent.traits_json,
    );
    let mut used = instruction.len();
    let mut selected = Vec::new();
    for message in messages.into_iter().rev() {
        if selected.len() == MAX_HISTORY_MESSAGES {
            break;
        }
        let bytes = message.content.len();
        if used.saturating_add(bytes) > MAX_CONTEXT_BYTES {
            break;
        }
        used += bytes;
        selected.push(message);
    }
    selected.reverse();
    let mut prompt = Vec::with_capacity(selected.len() + 1);
    prompt.push(PromptMessage {
        role: "system",
        content: instruction,
    });
    prompt.extend(selected.into_iter().map(|message| PromptMessage {
        role: match message.author {
            MessageAuthor::User => "user",
            MessageAuthor::Agent => "assistant",
            MessageAuthor::System => "system",
        },
        content: message.content,
    }));
    prompt
}

fn routing_policy_with_selection(
    policy: &RoutingPolicy,
    selected_model: Option<&str>,
    explicit_model: Option<&str>,
) -> RoutingPolicy {
    let mut policy = policy.clone();
    if let Some(explicit_model) = explicit_model {
        // An explicit conversation/temporary selection takes precedence over global routing
        // preferences, including exclusions and fallback-only hints.
        policy
            .excluded_model_refs
            .retain(|model_ref| model_ref != explicit_model);
        policy
            .fallback_only_model_refs
            .retain(|model_ref| model_ref != explicit_model);
        if policy.mode != RoutingMode::Manual {
            policy.preferred_model_ref = Some(explicit_model.to_string());
        }
    } else if let Some(selected_model) = selected_model {
        // An inherited agent default is only a soft preference. It must not bypass explicit
        // exclusions/fallback-only hints or replace a stronger policy preference.
        let eligible = !policy
            .excluded_model_refs
            .iter()
            .any(|model_ref| model_ref == selected_model)
            && !policy
                .fallback_only_model_refs
                .iter()
                .any(|model_ref| model_ref == selected_model);
        if policy.mode != RoutingMode::Manual && policy.preferred_model_ref.is_none() && eligible {
            policy.preferred_model_ref = Some(selected_model.to_string());
        }
    }
    policy
}

fn requested_model_ref(
    mode: RoutingMode,
    selected_model: Option<&str>,
    explicit_model: Option<&str>,
) -> Option<String> {
    explicit_model.map(str::to_string).or_else(|| {
        (mode == RoutingMode::Manual)
            .then(|| selected_model.map(str::to_string))
            .flatten()
    })
}

#[allow(clippy::too_many_arguments)]
fn send_blocked_code_for_state(
    safe_mode: bool,
    runtime_state: RuntimeState,
    provider: &ProviderSnapshot,
    selected_model: Option<&str>,
    selected_available: bool,
    explicit_override: bool,
    queue_length: usize,
    suspended: bool,
    auto_candidate_available: bool,
) -> Option<&'static str> {
    if safe_mode {
        Some("safe_mode_active")
    } else if suspended {
        Some("agent_suspended")
    } else if queue_length >= MAX_QUEUE_LENGTH {
        Some("queue_full")
    } else if runtime_state != RuntimeState::Ready {
        Some("runtime_unavailable")
    } else if provider.state == ProviderState::Checking {
        Some("provider_checking")
    } else if provider.state == ProviderState::Empty {
        Some("provider_empty")
    } else if provider.state != ProviderState::Available {
        Some("provider_unavailable")
    } else if selected_model.is_none() || !selected_available {
        if explicit_override && selected_model.is_some() && !selected_available {
            Some("selected_model_unavailable")
        } else if auto_candidate_available {
            None
        } else {
            Some("no_candidate")
        }
    } else {
        None
    }
}

fn explicit_retry_can_bypass_selection_block(
    blocked_code: Option<&str>,
    model_ref: &str,
    provider: &ProviderSnapshot,
) -> bool {
    blocked_code == Some("selected_model_unavailable")
        && provider
            .models
            .iter()
            .any(|candidate| candidate.model_ref == model_ref)
}

fn provider_error_snapshot(code: &str) -> ProviderSnapshot {
    let state = match code {
        "provider_malformed" | "provider_payload_too_large" | "provider_model_limit" => {
            ProviderState::Malformed
        }
        "provider_timeout" => ProviderState::Timeout,
        _ => ProviderState::Unavailable,
    };
    ProviderSnapshot {
        state,
        detail_code: code.to_string(),
        models: Vec::new(),
        refreshed_at: Some(now_millis()),
    }
}

fn orchestration_error_code(error: OrchestrationError) -> &'static str {
    match error {
        OrchestrationError::NoCompatibleCandidate
        | OrchestrationError::NoHealthyCandidate
        | OrchestrationError::NoResources
        | OrchestrationError::QueueFull
        | OrchestrationError::ReservationFailed
        | OrchestrationError::ReservationStoreFull => "no_candidate",
        OrchestrationError::InvalidContract("routing_policy") => "routing_policy_invalid",
        _ => "orchestration_unavailable",
    }
}

fn provider_health(state: ProviderState) -> HealthSnapshot {
    match state {
        ProviderState::Available | ProviderState::Empty => HealthSnapshot {
            state: HealthState::Healthy,
            connectivity: ConnectivityState::Connected,
            last_health_at_ms: Some(now_millis() as u64),
            latency_ms: None,
        },
        ProviderState::Checking => HealthSnapshot {
            state: HealthState::Unknown,
            connectivity: ConnectivityState::Checking,
            last_health_at_ms: None,
            latency_ms: None,
        },
        ProviderState::Unavailable | ProviderState::Malformed | ProviderState::Timeout => {
            HealthSnapshot {
                state: HealthState::Unavailable,
                connectivity: ConnectivityState::Disconnected,
                last_health_at_ms: None,
                latency_ms: None,
            }
        }
    }
}

fn provider_after_runtime_disconnect(
    current: ProviderSnapshot,
    detail_code: &str,
    cancellation_recovery: bool,
) -> ProviderSnapshot {
    if cancellation_recovery {
        current
    } else {
        ProviderSnapshot::unavailable(detail_code)
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
        fs,
        path::PathBuf,
        sync::{atomic::AtomicBool, mpsc, Arc, Mutex},
        thread,
        time::{Duration, Instant},
    };

    use crate::domain::{ConversationMessage, OllamaModel, ProviderState};
    use uuid::Uuid;

    use super::*;

    #[test]
    fn cancellation_recovery_preserves_provider_health() {
        let provider = ProviderSnapshot {
            state: ProviderState::Available,
            detail_code: "provider_available".into(),
            models: Vec::new(),
            refreshed_at: Some(1),
        };
        assert_eq!(
            provider_after_runtime_disconnect(provider.clone(), "runtime_disconnected", true),
            provider
        );
        assert_eq!(
            provider_after_runtime_disconnect(provider, "runtime_disconnected", false).state,
            ProviderState::Unavailable
        );
        assert_eq!(
            provider_after_runtime_disconnect(
                ProviderSnapshot::checking(),
                "runtime_process_exit_unexpected",
                false,
            )
            .detail_code,
            "runtime_process_exit_unexpected"
        );
    }

    fn available_provider() -> ProviderSnapshot {
        ProviderSnapshot {
            state: ProviderState::Available,
            detail_code: "provider_available".into(),
            models: Vec::new(),
            refreshed_at: Some(1),
        }
    }

    #[test]
    fn auto_routing_never_bypasses_suspension_or_queue_limits() {
        let provider = available_provider();
        assert_eq!(
            send_blocked_code_for_state(
                false,
                RuntimeState::Ready,
                &provider,
                None,
                false,
                false,
                0,
                true,
                true,
            ),
            Some("agent_suspended")
        );
        assert_eq!(
            send_blocked_code_for_state(
                false,
                RuntimeState::Ready,
                &provider,
                None,
                false,
                false,
                MAX_QUEUE_LENGTH,
                false,
                true,
            ),
            Some("queue_full")
        );
    }

    #[test]
    fn unavailable_explicit_override_never_falls_back_to_auto() {
        let provider = available_provider();
        assert_eq!(
            send_blocked_code_for_state(
                false,
                RuntimeState::Ready,
                &provider,
                Some("ollama:missing"),
                false,
                true,
                0,
                false,
                true,
            ),
            Some("selected_model_unavailable")
        );
        assert_eq!(
            send_blocked_code_for_state(
                false,
                RuntimeState::Ready,
                &provider,
                Some("ollama:default"),
                false,
                false,
                0,
                false,
                true,
            ),
            None
        );
        assert_eq!(
            send_blocked_code_for_state(
                false,
                RuntimeState::Ready,
                &provider,
                Some("ollama:default"),
                false,
                false,
                0,
                false,
                false,
            ),
            Some("no_candidate")
        );
    }

    #[test]
    fn explicit_retry_bypasses_only_a_selection_block_for_an_available_model() {
        let mut provider = available_provider();
        provider.models.push(OllamaModel {
            model_ref: "ollama:installed".into(),
            provider_model_id: "installed".into(),
            display_name: "Installed".into(),
            size: 1,
            family: None,
            parameter_size: None,
            quantization: None,
            capabilities: Vec::new(),
        });
        assert!(explicit_retry_can_bypass_selection_block(
            Some("selected_model_unavailable"),
            "ollama:installed",
            &provider,
        ));
        assert!(!explicit_retry_can_bypass_selection_block(
            Some("runtime_unavailable"),
            "ollama:installed",
            &provider,
        ));
        assert!(!explicit_retry_can_bypass_selection_block(
            Some("selected_model_unavailable"),
            "ollama:missing",
            &provider,
        ));
    }

    #[test]
    fn auto_selection_preserves_selected_model_as_preference() {
        let policy =
            routing_policy_with_selection(&RoutingPolicy::default(), Some("ollama:selected"), None);
        assert_eq!(
            policy.preferred_model_ref.as_deref(),
            Some("ollama:selected")
        );

        let manual = routing_policy_with_selection(
            &RoutingPolicy {
                mode: RoutingMode::Manual,
                ..RoutingPolicy::default()
            },
            Some("ollama:selected"),
            None,
        );
        assert_eq!(manual.preferred_model_ref, None);
    }

    #[test]
    fn explicit_model_selection_overrides_global_routing_hints() {
        let policy = routing_policy_with_selection(
            &RoutingPolicy {
                excluded_model_refs: vec!["ollama:selected".into()],
                fallback_only_model_refs: vec!["ollama:selected".into()],
                preferred_model_ref: Some("ollama:global".into()),
                ..RoutingPolicy::default()
            },
            Some("ollama:selected"),
            Some("ollama:selected"),
        );
        assert_eq!(
            policy.preferred_model_ref.as_deref(),
            Some("ollama:selected")
        );
        assert!(!policy
            .excluded_model_refs
            .iter()
            .any(|model_ref| model_ref == "ollama:selected"));
        assert!(!policy
            .fallback_only_model_refs
            .iter()
            .any(|model_ref| model_ref == "ollama:selected"));
    }

    #[test]
    fn inherited_default_preserves_exclusion_and_is_not_forced() {
        let policy = routing_policy_with_selection(
            &RoutingPolicy {
                excluded_model_refs: vec!["ollama:default".into()],
                ..RoutingPolicy::default()
            },
            Some("ollama:default"),
            None,
        );
        assert_eq!(policy.excluded_model_refs, vec!["ollama:default"]);
        assert_eq!(policy.preferred_model_ref, None);
        assert_eq!(
            requested_model_ref(RoutingMode::Auto, Some("ollama:default"), None),
            None
        );
    }

    #[test]
    fn inherited_default_preserves_fallback_only_restriction() {
        let policy = routing_policy_with_selection(
            &RoutingPolicy {
                fallback_only_model_refs: vec!["ollama:default".into()],
                ..RoutingPolicy::default()
            },
            Some("ollama:default"),
            None,
        );
        assert_eq!(policy.fallback_only_model_refs, vec!["ollama:default"]);
        assert_eq!(policy.preferred_model_ref, None);
        assert_eq!(
            requested_model_ref(RoutingMode::Quality, Some("ollama:default"), None),
            None
        );
    }

    #[test]
    fn inherited_default_does_not_replace_stronger_policy_preference() {
        let policy = routing_policy_with_selection(
            &RoutingPolicy {
                preferred_model_ref: Some("ollama:preferred".into()),
                ..RoutingPolicy::default()
            },
            Some("ollama:default"),
            None,
        );
        assert_eq!(
            policy.preferred_model_ref.as_deref(),
            Some("ollama:preferred")
        );
        assert_eq!(
            requested_model_ref(RoutingMode::Speed, Some("ollama:default"), None),
            None
        );
    }

    #[test]
    fn inherited_default_is_only_a_soft_preference_without_stronger_hints() {
        let policy =
            routing_policy_with_selection(&RoutingPolicy::default(), Some("ollama:default"), None);
        assert_eq!(
            policy.preferred_model_ref.as_deref(),
            Some("ollama:default")
        );
        for mode in [RoutingMode::Auto, RoutingMode::Quality, RoutingMode::Speed] {
            assert_eq!(
                requested_model_ref(mode, Some("ollama:default"), None),
                None
            );
        }
    }

    #[test]
    fn coordinator_route_precedence_separates_inherited_defaults_from_overrides() {
        let inherited = routing_policy_with_selection(
            &RoutingPolicy {
                excluded_model_refs: vec!["ollama:default".into()],
                preferred_model_ref: Some("ollama:preferred".into()),
                ..RoutingPolicy::default()
            },
            Some("ollama:default"),
            None,
        );
        assert_eq!(
            inherited.preferred_model_ref.as_deref(),
            Some("ollama:preferred")
        );
        assert_eq!(inherited.excluded_model_refs, vec!["ollama:default"]);
        assert_eq!(
            requested_model_ref(RoutingMode::Auto, Some("ollama:default"), None),
            None
        );

        let explicit = routing_policy_with_selection(
            &RoutingPolicy {
                excluded_model_refs: vec!["ollama:selected".into()],
                fallback_only_model_refs: vec!["ollama:selected".into()],
                preferred_model_ref: Some("ollama:preferred".into()),
                ..RoutingPolicy::default()
            },
            Some("ollama:selected"),
            Some("ollama:selected"),
        );
        assert_eq!(
            explicit.preferred_model_ref.as_deref(),
            Some("ollama:selected")
        );
        assert!(explicit.excluded_model_refs.is_empty());
        assert!(explicit.fallback_only_model_refs.is_empty());
        assert_eq!(
            requested_model_ref(
                RoutingMode::Auto,
                Some("ollama:selected"),
                Some("ollama:selected"),
            ),
            Some("ollama:selected".into())
        );
    }

    #[test]
    fn explicit_conversation_model_constrains_auto_routing() {
        assert_eq!(
            requested_model_ref(
                RoutingMode::Auto,
                Some("ollama:default"),
                Some("ollama:selected"),
            ),
            Some("ollama:selected".into())
        );
        assert_eq!(
            requested_model_ref(RoutingMode::Quality, Some("ollama:default"), None),
            None
        );
        assert_eq!(
            requested_model_ref(RoutingMode::Manual, Some("ollama:selected"), None),
            Some("ollama:selected".into())
        );
        assert_eq!(
            requested_model_ref(
                RoutingMode::Manual,
                Some("ollama:default"),
                Some("ollama:selected"),
            ),
            Some("ollama:selected".into())
        );
    }

    #[test]
    fn request_terminals_keep_provider_and_other_failures_distinct() {
        assert_eq!(
            request_terminal_state("generation.complete", None),
            Some(RequestTerminalState::Completed)
        );
        assert_eq!(
            request_terminal_state("generation.cancelled", None),
            Some(RequestTerminalState::Cancelled)
        );
        assert_eq!(
            request_terminal_state("generation.failed", Some("provider_stream_failed")),
            Some(RequestTerminalState::FailedProvider)
        );
        assert_eq!(
            request_terminal_state("generation.failed", Some("runtime_interrupted")),
            Some(RequestTerminalState::FailedOther)
        );
    }

    fn job(id: &str, agent: &str) -> GenerationJob {
        GenerationJob {
            request_id: id.into(),
            agent_id: agent.into(),
            conversation_id: format!("conversation-{agent}"),
            branch_id: format!("conversation-{agent}:main"),
            assistant_message_id: format!("message-{id}"),
            model_ref: "ollama:test".into(),
            temporary: false,
        }
    }

    #[test]
    fn queue_is_fifo_bounded_and_agent_independent() {
        let mut queue = GenerationQueue::default();
        queue.enqueue(job("one", "astra")).unwrap();
        queue.enqueue(job("two", "luma")).unwrap();
        assert_eq!(queue.activate_next().unwrap().request_id, "one");
        assert_eq!(queue.active_request(), Some("one"));
        assert_eq!(queue.finish_active("one").unwrap().agent_id, "astra");
        assert_eq!(queue.activate_next().unwrap().request_id, "two");
        assert_eq!(queue.finish_active("two").unwrap().agent_id, "luma");
        for index in 0..MAX_QUEUE_LENGTH {
            queue
                .enqueue(job(&format!("bound-{index}"), "astra"))
                .unwrap();
        }
        assert_eq!(queue.enqueue(job("overflow", "luma")), Err("queue_full"));
    }

    #[test]
    fn queue_watchdog_distinguishes_acceptance_and_response_timeouts() {
        let mut queue = GenerationQueue::default();
        queue.enqueue(job("timeout", "astra")).unwrap();
        queue.activate_next();

        let acceptance_deadline = Instant::now() + RUNTIME_ACCEPT_TIMEOUT + Duration::from_secs(1);
        assert_eq!(
            queue.expired_request(acceptance_deadline),
            Some(("timeout".into(), "runtime_accept_timeout"))
        );

        let active = queue.active.as_mut().unwrap();
        active.runtime_accepted = true;
        active.generation_started = true;
        active.last_progress_at =
            Instant::now() - GENERATION_RESPONSE_TIMEOUT - Duration::from_secs(1);
        assert_eq!(
            queue.expired_request(Instant::now()),
            Some(("timeout".into(), "generation_response_timeout"))
        );
    }

    #[test]
    fn title_watchdog_distinguishes_acceptance_and_response_timeouts() {
        let now = Instant::now();
        let mut request = TitleGeneration {
            agent_id: "astra".into(),
            conversation_id: "conversation-astra".into(),
            assistant_message_id: "title-message".into(),
            output: String::new(),
            last_sequence: 0,
            output_bytes: 0,
            cancellation_requested: false,
            dispatched_at: now - RUNTIME_ACCEPT_TIMEOUT - Duration::from_secs(1),
            last_progress_at: now,
            runtime_accepted: false,
            generation_started: false,
        };
        assert_eq!(
            title_generation_timeout(&request, now),
            Some("runtime_accept_timeout")
        );

        request.runtime_accepted = true;
        request.last_progress_at = now - GENERATION_RESPONSE_TIMEOUT - Duration::from_secs(1);
        assert_eq!(
            title_generation_timeout(&request, now),
            Some("generation_response_timeout")
        );
    }

    #[test]
    fn temporary_conversations_are_distinct_from_persisted_conversation_ids() {
        let astra = temporary_conversation("astra");
        let luma = temporary_conversation("luma");
        assert!(astra.id.starts_with("temporary-astra-"));
        assert!(luma.id.starts_with("temporary-luma-"));
        assert_ne!(astra.id, luma.id);
        assert_eq!(astra.model_override_ref, None);
    }

    #[test]
    fn conversion_invalidates_a_waiting_temporary_send_session() {
        let mut store = TemporaryChatStore::default();
        let conversation = temporary_conversation("astra");
        let conversation_id = conversation.id.clone();
        store.conversations.insert(
            "astra".into(),
            TemporaryConversation {
                conversation,
                messages: Vec::new(),
                model_override_ref: None,
            },
        );
        assert!(temporary_session_is_active(
            &store,
            "astra",
            &conversation_id
        ));

        clear_temporary_chat(&mut store, "astra");

        assert!(!temporary_session_is_active(
            &store,
            "astra",
            &conversation_id
        ));
    }

    #[test]
    fn failed_temporary_conversion_keeps_the_in_memory_chat() {
        let path = std::env::temp_dir()
            .join(format!("aip-temporary-conversion-{}", Uuid::now_v7()))
            .join("aip.sqlite3");
        let database = Database::initialize(&path).unwrap();
        database
            .open()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER fail_conversation_activation
                 BEFORE UPDATE OF active_conversation_id ON agent_phase3_settings
                 WHEN NEW.agent_id = 'agt_astra_provisional'
                   AND NEW.active_conversation_id <> OLD.active_conversation_id
                 BEGIN
                   SELECT RAISE(ABORT, 'simulated activation failure');
                 END;",
            )
            .unwrap();
        let mut temporary = TemporaryChatStore::default();
        let temporary_conversation = temporary_conversation(crate::database::ASTRA_ID);
        temporary.conversations.insert(
            crate::database::ASTRA_ID.into(),
            TemporaryConversation {
                conversation: temporary_conversation,
                messages: Vec::new(),
                model_override_ref: None,
            },
        );
        let temporary_chats = Mutex::new(temporary);

        assert_eq!(
            continue_temporary_in_database(&database, &temporary_chats, crate::database::ASTRA_ID,),
            Err("persistence_failed")
        );
        assert!(lock(&temporary_chats)
            .conversations
            .contains_key(crate::database::ASTRA_ID));
        drop(temporary_chats);
        drop(database);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn successful_temporary_conversion_marks_the_session_as_converted() {
        let path = std::env::temp_dir()
            .join(format!(
                "aip-temporary-conversion-success-{}",
                Uuid::now_v7()
            ))
            .join("aip.sqlite3");
        let database = Database::initialize(&path).unwrap();
        let mut temporary = TemporaryChatStore::default();
        temporary.conversations.insert(
            crate::database::ASTRA_ID.into(),
            TemporaryConversation {
                conversation: temporary_conversation(crate::database::ASTRA_ID),
                messages: Vec::new(),
                model_override_ref: None,
            },
        );
        let temporary_chats = Mutex::new(temporary);

        continue_temporary_in_database(&database, &temporary_chats, crate::database::ASTRA_ID)
            .unwrap();
        let mut chats = lock(&temporary_chats);
        assert!(!chats.conversations.contains_key(crate::database::ASTRA_ID));
        assert!(chats.converted_agents.contains(crate::database::ASTRA_ID));
        assert_eq!(
            ensure_temporary_chat(&mut chats, crate::database::ASTRA_ID).map(|_| ()),
            Err("operation_unavailable")
        );
        drop(chats);
        drop(temporary_chats);
        drop(database);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn closing_active_temporary_chat_releases_queue_and_ignores_late_events() {
        let mut temporary = TemporaryChatStore::default();
        let temporary_conversation = temporary_conversation("astra");
        temporary.conversations.insert(
            "astra".into(),
            TemporaryConversation {
                conversation: temporary_conversation.clone(),
                messages: vec![ConversationMessage {
                    id: "temporary-assistant".into(),
                    conversation_id: temporary_conversation.id.clone(),
                    agent_id: "astra".into(),
                    author: MessageAuthor::Agent,
                    content: "not persisted".into(),
                    model_ref: Some("ollama:test".into()),
                    status: MessageStatus::Streaming,
                    created_at: now_millis(),
                    completed_at: None,
                    error_code: None,
                    branch_id: temporary_conversation.id.clone(),
                    turn_group_id: "temporary-turn".into(),
                }],
                model_override_ref: None,
            },
        );
        let mut queue = GenerationQueue::default();
        let mut active = job("temporary-active", "astra");
        active.temporary = true;
        active.conversation_id = temporary_conversation.id.clone();
        active.assistant_message_id = "temporary-assistant".into();
        queue.enqueue(active.clone()).unwrap();
        queue.enqueue(job("next", "luma")).unwrap();
        assert_eq!(queue.activate_next(), Some(active.clone()));

        assert_eq!(
            queue.take_temporary_active("astra", &temporary_conversation.id),
            Some(active.clone())
        );
        queue.discard_temporary_pending("astra", &temporary_conversation.id);
        clear_temporary_chat(&mut temporary, "astra");
        assert!(temporary.conversations.is_empty());
        assert_eq!(queue.activate_next().unwrap().request_id, "next");

        let late_chunk = PhaseOneEvent {
            protocol_version: PROTOCOL_VERSION,
            event_type: "generation.chunk".into(),
            request_id: Some(active.request_id.clone()),
            agent_id: Some(active.agent_id.clone()),
            conversation_id: Some(active.conversation_id.clone()),
            assistant_message_id: Some(active.assistant_message_id.clone()),
            sequence: Some(1),
            content: Some("late".into()),
            error_code: None,
        };
        assert!(!queue.matches_event(&late_chunk));
        assert_eq!(
            queue.accept_chunk(&active.request_id, 1, 4),
            ChunkDecision::Ignored
        );
        let late_terminal = PhaseOneEvent {
            event_type: "generation.cancelled".into(),
            content: None,
            ..late_chunk
        };
        assert!(!queue.accepts_terminal(&late_terminal));

        temporary.conversations.insert(
            "astra".into(),
            TemporaryConversation {
                conversation: temporary_conversation.clone(),
                messages: Vec::new(),
                model_override_ref: None,
            },
        );
        assert!(temporary.conversations["astra"].messages.is_empty());

        let path = std::env::temp_dir()
            .join(format!("aip-temporary-close-{}", Uuid::now_v7()))
            .join("aip.sqlite3");
        let database = Database::initialize(&path).unwrap();
        let agent = database.agent(crate::database::ASTRA_ID).unwrap();
        assert!(database
            .messages(
                &agent.id,
                &database.main_conversation(&agent.id).unwrap().id
            )
            .unwrap()
            .is_empty());
        assert!(database
            .conversations(&agent.id)
            .unwrap()
            .iter()
            .all(|conversation| conversation.id != temporary_conversation.id));
        assert!(database.memories(&agent.id).unwrap().is_empty());
        drop(database);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn queued_cancel_never_activates_and_duplicate_is_rejected() {
        let mut queue = GenerationQueue::default();
        queue.enqueue(job("one", "astra")).unwrap();
        assert_eq!(queue.enqueue(job("one", "luma")), Err("duplicate_request"));
        assert_eq!(queue.cancel_queued("one").unwrap().agent_id, "astra");
        assert!(queue.activate_next().is_none());
    }

    #[test]
    fn active_terminal_race_has_one_winner() {
        let mut queue = GenerationQueue::default();
        queue.enqueue(job("one", "astra")).unwrap();
        queue.activate_next();
        assert!(queue.finish_active("one").is_some());
        assert!(queue.finish_active("one").is_none());
    }

    #[test]
    fn active_cancellation_is_idempotent_ignores_late_work_and_advances_fifo() {
        let mut queue = GenerationQueue::default();
        queue.enqueue(job("one", "astra")).unwrap();
        queue.enqueue(job("two", "luma")).unwrap();
        queue.activate_next();

        assert_eq!(
            queue.request_cancellation("wrong"),
            CancellationDecision::NotActive
        );
        assert_eq!(
            queue.request_cancellation("one"),
            CancellationDecision::Requested
        );
        assert_eq!(
            queue.request_cancellation("one"),
            CancellationDecision::AlreadyRequested
        );
        assert!(queue.snapshots()[0].cancellation_requested);
        assert_eq!(queue.accept_chunk("one", 1, 10), ChunkDecision::Ignored);
        let active = queue.active.as_ref().unwrap().job.clone();
        let cancelled = PhaseOneEvent {
            protocol_version: PROTOCOL_VERSION,
            event_type: "generation.cancelled".into(),
            request_id: Some(active.request_id.clone()),
            agent_id: Some(active.agent_id.clone()),
            conversation_id: Some(active.conversation_id.clone()),
            assistant_message_id: Some(active.assistant_message_id.clone()),
            sequence: Some(1),
            content: None,
            error_code: None,
        };
        assert!(queue.accepts_terminal(&cancelled));
        assert!(!queue.accepts_terminal(&PhaseOneEvent {
            event_type: "generation.complete".into(),
            ..cancelled.clone()
        }));
        assert!(queue.finish_active("one").is_some());
        assert_eq!(queue.activate_next().unwrap().request_id, "two");
        assert!(queue.accepts_terminal(&PhaseOneEvent {
            event_type: "generation.complete".into(),
            request_id: Some("two".into()),
            agent_id: Some("luma".into()),
            conversation_id: Some("conversation-luma".into()),
            assistant_message_id: Some("message-two".into()),
            sequence: Some(0),
            content: None,
            error_code: None,
            ..cancelled
        }));
    }

    #[test]
    fn stale_duplicate_and_out_of_order_chunks_are_ignored() {
        let mut queue = GenerationQueue::default();
        queue.enqueue(job("one", "astra")).unwrap();
        queue.activate_next();
        assert!(matches!(
            queue.accept_chunk("one", 1, 5),
            ChunkDecision::Accepted(_)
        ));
        assert_eq!(queue.accept_chunk("one", 1, 5), ChunkDecision::Ignored);
        assert_eq!(queue.accept_chunk("one", 3, 5), ChunkDecision::Ignored);
        assert_eq!(queue.accept_chunk("stale", 2, 5), ChunkDecision::Ignored);
        assert!(matches!(
            queue.accept_chunk("one", 2, 5),
            ChunkDecision::Accepted(_)
        ));
        let active = queue.active.as_ref().unwrap().job.clone();
        let matching = PhaseOneEvent {
            protocol_version: PROTOCOL_VERSION,
            event_type: "generation.complete".into(),
            request_id: Some(active.request_id.clone()),
            agent_id: Some(active.agent_id.clone()),
            conversation_id: Some(active.conversation_id.clone()),
            assistant_message_id: Some(active.assistant_message_id.clone()),
            sequence: Some(2),
            content: None,
            error_code: None,
        };
        assert!(queue.matches_event(&matching));
        assert!(!queue.matches_event(&PhaseOneEvent {
            request_id: Some("stale-session".into()),
            ..matching.clone()
        }));
        assert!(queue.accepts_terminal(&matching));
        assert!(!queue.accepts_terminal(&PhaseOneEvent {
            sequence: Some(1),
            ..matching.clone()
        }));
    }

    #[test]
    fn request_trace_is_bounded_and_content_free() {
        let mut traces = RequestTraceStore::default();
        traces.register(&job("request", "astra"));
        for index in 0..(MAX_REQUEST_TRACE_ENTRIES + 3) {
            traces.record("request", "chunk_persisted", Some(index as u64), None);
        }
        let entries = traces.entries("request");
        assert_eq!(entries.len(), MAX_REQUEST_TRACE_ENTRIES);
        assert_eq!(entries[0].sequence, Some(3));
        assert!(entries.iter().all(|entry| entry.code == "chunk_persisted"));
        for index in 0..(MAX_RETAINED_REQUEST_TRACES + 2) {
            let request_id = format!("request-{index}");
            traces.register(&job(&request_id, "astra"));
            traces.record(&request_id, "request_enqueued", None, None);
        }
        assert!(traces.entries("request").is_empty());
    }

    #[test]
    fn unregistered_trace_events_do_not_evict_persistable_history() {
        let path = std::env::temp_dir().join(format!(
            "aip-generation-trace-unregistered-{}.ndjson",
            Uuid::now_v7()
        ));
        let mut traces = RequestTraceStore::default();
        let durable = job("durable-trace-request", "astra");
        traces.register(&durable);
        traces.record(&durable.request_id, "database.finalized", None, None);
        for index in 0..(MAX_RETAINED_REQUEST_TRACES + 4) {
            let request_id = format!("unregistered-{index}");
            traces.record(&request_id, "chat.send.received", None, None);
        }
        for index in 0..(MAX_RETAINED_REQUEST_TRACES + 4) {
            let mut temporary = job(&format!("temporary-{index}"), "astra");
            temporary.temporary = true;
            traces.register(&temporary);
            traces.record(&temporary.request_id, "chat.send.received", None, None);
        }

        assert!(traces
            .entries(&durable.request_id)
            .iter()
            .any(|entry| { entry.code == "database.finalized" }));
        traces.persist(&path);
        assert!(fs::read_to_string(&path)
            .unwrap()
            .contains("durable-trace-request"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn request_trace_skips_streaming_chunks_until_a_milestone() {
        assert!(!should_persist_trace_code("chunk_persisted"));
        assert!(!should_persist_trace_code("generation.chunk"));
        assert!(!should_persist_trace_code("chunk_ignored"));
        assert!(should_persist_trace_code("generation.accepted"));
        assert!(should_persist_trace_code("terminal_persisted"));
    }

    #[test]
    fn request_trace_persistence_is_serialized_off_the_calling_thread() {
        let path = std::env::temp_dir().join(format!(
            "aip-async-generation-trace-{}.ndjson",
            Uuid::now_v7()
        ));
        let persistence = TracePersistence::new(path.clone());
        let mut traces = RequestTraceStore::default();
        let generation = job("async-trace-request", "astra");
        traces.register(&generation);
        traces.record("async-trace-request", "database.finalized", None, None);
        persistence.enqueue(traces);
        let deadline = Instant::now() + Duration::from_secs(2);
        while !path.exists() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(path.exists());
        assert!(fs::read_to_string(&path)
            .unwrap()
            .contains("async-trace-request"));
        drop(persistence);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn request_trace_persists_safe_correlated_metadata() {
        let path =
            std::env::temp_dir().join(format!("aip-generation-trace-{}.ndjson", Uuid::now_v7()));
        let mut traces = RequestTraceStore::default();
        let generation = job("trace-request", "astra");
        traces.register(&generation);
        traces.record("trace-request", "chat.finalized", Some(2), None);
        traces.persist(&path);
        let persisted = fs::read_to_string(&path).unwrap();
        assert!(persisted.contains("trace-request"));
        assert!(persisted.contains("conversation-astra"));
        assert!(persisted.contains("ollama:test"));
        assert!(!persisted.contains("Synthetic input"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn request_trace_persistence_retains_records_across_restarts() {
        let path = std::env::temp_dir().join(format!(
            "aip-generation-trace-restart-{}.ndjson",
            Uuid::now_v7()
        ));
        let previous = job("previous-trace-request", "astra");
        let mut first_session = RequestTraceStore::default();
        first_session.register(&previous);
        first_session.record(&previous.request_id, "database.finalized", Some(1), None);
        first_session.persist(&path);

        let mut restarted = RequestTraceStore::load(&path);
        assert_eq!(
            restarted.entries(&previous.request_id)[0].code,
            "database.finalized"
        );
        let current = job("current-trace-request", "astra");
        restarted.register(&current);
        restarted.record(&current.request_id, "database.finalized", Some(2), None);
        restarted.persist(&path);

        let persisted = fs::read_to_string(&path).unwrap();
        assert!(persisted.contains("previous-trace-request"));
        assert!(persisted.contains("current-trace-request"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn request_trace_does_not_persist_temporary_chat_metadata() {
        let path = std::env::temp_dir().join(format!(
            "aip-temporary-generation-trace-{}.ndjson",
            Uuid::now_v7()
        ));
        let mut traces = RequestTraceStore::default();
        let mut generation = job("temporary-trace-request", "astra");
        generation.temporary = true;
        traces.register(&generation);
        traces.record("temporary-trace-request", "chat.finalized", Some(2), None);
        traces.persist(&path);
        assert!(!path.exists() || fs::read_to_string(&path).unwrap().is_empty());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn request_trace_does_not_persist_unregistered_requests() {
        let path = std::env::temp_dir().join(format!(
            "aip-unregistered-generation-trace-{}.ndjson",
            Uuid::now_v7()
        ));
        let mut traces = RequestTraceStore::default();
        traces.record("unregistered-request", "chat.send.received", None, None);
        traces.persist(&path);
        assert!(!path.exists() || fs::read_to_string(&path).unwrap().is_empty());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn converted_temporary_session_is_tombstoned_until_explicit_restart() {
        let mut store = TemporaryChatStore::default();
        store.conversations.insert(
            "astra".into(),
            TemporaryConversation {
                conversation: temporary_conversation("astra"),
                messages: Vec::new(),
                model_override_ref: None,
            },
        );
        store.converted_agents.insert("astra".into());
        clear_temporary_chat(&mut store, "astra");
        assert!(store.converted_agents.contains("astra"));

        assert_eq!(
            ensure_temporary_chat(&mut store, "astra").map(|_| ()),
            Err("operation_unavailable")
        );

        store.converted_agents.remove("astra");
        assert!(ensure_temporary_chat(&mut store, "astra").is_ok());
        assert!(store.conversations.contains_key("astra"));
    }

    #[test]
    fn safe_clear_resolves_active_and_queued() {
        let mut queue = GenerationQueue::default();
        queue.enqueue(job("one", "astra")).unwrap();
        queue.enqueue(job("two", "luma")).unwrap();
        queue.activate_next();
        assert_eq!(queue.clear().len(), 2);
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn context_keeps_newest_complete_turns_within_byte_budget() {
        let messages = (0..40)
            .map(|index| ContextMessage {
                author: if index % 2 == 0 {
                    MessageAuthor::User
                } else {
                    MessageAuthor::Agent
                },
                content: format!("message-{index}"),
            })
            .collect();
        let prompt = assemble_context(
            &crate::domain::ProvisionalAgent {
                id: "astra".into(),
                name: "Astra".into(),
                profile_key: "owner".into(),
                sprite_key: "astra".into(),
                position: crate::domain::AgentPosition { x: 0.0, y: 0.0 },
                birthday: "2000-01-01".into(),
                fictive_age: 18,
                age_category: "adult".into(),
                species: "agent".into(),
                pronouns: "they/them".into(),
                gender: None,
                sexuality: None,
                personality_summary: "curious".into(),
                traits_json: r#"{"curiosity":80}"#.into(),
                appearance_preset: "astra".into(),
            },
            messages,
        );
        assert_eq!(prompt[0].role, "system");
        assert!(prompt.last().unwrap().content.ends_with("39"));
        assert!(
            prompt
                .iter()
                .map(|message| message.content.len())
                .sum::<usize>()
                <= MAX_CONTEXT_BYTES
        );
        assert_eq!(prompt.len(), MAX_HISTORY_MESSAGES + 1);
    }

    #[test]
    fn secondary_conversation_uses_its_own_context_and_override() {
        let path = std::env::temp_dir()
            .join(format!("aip-phase3-context-{}", Uuid::now_v7()))
            .join("aip.sqlite3");
        let database = Database::initialize(&path).unwrap();
        let agent = database.snapshot().unwrap().agents.remove(0);
        let secondary = database
            .create_conversation(&agent.id, "Secondary")
            .unwrap();
        database
            .set_conversation_override(
                &agent.id,
                &database.main_conversation(&agent.id).unwrap().id,
                Some("ollama:main"),
            )
            .unwrap();
        database
            .set_selected_model(&agent.id, "ollama:default")
            .unwrap();
        database
            .set_active_conversation(&agent.id, &secondary.id)
            .unwrap();
        let attempt = database
            .create_message_attempt(&agent.id, &secondary.id, "secondary-only", "ollama:default")
            .unwrap();
        database
            .mark_streaming(&attempt.assistant_message_id, &attempt.request_id)
            .unwrap();
        database
            .finish_assistant(
                &attempt.assistant_message_id,
                &attempt.request_id,
                MessageStatus::Complete,
                None,
            )
            .unwrap();
        let request = build_generation_request_from_database(
            &database,
            &job_from_attempt(&agent.id, &secondary.id, "ollama:default", &attempt),
        )
        .unwrap();
        assert!(request.contains("secondary-only"));
        assert!(!request.contains("ollama:main"));
        drop(database);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn edited_identity_enters_later_request_context_without_cross_agent_leakage() {
        let path = std::env::temp_dir()
            .join(format!("aip-identity-context-{}", Uuid::now_v7()))
            .join("aip.sqlite3");
        let database = Database::initialize(&path).unwrap();
        let mut astra = database.agent(crate::database::ASTRA_ID).unwrap();
        astra.name = "Nova Astra".into();
        astra.species = "fox".into();
        astra.pronouns = "ela/dela".into();
        astra.fictive_age = 42;
        astra.age_category = "custom".into();
        astra.personality_summary = "calma".into();
        astra.traits_json = r#"{"curiosity":80,"custom_focus":60}"#.into();
        database.update_profile(&astra).unwrap();
        let conversation = database.main_conversation(&astra.id).unwrap();
        let attempt = database
            .create_message_attempt(&astra.id, &conversation.id, "hello", "ollama:test")
            .unwrap();
        let request = build_generation_request_from_database(
            &database,
            &job_from_attempt(&astra.id, &conversation.id, "ollama:test", &attempt),
        )
        .unwrap();
        assert!(request.contains("Nova Astra"));
        assert!(request.contains("fox"));
        assert!(request.contains("custom_focus"));
        let luma = database.agent(crate::database::LUMA_ID).unwrap();
        let luma_conversation = database.main_conversation(&luma.id).unwrap();
        let luma_attempt = database
            .create_message_attempt(&luma.id, &luma_conversation.id, "hello", "ollama:test")
            .unwrap();
        let luma_request = build_generation_request_from_database(
            &database,
            &job_from_attempt(
                &luma.id,
                &luma_conversation.id,
                "ollama:test",
                &luma_attempt,
            ),
        )
        .unwrap();
        assert!(!luma_request.contains("Nova Astra"));
        drop(database);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn queued_generation_keeps_the_persisted_model_when_overrides_change() {
        let path = std::env::temp_dir()
            .join(format!("aip-model-freeze-{}", Uuid::now_v7()))
            .join("aip.sqlite3");
        let database = Database::initialize(&path).unwrap();
        let agent = database.agent(crate::database::ASTRA_ID).unwrap();
        let conversation = database.main_conversation(&agent.id).unwrap();
        let attempt = database
            .create_message_attempt(&agent.id, &conversation.id, "queued", "ollama:queued")
            .unwrap();
        database
            .set_conversation_override(&agent.id, &conversation.id, Some("ollama:changed"))
            .unwrap();
        let request = build_generation_request_from_database(
            &database,
            &job_from_attempt(&agent.id, &conversation.id, "ollama:queued", &attempt),
        )
        .unwrap();
        let payload: serde_json::Value = serde_json::from_str(&request).unwrap();
        assert_eq!(payload["params"]["model"], "queued");
        assert_eq!(
            database
                .messages(&agent.id, &conversation.id)
                .unwrap()
                .last()
                .unwrap()
                .model_ref
                .as_deref(),
            Some("ollama:queued")
        );
        drop(database);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn queue_snapshots_expose_one_active_generation() {
        let mut queue = GenerationQueue::default();
        queue.enqueue(job("one", "astra")).unwrap();
        queue.enqueue(job("two", "luma")).unwrap();
        queue.activate_next();
        let snapshots = queue.snapshots();
        assert!(snapshots[0].active);
        assert!(!snapshots[1].active);
        assert_eq!(snapshots[1].position, 1);
    }

    #[test]
    fn message_status_types_remain_serializable() {
        let message = ConversationMessage {
            id: "message".into(),
            conversation_id: "conversation".into(),
            agent_id: "agent".into(),
            author: MessageAuthor::Agent,
            content: String::new(),
            model_ref: Some("ollama:test".into()),
            status: MessageStatus::Pending,
            created_at: 1,
            completed_at: None,
            error_code: None,
            branch_id: "branch".into(),
            turn_group_id: "turn".into(),
        };
        assert!(serde_json::to_string(&message).unwrap().contains("pending"));
    }

    #[test]
    fn synthetic_persistent_generation_pipeline_survives_restart() {
        let path = std::env::temp_dir()
            .join(format!("aip-phase1-pipeline-{}", Uuid::now_v7()))
            .join("aip.sqlite3");
        let database = Database::initialize(&path).unwrap();
        let agent = database.snapshot().unwrap().agents.remove(0);
        let conversation = database.main_conversation(&agent.id).unwrap();
        let attempt = database
            .create_message_attempt(
                &agent.id,
                &conversation.id,
                "Synthetic input",
                "ollama:test",
            )
            .unwrap();
        let mut queue = GenerationQueue::default();
        queue
            .enqueue(job_from_attempt(
                &agent.id,
                &conversation.id,
                "ollama:test",
                &attempt,
            ))
            .unwrap();
        queue.activate_next();
        database
            .mark_streaming(&attempt.assistant_message_id, &attempt.request_id)
            .unwrap();
        for chunk in ["Synthetic ", "reply"] {
            database
                .append_assistant_chunk(&attempt.assistant_message_id, &attempt.request_id, chunk)
                .unwrap();
        }
        database
            .finish_assistant(
                &attempt.assistant_message_id,
                &attempt.request_id,
                MessageStatus::Complete,
                None,
            )
            .unwrap();
        assert!(queue.finish_active(&attempt.request_id).is_some());
        drop(database);

        let reopened = Database::initialize(&path).unwrap();
        let messages = reopened.messages(&agent.id, &conversation.id).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1].content, "Synthetic reply");
        assert_eq!(messages[1].status, MessageStatus::Complete);

        let cancelled = reopened
            .create_message_attempt(&agent.id, &conversation.id, "Cancel this", "ollama:test")
            .unwrap();
        reopened
            .finish_assistant(
                &cancelled.assistant_message_id,
                &cancelled.request_id,
                MessageStatus::Cancelled,
                None,
            )
            .unwrap();
        assert_eq!(
            reopened.messages(&agent.id, &conversation.id).unwrap()[3].status,
            MessageStatus::Cancelled
        );
        drop(reopened);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn persisted_history_uses_the_desktop_generation_request_shape() {
        let path = std::env::temp_dir()
            .join(format!("aip-phase1-request-{}", Uuid::now_v7()))
            .join("aip.sqlite3");
        let database = Database::initialize(&path).unwrap();
        let agent = database.snapshot().unwrap().agents.remove(0);
        let conversation = database.main_conversation(&agent.id).unwrap();

        let completed = database
            .create_message_attempt(
                &agent.id,
                &conversation.id,
                "Synthetic completed user",
                "ollama:llama3.2:1b",
            )
            .unwrap();
        database
            .mark_streaming(&completed.assistant_message_id, &completed.request_id)
            .unwrap();
        database
            .append_assistant_chunk(
                &completed.assistant_message_id,
                &completed.request_id,
                "Synthetic completed assistant",
            )
            .unwrap();
        database
            .finish_assistant(
                &completed.assistant_message_id,
                &completed.request_id,
                MessageStatus::Complete,
                None,
            )
            .unwrap();

        for status in [MessageStatus::Failed, MessageStatus::Cancelled] {
            let terminal = database
                .create_message_attempt(
                    &agent.id,
                    &conversation.id,
                    "Synthetic terminal user",
                    "ollama:llama3.2:1b",
                )
                .unwrap();
            database
                .finish_assistant(
                    &terminal.assistant_message_id,
                    &terminal.request_id,
                    status,
                    None,
                )
                .unwrap();
        }

        let fresh = database
            .create_message_attempt(
                &agent.id,
                &conversation.id,
                "Synthetic short user",
                "ollama:llama3.2:1b",
            )
            .unwrap();
        let job = job_from_attempt(&agent.id, &conversation.id, "ollama:llama3.2:1b", &fresh);
        let request = build_generation_request_from_database(&database, &job).unwrap();
        let payload: serde_json::Value = serde_json::from_str(&request).unwrap();
        let params = payload["params"].as_object().unwrap();
        let messages = params["messages"].as_array().unwrap();

        for model_ref in ["ollama:llama3.2:1b", "ollama:qwen2.5:7b"] {
            let request = build_generation_request_from_database(
                &database,
                &job_from_attempt(&agent.id, &conversation.id, model_ref, &fresh),
            )
            .unwrap();
            let payload: serde_json::Value = serde_json::from_str(&request).unwrap();
            assert_eq!(
                payload["params"]["model"],
                model_ref.trim_start_matches("ollama:")
            );
        }
        assert_eq!(messages.len(), 4);
        assert_eq!(messages.first().unwrap()["role"], "system");
        assert_eq!(
            messages
                .iter()
                .filter(|message| message["role"] == "assistant")
                .count(),
            1
        );
        assert_eq!(messages.last().unwrap()["role"], "user");
        drop(database);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn different_current_prompts_produce_different_runtime_payloads() {
        let agent = crate::domain::ProvisionalAgent {
            id: "agent".into(),
            name: "Astra".into(),
            profile_key: "owner".into(),
            sprite_key: "astra".into(),
            position: crate::domain::AgentPosition { x: 0.0, y: 0.0 },
            birthday: "2000-01-01".into(),
            fictive_age: 18,
            age_category: "adult".into(),
            species: "agent".into(),
            pronouns: "they/them".into(),
            personality_summary: String::new(),
            traits_json: "{}".into(),
            gender: None,
            sexuality: None,
            appearance_preset: "astra".into(),
        };
        let first = assemble_context(
            &agent,
            vec![ContextMessage {
                author: MessageAuthor::User,
                content: "Planeje uma viagem".into(),
            }],
        );
        let second = assemble_context(
            &agent,
            vec![ContextMessage {
                author: MessageAuthor::User,
                content: "Explique este erro".into(),
            }],
        );
        assert_ne!(
            first.last().map(|message| message.content.as_str()),
            second.last().map(|message| message.content.as_str())
        );
    }

    #[test]
    fn desktop_generation_requests_forward_distinct_current_prompts() {
        let path = std::env::temp_dir()
            .join(format!("aip-phase1-prompt-probe-{}", Uuid::now_v7()))
            .join("aip.sqlite3");
        let database = Database::initialize(&path).unwrap();
        let agent = database.snapshot().unwrap().agents.remove(0);
        let mut payloads = Vec::new();
        for prompt in [
            "Responda apenas com: AZUL-17",
            "Responda apenas com: VERDE-93",
        ] {
            let conversation = database
                .create_conversation(&agent.id, "Nova conversa")
                .unwrap();
            let attempt = database
                .create_message_attempt(&agent.id, &conversation.id, prompt, "ollama:llama3.2:1b")
                .unwrap();
            let job = job_from_attempt(&agent.id, &conversation.id, "ollama:llama3.2:1b", &attempt);
            let request = build_generation_request_from_database(&database, &job).unwrap();
            let body: serde_json::Value = serde_json::from_str(&request).unwrap();
            payloads.push(
                body["params"]["messages"]
                    .as_array()
                    .unwrap()
                    .last()
                    .unwrap()["content"]
                    .as_str()
                    .unwrap()
                    .to_string(),
            );
        }
        assert_eq!(
            payloads,
            [
                "Responda apenas com: AZUL-17",
                "Responda apenas com: VERDE-93",
            ]
        );
        assert_ne!(payloads[0], payloads[1]);
        drop(database);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn coordinator_runtime_dispatch_isolates_sequential_prompts() {
        let _runtime_guard = crate::runtime::RUNTIME_TEST_LOCK.lock().unwrap();
        let path = std::env::temp_dir()
            .join(format!("aip-coordinator-runtime-{}", Uuid::now_v7()))
            .join("aip.sqlite3");
        let database = Database::initialize(&path).unwrap();
        let agent = database.agent(crate::database::ASTRA_ID).unwrap();
        let conversation = database.main_conversation(&agent.id).unwrap();
        database
            .set_selected_model(&agent.id, "ollama:test")
            .unwrap();

        let source_root = coordinator_fixture_source_root();
        let runtime = RuntimeController::new(source_root.clone(), false);
        let coordinator = ChatCoordinator::new_for_test(
            database.clone(),
            runtime.clone(),
            Arc::new(AtomicBool::new(false)),
            Arc::new(Mutex::new(OrchestrationManager::default())),
        );
        let provider = ProviderSnapshot {
            state: ProviderState::Checking,
            detail_code: "provider_checking".into(),
            models: vec![OllamaModel {
                model_ref: "ollama:test".into(),
                provider_model_id: "test".into(),
                display_name: "Test".into(),
                size: 1,
                family: None,
                parameter_size: None,
                quantization: None,
                capabilities: vec![],
            }],
            refreshed_at: Some(now_millis()),
        };
        coordinator.sync_orchestration(&provider);
        *lock(&coordinator.inner.provider) = provider;
        runtime.start();
        wait_for_runtime_state(&runtime, RuntimeState::Ready);
        // HealthReady triggers provider discovery; the initial Checking state makes
        // completion observable without depending on the transient state timing.
        wait_for_provider_state(&coordinator, ProviderState::Available);

        let first = coordinator
            .send_message(&agent.id, &conversation.id, "Responda apenas com AZUL-17")
            .unwrap_or_else(|error| {
                panic!(
                    "first dispatch failed: {error}; runtime={:?}; diagnostics={:?}; provider={:?}",
                    runtime.snapshot(),
                    runtime.diagnostics(),
                    coordinator.provider_snapshot(),
                )
            });
        wait_for_message_status(
            &database,
            &agent.id,
            &conversation.id,
            &first.assistant_message_id,
            MessageStatus::Complete,
        );
        wait_for_request_trace(&coordinator, &first.request_id, "terminal_persisted");
        let first_message = database
            .messages(&agent.id, &conversation.id)
            .unwrap()
            .into_iter()
            .find(|message| message.id == first.assistant_message_id)
            .unwrap();
        assert_eq!(first_message.content, "AZUL-17");

        let second = coordinator
            .send_message(&agent.id, &conversation.id, "Responda apenas com VERDE-93")
            .unwrap();
        wait_for_message_status(
            &database,
            &agent.id,
            &conversation.id,
            &second.assistant_message_id,
            MessageStatus::Complete,
        );
        let second_message = database
            .messages(&agent.id, &conversation.id)
            .unwrap()
            .into_iter()
            .find(|message| message.id == second.assistant_message_id)
            .unwrap();
        assert_eq!(second_message.content, "VERDE-93");
        assert_ne!(first_message.content, second_message.content);

        for request_id in [&first.request_id, &second.request_id] {
            wait_for_request_trace(&coordinator, request_id, "runtime_accepted");
            wait_for_request_trace(&coordinator, request_id, "terminal_persisted");
            let trace = lock(&coordinator.inner.request_traces).entries(request_id);
            let codes = trace
                .iter()
                .map(|entry| entry.code.as_str())
                .collect::<Vec<_>>();
            assert!(codes.contains(&"request_written"));
            assert!(codes.contains(&"runtime_accepted"));
            assert!(codes.contains(&"generation_started"));
            assert!(codes.contains(&"terminal_persisted"));
        }

        runtime.shutdown();
        drop(database);
        let _ = fs::remove_dir_all(path.parent().unwrap());
        let _ = fs::remove_dir_all(source_root);
    }

    fn coordinator_fixture_source_root() -> PathBuf {
        const RUNTIME: &str = r#"
import json
import sys

def write(value):
    sys.stdout.write(json.dumps(value, separators=(",", ":")) + "\n")
    sys.stdout.flush()

def event(request, kind, sequence, content=None):
    value = {
        "protocolVersion": 1,
        "event": kind,
        "requestId": request["id"],
        "agentId": request["params"]["agentId"],
        "conversationId": request["params"]["conversationId"],
        "assistantMessageId": request["params"]["assistantMessageId"],
        "sequence": sequence,
    }
    if content is not None:
        value["content"] = content
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
        write({"protocolVersion": 1, "id": request["id"], "result": {"provider": "ollama", "state": "available", "models": [{"ref": "ollama:test", "providerModelId": "test", "displayName": "Test", "size": 1, "capabilities": []}]}})
    elif method == "generation.start":
        write({"protocolVersion": 1, "id": request["id"], "result": {"status": "accepted"}})
        event(request, "generation.started", 0)
        prompt = request["params"]["messages"][-1]["content"]
        marker = "AZUL-17" if "AZUL-17" in prompt else ("VERDE-93" if "VERDE-93" in prompt else "TITULO")
        event(request, "generation.chunk", 1, marker)
        event(request, "generation.complete", 1)
    elif method == "generation.cancel":
        write({"protocolVersion": 1, "id": request["id"], "error": {"code": "generation_not_active"}})
"#;
        let root = std::env::temp_dir().join(format!(
            "aip-coordinator-runtime-fixture-{}",
            Uuid::now_v7()
        ));
        let package = root.join("aip_runtime");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("__init__.py"), "").unwrap();
        fs::write(package.join("__main__.py"), RUNTIME).unwrap();
        root
    }

    fn wait_for_runtime_state(runtime: &RuntimeController, expected: RuntimeState) {
        // CI Windows runners can cold-start the Python runtime well beyond the
        // interactive local path; keep this assertion bounded without making
        // the test sensitive to that startup variance.
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            if runtime.snapshot().state == expected {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("runtime did not reach expected state");
    }

    fn wait_for_message_status(
        database: &Database,
        agent_id: &str,
        conversation_id: &str,
        message_id: &str,
        expected: MessageStatus,
    ) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if database
                .messages(agent_id, conversation_id)
                .unwrap()
                .iter()
                .any(|message| message.id == message_id && message.status == expected)
            {
                return;
            }
            thread::sleep(Duration::from_millis(25));
        }
        panic!("message did not reach expected status");
    }

    fn wait_for_provider_state(coordinator: &ChatCoordinator, expected: ProviderState) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if coordinator.provider_snapshot().state == expected {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!(
            "provider did not reach expected state {expected:?}; actual={:?}",
            coordinator.provider_snapshot().state
        );
    }

    fn wait_for_request_trace(
        coordinator: &ChatCoordinator,
        request_id: &str,
        expected_code: &'static str,
    ) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if lock(&coordinator.inner.request_traces)
                .entries(request_id)
                .iter()
                .any(|entry| entry.code == expected_code)
            {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("request trace did not reach expected state");
    }

    #[test]
    fn generated_title_output_is_bounded_and_plain() {
        assert_eq!(
            sanitize_generated_title(" Título: \"Planejamento do litoral\"\n"),
            Some("Planejamento do litoral".into())
        );
        assert_eq!(
            sanitize_generated_title("“Planejamento do litoral”"),
            Some("Planejamento do litoral".into())
        );
        assert_eq!(sanitize_generated_title("\n\t"), None);
        assert!(
            sanitize_generated_title(&"x".repeat(100))
                .unwrap()
                .chars()
                .count()
                <= 64
        );
    }

    #[test]
    #[ignore = "requires a local Ollama llama3.2:1b model"]
    fn desktop_equivalent_persisted_generation_reaches_runtime_and_persists() {
        let _runtime_guard = crate::runtime::RUNTIME_TEST_LOCK.lock().unwrap();
        let path = std::env::temp_dir()
            .join(format!("aip-phase1-provider-probe-{}", Uuid::now_v7()))
            .join("aip.sqlite3");
        let database = Database::initialize(&path).unwrap();
        let agent = database.snapshot().unwrap().agents.remove(0);
        let conversation = database.main_conversation(&agent.id).unwrap();
        let model = std::env::var("AIP_OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2:1b".into());
        let model_ref = format!("ollama:{model}");
        let completed = database
            .create_message_attempt(
                &agent.id,
                &conversation.id,
                "Synthetic completed user",
                &model_ref,
            )
            .unwrap();
        database
            .mark_streaming(&completed.assistant_message_id, &completed.request_id)
            .unwrap();
        database
            .append_assistant_chunk(
                &completed.assistant_message_id,
                &completed.request_id,
                "Synthetic completed assistant",
            )
            .unwrap();
        database
            .finish_assistant(
                &completed.assistant_message_id,
                &completed.request_id,
                MessageStatus::Complete,
                None,
            )
            .unwrap();
        for status in [MessageStatus::Failed, MessageStatus::Cancelled] {
            let terminal = database
                .create_message_attempt(
                    &agent.id,
                    &conversation.id,
                    "Synthetic terminal user",
                    &model_ref,
                )
                .unwrap();
            database
                .finish_assistant(
                    &terminal.assistant_message_id,
                    &terminal.request_id,
                    status,
                    None,
                )
                .unwrap();
        }
        let fresh = database
            .create_message_attempt(&agent.id, &conversation.id, "ping", &model_ref)
            .unwrap();
        let job = job_from_attempt(&agent.id, &conversation.id, &model_ref, &fresh);
        let request = build_generation_request_from_database(&database, &job).unwrap();
        database
            .mark_streaming(&fresh.assistant_message_id, &fresh.request_id)
            .unwrap();

        let source_root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../services/runtime/src");
        let runtime = RuntimeController::new(source_root, false);
        let receiver = runtime.subscribe();
        runtime.start();
        wait_for_runtime_ready(&receiver);
        runtime.send(request).unwrap();

        let mut chunks = 0;
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let notice = receiver.recv_timeout(remaining).unwrap();
            let RuntimeNotice::Output(RuntimeOutput::Event(event)) = notice else {
                continue;
            };
            if event.request_id.as_deref() != Some(fresh.request_id.as_str()) {
                continue;
            }
            match event.event_type.as_str() {
                "generation.chunk" => {
                    chunks += 1;
                    database
                        .append_assistant_chunk(
                            &fresh.assistant_message_id,
                            &fresh.request_id,
                            event.content.as_deref().unwrap(),
                        )
                        .unwrap();
                }
                "generation.complete" => {
                    database
                        .finish_assistant(
                            &fresh.assistant_message_id,
                            &fresh.request_id,
                            MessageStatus::Complete,
                            None,
                        )
                        .unwrap();
                    break;
                }
                "generation.failed" | "generation.cancelled" => {
                    panic!("desktop-equivalent provider probe did not complete")
                }
                _ => {}
            }
        }
        runtime.shutdown();
        assert!(chunks > 0);
        let messages = database.messages(&agent.id, &conversation.id).unwrap();
        assert_eq!(messages.last().unwrap().status, MessageStatus::Complete);
        drop(database);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    fn wait_for_runtime_ready(receiver: &mpsc::Receiver<RuntimeNotice>) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if matches!(
                receiver.recv_timeout(Duration::from_millis(100)),
                Ok(RuntimeNotice::Output(RuntimeOutput::HealthReady { .. }))
            ) {
                return;
            }
        }
        panic!("desktop-equivalent runtime did not become ready");
    }
}
