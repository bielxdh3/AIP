#![allow(dead_code)]

use std::{
    collections::HashMap,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::database::{now_millis, Database, DatabaseError, OWNER_ID};

pub const BASE_VOICE_ID: &str = "aip-base-v1";
pub const MAX_VOICE_DEVICES: usize = 32;
const VOICE_SCHEMA_VERSION: i64 = 1;
const MAX_REFERENCE_LENGTH: usize = 160;
const MAX_IDEMPOTENCY_LENGTH: usize = 128;
const MAX_TEXT_BYTES: usize = 4_096;
const MAX_EMOTION_TEXT_BYTES: usize = 2_048;
pub const MAX_VOICE_CAPTURE_DURATION_MS: u64 = 30_000;
const MIN_VOICE_CAPTURE_DURATION_MS: u64 = 100;
const VOICE_SAMPLE_RATE: u32 = 16_000;
const VOICE_CHANNELS: u16 = 1;
const VOICE_BYTES_PER_SAMPLE: usize = 2;
const MAX_VOICE_CAPTURE_BYTES: usize =
    (VOICE_SAMPLE_RATE as usize * VOICE_BYTES_PER_SAMPLE * MAX_VOICE_CAPTURE_DURATION_MS as usize)
        / 1_000;
const MAX_PROVIDER_OUTPUT_BYTES: usize = 1_048_576;
const MAX_TTS_PCM_BYTES: usize =
    (VOICE_SAMPLE_RATE as usize * VOICE_BYTES_PER_SAMPLE * MAX_VOICE_CAPTURE_DURATION_MS as usize)
        / 1_000;
const VOICE_PROVIDER_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceSettings {
    pub agent_id: String,
    pub schema_version: i64,
    pub base_voice_id: String,
    pub base_voice_protected: bool,
    pub custom_voice_ref: Option<String>,
    pub custom_voice_consent: String,
    pub recognition_model_ref: Option<String>,
    pub synthesis_model_ref: Option<String>,
    pub input_device_ref: Option<String>,
    pub output_device_ref: Option<String>,
    pub mode: String,
    pub voice_muted: bool,
    pub silent: bool,
    pub suspended: bool,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceDevice {
    pub schema_version: i64,
    pub reference: String,
    pub direction: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceProviderCheck {
    pub state: String,
    pub reference: Option<String>,
    pub synthetic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceProviderStatus {
    pub recognition: VoiceProviderCheck,
    pub synthesis: VoiceProviderCheck,
}

pub fn list_voice_devices() -> Vec<VoiceDevice> {
    #[cfg(windows)]
    {
        list_windows_voice_devices()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(windows)]
fn list_windows_voice_devices() -> Vec<VoiceDevice> {
    use std::mem::MaybeUninit;
    use windows_sys::Win32::Media::Audio::{
        waveInGetDevCapsW, waveInGetNumDevs, waveOutGetDevCapsW, waveOutGetNumDevs, WAVEINCAPSW,
        WAVEOUTCAPSW,
    };
    fn name(chars: &[u16]) -> String {
        String::from_utf16_lossy(
            &chars[..chars.iter().position(|c| *c == 0).unwrap_or(chars.len())],
        )
        .chars()
        .take(120)
        .collect()
    }
    let mut devices = Vec::new();
    for index in 0..unsafe { waveInGetNumDevs() }.min(MAX_VOICE_DEVICES as u32) {
        let mut caps = MaybeUninit::<WAVEINCAPSW>::zeroed();
        if unsafe {
            waveInGetDevCapsW(
                index as usize,
                caps.as_mut_ptr(),
                std::mem::size_of::<WAVEINCAPSW>() as u32,
            )
        } == 0
        {
            let caps = unsafe { caps.assume_init() };
            let display_name =
                name(unsafe { std::ptr::addr_of!(caps.szPname).read_unaligned() }.as_slice());
            devices.push(VoiceDevice {
                schema_version: 1,
                reference: format!("local:wavein:{index}"),
                direction: "input".into(),
                display_name,
            });
        }
    }
    for index in 0..unsafe { waveOutGetNumDevs() }.min(MAX_VOICE_DEVICES as u32) {
        let mut caps = MaybeUninit::<WAVEOUTCAPSW>::zeroed();
        if unsafe {
            waveOutGetDevCapsW(
                index as usize,
                caps.as_mut_ptr(),
                std::mem::size_of::<WAVEOUTCAPSW>() as u32,
            )
        } == 0
        {
            let caps = unsafe { caps.assume_init() };
            let display_name =
                name(unsafe { std::ptr::addr_of!(caps.szPname).read_unaligned() }.as_slice());
            devices.push(VoiceDevice {
                schema_version: 1,
                reference: format!("local:waveout:{index}"),
                direction: "output".into(),
                display_name,
            });
        }
    }
    devices
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceSettingsRequest {
    pub agent_id: String,
    pub recognition_model_ref: Option<String>,
    pub synthesis_model_ref: Option<String>,
    pub input_device_ref: Option<String>,
    pub output_device_ref: Option<String>,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomVoiceConsentRequest {
    pub agent_id: String,
    pub granted: bool,
    pub custom_voice_ref: Option<String>,
    pub idempotency_key: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceTranscriptionRequest {
    pub agent_id: String,
    pub fixture_id: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceSynthesisRequest {
    pub agent_id: String,
    pub text: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceWakeWordRequest {
    pub agent_id: String,
    pub fixture_id: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceCaptureRequest {
    pub agent_id: String,
    pub operation_id: String,
    pub idempotency_key: String,
    pub duration_ms: u64,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceSynthesisRuntimeRequest {
    pub agent_id: String,
    pub operation_id: String,
    pub idempotency_key: String,
    pub text: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceOperationCancellationRequest {
    pub agent_id: String,
    pub operation_id: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceOperationStatusRequest {
    pub agent_id: String,
    pub operation_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VoiceOperationStartRequest {
    pub agent_id: String,
    pub operation_id: String,
    pub idempotency_key: String,
    pub operation: &'static str,
    pub provider_ref: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceOperationStatus {
    pub operation_id: String,
    pub agent_id: String,
    pub operation: String,
    pub status: String,
    pub code: Option<String>,
    pub provider_ref: Option<String>,
    pub duration_ms: Option<i64>,
    pub raw_audio_persisted: bool,
    pub listener_active: bool,
    pub started_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceRuntimeTranscriptionResult {
    pub operation_id: String,
    pub status: String,
    pub code: Option<String>,
    pub text: Option<String>,
    pub confidence: Option<f64>,
    pub duration_ms: i64,
    pub provider_ref: Option<String>,
    pub source: String,
    pub metadata_only: bool,
    pub raw_audio_persisted: bool,
    pub text_chat_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceRuntimeSynthesisResult {
    pub operation_id: String,
    pub status: String,
    pub code: Option<String>,
    pub voice_ref: String,
    pub duration_ms: i64,
    pub provider_ref: Option<String>,
    pub source: String,
    pub metadata_only: bool,
    pub raw_audio_persisted: bool,
    pub text_chat_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceRuntimeWakeWordResult {
    pub operation_id: String,
    pub status: String,
    pub code: Option<String>,
    pub detected: bool,
    pub capture_duration_ms: i64,
    pub provider_ref: Option<String>,
    pub source: String,
    pub listener_active: bool,
    pub metadata_only: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceEmotionHypothesisRequest {
    pub text: String,
    pub temporary_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceTranscriptionResult {
    pub status: String,
    pub code: Option<String>,
    pub fixture_id: String,
    pub text: Option<String>,
    pub confidence: Option<f64>,
    pub metadata_only: bool,
    pub raw_audio_persisted: bool,
    pub text_chat_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceSynthesisResult {
    pub status: String,
    pub code: Option<String>,
    pub voice_ref: String,
    pub duration_ms: i64,
    pub metadata_only: bool,
    pub raw_audio_persisted: bool,
    pub text_chat_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceWakeWordResult {
    pub status: String,
    pub code: Option<String>,
    pub fixture_id: String,
    pub detected: bool,
    pub listener_active: bool,
    pub metadata_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceEmotionHypothesisResult {
    pub label: String,
    pub confidence: f64,
    pub uncertain: bool,
    pub diagnostic: bool,
    pub source: String,
}

#[derive(Clone, Default)]
pub struct VoiceRuntime {
    cancellations: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl VoiceRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    fn register(&self, operation_id: &str) -> Result<Arc<AtomicBool>, DatabaseError> {
        let mut cancellations = self
            .cancellations
            .lock()
            .map_err(|_| DatabaseError::Unavailable)?;
        if cancellations.contains_key(operation_id) {
            return Err(DatabaseError::Cognitive("voice_operation_busy"));
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        cancellations.insert(operation_id.to_string(), Arc::clone(&cancelled));
        Ok(cancelled)
    }

    fn remove(&self, operation_id: &str) {
        if let Ok(mut cancellations) = self.cancellations.lock() {
            cancellations.remove(operation_id);
        }
    }

    pub fn cancel(&self, operation_id: &str) -> Result<bool, DatabaseError> {
        let cancellations = self
            .cancellations
            .lock()
            .map_err(|_| DatabaseError::Unavailable)?;
        let Some(cancelled) = cancellations.get(operation_id) else {
            return Ok(false);
        };
        cancelled.store(true, Ordering::Release);
        Ok(true)
    }

    pub fn transcribe(
        &self,
        database: &Database,
        request: VoiceCaptureRequest,
    ) -> Result<VoiceRuntimeTranscriptionResult, DatabaseError> {
        let duration_ms = bounded_capture_duration(request.duration_ms)?;
        let settings = database.voice_settings(&request.agent_id)?;
        if request.temporary_chat {
            return Err(DatabaseError::Cognitive("conversation_temporary_blocked"));
        }
        let provider_ref = settings.recognition_model_ref.clone();
        let operation = database.start_voice_operation(VoiceOperationStartRequest {
            agent_id: request.agent_id.clone(),
            operation_id: request.operation_id.clone(),
            idempotency_key: request.idempotency_key.clone(),
            operation: "transcription",
            provider_ref: provider_ref.clone(),
            duration_ms: Some(duration_ms),
        })?;
        if operation.status != "started" {
            return Ok(runtime_transcription_degraded(
                &request.operation_id,
                provider_ref,
                "voice_operation_replayed",
                duration_ms,
            ));
        }

        let cancelled = match self.register(&request.operation_id) {
            Ok(cancelled) => cancelled,
            Err(error) => {
                let _ = database.finish_voice_operation(
                    &request.agent_id,
                    &request.operation_id,
                    "degraded",
                    Some(error.code()),
                    Some(duration_ms),
                );
                return Ok(runtime_transcription_degraded(
                    &request.operation_id,
                    provider_ref,
                    error.code(),
                    duration_ms,
                ));
            }
        };

        let result =
            self.transcribe_registered(database, &request, duration_ms, provider_ref, &cancelled);
        self.remove(&request.operation_id);
        result
    }

    fn transcribe_registered(
        &self,
        database: &Database,
        request: &VoiceCaptureRequest,
        duration_ms: u64,
        provider_ref: Option<String>,
        cancelled: &AtomicBool,
    ) -> Result<VoiceRuntimeTranscriptionResult, DatabaseError> {
        let result: Result<VoiceRuntimeTranscriptionResult, DatabaseError> = (|| {
            if let Some(code) = voice_runtime_block_code(request, database)? {
                return Ok(runtime_transcription_degraded(
                    &request.operation_id,
                    provider_ref.clone(),
                    code,
                    duration_ms,
                ));
            }
            let provider_ref = provider_ref
                .as_deref()
                .ok_or(DatabaseError::Cognitive("voice_model_unavailable"))?;
            let provider = provider_executable(provider_ref, ProviderKind::Transcription)
                .map_err(DatabaseError::Cognitive)?;
            let pcm = capture_pcm(
                request_input_device(database, &request.agent_id)?,
                duration_ms,
                cancelled,
            )
            .map_err(DatabaseError::Cognitive)?;
            let output = run_local_provider(
                &provider,
                &[
                    "--input-format",
                    "pcm_s16le",
                    "--sample-rate",
                    "16000",
                    "--channels",
                    "1",
                    "--output-format",
                    "json",
                ],
                &pcm,
                MAX_PROVIDER_OUTPUT_BYTES,
                cancelled,
            )
            .map_err(DatabaseError::Cognitive)?;
            let transcription = parse_transcription_output(&output)?;
            Ok(VoiceRuntimeTranscriptionResult {
                operation_id: request.operation_id.clone(),
                status: "completed".into(),
                code: None,
                text: Some(transcription.text),
                confidence: transcription.confidence,
                duration_ms: duration_ms as i64,
                provider_ref: Some(provider_ref.to_string()),
                source: "local_provider".into(),
                metadata_only: false,
                raw_audio_persisted: false,
                text_chat_fallback: false,
            })
        })();

        match result {
            Ok(result) if result.status == "completed" => {
                database.finish_voice_operation(
                    &request.agent_id,
                    &request.operation_id,
                    "completed",
                    None,
                    Some(duration_ms),
                )?;
                Ok(result)
            }
            Ok(result) => {
                let code = result.code.as_deref().unwrap_or("voice_model_unavailable");
                let status = if code == "voice_operation_cancelled" {
                    "cancelled"
                } else {
                    "degraded"
                };
                database.finish_voice_operation(
                    &request.agent_id,
                    &request.operation_id,
                    status,
                    Some(code),
                    Some(duration_ms),
                )?;
                Ok(result)
            }
            Err(error) => {
                let code = error.code();
                let status = if code == "voice_operation_cancelled" {
                    "cancelled"
                } else {
                    "degraded"
                };
                database.finish_voice_operation(
                    &request.agent_id,
                    &request.operation_id,
                    status,
                    Some(code),
                    Some(duration_ms),
                )?;
                Ok(runtime_transcription_degraded(
                    &request.operation_id,
                    provider_ref,
                    code,
                    duration_ms,
                ))
            }
        }
    }

    pub fn synthesize(
        &self,
        database: &Database,
        request: VoiceSynthesisRuntimeRequest,
    ) -> Result<VoiceRuntimeSynthesisResult, DatabaseError> {
        let text = bounded_text(&request.text, MAX_TEXT_BYTES, "voice_input_invalid")?;
        let settings = database.voice_settings(&request.agent_id)?;
        if request.temporary_chat {
            return Err(DatabaseError::Cognitive("conversation_temporary_blocked"));
        }
        let provider_ref = settings.synthesis_model_ref.clone();
        let voice_ref = settings
            .custom_voice_ref
            .clone()
            .unwrap_or_else(|| settings.base_voice_id.clone());
        let operation = database.start_voice_operation(VoiceOperationStartRequest {
            agent_id: request.agent_id.clone(),
            operation_id: request.operation_id.clone(),
            idempotency_key: request.idempotency_key.clone(),
            operation: "synthesis",
            provider_ref: provider_ref.clone(),
            duration_ms: None,
        })?;
        if operation.status != "started" {
            return Ok(runtime_synthesis_degraded(
                &request.operation_id,
                voice_ref,
                provider_ref,
                "voice_operation_replayed",
            ));
        }
        let cancelled = match self.register(&request.operation_id) {
            Ok(cancelled) => cancelled,
            Err(error) => {
                let _ = database.finish_voice_operation(
                    &request.agent_id,
                    &request.operation_id,
                    "degraded",
                    Some(error.code()),
                    None,
                );
                return Ok(runtime_synthesis_degraded(
                    &request.operation_id,
                    voice_ref,
                    provider_ref,
                    error.code(),
                ));
            }
        };
        let result = self.synthesize_registered(
            database,
            &request,
            text,
            voice_ref,
            provider_ref,
            &cancelled,
        );
        self.remove(&request.operation_id);
        result
    }

    fn synthesize_registered(
        &self,
        database: &Database,
        request: &VoiceSynthesisRuntimeRequest,
        text: &str,
        voice_ref: String,
        provider_ref: Option<String>,
        cancelled: &AtomicBool,
    ) -> Result<VoiceRuntimeSynthesisResult, DatabaseError> {
        let result: Result<VoiceRuntimeSynthesisResult, DatabaseError> = (|| {
            if let Some(code) = voice_runtime_block_code(
                &VoiceCaptureRequest {
                    agent_id: request.agent_id.clone(),
                    operation_id: request.operation_id.clone(),
                    idempotency_key: request.idempotency_key.clone(),
                    duration_ms: 100,
                    temporary_chat: request.temporary_chat,
                },
                database,
            )? {
                return Ok(runtime_synthesis_degraded(
                    &request.operation_id,
                    voice_ref.clone(),
                    provider_ref.clone(),
                    code,
                ));
            }
            let settings = database.voice_settings(&request.agent_id)?;
            if settings.voice_muted {
                return Ok(runtime_synthesis_degraded(
                    &request.operation_id,
                    voice_ref.clone(),
                    provider_ref.clone(),
                    "voice_muted",
                ));
            }
            let provider_ref = provider_ref
                .as_deref()
                .ok_or(DatabaseError::Cognitive("voice_model_unavailable"))?;
            let provider = provider_executable(provider_ref, ProviderKind::Synthesis)
                .map_err(DatabaseError::Cognitive)?;
            let output = run_local_provider(
                &provider,
                &[
                    "--input-format",
                    "utf8",
                    "--output-format",
                    "pcm_s16le",
                    "--sample-rate",
                    "16000",
                    "--channels",
                    "1",
                    "--voice-ref",
                    &voice_ref,
                ],
                text.as_bytes(),
                MAX_TTS_PCM_BYTES,
                cancelled,
            )
            .map_err(DatabaseError::Cognitive)?;
            let output = validate_pcm_output(output)?;
            let output_device = request_output_device(database, &request.agent_id)?;
            play_pcm(output_device, &output, cancelled).map_err(DatabaseError::Cognitive)?;
            let duration_ms = (output.len() as i64 * 1_000)
                / (VOICE_SAMPLE_RATE as i64 * VOICE_BYTES_PER_SAMPLE as i64);
            Ok(VoiceRuntimeSynthesisResult {
                operation_id: request.operation_id.clone(),
                status: "completed".into(),
                code: None,
                voice_ref: voice_ref.clone(),
                duration_ms,
                provider_ref: Some(provider_ref.to_string()),
                source: "local_provider".into(),
                metadata_only: false,
                raw_audio_persisted: false,
                text_chat_fallback: false,
            })
        })();

        match result {
            Ok(result) if result.status == "completed" => {
                database.finish_voice_operation(
                    &request.agent_id,
                    &request.operation_id,
                    "completed",
                    None,
                    Some(result.duration_ms as u64),
                )?;
                Ok(result)
            }
            Ok(result) => {
                let code = result.code.as_deref().unwrap_or("voice_model_unavailable");
                let status = if code == "voice_operation_cancelled" {
                    "cancelled"
                } else {
                    "degraded"
                };
                database.finish_voice_operation(
                    &request.agent_id,
                    &request.operation_id,
                    status,
                    Some(code),
                    None,
                )?;
                Ok(result)
            }
            Err(error) => {
                let code = error.code();
                let status = if code == "voice_operation_cancelled" {
                    "cancelled"
                } else {
                    "degraded"
                };
                database.finish_voice_operation(
                    &request.agent_id,
                    &request.operation_id,
                    status,
                    Some(code),
                    None,
                )?;
                Ok(runtime_synthesis_degraded(
                    &request.operation_id,
                    voice_ref,
                    provider_ref,
                    code,
                ))
            }
        }
    }

    pub fn detect_wake_word(
        &self,
        database: &Database,
        request: VoiceCaptureRequest,
    ) -> Result<VoiceRuntimeWakeWordResult, DatabaseError> {
        let duration_ms = bounded_capture_duration(request.duration_ms)?;
        let settings = database.voice_settings(&request.agent_id)?;
        if request.temporary_chat {
            return Err(DatabaseError::Cognitive("conversation_temporary_blocked"));
        }
        let provider_ref = settings.recognition_model_ref.clone();
        let operation = database.start_voice_operation(VoiceOperationStartRequest {
            agent_id: request.agent_id.clone(),
            operation_id: request.operation_id.clone(),
            idempotency_key: request.idempotency_key.clone(),
            operation: "wake_word",
            provider_ref: provider_ref.clone(),
            duration_ms: Some(duration_ms),
        })?;
        if operation.status != "started" {
            return Ok(runtime_wake_degraded(
                &request.operation_id,
                provider_ref,
                "voice_operation_replayed",
                duration_ms,
            ));
        }
        let cancelled = match self.register(&request.operation_id) {
            Ok(cancelled) => cancelled,
            Err(error) => {
                let _ = database.finish_voice_operation(
                    &request.agent_id,
                    &request.operation_id,
                    "degraded",
                    Some(error.code()),
                    Some(duration_ms),
                );
                return Ok(runtime_wake_degraded(
                    &request.operation_id,
                    provider_ref,
                    error.code(),
                    duration_ms,
                ));
            }
        };
        let result =
            self.detect_wake_registered(database, &request, duration_ms, provider_ref, &cancelled);
        self.remove(&request.operation_id);
        result
    }

    fn detect_wake_registered(
        &self,
        database: &Database,
        request: &VoiceCaptureRequest,
        duration_ms: u64,
        provider_ref: Option<String>,
        cancelled: &AtomicBool,
    ) -> Result<VoiceRuntimeWakeWordResult, DatabaseError> {
        let result: Result<VoiceRuntimeWakeWordResult, DatabaseError> = (|| {
            let settings = database.voice_settings(&request.agent_id)?;
            if settings.suspended {
                return Ok(runtime_wake_degraded(
                    &request.operation_id,
                    provider_ref.clone(),
                    "voice_blocked_suspended",
                    duration_ms,
                ));
            }
            if settings.silent {
                return Ok(runtime_wake_degraded(
                    &request.operation_id,
                    provider_ref.clone(),
                    "voice_blocked_silent",
                    duration_ms,
                ));
            }
            let provider_ref = provider_ref
                .as_deref()
                .ok_or(DatabaseError::Cognitive("voice_model_unavailable"))?;
            let provider = provider_executable(provider_ref, ProviderKind::Transcription)
                .map_err(DatabaseError::Cognitive)?;
            let pcm = capture_pcm(
                request_input_device(database, &request.agent_id)?,
                duration_ms,
                cancelled,
            )
            .map_err(DatabaseError::Cognitive)?;
            let output = run_local_provider(
                &provider,
                &[
                    "--input-format",
                    "pcm_s16le",
                    "--sample-rate",
                    "16000",
                    "--channels",
                    "1",
                    "--output-format",
                    "json",
                    "--wake-word",
                    "aip",
                ],
                &pcm,
                MAX_PROVIDER_OUTPUT_BYTES,
                cancelled,
            )
            .map_err(DatabaseError::Cognitive)?;
            let wake = parse_wake_output(&output)?;
            Ok(VoiceRuntimeWakeWordResult {
                operation_id: request.operation_id.clone(),
                status: if wake.detected { "detected" } else { "ignored" }.into(),
                code: None,
                detected: wake.detected,
                capture_duration_ms: duration_ms as i64,
                provider_ref: Some(provider_ref.to_string()),
                source: "local_provider".into(),
                listener_active: false,
                metadata_only: false,
            })
        })();
        match result {
            Ok(result) if result.code.is_none() => {
                database.finish_voice_operation(
                    &request.agent_id,
                    &request.operation_id,
                    "completed",
                    None,
                    Some(duration_ms),
                )?;
                Ok(result)
            }
            Ok(result) => {
                let code = result.code.as_deref().unwrap_or("voice_model_unavailable");
                let status = if code == "voice_operation_cancelled" {
                    "cancelled"
                } else {
                    "degraded"
                };
                database.finish_voice_operation(
                    &request.agent_id,
                    &request.operation_id,
                    status,
                    Some(code),
                    Some(duration_ms),
                )?;
                Ok(result)
            }
            Err(error) => {
                let code = error.code();
                let status = if code == "voice_operation_cancelled" {
                    "cancelled"
                } else {
                    "degraded"
                };
                database.finish_voice_operation(
                    &request.agent_id,
                    &request.operation_id,
                    status,
                    Some(code),
                    Some(duration_ms),
                )?;
                Ok(runtime_wake_degraded(
                    &request.operation_id,
                    provider_ref,
                    code,
                    duration_ms,
                ))
            }
        }
    }
}

impl Database {
    pub fn voice_settings(&self, agent_id: &str) -> Result<VoiceSettings, DatabaseError> {
        let connection = self.open()?;
        ensure_owner(&connection, agent_id)?;
        connection
            .query_row(voice_settings_sql(), params![agent_id], map_voice_settings)
            .optional()?
            .ok_or(DatabaseError::Cognitive("voice_settings_not_found"))
    }

    pub fn voice_provider_status(
        &self,
        agent_id: &str,
    ) -> Result<VoiceProviderStatus, DatabaseError> {
        let settings = self.voice_settings(agent_id)?;
        Ok(VoiceProviderStatus {
            recognition: provider_check(
                settings.recognition_model_ref.as_deref(),
                ProviderKind::Transcription,
            ),
            synthesis: provider_check(
                settings.synthesis_model_ref.as_deref(),
                ProviderKind::Synthesis,
            ),
        })
    }

    pub fn start_voice_operation(
        &self,
        request: VoiceOperationStartRequest,
    ) -> Result<VoiceOperationStatus, DatabaseError> {
        let operation_id = idempotency(&request.operation_id)?;
        let idempotency_key = idempotency(&request.idempotency_key)?;
        if !matches!(
            request.operation,
            "transcription" | "synthesis" | "wake_word"
        ) {
            return Err(DatabaseError::Cognitive("voice_operation_invalid"));
        }
        let provider_ref = request
            .provider_ref
            .as_deref()
            .map(|reference| optional_reference(Some(reference), "voice_reference_invalid"))
            .transpose()?
            .flatten();
        let duration_ms = request
            .duration_ms
            .map(bounded_capture_duration)
            .transpose()?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let owner_id = ensure_owner_tx(&transaction, &request.agent_id)?;
        let existing = transaction
            .query_row(
                "SELECT id, agent_id, operation, status, code, provider_ref,
                        duration_ms, started_at, completed_at
                 FROM voice_operation_records
                 WHERE agent_id = ?1 AND idempotency_key = ?2",
                params![request.agent_id, idempotency_key],
                map_voice_operation,
            )
            .optional()?;
        if let Some(existing) = existing {
            if existing.operation != request.operation || existing.operation_id != operation_id {
                return Err(DatabaseError::Cognitive("idempotency_conflict"));
            }
            transaction.commit()?;
            return Ok(existing);
        }
        let now = now_millis();
        transaction.execute(
            "INSERT INTO voice_operation_records
             (id, agent_id, owner_user_id, operation, status, code, provider_ref,
              duration_ms, idempotency_key, started_at, completed_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'started', NULL, ?5, ?6, ?7, ?8, NULL, ?8)",
            params![
                operation_id,
                request.agent_id,
                owner_id,
                request.operation,
                provider_ref,
                duration_ms.map(|value| value as i64),
                idempotency_key,
                now,
            ],
        )?;
        let status = VoiceOperationStatus {
            operation_id,
            agent_id: request.agent_id,
            operation: request.operation.into(),
            status: "started".into(),
            code: None,
            provider_ref,
            duration_ms: duration_ms.map(|value| value as i64),
            raw_audio_persisted: false,
            listener_active: false,
            started_at: now,
            completed_at: None,
        };
        transaction.commit()?;
        Ok(status)
    }

    pub fn finish_voice_operation(
        &self,
        agent_id: &str,
        operation_id: &str,
        status: &str,
        code: Option<&str>,
        duration_ms: Option<u64>,
    ) -> Result<VoiceOperationStatus, DatabaseError> {
        let operation_id = idempotency(operation_id)?;
        if !matches!(status, "completed" | "cancelled" | "degraded" | "failed") {
            return Err(DatabaseError::Cognitive("voice_operation_invalid"));
        }
        let code = code.map(str::trim).filter(|value| !value.is_empty());
        if code.is_some_and(|value| value.len() > 96) {
            return Err(DatabaseError::Cognitive("voice_operation_invalid"));
        }
        let duration_ms = duration_ms
            .map(|value| {
                if value > MAX_VOICE_CAPTURE_DURATION_MS {
                    Err(DatabaseError::Cognitive("voice_capture_limit"))
                } else {
                    Ok(value as i64)
                }
            })
            .transpose()?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let owner_id = ensure_owner_tx(&transaction, agent_id)?;
        let updated = transaction.execute(
            "UPDATE voice_operation_records
             SET status = ?1, code = ?2, duration_ms = COALESCE(?3, duration_ms),
                 completed_at = ?4, updated_at = ?4
             WHERE id = ?5 AND agent_id = ?6 AND owner_user_id = ?7",
            params![
                status,
                code,
                duration_ms,
                now_millis(),
                operation_id,
                agent_id,
                owner_id
            ],
        )?;
        if updated == 0 {
            return Err(DatabaseError::Cognitive("voice_operation_not_found"));
        }
        let status = transaction.query_row(
            "SELECT id, agent_id, operation, status, code, provider_ref,
                    duration_ms, started_at, completed_at
             FROM voice_operation_records
             WHERE id = ?1 AND agent_id = ?2",
            params![operation_id, agent_id],
            map_voice_operation,
        )?;
        transaction.commit()?;
        Ok(status)
    }

    pub fn voice_operation_status(
        &self,
        request: VoiceOperationStatusRequest,
    ) -> Result<VoiceOperationStatus, DatabaseError> {
        let operation_id = idempotency(&request.operation_id)?;
        let connection = self.open()?;
        ensure_owner(&connection, &request.agent_id)?;
        connection
            .query_row(
                "SELECT id, agent_id, operation, status, code, provider_ref,
                        duration_ms, started_at, completed_at
                 FROM voice_operation_records
                 WHERE id = ?1 AND agent_id = ?2",
                params![operation_id, request.agent_id],
                map_voice_operation,
            )
            .optional()?
            .ok_or(DatabaseError::Cognitive("voice_operation_not_found"))
    }

    pub fn ensure_voice_mutation_allowed(&self, agent_id: &str) -> Result<(), DatabaseError> {
        let connection = self.open()?;
        let state = connection
            .query_row(
                "SELECT mode, suspended FROM agent_simulated_states WHERE agent_id = ?1",
                params![agent_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, bool>(1)?)),
            )
            .optional()?
            .ok_or(DatabaseError::Cognitive("agent_not_found"))?;
        ensure_mutation_mode(&state.0, state.1)
    }

    pub fn update_voice_settings(
        &self,
        request: VoiceSettingsRequest,
    ) -> Result<VoiceSettings, DatabaseError> {
        if request.temporary_chat {
            return Err(DatabaseError::Cognitive("conversation_temporary_blocked"));
        }
        let recognition_model_ref = optional_reference(
            request.recognition_model_ref.as_deref(),
            "voice_reference_invalid",
        )?;
        let synthesis_model_ref = optional_reference(
            request.synthesis_model_ref.as_deref(),
            "voice_reference_invalid",
        )?;
        let input_device_ref = optional_reference(
            request.input_device_ref.as_deref(),
            "voice_reference_invalid",
        )?;
        let output_device_ref = optional_reference(
            request.output_device_ref.as_deref(),
            "voice_reference_invalid",
        )?;
        let idempotency_key = idempotency(&request.idempotency_key)?;
        let request_json = serde_json::to_string(&json!({
            "recognitionModelRef": recognition_model_ref,
            "synthesisModelRef": synthesis_model_ref,
            "inputDeviceRef": input_device_ref,
            "outputDeviceRef": output_device_ref,
        }))
        .map_err(|_| DatabaseError::Unavailable)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let owner_id = ensure_owner_tx(&transaction, &request.agent_id)?;
        ensure_mutation_mode_tx(&transaction, &request.agent_id)?;
        if let Some(existing) =
            load_mutation_event(&transaction, &request.agent_id, &idempotency_key)?
        {
            if existing.operation != "settings" || existing.request_json != request_json {
                return Err(DatabaseError::Cognitive("idempotency_conflict"));
            }
            let settings = load_voice_settings_tx(&transaction, &request.agent_id)?;
            transaction.commit()?;
            return Ok(settings);
        }
        let now = now_millis();
        transaction.execute(
            "UPDATE agent_voice_settings
             SET recognition_model_ref = ?1, synthesis_model_ref = ?2,
                 input_device_ref = ?3, output_device_ref = ?4, updated_at = ?5
             WHERE agent_id = ?6 AND owner_user_id = ?7",
            params![
                recognition_model_ref,
                synthesis_model_ref,
                input_device_ref,
                output_device_ref,
                now,
                request.agent_id,
                owner_id,
            ],
        )?;
        let settings = load_voice_settings_tx(&transaction, &request.agent_id)?;
        insert_mutation_event(
            &transaction,
            &request.agent_id,
            &owner_id,
            "settings",
            &idempotency_key,
            &request_json,
            &settings,
        )?;
        transaction.commit()?;
        Ok(settings)
    }

    pub fn set_custom_voice_consent(
        &self,
        request: CustomVoiceConsentRequest,
    ) -> Result<VoiceSettings, DatabaseError> {
        if request.temporary_chat {
            return Err(DatabaseError::Cognitive("conversation_temporary_blocked"));
        }
        let custom_voice_ref = if request.granted {
            Some(custom_voice_reference(request.custom_voice_ref.as_deref())?)
        } else {
            if request.custom_voice_ref.is_some() {
                return Err(DatabaseError::Cognitive("voice_consent_invalid"));
            }
            None
        };
        let consent = if request.granted {
            "granted"
        } else {
            "revoked"
        };
        let idempotency_key = idempotency(&request.idempotency_key)?;
        let request_json = serde_json::to_string(&json!({
            "granted": request.granted,
            "customVoiceRef": custom_voice_ref,
        }))
        .map_err(|_| DatabaseError::Unavailable)?;
        let mut connection = self.open()?;
        let transaction = connection.transaction()?;
        let owner_id = ensure_owner_tx(&transaction, &request.agent_id)?;
        ensure_mutation_mode_tx(&transaction, &request.agent_id)?;
        if let Some(existing) =
            load_mutation_event(&transaction, &request.agent_id, &idempotency_key)?
        {
            if existing.operation != "custom_consent" || existing.request_json != request_json {
                return Err(DatabaseError::Cognitive("idempotency_conflict"));
            }
            let settings = load_voice_settings_tx(&transaction, &request.agent_id)?;
            transaction.commit()?;
            return Ok(settings);
        }
        let now = now_millis();
        transaction.execute(
            "UPDATE agent_voice_settings
             SET custom_voice_ref = ?1, custom_voice_consent = ?2, updated_at = ?3
             WHERE agent_id = ?4 AND owner_user_id = ?5",
            params![custom_voice_ref, consent, now, request.agent_id, owner_id,],
        )?;
        let settings = load_voice_settings_tx(&transaction, &request.agent_id)?;
        insert_mutation_event(
            &transaction,
            &request.agent_id,
            &owner_id,
            "custom_consent",
            &idempotency_key,
            &request_json,
            &settings,
        )?;
        transaction.commit()?;
        Ok(settings)
    }

    pub fn transcribe_voice_fixture(
        &self,
        request: VoiceTranscriptionRequest,
    ) -> Result<VoiceTranscriptionResult, DatabaseError> {
        let settings = self.voice_settings(&request.agent_id)?;
        let Some((text, confidence)) = transcription_fixture(&request.fixture_id) else {
            return Ok(transcription_degraded(
                request.fixture_id,
                "voice_fixture_unavailable",
            ));
        };
        if settings.suspended {
            return Ok(transcription_degraded(
                request.fixture_id,
                "voice_blocked_suspended",
            ));
        }
        if settings.input_device_ref.is_none() {
            return Ok(transcription_degraded(
                request.fixture_id,
                "voice_device_unavailable",
            ));
        }
        if settings.recognition_model_ref.is_none() {
            return Ok(transcription_degraded(
                request.fixture_id,
                "voice_model_unavailable",
            ));
        }
        Ok(VoiceTranscriptionResult {
            status: "ready".into(),
            code: None,
            fixture_id: request.fixture_id,
            text: Some(text.into()),
            confidence: Some(confidence),
            metadata_only: true,
            raw_audio_persisted: false,
            text_chat_fallback: false,
        })
    }

    pub fn synthesize_voice_fixture(
        &self,
        request: VoiceSynthesisRequest,
    ) -> Result<VoiceSynthesisResult, DatabaseError> {
        let text = bounded_text(&request.text, MAX_TEXT_BYTES, "voice_input_invalid")?;
        let settings = self.voice_settings(&request.agent_id)?;
        let voice_ref = settings
            .custom_voice_ref
            .clone()
            .unwrap_or_else(|| settings.base_voice_id.clone());
        if settings.suspended {
            return Ok(synthesis_degraded(voice_ref, "voice_blocked_suspended"));
        }
        if settings.voice_muted {
            return Ok(VoiceSynthesisResult {
                status: "muted".into(),
                code: Some("voice_muted".into()),
                voice_ref,
                duration_ms: 0,
                metadata_only: true,
                raw_audio_persisted: false,
                text_chat_fallback: true,
            });
        }
        if settings.output_device_ref.is_none() {
            return Ok(synthesis_degraded(voice_ref, "voice_device_unavailable"));
        }
        if settings.synthesis_model_ref.is_none() {
            return Ok(synthesis_degraded(voice_ref, "voice_model_unavailable"));
        }
        let duration_ms = ((text.chars().count() as i64) * 45).clamp(300, 30_000);
        Ok(VoiceSynthesisResult {
            status: "ready".into(),
            code: None,
            voice_ref,
            duration_ms,
            metadata_only: true,
            raw_audio_persisted: false,
            text_chat_fallback: false,
        })
    }

    pub fn detect_voice_wake_word_fixture(
        &self,
        request: VoiceWakeWordRequest,
    ) -> Result<VoiceWakeWordResult, DatabaseError> {
        let settings = self.voice_settings(&request.agent_id)?;
        if settings.suspended || settings.silent {
            return Ok(VoiceWakeWordResult {
                status: "ignored".into(),
                code: Some(
                    if settings.suspended {
                        "voice_blocked_suspended"
                    } else {
                        "voice_blocked_silent"
                    }
                    .into(),
                ),
                fixture_id: request.fixture_id,
                detected: false,
                listener_active: false,
                metadata_only: true,
            });
        }
        if settings.input_device_ref.is_none() || settings.recognition_model_ref.is_none() {
            return Ok(VoiceWakeWordResult {
                status: "degraded".into(),
                code: Some("voice_device_or_model_unavailable".into()),
                fixture_id: request.fixture_id,
                detected: false,
                listener_active: false,
                metadata_only: true,
            });
        }
        let detected = request.fixture_id == "fixture:wake-aip";
        Ok(VoiceWakeWordResult {
            status: if detected { "detected" } else { "ignored" }.into(),
            code: None,
            fixture_id: request.fixture_id,
            detected,
            listener_active: false,
            metadata_only: true,
        })
    }

    pub fn classify_voice_emotion(
        &self,
        request: VoiceEmotionHypothesisRequest,
    ) -> Result<VoiceEmotionHypothesisResult, DatabaseError> {
        let text = bounded_text(&request.text, MAX_EMOTION_TEXT_BYTES, "voice_input_invalid")?
            .to_lowercase();
        let label = if ["feliz", "obrigado", "ótimo", "otimo", "alegre"]
            .iter()
            .any(|marker| text.contains(marker))
        {
            "positive"
        } else if ["triste", "medo", "preocup", "raiva", "difícil", "dificil"]
            .iter()
            .any(|marker| text.contains(marker))
        {
            "concerned"
        } else {
            "neutral"
        };
        Ok(VoiceEmotionHypothesisResult {
            label: label.into(),
            confidence: 0.55,
            uncertain: true,
            diagnostic: false,
            source: "fixture_text_heuristic".into(),
        })
    }
}

fn map_voice_operation(row: &rusqlite::Row<'_>) -> rusqlite::Result<VoiceOperationStatus> {
    Ok(VoiceOperationStatus {
        operation_id: row.get(0)?,
        agent_id: row.get(1)?,
        operation: row.get(2)?,
        status: row.get(3)?,
        code: row.get(4)?,
        provider_ref: row.get(5)?,
        duration_ms: row.get(6)?,
        raw_audio_persisted: false,
        listener_active: false,
        started_at: row.get(7)?,
        completed_at: row.get(8)?,
    })
}

fn bounded_capture_duration(value: u64) -> Result<u64, DatabaseError> {
    if !(MIN_VOICE_CAPTURE_DURATION_MS..=MAX_VOICE_CAPTURE_DURATION_MS).contains(&value) {
        return Err(DatabaseError::Cognitive("voice_capture_limit"));
    }
    Ok(value)
}

fn voice_runtime_block_code(
    request: &VoiceCaptureRequest,
    database: &Database,
) -> Result<Option<&'static str>, DatabaseError> {
    let settings = database.voice_settings(&request.agent_id)?;
    if settings.suspended {
        return Ok(Some("voice_blocked_suspended"));
    }
    if settings.silent {
        return Ok(Some("voice_blocked_silent"));
    }
    Ok(None)
}

fn request_input_device(database: &Database, agent_id: &str) -> Result<String, DatabaseError> {
    database
        .voice_settings(agent_id)?
        .input_device_ref
        .ok_or(DatabaseError::Cognitive("voice_device_unavailable"))
}

fn request_output_device(database: &Database, agent_id: &str) -> Result<String, DatabaseError> {
    database
        .voice_settings(agent_id)?
        .output_device_ref
        .ok_or(DatabaseError::Cognitive("voice_device_unavailable"))
}

fn runtime_transcription_degraded(
    operation_id: &str,
    provider_ref: Option<String>,
    code: &str,
    duration_ms: u64,
) -> VoiceRuntimeTranscriptionResult {
    VoiceRuntimeTranscriptionResult {
        operation_id: operation_id.into(),
        status: if code == "voice_operation_cancelled" {
            "cancelled"
        } else {
            "degraded"
        }
        .into(),
        code: Some(code.into()),
        text: None,
        confidence: None,
        duration_ms: duration_ms as i64,
        provider_ref,
        source: "local_provider".into(),
        metadata_only: false,
        raw_audio_persisted: false,
        text_chat_fallback: true,
    }
}

fn runtime_synthesis_degraded(
    operation_id: &str,
    voice_ref: String,
    provider_ref: Option<String>,
    code: &str,
) -> VoiceRuntimeSynthesisResult {
    VoiceRuntimeSynthesisResult {
        operation_id: operation_id.into(),
        status: if code == "voice_operation_cancelled" {
            "cancelled"
        } else if code == "voice_muted" {
            "muted"
        } else {
            "degraded"
        }
        .into(),
        code: Some(code.into()),
        voice_ref,
        duration_ms: 0,
        provider_ref,
        source: "local_provider".into(),
        metadata_only: false,
        raw_audio_persisted: false,
        text_chat_fallback: true,
    }
}

fn runtime_wake_degraded(
    operation_id: &str,
    provider_ref: Option<String>,
    code: &str,
    duration_ms: u64,
) -> VoiceRuntimeWakeWordResult {
    VoiceRuntimeWakeWordResult {
        operation_id: operation_id.into(),
        status: if matches!(code, "voice_blocked_silent" | "voice_blocked_suspended") {
            "ignored"
        } else if code == "voice_operation_cancelled" {
            "cancelled"
        } else {
            "degraded"
        }
        .into(),
        code: Some(code.into()),
        detected: false,
        capture_duration_ms: duration_ms as i64,
        provider_ref,
        source: "local_provider".into(),
        listener_active: false,
        metadata_only: false,
    }
}

#[derive(Clone, Copy)]
enum ProviderKind {
    Transcription,
    Synthesis,
}

fn provider_check(reference: Option<&str>, kind: ProviderKind) -> VoiceProviderCheck {
    let Some(reference) = reference else {
        return VoiceProviderCheck {
            state: "not_configured".into(),
            reference: None,
            synthetic: false,
        };
    };
    if reference.starts_with("fixture:") {
        return VoiceProviderCheck {
            state: "ready".into(),
            reference: Some(reference.into()),
            synthetic: true,
        };
    }
    let state = match provider_executable(reference, kind) {
        Ok(_) => "ready",
        Err("voice_model_unavailable") => "unavailable",
        Err(_) => "invalid",
    };
    VoiceProviderCheck {
        state: state.into(),
        reference: Some(reference.into()),
        synthetic: false,
    }
}

fn provider_executable(reference: &str, kind: ProviderKind) -> Result<PathBuf, &'static str> {
    let prefix = match kind {
        ProviderKind::Transcription => "local:stt:",
        ProviderKind::Synthesis => "local:tts:",
    };
    let provider_id = reference
        .strip_prefix(prefix)
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 80
                && value
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
        })
        .ok_or("voice_model_unavailable")?;
    let _ = provider_id;
    let env_name = match kind {
        ProviderKind::Transcription => "AIP_VOICE_STT_PROVIDER_PATH",
        ProviderKind::Synthesis => "AIP_VOICE_TTS_PROVIDER_PATH",
    };
    let configured = std::env::var_os(env_name).ok_or("voice_model_unavailable")?;
    let configured = configured
        .to_str()
        .filter(|value| !value.is_empty() && !value.contains('\0'))
        .ok_or("voice_provider_invalid")?;
    let path = PathBuf::from(configured);
    if !path.is_absolute() || !path.is_file() {
        return Err("voice_provider_invalid");
    }
    #[cfg(windows)]
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_none_or(|extension| !extension.eq_ignore_ascii_case("exe"))
    {
        return Err("voice_provider_invalid");
    }
    Ok(path)
}

fn run_local_provider(
    executable: &Path,
    args: &[&str],
    input: &[u8],
    maximum_output: usize,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, &'static str> {
    if cancelled.load(Ordering::Acquire) {
        return Err("voice_operation_cancelled");
    }
    if input.len() > MAX_VOICE_CAPTURE_BYTES || maximum_output > MAX_PROVIDER_OUTPUT_BYTES {
        return Err("voice_provider_output_invalid");
    }
    let mut child = Command::new(executable)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "voice_model_unavailable")?;
    let mut stdin = child.stdin.take();
    let input = input.to_vec();
    let writer = thread::spawn(move || {
        if let Some(mut stdin) = stdin.take() {
            stdin.write_all(&input).is_ok()
        } else {
            false
        }
    });
    let stdout = child.stdout.take().ok_or("voice_provider_invalid")?;
    let reader = thread::spawn(move || {
        let mut output = Vec::new();
        let mut bounded = stdout.take((maximum_output.saturating_add(1)) as u64);
        let result = bounded.read_to_end(&mut output);
        (result, output)
    });
    let deadline = Instant::now() + VOICE_PROVIDER_TIMEOUT;
    loop {
        if cancelled.load(Ordering::Acquire) {
            terminate_child(&mut child);
            let _ = writer.join();
            let _ = reader.join();
            return Err("voice_operation_cancelled");
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let writer_ok = writer.join().unwrap_or(false);
                let (read_result, output) = reader.join().unwrap_or((
                    Err(std::io::Error::other("voice_provider_output_invalid")),
                    Vec::new(),
                ));
                if !status.success() || !writer_ok {
                    return Err("voice_provider_invalid");
                }
                read_result.map_err(|_| "voice_provider_output_invalid")?;
                if output.len() > maximum_output {
                    return Err("voice_provider_output_invalid");
                }
                return Ok(output);
            }
            Ok(None) => {}
            Err(_) => {
                terminate_child(&mut child);
                let _ = writer.join();
                let _ = reader.join();
                return Err("voice_provider_invalid");
            }
        }
        if Instant::now() >= deadline {
            terminate_child(&mut child);
            let _ = writer.join();
            let _ = reader.join();
            return Err("voice_provider_timeout");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[derive(Debug, Deserialize)]
struct ProviderTranscriptionOutput {
    text: String,
    confidence: Option<f64>,
}

fn parse_transcription_output(output: &[u8]) -> Result<ProviderTranscriptionOutput, DatabaseError> {
    let parsed: ProviderTranscriptionOutput = serde_json::from_slice(output)
        .map_err(|_| DatabaseError::Cognitive("voice_provider_output_invalid"))?;
    if parsed.text.len() > MAX_TEXT_BYTES
        || parsed
            .confidence
            .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
    {
        return Err(DatabaseError::Cognitive("voice_provider_output_invalid"));
    }
    Ok(parsed)
}

#[derive(Debug, Deserialize)]
struct ProviderWakeOutput {
    detected: bool,
}

fn parse_wake_output(output: &[u8]) -> Result<ProviderWakeOutput, DatabaseError> {
    serde_json::from_slice(output)
        .map_err(|_| DatabaseError::Cognitive("voice_provider_output_invalid"))
}

fn validate_pcm_output(output: Vec<u8>) -> Result<Vec<u8>, DatabaseError> {
    if output.is_empty()
        || output.len() > MAX_TTS_PCM_BYTES
        || !output.len().is_multiple_of(VOICE_BYTES_PER_SAMPLE)
    {
        return Err(DatabaseError::Cognitive("voice_provider_output_invalid"));
    }
    Ok(output)
}

#[cfg(windows)]
fn parse_wave_device(reference: &str, direction: &str) -> Result<u32, &'static str> {
    let prefix = match direction {
        "input" => "local:wavein:",
        "output" => "local:waveout:",
        _ => return Err("voice_device_unavailable"),
    };
    let index = reference
        .strip_prefix(prefix)
        .filter(|value| {
            !value.is_empty() && value.chars().all(|character| character.is_ascii_digit())
        })
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or("voice_device_unavailable")?;
    let available = unsafe {
        if direction == "input" {
            windows_sys::Win32::Media::Audio::waveInGetNumDevs()
        } else {
            windows_sys::Win32::Media::Audio::waveOutGetNumDevs()
        }
    };
    if index >= available {
        return Err("voice_device_unavailable");
    }
    Ok(index)
}

#[cfg(not(windows))]
fn parse_wave_device(_reference: &str, _direction: &str) -> Result<u32, &'static str> {
    Err("voice_unsupported_platform")
}

#[cfg(windows)]
fn pcm_format() -> windows_sys::Win32::Media::Audio::WAVEFORMATEX {
    windows_sys::Win32::Media::Audio::WAVEFORMATEX {
        wFormatTag: windows_sys::Win32::Media::Audio::WAVE_FORMAT_PCM as u16,
        nChannels: VOICE_CHANNELS,
        nSamplesPerSec: VOICE_SAMPLE_RATE,
        nAvgBytesPerSec: VOICE_SAMPLE_RATE * VOICE_BYTES_PER_SAMPLE as u32,
        nBlockAlign: (VOICE_CHANNELS as u32 * VOICE_BYTES_PER_SAMPLE as u32) as u16,
        wBitsPerSample: (VOICE_BYTES_PER_SAMPLE * 8) as u16,
        cbSize: 0,
    }
}

#[cfg(windows)]
struct WaveInSession {
    handle: windows_sys::Win32::Media::Audio::HWAVEIN,
    header: windows_sys::Win32::Media::Audio::WAVEHDR,
    prepared: bool,
}

#[cfg(windows)]
impl Drop for WaveInSession {
    fn drop(&mut self) {
        unsafe {
            let _ = windows_sys::Win32::Media::Audio::waveInStop(self.handle);
            let _ = windows_sys::Win32::Media::Audio::waveInReset(self.handle);
            if self.prepared {
                let _ = windows_sys::Win32::Media::Audio::waveInUnprepareHeader(
                    self.handle,
                    &mut self.header,
                    std::mem::size_of::<windows_sys::Win32::Media::Audio::WAVEHDR>() as u32,
                );
            }
            let _ = windows_sys::Win32::Media::Audio::waveInClose(self.handle);
        }
    }
}

#[cfg(windows)]
struct CaptureCallbackState {
    pcm: Mutex<Vec<u8>>,
    completed: AtomicBool,
    overflowed: AtomicBool,
}

#[cfg(windows)]
unsafe extern "system" fn wave_in_callback(
    _handle: windows_sys::Win32::Media::Audio::HWAVEIN,
    message: u32,
    instance: usize,
    parameter_one: usize,
    _parameter_two: usize,
) {
    const WIM_DATA: u32 = 0x3C0;
    if message != WIM_DATA || instance == 0 || parameter_one == 0 {
        return;
    }
    let state = &*(instance as *const CaptureCallbackState);
    let header = parameter_one as *const windows_sys::Win32::Media::Audio::WAVEHDR;
    let bytes_recorded =
        std::ptr::read_unaligned(std::ptr::addr_of!((*header).dwBytesRecorded)) as usize;
    let buffer_length =
        std::ptr::read_unaligned(std::ptr::addr_of!((*header).dwBufferLength)) as usize;
    let data = std::ptr::read_unaligned(std::ptr::addr_of!((*header).lpData));
    let bytes_recorded = bytes_recorded.min(buffer_length);
    if bytes_recorded > 0 && !data.is_null() {
        let bytes = std::slice::from_raw_parts(data as *const u8, bytes_recorded);
        if let Ok(mut pcm) = state.pcm.lock() {
            if pcm.len().saturating_add(bytes.len()) <= MAX_VOICE_CAPTURE_BYTES {
                pcm.extend_from_slice(bytes);
            } else {
                state.overflowed.store(true, Ordering::Release);
            }
        }
    }
    state.completed.store(true, Ordering::Release);
}

#[cfg(windows)]
fn capture_pcm(
    device_reference: String,
    duration_ms: u64,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, &'static str> {
    let device = parse_wave_device(&device_reference, "input")?;
    let buffer_length =
        ((VOICE_SAMPLE_RATE as u64 * VOICE_BYTES_PER_SAMPLE as u64 * duration_ms) / 1_000) as usize;
    if buffer_length == 0 || buffer_length > MAX_VOICE_CAPTURE_BYTES {
        return Err("voice_capture_limit");
    }
    let callback_state = Box::new(CaptureCallbackState {
        pcm: Mutex::new(Vec::with_capacity(buffer_length)),
        completed: AtomicBool::new(false),
        overflowed: AtomicBool::new(false),
    });
    let callback_state_pointer = (&*callback_state) as *const CaptureCallbackState as usize;
    let mut buffer = vec![0_u8; buffer_length];
    let header = windows_sys::Win32::Media::Audio::WAVEHDR {
        lpData: buffer.as_mut_ptr() as windows_sys::core::PSTR,
        dwBufferLength: buffer_length as u32,
        ..Default::default()
    };
    let mut handle = std::ptr::null_mut();
    let open_result = unsafe {
        windows_sys::Win32::Media::Audio::waveInOpen(
            &mut handle,
            device,
            &pcm_format(),
            wave_in_callback as *const () as usize,
            callback_state_pointer,
            windows_sys::Win32::Media::Audio::CALLBACK_FUNCTION,
        )
    };
    if open_result != 0 || handle.is_null() {
        return Err("voice_device_unavailable");
    }
    let mut session = WaveInSession {
        handle,
        header,
        prepared: false,
    };
    let header_size = std::mem::size_of::<windows_sys::Win32::Media::Audio::WAVEHDR>() as u32;
    let prepare_result = unsafe {
        windows_sys::Win32::Media::Audio::waveInPrepareHeader(
            session.handle,
            &mut session.header,
            header_size,
        )
    };
    if prepare_result != 0 {
        return Err("voice_device_unavailable");
    }
    session.prepared = true;
    let add_result = unsafe {
        windows_sys::Win32::Media::Audio::waveInAddBuffer(
            session.handle,
            &mut session.header,
            header_size,
        )
    };
    if add_result != 0 {
        return Err("voice_device_unavailable");
    }
    let start_result = unsafe { windows_sys::Win32::Media::Audio::waveInStart(session.handle) };
    if start_result != 0 {
        return Err("voice_device_unavailable");
    }
    let deadline = Instant::now() + Duration::from_millis(duration_ms + 1_000);
    while !callback_state.completed.load(Ordering::Acquire) {
        if cancelled.load(Ordering::Acquire) {
            return Err("voice_operation_cancelled");
        }
        if Instant::now() >= deadline {
            return Err("voice_capture_timeout");
        }
        thread::sleep(Duration::from_millis(10));
    }
    if callback_state.overflowed.load(Ordering::Acquire) {
        return Err("voice_capture_limit");
    }
    let pcm = callback_state
        .pcm
        .lock()
        .map_err(|_| "voice_device_unavailable")?
        .clone();
    if pcm.is_empty() {
        return Err("voice_device_unavailable");
    }
    drop(session);
    drop(callback_state);
    drop(buffer);
    Ok(pcm)
}

#[cfg(not(windows))]
fn capture_pcm(
    _device_reference: String,
    _duration_ms: u64,
    _cancelled: &AtomicBool,
) -> Result<Vec<u8>, &'static str> {
    Err("voice_unsupported_platform")
}

#[cfg(windows)]
struct WaveOutSession {
    handle: windows_sys::Win32::Media::Audio::HWAVEOUT,
    header: windows_sys::Win32::Media::Audio::WAVEHDR,
    prepared: bool,
}

#[cfg(windows)]
impl Drop for WaveOutSession {
    fn drop(&mut self) {
        unsafe {
            let _ = windows_sys::Win32::Media::Audio::waveOutReset(self.handle);
            if self.prepared {
                let _ = windows_sys::Win32::Media::Audio::waveOutUnprepareHeader(
                    self.handle,
                    &mut self.header,
                    std::mem::size_of::<windows_sys::Win32::Media::Audio::WAVEHDR>() as u32,
                );
            }
            let _ = windows_sys::Win32::Media::Audio::waveOutClose(self.handle);
        }
    }
}

#[cfg(windows)]
fn play_pcm(
    device_reference: String,
    pcm: &[u8],
    cancelled: &AtomicBool,
) -> Result<(), &'static str> {
    let device = parse_wave_device(&device_reference, "output")?;
    if pcm.is_empty()
        || pcm.len() > MAX_TTS_PCM_BYTES
        || !pcm.len().is_multiple_of(VOICE_BYTES_PER_SAMPLE)
    {
        return Err("voice_provider_output_invalid");
    }
    let mut audio = pcm.to_vec();
    let header = windows_sys::Win32::Media::Audio::WAVEHDR {
        lpData: audio.as_mut_ptr() as windows_sys::core::PSTR,
        dwBufferLength: audio.len() as u32,
        ..Default::default()
    };
    let mut handle = std::ptr::null_mut();
    let open_result = unsafe {
        windows_sys::Win32::Media::Audio::waveOutOpen(&mut handle, device, &pcm_format(), 0, 0, 0)
    };
    if open_result != 0 || handle.is_null() {
        return Err("voice_device_unavailable");
    }
    let mut session = WaveOutSession {
        handle,
        header,
        prepared: false,
    };
    let header_size = std::mem::size_of::<windows_sys::Win32::Media::Audio::WAVEHDR>() as u32;
    if unsafe {
        windows_sys::Win32::Media::Audio::waveOutPrepareHeader(
            session.handle,
            &mut session.header,
            header_size,
        )
    } != 0
    {
        return Err("voice_device_unavailable");
    }
    session.prepared = true;
    if unsafe {
        windows_sys::Win32::Media::Audio::waveOutWrite(
            session.handle,
            &mut session.header,
            header_size,
        )
    } != 0
    {
        return Err("voice_device_unavailable");
    }
    let duration_ms =
        (pcm.len() as u64 * 1_000) / (VOICE_SAMPLE_RATE as u64 * VOICE_BYTES_PER_SAMPLE as u64);
    let deadline = Instant::now() + Duration::from_millis(duration_ms + 2_000);
    loop {
        let flags = unsafe { std::ptr::read_unaligned(std::ptr::addr_of!(session.header.dwFlags)) };
        if flags & windows_sys::Win32::Media::Audio::WHDR_DONE != 0 {
            break;
        }
        if cancelled.load(Ordering::Acquire) {
            return Err("voice_operation_cancelled");
        }
        if Instant::now() >= deadline {
            return Err("voice_output_timeout");
        }
        thread::sleep(Duration::from_millis(10));
    }
    drop(session);
    drop(audio);
    Ok(())
}

#[cfg(not(windows))]
fn play_pcm(
    _device_reference: String,
    _pcm: &[u8],
    _cancelled: &AtomicBool,
) -> Result<(), &'static str> {
    Err("voice_unsupported_platform")
}

fn voice_settings_sql() -> &'static str {
    "SELECT s.agent_id, s.schema_version, s.base_voice_id, s.custom_voice_ref,
            s.custom_voice_consent, s.recognition_model_ref, s.synthesis_model_ref,
            s.input_device_ref, s.output_device_ref, s.updated_at,
            state.mode, state.suspended
     FROM agent_voice_settings s
     JOIN agent_simulated_states state ON state.agent_id = s.agent_id
     WHERE s.agent_id = ?1"
}

fn load_voice_settings_tx(
    transaction: &Transaction<'_>,
    agent_id: &str,
) -> Result<VoiceSettings, DatabaseError> {
    transaction
        .query_row(voice_settings_sql(), params![agent_id], map_voice_settings)
        .optional()?
        .ok_or(DatabaseError::Cognitive("voice_settings_not_found"))
}

fn map_voice_settings(row: &rusqlite::Row<'_>) -> rusqlite::Result<VoiceSettings> {
    let schema_version: i64 = row.get(1)?;
    let base_voice_id: String = row.get(2)?;
    if schema_version != VOICE_SCHEMA_VERSION || base_voice_id != BASE_VOICE_ID {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let mode: String = row.get(10)?;
    Ok(VoiceSettings {
        agent_id: row.get(0)?,
        schema_version,
        base_voice_id,
        base_voice_protected: true,
        custom_voice_ref: row.get(3)?,
        custom_voice_consent: row.get(4)?,
        recognition_model_ref: row.get(5)?,
        synthesis_model_ref: row.get(6)?,
        input_device_ref: row.get(7)?,
        output_device_ref: row.get(8)?,
        mode: mode.clone(),
        voice_muted: mode == "voice_muted",
        silent: mode == "silent",
        suspended: row.get(11)?,
        updated_at: row.get(9)?,
    })
}

fn ensure_owner(
    connection: &rusqlite::Connection,
    agent_id: &str,
) -> Result<String, DatabaseError> {
    connection
        .query_row(
            "SELECT owner_user_id FROM agents WHERE id = ?1",
            params![agent_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("agent_not_found"))
        .and_then(|owner| {
            if owner == OWNER_ID {
                Ok(owner)
            } else {
                Err(DatabaseError::OwnershipMismatch)
            }
        })
}

fn ensure_owner_tx(transaction: &Transaction<'_>, agent_id: &str) -> Result<String, DatabaseError> {
    transaction
        .query_row(
            "SELECT owner_user_id FROM agents WHERE id = ?1",
            params![agent_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("agent_not_found"))
        .and_then(|owner| {
            if owner == OWNER_ID {
                Ok(owner)
            } else {
                Err(DatabaseError::OwnershipMismatch)
            }
        })
}

fn ensure_mutation_mode(mode: &str, suspended: bool) -> Result<(), DatabaseError> {
    if suspended {
        return Err(DatabaseError::Cognitive("voice_blocked_suspended"));
    }
    if mode == "silent" {
        return Err(DatabaseError::Cognitive("voice_blocked_silent"));
    }
    Ok(())
}

fn ensure_mutation_mode_tx(
    transaction: &Transaction<'_>,
    agent_id: &str,
) -> Result<(), DatabaseError> {
    let (mode, suspended): (String, bool) = transaction
        .query_row(
            "SELECT mode, suspended FROM agent_simulated_states WHERE agent_id = ?1",
            params![agent_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or(DatabaseError::Cognitive("agent_not_found"))?;
    ensure_mutation_mode(&mode, suspended)
}

#[derive(Debug)]
struct MutationEvent {
    operation: String,
    request_json: String,
}

fn load_mutation_event(
    transaction: &Transaction<'_>,
    agent_id: &str,
    idempotency_key: &str,
) -> Result<Option<MutationEvent>, DatabaseError> {
    transaction
        .query_row(
            "SELECT operation, request_json FROM voice_mutation_events
             WHERE agent_id = ?1 AND idempotency_key = ?2",
            params![agent_id, idempotency_key],
            |row| {
                Ok(MutationEvent {
                    operation: row.get(0)?,
                    request_json: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(DatabaseError::from)
}

fn insert_mutation_event(
    transaction: &Transaction<'_>,
    agent_id: &str,
    owner_id: &str,
    operation: &str,
    idempotency_key: &str,
    request_json: &str,
    settings: &VoiceSettings,
) -> Result<(), DatabaseError> {
    let result_json = serde_json::to_string(settings).map_err(|_| DatabaseError::Unavailable)?;
    transaction.execute(
        "INSERT INTO voice_mutation_events
         (id, agent_id, owner_user_id, operation, idempotency_key, request_json, result_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            Uuid::now_v7().to_string(),
            agent_id,
            owner_id,
            operation,
            idempotency_key,
            request_json,
            result_json,
            now_millis(),
        ],
    )?;
    Ok(())
}

fn optional_reference(
    value: Option<&str>,
    code: &'static str,
) -> Result<Option<String>, DatabaseError> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.len() > MAX_REFERENCE_LENGTH
        || (!value.starts_with("fixture:") && !value.starts_with("local:"))
        || value
            .chars()
            .any(|character| !(character.is_ascii_alphanumeric() || ":._-".contains(character)))
    {
        return Err(DatabaseError::Cognitive(code));
    }
    Ok(Some(value.to_string()))
}

fn custom_voice_reference(value: Option<&str>) -> Result<String, DatabaseError> {
    let reference = optional_reference(value, "voice_reference_invalid")?
        .ok_or(DatabaseError::Cognitive("voice_consent_invalid"))?;
    if reference.starts_with("fixture:custom-") {
        return Ok(reference);
    }
    let Some(identifier) = reference.strip_prefix("local:custom-") else {
        return Err(DatabaseError::Cognitive("voice_consent_invalid"));
    };
    if identifier.is_empty()
        || identifier
            .chars()
            .any(|character| !(character.is_ascii_alphanumeric() || character == '-'))
        || identifier.contains("real-person")
        || identifier.contains("clone")
    {
        return Err(DatabaseError::Cognitive("voice_consent_invalid"));
    }
    Ok(reference)
}

fn idempotency(value: &str) -> Result<String, DatabaseError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAX_IDEMPOTENCY_LENGTH
        || value
            .chars()
            .any(|character| !(character.is_ascii_alphanumeric() || ":._-".contains(character)))
    {
        return Err(DatabaseError::Cognitive("invalid_idempotency_key"));
    }
    Ok(value.to_string())
}

fn bounded_text<'a>(
    value: &'a str,
    maximum: usize,
    code: &'static str,
) -> Result<&'a str, DatabaseError> {
    let value = value.trim();
    if value.is_empty() || value.len() > maximum {
        return Err(DatabaseError::Cognitive(code));
    }
    Ok(value)
}

fn transcription_fixture(fixture_id: &str) -> Option<(&'static str, f64)> {
    match fixture_id {
        "fixture:hello" => Some(("Olá, Astra.", 0.92)),
        "fixture:owner-greeting" => Some(("Olá, agente.", 0.94)),
        "fixture:empty" => Some(("", 0.99)),
        _ => None,
    }
}

fn transcription_degraded(fixture_id: String, code: &str) -> VoiceTranscriptionResult {
    VoiceTranscriptionResult {
        status: "degraded".into(),
        code: Some(code.into()),
        fixture_id,
        text: None,
        confidence: None,
        metadata_only: true,
        raw_audio_persisted: false,
        text_chat_fallback: true,
    }
}

fn synthesis_degraded(voice_ref: String, code: &str) -> VoiceSynthesisResult {
    VoiceSynthesisResult {
        status: "degraded".into(),
        code: Some(code.into()),
        voice_ref,
        duration_ms: 0,
        metadata_only: true,
        raw_audio_persisted: false,
        text_chat_fallback: true,
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use uuid::Uuid;

    use super::*;
    use crate::database::{ASTRA_ID, LUMA_ID};

    fn test_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("aip-voice-test-{}", Uuid::now_v7()))
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    fn settings_request(key: &str) -> VoiceSettingsRequest {
        VoiceSettingsRequest {
            agent_id: ASTRA_ID.into(),
            recognition_model_ref: Some("fixture:stt-v1".into()),
            synthesis_model_ref: Some("fixture:tts-v1".into()),
            input_device_ref: Some("fixture:microphone-1".into()),
            output_device_ref: Some("fixture:speaker-1".into()),
            idempotency_key: key.into(),
            temporary_chat: false,
        }
    }

    #[test]
    fn capture_duration_limits_are_enforced() {
        assert_eq!(
            bounded_capture_duration(MIN_VOICE_CAPTURE_DURATION_MS - 1),
            Err(DatabaseError::Cognitive("voice_capture_limit"))
        );
        assert_eq!(
            bounded_capture_duration(MAX_VOICE_CAPTURE_DURATION_MS),
            Ok(MAX_VOICE_CAPTURE_DURATION_MS)
        );
        assert_eq!(
            bounded_capture_duration(MAX_VOICE_CAPTURE_DURATION_MS + 1),
            Err(DatabaseError::Cognitive("voice_capture_limit"))
        );
    }

    #[test]
    fn unavailable_provider_and_device_fail_closed() {
        assert_eq!(
            provider_executable("fixture:stt-v1", ProviderKind::Transcription),
            Err("voice_model_unavailable")
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn unavailable_device_fails_closed() {
        assert_eq!(
            parse_wave_device("fixture:microphone-1", "input"),
            Err("voice_unsupported_platform")
        );
    }

    #[test]
    fn consent_and_base_voice_are_explicit_and_protected() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let initial = database.voice_settings(ASTRA_ID).unwrap();
        assert_eq!(initial.base_voice_id, BASE_VOICE_ID);
        assert!(initial.base_voice_protected);
        assert_eq!(initial.custom_voice_consent, "not_granted");
        let granted = database
            .set_custom_voice_consent(CustomVoiceConsentRequest {
                agent_id: ASTRA_ID.into(),
                granted: true,
                custom_voice_ref: Some("fixture:custom-neutral-v1".into()),
                idempotency_key: "consent-grant".into(),
                temporary_chat: false,
            })
            .unwrap();
        assert_eq!(granted.custom_voice_consent, "granted");
        assert_eq!(
            granted.custom_voice_ref.as_deref(),
            Some("fixture:custom-neutral-v1")
        );
        assert_eq!(granted.base_voice_id, BASE_VOICE_ID);
        let local = database
            .set_custom_voice_consent(CustomVoiceConsentRequest {
                agent_id: ASTRA_ID.into(),
                granted: true,
                custom_voice_ref: Some("local:custom-neutral-v1".into()),
                idempotency_key: "consent-local-grant".into(),
                temporary_chat: false,
            })
            .unwrap();
        assert_eq!(
            local.custom_voice_ref.as_deref(),
            Some("local:custom-neutral-v1")
        );
        assert_eq!(
            database.set_custom_voice_consent(CustomVoiceConsentRequest {
                agent_id: ASTRA_ID.into(),
                granted: true,
                custom_voice_ref: Some("local:real-person-clone".into()),
                idempotency_key: "consent-invalid".into(),
                temporary_chat: false,
            }),
            Err(DatabaseError::Cognitive("voice_consent_invalid"))
        );
        for (idempotency_key, custom_voice_ref) in [
            ("consent-empty", "local:custom-"),
            ("consent-path", "local:custom-../voice"),
            ("consent-space", "local:custom-neutral voice"),
        ] {
            assert!(database
                .set_custom_voice_consent(CustomVoiceConsentRequest {
                    agent_id: ASTRA_ID.into(),
                    granted: true,
                    custom_voice_ref: Some(custom_voice_ref.into()),
                    idempotency_key: idempotency_key.into(),
                    temporary_chat: false,
                })
                .is_err());
        }
        let revoked = database
            .set_custom_voice_consent(CustomVoiceConsentRequest {
                agent_id: ASTRA_ID.into(),
                granted: false,
                custom_voice_ref: None,
                idempotency_key: "consent-revoke".into(),
                temporary_chat: false,
            })
            .unwrap();
        assert_eq!(revoked.custom_voice_consent, "revoked");
        assert!(revoked.custom_voice_ref.is_none());
        assert_eq!(
            database.voice_settings(LUMA_ID).unwrap().base_voice_id,
            BASE_VOICE_ID
        );
        cleanup(&path);
    }

    #[test]
    fn provider_status_uses_only_bounded_saved_references() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let initial = database.voice_provider_status(ASTRA_ID).unwrap();
        assert_eq!(initial.recognition.state, "not_configured");
        assert_eq!(initial.synthesis.state, "not_configured");
        assert!(!initial.recognition.synthetic);
        assert!(!initial.synthesis.synthetic);

        database
            .update_voice_settings(settings_request("provider-status"))
            .unwrap();
        let configured = database.voice_provider_status(ASTRA_ID).unwrap();
        assert_eq!(configured.recognition.state, "ready");
        assert_eq!(configured.synthesis.state, "ready");
        assert_eq!(
            configured.recognition.reference.as_deref(),
            Some("fixture:stt-v1")
        );
        assert_eq!(
            configured.synthesis.reference.as_deref(),
            Some("fixture:tts-v1")
        );
        assert!(configured.recognition.synthetic);
        assert!(configured.synthesis.synthetic);
        cleanup(&path);
    }

    #[test]
    fn settings_are_bounded_temporary_safe_and_replayable() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let mut temporary = settings_request("temporary-settings");
        temporary.temporary_chat = true;
        assert_eq!(
            database.update_voice_settings(temporary),
            Err(DatabaseError::Cognitive("conversation_temporary_blocked"))
        );
        let request = settings_request("settings-replay");
        let first = database.update_voice_settings(request.clone()).unwrap();
        let replay = database.update_voice_settings(request).unwrap();
        assert_eq!(first, replay);
        let mut conflict = settings_request("settings-replay");
        conflict.output_device_ref = Some("fixture:other-speaker".into());
        assert_eq!(
            database.update_voice_settings(conflict),
            Err(DatabaseError::Cognitive("idempotency_conflict"))
        );
        let mut oversized = settings_request("settings-oversized");
        oversized.input_device_ref = Some(format!("fixture:{}", "x".repeat(200)));
        assert_eq!(
            database.update_voice_settings(oversized),
            Err(DatabaseError::Cognitive("voice_reference_invalid"))
        );
        cleanup(&path);
    }

    #[test]
    fn missing_devices_models_and_modes_degrade_without_breaking_text() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let transcription = database
            .transcribe_voice_fixture(VoiceTranscriptionRequest {
                agent_id: ASTRA_ID.into(),
                fixture_id: "fixture:hello".into(),
                temporary_chat: false,
            })
            .unwrap();
        assert_eq!(transcription.status, "degraded");
        assert_eq!(
            transcription.code.as_deref(),
            Some("voice_device_unavailable")
        );
        assert!(transcription.text_chat_fallback);
        database.set_agent_mode(ASTRA_ID, "voice_muted").unwrap();
        let muted = database
            .synthesize_voice_fixture(VoiceSynthesisRequest {
                agent_id: ASTRA_ID.into(),
                text: "Olá".into(),
                temporary_chat: false,
            })
            .unwrap();
        assert_eq!(muted.status, "muted");
        assert!(muted.text_chat_fallback);
        database
            .update_voice_settings(settings_request("mode-settings"))
            .unwrap();
        database.set_agent_mode(ASTRA_ID, "silent").unwrap();
        assert_eq!(
            database.update_voice_settings(settings_request("silent-settings")),
            Err(DatabaseError::Cognitive("voice_blocked_silent"))
        );
        let silent = database
            .synthesize_voice_fixture(VoiceSynthesisRequest {
                agent_id: ASTRA_ID.into(),
                text: "Olá".into(),
                temporary_chat: false,
            })
            .unwrap();
        assert_eq!(silent.status, "ready");
        let wake = database
            .detect_voice_wake_word_fixture(VoiceWakeWordRequest {
                agent_id: ASTRA_ID.into(),
                fixture_id: "fixture:wake-aip".into(),
                temporary_chat: false,
            })
            .unwrap();
        assert_eq!(wake.status, "ignored");
        assert!(!wake.listener_active);
        cleanup(&path);
    }

    #[test]
    fn fixture_pipeline_is_metadata_only_and_survives_restart() {
        let path = test_path();
        {
            let database = Database::initialize(&path).unwrap();
            database
                .update_voice_settings(settings_request("restart-settings"))
                .unwrap();
            database
                .set_custom_voice_consent(CustomVoiceConsentRequest {
                    agent_id: ASTRA_ID.into(),
                    granted: true,
                    custom_voice_ref: Some("fixture:custom-neutral-v1".into()),
                    idempotency_key: "restart-consent".into(),
                    temporary_chat: false,
                })
                .unwrap();
            let transcription = database
                .transcribe_voice_fixture(VoiceTranscriptionRequest {
                    agent_id: ASTRA_ID.into(),
                    fixture_id: "fixture:hello".into(),
                    temporary_chat: false,
                })
                .unwrap();
            let synthesis = database
                .synthesize_voice_fixture(VoiceSynthesisRequest {
                    agent_id: ASTRA_ID.into(),
                    text: "Olá".into(),
                    temporary_chat: false,
                })
                .unwrap();
            assert!(transcription.metadata_only);
            assert!(!transcription.raw_audio_persisted);
            assert!(synthesis.metadata_only);
            assert!(!synthesis.raw_audio_persisted);
            let connection = database.open().unwrap();
            let audio_tables: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name LIKE '%audio%'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(audio_tables, 0);
        }
        let database = Database::initialize(&path).unwrap();
        let settings = database.voice_settings(ASTRA_ID).unwrap();
        assert_eq!(settings.schema_version, VOICE_SCHEMA_VERSION);
        assert_eq!(settings.custom_voice_consent, "granted");
        assert_eq!(settings.base_voice_id, BASE_VOICE_ID);
        cleanup(&path);
    }

    #[test]
    fn emotion_result_is_uncertain_and_non_diagnostic() {
        let path = test_path();
        let database = Database::initialize(&path).unwrap();
        let result = database
            .classify_voice_emotion(VoiceEmotionHypothesisRequest {
                text: "Estou preocupado".into(),
                temporary_chat: true,
            })
            .unwrap();
        assert_eq!(result.label, "concerned");
        assert!(result.uncertain);
        assert!(!result.diagnostic);
        cleanup(&path);
    }
}
