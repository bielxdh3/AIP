from __future__ import annotations

import io
import json
import queue
import threading
import time
import unittest
from typing import IO, Any, cast

from aip_runtime.ollama import CancelledError, ConnectionLike, ProviderError
from aip_runtime.server import RuntimeServer


class FakeConnection(ConnectionLike):
    def request(
        self,
        method: str,
        url: str,
        body: str | bytes | None = None,
        headers: dict[str, str] | None = None,
    ) -> None:
        del method, url, body, headers

    def getresponse(self) -> Any:
        raise AssertionError("unused")

    def close(self) -> None:
        return


class ExplodingCloseConnection(FakeConnection):
    def close(self) -> None:
        raise RuntimeError("synthetic private close detail")


class FakeClient:
    def __init__(self) -> None:
        self.started = threading.Event()

    def discover(self) -> list[dict[str, object]]:
        return [
            {
                "ref": "ollama:synthetic:latest",
                "providerModelId": "synthetic:latest",
                "displayName": "synthetic:latest",
                "size": 1,
                "family": "synthetic",
                "parameterSize": "1B",
                "quantization": "Q4",
                "capabilities": [],
            }
        ]

    def show(self, model_id: str) -> dict[str, object]:
        return {"providerModelId": model_id, "capabilities": ["completion"]}

    def stream_chat(
        self,
        *,
        model_id: str,
        messages: list[dict[str, str]],
        keep_alive_minutes: int,
        cancel_event: threading.Event,
        observe_connection: Any,
        emit_chunk: Any,
    ) -> None:
        del model_id, messages, keep_alive_minutes
        observe_connection(FakeConnection())
        self.started.set()
        emit_chunk(1, "Synthetic")
        if cancel_event.wait(timeout=2.0):
            raise CancelledError()
        emit_chunk(2, " reply")


class QueueInput:
    def __init__(self) -> None:
        self._lines: queue.Queue[bytes] = queue.Queue()

    def send(self, request: dict[str, object]) -> None:
        self._lines.put(json.dumps(request, separators=(",", ":")).encode() + b"\n")

    def readline(self, _limit: int = -1) -> bytes:
        return self._lines.get(timeout=2.0)


class LockedOutput:
    def __init__(self) -> None:
        self._lines: list[str] = []
        self._condition = threading.Condition()

    def write(self, value: str) -> int:
        with self._condition:
            self._lines.extend(line for line in value.splitlines() if line)
            self._condition.notify_all()
        return len(value)

    def flush(self) -> None:
        return

    def wait_for(self, predicate: Any, timeout: float = 2.0) -> dict[str, object]:
        deadline = time.monotonic() + timeout
        with self._condition:
            while True:
                for line in self._lines:
                    decoded = json.loads(line)
                    if predicate(decoded):
                        return decoded
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise AssertionError("synthetic output timeout")
                self._condition.wait(remaining)

    def decoded(self) -> list[dict[str, object]]:
        with self._condition:
            return [json.loads(line) for line in self._lines]


class LifecycleClient:
    def __init__(self) -> None:
        self.cancel_started = threading.Event()
        self.models: list[str] = []

    def discover(self) -> list[dict[str, object]]:
        return []

    def show(self, model_id: str) -> dict[str, object]:
        return {"providerModelId": model_id, "capabilities": []}

    def stream_chat(
        self,
        *,
        model_id: str,
        cancel_event: threading.Event,
        observe_connection: Any,
        emit_chunk: Any,
        **_kwargs: Any,
    ) -> None:
        self.models.append(model_id)
        if model_id == "failure:latest":
            raise ProviderError("provider_stream_failed")
        if model_id == "cancel:latest":
            observe_connection(ExplodingCloseConnection())
            self.cancel_started.set()
            if not cancel_event.wait(timeout=2.0):
                raise AssertionError("synthetic cancellation timeout")
            emit_chunk(1, "late synthetic chunk")
            raise AssertionError("late chunk must be rejected")
        emit_chunk(1, "Synthetic reply")


def generation_params() -> dict[str, object]:
    return {
        "agentId": "agent-astra",
        "conversationId": "conversation-astra-main",
        "assistantMessageId": "message-assistant",
        "model": "synthetic:latest",
        "keepAliveMinutes": 15,
        "messages": [{"role": "user", "content": "Hello"}],
    }


def request(request_id: str, method: str, params: dict[str, object]) -> dict[str, object]:
    return {
        "protocolVersion": 1,
        "id": request_id,
        "method": method,
        "params": params,
    }


