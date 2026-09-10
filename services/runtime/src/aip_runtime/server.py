"""Concurrent stdio server for provider discovery and one active generation."""

from __future__ import annotations

import threading
from collections.abc import Callable
from dataclasses import dataclass, field
from typing import IO, Any

from .diagnostics import emit_diagnostic, emit_trace
from .ollama import CancelledError, ConnectionLike, OllamaClient, ProviderError
from .protocol import (
    MAX_CONTEXT_BYTES,
    MAX_HISTORY_MESSAGES,
    MAX_MESSAGE_BYTES,
    MAX_USER_MESSAGE_BYTES,
    PROTOCOL_VERSION,
    ProtocolError,
    bounded_identifier,
    encode_message,
    error_response,
    parse_request,
    result_response,
)
from .readiness import OllamaRuntimeManager


@dataclass
class ActiveGeneration:
    request_id: str
    agent_id: str
    conversation_id: str
    assistant_message_id: str
    cancel_event: threading.Event = field(default_factory=threading.Event)
    connection: ConnectionLike | None = None
    thread: threading.Thread | None = None
    provider_chunks_received: int = 0
    provider_bytes_received: int = 0
    provider_characters_received: int = 0
    runtime_chunks_emitted: int = 0
    runtime_bytes_emitted: int = 0
    runtime_characters_emitted: int = 0
    runtime_terminal_emitted: int = 0


