use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionalAgent {
    pub id: String,
    pub name: String,
    pub profile_key: String,
    pub sprite_key: String,
    pub position: AgentPosition,
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
