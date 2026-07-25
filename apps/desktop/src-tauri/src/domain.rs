use serde::{Deserialize, Serialize};

pub const MAX_USER_MESSAGE_BYTES: usize = 16_384;
pub const MAX_HISTORY_MESSAGES: usize = 32;
pub const MAX_CONTEXT_BYTES: usize = 49_152;
pub const MAX_STREAM_CHUNK_BYTES: usize = 8_192;
pub const MAX_ASSISTANT_OUTPUT_BYTES: usize = 65_536;
pub const MAX_QUEUE_LENGTH: usize = 8;
pub const MAX_DISCOVERED_MODELS: usize = 64;
pub const MAX_PROVIDER_ERROR_BYTES: usize = 256;
pub const DEFAULT_KEEP_ALIVE_MINUTES: u32 = 15;
pub const MAX_KEEP_ALIVE_MINUTES: u32 = 120;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionalAgent {
    pub id: String,
    pub name: String,
    pub profile_key: String,
    pub sprite_key: String,
    pub position: AgentPosition,
    pub birthday: String,
    pub fictive_age: u32,
    pub age_category: String,
    pub species: String,
    pub pronouns: String,
    pub personality_summary: String,
    pub traits_json: String,
    pub appearance_preset: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeState {
    Stopped,
    Starting,
    Ready,
    Unavailable,
    Crashed,
    SafeMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub state: RuntimeState,
    pub protocol_version: Option<u32>,
    pub detail_code: &'static str,
}

impl RuntimeStatus {
    pub fn stopped() -> Self {
        Self {
            state: RuntimeState::Stopped,
            protocol_version: None,
            detail_code: "runtime_stopped",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub app_version: String,
    pub safe_mode: bool,
    pub database_ready: bool,
    pub migration_version: i64,
    pub runtime: RuntimeStatus,
    pub agents: Vec<ProvisionalAgent>,
    pub onboarding_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderState {
    Checking,
    Available,
    Empty,
    Unavailable,
    Malformed,
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaModel {
    #[serde(rename = "ref")]
    pub model_ref: String,
    pub provider_model_id: String,
    pub display_name: String,
    pub size: u64,
    pub family: Option<String>,
    pub parameter_size: Option<String>,
    pub quantization: Option<String>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSnapshot {
    pub state: ProviderState,
    pub detail_code: String,
    pub models: Vec<OllamaModel>,
    pub refreshed_at: Option<i64>,
}

impl ProviderSnapshot {
    pub fn checking() -> Self {
        Self {
            state: ProviderState::Checking,
            detail_code: "provider_checking".into(),
            models: Vec::new(),
            refreshed_at: None,
        }
    }

    pub fn unavailable(code: &str) -> Self {
        Self {
            state: ProviderState::Unavailable,
            detail_code: code.to_string(),
            models: Vec::new(),
            refreshed_at: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageAuthor {
    User,
    Agent,
    System,
}

impl TryFrom<&str> for MessageAuthor {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "user" => Ok(Self::User),
            "agent" => Ok(Self::Agent),
            "system" => Ok(Self::System),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    Pending,
    Streaming,
    Complete,
    Failed,
    Cancelled,
}

impl MessageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Streaming => "streaming",
            Self::Complete => "complete",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl TryFrom<&str> for MessageStatus {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(Self::Pending),
            "streaming" => Ok(Self::Streaming),
            "complete" => Ok(Self::Complete),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(()),
        }
    }
}

pub fn can_transition_message(from: MessageStatus, to: MessageStatus) -> bool {
    use MessageStatus::{Cancelled, Complete, Failed, Pending, Streaming};
    matches!(
        (from, to),
        (Pending, Streaming | Failed | Cancelled) | (Streaming, Complete | Failed | Cancelled)
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseOneConversation {
    pub id: String,
    pub agent_id: String,
    pub title: String,
    pub model_override_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMemory {
    pub id: String,
    pub agent_id: String,
    pub category: String,
    pub content: String,
    pub status: String,
    pub confirmation_status: String,
    pub confidence_milli: u16,
    pub importance: u8,
    pub source_type: String,
    pub source_message_id: Option<String>,
    pub source_conversation_id: Option<String>,
    pub conflict_key: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationMessage {
    pub id: String,
    pub conversation_id: String,
    pub agent_id: String,
    pub author: MessageAuthor,
    pub content: String,
    pub model_ref: Option<String>,
    pub status: MessageStatus,
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueEntrySnapshot {
    pub request_id: String,
    pub agent_id: String,
    pub conversation_id: String,
    pub assistant_message_id: String,
    pub position: usize,
    pub active: bool,
    pub cancellation_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseOneState {
    pub agent: ProvisionalAgent,
    pub conversation: PhaseOneConversation,
    pub messages: Vec<ConversationMessage>,
    pub provider: ProviderSnapshot,
    pub selected_model_ref: Option<String>,
    pub default_model_ref: Option<String>,
    pub model_override_ref: Option<String>,
    pub selected_model_available: bool,
    pub keep_alive_minutes: u32,
    pub queue: Vec<QueueEntrySnapshot>,
    pub can_send: bool,
    pub send_blocked_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageResult {
    pub request_id: String,
    pub conversation_id: String,
    pub user_message_id: String,
    pub assistant_message_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseOneEvent {
    pub protocol_version: u32,
    pub event_type: String,
    pub request_id: Option<String>,
    pub agent_id: Option<String>,
    pub conversation_id: Option<String>,
    pub assistant_message_id: Option<String>,
    pub sequence: Option<u64>,
    pub content: Option<String>,
    pub error_code: Option<String>,
}

pub fn can_transition_runtime(from: RuntimeState, to: RuntimeState) -> bool {
    use RuntimeState::{Crashed, Ready, SafeMode, Starting, Stopped, Unavailable};
    matches!(
        (from, to),
        (Stopped, Starting | SafeMode)
            | (Starting, Ready | Unavailable | Crashed | Stopped | SafeMode)
            | (Ready, Crashed | Stopped | SafeMode)
            | (Unavailable, Starting | Stopped | SafeMode)
            | (Crashed, Starting | Stopped | SafeMode)
            | (SafeMode, Stopped | Starting)
    )
}

#[cfg(test)]
mod tests {
    use super::{can_transition_runtime, RuntimeState};

    #[test]
    fn runtime_transitions_are_bounded() {
        assert!(can_transition_runtime(
            RuntimeState::Ready,
            RuntimeState::SafeMode
        ));
        assert!(!can_transition_runtime(
            RuntimeState::SafeMode,
            RuntimeState::Ready
        ));
        assert!(can_transition_runtime(
            RuntimeState::SafeMode,
            RuntimeState::Starting
        ));
    }
}
