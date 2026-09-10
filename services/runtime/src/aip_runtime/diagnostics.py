"""Bounded, content-free diagnostics for the managed runtime process."""

from __future__ import annotations

import json
import sys
from collections.abc import Callable, Mapping

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
        "ollama.connected",
        "ollama.request.started",
        "ollama.first_chunk",
        "ollama.request.completed",
        "ollama.request.failed",
        "provider.stream.started",
        "provider.chunk.received",
        "provider.stream.completed",
        "runtime.chunk.emitted",
        "runtime.terminal.emitted",
    }
)
TRACE_COUNTER_KEYS = frozenset(
    {
        "provider_chunks",
        "provider_bytes",
        "provider_characters",
        "runtime_chunks",
        "runtime_bytes",
        "runtime_characters",
        "runtime_terminal_events",
        "rust_chunks",
        "rust_bytes",
        "rust_characters",
        "persisted_bytes",
        "persisted_chars",
        "persisted_batches",
        "heartbeat_max_latency_ms",
    }
)
MAX_TRACE_COUNTER = 2_147_483_647


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

    code = sanitize_diagnostic_code(candidate)
    try:
        if write is None:
            sys.stderr.write(f"{DIAGNOSTIC_PREFIX}{code}\n")
            sys.stderr.flush()
        else:
            write(f"{DIAGNOSTIC_PREFIX}{code}\n")
    except Exception:
        # Diagnostics must never become a second runtime failure.
        return


def emit_trace(
    event: object,
    *,
    request_id: object,
    model: object | None = None,
    error_code: object | None = None,
    counters: Mapping[str, int] | None = None,
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
    safe_counters: dict[str, int] = {}
    if counters is not None:
        for key, value in counters.items():
            if (
                not isinstance(key, str)
                or key not in TRACE_COUNTER_KEYS
                or not isinstance(value, int)
                or isinstance(value, bool)
                or value < 0
                or value > MAX_TRACE_COUNTER
            ):
                return
            safe_counters[key] = value
    payload: dict[str, object] = {"event": event, "requestId": request_id}
    if (
        isinstance(model, str)
        and model
        and len(model) <= 200
        and all(
            character.isascii() and (character.isalnum() or character in ".:_/-")
            for character in model
        )
    ):
        payload["model"] = model
    if (
        isinstance(error_code, str)
        and error_code
        and len(error_code) <= 64
        and all(
            character.isascii() and (character.islower() or character == "_")
            for character in error_code
        )
    ):
        payload["errorCode"] = error_code
    if safe_counters:
        payload["counters"] = safe_counters
    try:
        line = f"{TRACE_PREFIX}{json.dumps(payload, separators=(',', ':'))}\n"
        if write is None:
            sys.stderr.write(line)
            sys.stderr.flush()
        else:
            write(line)
    except Exception:
        return
