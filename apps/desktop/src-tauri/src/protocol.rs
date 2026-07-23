use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::domain::{
    OllamaModel, PhaseOneEvent, ProviderSnapshot, ProviderState, MAX_DISCOVERED_MODELS,
    MAX_PROVIDER_ERROR_BYTES, MAX_STREAM_CHUNK_BYTES,
};

pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_MESSAGE_BYTES: usize = 65_536;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptMessage {
    pub role: &'static str,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeOutput {
    HealthReady {
        id: String,
    },
    Provider {
        id: String,
        snapshot: ProviderSnapshot,
    },
    ModelDetails {
        id: String,
        provider_model_id: String,
        capabilities: Vec<String>,
    },
    Accepted {
        id: String,
    },
    Error {
        id: String,
        code: String,
    },
    Event(PhaseOneEvent),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Request<'a> {
    protocol_version: u32,
    id: &'a str,
    method: &'a str,
    params: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResponseEnvelope {
    protocol_version: u32,
    id: String,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<ErrorBody>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ErrorBody {
    code: String,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventEnvelope {
    protocol_version: u32,
    event: String,
    request_id: String,
    agent_id: String,
    conversation_id: String,
    assistant_message_id: String,
    #[serde(default)]
    sequence: Option<u64>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    error_code: Option<String>,
}

pub fn health_request(id: &str) -> Result<String, ()> {
    request(id, "runtime.health", json!({}))
}

pub fn shutdown_request(id: &str) -> Result<String, ()> {
    request(id, "runtime.shutdown", json!({}))
}

pub fn discovery_request(id: &str) -> Result<String, ()> {
    request(id, "provider.discover", json!({}))
}

pub fn show_model_request(id: &str, provider_model_id: &str) -> Result<String, ()> {
    if !valid_provider_model_id(provider_model_id) {
        return Err(());
    }
    request(id, "provider.show", json!({ "model": provider_model_id }))
}

#[allow(clippy::too_many_arguments)]
pub fn generation_request(
    request_id: &str,
    agent_id: &str,
    conversation_id: &str,
    assistant_message_id: &str,
    provider_model_id: &str,
    keep_alive_minutes: u32,
    messages: &[PromptMessage],
) -> Result<String, ()> {
    if !valid_provider_model_id(provider_model_id) {
        return Err(());
    }
    let messages = messages
        .iter()
        .map(|message| json!({ "role": message.role, "content": message.content }))
        .collect::<Vec<_>>();
    request(
        request_id,
        "generation.start",
        json!({
            "agentId": agent_id,
            "conversationId": conversation_id,
            "assistantMessageId": assistant_message_id,
            "model": provider_model_id,
            "keepAliveMinutes": keep_alive_minutes,
            "messages": messages,
        }),
    )
}

pub fn cancellation_request(id: &str, target_request_id: &str) -> Result<String, ()> {
    request(
        id,
        "generation.cancel",
        json!({ "requestId": target_request_id }),
    )
}

fn request(id: &str, method: &str, params: Value) -> Result<String, ()> {
    if !valid_identifier(id) {
        return Err(());
    }
    let encoded = serde_json::to_string(&Request {
        protocol_version: PROTOCOL_VERSION,
        id,
        method,
        params,
    })
    .map_err(|_| ())?;
    if encoded.len() > MAX_MESSAGE_BYTES {
        return Err(());
    }
    Ok(encoded)
}

pub fn parse_health_response(line: &str, expected_id: &str) -> Result<(), ()> {
    match parse_runtime_output(line)? {
        RuntimeOutput::HealthReady { id } if id == expected_id => Ok(()),
        _ => Err(()),
    }
}

pub fn parse_runtime_output(line: &str) -> Result<RuntimeOutput, ()> {
    if line.len() > MAX_MESSAGE_BYTES {
        return Err(());
    }
    let value: Value = serde_json::from_str(line).map_err(|_| ())?;
    if value.get("event").is_some() {
        return parse_event(value);
    }
    let response: ResponseEnvelope = serde_json::from_value(value).map_err(|_| ())?;
    if response.protocol_version != PROTOCOL_VERSION || !valid_identifier(&response.id) {
        return Err(());
    }
    match (response.result, response.error) {
        (Some(result), None) => parse_result(response.id, result),
        (None, Some(error)) => {
            let _ = error.message;
            if !valid_code(&error.code) {
                return Err(());
            }
            Ok(RuntimeOutput::Error {
                id: response.id,
                code: error.code,
            })
        }
        _ => Err(()),
    }
}

fn parse_result(id: String, result: Value) -> Result<RuntimeOutput, ()> {
    if result.get("name").and_then(Value::as_str) == Some("aip-runtime") {
        let status = result.get("status").and_then(Value::as_str);
        let version = result.get("protocolVersion").and_then(Value::as_u64);
        return if status == Some("ready") && version == Some(u64::from(PROTOCOL_VERSION)) {
            Ok(RuntimeOutput::HealthReady { id })
        } else {
            Err(())
        };
    }
    if result.get("provider").and_then(Value::as_str) == Some("ollama") {
        let state = match result.get("state").and_then(Value::as_str) {
            Some("available") => ProviderState::Available,
            Some("empty") => ProviderState::Empty,
            _ => return Err(()),
        };
        let models: Vec<OllamaModel> =
            serde_json::from_value(result.get("models").cloned().ok_or(())?).map_err(|_| ())?;
        if models.len() > MAX_DISCOVERED_MODELS || models.iter().any(|model| !valid_model(model)) {
            return Err(());
        }
        return Ok(RuntimeOutput::Provider {
            id,
            snapshot: ProviderSnapshot {
                state,
                detail_code: if models.is_empty() {
                    "provider_empty".into()
                } else {
                    "provider_available".into()
                },
                models,
                refreshed_at: None,
            },
        });
    }
    if let Some(model) = result.get("model").and_then(Value::as_object) {
        let provider_model_id = model
            .get("providerModelId")
            .and_then(Value::as_str)
            .filter(|value| valid_provider_model_id(value))
            .ok_or(())?
            .to_string();
        let capabilities = model
            .get("capabilities")
            .and_then(Value::as_array)
            .ok_or(())?
            .iter()
            .map(|capability| capability.as_str().ok_or(()).map(str::to_string))
            .collect::<Result<Vec<_>, _>>()?;
        if capabilities.len() > 16
            || capabilities
                .iter()
                .any(|capability| capability.is_empty() || capability.len() > 64)
        {
            return Err(());
        }
        return Ok(RuntimeOutput::ModelDetails {
            id,
            provider_model_id,
            capabilities,
        });
    }
    if result.get("status").and_then(Value::as_str).is_some() {
        return Ok(RuntimeOutput::Accepted { id });
    }
    Err(())
}

fn parse_event(value: Value) -> Result<RuntimeOutput, ()> {
    let event: EventEnvelope = serde_json::from_value(value).map_err(|_| ())?;
    if event.protocol_version != PROTOCOL_VERSION
        || !valid_identifier(&event.request_id)
        || !valid_identifier(&event.agent_id)
        || !valid_identifier(&event.conversation_id)
        || !valid_identifier(&event.assistant_message_id)
        || !matches!(
            event.event.as_str(),
            "generation.started"
                | "generation.chunk"
                | "generation.complete"
                | "generation.failed"
                | "generation.cancelled"
        )
        || event
            .content
            .as_ref()
            .is_some_and(|content| content.is_empty() || content.len() > MAX_STREAM_CHUNK_BYTES)
        || event
            .error_code
            .as_ref()
            .is_some_and(|code| !valid_code(code))
    {
        return Err(());
    }
    if event.event == "generation.chunk"
        && (event.sequence.is_none() || event.content.is_none() || event.error_code.is_some())
    {
        return Err(());
    }
    if event.event == "generation.started"
        && (event.sequence != Some(0) || event.content.is_some() || event.error_code.is_some())
    {
        return Err(());
    }
    if matches!(
        event.event.as_str(),
        "generation.complete" | "generation.cancelled"
    ) && (event.sequence.is_none() || event.content.is_some() || event.error_code.is_some())
    {
        return Err(());
    }
    if event.event == "generation.failed"
        && (event.sequence.is_none() || event.content.is_some() || event.error_code.is_none())
    {
        return Err(());
    }
    if event.event != "generation.chunk" && event.content.is_some() {
        return Err(());
    }
    Ok(RuntimeOutput::Event(PhaseOneEvent {
        protocol_version: event.protocol_version,
        event_type: event.event,
        request_id: Some(event.request_id),
        agent_id: Some(event.agent_id),
        conversation_id: Some(event.conversation_id),
        assistant_message_id: Some(event.assistant_message_id),
        sequence: event.sequence,
        content: event.content,
        error_code: event.error_code,
    }))
}

fn valid_model(model: &OllamaModel) -> bool {
    model.model_ref == format!("ollama:{}", model.provider_model_id)
        && valid_provider_model_id(&model.provider_model_id)
        && !model.display_name.is_empty()
        && model.display_name.len() <= 200
        && model.size <= (1_u64 << 50)
        && optional_metadata_valid(model.family.as_deref(), 128)
        && optional_metadata_valid(model.parameter_size.as_deref(), 64)
        && optional_metadata_valid(model.quantization.as_deref(), 64)
        && model.capabilities.len() <= 16
        && model
            .capabilities
            .iter()
            .all(|capability| !capability.is_empty() && capability.len() <= 64)
}

fn optional_metadata_valid(value: Option<&str>, maximum: usize) -> bool {
    value.is_none_or(|value| {
        !value.is_empty()
            && value.len() <= maximum
            && value.chars().all(|character| !character.is_control())
    })
}

pub fn valid_provider_model_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 200
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".:_/-".contains(character))
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 200
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_-:.".contains(character))
}

fn valid_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PROVIDER_ERROR_BYTES
        && value
            .chars()
            .all(|character| character.is_ascii_lowercase() || character == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_round_trip_is_versioned() {
        let request = health_request("health-test").expect("request should serialize");
        assert!(request.contains("\"protocolVersion\":1"));
        let response = format!(
            "{{\"protocolVersion\":{PROTOCOL_VERSION},\"id\":\"health-test\",\"result\":{{\"name\":\"aip-runtime\",\"status\":\"ready\",\"protocolVersion\":{PROTOCOL_VERSION}}}}}"
        );
        assert!(parse_health_response(&response, "health-test").is_ok());
    }

    #[test]
    fn discovery_and_stream_events_are_bounded_and_correlated() {
        let discovery = r#"{"protocolVersion":1,"id":"discover","result":{"provider":"ollama","state":"available","models":[{"ref":"ollama:test:latest","providerModelId":"test:latest","displayName":"test:latest","size":42,"family":"test","parameterSize":"1B","quantization":"Q4","capabilities":[]}]}}"#;
        assert!(matches!(
            parse_runtime_output(discovery),
            Ok(RuntimeOutput::Provider { .. })
        ));
        let chunk = r#"{"protocolVersion":1,"event":"generation.chunk","requestId":"request","agentId":"agent","conversationId":"conversation","assistantMessageId":"message","sequence":1,"content":"hello"}"#;
        assert!(matches!(
            parse_runtime_output(chunk),
            Ok(RuntimeOutput::Event(PhaseOneEvent {
                sequence: Some(1),
                ..
            }))
        ));
        let oversized = format!(
            "{{\"protocolVersion\":1,\"event\":\"generation.chunk\",\"requestId\":\"request\",\"agentId\":\"agent\",\"conversationId\":\"conversation\",\"assistantMessageId\":\"message\",\"sequence\":1,\"content\":\"{}\"}}",
            "x".repeat(MAX_STREAM_CHUNK_BYTES + 1)
        );
        assert!(parse_runtime_output(&oversized).is_err());
    }

    #[test]
    fn malformed_or_mismatched_messages_are_rejected() {
        assert!(parse_runtime_output("not-json").is_err());
        assert!(parse_runtime_output(
            r#"{"protocolVersion":99,"id":"health","result":{"status":"ready"}}"#
        )
        .is_err());
        assert!(show_model_request("show", "../../bad model").is_err());
        assert!(!valid_provider_model_id("../../bad model"));
    }

    #[test]
    fn generation_request_keeps_prompt_structured() {
        let encoded = generation_request(
            "request",
            "agent",
            "conversation",
            "assistant",
            "test:latest",
            15,
            &[PromptMessage {
                role: "user",
                content: "Olá".into(),
            }],
        )
        .unwrap();
        let value: Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(value["params"]["model"], "test:latest");
        assert_eq!(value["params"]["messages"][0]["content"], "Olá");
    }
}