def decoded_lines(output: io.StringIO) -> list[dict[str, object]]:
    return [json.loads(line) for line in output.getvalue().splitlines()]


class RuntimeServerTests(unittest.TestCase):
    def test_session_survives_complete_cancel_and_provider_failure_until_shutdown(self) -> None:
        input_stream = QueueInput()
        output = LockedOutput()
        diagnostics: list[object] = []
        client = LifecycleClient()
        server = RuntimeServer(
            output,  # type: ignore[arg-type]
            client,  # type: ignore[arg-type]
            diagnostics.append,
        )
        result: list[int] = []
        worker = threading.Thread(
            target=lambda: result.append(server.serve(cast(IO[bytes], input_stream)))
        )
        worker.start()

        input_stream.send(request("health-one", "runtime.health", {}))
        output.wait_for(lambda line: line.get("id") == "health-one")

        complete = generation_params()
        complete["model"] = "complete:latest"
        input_stream.send(request("complete-one", "generation.start", complete))
        output.wait_for(
            lambda line: (
                line.get("event") == "generation.complete"
                and line.get("requestId") == "complete-one"
            )
        )
        input_stream.send(request("health-two", "runtime.health", {}))
        output.wait_for(lambda line: line.get("id") == "health-two")

        cancelled = generation_params()
        cancelled["assistantMessageId"] = "assistant-cancelled"
        cancelled["model"] = "cancel:latest"
        input_stream.send(request("cancel-target", "generation.start", cancelled))
        self.assertTrue(client.cancel_started.wait(timeout=1.0))
        input_stream.send(
            request("cancel-command", "generation.cancel", {"requestId": "cancel-target"})
        )
        output.wait_for(
            lambda line: (
                line.get("event") == "generation.cancelled"
                and line.get("requestId") == "cancel-target"
            )
        )
        input_stream.send(request("health-three", "runtime.health", {}))
        output.wait_for(lambda line: line.get("id") == "health-three")

        failed = generation_params()
        failed["assistantMessageId"] = "assistant-failed"
        failed["model"] = "failure:latest"
        input_stream.send(request("failure-one", "generation.start", failed))
        output.wait_for(
            lambda line: (
                line.get("event") == "generation.failed" and line.get("requestId") == "failure-one"
            )
        )
        input_stream.send(request("health-four", "runtime.health", {}))
        output.wait_for(lambda line: line.get("id") == "health-four")

        final = generation_params()
        final["assistantMessageId"] = "assistant-final"
        final["model"] = "complete:latest"
        input_stream.send(request("complete-two", "generation.start", final))
        output.wait_for(
            lambda line: (
                line.get("event") == "generation.complete"
                and line.get("requestId") == "complete-two"
            )
        )
        input_stream.send(request("stop", "runtime.shutdown", {}))
        output.wait_for(lambda line: line.get("id") == "stop")
        worker.join(timeout=2.0)

        self.assertFalse(worker.is_alive())
        self.assertEqual(result, [0])
        events = [line for line in output.decoded() if line.get("event")]
        self.assertEqual(
            [line["event"] for line in events if line.get("requestId") == "cancel-target"],
            ["generation.started", "generation.cancelled"],
        )
        self.assertEqual(sum(line.get("event") == "generation.cancelled" for line in events), 1)
        self.assertIn("ollama_cancel_close_failed", diagnostics)
        self.assertIn("ollama_stream_cancelled", diagnostics)
        self.assertIn("ollama_stream_failed", diagnostics)
        self.assertIn("runtime_shutdown_requested", diagnostics)
        self.assertNotIn("runtime_request_exception", diagnostics)

    def test_provider_discovery_and_shutdown_use_versioned_envelopes(self) -> None:
        output = io.StringIO()
        server = RuntimeServer(output, FakeClient())  # type: ignore[arg-type]
        payload = b"\n".join(
            [
                b'{"protocolVersion":1,"id":"discover","method":"provider.discover","params":{}}',
                b'{"protocolVersion":1,"id":"stop","method":"runtime.shutdown","params":{}}',
                b"",
            ]
        )
        self.assertEqual(server.serve(io.BytesIO(payload)), 0)
        lines = decoded_lines(output)
        by_id = {str(line.get("id")): line for line in lines}
        self.assertEqual(by_id["discover"]["result"]["provider"], "ollama")  # type: ignore[index]
        self.assertEqual(by_id["stop"]["result"], {"status": "stopping"})
        self.assertTrue(all(line.get("protocolVersion") == 1 for line in lines))

    def test_generation_stream_is_correlated_and_cancelled(self) -> None:
        output = io.StringIO()
        client = FakeClient()
        server = RuntimeServer(output, client)  # type: ignore[arg-type]
        server._start_generation("request-one", generation_params())
        self.assertTrue(client.started.wait(timeout=1.0))
        server._cancel_generation("cancel-one", {"requestId": "request-one"})
        server.shutdown()

        lines = decoded_lines(output)
        events = [line for line in lines if line.get("event")]
        self.assertEqual(
            [event["event"] for event in events],
            ["generation.started", "generation.chunk", "generation.cancelled"],
        )
        self.assertTrue(all(event["requestId"] == "request-one" for event in events))
        self.assertEqual(events[1]["sequence"], 1)
        self.assertEqual(events[1]["content"], "Synthetic")
        self.assertEqual(events[-1]["sequence"], 1)
        self.assertTrue(all(line.get("error") is None for line in lines))

    def test_only_one_generation_can_be_active(self) -> None:
        output = io.StringIO()
        client = FakeClient()
        server = RuntimeServer(output, client)  # type: ignore[arg-type]
        server._start_generation("request-one", generation_params())
        self.assertTrue(client.started.wait(timeout=1.0))
        second = generation_params()
        second["assistantMessageId"] = "message-two"
        server._start_generation("request-two", second)
        server._cancel_generation("cancel-one", {"requestId": "request-one"})
        server.shutdown()

        lines = decoded_lines(output)
        busy = next(line for line in lines if line.get("id") == "request-two")
        self.assertEqual(busy["error"]["code"], "generation_busy")  # type: ignore[index]

    def test_shutdown_cancels_and_joins_active_generation(self) -> None:
        output = io.StringIO()
        client = FakeClient()
        server = RuntimeServer(output, client)  # type: ignore[arg-type]
        server._start_generation("request-one", generation_params())
        self.assertTrue(client.started.wait(timeout=1.0))
        server.shutdown()
        lines = decoded_lines(output)
        self.assertTrue(any(line.get("event") == "generation.cancelled" for line in lines))
        self.assertIsNone(server._active)

    def test_invalid_context_is_rejected_without_starting_worker(self) -> None:
        output = io.StringIO()
        client = FakeClient()
        diagnostics: list[object] = []
        server = RuntimeServer(output, client, diagnostics.append)  # type: ignore[arg-type]
        params = generation_params()
        params["messages"] = [{"role": "system", "content": ""}]
        server._start_generation("request-invalid", params)

        lines = decoded_lines(output)
        self.assertEqual(lines[0]["error"]["code"], "invalid_context")  # type: ignore[index]
        self.assertFalse(client.started.is_set())
        self.assertEqual(diagnostics, ["generation_validation_failed"])

    def test_assistant_context_above_user_limit_reaches_provider_dispatch(self) -> None:
        output = LockedOutput()
        client = LifecycleClient()
        server = RuntimeServer(output, client)  # type: ignore[arg-type]
        params = generation_params()
        params["model"] = "llama3.2:1b"
        params["messages"] = [
            {"role": "assistant", "content": "x" * 16_385},
            {"role": "user", "content": "Continue"},
        ]
        server._start_generation("llama-dispatch", params)
        output.wait_for(
            lambda line: (
                line.get("event") == "generation.complete"
                and line.get("requestId") == "llama-dispatch"
            )
        )
        self.assertEqual(client.models, ["llama3.2:1b"])
        server.shutdown()

    def test_concurrent_writes_are_complete_json_lines(self) -> None:
        output = io.StringIO()
        server = RuntimeServer(output, FakeClient())  # type: ignore[arg-type]
        threads = [
            threading.Thread(
                target=server._write,
                args=({"protocolVersion": 1, "id": f"line-{index}", "result": {}},),
            )
            for index in range(20)
        ]
        for thread in threads:
            thread.start()
        for thread in threads:
            thread.join(timeout=1.0)
        lines = decoded_lines(output)
        self.assertEqual(len(lines), 20)
        self.assertEqual({line["id"] for line in lines}, {f"line-{index}" for index in range(20)})

    def test_provider_worker_registry_is_empty_after_completion(self) -> None:
        output = io.StringIO()
        server = RuntimeServer(output, FakeClient())  # type: ignore[arg-type]
        server._spawn_provider("discover", "discover", {})
        deadline = time.monotonic() + 1.0
        while time.monotonic() < deadline:
            with server._provider_lock:
                if not server._provider_workers:
                    break
            time.sleep(0.005)
        with server._provider_lock:
            self.assertFalse(server._provider_workers)


if __name__ == "__main__":
    unittest.main()
