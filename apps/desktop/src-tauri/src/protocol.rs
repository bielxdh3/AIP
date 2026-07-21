use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_MESSAGE_BYTES: usize = 65_536;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthRequest<'a> {
    protocol_version: u32,
    id: &'a str,
    method: &'static str,
    params: EmptyParams,
}

#[derive(Debug, Serialize)]
struct EmptyParams {}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HealthResponse {
    protocol_version: u32,
    id: String,
    result: HealthResult,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HealthResult {
    name: String,
    status: String,
    protocol_version: u32,
}

pub fn health_request(id: &str) -> Result<String, ()> {
    serde_json::to_string(&HealthRequest {
        protocol_version: PROTOCOL_VERSION,
        id,
        method: "runtime.health",
        params: EmptyParams {},
    })
    .map_err(|_| ())
}

pub fn shutdown_request(id: &str) -> Result<String, ()> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ShutdownRequest<'a> {
        protocol_version: u32,
        id: &'a str,
        method: &'static str,
        params: EmptyParams,
    }

    serde_json::to_string(&ShutdownRequest {
        protocol_version: PROTOCOL_VERSION,
        id,
        method: "runtime.shutdown",
        params: EmptyParams {},
    })
    .map_err(|_| ())
}

pub fn parse_health_response(line: &str, expected_id: &str) -> Result<(), ()> {
    if line.len() > MAX_MESSAGE_BYTES {
        return Err(());
    }
    let response: HealthResponse = serde_json::from_str(line).map_err(|_| ())?;
    if response.protocol_version != PROTOCOL_VERSION
        || response.id != expected_id
        || response.result.name != "aip-runtime"
        || response.result.status != "ready"
        || response.result.protocol_version != PROTOCOL_VERSION
    {
        return Err(());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{health_request, parse_health_response, PROTOCOL_VERSION};

    #[test]
    fn health_round_trip_is_versioned() {
        let request = health_request("health-test").expect("request should serialize");
        assert!(request.contains("\"protocolVersion\":1"));
        assert!(request.contains("\"runtime.health\""));

        let response = format!(
            "{{\"protocolVersion\":{PROTOCOL_VERSION},\"id\":\"health-test\",\"result\":{{\"name\":\"aip-runtime\",\"status\":\"ready\",\"protocolVersion\":{PROTOCOL_VERSION}}}}}"
        );
        assert!(parse_health_response(&response, "health-test").is_ok());
    }

    #[test]
    fn malformed_or_mismatched_health_is_rejected() {
        assert!(parse_health_response("not-json", "health-test").is_err());
        assert!(
            parse_health_response(
                "{\"protocolVersion\":99,\"id\":\"health-test\",\"result\":{\"name\":\"aip-runtime\",\"status\":\"ready\",\"protocolVersion\":99}}",
                "health-test"
            )
            .is_err()
        );
        assert!(
            parse_health_response(
                "{\"protocolVersion\":1,\"id\":\"other\",\"result\":{\"name\":\"aip-runtime\",\"status\":\"ready\",\"protocolVersion\":1}}",
                "health-test"
            )
            .is_err()
        );
    }
}
