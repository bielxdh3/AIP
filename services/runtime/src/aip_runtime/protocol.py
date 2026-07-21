"""Versioned, bounded NDJSON protocol for the Phase 0 runtime."""

from __future__ import annotations

import json
from typing import Any

PROTOCOL_VERSION = 1
MAX_MESSAGE_BYTES = 65_536
MAX_REQUEST_ID_LENGTH = 128


class ProtocolError(ValueError):
    """A safe protocol failure with a stable public code."""

    def __init__(self, code: str) -> None:
        super().__init__(code)
        self.code = code


def health_document() -> dict[str, object]:
    """Return deterministic runtime capability state without machine details."""

    return {
        "capabilities": [],
        "name": "aip-runtime",
        "protocolVersion": PROTOCOL_VERSION,
        "status": "ready",
    }


def _request_id(value: Any) -> str:
    if not isinstance(value, str) or not value or len(value) > MAX_REQUEST_ID_LENGTH:
        raise ProtocolError("invalid_request_id")
    return value


def parse_request(line: str) -> dict[str, object]:
    if len(line.encode("utf-8")) > MAX_MESSAGE_BYTES:
        raise ProtocolError("message_too_large")

    try:
        candidate = json.loads(line)
    except json.JSONDecodeError as error:
        raise ProtocolError("malformed_json") from error

    if not isinstance(candidate, dict):
        raise ProtocolError("invalid_envelope")
    if candidate.get("protocolVersion") != PROTOCOL_VERSION:
        raise ProtocolError("unsupported_protocol")

    request_id = _request_id(candidate.get("id"))
    method = candidate.get("method")
    params = candidate.get("params")
    if method not in {"runtime.health", "runtime.shutdown"} or not isinstance(params, dict):
        raise ProtocolError("unsupported_request")

    return {
        "id": request_id,
        "method": method,
        "params": params,
        "protocolVersion": PROTOCOL_VERSION,
    }


def _error_response(code: str) -> dict[str, object]:
    return {
        "error": {"code": code, "message": "Request rejected."},
        "id": "invalid",
        "protocolVersion": PROTOCOL_VERSION,
    }


def handle_line(line: str) -> tuple[dict[str, object], bool]:
    """Handle one line and return a safe response plus a shutdown signal."""

    try:
        request = parse_request(line)
    except ProtocolError as error:
        return _error_response(error.code), False

    request_id = str(request["id"])
    if request["method"] == "runtime.shutdown":
        return (
            {
                "id": request_id,
                "protocolVersion": PROTOCOL_VERSION,
                "result": {"status": "stopping"},
            },
            True,
        )

    return (
        {
            "id": request_id,
            "protocolVersion": PROTOCOL_VERSION,
            "result": {
                "name": "aip-runtime",
                "protocolVersion": PROTOCOL_VERSION,
                "status": "ready",
            },
        },
        False,
    )


def encode_message(message: dict[str, object]) -> str:
    return json.dumps(message, ensure_ascii=False, separators=(",", ":"), sort_keys=True)
