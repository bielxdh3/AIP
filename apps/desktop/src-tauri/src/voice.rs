use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::database::{now_millis, Database, DatabaseError, OWNER_ID};

pub const BASE_VOICE_ID: &str = "aip-base-v1";
const VOICE_SCHEMA_VERSION: i64 = 1;
const MAX_REFERENCE_LENGTH: usize = 160;
const MAX_IDEMPOTENCY_LENGTH: usize = 128;
const MAX_TEXT_BYTES: usize = 4_096;
const MAX_EMOTION_TEXT_BYTES: usize = 2_048;

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

impl Database {
    pub fn voice_settings(&self, agent_id: &str) -> Result<VoiceSettings, DatabaseError> {
        let connection = self.open()?;
        ensure_owner(&connection, agent_id)?;
        connection
            .query_row(voice_settings_sql(), params![agent_id], map_voice_settings)
            .optional()?
            .ok_or(DatabaseError::Cognitive("voice_settings_not_found"))
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
    if !reference.starts_with("fixture:custom-") {
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
