"""Narrow loopback-only Ollama REST adapter."""

from __future__ import annotations

import http.client
import json
import socket
import threading
from collections.abc import Callable, Iterable
from contextlib import suppress
from typing import Any, Protocol, cast

from .protocol import (
    MAX_ASSISTANT_OUTPUT_BYTES,
    MAX_DISCOVERED_MODELS,
    MAX_HISTORY_MESSAGES,
    MAX_IDENTIFIER_LENGTH,
    MAX_MESSAGE_BYTES,
    MAX_STREAM_CHUNK_BYTES,
    ProtocolError,
    bounded_identifier,
)

OLLAMA_HOST = "127.0.0.1"
OLLAMA_PORT = 11434
MAX_PROVIDER_PAYLOAD_BYTES = 1_048_576
MAX_MODEL_SIZE_BYTES = 1 << 50


class ProviderError(RuntimeError):
    """Sanitized provider failure with a stable public code."""

    def __init__(self, code: str) -> None:
        super().__init__(code)
        self.code = code


class CancelledError(ProviderError):
    def __init__(self) -> None:
        super().__init__("generation_cancelled")


class ConnectionLike(Protocol):
    def request(
        self,
        method: str,
        url: str,
        body: str | None = None,
        headers: dict[str, str] | None = None,
    ) -> None: ...

    def getresponse(self) -> ResponseLike: ...

    def close(self) -> None: ...


class ResponseLike(Protocol):
    status: int

    def read(self, amount: int | None = None) -> bytes: ...

    def readline(self, limit: int = -1) -> bytes: ...


ConnectionFactory = Callable[[], ConnectionLike]
ConnectionObserver = Callable[[ConnectionLike | None], None]
ChunkEmitter = Callable[[int, str], None]


class _InterruptibleHttpConnection:
    """Close a streaming socket once and interrupt a concurrent blocking read."""

    def __init__(self, connection: http.client.HTTPConnection) -> None:
        self._connection = connection
        self._close_lock = threading.Lock()
        self._closed = False

    def request(
        self,
        method: str,
        url: str,
        body: str | None = None,
        headers: dict[str, str] | None = None,
    ) -> None:
        self._connection.request(method, url, body=body, headers=headers or {})

    def getresponse(self) -> ResponseLike:
        return cast(ResponseLike, self._connection.getresponse())

    def close(self) -> None:
        with self._close_lock:
            if self._closed:
                return
            self._closed = True
            sock = self._connection.sock
            if sock is not None:
                with suppress(OSError):
                    sock.shutdown(socket.SHUT_RDWR)
            self._connection.close()


def _request_connection() -> ConnectionLike:
    # HTTPConnection does not consult proxy environment variables and never follows redirects.
    return cast(
        ConnectionLike,
        http.client.HTTPConnection(OLLAMA_HOST, OLLAMA_PORT, timeout=4.0),
    )


def _stream_connection() -> ConnectionLike:
    return _InterruptibleHttpConnection(
        http.client.HTTPConnection(OLLAMA_HOST, OLLAMA_PORT, timeout=120.0)
    )


