from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

from aip_runtime import PROTOCOL_VERSION, handle_line, health_document
from aip_runtime.protocol import encode_message


class RuntimeProtocolTests(unittest.TestCase):
    def test_health_document_is_deterministic(self) -> None:
        expected = {
            "capabilities": [],
            "name": "aip-runtime",
            "protocolVersion": PROTOCOL_VERSION,
            "status": "ready",
        }
        self.assertEqual(health_document(), expected)
        self.assertEqual(encode_message(health_document()), encode_message(expected))

    def test_health_request_round_trip(self) -> None:
        response, should_stop = handle_line(
            json.dumps(
                {
                    "protocolVersion": PROTOCOL_VERSION,
                    "id": "health-test",
                    "method": "runtime.health",
                    "params": {},
                }
            )
        )
        self.assertFalse(should_stop)
        self.assertEqual(response["id"], "health-test")
        self.assertEqual(response["result"]["status"], "ready")  # type: ignore[index]

    def test_malformed_and_wrong_version_messages_are_rejected(self) -> None:
        malformed, _ = handle_line("not-json")
        wrong_version, _ = handle_line(
            '{"protocolVersion":99,"id":"x","method":"runtime.health","params":{}}'
        )
        self.assertEqual(malformed["error"]["code"], "malformed_json")  # type: ignore[index]
        self.assertEqual(wrong_version["error"]["code"], "unsupported_protocol")  # type: ignore[index]

    def test_health_cli_is_deterministic(self) -> None:
        source_root = Path(__file__).resolve().parents[1] / "src"
        environment = {"PYTHONPATH": str(source_root)}
        completed = subprocess.run(
            [sys.executable, "-m", "aip_runtime", "--health"],
            check=True,
            capture_output=True,
            text=True,
            env=environment,
        )
        self.assertEqual(json.loads(completed.stdout), health_document())
        self.assertEqual(completed.stderr, "")


if __name__ == "__main__":
    unittest.main()
