use std::collections::HashSet;

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::database::{now_millis, Database, DatabaseError};

pub const SCREEN_VISION_MODEL_FIXTURE_ID: &str = "fixture:visual-model/screen-neutral-v1";
const SCREEN_VISION_RESOURCE_KEY: &str = "reference-gpu";
const MAX_SCREEN_VISION_SESSIONS: i64 = 4;
const MAX_SCREEN_VISION_JOBS: i64 = 8;
const MAX_SCREEN_VISION_DURATION_MS: i64 = 15_000;
const MAX_SCREEN_VISION_REDACTION_RULES: usize = 8;
const MAX_SCREEN_VISION_AUDIT_ROWS: i64 = 100;
const SCREEN_VISION_AUDIT_RETENTION_MS: i64 = 30 * 24 * 60 * 60 * 1_000;
const SCREEN_VISION_PREVIEW_TTL_MS: i64 = 10 * 60 * 1_000;
const MAX_SCREEN_VISION_REQUEST_BYTES: usize = 16_384;
const MAX_SCREEN_VISION_RESULT_BYTES: usize = 1_024;
const MAX_SCREEN_VISION_CAPTURE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenVisionCaptureError {
    Unavailable,
    Cancelled,
    Oversized,
    Failed,
}

pub trait ScreenVisionCaptureProvider {
    fn capture(
        &self,
        fixture: &ScreenVisionFixture,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>, ScreenVisionCaptureError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenVisionAdapterError {
    Unavailable,
    Cancelled,
}

pub trait ScreenVisionVisualAdapter {
    fn analyze(
        &self,
        pixels: &[u8],
        cancelled: &dyn Fn() -> bool,
    ) -> Result<ScreenVisionHypothesis, ScreenVisionAdapterError>;
}

#[derive(Debug, Default)]
pub struct DeterministicScreenVisionVisualAdapter;

impl ScreenVisionVisualAdapter for DeterministicScreenVisionVisualAdapter {
    fn analyze(
        &self,
        _pixels: &[u8],
        cancelled: &dyn Fn() -> bool,
    ) -> Result<ScreenVisionHypothesis, ScreenVisionAdapterError> {
        if cancelled() {
            return Err(ScreenVisionAdapterError::Cancelled);
        }
        Ok(ScreenVisionHypothesis {
            text: "Hipótese incerta: fixture visual neutra; confirme visualmente.".into(),
            confidence: 42,
            uncertain: true,
            diagnostic: false,
            durable: false,
            sensitive_attribute_inferred: false,
            source: "synthetic_fixture_visual_model".into(),
        })
    }
}

#[derive(Debug, Default)]
pub struct LocalScreenVisionVisualAdapter;

impl ScreenVisionVisualAdapter for LocalScreenVisionVisualAdapter {
    fn analyze(
        &self,
        _pixels: &[u8],
        cancelled: &dyn Fn() -> bool,
    ) -> Result<ScreenVisionHypothesis, ScreenVisionAdapterError> {
        if cancelled() {
            Err(ScreenVisionAdapterError::Cancelled)
        } else {
            Err(ScreenVisionAdapterError::Unavailable)
        }
    }
}

#[derive(Debug, Default)]
pub struct DeterministicScreenVisionCaptureProvider;

impl ScreenVisionCaptureProvider for DeterministicScreenVisionCaptureProvider {
    fn capture(
        &self,
        fixture: &ScreenVisionFixture,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>, ScreenVisionCaptureError> {
        if cancelled() {
            return Err(ScreenVisionCaptureError::Cancelled);
        }
        if !fixture.synthetic || !fixture.metadata_only {
            return Err(ScreenVisionCaptureError::Unavailable);
        }
        Ok(Vec::new())
    }
}

#[cfg(windows)]
#[derive(Debug, Default)]
pub struct WindowsScreenVisionCaptureProvider;

#[cfg(windows)]
impl ScreenVisionCaptureProvider for WindowsScreenVisionCaptureProvider {
    fn capture(
        &self,
        fixture: &ScreenVisionFixture,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<u8>, ScreenVisionCaptureError> {
        use windows_sys::Win32::Graphics::Gdi::{
            BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
            GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
            DIB_RGB_COLORS, HGDIOBJ, RGBQUAD, SRCCOPY,
        };
        if fixture.synthetic || fixture.metadata_only || cancelled() {
            return Err(if cancelled() {
                ScreenVisionCaptureError::Cancelled
            } else {
                ScreenVisionCaptureError::Unavailable
            });
        }
        let width = i32::try_from(fixture.width).map_err(|_| ScreenVisionCaptureError::Failed)?;
        let height = i32::try_from(fixture.height).map_err(|_| ScreenVisionCaptureError::Failed)?;
        if width <= 0 || height <= 0 {
            return Err(ScreenVisionCaptureError::Failed);
        }
        let bytes = usize::try_from(width)
            .ok()
            .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(ScreenVisionCaptureError::Oversized)?;
        if bytes > MAX_SCREEN_VISION_CAPTURE_BYTES {
            return Err(ScreenVisionCaptureError::Oversized);
        }
        unsafe {
            let source = GetDC(std::ptr::null_mut());
            if source.is_null() {
                return Err(ScreenVisionCaptureError::Unavailable);
            }
            let memory = CreateCompatibleDC(source);
            let bitmap = CreateCompatibleBitmap(source, width, height);
            if memory.is_null() || bitmap.is_null() {
                if !memory.is_null() {
                    DeleteDC(memory);
                }
                ReleaseDC(std::ptr::null_mut(), source);
                return Err(ScreenVisionCaptureError::Unavailable);
            }
            let previous = SelectObject(memory, bitmap as HGDIOBJ);
            let copied = BitBlt(memory, 0, 0, width, height, source, 0, 0, SRCCOPY) != 0;
            let mut output = vec![0u8; bytes];
            let mut info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [RGBQUAD {
                    rgbBlue: 0,
                    rgbGreen: 0,
                    rgbRed: 0,
                    rgbReserved: 0,
                }],
            };
            let rows = if copied {
                GetDIBits(
                    memory,
                    bitmap,
                    0,
                    height as u32,
                    output.as_mut_ptr().cast(),
                    &mut info,
                    DIB_RGB_COLORS,
                )
            } else {
                0
            };
            SelectObject(memory, previous);
            DeleteObject(bitmap as HGDIOBJ);
            DeleteDC(memory);
            ReleaseDC(std::ptr::null_mut(), source);
            if cancelled() {
                return Err(ScreenVisionCaptureError::Cancelled);
            }
            if rows == 0 {
                return Err(ScreenVisionCaptureError::Failed);
            }
            Ok(output)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenVisionPermission {
    CaptureFixture,
    AnalyzeFixture,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenVisionSessionStatus {
    Active,
    Cancelled,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenVisionJobStatus {
    Previewed,
    Queued,
    Running,
    Completed,
    Cancelled,
    Failed,
    Cleaned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenVisionModelLifecycle {
    NotLoaded,
    Loading,
    Ready,
    Running,
    Unloaded,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenVisionCleanupStatus {
    Pending,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenVisionRedactionKind {
    ExcludeSensitiveRegions,
    ExcludeTextLikeRegions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenVisionRedactionRule {
    pub kind: ScreenVisionRedactionKind,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenVisionPrivacyPolicy {
    pub exclude_sensitive_content: bool,
    pub redaction_rules: Vec<ScreenVisionRedactionRule>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenVisionFixture {
    pub fixture_id: String,
    pub monitor_id: String,
    pub display_name: String,
    pub width: i64,
    pub height: i64,
    pub scale: f64,
    pub synthetic: bool,
    pub metadata_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenVisionPreview {
    pub fixture_id: String,
    pub monitor_id: String,
    pub display_name: String,
    pub width: i64,
    pub height: i64,
    pub synthetic: bool,
    pub metadata_only: bool,
    pub confirmation_required: bool,
    pub redaction_rule_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenVisionSession {
    pub id: String,
    pub agent_id: String,
    pub owner_user_id: String,
    pub monitor_id: String,
    pub fixture_id: String,
    pub status: ScreenVisionSessionStatus,
    pub permissions: Vec<ScreenVisionPermission>,
    pub privacy: ScreenVisionPrivacyPolicy,
    pub max_jobs: i64,
    pub max_duration_ms: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub closed_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenVisionJob {
    pub id: String,
    pub session_id: String,
    pub agent_id: String,
    pub owner_user_id: String,
    pub monitor_id: String,
    pub fixture_id: String,
    pub model_fixture_id: String,
    pub resource_key: String,
    pub resource_status: String,
    pub status: ScreenVisionJobStatus,
    pub terminal_status: Option<String>,
    pub model_lifecycle: ScreenVisionModelLifecycle,
    pub model_loaded_at: Option<i64>,
    pub model_run_at: Option<i64>,
    pub model_cleanup_at: Option<i64>,
    pub cleanup_status: ScreenVisionCleanupStatus,
    pub preview: ScreenVisionPreview,
    pub privacy: ScreenVisionPrivacyPolicy,
    pub frame_metadata_present: bool,
    pub result_durable: bool,
    pub error_code: Option<String>,
    pub created_at: i64,
    pub queued_at: Option<i64>,
    pub running_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub cleaned_at: Option<i64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenVisionHypothesis {
    pub text: String,
    pub confidence: i64,
    pub uncertain: bool,
    pub diagnostic: bool,
    pub durable: bool,
    pub sensitive_attribute_inferred: bool,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenVisionAnalysisResult {
    pub job: ScreenVisionJob,
    pub hypothesis: ScreenVisionHypothesis,
    pub output_bounded: bool,
    pub screenshot_bytes_persisted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenVisionAuditRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub job_id: Option<String>,
    pub agent_id: String,
    pub event: String,
    pub result: String,
    pub code: Option<String>,
    pub summary: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenVisionSessionRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub monitor_id: String,
    pub fixture_id: String,
    pub permissions: Vec<ScreenVisionPermission>,
    pub privacy: ScreenVisionPrivacyPolicy,
    pub max_jobs: i64,
    pub max_duration_ms: i64,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenVisionJobPreviewRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub session_id: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenVisionJobConfirmationRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub job_id: String,
    pub confirmed: bool,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenVisionJobCancellationRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub job_id: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenVisionJobCleanupRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub job_id: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenVisionSessionCancellationRequest {
    pub agent_id: String,
    pub owner_user_id: String,
    pub session_id: String,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

struct ScreenVisionJobTransitionContext {
    agent_id: String,
    owner_user_id: String,
    job_id: String,
    idempotency_key: String,
    temporary_chat: bool,
    operation: &'static str,
    terminal_status: &'static str,
}

struct ScreenVisionAuditContext<'a> {
    session_id: Option<&'a str>,
    job_id: Option<&'a str>,
    agent_id: &'a str,
    owner_user_id: &'a str,
    event: &'a str,
    result: &'a str,
    code: Option<&'a str>,
    summary: &'a str,
}

struct ScreenVisionIdempotencyContext<'a> {
    owner_user_id: &'a str,
    operation: &'a str,
    idempotency_key: &'a str,
    request_json: &'a str,
    result_kind: &'a str,
    result_id: &'a str,
    created_at: i64,
}

pub fn screen_vision_fixtures() -> Vec<ScreenVisionFixture> {
    let mut fixtures = vec![
        ScreenVisionFixture {
            fixture_id: "fixture:screen/monitor-1/desktop-neutral-v1".into(),
            monitor_id: "monitor-1".into(),
            display_name: "Monitor sintético 1".into(),
            width: 1280,
            height: 720,
            scale: 1.0,
            synthetic: true,
            metadata_only: true,
        },
        ScreenVisionFixture {
            fixture_id: "fixture:screen/monitor-2/desktop-neutral-v1".into(),
            monitor_id: "monitor-2".into(),
            display_name: "Monitor sintético 2".into(),
            width: 1920,
            height: 1080,
            scale: 1.25,
            synthetic: true,
            metadata_only: true,
        },
    ];
    #[cfg(windows)]
    if let Some(display) = windows_primary_display() {
        fixtures.push(display);
    }
    fixtures
}

#[cfg(windows)]
fn windows_primary_display() -> Option<ScreenVisionFixture> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CMONITORS, SM_CXSCREEN, SM_CYSCREEN,
    };
    let monitors = unsafe { GetSystemMetrics(SM_CMONITORS) };
    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    if monitors <= 0 || width <= 0 || height <= 0 {
        return None;
    }
    Some(ScreenVisionFixture {
        fixture_id: "display:primary".into(),
        monitor_id: "display-primary".into(),
        display_name: "Tela principal do Windows".into(),
        width: i64::from(width),
        height: i64::from(height),
        scale: 1.0,
        synthetic: false,
        metadata_only: false,
    })
}

impl Database {
    pub fn list_screen_vision_fixtures(&self) -> Result<Vec<ScreenVisionFixture>, DatabaseError> {
        Ok(screen_vision_fixtures())
    }

    pub fn create_screen_vision_session(
        &self,
        request: ScreenVisionSessionRequest,
    ) -> Result<ScreenVisionSession, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        let agent_id = bounded_reference(&request.agent_id, 96, "screen_vision_agent_invalid")?;
        let owner_user_id =
            bounded_reference(&request.owner_user_id, 96, "screen_vision_owner_required")?;
        let idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let fixture = fixture_for(&request.monitor_id, &request.fixture_id)?;
        validate_permissions(&request.permissions)?;
        validate_privacy(&request.privacy)?;
        validate_quotas(request.max_jobs, request.max_duration_ms)?;
        let request_json = bounded_json(json!({
            "agentId": agent_id,
            "ownerUserId": owner_user_id,
            "monitorId": fixture.monitor_id,
            "fixtureId": fixture.fixture_id,
            "permissions": request.permissions,
            "privacy": request.privacy,
            "maxJobs": request.max_jobs,
            "maxDurationMs": request.max_duration_ms,
        }))?;

        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        cleanup_expired_jobs(&transaction)?;
        ensure_owner_and_mode(&transaction, &agent_id, &owner_user_id)?;
        if let Some(result_id) = existing_idempotency(
            &transaction,
            &owner_user_id,
            "session_create",
            &idempotency_key,
            &request_json,
            "session",
        )? {
            let session = load_session_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(session);
        }
        let active_sessions: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM screen_vision_sessions
             WHERE agent_id = ?1 AND status = 'active'",
            params![agent_id],
            |row| row.get(0),
        )?;
        if active_sessions >= MAX_SCREEN_VISION_SESSIONS {
            return Err(DatabaseError::Cognitive("screen_vision_session_limit"));
        }
        let now = now_millis();
        let session_id = Uuid::now_v7().to_string();
        let privacy_json =
            serde_json::to_string(&request.privacy).map_err(|_| DatabaseError::Unavailable)?;
        transaction.execute(
            "INSERT INTO screen_vision_sessions
             (id, agent_id, owner_user_id, monitor_id, fixture_id, status,
              max_jobs, max_duration_ms, privacy_json, idempotency_key, request_json,
              created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
            params![
                session_id,
                agent_id,
                owner_user_id,
                fixture.monitor_id,
                fixture.fixture_id,
                request.max_jobs,
                request.max_duration_ms,
                privacy_json,
                idempotency_key,
                request_json,
                now,
            ],
        )?;
        for permission in &request.permissions {
            transaction.execute(
                "INSERT INTO screen_vision_session_permissions
                 (session_id, permission, created_at) VALUES (?1, ?2, ?3)",
                params![session_id, permission.as_str(), now],
            )?;
        }
        insert_audit_tx(
            &transaction,
            ScreenVisionAuditContext {
                session_id: Some(&session_id),
                job_id: None,
                agent_id: &agent_id,
                owner_user_id: &owner_user_id,
                event: "session_created",
                result: "accepted",
                code: None,
                summary: "Owner autorizou sessão sintética de visão de tela",
            },
        )?;
        insert_idempotency(
            &transaction,
            ScreenVisionIdempotencyContext {
                owner_user_id: &owner_user_id,
                operation: "session_create",
                idempotency_key: &idempotency_key,
                request_json: &request_json,
                result_kind: "session",
                result_id: &session_id,
                created_at: now,
            },
        )?;
        let session = load_session_tx(&transaction, &session_id)?;
        transaction.commit()?;
        Ok(session)
    }

    pub fn list_screen_vision_sessions(
        &self,
        agent_id: &str,
    ) -> Result<Vec<ScreenVisionSession>, DatabaseError> {
        let agent_id = bounded_reference(agent_id, 96, "screen_vision_agent_invalid")?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_agent_owner(&transaction, &agent_id)?;
        cleanup_expired_jobs(&transaction)?;
        let ids = transaction
            .prepare(
                "SELECT id FROM screen_vision_sessions
                 WHERE agent_id = ?1 ORDER BY updated_at DESC, id DESC LIMIT 32",
            )?
            .query_map(params![agent_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let sessions = ids
            .iter()
            .map(|id| load_session_tx(&transaction, id))
            .collect::<Result<Vec<_>, _>>()?;
        transaction.commit()?;
        Ok(sessions)
    }

    pub fn preview_screen_vision_job(
        &self,
        request: ScreenVisionJobPreviewRequest,
    ) -> Result<ScreenVisionJob, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        let agent_id = bounded_reference(&request.agent_id, 96, "screen_vision_agent_invalid")?;
        let owner_user_id =
            bounded_reference(&request.owner_user_id, 96, "screen_vision_owner_required")?;
        let session_id =
            bounded_reference(&request.session_id, 128, "screen_vision_session_not_found")?;
        let idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let request_json = bounded_json(json!({ "sessionId": session_id }))?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        cleanup_expired_jobs(&transaction)?;
        ensure_owner_and_mode(&transaction, &agent_id, &owner_user_id)?;
        if let Some(result_id) = existing_idempotency(
            &transaction,
            &owner_user_id,
            "job_preview",
            &idempotency_key,
            &request_json,
            "job",
        )? {
            let job = load_job_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(job);
        }
        let session = load_session_tx(&transaction, &session_id)?;
        ensure_session_owner(&session, &agent_id, &owner_user_id)?;
        if session.status != ScreenVisionSessionStatus::Active {
            return Err(DatabaseError::Cognitive("screen_vision_session_cancelled"));
        }
        require_permission(&session, ScreenVisionPermission::CaptureFixture)?;
        require_permission(&session, ScreenVisionPermission::AnalyzeFixture)?;
        let fixture = fixture_for(&session.monitor_id, &session.fixture_id)?;
        let job_count: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM screen_vision_jobs WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;
        if job_count >= session.max_jobs {
            return Err(DatabaseError::Cognitive("screen_vision_job_limit"));
        }
        let now = now_millis();
        let job_id = Uuid::now_v7().to_string();
        let preview = preview_for(&fixture, &session.privacy);
        let preview_json =
            serde_json::to_string(&preview).map_err(|_| DatabaseError::Unavailable)?;
        let redaction_json =
            serde_json::to_string(&session.privacy).map_err(|_| DatabaseError::Unavailable)?;
        let frame_metadata_json = serde_json::to_string(&json!({
            "syntheticFixture": true,
            "fixtureId": fixture.fixture_id,
            "monitorId": fixture.monitor_id,
            "redactionApplied": true,
        }))
        .map_err(|_| DatabaseError::Unavailable)?;
        transaction.execute(
            "INSERT INTO screen_vision_jobs
             (id, session_id, agent_id, owner_user_id, monitor_id, fixture_id,
              model_fixture_id, resource_key, resource_status, status, model_lifecycle,
              cleanup_status, preview_json, redaction_json, frame_metadata_json,
              idempotency_key, request_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'available', 'previewed',
                     'not_loaded', 'pending', ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
            params![
                job_id,
                session_id,
                agent_id,
                owner_user_id,
                fixture.monitor_id,
                fixture.fixture_id,
                SCREEN_VISION_MODEL_FIXTURE_ID,
                SCREEN_VISION_RESOURCE_KEY,
                preview_json,
                redaction_json,
                frame_metadata_json,
                idempotency_key,
                request_json,
                now,
            ],
        )?;
        insert_audit_tx(
            &transaction,
            ScreenVisionAuditContext {
                session_id: Some(&session_id),
                job_id: Some(&job_id),
                agent_id: &agent_id,
                owner_user_id: &owner_user_id,
                event: "job_previewed",
                result: "accepted",
                code: None,
                summary: "Prévia sintética criada; confirmação explícita necessária",
            },
        )?;
        insert_idempotency(
            &transaction,
            ScreenVisionIdempotencyContext {
                owner_user_id: &owner_user_id,
                operation: "job_preview",
                idempotency_key: &idempotency_key,
                request_json: &request_json,
                result_kind: "job",
                result_id: &job_id,
                created_at: now,
            },
        )?;
        let job = load_job_tx(&transaction, &job_id)?;
        transaction.commit()?;
        Ok(job)
    }

    pub fn confirm_screen_vision_job(
        &self,
        request: ScreenVisionJobConfirmationRequest,
    ) -> Result<ScreenVisionAnalysisResult, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        if !request.confirmed {
            return Err(DatabaseError::Cognitive(
                "screen_vision_confirmation_required",
            ));
        }
        let agent_id = bounded_reference(&request.agent_id, 96, "screen_vision_agent_invalid")?;
        let owner_user_id =
            bounded_reference(&request.owner_user_id, 96, "screen_vision_owner_required")?;
        let job_id = bounded_reference(&request.job_id, 128, "screen_vision_job_not_found")?;
        let idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let request_json = bounded_json(json!({
            "jobId": job_id,
            "confirmed": true,
        }))?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        cleanup_expired_jobs(&transaction)?;
        ensure_owner_and_mode(&transaction, &agent_id, &owner_user_id)?;
        if let Some(result_id) = existing_idempotency(
            &transaction,
            &owner_user_id,
            "job_confirm",
            &idempotency_key,
            &request_json,
            "job",
        )? {
            let job = load_job_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return analysis_result_for_job(job);
        }
        let job = load_job_tx(&transaction, &job_id)?;
        ensure_job_owner(&job, &agent_id, &owner_user_id)?;
        if job.status != ScreenVisionJobStatus::Previewed {
            return Err(DatabaseError::Cognitive("screen_vision_job_invalid"));
        }
        let session = load_session_tx(&transaction, &job.session_id)?;
        if session.status != ScreenVisionSessionStatus::Active {
            return Err(DatabaseError::Cognitive("screen_vision_session_cancelled"));
        }
        require_permission(&session, ScreenVisionPermission::AnalyzeFixture)?;
        let fixture = fixture_for(&job.monitor_id, &job.fixture_id)?;
        let resource_busy: bool = transaction.query_row(
            "SELECT EXISTS(
               SELECT 1 FROM screen_vision_jobs
               WHERE resource_key = ?1 AND status IN ('queued', 'running')
             )",
            params![SCREEN_VISION_RESOURCE_KEY],
            |row| row.get(0),
        )?;
        if resource_busy {
            return Err(DatabaseError::Cognitive("screen_vision_resource_busy"));
        }
        if !fixture.synthetic {
            #[cfg(windows)]
            let capture = WindowsScreenVisionCaptureProvider.capture(&fixture, &|| false);
            #[cfg(not(windows))]
            let capture = Err(ScreenVisionCaptureError::Unavailable);
            let pixels = capture.map_err(|error| match error {
                ScreenVisionCaptureError::Cancelled => {
                    DatabaseError::Cognitive("screen_vision_cancelled")
                }
                ScreenVisionCaptureError::Oversized => {
                    DatabaseError::Cognitive("screen_vision_capture_oversized")
                }
                ScreenVisionCaptureError::Unavailable | ScreenVisionCaptureError::Failed => {
                    DatabaseError::Cognitive("screen_vision_unavailable")
                }
            })?;
            let adapter = LocalScreenVisionVisualAdapter;
            let code = match adapter.analyze(&pixels, &|| false) {
                Ok(_) => "screen_vision_model_unavailable",
                Err(ScreenVisionAdapterError::Cancelled) => "screen_vision_cancelled",
                Err(ScreenVisionAdapterError::Unavailable) => "screen_vision_model_unavailable",
            };
            let now = now_millis();
            transaction.execute("UPDATE screen_vision_jobs SET status='cleaned', terminal_status='failed', model_lifecycle='unavailable', resource_status='released', cleanup_status='complete', frame_metadata_json=NULL, error_code=?1, model_cleanup_at=?2, cleaned_at=?2, updated_at=?2 WHERE id=?3 AND status='previewed'", params![code, now, job_id])?;
            insert_audit_tx(
                &transaction,
                ScreenVisionAuditContext {
                    session_id: Some(&job.session_id),
                    job_id: Some(&job_id),
                    agent_id: &agent_id,
                    owner_user_id: &owner_user_id,
                    event: "job_degraded",
                    result: "unavailable",
                    code: Some(code),
                    summary: "Captura real limpa; adaptador visual local indisponível",
                },
            )?;
            transaction.commit()?;
            return Err(DatabaseError::Cognitive(code));
        }
        DeterministicScreenVisionCaptureProvider
            .capture(&fixture, &|| false)
            .map_err(|_| DatabaseError::Cognitive("screen_vision_capture_failed"))?;
        let now = now_millis();
        transaction.execute(
            "UPDATE screen_vision_jobs
             SET status = 'queued', resource_status = 'reserved', queued_at = ?1, updated_at = ?1
             WHERE id = ?2 AND status = 'previewed'",
            params![now, job_id],
        )?;
        transaction.execute(
            "UPDATE screen_vision_jobs
             SET status = 'running', model_lifecycle = 'running',
                 model_loaded_at = ?1, model_run_at = ?1, running_at = ?1, updated_at = ?1
             WHERE id = ?2 AND status = 'queued'",
            params![now, job_id],
        )?;
        transaction.execute(
            "UPDATE screen_vision_jobs
             SET status = 'cleaned', terminal_status = 'completed',
                 model_lifecycle = 'unloaded', resource_status = 'released',
                 cleanup_status = 'complete', frame_metadata_json = NULL,
                 completed_at = ?1, model_cleanup_at = ?1, cleaned_at = ?1, updated_at = ?1
             WHERE id = ?2 AND status = 'running'",
            params![now, job_id],
        )?;
        insert_audit_tx(
            &transaction,
            ScreenVisionAuditContext {
                session_id: Some(&job.session_id),
                job_id: Some(&job_id),
                agent_id: &agent_id,
                owner_user_id: &owner_user_id,
                event: "job_completed",
                result: "synthetic",
                code: None,
                summary: "Modelo visual fixture executado sob demanda",
            },
        )?;
        insert_audit_tx(
            &transaction,
            ScreenVisionAuditContext {
                session_id: Some(&job.session_id),
                job_id: Some(&job_id),
                agent_id: &agent_id,
                owner_user_id: &owner_user_id,
                event: "job_cleaned",
                result: "complete",
                code: None,
                summary: "Metadados transitórios da fixture foram limpos",
            },
        )?;
        insert_idempotency(
            &transaction,
            ScreenVisionIdempotencyContext {
                owner_user_id: &owner_user_id,
                operation: "job_confirm",
                idempotency_key: &idempotency_key,
                request_json: &request_json,
                result_kind: "job",
                result_id: &job_id,
                created_at: now,
            },
        )?;
        let completed = load_job_tx(&transaction, &job_id)?;
        transaction.commit()?;
        analysis_result_for_job(completed)
    }

    pub fn cancel_screen_vision_job(
        &self,
        request: ScreenVisionJobCancellationRequest,
    ) -> Result<ScreenVisionJob, DatabaseError> {
        self.transition_screen_vision_job(ScreenVisionJobTransitionContext {
            agent_id: request.agent_id,
            owner_user_id: request.owner_user_id,
            job_id: request.job_id,
            idempotency_key: request.idempotency_key,
            temporary_chat: request.temporary_chat,
            operation: "job_cancel",
            terminal_status: "cancelled",
        })
    }

    pub fn cleanup_screen_vision_job(
        &self,
        request: ScreenVisionJobCleanupRequest,
    ) -> Result<ScreenVisionJob, DatabaseError> {
        self.transition_screen_vision_job(ScreenVisionJobTransitionContext {
            agent_id: request.agent_id,
            owner_user_id: request.owner_user_id,
            job_id: request.job_id,
            idempotency_key: request.idempotency_key,
            temporary_chat: request.temporary_chat,
            operation: "job_cleanup",
            terminal_status: "cleaned",
        })
    }

    fn transition_screen_vision_job(
        &self,
        context: ScreenVisionJobTransitionContext,
    ) -> Result<ScreenVisionJob, DatabaseError> {
        let ScreenVisionJobTransitionContext {
            agent_id,
            owner_user_id,
            job_id,
            idempotency_key,
            temporary_chat,
            operation,
            terminal_status,
        } = context;
        ensure_not_temporary(temporary_chat)?;
        let agent_id = bounded_reference(&agent_id, 96, "screen_vision_agent_invalid")?;
        let owner_user_id = bounded_reference(&owner_user_id, 96, "screen_vision_owner_required")?;
        let job_id = bounded_reference(&job_id, 128, "screen_vision_job_not_found")?;
        let idempotency_key = valid_idempotency(&idempotency_key)?;
        let request_json = bounded_json(json!({ "jobId": job_id }))?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        cleanup_expired_jobs(&transaction)?;
        ensure_owner_and_mode(&transaction, &agent_id, &owner_user_id)?;
        if let Some(result_id) = existing_idempotency(
            &transaction,
            &owner_user_id,
            operation,
            &idempotency_key,
            &request_json,
            "job",
        )? {
            let job = load_job_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(job);
        }
        let job = load_job_tx(&transaction, &job_id)?;
        ensure_job_owner(&job, &agent_id, &owner_user_id)?;
        if !matches!(
            job.status,
            ScreenVisionJobStatus::Previewed
                | ScreenVisionJobStatus::Queued
                | ScreenVisionJobStatus::Running
                | ScreenVisionJobStatus::Failed
        ) {
            let current = load_job_tx(&transaction, &job_id)?;
            transaction.commit()?;
            return Ok(current);
        }
        let now = now_millis();
        transaction.execute(
            "UPDATE screen_vision_jobs
             SET status = 'cleaned', terminal_status = ?1,
                 model_lifecycle = 'unloaded', resource_status = 'released',
                 cleanup_status = 'complete', frame_metadata_json = NULL,
                 error_code = CASE WHEN ?1 = 'cancelled' THEN 'screen_vision_cancelled' ELSE error_code END,
                 model_cleanup_at = ?2, cleaned_at = ?2, updated_at = ?2
             WHERE id = ?3",
            params![terminal_status, now, job_id],
        )?;
        insert_audit_tx(
            &transaction,
            ScreenVisionAuditContext {
                session_id: Some(&job.session_id),
                job_id: Some(&job_id),
                agent_id: &agent_id,
                owner_user_id: &owner_user_id,
                event: if terminal_status == "cancelled" {
                    "job_cancelled"
                } else {
                    "job_cleaned"
                },
                result: "complete",
                code: None,
                summary: if terminal_status == "cancelled" {
                    "Job de visão sintética cancelado e limpo"
                } else {
                    "Metadados transitórios da fixture limpos pelo Owner"
                },
            },
        )?;
        insert_idempotency(
            &transaction,
            ScreenVisionIdempotencyContext {
                owner_user_id: &owner_user_id,
                operation,
                idempotency_key: &idempotency_key,
                request_json: &request_json,
                result_kind: "job",
                result_id: &job_id,
                created_at: now,
            },
        )?;
        let result = load_job_tx(&transaction, &job_id)?;
        transaction.commit()?;
        Ok(result)
    }

    pub fn cancel_screen_vision_session(
        &self,
        request: ScreenVisionSessionCancellationRequest,
    ) -> Result<ScreenVisionSession, DatabaseError> {
        ensure_not_temporary(request.temporary_chat)?;
        let agent_id = bounded_reference(&request.agent_id, 96, "screen_vision_agent_invalid")?;
        let owner_user_id =
            bounded_reference(&request.owner_user_id, 96, "screen_vision_owner_required")?;
        let session_id =
            bounded_reference(&request.session_id, 128, "screen_vision_session_not_found")?;
        let idempotency_key = valid_idempotency(&request.idempotency_key)?;
        let request_json = bounded_json(json!({ "sessionId": session_id }))?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        cleanup_expired_jobs(&transaction)?;
        ensure_owner_and_mode(&transaction, &agent_id, &owner_user_id)?;
        if let Some(result_id) = existing_idempotency(
            &transaction,
            &owner_user_id,
            "session_cancel",
            &idempotency_key,
            &request_json,
            "session",
        )? {
            let session = load_session_tx(&transaction, &result_id)?;
            transaction.commit()?;
            return Ok(session);
        }
        let session = load_session_tx(&transaction, &session_id)?;
        ensure_session_owner(&session, &agent_id, &owner_user_id)?;
        let now = now_millis();
        transaction.execute(
            "UPDATE screen_vision_sessions
             SET status = 'cancelled', closed_at = ?1, updated_at = ?1
             WHERE id = ?2 AND status = 'active'",
            params![now, session_id],
        )?;
        transaction.execute(
            "UPDATE screen_vision_jobs
             SET status = 'cleaned', terminal_status = 'cancelled',
                 model_lifecycle = 'unloaded', resource_status = 'released',
                 cleanup_status = 'complete', frame_metadata_json = NULL,
                 error_code = 'screen_vision_cancelled', model_cleanup_at = ?1,
                 cleaned_at = ?1, updated_at = ?1
             WHERE session_id = ?2 AND status IN ('previewed', 'queued', 'running', 'failed')",
            params![now, session_id],
        )?;
        insert_audit_tx(
            &transaction,
            ScreenVisionAuditContext {
                session_id: Some(&session_id),
                job_id: None,
                agent_id: &agent_id,
                owner_user_id: &owner_user_id,
                event: "session_cancelled",
                result: "complete",
                code: None,
                summary: "Sessão de visão sintética cancelada pelo Owner",
            },
        )?;
        insert_idempotency(
            &transaction,
            ScreenVisionIdempotencyContext {
                owner_user_id: &owner_user_id,
                operation: "session_cancel",
                idempotency_key: &idempotency_key,
                request_json: &request_json,
                result_kind: "session",
                result_id: &session_id,
                created_at: now,
            },
        )?;
        let result = load_session_tx(&transaction, &session_id)?;
        transaction.commit()?;
        Ok(result)
    }

    pub fn list_screen_vision_jobs(
        &self,
        agent_id: &str,
    ) -> Result<Vec<ScreenVisionJob>, DatabaseError> {
        let agent_id = bounded_reference(agent_id, 96, "screen_vision_agent_invalid")?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        ensure_agent_owner(&transaction, &agent_id)?;
        cleanup_expired_jobs(&transaction)?;
        let ids = transaction
            .prepare(
                "SELECT id FROM screen_vision_jobs
                 WHERE agent_id = ?1 ORDER BY updated_at DESC, id DESC LIMIT 64",
            )?
            .query_map(params![agent_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let jobs = ids
            .iter()
            .map(|id| load_job_tx(&transaction, id))
            .collect::<Result<Vec<_>, _>>()?;
        transaction.commit()?;
        Ok(jobs)
    }

    pub fn list_screen_vision_audit(
        &self,
        agent_id: &str,
    ) -> Result<Vec<ScreenVisionAuditRecord>, DatabaseError> {
        let agent_id = bounded_reference(agent_id, 96, "screen_vision_agent_invalid")?;
        let connection = self.open()?;
        ensure_agent_owner_connection(&connection, &agent_id)?;
        let mut statement = connection.prepare(
            "SELECT id, session_id, job_id, agent_id, event, result, code,
                    details_json, created_at
             FROM screen_vision_audit_log
             WHERE agent_id = ?1 ORDER BY created_at DESC, id DESC LIMIT ?2",
        )?;
        let records = statement
            .query_map(params![agent_id, MAX_SCREEN_VISION_AUDIT_ROWS], |row| {
                let details_json: String = row.get(7)?;
                let summary = serde_json::from_str::<serde_json::Value>(&details_json)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("summary")
                            .and_then(|value| value.as_str())
                            .map(|value| value.to_owned())
                    })
                    .unwrap_or_else(|| "Evento de visão sintética".to_owned());
                Ok(ScreenVisionAuditRecord {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    job_id: row.get(2)?,
                    agent_id: row.get(3)?,
                    event: row.get(4)?,
                    result: row.get(5)?,
                    code: row.get(6)?,
                    summary,
                    created_at: row.get(8)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(records)
    }
}

fn ensure_not_temporary(temporary_chat: bool) -> Result<(), DatabaseError> {
    if temporary_chat {
        Err(DatabaseError::Cognitive("screen_vision_blocked_temporary"))
    } else {
        Ok(())
    }
}

fn ensure_owner_and_mode(
    transaction: &Transaction<'_>,
    agent_id: &str,
    owner_user_id: &str,
) -> Result<(), DatabaseError> {
    ensure_owner_tx(transaction, agent_id, owner_user_id)?;
    let safe_mode = transaction
        .query_row(
            "SELECT value_json FROM app_settings WHERE key = 'safe_mode'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .is_some_and(|value| value == "true");
    if safe_mode {
        return Err(DatabaseError::Cognitive("screen_vision_blocked_safe_mode"));
    }
    let (mode, suspended): (String, bool) = transaction
        .query_row(
            "SELECT mode, suspended FROM agent_simulated_states WHERE agent_id = ?1",
            params![agent_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("screen_vision_agent_invalid"))?;
    if suspended {
        return Err(DatabaseError::Cognitive("screen_vision_blocked_suspended"));
    }
    if mode == "safe" {
        return Err(DatabaseError::Cognitive("screen_vision_blocked_safe_mode"));
    }
    Ok(())
}

fn ensure_owner_tx(
    transaction: &Transaction<'_>,
    agent_id: &str,
    owner_user_id: &str,
) -> Result<(), DatabaseError> {
    let valid: bool = transaction.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM users u
           JOIN agents a ON a.owner_user_id = u.id
           WHERE u.id = ?1 AND u.role = 'owner'
             AND a.id = ?2 AND a.owner_user_id = ?1
         )",
        params![owner_user_id, agent_id],
        |row| row.get(0),
    )?;
    if valid {
        Ok(())
    } else {
        Err(DatabaseError::Cognitive("screen_vision_owner_required"))
    }
}

fn ensure_agent_owner(transaction: &Transaction<'_>, agent_id: &str) -> Result<(), DatabaseError> {
    let valid: bool = transaction.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM users u
           JOIN agents a ON a.owner_user_id = u.id
           WHERE a.id = ?1 AND u.role = 'owner'
         )",
        params![agent_id],
        |row| row.get(0),
    )?;
    if valid {
        Ok(())
    } else {
        Err(DatabaseError::Cognitive("screen_vision_agent_invalid"))
    }
}

fn ensure_agent_owner_connection(
    connection: &Connection,
    agent_id: &str,
) -> Result<(), DatabaseError> {
    let valid: bool = connection.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM users u
           JOIN agents a ON a.owner_user_id = u.id
           WHERE a.id = ?1 AND u.role = 'owner'
         )",
        params![agent_id],
        |row| row.get(0),
    )?;
    if valid {
        Ok(())
    } else {
        Err(DatabaseError::Cognitive("screen_vision_agent_invalid"))
    }
}

fn ensure_session_owner(
    session: &ScreenVisionSession,
    agent_id: &str,
    owner_user_id: &str,
) -> Result<(), DatabaseError> {
    if session.agent_id != agent_id || session.owner_user_id != owner_user_id {
        Err(DatabaseError::OwnershipMismatch)
    } else {
        Ok(())
    }
}

fn ensure_job_owner(
    job: &ScreenVisionJob,
    agent_id: &str,
    owner_user_id: &str,
) -> Result<(), DatabaseError> {
    if job.agent_id != agent_id || job.owner_user_id != owner_user_id {
        Err(DatabaseError::OwnershipMismatch)
    } else {
        Ok(())
    }
}

fn require_permission(
    session: &ScreenVisionSession,
    required: ScreenVisionPermission,
) -> Result<(), DatabaseError> {
    if session.permissions.contains(&required) {
        Ok(())
    } else {
        Err(DatabaseError::Cognitive("screen_vision_permission_invalid"))
    }
}

fn validate_permissions(permissions: &[ScreenVisionPermission]) -> Result<(), DatabaseError> {
    if permissions.len() != 2 {
        return Err(DatabaseError::Cognitive("screen_vision_permission_invalid"));
    }
    let unique = permissions
        .iter()
        .map(ScreenVisionPermission::as_str)
        .collect::<HashSet<_>>();
    if unique.len() != 2
        || !permissions.contains(&ScreenVisionPermission::CaptureFixture)
        || !permissions.contains(&ScreenVisionPermission::AnalyzeFixture)
    {
        return Err(DatabaseError::Cognitive("screen_vision_permission_invalid"));
    }
    Ok(())
}

fn validate_privacy(privacy: &ScreenVisionPrivacyPolicy) -> Result<(), DatabaseError> {
    if !privacy.exclude_sensitive_content
        || privacy.redaction_rules.is_empty()
        || privacy.redaction_rules.len() > MAX_SCREEN_VISION_REDACTION_RULES
        || privacy.redaction_rules.iter().any(|rule| !rule.enabled)
        || !privacy
            .redaction_rules
            .iter()
            .any(|rule| rule.kind == ScreenVisionRedactionKind::ExcludeSensitiveRegions)
    {
        return Err(DatabaseError::Cognitive("screen_vision_privacy_invalid"));
    }
    Ok(())
}

fn validate_quotas(max_jobs: i64, max_duration_ms: i64) -> Result<(), DatabaseError> {
    if !(1..=MAX_SCREEN_VISION_JOBS).contains(&max_jobs)
        || !(100..=MAX_SCREEN_VISION_DURATION_MS).contains(&max_duration_ms)
    {
        return Err(DatabaseError::Cognitive("screen_vision_quota_invalid"));
    }
    Ok(())
}

fn fixture_for(monitor_id: &str, fixture_id: &str) -> Result<ScreenVisionFixture, DatabaseError> {
    screen_vision_fixtures()
        .into_iter()
        .find(|fixture| fixture.monitor_id == monitor_id && fixture.fixture_id == fixture_id)
        .ok_or(DatabaseError::Cognitive("screen_vision_fixture_invalid"))
}

fn preview_for(
    fixture: &ScreenVisionFixture,
    privacy: &ScreenVisionPrivacyPolicy,
) -> ScreenVisionPreview {
    ScreenVisionPreview {
        fixture_id: fixture.fixture_id.clone(),
        monitor_id: fixture.monitor_id.clone(),
        display_name: fixture.display_name.clone(),
        width: fixture.width,
        height: fixture.height,
        synthetic: fixture.synthetic,
        metadata_only: fixture.metadata_only,
        confirmation_required: true,
        redaction_rule_count: privacy.redaction_rules.len(),
    }
}

fn analysis_result_for_job(
    job: ScreenVisionJob,
) -> Result<ScreenVisionAnalysisResult, DatabaseError> {
    let text = if job.monitor_id == "monitor-2" {
        "Hipótese incerta: a fixture representa uma área de trabalho ampla e neutra; confirme visualmente."
    } else {
        "Hipótese incerta: a fixture representa uma área de trabalho neutra; confirme visualmente."
    };
    if text.len() > MAX_SCREEN_VISION_RESULT_BYTES {
        return Err(DatabaseError::Cognitive("screen_vision_result_oversized"));
    }
    let mut hypothesis = DeterministicScreenVisionVisualAdapter
        .analyze(&[], &|| false)
        .map_err(|_| DatabaseError::Cognitive("screen_vision_model_unavailable"))?;
    hypothesis.text = text.into();
    Ok(ScreenVisionAnalysisResult {
        job,
        hypothesis,
        output_bounded: true,
        screenshot_bytes_persisted: false,
    })
}

fn cleanup_expired_jobs(transaction: &Transaction<'_>) -> Result<(), DatabaseError> {
    let cutoff = now_millis() - SCREEN_VISION_PREVIEW_TTL_MS;
    let stale = transaction
        .prepare(
            "SELECT id, session_id, agent_id, owner_user_id FROM screen_vision_jobs
             WHERE status = 'previewed' AND created_at < ?1",
        )?
        .query_map(params![cutoff], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let now = now_millis();
    for (job_id, session_id, agent_id, owner_user_id) in stale {
        transaction.execute(
            "UPDATE screen_vision_jobs
             SET status = 'cleaned', terminal_status = 'expired',
                 model_lifecycle = 'unloaded', resource_status = 'released',
                 cleanup_status = 'complete', frame_metadata_json = NULL,
                 error_code = 'screen_vision_preview_expired', model_cleanup_at = ?1,
                 cleaned_at = ?1, updated_at = ?1
             WHERE id = ?2 AND status = 'previewed'",
            params![now, job_id],
        )?;
        insert_audit_tx(
            transaction,
            ScreenVisionAuditContext {
                session_id: Some(&session_id),
                job_id: Some(&job_id),
                agent_id: &agent_id,
                owner_user_id: &owner_user_id,
                event: "job_auto_cleaned",
                result: "expired",
                code: Some("screen_vision_preview_expired"),
                summary: "Prévia expirada e metadados transitórios limpos automaticamente",
            },
        )?;
    }
    Ok(())
}

fn insert_audit_tx(
    transaction: &Transaction<'_>,
    context: ScreenVisionAuditContext<'_>,
) -> Result<(), DatabaseError> {
    if context.summary.is_empty() || context.summary.len() > 512 {
        return Err(DatabaseError::Cognitive("screen_vision_audit_oversized"));
    }
    let details_json = serde_json::to_string(&json!({ "summary": context.summary }))
        .map_err(|_| DatabaseError::Unavailable)?;
    if details_json.len() > 2_048 {
        return Err(DatabaseError::Cognitive("screen_vision_audit_oversized"));
    }
    let now = now_millis();
    transaction.execute(
        "DELETE FROM screen_vision_audit_log WHERE created_at < ?1",
        params![now - SCREEN_VISION_AUDIT_RETENTION_MS],
    )?;
    transaction.execute(
        "INSERT INTO screen_vision_audit_log
         (id, session_id, job_id, agent_id, owner_user_id, event, result, code,
          details_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            Uuid::now_v7().to_string(),
            context.session_id,
            context.job_id,
            context.agent_id,
            context.owner_user_id,
            context.event,
            context.result,
            context.code,
            details_json,
            now,
        ],
    )?;
    Ok(())
}

fn existing_idempotency(
    transaction: &Transaction<'_>,
    owner_user_id: &str,
    operation: &str,
    idempotency_key: &str,
    request_json: &str,
    result_kind: &str,
) -> Result<Option<String>, DatabaseError> {
    let existing = transaction
        .query_row(
            "SELECT request_json, result_kind, result_id FROM screen_vision_idempotency
             WHERE owner_user_id = ?1 AND operation = ?2 AND idempotency_key = ?3",
            params![owner_user_id, operation, idempotency_key],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?;
    let Some((existing_request, existing_kind, result_id)) = existing else {
        return Ok(None);
    };
    if existing_request != request_json || existing_kind != result_kind {
        return Err(DatabaseError::Cognitive("idempotency_conflict"));
    }
    Ok(Some(result_id))
}

fn insert_idempotency(
    transaction: &Transaction<'_>,
    context: ScreenVisionIdempotencyContext<'_>,
) -> Result<(), DatabaseError> {
    transaction.execute(
        "INSERT INTO screen_vision_idempotency
         (owner_user_id, operation, idempotency_key, request_json, result_kind, result_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            context.owner_user_id,
            context.operation,
            context.idempotency_key,
            context.request_json,
            context.result_kind,
            context.result_id,
            context.created_at,
        ],
    )?;
    Ok(())
}

fn load_session_tx(
    transaction: &Transaction<'_>,
    session_id: &str,
) -> Result<ScreenVisionSession, DatabaseError> {
    let row = transaction
        .query_row(
            "SELECT id, agent_id, owner_user_id, monitor_id, fixture_id, status,
                    max_jobs, max_duration_ms, privacy_json, created_at, updated_at, closed_at
             FROM screen_vision_sessions WHERE id = ?1",
            params![session_id],
            |row| {
                let status: String = row.get(5)?;
                let privacy_json: String = row.get(8)?;
                let privacy = serde_json::from_str(&privacy_json)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    session_status_from_str(&status)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    privacy,
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, Option<i64>>(11)?,
                ))
            },
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("screen_vision_session_not_found"))?;
    let permissions = transaction
        .prepare(
            "SELECT permission FROM screen_vision_session_permissions
             WHERE session_id = ?1 ORDER BY permission",
        )?
        .query_map(params![session_id], |row| row.get::<_, String>(0))?
        .map(|result| result.and_then(|value| permission_from_str(&value)))
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(ScreenVisionSession {
        id: row.0,
        agent_id: row.1,
        owner_user_id: row.2,
        monitor_id: row.3,
        fixture_id: row.4,
        status: row.5,
        permissions,
        privacy: row.8,
        max_jobs: row.6,
        max_duration_ms: row.7,
        created_at: row.9,
        updated_at: row.10,
        closed_at: row.11,
    })
}

fn load_job_tx(
    transaction: &Transaction<'_>,
    job_id: &str,
) -> Result<ScreenVisionJob, DatabaseError> {
    transaction
        .query_row(
            "SELECT id, session_id, agent_id, owner_user_id, monitor_id, fixture_id,
                    model_fixture_id, resource_key, resource_status, status, terminal_status,
                    model_lifecycle, model_loaded_at, model_run_at, model_cleanup_at,
                    cleanup_status, preview_json, redaction_json, frame_metadata_json,
                    error_code, created_at, queued_at, running_at, completed_at, cleaned_at, updated_at
             FROM screen_vision_jobs WHERE id = ?1",
            params![job_id],
            |row| {
                let preview_json: String = row.get(16)?;
                let redaction_json: String = row.get(17)?;
                Ok(ScreenVisionJob {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    agent_id: row.get(2)?,
                    owner_user_id: row.get(3)?,
                    monitor_id: row.get(4)?,
                    fixture_id: row.get(5)?,
                    model_fixture_id: row.get(6)?,
                    resource_key: row.get(7)?,
                    resource_status: row.get(8)?,
                    status: job_status_from_str(&row.get::<_, String>(9)?)?,
                    terminal_status: row.get(10)?,
                    model_lifecycle: model_lifecycle_from_str(&row.get::<_, String>(11)?)?,
                    model_loaded_at: row.get(12)?,
                    model_run_at: row.get(13)?,
                    model_cleanup_at: row.get(14)?,
                    cleanup_status: cleanup_status_from_str(&row.get::<_, String>(15)?)?,
                    preview: serde_json::from_str(&preview_json)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    privacy: serde_json::from_str(&redaction_json)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    frame_metadata_present: row.get::<_, Option<String>>(18)?.is_some(),
                    result_durable: false,
                    error_code: row.get(19)?,
                    created_at: row.get(20)?,
                    queued_at: row.get(21)?,
                    running_at: row.get(22)?,
                    completed_at: row.get(23)?,
                    cleaned_at: row.get(24)?,
                    updated_at: row.get(25)?,
                })
            },
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("screen_vision_job_not_found"))
}

fn bounded_reference(
    value: &str,
    maximum: usize,
    code: &'static str,
) -> Result<String, DatabaseError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > maximum
        || value.contains("..")
        || value.contains('\\')
        || value
            .chars()
            .any(|character| !(character.is_ascii_alphanumeric() || ":._/-".contains(character)))
    {
        return Err(DatabaseError::Cognitive(code));
    }
    Ok(value.into())
}

fn valid_idempotency(value: &str) -> Result<String, DatabaseError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 128
        || value
            .chars()
            .any(|character| !(character.is_ascii_alphanumeric() || ":._-".contains(character)))
    {
        return Err(DatabaseError::Cognitive(
            "screen_vision_idempotency_invalid",
        ));
    }
    Ok(value.into())
}

fn bounded_json(value: serde_json::Value) -> Result<String, DatabaseError> {
    let json = serde_json::to_string(&value).map_err(|_| DatabaseError::Unavailable)?;
    if json.len() > MAX_SCREEN_VISION_REQUEST_BYTES {
        Err(DatabaseError::Cognitive("screen_vision_request_oversized"))
    } else {
        Ok(json)
    }
}

fn permission_from_str(value: &str) -> rusqlite::Result<ScreenVisionPermission> {
    match value {
        "capture_fixture" => Ok(ScreenVisionPermission::CaptureFixture),
        "analyze_fixture" => Ok(ScreenVisionPermission::AnalyzeFixture),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn session_status_from_str(value: &str) -> rusqlite::Result<ScreenVisionSessionStatus> {
    match value {
        "active" => Ok(ScreenVisionSessionStatus::Active),
        "cancelled" => Ok(ScreenVisionSessionStatus::Cancelled),
        "closed" => Ok(ScreenVisionSessionStatus::Closed),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn job_status_from_str(value: &str) -> rusqlite::Result<ScreenVisionJobStatus> {
    match value {
        "previewed" => Ok(ScreenVisionJobStatus::Previewed),
        "queued" => Ok(ScreenVisionJobStatus::Queued),
        "running" => Ok(ScreenVisionJobStatus::Running),
        "completed" => Ok(ScreenVisionJobStatus::Completed),
        "cancelled" => Ok(ScreenVisionJobStatus::Cancelled),
        "failed" => Ok(ScreenVisionJobStatus::Failed),
        "cleaned" => Ok(ScreenVisionJobStatus::Cleaned),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn model_lifecycle_from_str(value: &str) -> rusqlite::Result<ScreenVisionModelLifecycle> {
    match value {
        "not_loaded" => Ok(ScreenVisionModelLifecycle::NotLoaded),
        "loading" => Ok(ScreenVisionModelLifecycle::Loading),
        "ready" => Ok(ScreenVisionModelLifecycle::Ready),
        "running" => Ok(ScreenVisionModelLifecycle::Running),
        "unloaded" => Ok(ScreenVisionModelLifecycle::Unloaded),
        "unavailable" => Ok(ScreenVisionModelLifecycle::Unavailable),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn cleanup_status_from_str(value: &str) -> rusqlite::Result<ScreenVisionCleanupStatus> {
    match value {
        "pending" => Ok(ScreenVisionCleanupStatus::Pending),
        "complete" => Ok(ScreenVisionCleanupStatus::Complete),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

impl ScreenVisionPermission {
    fn as_str(&self) -> &'static str {
        match self {
            Self::CaptureFixture => "capture_fixture",
            Self::AnalyzeFixture => "analyze_fixture",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use uuid::Uuid;

    use super::*;
    use crate::database::{ASTRA_ID, OWNER_ID};

    fn test_path() -> PathBuf {
        std::env::temp_dir().join(format!("aip-screen-vision-test-{}", Uuid::now_v7()))
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    fn privacy() -> ScreenVisionPrivacyPolicy {
        ScreenVisionPrivacyPolicy {
            exclude_sensitive_content: true,
            redaction_rules: vec![ScreenVisionRedactionRule {
                kind: ScreenVisionRedactionKind::ExcludeSensitiveRegions,
                enabled: true,
            }],
        }
    }

    fn session_request(key: &str) -> ScreenVisionSessionRequest {
        ScreenVisionSessionRequest {
            agent_id: ASTRA_ID.into(),
            owner_user_id: OWNER_ID.into(),
            monitor_id: "monitor-1".into(),
            fixture_id: "fixture:screen/monitor-1/desktop-neutral-v1".into(),
            permissions: vec![
                ScreenVisionPermission::CaptureFixture,
                ScreenVisionPermission::AnalyzeFixture,
            ],
            privacy: privacy(),
            max_jobs: 4,
            max_duration_ms: 5_000,
            idempotency_key: key.into(),
            temporary_chat: false,
        }
    }

    #[test]
    fn synthetic_preview_confirmation_and_cleanup_are_metadata_only() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let session = database
            .create_screen_vision_session(session_request("session-1"))
            .unwrap();
        let preview = database
            .preview_screen_vision_job(ScreenVisionJobPreviewRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                session_id: session.id.clone(),
                idempotency_key: "preview-1".into(),
                temporary_chat: false,
            })
            .unwrap();
        assert_eq!(preview.status, ScreenVisionJobStatus::Previewed);
        assert!(preview.frame_metadata_present);
        assert!(preview.preview.confirmation_required);
        let result = database
            .confirm_screen_vision_job(ScreenVisionJobConfirmationRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                job_id: preview.id,
                confirmed: true,
                idempotency_key: "confirm-1".into(),
                temporary_chat: false,
            })
            .unwrap();
        assert_eq!(result.job.status, ScreenVisionJobStatus::Cleaned);
        assert_eq!(
            result.job.model_lifecycle,
            ScreenVisionModelLifecycle::Unloaded
        );
        assert_eq!(result.job.resource_status, "released");
        assert!(!result.job.frame_metadata_present);
        assert!(!result.hypothesis.durable);
        assert!(result.hypothesis.uncertain);
        assert!(!result.hypothesis.diagnostic);
        assert!(!result.hypothesis.sensitive_attribute_inferred);
        assert!(!result.screenshot_bytes_persisted);
        cleanup(&path);
    }

    #[test]
    fn owner_permissions_privacy_and_modes_fail_closed() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let mut temporary = session_request("temporary-1");
        temporary.temporary_chat = true;
        assert_eq!(
            database.create_screen_vision_session(temporary),
            Err(DatabaseError::Cognitive("screen_vision_blocked_temporary"))
        );
        let mut unsafe_privacy = session_request("privacy-1");
        unsafe_privacy.privacy.exclude_sensitive_content = false;
        assert_eq!(
            database.create_screen_vision_session(unsafe_privacy),
            Err(DatabaseError::Cognitive("screen_vision_privacy_invalid"))
        );
        let mut wrong_owner = session_request("owner-1");
        wrong_owner.owner_user_id = "usr_not_owner".into();
        assert_eq!(
            database.create_screen_vision_session(wrong_owner),
            Err(DatabaseError::Cognitive("screen_vision_owner_required"))
        );
        database.set_safe_mode(true).unwrap();
        assert_eq!(
            database.create_screen_vision_session(session_request("safe-1")),
            Err(DatabaseError::Cognitive("screen_vision_blocked_safe_mode"))
        );
        cleanup(&path);
    }

    #[test]
    fn fixture_selection_and_idempotency_are_bounded() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let mut invalid = session_request("fixture-1");
        invalid.monitor_id = "monitor-2".into();
        assert_eq!(
            database.create_screen_vision_session(invalid),
            Err(DatabaseError::Cognitive("screen_vision_fixture_invalid"))
        );
        let first = database
            .create_screen_vision_session(session_request("same-1"))
            .unwrap();
        let replay = database
            .create_screen_vision_session(session_request("same-1"))
            .unwrap();
        assert_eq!(first.id, replay.id);
        let mut conflict = session_request("same-1");
        conflict.monitor_id = "monitor-2".into();
        conflict.fixture_id = "fixture:screen/monitor-2/desktop-neutral-v1".into();
        assert_eq!(
            database.create_screen_vision_session(conflict),
            Err(DatabaseError::Cognitive("idempotency_conflict"))
        );
        cleanup(&path);
    }

    #[test]
    fn cancellation_cleans_jobs_and_releases_the_single_resource() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let first = database
            .create_screen_vision_session(session_request("session-a"))
            .unwrap();
        let second = database
            .create_screen_vision_session({
                let mut request = session_request("session-b");
                request.monitor_id = "monitor-2".into();
                request.fixture_id = "fixture:screen/monitor-2/desktop-neutral-v1".into();
                request
            })
            .unwrap();
        let first_job = database
            .preview_screen_vision_job(ScreenVisionJobPreviewRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                session_id: first.id.clone(),
                idempotency_key: "preview-a".into(),
                temporary_chat: false,
            })
            .unwrap();
        let second_job = database
            .preview_screen_vision_job(ScreenVisionJobPreviewRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                session_id: second.id.clone(),
                idempotency_key: "preview-b".into(),
                temporary_chat: false,
            })
            .unwrap();
        let connection = database.open().unwrap();
        connection
            .execute(
                "UPDATE screen_vision_jobs SET status = 'running', resource_status = 'reserved'
                 WHERE id = ?1",
                params![first_job.id],
            )
            .unwrap();
        assert_eq!(
            database.confirm_screen_vision_job(ScreenVisionJobConfirmationRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                job_id: second_job.id.clone(),
                confirmed: true,
                idempotency_key: "confirm-busy".into(),
                temporary_chat: false,
            }),
            Err(DatabaseError::Cognitive("screen_vision_resource_busy"))
        );
        drop(connection);
        let cancelled = database
            .cancel_screen_vision_job(ScreenVisionJobCancellationRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                job_id: first_job.id,
                idempotency_key: "cancel-a".into(),
                temporary_chat: false,
            })
            .unwrap();
        assert_eq!(cancelled.status, ScreenVisionJobStatus::Cleaned);
        assert_eq!(cancelled.terminal_status.as_deref(), Some("cancelled"));
        assert!(!cancelled.frame_metadata_present);
        let result = database
            .confirm_screen_vision_job(ScreenVisionJobConfirmationRequest {
                agent_id: ASTRA_ID.into(),
                owner_user_id: OWNER_ID.into(),
                job_id: second_job.id,
                confirmed: true,
                idempotency_key: "confirm-b".into(),
                temporary_chat: false,
            })
            .unwrap();
        assert_eq!(result.job.status, ScreenVisionJobStatus::Cleaned);
        cleanup(&path);
    }

    #[test]
    fn capture_providers_are_bounded_and_fail_closed() {
        let fixture = screen_vision_fixtures().remove(0);
        let provider = DeterministicScreenVisionCaptureProvider;
        assert_eq!(provider.capture(&fixture, &|| false), Ok(Vec::new()));
        assert_eq!(
            provider.capture(&fixture, &|| true),
            Err(ScreenVisionCaptureError::Cancelled)
        );
        let real = ScreenVisionFixture {
            synthetic: false,
            metadata_only: false,
            ..fixture
        };
        #[cfg(windows)]
        assert!(matches!(
            WindowsScreenVisionCaptureProvider.capture(&real, &|| false),
            Err(ScreenVisionCaptureError::Unavailable
                | ScreenVisionCaptureError::Oversized
                | ScreenVisionCaptureError::Failed)
        ));
        #[cfg(not(windows))]
        let _ = real;
    }
}