class OllamaClient:
    def __init__(self, connection_factory: ConnectionFactory | None = None) -> None:
        self._request_connection_factory = connection_factory or _request_connection
        self._stream_connection_factory = connection_factory or _stream_connection

    def discover(self) -> list[dict[str, object]]:
        payload = self._json_request("GET", "/api/tags")
        raw_models = payload.get("models")
        if not isinstance(raw_models, list):
            raise ProviderError("provider_malformed")
        if len(raw_models) > MAX_DISCOVERED_MODELS:
            raise ProviderError("provider_model_limit")
        models = [self._normalize_model(candidate) for candidate in raw_models]
        models.sort(key=lambda model: str(model["displayName"]).casefold())
        return models

    def show(self, model_id: str) -> dict[str, object]:
        model_id = _model_id(model_id)
        payload = self._json_request("POST", "/api/show", {"model": model_id, "verbose": False})
        details = payload.get("details")
        capabilities = payload.get("capabilities", [])
        if not isinstance(details, dict) or not isinstance(capabilities, list):
            raise ProviderError("provider_malformed")
        if len(capabilities) > 16:
            raise ProviderError("provider_malformed")
        safe_capabilities: list[str] = []
        for value in capabilities:
            normalized = _optional_text(value, 64)
            if normalized is None:
                raise ProviderError("provider_malformed")
            safe_capabilities.append(normalized)
        return {
            "providerModelId": model_id,
            "capabilities": safe_capabilities,
            "family": _optional_text(details.get("family"), 128),
            "parameterSize": _optional_text(details.get("parameter_size"), 64),
            "quantization": _optional_text(details.get("quantization_level"), 64),
        }

    def stream_chat(
        self,
        *,
        model_id: str,
        messages: list[dict[str, str]],
        keep_alive_minutes: int,
        cancel_event: threading.Event,
        observe_connection: ConnectionObserver,
        emit_chunk: ChunkEmitter,
    ) -> None:
        model_id = _model_id(model_id)
        if not 0 <= keep_alive_minutes <= 120:
            raise ProviderError("invalid_keep_alive")
        normalized_messages = _messages(messages)
        body = json.dumps(
            {
                "model": model_id,
                "messages": normalized_messages,
                "stream": True,
                "keep_alive": f"{keep_alive_minutes}m" if keep_alive_minutes else 0,
            },
            ensure_ascii=False,
            separators=(",", ":"),
        )
        if len(body.encode("utf-8")) > MAX_MESSAGE_BYTES:
            raise ProviderError("provider_request_too_large")

        try:
            connection = self._stream_connection_factory()
        except TimeoutError as error:
            raise ProviderError("provider_timeout") from error
        except (OSError, http.client.HTTPException) as error:
            raise ProviderError("provider_unavailable") from error
        observe_connection(connection)
        total_bytes = 0
        sequence = 0
        saw_done = False
        try:
            if cancel_event.is_set():
                raise CancelledError()
            connection.request(
                "POST",
                "/api/chat",
                body=body,
                headers={"Content-Type": "application/json", "Accept": "application/x-ndjson"},
            )
            response = connection.getresponse()
            if response.status != 200:
                raise ProviderError("provider_http_error")
            while True:
                if cancel_event.is_set():
                    raise CancelledError()
                raw_line = response.readline(MAX_MESSAGE_BYTES + 1)
                if not raw_line:
                    break
                if len(raw_line) > MAX_MESSAGE_BYTES:
                    raise ProviderError("provider_chunk_too_large")
                try:
                    chunk = json.loads(raw_line.decode("utf-8", errors="strict"))
                except (UnicodeDecodeError, json.JSONDecodeError, RecursionError) as error:
                    raise ProviderError("provider_stream_malformed") from error
                if not isinstance(chunk, dict):
                    raise ProviderError("provider_stream_malformed")
                if "error" in chunk:
                    raise ProviderError("provider_stream_error")
                message = chunk.get("message")
                if message is not None and not isinstance(message, dict):
                    raise ProviderError("provider_stream_malformed")
                if isinstance(message, dict):
                    content = message.get("content", "")
                    if not isinstance(content, str):
                        raise ProviderError("provider_stream_malformed")
                    tool_calls = message.get("tool_calls")
                    if tool_calls and not content:
                        raise ProviderError("provider_tools_unsupported")
                    # Raw thinking and tool calls are intentionally never emitted.
                    if content:
                        encoded_size = len(content.encode("utf-8"))
                        if encoded_size > MAX_STREAM_CHUNK_BYTES:
                            raise ProviderError("provider_chunk_too_large")
                        total_bytes += encoded_size
                        if total_bytes > MAX_ASSISTANT_OUTPUT_BYTES:
                            raise ProviderError("provider_output_too_large")
                        if cancel_event.is_set():
                            raise CancelledError()
                        sequence += 1
                        emit_chunk(sequence, content)
                done = chunk.get("done", False)
                if not isinstance(done, bool):
                    raise ProviderError("provider_stream_malformed")
                if done:
                    saw_done = True
                    break
            if cancel_event.is_set():
                raise CancelledError()
            if not saw_done:
                raise ProviderError("provider_stream_incomplete")
        except CancelledError:
            raise
        except (OSError, http.client.HTTPException, TimeoutError) as error:
            if cancel_event.is_set():
                raise CancelledError() from error
            raise ProviderError("provider_interrupted") from error
        finally:
            observe_connection(None)
            connection.close()

    def _json_request(
        self, method: str, path: str, body: dict[str, object] | None = None
    ) -> dict[str, Any]:
        try:
            connection = self._request_connection_factory()
        except TimeoutError as error:
            raise ProviderError("provider_timeout") from error
        except (OSError, http.client.HTTPException) as error:
            raise ProviderError("provider_unavailable") from error
        encoded = (
            json.dumps(body, ensure_ascii=False, separators=(",", ":"))
            if body is not None
            else None
        )
        try:
            connection.request(
                method,
                path,
                body=encoded,
                headers={"Content-Type": "application/json"} if encoded is not None else {},
            )
            response = connection.getresponse()
            if response.status != 200:
                raise ProviderError("provider_http_error")
            raw = response.read(MAX_PROVIDER_PAYLOAD_BYTES + 1)
            if len(raw) > MAX_PROVIDER_PAYLOAD_BYTES:
                raise ProviderError("provider_payload_too_large")
            try:
                payload = json.loads(raw.decode("utf-8", errors="strict"))
            except (UnicodeDecodeError, json.JSONDecodeError, RecursionError) as error:
                raise ProviderError("provider_malformed") from error
            if not isinstance(payload, dict):
                raise ProviderError("provider_malformed")
            return payload
        except ProviderError:
            raise
        except TimeoutError as error:
            raise ProviderError("provider_timeout") from error
        except (OSError, http.client.HTTPException) as error:
            raise ProviderError("provider_unavailable") from error
        finally:
            connection.close()

    @staticmethod
    def _normalize_model(candidate: object) -> dict[str, object]:
        if not isinstance(candidate, dict):
            raise ProviderError("provider_malformed")
        raw_id = candidate.get("model", candidate.get("name"))
        model_id = _model_id(raw_id)
        raw_size = candidate.get("size", 0)
        if not isinstance(raw_size, int) or isinstance(raw_size, bool):
            raise ProviderError("provider_malformed")
        if raw_size < 0 or raw_size > MAX_MODEL_SIZE_BYTES:
            raise ProviderError("provider_malformed")
        details = candidate.get("details", {})
        if not isinstance(details, dict):
            raise ProviderError("provider_malformed")
        return {
            "ref": f"ollama:{model_id}",
            "providerModelId": model_id,
            "displayName": model_id,
            "size": raw_size,
            "family": _optional_text(details.get("family"), 128),
            "parameterSize": _optional_text(details.get("parameter_size"), 64),
            "quantization": _optional_text(details.get("quantization_level"), 64),
            "capabilities": [],
        }


