mod chat;
mod cognitive;
mod companion;
mod companion_transport;
mod conversation;
mod database;
mod domain;
mod extensions;
mod fullscreen;
mod gateway;
#[cfg(test)]
mod gateway_integration_tests;
mod gateway_transport;
mod native_overlay_region;
mod orchestration;
mod overlays;
mod protocol;
mod runtime;
mod screen_vision;
mod tools;
mod voice;

use std::{
    io,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use chat::ChatCoordinator;
use cognitive::{
    CognitiveGoal, CognitiveOpinion, FictionalActivity, FictionalActivityRequest,
    FictionalActivityStatusRequest, GoalRequest, OpinionCandidateRequest,
    OpinionEvidenceCorrectionRequest, RelationshipCandidateRequest, RelationshipState,
};
use companion::{
    CompanionAuditRecord, CompanionDevice, CompanionDeviceActionRequest, CompanionHistoryRecord,
    CompanionKeyRotation, CompanionPairingConfirmationRequest, CompanionPairingRequest,
    CompanionQueueActionRequest, CompanionQueueDecisionRequest, CompanionQueueItem,
    CompanionQueuePreviewRequest, CompanionReconnectRequest, CompanionRevocation, CompanionSession,
    CompanionSessionRequest,
};
use companion_transport::{
    start_secure, EphemeralPairingKey, HandlerResponse, SecureHandler, TransportHandle,
};
use conversation::{
    AgentConversationInspection, AgentConversationSummary, CognitiveCandidate,
    CognitiveCandidateRejectionRequest, CognitiveCandidateRequest, CognitiveResourceJob,
    ConversationInterruptRequest, ConversationPolicy, ConversationPolicyRequest,
    ConversationStartRequest, HeavyGenerationRequest, PublicConversationTurnRequest,
    ResourceJobCompletionRequest,
};
use database::Database;
use domain::{
    AgentMemory, AgentSimulatedState, AppSnapshot, CognitiveEvent, CognitiveEventExplanation,
    CognitiveTrait, ConversationMessage, PhaseOneConversation, PhaseOneState, ProviderSnapshot,
    SendMessageResult,
};
use extensions::{
    ExtensionActivationRequest, ExtensionAgentProposalRequest, ExtensionAuditRecord,
    ExtensionCatalogEntry, ExtensionDisableRequest, ExtensionExecutionCancellationRequest,
    ExtensionExecutionRequest, ExtensionExecutionResult, ExtensionImportRequest,
    ExtensionInstruction, ExtensionPackage, ExtensionProposal, ExtensionProposalRequest,
    ExtensionReviewRequest, ExtensionRollbackRequest, ExtensionUpdateRequest,
};
use gateway::{
    GatewayAccount, GatewayAuditRecord, GatewayProtocolInfo, GatewayReconnectRequest,
    GatewayRecovery, GatewayRecoveryApprovalRequest, GatewayRecoveryRequest, GatewayRevocation,
    GatewaySession, GatewaySessionActionRequest, GatewaySessionRequest, GatewayTransfer,
    GatewayTransferActionRequest, GatewayTransferApprovalRequest, GatewayTransferRequest,
};
use gateway_transport::{
    start_secure as start_gateway_secure, EphemeralPairingKey as GatewayPairingKey,
    HandlerResponse as GatewayHandlerResponse, SecureHandler as GatewayHandler,
    TransportHandle as GatewayTransportHandle,
};
use orchestration::{OrchestrationManager, RoutingPolicy};
use overlays::{InteractiveRegion, OverlayInputState};
use runtime::RuntimeController;
use screen_vision::{
    ScreenVisionAnalysisResult, ScreenVisionAuditRecord, ScreenVisionFixture, ScreenVisionJob,
    ScreenVisionJobCancellationRequest, ScreenVisionJobCleanupRequest,
    ScreenVisionJobConfirmationRequest, ScreenVisionJobPreviewRequest, ScreenVisionProviderStatus,
    ScreenVisionSession, ScreenVisionSessionCancellationRequest, ScreenVisionSessionRequest,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tools::{
    ToolAction, ToolActionCancellationRequest, ToolActionConfirmationRequest,
    ToolActionDecisionRequest, ToolActionExecutionRequest, ToolActionPreviewRequest,
    ToolAuditRecord, ToolManifest, ToolSession, ToolSessionCancellationRequest, ToolSessionRequest,
    WorkspaceRoot, WorkspaceRootIdRequest, WorkspaceRootRequest,
};
use voice::{
    CustomVoiceConsentRequest, LocalProvider, LocalProviderIdRequest, LocalProviderRequest,
    VoiceCaptureRequest, VoiceDevice, VoiceEmotionHypothesisRequest, VoiceEmotionHypothesisResult,
    VoiceOperationCancellationRequest, VoiceOperationStatus, VoiceOperationStatusRequest,
    VoiceProviderStatus, VoiceRuntime, VoiceRuntimeSynthesisResult,
    VoiceRuntimeTranscriptionResult, VoiceRuntimeWakeWordResult, VoiceSettings,
    VoiceSettingsRequest, VoiceSynthesisRequest, VoiceSynthesisResult,
    VoiceSynthesisRuntimeRequest, VoiceTranscriptionRequest, VoiceTranscriptionResult,
    VoiceWakeWordRequest, VoiceWakeWordResult,
};

#[tauri::command]
fn list_voice_devices() -> Vec<VoiceDevice> {
    voice::list_voice_devices()
}

struct AppState {
    database: Option<Database>,
    runtime: RuntimeController,
    voice_runtime: VoiceRuntime,
    chat: Option<ChatCoordinator>,
    safe_mode: Arc<AtomicBool>,
    overlay_input: OverlayInputState,
    companion_transport: Arc<Mutex<CompanionTransportState>>,
    gateway_transport: Arc<Mutex<GatewayTransportState>>,
}

fn routing_policy_or_default(policy: Option<RoutingPolicy>) -> RoutingPolicy {
    policy.unwrap_or_default()
}

#[derive(Default)]
struct GatewayTransportState {
    handle: Option<GatewayTransportHandle>,
    endpoint: Option<String>,
    pairing: Option<GatewayPairingKey>,
}

#[derive(Default)]
struct CompanionTransportState {
    handle: Option<TransportHandle>,
    endpoint: Option<String>,
    pairing: Option<EphemeralPairingKey>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompanionTransportStartRequest {
    agent_id: String,
    owner_confirmed: bool,
    private_network_confirmed: bool,
    bind_address: Option<String>,
    port: Option<u16>,
    temporary_chat: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CompanionTransportStartResult {
    enabled: bool,
    endpoint: String,
    pairing_code: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CompanionTransportStatus {
    enabled: bool,
    endpoint: Option<String>,
    pairing_available: bool,
}

fn companion_transport_handler(database: Database, safe_mode: Arc<AtomicBool>) -> SecureHandler {
    Arc::new(move |frame| {
        if safe_mode.load(Ordering::Acquire) {
            return Err("companion_blocked_safe_mode".into());
        }
        let payload = |value: serde_json::Value| {
            serde_json::to_string(&value).map_err(|_| "companion_payload_invalid".to_owned())
        };
        let result = match frame.kind.as_str() {
            "pair" => {
                let request: CompanionPairingRequest = serde_json::from_str(&frame.payload)
                    .map_err(|_| "companion_payload_invalid")?;
                database
                    .start_companion_pairing(request)
                    .map(|v| serde_json::to_value(v).unwrap_or(json!({})))
            }
            "session" => {
                let request: CompanionSessionRequest = serde_json::from_str(&frame.payload)
                    .map_err(|_| "companion_payload_invalid")?;
                database
                    .connect_companion_session(request)
                    .map(|v| serde_json::to_value(v).unwrap_or(json!({})))
            }
            "reconnect" => {
                let request: CompanionReconnectRequest = serde_json::from_str(&frame.payload)
                    .map_err(|_| "companion_payload_invalid")?;
                database
                    .reconnect_companion_session(request)
                    .map(|v| serde_json::to_value(v).unwrap_or(json!({})))
            }
            "history" => {
                #[derive(Deserialize)]
                struct Request {
                    agent_id: String,
                }
                let request: Request = serde_json::from_str(&frame.payload)
                    .map_err(|_| "companion_payload_invalid")?;
                database
                    .list_companion_history(&request.agent_id)
                    .map(|v| serde_json::to_value(v).unwrap_or(json!([])))
            }
            "queue_preview" => {
                let request: CompanionQueuePreviewRequest = serde_json::from_str(&frame.payload)
                    .map_err(|_| "companion_payload_invalid")?;
                database
                    .preview_companion_queue(request)
                    .map(|v| serde_json::to_value(v).unwrap_or(json!({})))
            }
            "queue_approve" => {
                let request: CompanionQueueDecisionRequest =
                    serde_json::from_str(&frame.payload)
                        .map_err(|_| "companion_payload_invalid")?;
                database
                    .approve_companion_queue(request)
                    .map(|v| serde_json::to_value(v).unwrap_or(json!({})))
            }
            "queue_cancel" => {
                let request: CompanionQueueActionRequest = serde_json::from_str(&frame.payload)
                    .map_err(|_| "companion_payload_invalid")?;
                database
                    .cancel_companion_queue(request)
                    .map(|v| serde_json::to_value(v).unwrap_or(json!({})))
            }
            "queue_retry" => {
                let request: CompanionQueueActionRequest = serde_json::from_str(&frame.payload)
                    .map_err(|_| "companion_payload_invalid")?;
                database
                    .retry_companion_queue(request)
                    .map(|v| serde_json::to_value(v).unwrap_or(json!({})))
            }
            _ => Err(database::DatabaseError::Cognitive(
                "companion_payload_invalid",
            )),
        };
        match result {
            Ok(value) => Ok(Some(HandlerResponse {
                kind: format!("{}_result", frame.kind),
                payload: payload(value)?,
            })),
            Err(error) => Err(error.code().to_owned()),
        }
    })
}

#[tauri::command]
fn start_companion_transport(
    state: State<'_, AppState>,
    request: CompanionTransportStartRequest,
) -> Result<CompanionTransportStartResult, String> {
    if !request.owner_confirmed {
        return Err("companion_owner_confirmation_required".into());
    }
    if request.temporary_chat
        || state
            .chat
            .as_ref()
            .is_some_and(|chat| chat.temporary_chat_active(&request.agent_id))
    {
        return Err("companion_blocked_temporary".into());
    }
    if state.safe_mode.load(Ordering::Acquire) {
        return Err("companion_blocked_safe_mode".into());
    }
    let database = state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .clone();
    if ![database::ASTRA_ID, database::LUMA_ID].contains(&request.agent_id.as_str()) {
        return Err("companion_owner_required".into());
    }
    let ip: IpAddr = request
        .bind_address
        .as_deref()
        .unwrap_or("127.0.0.1")
        .parse()
        .map_err(|_| "companion_bind_invalid")?;
    let port = request.port.unwrap_or(0);
    let address = SocketAddr::new(ip, port);
    let (pairing, code) =
        EphemeralPairingKey::generate().map_err(|_| "companion_pairing_unavailable")?;
    let key = pairing
        .consume()
        .map_err(|_| "companion_pairing_unavailable")?;
    let handler = companion_transport_handler(database, Arc::clone(&state.safe_mode));
    let handle = start_secure(address, request.private_network_confirmed, key, handler)
        .map_err(|e| e.to_string())?;
    let endpoint = handle.addr.to_string();
    let mut transport = state
        .companion_transport
        .lock()
        .map_err(|_| "companion_state_unavailable")?;
    if let Some(mut old) = transport.handle.take() {
        old.stop();
    }
    transport.endpoint = Some(endpoint.clone());
    transport.pairing = Some(pairing);
    transport.handle = Some(handle);
    Ok(CompanionTransportStartResult {
        enabled: true,
        endpoint,
        pairing_code: code,
    })
}

#[tauri::command]
fn stop_companion_transport(state: State<'_, AppState>) -> Result<(), String> {
    let mut transport = state
        .companion_transport
        .lock()
        .map_err(|_| "companion_state_unavailable")?;
    if let Some(mut handle) = transport.handle.take() {
        handle.stop();
    }
    transport.endpoint = None;
    transport.pairing = None;
    Ok(())
}
#[tauri::command]
fn get_companion_transport_status(
    state: State<'_, AppState>,
) -> Result<CompanionTransportStatus, String> {
    let transport = state
        .companion_transport
        .lock()
        .map_err(|_| "companion_state_unavailable")?;
    Ok(CompanionTransportStatus {
        enabled: transport.handle.is_some(),
        endpoint: transport.endpoint.clone(),
        pairing_available: transport
            .pairing
            .as_ref()
            .is_some_and(|p| p.status().available),
    })
}

fn ensure_conversation_not_temporary(
    state: &AppState,
    agent_id: &str,
    requested_temporary: bool,
) -> Result<(), &'static str> {
    if requested_temporary
        || state
            .chat
            .as_ref()
            .is_some_and(|chat| chat.temporary_chat_active(agent_id))
    {
        Err("conversation_temporary_blocked")
    } else {
        Ok(())
    }
}

fn ensure_voice_mutation_allowed(
    state: &AppState,
    agent_id: &str,
    requested_temporary: bool,
) -> Result<(), &'static str> {
    ensure_conversation_not_temporary(state, agent_id, requested_temporary)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .ensure_voice_mutation_allowed(agent_id)
        .map_err(|error| error.code())
}

fn ensure_voice_runtime_allowed<'a>(
    state: &'a AppState,
    agent_id: &str,
    temporary_chat: bool,
) -> Result<&'a Database, &'static str> {
    if state.safe_mode.load(Ordering::Acquire) {
        return Err("conversation_blocked_safe_mode");
    }
    ensure_voice_mutation_allowed(state, agent_id, temporary_chat)?;
    state.database.as_ref().ok_or("operation_unavailable")
}

#[tauri::command]
fn list_cognitive_traits(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<CognitiveTrait>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .cognitive_traits(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_cognitive_events(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<CognitiveEvent>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .cognitive_events(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn explain_cognitive_event(
    state: State<'_, AppState>,
    agent_id: String,
    event_id: String,
) -> Result<CognitiveEventExplanation, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .cognitive_event_explanation(&agent_id, &event_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn create_owner_trait_correction(
    state: State<'_, AppState>,
    agent_id: String,
    trait_key: String,
    value: f64,
    reason: String,
    idempotency_key: String,
    temporary_chat: bool,
) -> Result<CognitiveEvent, &'static str> {
    ensure_conversation_not_temporary(&state, &agent_id, temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .owner_correct_trait(&agent_id, &trait_key, value, &reason, &idempotency_key)
        .map_err(|error| error.code())
}

#[tauri::command]
fn rollback_cognitive_event(
    state: State<'_, AppState>,
    agent_id: String,
    event_id: String,
    idempotency_key: String,
    temporary_chat: bool,
) -> Result<CognitiveEvent, &'static str> {
    ensure_conversation_not_temporary(&state, &agent_id, temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .rollback_cognitive_event(&agent_id, &event_id, &idempotency_key)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_cognitive_opinions(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<CognitiveOpinion>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_cognitive_opinions(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn propose_cognitive_opinion(
    state: State<'_, AppState>,
    request: OpinionCandidateRequest,
) -> Result<CognitiveOpinion, &'static str> {
    ensure_conversation_not_temporary(&state, &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .propose_cognitive_opinion(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn correct_cognitive_opinion_evidence(
    state: State<'_, AppState>,
    request: OpinionEvidenceCorrectionRequest,
) -> Result<CognitiveOpinion, &'static str> {
    ensure_conversation_not_temporary(&state, &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .correct_opinion_evidence(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn set_cognitive_opinion_status(
    state: State<'_, AppState>,
    agent_id: String,
    opinion_id: String,
    status: String,
    reason: String,
    idempotency_key: String,
    temporary_chat: bool,
) -> Result<CognitiveOpinion, &'static str> {
    ensure_conversation_not_temporary(&state, &agent_id, temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .set_opinion_status(&agent_id, &opinion_id, &status, &reason, &idempotency_key)
        .map_err(|error| error.code())
}

#[tauri::command]
fn recalculate_cognitive_opinion(
    state: State<'_, AppState>,
    agent_id: String,
    opinion_id: String,
    reason: String,
    idempotency_key: String,
    temporary_chat: bool,
) -> Result<CognitiveOpinion, &'static str> {
    ensure_conversation_not_temporary(&state, &agent_id, temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .recalculate_opinion(&agent_id, &opinion_id, &reason, &idempotency_key)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_cognitive_relationships(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<RelationshipState>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_relationships(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn propose_cognitive_relationship(
    state: State<'_, AppState>,
    request: RelationshipCandidateRequest,
) -> Result<RelationshipState, &'static str> {
    ensure_conversation_not_temporary(&state, &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .propose_relationship_event(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn reset_cognitive_relationship(
    state: State<'_, AppState>,
    agent_id: String,
    relationship_id: String,
    reason: String,
    idempotency_key: String,
    temporary_chat: bool,
) -> Result<RelationshipState, &'static str> {
    ensure_conversation_not_temporary(&state, &agent_id, temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .reset_relationship(&agent_id, &relationship_id, &reason, &idempotency_key)
        .map_err(|error| error.code())
}

#[tauri::command]
fn rollback_cognitive_relationship(
    state: State<'_, AppState>,
    agent_id: String,
    event_id: String,
    idempotency_key: String,
    temporary_chat: bool,
) -> Result<RelationshipState, &'static str> {
    ensure_conversation_not_temporary(&state, &agent_id, temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .rollback_relationship_event(&agent_id, &event_id, &idempotency_key)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_cognitive_goals(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<CognitiveGoal>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_cognitive_goals(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn create_owner_cognitive_goal(
    state: State<'_, AppState>,
    request: GoalRequest,
) -> Result<CognitiveGoal, &'static str> {
    ensure_conversation_not_temporary(&state, &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .create_owner_goal(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn propose_agent_cognitive_goal(
    state: State<'_, AppState>,
    request: GoalRequest,
) -> Result<CognitiveGoal, &'static str> {
    ensure_conversation_not_temporary(&state, &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .propose_agent_goal(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn approve_cognitive_goal(
    state: State<'_, AppState>,
    agent_id: String,
    goal_id: String,
    idempotency_key: String,
    temporary_chat: bool,
) -> Result<CognitiveGoal, &'static str> {
    ensure_conversation_not_temporary(&state, &agent_id, temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .approve_cognitive_goal(&agent_id, &goal_id, &idempotency_key)
        .map_err(|error| error.code())
}

#[tauri::command]
fn update_cognitive_goal_status(
    state: State<'_, AppState>,
    agent_id: String,
    goal_id: String,
    status: String,
    completion_evidence: Option<String>,
    idempotency_key: String,
    temporary_chat: bool,
) -> Result<CognitiveGoal, &'static str> {
    ensure_conversation_not_temporary(&state, &agent_id, temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .update_goal_status(
            &agent_id,
            &goal_id,
            &status,
            completion_evidence.as_deref(),
            &idempotency_key,
        )
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_fictional_activities(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<FictionalActivity>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_fictional_activities(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn start_fictional_activity(
    state: State<'_, AppState>,
    request: FictionalActivityRequest,
) -> Result<FictionalActivity, &'static str> {
    ensure_conversation_not_temporary(&state, &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .start_fictional_activity(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn update_fictional_activity_status(
    state: State<'_, AppState>,
    request: FictionalActivityStatusRequest,
) -> Result<FictionalActivity, &'static str> {
    ensure_conversation_not_temporary(&state, &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .update_fictional_activity_status(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_agent_conversation_policies(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<ConversationPolicy>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_conversation_policies(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn set_agent_conversation_policy(
    state: State<'_, AppState>,
    request: ConversationPolicyRequest,
) -> Result<ConversationPolicy, &'static str> {
    ensure_conversation_not_temporary(&state, &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .set_conversation_policy(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn start_agent_conversation(
    state: State<'_, AppState>,
    request: ConversationStartRequest,
) -> Result<AgentConversationSummary, &'static str> {
    start_agent_conversation_for_state(state.inner(), request)
}

fn start_agent_conversation_for_state(
    state: &AppState,
    request: ConversationStartRequest,
) -> Result<AgentConversationSummary, &'static str> {
    ensure_conversation_not_temporary(state, &request.initiator_agent_id, request.temporary_chat)?;
    ensure_conversation_not_temporary(
        state,
        &request.participant_agent_id,
        request.temporary_chat,
    )?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .start_agent_conversation(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn append_public_conversation_turn(
    state: State<'_, AppState>,
    request: PublicConversationTurnRequest,
) -> Result<AgentConversationInspection, &'static str> {
    append_public_conversation_turn_for_state(state.inner(), request)
}

fn append_public_conversation_turn_for_state(
    state: &AppState,
    request: PublicConversationTurnRequest,
) -> Result<AgentConversationInspection, &'static str> {
    ensure_conversation_not_temporary(state, &request.agent_id, request.temporary_chat)?;
    ensure_conversation_not_temporary(state, &request.speaker_agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .append_public_conversation_turn(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn emit_cognitive_candidate(
    state: State<'_, AppState>,
    request: CognitiveCandidateRequest,
) -> Result<CognitiveCandidate, &'static str> {
    emit_cognitive_candidate_for_state(state.inner(), request)
}

fn emit_cognitive_candidate_for_state(
    state: &AppState,
    request: CognitiveCandidateRequest,
) -> Result<CognitiveCandidate, &'static str> {
    ensure_conversation_not_temporary(state, &request.agent_id, false)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .emit_cognitive_candidate(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn reserve_heavy_generation(
    state: State<'_, AppState>,
    request: HeavyGenerationRequest,
) -> Result<CognitiveResourceJob, &'static str> {
    reserve_heavy_generation_for_state(state.inner(), request)
}

fn reserve_heavy_generation_for_state(
    state: &AppState,
    request: HeavyGenerationRequest,
) -> Result<CognitiveResourceJob, &'static str> {
    ensure_conversation_not_temporary(state, &request.agent_id, false)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .reserve_heavy_generation(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn complete_resource_job(
    state: State<'_, AppState>,
    request: ResourceJobCompletionRequest,
) -> Result<CognitiveResourceJob, &'static str> {
    complete_resource_job_for_state(state.inner(), request)
}

fn complete_resource_job_for_state(
    state: &AppState,
    request: ResourceJobCompletionRequest,
) -> Result<CognitiveResourceJob, &'static str> {
    ensure_conversation_not_temporary(state, &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .complete_resource_job(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_cognitive_conversations(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<AgentConversationSummary>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_agent_conversations(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn inspect_agent_conversation(
    state: State<'_, AppState>,
    agent_id: String,
    conversation_id: String,
) -> Result<AgentConversationInspection, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .inspect_agent_conversation(&agent_id, &conversation_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn interrupt_agent_conversation(
    state: State<'_, AppState>,
    request: ConversationInterruptRequest,
) -> Result<AgentConversationSummary, &'static str> {
    ensure_conversation_not_temporary(&state, &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .interrupt_agent_conversation(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_cognitive_candidates(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<CognitiveCandidate>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_cognitive_candidates(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn reject_cognitive_candidate(
    state: State<'_, AppState>,
    request: CognitiveCandidateRejectionRequest,
) -> Result<CognitiveCandidate, &'static str> {
    ensure_conversation_not_temporary(&state, &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .reject_cognitive_candidate(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn get_voice_settings(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<VoiceSettings, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .voice_settings(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn get_voice_provider_status(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<VoiceProviderStatus, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .voice_provider_status(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_local_providers(state: State<'_, AppState>) -> Result<Vec<LocalProvider>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_local_providers()
        .map_err(|error| error.code())
}

#[tauri::command]
fn register_local_provider(
    state: State<'_, AppState>,
    request: LocalProviderRequest,
) -> Result<LocalProvider, &'static str> {
    ensure_voice_mutation_allowed(&state, &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .register_local_provider(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn disable_local_provider(
    state: State<'_, AppState>,
    request: LocalProviderIdRequest,
) -> Result<LocalProvider, &'static str> {
    ensure_voice_mutation_allowed(&state, &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .disable_local_provider(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn update_voice_settings(
    state: State<'_, AppState>,
    request: VoiceSettingsRequest,
) -> Result<VoiceSettings, &'static str> {
    update_voice_settings_for_state(state.inner(), request)
}

fn update_voice_settings_for_state(
    state: &AppState,
    request: VoiceSettingsRequest,
) -> Result<VoiceSettings, &'static str> {
    ensure_voice_mutation_allowed(state, &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .update_voice_settings(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn set_custom_voice_consent(
    state: State<'_, AppState>,
    request: CustomVoiceConsentRequest,
) -> Result<VoiceSettings, &'static str> {
    set_custom_voice_consent_for_state(state.inner(), request)
}

fn set_custom_voice_consent_for_state(
    state: &AppState,
    request: CustomVoiceConsentRequest,
) -> Result<VoiceSettings, &'static str> {
    ensure_voice_mutation_allowed(state, &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .set_custom_voice_consent(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn transcribe_voice_fixture(
    state: State<'_, AppState>,
    request: VoiceTranscriptionRequest,
) -> Result<VoiceTranscriptionResult, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .transcribe_voice_fixture(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn synthesize_voice_fixture(
    state: State<'_, AppState>,
    request: VoiceSynthesisRequest,
) -> Result<VoiceSynthesisResult, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .synthesize_voice_fixture(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn detect_voice_wake_word_fixture(
    state: State<'_, AppState>,
    request: VoiceWakeWordRequest,
) -> Result<VoiceWakeWordResult, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .detect_voice_wake_word_fixture(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn transcribe_voice_local(
    state: State<'_, AppState>,
    request: VoiceCaptureRequest,
) -> Result<VoiceRuntimeTranscriptionResult, &'static str> {
    let database = ensure_voice_runtime_allowed(&state, &request.agent_id, request.temporary_chat)?;
    state
        .voice_runtime
        .transcribe(database, request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn synthesize_voice_local(
    state: State<'_, AppState>,
    request: VoiceSynthesisRuntimeRequest,
) -> Result<VoiceRuntimeSynthesisResult, &'static str> {
    let database = ensure_voice_runtime_allowed(&state, &request.agent_id, request.temporary_chat)?;
    state
        .voice_runtime
        .synthesize(database, request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn detect_voice_wake_word_local(
    state: State<'_, AppState>,
    request: VoiceCaptureRequest,
) -> Result<VoiceRuntimeWakeWordResult, &'static str> {
    let database = ensure_voice_runtime_allowed(&state, &request.agent_id, request.temporary_chat)?;
    state
        .voice_runtime
        .detect_wake_word(database, request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn cancel_voice_operation(
    state: State<'_, AppState>,
    request: VoiceOperationCancellationRequest,
) -> Result<bool, &'static str> {
    let database = state.database.as_ref().ok_or("operation_unavailable")?;
    database
        .voice_operation_status(VoiceOperationStatusRequest {
            agent_id: request.agent_id,
            operation_id: request.operation_id.clone(),
        })
        .map_err(|error| error.code())?;
    state
        .voice_runtime
        .cancel(&request.operation_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn get_voice_operation_status(
    state: State<'_, AppState>,
    request: VoiceOperationStatusRequest,
) -> Result<VoiceOperationStatus, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .voice_operation_status(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn classify_voice_emotion(
    state: State<'_, AppState>,
    request: VoiceEmotionHypothesisRequest,
) -> Result<VoiceEmotionHypothesisResult, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .classify_voice_emotion(request)
        .map_err(|error| error.code())
}

fn ensure_tool_mutation_allowed(
    state: &AppState,
    agent_id: &str,
    requested_temporary: bool,
) -> Result<(), &'static str> {
    if state.safe_mode.load(Ordering::Acquire) {
        return Err("tools_blocked_safe_mode");
    }
    ensure_conversation_not_temporary(state, agent_id, requested_temporary)
}

#[tauri::command]
fn list_tool_catalog(state: State<'_, AppState>) -> Result<Vec<ToolManifest>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_tool_catalog()
        .map_err(|error| error.code())
}

#[tauri::command]
fn add_workspace_root(
    state: State<'_, AppState>,
    request: WorkspaceRootRequest,
) -> Result<WorkspaceRoot, &'static str> {
    ensure_tool_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .add_workspace_root(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_workspace_roots(state: State<'_, AppState>) -> Result<Vec<WorkspaceRoot>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_workspace_roots()
        .map_err(|error| error.code())
}

#[tauri::command]
fn remove_workspace_root(
    state: State<'_, AppState>,
    request: WorkspaceRootIdRequest,
) -> Result<WorkspaceRoot, &'static str> {
    ensure_tool_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .remove_workspace_root(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn create_tool_session(
    state: State<'_, AppState>,
    request: ToolSessionRequest,
) -> Result<ToolSession, &'static str> {
    ensure_tool_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .create_tool_session(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_tool_sessions(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<ToolSession>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_tool_sessions(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn preview_tool_action(
    state: State<'_, AppState>,
    request: ToolActionPreviewRequest,
) -> Result<ToolAction, &'static str> {
    ensure_tool_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .preview_tool_action(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn approve_tool_action(
    state: State<'_, AppState>,
    request: ToolActionDecisionRequest,
) -> Result<ToolAction, &'static str> {
    ensure_tool_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .decide_tool_action(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn confirm_tool_action(
    state: State<'_, AppState>,
    request: ToolActionConfirmationRequest,
) -> Result<ToolAction, &'static str> {
    ensure_tool_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .confirm_tool_action(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn execute_tool_action(
    state: State<'_, AppState>,
    request: ToolActionExecutionRequest,
) -> Result<ToolAction, &'static str> {
    ensure_tool_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .execute_tool_action(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn cancel_tool_action(
    state: State<'_, AppState>,
    request: ToolActionCancellationRequest,
) -> Result<ToolAction, &'static str> {
    ensure_tool_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .cancel_tool_action(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn compensate_tool_action(
    state: State<'_, AppState>,
    request: ToolActionCancellationRequest,
) -> Result<ToolAction, &'static str> {
    ensure_tool_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .compensate_tool_action(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn cancel_tool_session(
    state: State<'_, AppState>,
    request: ToolSessionCancellationRequest,
) -> Result<ToolSession, &'static str> {
    ensure_tool_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .cancel_tool_session(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_tool_audit(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<ToolAuditRecord>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_tool_audit(&agent_id)
        .map_err(|error| error.code())
}

fn ensure_extension_mutation_allowed(
    state: &AppState,
    agent_id: &str,
    requested_temporary: bool,
) -> Result<(), &'static str> {
    ensure_conversation_not_temporary(state, agent_id, requested_temporary)
}

fn ensure_extension_execution_allowed<'a>(
    state: &'a AppState,
    agent_id: &str,
    temporary_chat: bool,
) -> Result<&'a Database, &'static str> {
    if state.safe_mode.load(Ordering::Acquire) {
        return Err("extensions_blocked_safe_mode");
    }
    ensure_extension_mutation_allowed(state, agent_id, temporary_chat)?;
    state.database.as_ref().ok_or("operation_unavailable")
}

#[tauri::command]
fn build_extension_package(
    instructions: Vec<ExtensionInstruction>,
) -> Result<ExtensionPackage, &'static str> {
    extensions::build_extension_package(instructions).map_err(|error| error.code())
}

#[tauri::command]
fn execute_extension(
    state: State<'_, AppState>,
    request: ExtensionExecutionRequest,
) -> Result<ExtensionExecutionResult, &'static str> {
    let database = ensure_extension_execution_allowed(
        state.inner(),
        &request.agent_id,
        request.temporary_chat,
    )?;
    database
        .execute_extension(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn cancel_extension_execution(
    state: State<'_, AppState>,
    request: ExtensionExecutionCancellationRequest,
) -> Result<(), &'static str> {
    if state.safe_mode.load(Ordering::Acquire) {
        return Err("extensions_blocked_safe_mode");
    }
    ensure_extension_mutation_allowed(state.inner(), &request.agent_id, false)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .cancel_extension_execution(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_extension_catalog(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<ExtensionCatalogEntry>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_extension_catalog(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_extension_proposals(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<ExtensionProposal>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_extension_proposals(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn create_extension_proposal(
    state: State<'_, AppState>,
    request: ExtensionProposalRequest,
) -> Result<ExtensionProposal, &'static str> {
    ensure_extension_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .create_extension_proposal(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn import_extension_manifest(
    state: State<'_, AppState>,
    request: ExtensionImportRequest,
) -> Result<ExtensionProposal, &'static str> {
    ensure_extension_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .import_extension_manifest(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn create_agent_extension_proposal(
    state: State<'_, AppState>,
    request: ExtensionAgentProposalRequest,
) -> Result<ExtensionProposal, &'static str> {
    ensure_extension_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .create_agent_extension_proposal(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn review_extension_proposal(
    state: State<'_, AppState>,
    request: ExtensionReviewRequest,
) -> Result<ExtensionProposal, &'static str> {
    ensure_extension_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .review_extension_proposal(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn activate_extension(
    state: State<'_, AppState>,
    request: ExtensionActivationRequest,
) -> Result<ExtensionCatalogEntry, &'static str> {
    ensure_extension_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .activate_extension(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn update_extension(
    state: State<'_, AppState>,
    request: ExtensionUpdateRequest,
) -> Result<ExtensionProposal, &'static str> {
    ensure_extension_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .update_extension(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn rollback_extension(
    state: State<'_, AppState>,
    request: ExtensionRollbackRequest,
) -> Result<ExtensionCatalogEntry, &'static str> {
    ensure_extension_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .rollback_extension(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn disable_extension(
    state: State<'_, AppState>,
    request: ExtensionDisableRequest,
) -> Result<ExtensionCatalogEntry, &'static str> {
    ensure_extension_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .disable_extension(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_extension_audit(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<ExtensionAuditRecord>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_extension_audit(&agent_id)
        .map_err(|error| error.code())
}

fn ensure_companion_mutation_allowed(
    state: &AppState,
    agent_id: &str,
    requested_temporary: bool,
) -> Result<(), &'static str> {
    ensure_conversation_not_temporary(state, agent_id, requested_temporary)
}

#[tauri::command]
fn list_companion_devices(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<CompanionDevice>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_companion_devices(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn start_companion_pairing(
    state: State<'_, AppState>,
    request: CompanionPairingRequest,
) -> Result<CompanionDevice, &'static str> {
    ensure_companion_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .start_companion_pairing(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn confirm_companion_pairing(
    state: State<'_, AppState>,
    request: CompanionPairingConfirmationRequest,
) -> Result<CompanionDevice, &'static str> {
    ensure_companion_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .confirm_companion_pairing(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_companion_sessions(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<CompanionSession>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_companion_sessions(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn connect_companion_session(
    state: State<'_, AppState>,
    request: CompanionSessionRequest,
) -> Result<CompanionSession, &'static str> {
    ensure_companion_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .connect_companion_session(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn reconnect_companion_session(
    state: State<'_, AppState>,
    request: CompanionReconnectRequest,
) -> Result<CompanionSession, &'static str> {
    ensure_companion_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .reconnect_companion_session(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_companion_queue(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<CompanionQueueItem>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_companion_queue(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn preview_companion_queue(
    state: State<'_, AppState>,
    request: CompanionQueuePreviewRequest,
) -> Result<CompanionQueueItem, &'static str> {
    ensure_companion_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .preview_companion_queue(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn approve_companion_queue(
    state: State<'_, AppState>,
    request: CompanionQueueDecisionRequest,
) -> Result<CompanionQueueItem, &'static str> {
    ensure_companion_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .approve_companion_queue(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn cancel_companion_queue(
    state: State<'_, AppState>,
    request: CompanionQueueActionRequest,
) -> Result<CompanionQueueItem, &'static str> {
    ensure_companion_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .cancel_companion_queue(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn retry_companion_queue(
    state: State<'_, AppState>,
    request: CompanionQueueActionRequest,
) -> Result<CompanionQueueItem, &'static str> {
    ensure_companion_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .retry_companion_queue(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_companion_history(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<CompanionHistoryRecord>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_companion_history(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_companion_audit(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<CompanionAuditRecord>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_companion_audit(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_companion_key_rotations(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<CompanionKeyRotation>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_companion_key_rotations(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_companion_revocations(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<CompanionRevocation>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_companion_revocations(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn rotate_companion_key(
    state: State<'_, AppState>,
    request: CompanionDeviceActionRequest,
) -> Result<CompanionKeyRotation, &'static str> {
    ensure_companion_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .rotate_companion_key(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn revoke_companion_device(
    state: State<'_, AppState>,
    request: CompanionDeviceActionRequest,
) -> Result<CompanionRevocation, &'static str> {
    ensure_companion_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .revoke_companion_device(request)
        .map_err(|error| error.code())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GatewayTransportStartRequest {
    agent_id: String,
    owner_confirmed: bool,
    private_network_confirmed: bool,
    bind_address: Option<String>,
    port: Option<u16>,
    temporary_chat: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GatewayTransportStartResult {
    enabled: bool,
    endpoint: String,
    pairing_code: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GatewayTransportStatus {
    enabled: bool,
    endpoint: Option<String>,
    pairing_available: bool,
}

fn gateway_transport_handler(database: Database, safe_mode: Arc<AtomicBool>) -> GatewayHandler {
    Arc::new(move |frame| {
        if safe_mode.load(Ordering::Acquire) {
            return Err("gateway_blocked_safe_mode".into());
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct AgentRequest {
            agent_id: String,
        }
        let agent = || -> Result<AgentRequest, String> {
            serde_json::from_str(&frame.payload).map_err(|_| "gateway_payload_invalid".into())
        };
        let value = match frame.kind.as_str() {
            "protocol" => serde_json::to_value(
                database
                    .gateway_protocol_info(&agent()?.agent_id)
                    .map_err(|e| e.code().to_owned())?,
            )
            .map_err(|_| "gateway_response_invalid".to_owned())?,
            "accounts" => serde_json::to_value(
                database
                    .list_gateway_accounts(&agent()?.agent_id)
                    .map_err(|e| e.code().to_owned())?,
            )
            .map_err(|_| "gateway_response_invalid".to_owned())?,
            "transfers" => serde_json::to_value(
                database
                    .list_gateway_transfers(&agent()?.agent_id)
                    .map_err(|e| e.code().to_owned())?,
            )
            .map_err(|_| "gateway_response_invalid".to_owned())?,
            "sessions" => serde_json::to_value(
                database
                    .list_gateway_sessions(&agent()?.agent_id)
                    .map_err(|e| e.code().to_owned())?,
            )
            .map_err(|_| "gateway_response_invalid".to_owned())?,
            "recoveries" => serde_json::to_value(
                database
                    .list_gateway_recoveries(&agent()?.agent_id)
                    .map_err(|e| e.code().to_owned())?,
            )
            .map_err(|_| "gateway_response_invalid".to_owned())?,
            "audit" => serde_json::to_value(
                database
                    .list_gateway_audit(&agent()?.agent_id)
                    .map_err(|e| e.code().to_owned())?,
            )
            .map_err(|_| "gateway_response_invalid".to_owned())?,
            "revocations" => serde_json::to_value(
                database
                    .list_gateway_revocations(&agent()?.agent_id)
                    .map_err(|e| e.code().to_owned())?,
            )
            .map_err(|_| "gateway_response_invalid".to_owned())?,
            "transfer_prepare" => serde_json::to_value(
                database
                    .prepare_gateway_transfer(
                        serde_json::from_str(&frame.payload)
                            .map_err(|_| "gateway_payload_invalid")?,
                    )
                    .map_err(|e| e.code().to_owned())?,
            )
            .map_err(|_| "gateway_response_invalid".to_owned())?,
            "transfer_approve" => serde_json::to_value(
                database
                    .approve_gateway_transfer(
                        serde_json::from_str(&frame.payload)
                            .map_err(|_| "gateway_payload_invalid")?,
                    )
                    .map_err(|e| e.code().to_owned())?,
            )
            .map_err(|_| "gateway_response_invalid".to_owned())?,
            "session_connect" => serde_json::to_value(
                database
                    .connect_gateway_session(
                        serde_json::from_str(&frame.payload)
                            .map_err(|_| "gateway_payload_invalid")?,
                    )
                    .map_err(|e| e.code().to_owned())?,
            )
            .map_err(|_| "gateway_response_invalid".to_owned())?,
            "session_reconnect" => serde_json::to_value(
                database
                    .reconnect_gateway_session(
                        serde_json::from_str(&frame.payload)
                            .map_err(|_| "gateway_payload_invalid")?,
                    )
                    .map_err(|e| e.code().to_owned())?,
            )
            .map_err(|_| "gateway_response_invalid".to_owned())?,
            "recovery_request" => serde_json::to_value(
                database
                    .request_gateway_recovery(
                        serde_json::from_str(&frame.payload)
                            .map_err(|_| "gateway_payload_invalid")?,
                    )
                    .map_err(|e| e.code().to_owned())?,
            )
            .map_err(|_| "gateway_response_invalid".to_owned())?,
            "recovery_approve" => serde_json::to_value(
                database
                    .approve_gateway_recovery(
                        serde_json::from_str(&frame.payload)
                            .map_err(|_| "gateway_payload_invalid")?,
                    )
                    .map_err(|e| e.code().to_owned())?,
            )
            .map_err(|_| "gateway_response_invalid".to_owned())?,
            "session_revoke" => serde_json::to_value(
                database
                    .revoke_gateway_session(
                        serde_json::from_str(&frame.payload)
                            .map_err(|_| "gateway_payload_invalid")?,
                    )
                    .map_err(|e| e.code().to_owned())?,
            )
            .map_err(|_| "gateway_response_invalid".to_owned())?,
            "transfer_revoke" => serde_json::to_value(
                database
                    .revoke_gateway_transfer(
                        serde_json::from_str(&frame.payload)
                            .map_err(|_| "gateway_payload_invalid")?,
                    )
                    .map_err(|e| e.code().to_owned())?,
            )
            .map_err(|_| "gateway_response_invalid".to_owned())?,
            _ => return Err("gateway_payload_invalid".into()),
        };
        Ok(Some(GatewayHandlerResponse {
            kind: format!("{}_result", frame.kind),
            payload: serde_json::to_string(&value).map_err(|_| "gateway_response_invalid")?,
        }))
    })
}

#[tauri::command]
fn start_gateway_transport(
    state: State<'_, AppState>,
    request: GatewayTransportStartRequest,
) -> Result<GatewayTransportStartResult, String> {
    if !request.owner_confirmed {
        return Err("gateway_owner_confirmation_required".into());
    }
    if request.temporary_chat
        || state
            .chat
            .as_ref()
            .is_some_and(|chat| chat.temporary_chat_active(&request.agent_id))
    {
        return Err("gateway_blocked_temporary".into());
    }
    if state.safe_mode.load(Ordering::Acquire) {
        return Err("gateway_blocked_safe_mode".into());
    }
    let database = state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .clone();
    if request.agent_id != database::LUMA_ID {
        return Err("gateway_owner_required".into());
    }
    let ip: IpAddr = request
        .bind_address
        .as_deref()
        .unwrap_or("127.0.0.1")
        .parse()
        .map_err(|_| "gateway_bind_invalid")?;
    let address = SocketAddr::new(ip, request.port.unwrap_or(0));
    let (pairing, code) =
        GatewayPairingKey::generate().map_err(|_| "gateway_pairing_unavailable")?;
    let key = pairing
        .consume()
        .map_err(|_| "gateway_pairing_unavailable")?;
    let handle = start_gateway_secure(
        address,
        request.private_network_confirmed,
        key,
        gateway_transport_handler(database, Arc::clone(&state.safe_mode)),
    )
    .map_err(|e| e.to_string())?;
    let endpoint = handle.addr.to_string();
    let mut transport = state
        .gateway_transport
        .lock()
        .map_err(|_| "gateway_state_unavailable")?;
    if let Some(mut old) = transport.handle.take() {
        old.stop();
    }
    transport.endpoint = Some(endpoint.clone());
    transport.pairing = Some(pairing);
    transport.handle = Some(handle);
    Ok(GatewayTransportStartResult {
        enabled: true,
        endpoint,
        pairing_code: code,
    })
}

#[tauri::command]
fn stop_gateway_transport(state: State<'_, AppState>) -> Result<(), String> {
    let mut transport = state
        .gateway_transport
        .lock()
        .map_err(|_| "gateway_state_unavailable")?;
    if let Some(mut handle) = transport.handle.take() {
        handle.stop();
    }
    transport.endpoint = None;
    transport.pairing = None;
    Ok(())
}

#[tauri::command]
fn get_gateway_transport_status(
    state: State<'_, AppState>,
) -> Result<GatewayTransportStatus, String> {
    let transport = state
        .gateway_transport
        .lock()
        .map_err(|_| "gateway_state_unavailable")?;
    Ok(GatewayTransportStatus {
        enabled: transport.handle.is_some(),
        endpoint: transport.endpoint.clone(),
        pairing_available: transport
            .pairing
            .as_ref()
            .is_some_and(|p| p.status().available),
    })
}

fn ensure_gateway_mutation_allowed(
    state: &AppState,
    agent_id: &str,
    requested_temporary: bool,
) -> Result<(), &'static str> {
    ensure_conversation_not_temporary(state, agent_id, requested_temporary)
}

#[tauri::command]
fn get_gateway_protocol(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<GatewayProtocolInfo, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .gateway_protocol_info(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_gateway_accounts(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<GatewayAccount>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_gateway_accounts(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_gateway_transfers(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<GatewayTransfer>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_gateway_transfers(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn prepare_gateway_transfer(
    state: State<'_, AppState>,
    request: GatewayTransferRequest,
) -> Result<GatewayTransfer, &'static str> {
    ensure_gateway_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .prepare_gateway_transfer(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn approve_gateway_transfer(
    state: State<'_, AppState>,
    request: GatewayTransferApprovalRequest,
) -> Result<GatewayTransfer, &'static str> {
    ensure_gateway_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .approve_gateway_transfer(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_gateway_sessions(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<GatewaySession>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_gateway_sessions(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn connect_gateway_session(
    state: State<'_, AppState>,
    request: GatewaySessionRequest,
) -> Result<GatewaySession, &'static str> {
    ensure_gateway_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .connect_gateway_session(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn reconnect_gateway_session(
    state: State<'_, AppState>,
    request: GatewayReconnectRequest,
) -> Result<GatewaySession, &'static str> {
    ensure_gateway_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .reconnect_gateway_session(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_gateway_recoveries(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<GatewayRecovery>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_gateway_recoveries(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn request_gateway_recovery(
    state: State<'_, AppState>,
    request: GatewayRecoveryRequest,
) -> Result<GatewayRecovery, &'static str> {
    ensure_gateway_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .request_gateway_recovery(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn approve_gateway_recovery(
    state: State<'_, AppState>,
    request: GatewayRecoveryApprovalRequest,
) -> Result<GatewayRecovery, &'static str> {
    ensure_gateway_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .approve_gateway_recovery(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_gateway_audit(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<GatewayAuditRecord>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_gateway_audit(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_gateway_revocations(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<GatewayRevocation>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_gateway_revocations(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn revoke_gateway_session(
    state: State<'_, AppState>,
    request: GatewaySessionActionRequest,
) -> Result<GatewayRevocation, &'static str> {
    ensure_gateway_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .revoke_gateway_session(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn revoke_gateway_transfer(
    state: State<'_, AppState>,
    request: GatewayTransferActionRequest,
) -> Result<GatewayRevocation, &'static str> {
    ensure_gateway_mutation_allowed(state.inner(), &request.agent_id, request.temporary_chat)?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .revoke_gateway_transfer(request)
        .map_err(|error| error.code())
}

fn ensure_screen_vision_mutation_allowed(
    state: &AppState,
    agent_id: &str,
    requested_temporary: bool,
) -> Result<(), &'static str> {
    ensure_conversation_not_temporary(state, agent_id, requested_temporary)
}

#[tauri::command]
fn list_screen_vision_fixtures(
    state: State<'_, AppState>,
) -> Result<Vec<ScreenVisionFixture>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_screen_vision_fixtures()
        .map_err(|error| error.code())
}

#[tauri::command]
fn get_screen_vision_provider_status(state: State<'_, AppState>) -> ScreenVisionProviderStatus {
    let Some(database) = state.database.as_ref() else {
        return ScreenVisionProviderStatus {
            state: "unavailable".into(),
        };
    };
    screen_vision::screen_vision_provider_status(database)
}

#[tauri::command]
fn list_screen_vision_sessions(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<ScreenVisionSession>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_screen_vision_sessions(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_screen_vision_jobs(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<ScreenVisionJob>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_screen_vision_jobs(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn list_screen_vision_audit(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<ScreenVisionAuditRecord>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .list_screen_vision_audit(&agent_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn create_screen_vision_session(
    state: State<'_, AppState>,
    request: ScreenVisionSessionRequest,
) -> Result<ScreenVisionSession, &'static str> {
    ensure_screen_vision_mutation_allowed(
        state.inner(),
        &request.agent_id,
        request.temporary_chat,
    )?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .create_screen_vision_session(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn preview_screen_vision_job(
    state: State<'_, AppState>,
    request: ScreenVisionJobPreviewRequest,
) -> Result<ScreenVisionJob, &'static str> {
    ensure_screen_vision_mutation_allowed(
        state.inner(),
        &request.agent_id,
        request.temporary_chat,
    )?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .preview_screen_vision_job(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn confirm_screen_vision_job(
    state: State<'_, AppState>,
    request: ScreenVisionJobConfirmationRequest,
) -> Result<ScreenVisionAnalysisResult, &'static str> {
    ensure_screen_vision_mutation_allowed(
        state.inner(),
        &request.agent_id,
        request.temporary_chat,
    )?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .confirm_screen_vision_job(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn cancel_screen_vision_job(
    state: State<'_, AppState>,
    request: ScreenVisionJobCancellationRequest,
) -> Result<ScreenVisionJob, &'static str> {
    ensure_screen_vision_mutation_allowed(
        state.inner(),
        &request.agent_id,
        request.temporary_chat,
    )?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .cancel_screen_vision_job(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn cleanup_screen_vision_job(
    state: State<'_, AppState>,
    request: ScreenVisionJobCleanupRequest,
) -> Result<ScreenVisionJob, &'static str> {
    ensure_screen_vision_mutation_allowed(
        state.inner(),
        &request.agent_id,
        request.temporary_chat,
    )?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .cleanup_screen_vision_job(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn cancel_screen_vision_session(
    state: State<'_, AppState>,
    request: ScreenVisionSessionCancellationRequest,
) -> Result<ScreenVisionSession, &'static str> {
    ensure_screen_vision_mutation_allowed(
        state.inner(),
        &request.agent_id,
        request.temporary_chat,
    )?;
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .cancel_screen_vision_session(request)
        .map_err(|error| error.code())
}

#[tauri::command]
fn get_app_snapshot(state: State<'_, AppState>) -> Result<AppSnapshot, &'static str> {
    snapshot(&state)
}

#[tauri::command]
fn set_safe_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<AppSnapshot, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .set_safe_mode(enabled)
        .map_err(|_| "operation_failed")?;
    state.safe_mode.store(enabled, Ordering::SeqCst);
    if enabled {
        if let Some(chat) = &state.chat {
            chat.cancel_all("safe_mode_active");
        }
        overlays::clear_native_regions(&app, &state.overlay_input);
        state.runtime.enter_safe_mode();
        overlays::set_visible(&app, &state.overlay_input, false);
    } else {
        state.runtime.leave_safe_mode();
        overlays::set_visible(&app, &state.overlay_input, true);
    }
    snapshot(&state)
}

#[tauri::command]
fn get_phase_one_state(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<PhaseOneState, &'static str> {
    state
        .chat
        .as_ref()
        .ok_or("operation_unavailable")?
        .state(&agent_id)
}

#[tauri::command]
fn get_temporary_phase_one_state(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<PhaseOneState, &'static str> {
    state
        .chat
        .as_ref()
        .ok_or("operation_unavailable")?
        .temporary_state(&agent_id)
}

#[tauri::command]
fn load_phase_one_messages(
    state: State<'_, AppState>,
    agent_id: String,
    conversation_id: String,
) -> Result<Vec<ConversationMessage>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .messages(&agent_id, &conversation_id)
        .map_err(|_| "operation_unavailable")
}

#[tauri::command]
fn list_agent_conversations(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<PhaseOneConversation>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .conversations(&agent_id)
        .map_err(|_| "operation_unavailable")
}

#[tauri::command]
fn list_archived_agent_conversations(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<PhaseOneConversation>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .archived_conversations(&agent_id)
        .map_err(|_| "operation_unavailable")
}

#[tauri::command]
fn create_agent_conversation(
    state: State<'_, AppState>,
    agent_id: String,
    title: String,
) -> Result<PhaseOneConversation, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .create_conversation(&agent_id, &title)
        .map_err(|_| "invalid_conversation")
}

#[tauri::command]
fn set_active_agent_conversation(
    state: State<'_, AppState>,
    agent_id: String,
    conversation_id: String,
) -> Result<(), &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .set_active_conversation(&agent_id, &conversation_id)
        .map_err(|_| "invalid_conversation")
}

#[tauri::command]
fn rename_agent_conversation(
    state: State<'_, AppState>,
    agent_id: String,
    conversation_id: String,
    title: String,
) -> Result<(), &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .rename_conversation(&agent_id, &conversation_id, &title)
        .map_err(|_| "invalid_conversation")
}

#[tauri::command]
fn auto_title_phase_one_conversation(
    state: State<'_, AppState>,
    agent_id: String,
    conversation_id: String,
) -> Result<Option<String>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .auto_title_conversation(&agent_id, &conversation_id)
        .map_err(|error| error.code())
}

#[tauri::command]
fn archive_agent_conversation(
    state: State<'_, AppState>,
    agent_id: String,
    conversation_id: String,
) -> Result<(), &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .archive_conversation(&agent_id, &conversation_id)
        .map_err(|_| "invalid_conversation")
}

#[tauri::command]
fn restore_agent_conversation(
    state: State<'_, AppState>,
    agent_id: String,
    conversation_id: String,
) -> Result<(), &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .restore_conversation(&agent_id, &conversation_id)
        .map_err(|_| "invalid_conversation")
}

#[tauri::command]
fn pin_agent_conversation(
    state: State<'_, AppState>,
    agent_id: String,
    conversation_id: String,
    pinned: bool,
) -> Result<(), &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .set_conversation_pinned(&agent_id, &conversation_id, pinned)
        .map_err(|_| "invalid_conversation")
}

#[tauri::command]
fn delete_agent_conversation(
    state: State<'_, AppState>,
    agent_id: String,
    conversation_id: String,
) -> Result<(), &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .delete_conversation(&agent_id, &conversation_id)
        .map_err(|_| "invalid_conversation")
}

#[tauri::command]
fn list_agent_memories(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<AgentMemory>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .memories(&agent_id)
        .map_err(|_| "operation_unavailable")
}

#[tauri::command]
fn search_agent_memories(
    state: State<'_, AppState>,
    agent_id: String,
    query: Option<String>,
    status: Option<String>,
    category: Option<String>,
    source_type: Option<String>,
) -> Result<Vec<AgentMemory>, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .search_memories(
            &agent_id,
            query.as_deref(),
            status.as_deref(),
            category.as_deref(),
            source_type.as_deref(),
        )
        .map_err(|_| "invalid_memory")
}

#[tauri::command]
fn create_agent_memory(
    state: State<'_, AppState>,
    agent_id: String,
    category: String,
    content: String,
    confirmed: Option<bool>,
) -> Result<AgentMemory, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .create_memory(&agent_id, &category, &content, confirmed.unwrap_or(true))
        .map_err(|_| "invalid_memory")
}

#[tauri::command]
fn send_temporary_phase_one_message(
    state: State<'_, AppState>,
    agent_id: String,
    content: String,
    policy: Option<RoutingPolicy>,
) -> Result<SendMessageResult, &'static str> {
    state
        .chat
        .as_ref()
        .ok_or("operation_unavailable")?
        .send_temporary_message_with_policy(&agent_id, &content, routing_policy_or_default(policy))
}

#[tauri::command]
fn close_temporary_phase_one_chat(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<(), &'static str> {
    state
        .chat
        .as_ref()
        .ok_or("operation_unavailable")?
        .reset_temporary(&agent_id)
}

#[tauri::command]
fn persist_temporary_phase_one_chat(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<PhaseOneConversation, &'static str> {
    state
        .chat
        .as_ref()
        .ok_or("operation_unavailable")?
        .persist_temporary(&agent_id)
}

#[tauri::command]
fn set_temporary_phase_one_model(
    state: State<'_, AppState>,
    agent_id: String,
    model_ref: Option<String>,
) -> Result<(), &'static str> {
    state
        .chat
        .as_ref()
        .ok_or("operation_unavailable")?
        .set_temporary_model(&agent_id, model_ref.as_deref())
}

#[tauri::command]
fn get_agent_simulated_state(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<AgentSimulatedState, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .simulated_state(&agent_id)
        .map_err(|_| "operation_unavailable")
}

#[tauri::command]
fn set_agent_simulated_mode(
    state: State<'_, AppState>,
    agent_id: String,
    mode: String,
) -> Result<(), &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .set_agent_mode(&agent_id, &mode)
        .map_err(|_| "invalid_mode")
}

#[tauri::command]
fn set_agent_suspension(
    state: State<'_, AppState>,
    agent_id: String,
    suspended: bool,
) -> Result<(), &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .set_agent_suspended(&agent_id, suspended)
        .map_err(|_| "operation_unavailable")
}

#[tauri::command]
fn wake_agent_now(state: State<'_, AppState>, agent_id: String) -> Result<(), &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .wake_agent_now(&agent_id, database::now_millis() + 60 * 60 * 1000)
        .map_err(|_| "operation_unavailable")
}

#[tauri::command]
fn load_pixel_document(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<String, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .pixel_document(&agent_id)
        .map_err(|_| "operation_unavailable")
}

#[tauri::command]
fn save_pixel_document(
    app: AppHandle,
    state: State<'_, AppState>,
    agent_id: String,
    source_json: String,
) -> Result<(), &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .save_pixel_document(&agent_id, &source_json)
        .map_err(|_| "invalid_pixel_document")?;
    app.emit("pixel-document-updated", agent_id)
        .map_err(|_| "operation_failed")
}

#[tauri::command]
fn set_agent_memory_status(
    state: State<'_, AppState>,
    agent_id: String,
    memory_id: String,
    status: String,
) -> Result<(), &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .set_memory_status(&agent_id, &memory_id, &status)
        .map_err(|_| "invalid_memory")
}

#[tauri::command]
fn update_agent_memory(
    state: State<'_, AppState>,
    agent_id: String,
    memory_id: String,
    category: String,
    content: String,
) -> Result<(), &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .update_memory(&agent_id, &memory_id, &category, &content)
        .map_err(|_| "invalid_memory")
}

#[tauri::command]
fn refresh_ollama_models(state: State<'_, AppState>) -> Result<(), &'static str> {
    state
        .chat
        .as_ref()
        .ok_or("operation_unavailable")?
        .refresh_models()
}

#[tauri::command]
fn get_ollama_status(state: State<'_, AppState>) -> Result<ProviderSnapshot, &'static str> {
    state
        .chat
        .as_ref()
        .map(ChatCoordinator::provider_snapshot)
        .ok_or("operation_unavailable")
}

#[tauri::command]
fn select_phase_one_model(
    state: State<'_, AppState>,
    agent_id: String,
    model_ref: String,
) -> Result<(), &'static str> {
    state
        .chat
        .as_ref()
        .ok_or("operation_unavailable")?
        .select_model(&agent_id, &model_ref)
}

#[tauri::command]
fn update_keep_alive(
    state: State<'_, AppState>,
    agent_id: String,
    minutes: u32,
) -> Result<(), &'static str> {
    state
        .chat
        .as_ref()
        .ok_or("operation_unavailable")?
        .set_keep_alive(&agent_id, minutes)
}

#[tauri::command]
fn update_agent_profile(
    state: State<'_, AppState>,
    agent: domain::ProvisionalAgent,
) -> Result<(), &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .update_profile(&agent)
        .map_err(|_| "invalid_profile")
}

#[tauri::command]
fn complete_phase_two_onboarding(
    state: State<'_, AppState>,
    agents: Vec<domain::ProvisionalAgent>,
) -> Result<(), &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .complete_onboarding(&agents)
        .map_err(|_| "invalid_profile")
}

#[tauri::command]
fn set_conversation_model_override(
    state: State<'_, AppState>,
    agent_id: String,
    conversation_id: String,
    model_ref: Option<String>,
) -> Result<(), &'static str> {
    state
        .chat
        .as_ref()
        .ok_or("operation_unavailable")?
        .set_conversation_override(&agent_id, &conversation_id, model_ref.as_deref())
}

#[tauri::command]
fn send_phase_one_message(
    state: State<'_, AppState>,
    agent_id: String,
    conversation_id: String,
    content: String,
    policy: Option<RoutingPolicy>,
) -> Result<SendMessageResult, &'static str> {
    state
        .chat
        .as_ref()
        .ok_or("operation_unavailable")?
        .send_message_with_policy(
            &agent_id,
            &conversation_id,
            &content,
            routing_policy_or_default(policy),
        )
}

#[tauri::command]
fn regenerate_phase_one_message(
    state: State<'_, AppState>,
    agent_id: String,
    conversation_id: String,
    assistant_message_id: String,
    model_ref: Option<String>,
    request_id: String,
) -> Result<SendMessageResult, &'static str> {
    state
        .chat
        .as_ref()
        .ok_or("operation_unavailable")?
        .regenerate_message(
            &agent_id,
            &conversation_id,
            &assistant_message_id,
            model_ref.as_deref(),
            &request_id,
        )
}

#[tauri::command]
fn edit_phase_one_message(
    state: State<'_, AppState>,
    agent_id: String,
    conversation_id: String,
    user_message_id: String,
    content: String,
) -> Result<SendMessageResult, &'static str> {
    state
        .chat
        .as_ref()
        .ok_or("operation_unavailable")?
        .edit_message(&agent_id, &conversation_id, &user_message_id, &content)
}

#[tauri::command]
fn set_active_conversation_branch(
    state: State<'_, AppState>,
    agent_id: String,
    conversation_id: String,
    branch_id: String,
) -> Result<(), &'static str> {
    state
        .chat
        .as_ref()
        .ok_or("operation_unavailable")?
        .select_branch(&agent_id, &conversation_id, &branch_id)
}

#[tauri::command]
fn cancel_phase_one_generation(
    state: State<'_, AppState>,
    request_id: String,
) -> Result<(), &'static str> {
    state
        .chat
        .as_ref()
        .ok_or("operation_unavailable")?
        .cancel(&request_id)
}

#[tauri::command]
fn retry_phase_one_runtime(state: State<'_, AppState>) -> Result<(), &'static str> {
    state
        .chat
        .as_ref()
        .ok_or("operation_unavailable")?
        .retry_runtime();
    Ok(())
}

#[tauri::command]
fn open_agent_conversations(
    app: AppHandle,
    state: State<'_, AppState>,
    agent_id: String,
    conversation_id: Option<String>,
) -> Result<(), &'static str> {
    let database = state.database.as_ref().ok_or("operation_unavailable")?;
    database
        .agent(&agent_id)
        .map_err(|_| "operation_unavailable")?;
    let conversation_id = match conversation_id {
        Some(conversation_id) => {
            database
                .set_active_conversation(&agent_id, &conversation_id)
                .map_err(|_| "operation_unavailable")?;
            conversation_id
        }
        None => {
            database
                .active_conversation(&agent_id)
                .map_err(|_| "operation_unavailable")?
                .id
        }
    };
    let window = app
        .get_webview_window("main")
        .ok_or("operation_unavailable")?;
    window.unminimize().map_err(|_| "operation_failed")?;
    if !window.is_visible().map_err(|_| "operation_failed")? {
        window.show().map_err(|_| "operation_failed")?;
    }
    window.set_focus().map_err(|_| "operation_failed")?;
    app.emit_to(
        "main",
        "open-agent-conversations",
        OpenAgentConversationsEvent {
            agent_id,
            conversation_id,
        },
    )
    .map_err(|_| "operation_failed")
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenAgentConversationsEvent {
    agent_id: String,
    conversation_id: String,
}

#[cfg(test)]
fn workspace_window_action(is_visible: bool) -> &'static str {
    if is_visible {
        "unminimize-and-focus"
    } else {
        "unminimize-show-and-focus"
    }
}

#[tauri::command]
fn move_overlay(
    app: AppHandle,
    state: State<'_, AppState>,
    agent_id: String,
    delta_x: f64,
    delta_y: f64,
) -> Result<(), &'static str> {
    if state.safe_mode.load(Ordering::SeqCst) {
        return Err("operation_unavailable");
    }
    let label = overlays::window_label(&agent_id).ok_or("operation_unavailable")?;
    let window = app
        .get_webview_window(label)
        .ok_or("operation_unavailable")?;
    let position = window.outer_position().map_err(|_| "operation_failed")?;
    let scale = window.scale_factor().map_err(|_| "operation_failed")?;
    let next = overlays::offset_overlay_position(position, scale, delta_x, delta_y)
        .ok_or("operation_unavailable")?;
    window
        .set_position(tauri::Position::Logical(next))
        .map_err(|_| "operation_failed")
}

#[tauri::command]
fn set_overlay_interactive_regions(
    window: WebviewWindow,
    state: State<'_, AppState>,
    agent_id: String,
    regions: Vec<InteractiveRegion>,
) -> Result<(), &'static str> {
    let agent_label = overlays::window_label(&agent_id).ok_or("operation_unavailable")?;
    let bubble_label = overlays::bubble_window_label(&agent_id).ok_or("operation_unavailable")?;
    if window.label() != agent_label && window.label() != bubble_label {
        return Err("operation_unavailable");
    }
    if state.safe_mode.load(Ordering::SeqCst) && !regions.is_empty() {
        return Err("operation_unavailable");
    }
    if overlays::install_regions(&window, window.label(), &state.overlay_input, regions).is_err() {
        let _ = window.hide();
        return Err("operation_failed");
    }
    Ok(())
}

#[tauri::command]
fn set_overlay_bubble_visible(
    app: AppHandle,
    state: State<'_, AppState>,
    agent_id: String,
    visible: bool,
) -> Result<(), &'static str> {
    if state.safe_mode.load(Ordering::SeqCst) && visible {
        return Err("operation_unavailable");
    }
    overlays::set_bubble_visible(&app, &state.overlay_input, &agent_id, visible)
        .map_err(|_| "operation_failed")
}

#[tauri::command]
fn set_overlay_bubble_geometry(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    agent_id: String,
    width: f64,
    height: f64,
) -> Result<(), &'static str> {
    if state.safe_mode.load(Ordering::SeqCst) {
        return Err("operation_unavailable");
    }
    let bubble_label = overlays::bubble_window_label(&agent_id).ok_or("operation_unavailable")?;
    if bubble_label != window.label() {
        return Err("operation_unavailable");
    }
    overlays::set_bubble_geometry(&app, &agent_id, width, height).map_err(|_| "operation_failed")
}

fn snapshot(state: &AppState) -> Result<AppSnapshot, &'static str> {
    let Some(database) = state.database.as_ref() else {
        return Ok(AppSnapshot {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            build_sha: env!("AIP_BUILD_SHA").to_string(),
            build_timestamp: env!("AIP_BUILD_TIMESTAMP").to_string(),
            runtime_packaging_mode: env!("AIP_RUNTIME_PACKAGING_MODE").to_string(),
            safe_mode: true,
            database_ready: false,
            migration_version: 0,
            runtime: state.runtime.snapshot(),
            agents: Vec::new(),
            onboarding_required: false,
        });
    };
    let stored = database.snapshot().map_err(|_| "operation_failed")?;
    Ok(AppSnapshot {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        build_sha: env!("AIP_BUILD_SHA").to_string(),
        build_timestamp: env!("AIP_BUILD_TIMESTAMP").to_string(),
        runtime_packaging_mode: env!("AIP_RUNTIME_PACKAGING_MODE").to_string(),
        safe_mode: stored.safe_mode,
        database_ready: true,
        migration_version: stored.migration_version,
        runtime: state.runtime.snapshot(),
        agents: stored.agents,
        onboarding_required: stored.onboarding_required,
    })
}

fn runtime_source_root(_app: &AppHandle) -> PathBuf {
    #[cfg(debug_assertions)]
    {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../services/runtime/src")
    }
    #[cfg(not(debug_assertions))]
    {
        _app.path()
            .resource_dir()
            .map(|path| path.join("aip-runtime.exe"))
            .unwrap_or_else(|_| PathBuf::from("aip-runtime.exe"))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            let data_directory = app
                .path()
                .app_local_data_dir()
                .map_err(|_| io::Error::other("app_data_unavailable"))?;
            let database = Database::initialize(data_directory.join("database/aip.sqlite3"));
            let force_safe_mode =
                std::env::args_os().any(|argument| argument == std::ffi::OsStr::new("--safe-mode"));
            let (database, stored_safe_mode) = match database {
                Ok(database) => {
                    let stored = database
                        .snapshot()
                        .map_err(|_| io::Error::other("database_unavailable"))?;
                    let safe_mode = stored.safe_mode || force_safe_mode;
                    if safe_mode != stored.safe_mode {
                        database
                            .set_safe_mode(safe_mode)
                            .map_err(|_| io::Error::other("database_unavailable"))?;
                    }
                    (Some(database), safe_mode)
                }
                Err(_) => (None, true),
            };
            let safe_mode = Arc::new(AtomicBool::new(stored_safe_mode));
            let runtime =
                RuntimeController::new(runtime_source_root(app.handle()), stored_safe_mode);
            let overlay_input = OverlayInputState::default();
            let orchestration = Arc::new(Mutex::new(OrchestrationManager::default()));
            let chat = database.as_ref().map(|database| {
                ChatCoordinator::new(
                    app.handle().clone(),
                    database.clone(),
                    runtime.clone(),
                    Arc::clone(&safe_mode),
                    Arc::clone(&orchestration),
                )
            });

            app.manage(AppState {
                database: database.clone(),
                runtime: runtime.clone(),
                voice_runtime: VoiceRuntime::new(),
                chat,
                safe_mode: Arc::clone(&safe_mode),
                overlay_input: overlay_input.clone(),
                companion_transport: Arc::new(Mutex::new(CompanionTransportState::default())),
                gateway_transport: Arc::new(Mutex::new(GatewayTransportState::default())),
            });
            if let Some(database) = database.as_ref() {
                overlays::create_windows(app, database, stored_safe_mode, overlay_input.clone())?;
            }
            fullscreen::spawn_monitor(app.handle().clone(), safe_mode, overlay_input.clone());
            if !stored_safe_mode {
                runtime.start();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_snapshot,
            list_cognitive_traits,
            list_cognitive_events,
            explain_cognitive_event,
            create_owner_trait_correction,
            rollback_cognitive_event,
            list_cognitive_opinions,
            propose_cognitive_opinion,
            correct_cognitive_opinion_evidence,
            set_cognitive_opinion_status,
            recalculate_cognitive_opinion,
            list_cognitive_relationships,
            propose_cognitive_relationship,
            reset_cognitive_relationship,
            rollback_cognitive_relationship,
            list_cognitive_goals,
            create_owner_cognitive_goal,
            propose_agent_cognitive_goal,
            approve_cognitive_goal,
            update_cognitive_goal_status,
            list_fictional_activities,
            start_fictional_activity,
            update_fictional_activity_status,
            list_agent_conversation_policies,
            set_agent_conversation_policy,
            start_agent_conversation,
            append_public_conversation_turn,
            emit_cognitive_candidate,
            reserve_heavy_generation,
            complete_resource_job,
            list_cognitive_conversations,
            inspect_agent_conversation,
            interrupt_agent_conversation,
            list_cognitive_candidates,
            reject_cognitive_candidate,
            get_voice_settings,
            get_voice_provider_status,
            list_local_providers,
            register_local_provider,
            disable_local_provider,
            list_voice_devices,
            update_voice_settings,
            set_custom_voice_consent,
            transcribe_voice_fixture,
            synthesize_voice_fixture,
            detect_voice_wake_word_fixture,
            transcribe_voice_local,
            synthesize_voice_local,
            detect_voice_wake_word_local,
            cancel_voice_operation,
            get_voice_operation_status,
            classify_voice_emotion,
            list_tool_catalog,
            add_workspace_root,
            list_workspace_roots,
            remove_workspace_root,
            create_tool_session,
            list_tool_sessions,
            preview_tool_action,
            approve_tool_action,
            confirm_tool_action,
            execute_tool_action,
            cancel_tool_action,
            compensate_tool_action,
            cancel_tool_session,
            list_tool_audit,
            list_extension_catalog,
            list_extension_proposals,
            create_extension_proposal,
            import_extension_manifest,
            create_agent_extension_proposal,
            review_extension_proposal,
            activate_extension,
            update_extension,
            rollback_extension,
            disable_extension,
            list_extension_audit,
            execute_extension,
            cancel_extension_execution,
            build_extension_package,
            start_companion_transport,
            stop_companion_transport,
            get_companion_transport_status,
            list_companion_devices,
            start_companion_pairing,
            confirm_companion_pairing,
            list_companion_sessions,
            connect_companion_session,
            reconnect_companion_session,
            list_companion_queue,
            preview_companion_queue,
            approve_companion_queue,
            cancel_companion_queue,
            retry_companion_queue,
            list_companion_history,
            list_companion_audit,
            list_companion_key_rotations,
            list_companion_revocations,
            rotate_companion_key,
            revoke_companion_device,
            get_gateway_protocol,
            start_gateway_transport,
            stop_gateway_transport,
            get_gateway_transport_status,
            list_gateway_accounts,
            list_gateway_transfers,
            prepare_gateway_transfer,
            approve_gateway_transfer,
            list_gateway_sessions,
            connect_gateway_session,
            reconnect_gateway_session,
            list_gateway_recoveries,
            request_gateway_recovery,
            approve_gateway_recovery,
            list_gateway_audit,
            list_gateway_revocations,
            revoke_gateway_session,
            revoke_gateway_transfer,
            get_screen_vision_provider_status,
            list_screen_vision_fixtures,
            list_screen_vision_sessions,
            list_screen_vision_jobs,
            list_screen_vision_audit,
            create_screen_vision_session,
            preview_screen_vision_job,
            confirm_screen_vision_job,
            cancel_screen_vision_job,
            cleanup_screen_vision_job,
            cancel_screen_vision_session,
            set_safe_mode,
            get_phase_one_state,
            get_temporary_phase_one_state,
            load_phase_one_messages,
            list_agent_conversations,
            list_archived_agent_conversations,
            create_agent_conversation,
            set_active_agent_conversation,
            rename_agent_conversation,
            auto_title_phase_one_conversation,
            archive_agent_conversation,
            restore_agent_conversation,
            pin_agent_conversation,
            delete_agent_conversation,
            list_agent_memories,
            search_agent_memories,
            create_agent_memory,
            send_temporary_phase_one_message,
            close_temporary_phase_one_chat,
            persist_temporary_phase_one_chat,
            set_temporary_phase_one_model,
            get_agent_simulated_state,
            set_agent_simulated_mode,
            set_agent_suspension,
            wake_agent_now,
            load_pixel_document,
            save_pixel_document,
            set_agent_memory_status,
            update_agent_memory,
            refresh_ollama_models,
            get_ollama_status,
            select_phase_one_model,
            update_keep_alive,
            update_agent_profile,
            complete_phase_two_onboarding,
            set_conversation_model_override,
            send_phase_one_message,
            regenerate_phase_one_message,
            edit_phase_one_message,
            set_active_conversation_branch,
            cancel_phase_one_generation,
            retry_phase_one_runtime,
            open_agent_conversations,
            move_overlay,
            set_overlay_bubble_visible,
            set_overlay_bubble_geometry,
            set_overlay_interactive_regions
        ])
        .build(tauri::generate_context!())
        .expect("AIP desktop initialization failed");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::WindowEvent { label, event, .. } = &event {
            if label == "main" && matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                if let Some(state) = app_handle.try_state::<AppState>() {
                    overlays::close_all(app_handle, &state.overlay_input);
                }
                app_handle.exit(0);
                return;
            }
        }
        if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
            if let Some(state) = app_handle.try_state::<AppState>() {
                overlays::close_all(app_handle, &state.overlay_input);
                if let Ok(mut gateway) = state.gateway_transport.lock() {
                    if let Some(mut handle) = gateway.handle.take() {
                        handle.stop();
                    }
                    gateway.endpoint = None;
                    gateway.pairing = None;
                }
            } else {
                overlays::reset_native_regions(app_handle);
            }
        }
        if matches!(event, tauri::RunEvent::Exit) {
            if let Some(state) = app_handle.try_state::<AppState>() {
                state.runtime.shutdown();
            }
        }
    });
}

#[cfg(test)]
mod conversation_command_tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::{atomic::AtomicBool, Arc, Mutex},
    };

    use uuid::Uuid;

    use super::{
        append_public_conversation_turn_for_state, companion_transport_handler,
        complete_resource_job_for_state, emit_cognitive_candidate_for_state,
        reserve_heavy_generation_for_state, routing_policy_or_default,
        set_custom_voice_consent_for_state, start_agent_conversation_for_state,
        update_voice_settings_for_state, AppState, CompanionTransportState, GatewayTransportState,
        RoutingPolicy,
    };
    use crate::{
        companion::{
            CompanionDeviceActionRequest, CompanionPairingConfirmationRequest,
            CompanionPairingRequest, CompanionPlatform, CompanionSessionRequest,
            COMPANION_FIXTURE_APP_VERSION, COMPANION_FIXTURE_DEVICE_ID,
            COMPANION_FIXTURE_FINGERPRINT, COMPANION_FIXTURE_PAIRING_NONCE,
            COMPANION_PROTOCOL_VERSION,
        },
        companion_transport::{sign_frame, start_secure, Session, WireFrame, PROTOCOL},
        conversation::{
            CognitiveCandidateRequest, ConversationPolicyRequest, ConversationStartRequest,
            HeavyGenerationRequest, PublicConversationTurnRequest, ResourceJobCompletionRequest,
        },
        database::{Database, ASTRA_ID, LUMA_ID},
        overlays::OverlayInputState,
        runtime::RuntimeController,
        voice::{CustomVoiceConsentRequest, VoiceRuntime, VoiceSettingsRequest},
    };

    fn test_path() -> PathBuf {
        std::env::temp_dir().join(format!("aip-command-test-{}", Uuid::now_v7()))
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    fn test_state(path: &Path) -> AppState {
        AppState {
            database: Some(Database::initialize(path).unwrap()),
            runtime: RuntimeController::new(PathBuf::from("test-runtime"), false),
            voice_runtime: VoiceRuntime::new(),
            chat: None,
            safe_mode: Arc::new(AtomicBool::new(false)),
            overlay_input: OverlayInputState::default(),
            companion_transport: Arc::new(Mutex::new(CompanionTransportState::default())),
            gateway_transport: Arc::new(Mutex::new(GatewayTransportState::default())),
        }
    }

    #[test]
    fn workspace_navigation_only_shows_a_hidden_main_window() {
        assert_eq!(
            super::workspace_window_action(false),
            "unminimize-show-and-focus"
        );
        assert_eq!(super::workspace_window_action(true), "unminimize-and-focus");
    }

    #[test]
    fn omitted_chat_routing_policy_keeps_auto_default() {
        assert_eq!(routing_policy_or_default(None), RoutingPolicy::default());
    }

    #[test]
    fn workspace_navigation_event_preserves_agent_and_conversation() {
        let payload = serde_json::to_value(super::OpenAgentConversationsEvent {
            agent_id: "agt_luma_provisional".into(),
            conversation_id: "conversation-luma-2".into(),
        })
        .unwrap();
        assert_eq!(
            payload,
            serde_json::json!({
                "agentId": "agt_luma_provisional",
                "conversationId": "conversation-luma-2"
            })
        );
    }

    fn policy(agent_id: &str, purpose: &str, max_turns: i64) -> ConversationPolicyRequest {
        ConversationPolicyRequest {
            agent_id: agent_id.into(),
            purpose: purpose.into(),
            opted_in: true,
            max_turns,
            max_tokens: 256,
            max_duration_ms: 300_000,
            max_repetitions: 2,
            resource_budget: 20,
            temporary_chat: false,
        }
    }

    #[test]
    fn desktop_companion_authority_round_trip_and_fail_closed() {
        use std::io::{Read, Write};
        use std::net::TcpStream;
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let key = b"authority-test-key".to_vec();
        let handler =
            companion_transport_handler(database.clone(), Arc::new(AtomicBool::new(false)));
        let mut transport =
            start_secure("127.0.0.1:0".parse().unwrap(), false, key.clone(), handler).unwrap();
        fn exchange(address: std::net::SocketAddr, frame: WireFrame) -> Vec<u8> {
            let mut stream = TcpStream::connect(address).unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(1)))
                .unwrap();
            stream
                .write_all(&crate::companion_transport::encode(&frame).unwrap())
                .unwrap();
            let mut output = Vec::new();
            let _ = stream.read_to_end(&mut output);
            output
        }
        fn signed(key: &[u8], kind: &str, counter: u64, payload: String) -> WireFrame {
            let mut frame = WireFrame {
                protocol: PROTOCOL.into(),
                kind: kind.into(),
                client_id: COMPANION_FIXTURE_DEVICE_ID.into(),
                session_id: None,
                nonce: format!("nonce-{counter}"),
                counter,
                payload,
                mac: String::new(),
            };
            frame.mac = sign_frame(key, &frame);
            frame
        }
        let pairing = CompanionPairingRequest {
            agent_id: ASTRA_ID.into(),
            owner_user_id: crate::database::OWNER_ID.into(),
            device_id: COMPANION_FIXTURE_DEVICE_ID.into(),
            platform: CompanionPlatform::Android,
            app_version: COMPANION_FIXTURE_APP_VERSION.into(),
            protocol_version: COMPANION_PROTOCOL_VERSION,
            fingerprint: COMPANION_FIXTURE_FINGERPRINT.into(),
            pairing_nonce_metadata: COMPANION_FIXTURE_PAIRING_NONCE.into(),
            idempotency_key: "transport-pair".into(),
            temporary_chat: false,
        };
        let pair_response = exchange(
            transport.addr,
            signed(&key, "pair", 1, serde_json::to_string(&pairing).unwrap()),
        );
        assert!(!pair_response.is_empty());
        assert_eq!(
            crate::companion_transport::decode(&pair_response)
                .unwrap()
                .kind,
            "pair_result"
        );
        let device = database
            .list_companion_devices(ASTRA_ID)
            .unwrap()
            .pop()
            .unwrap();
        database
            .confirm_companion_pairing(CompanionPairingConfirmationRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: crate::database::OWNER_ID.into(),
                device_id: COMPANION_FIXTURE_DEVICE_ID.into(),
                fingerprint: COMPANION_FIXTURE_FINGERPRINT.into(),
                pairing_nonce_metadata: COMPANION_FIXTURE_PAIRING_NONCE.into(),
                confirmed: true,
                idempotency_key: "transport-pair-confirm".into(),
                temporary_chat: false,
            })
            .unwrap();
        let session = CompanionSessionRequest {
            agent_id: ASTRA_ID.into(),
            owner_user_id: crate::database::OWNER_ID.into(),
            device_id: COMPANION_FIXTURE_DEVICE_ID.into(),
            app_version: COMPANION_FIXTURE_APP_VERSION.into(),
            protocol_version: COMPANION_PROTOCOL_VERSION,
            fingerprint: COMPANION_FIXTURE_FINGERPRINT.into(),
            pairing_nonce_metadata: COMPANION_FIXTURE_PAIRING_NONCE.into(),
            message_nonce_metadata: "transport-message-1".into(),
            replay_counter: 1,
            idempotency_key: "transport-session".into(),
            temporary_chat: false,
        };
        let session_response = exchange(
            transport.addr,
            signed(&key, "session", 3, serde_json::to_string(&session).unwrap()),
        );
        assert!(!session_response.is_empty(), "session response missing");
        let mut verifier = Session::new(&key);
        let session_frame = crate::companion_transport::decode(&session_response).unwrap();
        verifier.authenticate(&session_frame).unwrap();
        assert_eq!(session_frame.kind, "session_result");
        let history_response = exchange(
            transport.addr,
            signed(
                &key,
                "history",
                5,
                format!(r#"{{"agent_id":"{ASTRA_ID}"}}"#),
            ),
        );
        assert!(!history_response.is_empty(), "history response missing");
        let history_frame = crate::companion_transport::decode(&history_response).unwrap();
        verifier.authenticate(&history_frame).unwrap();
        assert_eq!(history_frame.kind, "history_result");
        let mut wrong = signed(
            &key,
            "history",
            7,
            format!(r#"{{"agent_id":"{ASTRA_ID}"}}"#),
        );
        wrong.mac = "0".repeat(64);
        assert!(exchange(transport.addr, wrong).is_empty());
        assert!(exchange(
            transport.addr,
            signed(
                &key,
                "history",
                5,
                format!(r#"{{"agent_id":"{ASTRA_ID}"}}"#)
            )
        )
        .is_empty());
        let temporary = CompanionSessionRequest {
            temporary_chat: true,
            idempotency_key: "transport-temporary".into(),
            ..session.clone()
        };
        let temporary_response = exchange(
            transport.addr,
            signed(
                &key,
                "session",
                9,
                serde_json::to_string(&temporary).unwrap(),
            ),
        );
        assert_eq!(
            crate::companion_transport::decode(&temporary_response)
                .unwrap()
                .kind,
            "error"
        );
        database
            .revoke_companion_device(CompanionDeviceActionRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: crate::database::OWNER_ID.into(),
                device_id: COMPANION_FIXTURE_DEVICE_ID.into(),
                reason: "transport-test".into(),
                idempotency_key: "transport-revoke".into(),
                temporary_chat: false,
            })
            .unwrap();
        let revoked_session = CompanionSessionRequest {
            idempotency_key: "transport-revoked-session".into(),
            ..session.clone()
        };
        let revoked_response = exchange(
            transport.addr,
            signed(
                &key,
                "session",
                11,
                serde_json::to_string(&revoked_session).unwrap(),
            ),
        );
        assert_eq!(
            crate::companion_transport::decode(&revoked_response)
                .unwrap()
                .kind,
            "error"
        );
        let address = transport.addr;
        transport.stop();
        assert!(TcpStream::connect(address).is_err());
        assert!(!device.id.is_empty());
        cleanup(&path);
    }

    #[test]
    fn typed_commands_reach_public_candidate_and_resource_paths() {
        let path = test_path();
        let state = test_state(&path);

        for agent_id in [ASTRA_ID, LUMA_ID] {
            state
                .database
                .as_ref()
                .unwrap()
                .set_conversation_policy(policy(agent_id, "candidate-path", 1))
                .unwrap();
        }
        let candidate_conversation = start_agent_conversation_for_state(
            &state,
            ConversationStartRequest {
                initiator_agent_id: ASTRA_ID.into(),
                participant_agent_id: LUMA_ID.into(),
                purpose: "candidate-path".into(),
                idempotency_key: "command-candidate-start".into(),
                temporary_chat: false,
            },
        )
        .unwrap();
        assert_eq!(
            start_agent_conversation_for_state(
                &state,
                ConversationStartRequest {
                    initiator_agent_id: ASTRA_ID.into(),
                    participant_agent_id: LUMA_ID.into(),
                    purpose: "candidate-path".into(),
                    idempotency_key: "command-temporary-start".into(),
                    temporary_chat: true,
                },
            ),
            Err("conversation_temporary_blocked")
        );
        let completed = append_public_conversation_turn_for_state(
            &state,
            PublicConversationTurnRequest {
                agent_id: ASTRA_ID.into(),
                conversation_id: candidate_conversation.id.clone(),
                speaker_agent_id: ASTRA_ID.into(),
                content: "Public candidate source".into(),
                source_kind: "model_candidate".into(),
                idempotency_key: "command-candidate-turn".into(),
                temporary_chat: false,
            },
        )
        .unwrap();
        assert_eq!(completed.conversation.status, "completed");
        assert_eq!(completed.turns.len(), 1);
        let candidate = emit_cognitive_candidate_for_state(
            &state,
            CognitiveCandidateRequest {
                agent_id: ASTRA_ID.into(),
                conversation_id: candidate_conversation.id,
                candidate_kind: "opinion".into(),
                candidate_json: r#"{"subject":"fictional-topic","stance":0.2}"#.into(),
                idempotency_key: "command-candidate".into(),
            },
        )
        .unwrap();
        assert_eq!(candidate.status, "pending");
        assert_eq!(
            state
                .database
                .as_ref()
                .unwrap()
                .list_cognitive_candidates(ASTRA_ID)
                .unwrap()
                .len(),
            1
        );

        for agent_id in [ASTRA_ID, LUMA_ID] {
            state
                .database
                .as_ref()
                .unwrap()
                .set_conversation_policy(policy(agent_id, "resource-path", 4))
                .unwrap();
        }
        let resource_conversation = start_agent_conversation_for_state(
            &state,
            ConversationStartRequest {
                initiator_agent_id: ASTRA_ID.into(),
                participant_agent_id: LUMA_ID.into(),
                purpose: "resource-path".into(),
                idempotency_key: "command-resource-start".into(),
                temporary_chat: false,
            },
        )
        .unwrap();
        let first = reserve_heavy_generation_for_state(
            &state,
            HeavyGenerationRequest {
                agent_id: ASTRA_ID.into(),
                conversation_id: resource_conversation.id.clone(),
                priority: 50,
                budget_units: 10,
                idempotency_key: "command-heavy-1".into(),
            },
        )
        .unwrap();
        assert_eq!(first.status, "running");
        assert_eq!(
            complete_resource_job_for_state(
                &state,
                ResourceJobCompletionRequest {
                    agent_id: ASTRA_ID.into(),
                    job_id: first.id.clone(),
                    status: "completed".into(),
                    error_code: None,
                    idempotency_key: "command-heavy-temp-finish".into(),
                    temporary_chat: true,
                },
            ),
            Err("conversation_temporary_blocked")
        );
        assert_eq!(
            reserve_heavy_generation_for_state(
                &state,
                HeavyGenerationRequest {
                    agent_id: LUMA_ID.into(),
                    conversation_id: resource_conversation.id.clone(),
                    priority: 50,
                    budget_units: 10,
                    idempotency_key: "command-heavy-2".into(),
                },
            ),
            Err("heavy_generation_busy")
        );
        assert_eq!(
            complete_resource_job_for_state(
                &state,
                ResourceJobCompletionRequest {
                    agent_id: ASTRA_ID.into(),
                    job_id: first.id,
                    status: "completed".into(),
                    error_code: None,
                    idempotency_key: "command-heavy-finish".into(),
                    temporary_chat: false,
                },
            )
            .unwrap()
            .status,
            "completed"
        );

        cleanup(&path);
    }

    #[test]
    fn voice_commands_guard_temporary_chat_and_silent_mode() {
        let path = test_path();
        let state = test_state(&path);
        let request = VoiceSettingsRequest {
            agent_id: ASTRA_ID.into(),
            recognition_model_ref: Some("fixture:stt-v1".into()),
            synthesis_model_ref: Some("fixture:tts-v1".into()),
            input_device_ref: Some("fixture:microphone-1".into()),
            output_device_ref: Some("fixture:speaker-1".into()),
            idempotency_key: "command-voice-settings".into(),
            temporary_chat: false,
        };
        update_voice_settings_for_state(&state, request.clone()).unwrap();
        let mut temporary = request.clone();
        temporary.idempotency_key = "command-voice-temporary".into();
        temporary.temporary_chat = true;
        assert_eq!(
            update_voice_settings_for_state(&state, temporary),
            Err("conversation_temporary_blocked")
        );
        state
            .database
            .as_ref()
            .unwrap()
            .set_agent_mode(ASTRA_ID, "silent")
            .unwrap();
        let mut silent = request;
        silent.idempotency_key = "command-voice-silent".into();
        assert_eq!(
            update_voice_settings_for_state(&state, silent),
            Err("voice_blocked_silent")
        );
        state
            .database
            .as_ref()
            .unwrap()
            .set_agent_mode(ASTRA_ID, "normal")
            .unwrap();
        assert_eq!(
            set_custom_voice_consent_for_state(
                &state,
                CustomVoiceConsentRequest {
                    agent_id: ASTRA_ID.into(),
                    granted: true,
                    custom_voice_ref: Some("fixture:custom-neutral-v1".into()),
                    idempotency_key: "command-voice-consent-temporary".into(),
                    temporary_chat: true,
                },
            ),
            Err("conversation_temporary_blocked")
        );
        cleanup(&path);
    }
}
