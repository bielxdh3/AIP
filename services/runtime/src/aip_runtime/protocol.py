"""Versioned, bounded NDJSON protocol for the managed AIP runtime."""

from __future__ import annotations

import json
from typing import Any

PROTOCOL_VERSION = 1
MAX_MESSAGE_BYTES = 65_536
MAX_REQUEST_ID_LENGTH = 128
MAX_IDENTIFIER_LENGTH = 200
MAX_USER_MESSAGE_BYTES = 16_384
MAX_HISTORY_MESSAGES = 32
MAX_CONTEXT_BYTES = 49_152
MAX_STREAM_CHUNK_BYTES = 8_192
MAX_ASSISTANT_OUTPUT_BYTES = 65_536
MAX_DISCOVERED_MODELS = 64
MAX_PROVIDER_ERROR_BYTES = 256

METHODS = {
    "runtime.health",
    "runtime.shutdown",
    "provider.discover",
    "provider.show",
    "generation.start",
    "generation.cancel",
}


class ProtocolError(ValueError):
    """A safe protocol failure with a stable public code."""

    def __init__(self, code: str) -> None:
        super().__init__(code)
        self.code = code


def health_document() -> dict[str, object]:
    """Return deterministic runtime capability state without machine details."""

    return {
        "capabilities": ["ollama.discovery", "ollama.chat", "generation.cancel"],
        "name": "aip-runtime",
        "protocolVersion": PROTOCOL_VERSION,
        "status": "ready",
    }


def bounded_identifier(value: Any, code: str = "invalid_identifier") -> str:
    if (
        not isinstance(value, str)
        or not value
        or len(value) > MAX_IDENTIFIER_LENGTH
        or any(ord(character) < 32 for character in value)
    ):
        raise ProtocolError(code)
    return value


def _request_id(value: Any) -> str:
    if (
        not isinstance(value, str)
        or not value
        or len(value) > MAX_REQUEST_ID_LENGTH
        or not all(
            character.isascii() and (character.isalnum() or character in "_-:.")
            for character in value
        )
    ):
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
    if set(candidate) != {"protocolVersion", "id", "method", "params"}:
        raise ProtocolError("invalid_envelope")
    if candidate.get("protocolVersion") != PROTOCOL_VERSION:
        raise ProtocolError("unsupported_protocol")

    request_id = _request_id(candidate.get("id"))
    method = candidate.get("method")
    params = candidate.get("params")
    if method not in METHODS or not isinstance(params, dict):
        raise ProtocolError("unsupported_request")

    return {
        "id": request_id,
        "method": method,
        "params": params,
        "protocolVersion": PROTOCOL_VERSION,
    }


def error_response(request_id: str, code: str) -> dict[str, object]:
    return {
        "error": {"code": code, "message": "Request rejected."},
        "id": request_id,
        "protocolVersion": PROTOCOL_VERSION,
    }


def result_response(request_id: str, result: dict[str, object]) -> dict[str, object]:
    return {
        "id": request_id,
        "protocolVersion": PROTOCOL_VERSION,
        "result": result,
    }


def handle_line(line: str) -> tuple[dict[str, object], bool]:
    """Compatibility handler for deterministic health and shutdown tests."""

    try:
        request = parse_request(line)
    except ProtocolError as error:
        return error_response("invalid", error.code), False

    request_id = str(request["id"])
    method = request["method"]
    if method == "runtime.shutdown":
        return result_response(request_id, {"status": "stopping"}), True
    if method == "runtime.health":
        return (
            result_response(
                request_id,
                {
                    "name": "aip-runtime",
                    "protocolVersion": PROTOCOL_VERSION,
                    "status": "ready",
                },
            ),
            False,
        )
    return error_response(request_id, "unsupported_request"), False


def encode_message(message: dict[str, object]) -> str:
    encoded = json.dumps(message, ensure_ascii=False, separators=(",", ":"), sort_keys=True)
    if len(encoded.encode("utf-8")) > MAX_MESSAGE_BYTES:
        return json.dumps(
            error_response("invalid", "message_too_large"),
            ensure_ascii=True,
            separators=(",", ":"),
            sort_keys=True,
        )
    return encoded