class RuntimeServer:
    def __init__(
        self,
        output: IO[str],
        client: OllamaClient | None = None,
        diagnostic: Callable[[object], None] = emit_diagnostic,
        trace: Callable[..., None] = emit_trace,
        provider_manager: OllamaRuntimeManager | None = None,
    ) -> None:
        self._output = output
        self._client = client or OllamaClient()
        self._provider_manager = provider_manager or OllamaRuntimeManager(client=self._client)
        self._diagnostic = diagnostic
        self._trace = trace
        self._write_lock = threading.Lock()
        self._active_lock = threading.Lock()
        self._active: ActiveGeneration | None = None
        self._generation_lock = threading.Lock()
        self._generation_workers: set[threading.Thread] = set()
        self._provider_lock = threading.Lock()
        self._provider_workers: set[threading.Thread] = set()

    def serve(self, input_stream: IO[bytes]) -> int:
        while raw_line := input_stream.readline(MAX_MESSAGE_BYTES + 2):
            if len(raw_line) > MAX_MESSAGE_BYTES:
                self._write(error_response("invalid", "message_too_large"))
                continue
            try:
                line = raw_line.decode("utf-8", errors="strict").rstrip("\r\n")
                request = parse_request(line)
            except UnicodeDecodeError:
                self._write(error_response("invalid", "malformed_utf8"))
                continue
            except ProtocolError as error:
                self._write(error_response("invalid", error.code))
                continue

            request_id = str(request["id"])
            method = str(request["method"])
            params = request["params"]
            assert isinstance(params, dict)

            try:
                if method == "runtime.health":
                    self._write(
                        result_response(
                            request_id,
                            {
                                "name": "aip-runtime",
                                "protocolVersion": PROTOCOL_VERSION,
                                "status": "ready",
                            },
                        )
                    )
                elif method == "runtime.shutdown":
                    self._diagnostic("runtime_shutdown_requested")
                    self.shutdown()
                    self._write(result_response(request_id, {"status": "stopping"}))
                    return 0
                elif method == "provider.discover":
                    self._spawn_provider(request_id, "discover", params)
                elif method == "provider.show":
                    self._spawn_provider(request_id, "show", params)
                elif method == "generation.start":
                    self._start_generation(request_id, params)
                elif method == "generation.cancel":
                    self._cancel_generation(request_id, params)
            except (BrokenPipeError, OSError):
                self._diagnostic("runtime_stdout_write_failed")
                self.shutdown()
                return 2
            except Exception:
                self._diagnostic("runtime_request_exception")
                try:
                    self._write(error_response(request_id, "runtime_request_failed"))
                except Exception:
                    self._diagnostic("runtime_stdout_write_failed")
                    self.shutdown()
                    return 2
        self._diagnostic("runtime_stdin_eof")
        self.shutdown()
        return 0

    def shutdown(self) -> None:
        self._provider_manager.shutdown()
        connection: ConnectionLike | None = None
        with self._active_lock:
            active = self._active
            if active is not None:
                active.cancel_event.set()
                connection = active.connection
        if connection is not None:
            self._close_connection(connection)
        with self._generation_lock:
            generation_workers = list(self._generation_workers)
        for worker in generation_workers:
            worker.join(timeout=5.0)
        with self._provider_lock:
            provider_workers = list(self._provider_workers)
        for worker in provider_workers:
            worker.join(timeout=5.0)

    def _spawn_provider(self, request_id: str, operation: str, params: dict[str, Any]) -> None:
        def run() -> None:
            try:
                if operation == "discover":
                    models = self._provider_manager.discover()
                    self._write(
                        result_response(
                            request_id,
                            {
                                "provider": "ollama",
                                "state": "available" if models else "empty",
                                "models": models,
                            },
                        )
                    )
                else:
                    model_id = bounded_identifier(params.get("model"), "invalid_model")
                    details = self._client.show(model_id)
                    self._write(result_response(request_id, {"model": details}))
            except (ProtocolError, ProviderError) as error:
                code = error.code
                self._write(error_response(request_id, code))
            except Exception:
                self._write(error_response(request_id, "provider_internal_error"))
            finally:
                with self._provider_lock:
                    self._provider_workers.discard(threading.current_thread())

        worker = threading.Thread(target=run, name="aip-provider", daemon=False)
        with self._provider_lock:
            self._provider_workers.add(worker)
        worker.start()

    def _start_generation(self, request_id: str, params: dict[str, Any]) -> None:
        try:
            active = self._validate_generation(request_id, params)
        except ProtocolError as error:
            self._diagnostic("generation_validation_failed")
            self._write(error_response(request_id, error.code))
            return

        with self._active_lock:
            if self._active is not None:
                self._write(error_response(request_id, "generation_busy"))
                return
            self._active = active

        def observe_connection(connection: ConnectionLike | None) -> None:
            with self._active_lock:
                if self._active is active:
                    active.connection = connection
            if connection is not None:
                self._trace(
                    "ollama.connected",
                    request_id=active.request_id,
                    model=str(params.get("model", "")),
                )

        def emit_chunk(sequence: int, content: str) -> None:
            if active.cancel_event.is_set():
                raise CancelledError()
            active.provider_chunks_received += 1
            active.provider_bytes_received += len(content.encode("utf-8"))
            active.provider_characters_received += len(content)
            if sequence == 1:
                self._trace(
                    "ollama.first_chunk",
                    request_id=active.request_id,
                    model=str(params.get("model", "")),
                    counters=self._stream_counters(active),
                )
            self._event(active, "generation.chunk", sequence=sequence, content=content)
            active.runtime_chunks_emitted += 1
            active.runtime_bytes_emitted += len(content.encode("utf-8"))
            active.runtime_characters_emitted += len(content)
            if sequence == 1:
                self._trace(
                    "runtime.chunk.emitted",
                    request_id=active.request_id,
                    model=str(params.get("model", "")),
                    counters=self._stream_counters(active),
                )

        def run() -> None:
            terminal = "generation.complete"
            error_code: str | None = None
            last_sequence = 0

            def emit_ordered_chunk(sequence: int, content: str) -> None:
                nonlocal last_sequence
                emit_chunk(sequence, content)
                last_sequence = sequence

            try:
                self._event(active, "generation.started", sequence=0)
                model = str(params["model"])
                messages = params["messages"]
                keep_alive = int(params["keepAliveMinutes"])
                assert isinstance(messages, list)
                self._trace(
                    "ollama.request.started",
                    request_id=active.request_id,
                    model=model,
                )
                self._trace(
                    "provider.stream.started",
                    request_id=active.request_id,
                    model=model,
                )
                # Discovery normally warms this manager, but generation must also
                # enforce the provider-ready contract when a packaged app starts
                # between refreshes or Ollama was restarted by the Owner.
                if callable(getattr(self._client, "health", None)):
                    self._provider_manager.ensure_ready()
                self._client.stream_chat(
                    model_id=model,
                    messages=messages,
                    keep_alive_minutes=keep_alive,
                    cancel_event=active.cancel_event,
                    observe_connection=observe_connection,
                    emit_chunk=emit_ordered_chunk,
                )
                self._trace(
                    "provider.stream.completed",
                    request_id=active.request_id,
                    model=model,
                    counters=self._stream_counters(active),
                )
            except CancelledError:
                terminal = "generation.cancelled"
                self._diagnostic("ollama_stream_cancelled")
            except ProviderError as error:
                terminal = "generation.failed"
                error_code = error.code
                self._diagnostic("ollama_stream_failed")
                self._trace(
                    "ollama.request.failed",
                    request_id=active.request_id,
                    model=str(params.get("model", "")),
                    error_code=error_code,
                    counters=self._stream_counters(active),
                )
            except Exception:
                terminal = "generation.failed"
                error_code = "generation_failed_unknown"
                self._diagnostic("runtime_worker_exception")
                self._trace(
                    "ollama.request.failed",
                    request_id=active.request_id,
                    model=str(params.get("model", "")),
                    error_code=error_code,
                    counters=self._stream_counters(active),
                )
            finally:
                with self._active_lock:
                    if self._active is active:
                        self._active = None
                try:
                    self._event(
                        active,
                        terminal,
                        sequence=last_sequence,
                        error_code=error_code,
                    )
                    active.runtime_terminal_emitted += 1
                    self._trace(
                        "runtime.terminal.emitted",
                        request_id=active.request_id,
                        model=str(params.get("model", "")),
                        error_code=error_code,
                        counters=self._stream_counters(active),
                    )
                    if terminal == "generation.complete":
                        self._trace(
                            "ollama.request.completed",
                            request_id=active.request_id,
                            model=str(params.get("model", "")),
                            counters=self._stream_counters(active),
                        )
                except Exception:
                    self._diagnostic("runtime_stdout_write_failed")
                finally:
                    with self._generation_lock:
                        self._generation_workers.discard(threading.current_thread())

        worker = threading.Thread(target=run, name="aip-generation", daemon=False)
        active.thread = worker
        with self._generation_lock:
            self._generation_workers.add(worker)
        self._write(result_response(request_id, {"status": "accepted"}))
        self._trace(
            "generation.accepted",
            request_id=request_id,
            model=str(params.get("model", "")),
        )
        worker.start()

    def _cancel_generation(self, request_id: str, params: dict[str, Any]) -> None:
        try:
            target = bounded_identifier(params.get("requestId"), "invalid_request_id")
        except ProtocolError as error:
            self._write(error_response(request_id, error.code))
            return
        connection: ConnectionLike | None = None
        with self._active_lock:
            active = self._active
            if active is None or active.request_id != target:
                self._write(error_response(request_id, "generation_not_active"))
                return
            active.cancel_event.set()
            connection = active.connection
        if connection is not None:
            self._close_connection(connection)
        self._write(result_response(request_id, {"status": "cancelling"}))

    def _close_connection(self, connection: ConnectionLike) -> None:
        try:
            connection.close()
        except Exception:
            self._diagnostic("ollama_cancel_close_failed")

    @staticmethod
    def _validate_generation(request_id: str, params: dict[str, Any]) -> ActiveGeneration:
        agent_id = bounded_identifier(params.get("agentId"), "invalid_agent")
        conversation_id = bounded_identifier(params.get("conversationId"), "invalid_conversation")
        assistant_message_id = bounded_identifier(
            params.get("assistantMessageId"), "invalid_message"
        )
        model = bounded_identifier(params.get("model"), "invalid_model")
        if model.startswith("ollama:"):
            raise ProtocolError("invalid_model")
        keep_alive = params.get("keepAliveMinutes")
        if (
            not isinstance(keep_alive, int)
            or isinstance(keep_alive, bool)
            or not 0 <= keep_alive <= 120
        ):
            raise ProtocolError("invalid_keep_alive")
        messages = params.get("messages")
        if not isinstance(messages, list) or not 1 <= len(messages) <= MAX_HISTORY_MESSAGES + 1:
            raise ProtocolError("invalid_context")
        context_bytes = 0
        for message in messages:
            if not isinstance(message, dict) or set(message) != {"role", "content"}:
                raise ProtocolError("invalid_context")
            role = message.get("role")
            content = message.get("content")
            if role not in {"system", "user", "assistant"} or not isinstance(content, str):
                raise ProtocolError("invalid_context")
            encoded = len(content.encode("utf-8"))
            message_limit = MAX_USER_MESSAGE_BYTES if role == "user" else MAX_CONTEXT_BYTES
            if not content or encoded > message_limit:
                raise ProtocolError("invalid_context")
            context_bytes += encoded
        if context_bytes > MAX_CONTEXT_BYTES:
            raise ProtocolError("invalid_context")
        params["model"] = model
        params["messages"] = messages
        params["keepAliveMinutes"] = keep_alive
        return ActiveGeneration(
            request_id=request_id,
            agent_id=agent_id,
            conversation_id=conversation_id,
            assistant_message_id=assistant_message_id,
        )

    def _event(
        self,
        active: ActiveGeneration,
        event_type: str,
        *,
        sequence: int | None = None,
        content: str | None = None,
        error_code: str | None = None,
    ) -> None:
        event: dict[str, object] = {
            "protocolVersion": PROTOCOL_VERSION,
            "event": event_type,
            "requestId": active.request_id,
            "agentId": active.agent_id,
            "conversationId": active.conversation_id,
            "assistantMessageId": active.assistant_message_id,
        }
        if sequence is not None:
            event["sequence"] = sequence
        if content is not None:
            event["content"] = content
        if error_code is not None:
            event["errorCode"] = error_code
        self._write(event)

    @staticmethod
    def _stream_counters(active: ActiveGeneration) -> dict[str, int]:
        return {
            "provider_chunks": active.provider_chunks_received,
            "provider_bytes": active.provider_bytes_received,
            "provider_characters": active.provider_characters_received,
            "runtime_chunks": active.runtime_chunks_emitted,
            "runtime_bytes": active.runtime_bytes_emitted,
            "runtime_characters": active.runtime_characters_emitted,
            "runtime_terminal_events": active.runtime_terminal_emitted,
        }

    def _write(self, message: dict[str, object]) -> None:
        encoded = encode_message(message)
        with self._write_lock:
            self._output.write(encoded + "\n")
            self._output.flush()
