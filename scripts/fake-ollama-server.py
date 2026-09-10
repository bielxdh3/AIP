"""Deterministic, loopback-only Ollama fixture for installed Windows smoke tests."""

from __future__ import annotations

import argparse
import json
import re
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

MARKER = re.compile(r"marker-[A-Za-z0-9_-]{1,64}")


class FixtureHandler(BaseHTTPRequestHandler):
    server_version = "AIPFakeOllama/0.2.3.5"

    def log_message(self, _format: str, *_args: object) -> None:
        return

    def do_GET(self) -> None:  # noqa: N802
        if self.path == "/":
            self._send_text("Ollama is running")
        elif self.path == "/api/tags":
            self._send_json(
                {
                    "models": [
                        {
                            "name": "fixture:latest",
                            "size": 1,
                            "details": {"family": "fixture", "parameter_size": "1B"},
                        }
                    ]
                }
            )
        else:
            self.send_error(404)

    def do_POST(self) -> None:  # noqa: N802
        try:
            length = int(self.headers.get("Content-Length", "0"))
            if length <= 0 or length > 65_536:
                raise ValueError
            payload = json.loads(self.rfile.read(length).decode("utf-8"))
        except (ValueError, UnicodeDecodeError, json.JSONDecodeError):
            self.send_error(400)
            return
        if self.path == "/api/show":
            self._send_json({"details": {"family": "fixture"}, "capabilities": []})
            return
        if self.path != "/api/chat" or not isinstance(payload, dict):
            self.send_error(404)
            return
        model = payload.get("model")
        messages = payload.get("messages")
        if model != "fixture:latest" or not isinstance(messages, list):
            self.send_error(422)
            return
        marker = MARKER.search(json.dumps(messages, ensure_ascii=True))
        marker_text = marker.group(0) if marker else "marker-missing"
        response, chunks = fixture_response(marker_text)
        self._append_receipt(
            {
                "model": model,
                "marker": marker_text,
                "chunkCount": len(chunks),
                "responseBytes": len(response.encode("utf-8")),
            }
        )
        self.send_response(200)
        self.send_header("Content-Type", "application/x-ndjson")
        self.end_headers()
        for content in chunks:
            self.wfile.write(
                (json.dumps({"message": {"content": content}, "done": False}) + "\n").encode()
            )
            self.wfile.flush()
        self.wfile.write((json.dumps({"done": True}) + "\n").encode())
        self.wfile.flush()

    def _send_text(self, value: str) -> None:
        encoded = value.encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "text/plain; charset=utf-8")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)

    def _send_json(self, value: dict[str, object]) -> None:
        encoded = json.dumps(value, separators=(",", ":")).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)

    def _append_receipt(self, value: dict[str, object]) -> None:
        receipt = self.server.receipt  # type: ignore[attr-defined]
        with self.server.receipt_lock:  # type: ignore[attr-defined]
            with receipt.open("a", encoding="utf-8") as stream:
                stream.write(json.dumps(value, separators=(",", ":")) + "\n")


def fixture_response(marker: str) -> tuple[str, list[str]]:
    """Return deterministic content and a deliberately long stream for installed smoke."""

    if marker == "marker-one":
        response = "AIP-STREAM-TEST-OK"
        return response, [response[:4], response[4:11], response[11:16], response[16:]]
    if marker in {"marker-two", "marker-three"}:
        response = f"fixture-response:{marker}|" + "".join(
            f"{index:03d}," for index in range(128)
        )
        return response, list(response)
    response = f"fixture-response:{marker}"
    midpoint = max(1, len(response) // 2)
    return response, [response[:midpoint], response[midpoint:]]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=0)
    parser.add_argument("--ready-file", type=Path, required=True)
    parser.add_argument("--receipt", type=Path, required=True)
    args = parser.parse_args()
    args.ready_file.parent.mkdir(parents=True, exist_ok=True)
    args.receipt.parent.mkdir(parents=True, exist_ok=True)
    args.receipt.write_text("", encoding="utf-8")
    server = ThreadingHTTPServer(("127.0.0.1", args.port), FixtureHandler)
    server.receipt = args.receipt  # type: ignore[attr-defined]
    server.receipt_lock = threading.Lock()  # type: ignore[attr-defined]
    args.ready_file.write_text(str(server.server_port), encoding="ascii")
    try:
        server.serve_forever(poll_interval=0.05)
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
