mod chat;
mod cognitive;
mod database;
mod domain;
mod fullscreen;
mod native_overlay_region;
mod overlays;
mod protocol;
mod runtime;

use std::{
    io,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use chat::ChatCoordinator;
use cognitive::{
    CognitiveGoal, CognitiveOpinion, GoalRequest, OpinionCandidateRequest,
    OpinionEvidenceCorrectionRequest, RelationshipCandidateRequest, RelationshipState,
};
use database::Database;
use domain::{
    AgentMemory, AgentSimulatedState, AppSnapshot, CognitiveEvent, CognitiveEventExplanation,
    CognitiveTrait, ConversationMessage, PhaseOneConversation, PhaseOneState, SendMessageResult,
};
use overlays::{InteractiveRegion, OverlayInputState};
use runtime::RuntimeController;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

struct AppState {
    database: Option<Database>,
    runtime: RuntimeController,
    chat: Option<ChatCoordinator>,
    safe_mode: Arc<AtomicBool>,
    overlay_input: OverlayInputState,
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
) -> Result<CognitiveEvent, &'static str> {
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
) -> Result<CognitiveEvent, &'static str> {
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
) -> Result<CognitiveOpinion, &'static str> {
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
) -> Result<CognitiveOpinion, &'static str> {
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
) -> Result<RelationshipState, &'static str> {
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
) -> Result<RelationshipState, &'static str> {
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
) -> Result<CognitiveGoal, &'static str> {
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
) -> Result<CognitiveGoal, &'static str> {
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
) -> Result<SendMessageResult, &'static str> {
    state
        .chat
        .as_ref()
        .ok_or("operation_unavailable")?
        .send_temporary_message(&agent_id, &content)
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
) -> Result<SendMessageResult, &'static str> {
    state
        .chat
        .as_ref()
        .ok_or("operation_unavailable")?
        .send_message(&agent_id, &conversation_id, &content)
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
fn open_main_conversation(
    app: AppHandle,
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<(), &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .agent(&agent_id)
        .map_err(|_| "operation_unavailable")?;
    let window = app
        .get_webview_window("main")
        .ok_or("operation_unavailable")?;
    window.show().map_err(|_| "operation_failed")?;
    window.set_focus().map_err(|_| "operation_failed")?;
    app.emit_to("main", "open-conversation", agent_id)
        .map_err(|_| "operation_failed")
}

#[tauri::command]
fn start_overlay_drag(
    app: AppHandle,
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<(), &'static str> {
    if state.safe_mode.load(Ordering::SeqCst) {
        return Err("operation_unavailable");
    }
    let label = overlays::window_label(&agent_id).ok_or("operation_unavailable")?;
    let window = app
        .get_webview_window(label)
        .ok_or("operation_unavailable")?;
    window.start_dragging().map_err(|_| "operation_failed")
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
            let chat = database.as_ref().map(|database| {
                ChatCoordinator::new(
                    app.handle().clone(),
                    database.clone(),
                    runtime.clone(),
                    Arc::clone(&safe_mode),
                )
            });

            app.manage(AppState {
                database: database.clone(),
                runtime: runtime.clone(),
                chat,
                safe_mode: Arc::clone(&safe_mode),
                overlay_input: overlay_input.clone(),
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
            set_safe_mode,
            get_phase_one_state,
            get_temporary_phase_one_state,
            load_phase_one_messages,
            list_agent_conversations,
            list_archived_agent_conversations,
            create_agent_conversation,
            set_active_agent_conversation,
            rename_agent_conversation,
            archive_agent_conversation,
            restore_agent_conversation,
            list_agent_memories,
            search_agent_memories,
            create_agent_memory,
            send_temporary_phase_one_message,
            close_temporary_phase_one_chat,
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
            open_main_conversation,
            start_overlay_drag,
            set_overlay_bubble_visible,
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
