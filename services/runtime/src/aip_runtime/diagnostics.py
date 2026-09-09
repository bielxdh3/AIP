"""Bounded, content-free diagnostics for the managed runtime process."""

from __future__ import annotations

import json
import sys
from collections.abc import Callable

DIAGNOSTIC_PREFIX = "AIP_RUNTIME_DIAGNOSTIC "
TRACE_PREFIX = "AIP_RUNTIME_TRACE "
DIAGNOSTIC_CODES = frozenset(
    {
        "ollama_cancel_close_failed",
        "ollama_stream_cancelled",
        "ollama_stream_failed",
        "generation_validation_failed",
        "runtime_diagnostic_rejected",
        "runtime_request_exception",
        "runtime_server_exception",
        "runtime_shutdown_requested",
        "runtime_stdin_eof",
        "runtime_stdout_write_failed",
        "runtime_worker_exception",
    }
)
TRACE_EVENTS = frozenset(
    {
        "generation.accepted",
        "ollama.request.started",
        "ollama.first_chunk",
        "ollama.request.completed",
        "ollama.request.failed",
    }
)


def sanitize_diagnostic_code(candidate: object) -> str:
    if isinstance(candidate, str) and candidate in DIAGNOSTIC_CODES:
        return candidate
    return "runtime_diagnostic_rejected"


def emit_diagnostic(
    candidate: object,
    *,
    write: Callable[[str], object] | None = None,
) -> None:
    """Write one stable diagnostic code without exception or user content."""

    sink = write or sys.stderr.write
    code = sanitize_diagnostic_code(candidate)
    try:
        sink(f"{DIAGNOSTIC_PREFIX}{code}\n")
    except Exception:
        # Diagnostics must never become a second runtime failure.
        return


def emit_trace(
    event: object,
    *,
    request_id: object,
    model: object | None = None,
    error_code: object | None = None,
    write: Callable[[str], object] | None = None,
) -> None:
    """Write bounded, content-free request diagnostics for the native smoke path."""

    if (
        not isinstance(event, str)
        or event not in TRACE_EVENTS
        or not isinstance(request_id, str)
        or not request_id
        or len(request_id) > 128
        or not all(
            character.isascii() and (character.isalnum() or character in "_-:.")
            for character in request_id
        )
    ):
        return
    payload: dict[str, str] = {"event": event, "requestId": request_id}
    if isinstance(model, str) and model and len(model) <= 200:
        payload["model"] = model
    if isinstance(error_code, str) and error_code and len(error_code) <= 64:
        payload["errorCode"] = error_code
    sink = write or sys.stderr.write
    try:
        sink(f"{TRACE_PREFIX}{json.dumps(payload, separators=(',', ':'))}\n")
    except Exception:
        return