def _model_id(value: object) -> str:
    try:
        model_id = bounded_identifier(value, "invalid_model")
    except ProtocolError as error:
        raise ProviderError(error.code) from error
    if len(model_id) > MAX_IDENTIFIER_LENGTH or not all(
        character.isascii() and (character.isalnum() or character in ".:_/-")
        for character in model_id
    ):
        raise ProviderError("invalid_model")
    return model_id


def _optional_text(value: object, maximum: int) -> str | None:
    if value is None:
        return None
    if not isinstance(value, str) or len(value) > maximum or any(ord(char) < 32 for char in value):
        raise ProviderError("provider_malformed")
    return value or None


def _messages(values: Iterable[dict[str, str]]) -> list[dict[str, str]]:
    messages = list(values)
    if not messages or len(messages) > MAX_HISTORY_MESSAGES + 1:
        raise ProviderError("invalid_context")
    total_bytes = 0
    normalized: list[dict[str, str]] = []
    for candidate in messages:
        if set(candidate) != {"role", "content"}:
            raise ProviderError("invalid_context")
        role = candidate.get("role")
        content = candidate.get("content")
        if role not in {"system", "user", "assistant"} or not isinstance(content, str):
            raise ProviderError("invalid_context")
        encoded_size = len(content.encode("utf-8"))
        if not content or encoded_size > MAX_MESSAGE_BYTES:
            raise ProviderError("invalid_context")
        total_bytes += encoded_size
        if total_bytes > MAX_MESSAGE_BYTES:
            raise ProviderError("invalid_context")
        normalized.append({"role": role, "content": content})
    return normalized
