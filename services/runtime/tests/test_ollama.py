from __future__ import annotations

import json
import socket
import threading
import unittest
from typing import Any, cast

from aip_runtime.ollama import (
    CancelledError,
    ConnectionLike,
    OllamaClient,
    ProviderError,
    ResponseLike,
    _InterruptibleHttpConnection,
)
from aip_runtime.protocol import MAX_ASSISTANT_OUTPUT_BYTES, MAX_DISCOVERED_MODELS


class FakeResponse(ResponseLike):
    def __init__(self, payload: bytes | list[bytes], status: int = 200) -> None:
        self.status = status
        self._payload = payload if isinstance(payload, bytes) else b"".join(payload)
        self._lines = iter(payload if isinstance(payload, list) else [payload])

    def read(self, amount: int | None = None) -> bytes:
        return self._payload if amount is None else self._payload[:amount]

    def readline(self, limit: int = -1) -> bytes:
        try:
            value = next(self._lines)
        except StopIteration:
            return b""
        return value if limit < 0 else value[:limit]


class FakeConnection(ConnectionLike):
    def __init__(self, response: ResponseLike) -> None:
        self.response = response
        self.requests: list[tuple[str, str, str | None, dict[str, str]]] = []
        self.closed = False

    def request(
        self,
        method: str,
        url: str,
        body: str | None = None,
        headers: dict[str, str] | None = None,
    ) -> None:
        self.requests.append((method, url, body, headers or {}))

    def getresponse(self) -> ResponseLike:
        return self.response

    def close(self) -> None:
        self.closed = True


class FakeSocket:
    def __init__(self) -> None:
        self.shutdown_calls: list[int] = []

    def shutdown(self, how: int) -> None:
        self.shutdown_calls.append(how)


class FakeRawHttpConnection:
    def __init__(self) -> None:
        self.sock = FakeSocket()
        self.close_calls = 0

    def request(self, *_args: Any, **_kwargs: Any) -> None:
        return

    def getresponse(self) -> ResponseLike:
        return FakeResponse(b"{}")

    def close(self) -> None:
        self.close_calls += 1


def client_for(response: ResponseLike) -> tuple[OllamaClient, FakeConnection]:
    connection = FakeConnection(response)
    return OllamaClient(lambda: connection), connection


class OllamaDiscoveryTests(unittest.TestCase):
    def test_discovery_normalizes_bounded_metadata(self) -> None:
        payload = json.dumps(
            {
                "models": [
                    {
                        "model": "synthetic:latest",
                        "size": 1234,
                        "digest": "not-exposed",
                        "details": {
                            "family": "synthetic",
                            "parameter_size": "4B",
                            "quantization_level": "Q4",
                        },
                    }
                ]
            }
        ).encode()
        client, connection = client_for(FakeResponse(payload))
        models = client.discover()
        self.assertEqual(models[0]["ref"], "ollama:synthetic:latest")
        self.assertNotIn("digest", models[0])
        self.assertEqual(connection.requests[0][0:2], ("GET", "/api/tags"))

    def test_zero_models_and_selected_model_details(self) -> None:
        empty_client, _ = client_for(FakeResponse(b'{"models":[]}'))
        self.assertEqual(empty_client.discover(), [])
        details = {
            "details": {
                "family": "synthetic",
                "parameter_size": "7B",
                "quantization_level": "Q4_K_M",
            },
            "capabilities": ["completion"],
        }
        show_client, connection = client_for(FakeResponse(json.dumps(details).encode()))
        shown = show_client.show("synthetic:latest")
        self.assertEqual(shown["capabilities"], ["completion"])
        self.assertEqual(connection.requests[0][0:2], ("POST", "/api/show"))

    def test_unavailable_malformed_excessive_and_redirect_are_safe(self) -> None:
        def unavailable() -> ConnectionLike:
            raise OSError("synthetic")

        with self.assertRaisesRegex(ProviderError, "provider_unavailable"):
            OllamaClient(unavailable).discover()

        def timeout() -> ConnectionLike:
            raise TimeoutError("synthetic")

        with self.assertRaisesRegex(ProviderError, "provider_timeout"):
            OllamaClient(timeout).discover()
        malformed, _ = client_for(FakeResponse(b'{"models":"bad"}'))
        with self.assertRaisesRegex(ProviderError, "provider_malformed"):
            malformed.discover()
        excessive_payload = json.dumps(
            {"models": [{"model": f"m{index}"} for index in range(MAX_DISCOVERED_MODELS + 1)]}
        ).encode()
        excessive, _ = client_for(FakeResponse(excessive_payload))
        with self.assertRaisesRegex(ProviderError, "provider_model_limit"):
            excessive.discover()
        redirected, _ = client_for(FakeResponse(b"", status=302))
        with self.assertRaisesRegex(ProviderError, "provider_http_error"):
            redirected.discover()


