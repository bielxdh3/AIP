use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};

use tauri::{AppHandle, Emitter};

use crate::{
    database::{now_millis, ContextMessage, Database, MessageAttempt},
    domain::{
        MessageAuthor, MessageStatus, PhaseOneEvent, PhaseOneState, ProviderSnapshot,
        ProviderState, QueueEntrySnapshot, RuntimeState, SendMessageResult,
        MAX_ASSISTANT_OUTPUT_BYTES, MAX_CONTEXT_BYTES, MAX_HISTORY_MESSAGES, MAX_QUEUE_LENGTH,
        MAX_USER_MESSAGE_BYTES,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct GenerationJob {
    request_id: String,
    agent_id: String,
    conversation_id: String,
    assistant_message_id: String,
    model_ref: String,
    temporary: bool,
}

#[derive(Debug, Default)]
struct TemporaryChatStore {
    conversations: HashMap<String, TemporaryConversation>,
}

#[derive(Debug, Clone)]
struct TemporaryConversation {
    conversation: crate::domain::PhaseOneConversation,
    messages: Vec<crate::domain::ConversationMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveGeneration {
    job: GenerationJob,
    last_sequence: u64,
    output_bytes: usize,
    cancellation_requested: bool,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct RequestTraceEntry {
    code: &'static str,
    sequence: Option<u64>,
    terminal_code: Option<String>,
}

#[derive(Debug, Default)]
struct RequestTraceStore {
    order: VecDeque<String>,
    entries: HashMap<String, VecDeque<RequestTraceEntry>>,
}

impl RequestTraceStore {
    fn record(
        &mut self,
        request_id: &str,
        code: &'static str,
        sequence: Option<u64>,
        terminal_code: Option<&str>,
    ) {
        if !self.entries.contains_key(request_id) {
            if self.order.len() == MAX_RETAINED_REQUEST_TRACES {
                if let Some(oldest) = self.order.pop_front() {
                    self.entries.remove(&oldest);
                }
            }
            self.order.push_back(request_id.to_string());
            self.entries.insert(request_id.to_string(), VecDeque::new());
        }
        let trace = self
            .entries
            .get_mut(request_id)
            .expect("request trace exists");
        if trace.len() == MAX_REQUEST_TRACE_ENTRIES {
            trace.pop_front();
        }
        trace.push_back(RequestTraceEntry {
            code,
            sequence,
            terminal_code: terminal_code.map(str::to_string),
        });
    }

    #[cfg(test)]
    fn entries(&self, request_id: &str) -> Vec<RequestTraceEntry> {
        self.entries
            .get(request_id)
            .map(|entries| entries.iter().cloned().collect())
            .unwrap_or_default()
    }
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

    fn accepts_terminal(&self, event: &PhaseOneEvent) -> bool {
        self.active.as_ref().is_some_and(|active| {
            self.matches_event(event) && event.sequence == Some(active.last_sequence)
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
        if active.job.request_id != request_id
            || active.cancellation_requested
            || sequence != active.last_sequence + 1
        {
            return ChunkDecision::Ignored;
        }
        let next_size = active.output_bytes.saturating_add(content_bytes);
        if next_size > MAX_ASSISTANT_OUTPUT_BYTES {
            return ChunkDecision::OutputLimitExceeded;
        }
        active.last_sequence = sequence;
        active.output_bytes = next_size;
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
    app: AppHandle,
    database: Database,
    runtime: RuntimeController,
    safe_mode: Arc<AtomicBool>,
    provider: Mutex<ProviderSnapshot>,
    discovery_requests: Mutex<HashSet<String>>,
    model_detail_requests: Mutex<HashMap<String, String>>,
    send_lock: Mutex<()>,
    queue: Mutex<GenerationQueue>,
    request_traces: Mutex<RequestTraceStore>,
    temporary_chats: Mutex<TemporaryChatStore>,
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
    ) -> Self {
        let receiver = runtime.subscribe();
        let coordinator = Self {
            inner: Arc::new(ChatInner {
                app,
                database,
                runtime,
                safe_mode,
                provider: Mutex::new(ProviderSnapshot::checking()),
                discovery_requests: Mutex::new(HashSet::new()),
                model_detail_requests: Mutex::new(HashMap::new()),
                send_lock: Mutex::new(()),
                queue: Mutex::new(GenerationQueue::default()),
                request_traces: Mutex::new(RequestTraceStore::default()),
                temporary_chats: Mutex::new(TemporaryChatStore::default()),
            }),
        };
        let listener = coordinator.clone();
        thread::spawn(move || {
            while let Ok(notice) = receiver.recv() {
                listener.handle_notice(notice);
            }
        });
        coordinator
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
        let selected_model_available = selected_model_ref.as_ref().is_some_and(|selected| {
            provider
                .models
                .iter()
                .any(|model| &model.model_ref == selected)
        });
        let queue = lock(&self.inner.queue).snapshots();
        let blocked = self.send_blocked_code(
            &provider,
            selected_model_ref.as_deref(),
            selected_model_available,
            queue.len(),
        );
        Ok(PhaseOneState {
            agent,
            conversation,
            messages,
            provider,
            selected_model_ref,
            default_model_ref,
            model_override_ref,
            selected_model_available,
            keep_alive_minutes: settings.keep_alive_minutes,
            queue,
            can_send: blocked.is_none(),
            send_blocked_code: blocked.map(str::to_string),
        })
    }

    pub fn refresh_models(&self) -> Result<(), &'static str> {
        if self.inner.safe_mode.load(Ordering::SeqCst) {
            return Err("operation_unavailable");
        }
        if self.inner.runtime.snapshot().state != RuntimeState::Ready {
            *lock(&self.inner.provider) = ProviderSnapshot::unavailable("runtime_unavailable");
            self.emit_refresh(None);
            return Err("runtime_unavailable");
        }
        *lock(&self.inner.provider) = ProviderSnapshot::checking();
        let request_id = format!("discover-{}", uuid::Uuid::now_v7());
        lock(&self.inner.discovery_requests).insert(request_id.clone());
        let request = discovery_request(&request_id).map_err(|_| "operation_failed")?;
        if let Err(error) = self.inner.runtime.send(request) {
            lock(&self.inner.discovery_requests).remove(&request_id);
            *lock(&self.inner.provider) = ProviderSnapshot::unavailable(error);
            self.emit_refresh(None);
            return Err(error);
        }
        self.emit_refresh(None);
        Ok(())
    }

    pub fn temporary_state(&self, agent_id: &str) -> Result<PhaseOneState, &'static str> {
        let mut state = self.state(agent_id)?;
        let conversation = temporary_conversation(agent_id);
        state.conversation = conversation.clone();
        state.messages = lock(&self.inner.temporary_chats)
            .conversations
            .get(agent_id)
            .map(|chat| chat.messages.clone())
            .unwrap_or_default();
        state.model_override_ref = None;
        state.selected_model_ref = state.default_model_ref.clone();
        state.selected_model_available =
            state.selected_model_ref.as_ref().is_some_and(|selected| {
                state
                    .provider
                    .models
                    .iter()
                    .any(|model| &model.model_ref == selected)
            });
        state.send_blocked_code = self
            .send_blocked_code(
                &state.provider,
                state.selected_model_ref.as_deref(),
                state.selected_model_available,
                state.queue.len(),
            )
            .map(str::to_string);
        state.can_send = state.send_blocked_code.is_none();
        Ok(state)
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

    pub fn set_main_override(
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
        self.inner
            .database
            .set_main_conversation_override(agent_id, model_ref)
            .map_err(|_| "operation_failed")?;
        self.emit_refresh(Some(agent_id));
        Ok(())
    }

    pub fn send_message(
        &self,
        agent_id: &str,
        conversation_id: &str,
        content: &str,
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
            return Err(match code {
                "queue_full" => "queue_full",
                "safe_mode_active" => "safe_mode_active",
                "model_not_selected" | "selected_model_unavailable" => "model_unavailable",
                _ => "runtime_unavailable",
            });
        }
        let model_ref = state.selected_model_ref.ok_or("model_unavailable")?;
        let attempt = self
            .inner
            .database
            .create_message_attempt(agent_id, conversation_id, content, &model_ref)
            .map_err(|_| "operation_failed")?;
        let job = job_from_attempt(agent_id, conversation_id, &model_ref, &attempt);
        if let Err(code) = lock(&self.inner.queue).enqueue(job.clone()) {
            let _ = self.finish_job(&job, MessageStatus::Failed, Some(code));
            return Err(code);
        }
        self.trace(&attempt.request_id, "request_enqueued", None, None);
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

    pub fn send_temporary_message(
        &self,
        agent_id: &str,
        content: &str,
    ) -> Result<SendMessageResult, &'static str> {
        let _send_guard = lock(&self.inner.send_lock);
        if content.is_empty() || content.len() > MAX_USER_MESSAGE_BYTES {
            return Err("invalid_message");
        }
        let state = self.temporary_state(agent_id)?;
        if let Some(code) = state.send_blocked_code.as_deref() {
            return Err(match code {
                "queue_full" => "queue_full",
                "safe_mode_active" => "safe_mode_active",
                "model_not_selected" | "selected_model_unavailable" => "model_unavailable",
                _ => "runtime_unavailable",
            });
        }
        let model_ref = state.selected_model_ref.ok_or("model_unavailable")?;
        let now = now_millis();
        let request_id = uuid::Uuid::now_v7().to_string();
        let user_message_id = uuid::Uuid::now_v7().to_string();
        let assistant_message_id = uuid::Uuid::now_v7().to_string();
        let conversation = temporary_conversation(agent_id);
        {
            let mut chats = lock(&self.inner.temporary_chats);
            let chat = chats
                .conversations
                .entry(agent_id.to_string())
                .or_insert_with(|| TemporaryConversation {
                    conversation: conversation.clone(),
                    messages: Vec::new(),
                });
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
            });
        }
        let job = GenerationJob {
            request_id: request_id.clone(),
            agent_id: agent_id.into(),
            conversation_id: conversation.id.clone(),
            assistant_message_id: assistant_message_id.clone(),
            model_ref,
            temporary: true,
        };
        if let Err(code) = lock(&self.inner.queue).enqueue(job.clone()) {
            let _ = self.finish_temporary(&job, MessageStatus::Failed, Some(code));
            return Err(code);
        }
        self.trace(&request_id, "temporary_request_enqueued", None, None);
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
        Ok(())
    }

    pub fn cancel_all(&self, error_code: &'static str) {
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
    }

    pub fn retry_runtime(&self) {
        if !self.inner.safe_mode.load(Ordering::SeqCst) {
            self.inner.runtime.start();
            *lock(&self.inner.provider) = ProviderSnapshot::checking();
            self.emit_refresh(None);
        }
    }

    fn send_blocked_code(
        &self,
        provider: &ProviderSnapshot,
        selected_model: Option<&str>,
        selected_available: bool,
        queue_length: usize,
    ) -> Option<&'static str> {
        if self.inner.safe_mode.load(Ordering::SeqCst) {
            Some("safe_mode_active")
        } else if self.inner.runtime.snapshot().state != RuntimeState::Ready {
            Some("runtime_unavailable")
        } else if provider.state == ProviderState::Checking {
            Some("provider_checking")
        } else if provider.state == ProviderState::Empty {
            Some("provider_empty")
        } else if provider.state != ProviderState::Available {
            Some("provider_unavailable")
        } else if selected_model.is_none() {
            Some("model_not_selected")
        } else if !selected_available {
            Some("selected_model_unavailable")
        } else if queue_length >= MAX_QUEUE_LENGTH {
            Some("queue_full")
        } else {
            None
        }
    }

    fn handle_notice(&self, notice: RuntimeNotice) {
        match notice {
            RuntimeNotice::Output(RuntimeOutput::Provider { id, mut snapshot }) => {
                if lock(&self.inner.discovery_requests).remove(&id) {
                    snapshot.refreshed_at = Some(now_millis());
                    *lock(&self.inner.provider) = snapshot;
                    self.emit_refresh(None);
                }
            }
            RuntimeNotice::Output(RuntimeOutput::Error { id, code }) => {
                if lock(&self.inner.discovery_requests).remove(&id) {
                    *lock(&self.inner.provider) = provider_error_snapshot(&code);
                    self.emit_refresh(None);
                } else if lock(&self.inner.model_detail_requests)
                    .remove(&id)
                    .is_none()
                {
                    self.trace(&id, "request_error", None, Some(&code));
                    self.fail_active(&id, &code);
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
            RuntimeNotice::Disconnected { detail_code } => {
                *lock(&self.inner.provider) = ProviderSnapshot::unavailable(detail_code);
                self.fail_all("runtime_interrupted");
                self.emit_refresh(None);
            }
            RuntimeNotice::Output(RuntimeOutput::HealthReady { .. }) => {
                let _ = self.refresh_models();
            }
            RuntimeNotice::Output(RuntimeOutput::Accepted { .. }) => {}
        }
    }

    fn handle_generation_event(&self, event: PhaseOneEvent) {
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
                if !lock(&self.inner.queue).accepts_started(&event) {
                    self.trace(&request_id, "started_ignored", event.sequence, None);
                    return;
                }
                self.trace(&request_id, "generation_started", event.sequence, None);
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
                    self.trace(&request_id, "chunk_persisted", Some(sequence), None);
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
                self.finish_runtime_terminal(&request_id, MessageStatus::Complete, None, event)
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
                self.finish_runtime_terminal(
                    &request_id,
                    MessageStatus::Failed,
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
                self.finish_runtime_terminal(&request_id, MessageStatus::Cancelled, None, event)
            }
            _ => {}
        }
    }

    fn finish_runtime_terminal(
        &self,
        request_id: &str,
        status: MessageStatus,
        error_code: Option<&str>,
        event: PhaseOneEvent,
    ) {
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
            self.trace(request_id, "terminal_persisted", event.sequence, error_code);
            self.emit(event);
        } else {
            self.trace(request_id, "persistence_failed", event.sequence, None);
            self.emit_terminal(&job, "generation.failed", Some("persistence_failed"));
        }
        self.dispatch_next();
    }

    fn fail_active(&self, request_id: &str, code: &str) {
        let Some(job) = lock(&self.inner.queue).finish_active(request_id) else {
            return;
        };
        self.trace(request_id, "queue_finalized", None, Some(code));
        let _ = self.finish_job(&job, MessageStatus::Failed, Some(code));
        self.emit_terminal(&job, "generation.failed", Some(code));
        self.dispatch_next();
    }

    fn fail_all(&self, code: &str) {
        let jobs = lock(&self.inner.queue).clear();
        for job in jobs {
            let _ = self.finish_job(&job, MessageStatus::Failed, Some(code));
            self.emit_terminal(&job, "generation.failed", Some(code));
        }
    }

    fn dispatch_next(&self) {
        loop {
            if self.inner.runtime.snapshot().state != RuntimeState::Ready
                || self.inner.safe_mode.load(Ordering::SeqCst)
            {
                return;
            }
            let Some(job) = lock(&self.inner.queue).activate_next() else {
                return;
            };
            self.trace(&job.request_id, "queue_activated", None, None);
            let dispatch = self.build_generation_request(&job).and_then(|request| {
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
            &assemble_context(&agent.name, &agent.profile_key, messages),
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

    fn emit(&self, event: PhaseOneEvent) {
        if let Some(agent_id) = event.agent_id.as_deref() {
            let _ = self.inner.app.emit_to("main", EVENT_NAME, event.clone());
            if let Some(label) = overlays::window_label(agent_id) {
                let _ = self.inner.app.emit_to(label, EVENT_NAME, event.clone());
            }
            if let Some(label) = overlays::bubble_window_label(agent_id) {
                let _ = self.inner.app.emit_to(label, EVENT_NAME, event);
            }
        } else {
            let _ = self.inner.app.emit(EVENT_NAME, event);
        }
    }

    fn trace(
        &self,
        request_id: &str,
        code: &'static str,
        sequence: Option<u64>,
        terminal_code: Option<&str>,
    ) {
        lock(&self.inner.request_traces).record(request_id, code, sequence, terminal_code);
    }
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
    let conversation = database
        .conversation(&job.agent_id, &job.conversation_id)
        .map_err(|_| "persistence_failed")?;
    let context = database
        .context_messages(&job.agent_id, &job.conversation_id, MAX_HISTORY_MESSAGES)
        .map_err(|_| "persistence_failed")?;
    let messages = assemble_context(&agent.name, &agent.profile_key, context);
    let model_ref = conversation
        .model_override_ref
        .as_deref()
        .unwrap_or(&job.model_ref);
    let provider_model_id = model_ref
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
        assistant_message_id: attempt.assistant_message_id.clone(),
        model_ref: model_ref.to_string(),
        temporary: false,
    }
}

fn temporary_conversation(agent_id: &str) -> crate::domain::PhaseOneConversation {
    crate::domain::PhaseOneConversation {
        id: format!("temporary-{agent_id}"),
        agent_id: agent_id.to_string(),
        title: "Conversa temporária".into(),
        model_override_ref: None,
    }
}

fn assemble_context(
    agent_name: &str,
    profile_key: &str,
    messages: Vec<ContextMessage>,
) -> Vec<PromptMessage> {
    let instruction = format!(
        "Você é {agent_name}, um agente provisório local do perfil {profile_key}. Responda em português por padrão. Não afirme ter executado ações externas."
    );
    let mut used = instruction.len();
    let mut selected = Vec::new();
    for message in messages.into_iter().rev() {
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
        sync::mpsc,
        time::{Duration, Instant},
    };

    use crate::domain::ConversationMessage;
    use uuid::Uuid;

    use super::*;

    fn job(id: &str, agent: &str) -> GenerationJob {
        GenerationJob {
            request_id: id.into(),
            agent_id: agent.into(),
            conversation_id: format!("conversation-{agent}"),
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
    fn temporary_conversations_are_distinct_from_persisted_conversation_ids() {
        let astra = temporary_conversation("astra");
        let luma = temporary_conversation("luma");
        assert_eq!(astra.id, "temporary-astra");
        assert_eq!(luma.id, "temporary-luma");
        assert_ne!(astra.id, luma.id);
        assert_eq!(astra.model_override_ref, None);
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
            sequence: Some(0),
            content: None,
            error_code: None,
        };
        assert!(queue.accepts_terminal(&cancelled));
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
        for index in 0..(MAX_REQUEST_TRACE_ENTRIES + 3) {
            traces.record("request", "chunk_persisted", Some(index as u64), None);
        }
        let entries = traces.entries("request");
        assert_eq!(entries.len(), MAX_REQUEST_TRACE_ENTRIES);
        assert_eq!(entries[0].sequence, Some(3));
        assert!(entries.iter().all(|entry| entry.code == "chunk_persisted"));
        for index in 0..(MAX_RETAINED_REQUEST_TRACES + 2) {
            traces.record(&format!("request-{index}"), "request_enqueued", None, None);
        }
        assert!(traces.entries("request").is_empty());
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
        let prompt = assemble_context("Astra", "owner", messages);
        assert_eq!(prompt[0].role, "system");
        assert!(prompt.last().unwrap().content.ends_with("39"));
        assert!(
            prompt
                .iter()
                .map(|message| message.content.len())
                .sum::<usize>()
                <= MAX_CONTEXT_BYTES
        );
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
            .set_main_conversation_override(&agent.id, Some("ollama:main"))
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
    #[ignore = "requires a local Ollama llama3.2:1b model"]
    fn desktop_equivalent_persisted_generation_reaches_runtime_and_persists() {
        let path = std::env::temp_dir()
            .join(format!("aip-phase1-provider-probe-{}", Uuid::now_v7()))
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
            .create_message_attempt(&agent.id, &conversation.id, "ping", "ollama:llama3.2:1b")
            .unwrap();
        let job = job_from_attempt(&agent.id, &conversation.id, "ollama:llama3.2:1b", &fresh);
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
        let deadline = Instant::now() + Duration::from_secs(30);
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
