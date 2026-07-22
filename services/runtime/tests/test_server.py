from __future__ import annotations

import io
import json
import threading
import time
import unittest
from typing import Any

from aip_runtime.ollama import CancelledError, ConnectionLike
from aip_runtime.server import RuntimeServer


class FakeConnection(ConnectionLike):
    def request(
        self,
        method: str,
        url: str,
        body: str | None = None,
        headers: dict[str, str] | None = None,
    ) -> None:
        del method, url, body, headers

    def getresponse(self) -> Any:
        raise AssertionError("unused")

    def close(self) -> None:
        return


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


def generation_params() -> dict[str, object]:
    return {
        "agentId": "agent-astra",
        "conversationId": "conversation-astra-main",
        "assistantMessageId": "message-assistant",
        "model": "synthetic:latest",
        "keepAliveMinutes": 15,
        "messages": [{"role": "user", "content": "Hello"}],
    }


def decoded_lines(output: io.StringIO) -> list[dict[str, object]]:
    return [json.loads(line) for line in output.getvalue().splitlines()]


class RuntimeServerTests(unittest.TestCase):
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
        server = RuntimeServer(output, client)  # type: ignore[arg-type]
        params = generation_params()
        params["messages"] = [{"role": "system", "content": ""}]
        server._start_generation("request-invalid", params)

        lines = decoded_lines(output)
        self.assertEqual(lines[0]["error"]["code"], "invalid_context")  # type: ignore[index]
        self.assertFalse(client.started.is_set())

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
