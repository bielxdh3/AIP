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
        .data{
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
    ensur