class OllamaStreamingTests(unittest.TestCase):
    def test_stream_close_interrupts_socket_once(self) -> None:
        raw = FakeRawHttpConnection()
        connection = _InterruptibleHttpConnection(cast(Any, raw))
        connection.close()
        connection.close()
        self.assertEqual(raw.sock.shutdown_calls, [socket.SHUT_RDWR])
        self.assertEqual(raw.close_calls, 1)

    def run_chat(
        self,
        lines: list[bytes],
        *,
        cancel: threading.Event | None = None,
    ) -> tuple[list[tuple[int, str]], FakeConnection]:
        client, connection = client_for(FakeResponse(lines))
        chunks: list[tuple[int, str]] = []
        client.stream_chat(
            model_id="synthetic:latest",
            messages=[{"role": "user", "content": "Synthetic input"}],
            keep_alive_minutes=15,
            cancel_event=cancel or threading.Event(),
            observe_connection=lambda _connection: None,
            emit_chunk=lambda sequence, content: chunks.append((sequence, content)),
        )
        return chunks, connection

    def test_chat_body_streams_content_and_ignores_metrics_and_thinking(self) -> None:
        chunks, connection = self.run_chat(
            [
                b'{"message":{"role":"assistant","thinking":"hidden","content":"One"},"done":false}\n',
                b'{"message":{"role":"assistant","content":" two"},"done":false}\n',
                b'{"message":{"role":"assistant","content":""},"done":true,"eval_count":2}\n',
            ]
        )
        self.assertEqual(chunks, [(1, "One"), (2, " two")])
        body = json.loads(connection.requests[0][2] or "{}")
        self.assertEqual(body["model"], "synthetic:latest")
        self.assertTrue(body["stream"])
        self.assertEqual(body["keep_alive"], "15m")
        self.assertEqual(body["messages"][0]["content"], "Synthetic input")

    def test_midstream_error_malformed_tool_and_oversized_output_are_rejected(self) -> None:
        cases: list[tuple[list[bytes], str]] = [
            ([b'{"error":"private provider text"}\n'], "provider_stream_error"),
            ([b"not-json\n"], "provider_stream_malformed"),
            (
                [b'{"message":{"content":"","tool_calls":[{}]},"done":false}\n'],
                "provider_tools_unsupported",
            ),
            (
                [
                    json.dumps({"message": {"content": "x" * 8192}, "done": False}).encode() + b"\n"
                    for _ in range(MAX_ASSISTANT_OUTPUT_BYTES // 8192 + 1)
                ],
                "provider_output_too_large",
            ),
        ]
        for lines, expected in cases:
            with self.subTest(expected=expected), self.assertRaisesRegex(ProviderError, expected):
                self.run_chat(lines)

    def test_pre_cancelled_request_never_contacts_provider(self) -> None:
        cancel = threading.Event()
        cancel.set()
        client, connection = client_for(FakeResponse(b""))
        with self.assertRaises(CancelledError):
            client.stream_chat(
                model_id="synthetic:latest",
                messages=[{"role": "user", "content": "Synthetic input"}],
                keep_alive_minutes=0,
                cancel_event=cancel,
                observe_connection=lambda _connection: None,
                emit_chunk=lambda _sequence, _content: None,
            )
        self.assertEqual(connection.requests, [])


if __name__ == "__main__":
    unittest.main()
