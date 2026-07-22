"""Bounded, content-free diagnostics for the managed runtime process."""

from __future__ import annotations

import sys
from collections.abc import Callable

DIAGNOSTIC_PREFIX = "AIP_RUNTIME_DIAGNOSTIC "
DIAGNOSTIC_CODES = frozenset(
    {
        "ollama_cancel_close_failed",
        "ollama_stream_cancelled",
        "ollama_stream_failed",
        "runtime_diagnostic_rejected",
        "runtime_request_exception",
        "runtime_server_exception",
        "runtime_shutdown_requested",
        "runtime_stdin_eof",
        "runtime_stdout_write_failed",
        "runtime_worker_exception",
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
