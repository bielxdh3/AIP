from __future__ import annotations

import unittest

from aip_runtime.diagnostics import (
    DIAGNOSTIC_PREFIX,
    TRACE_PREFIX,
    emit_diagnostic,
    emit_trace,
)


class RuntimeDiagnosticTests(unittest.TestCase):
    def test_diagnostics_are_stable_bounded_and_content_free(self) -> None:
        output: list[str] = []
        emit_diagnostic("runtime_worker_exception", write=output.append)
        emit_diagnostic("private conversation text", write=output.append)
        emit_diagnostic("x" * 10_000, write=output.append)

        self.assertEqual(output[0], f"{DIAGNOSTIC_PREFIX}runtime_worker_exception\n")
        self.assertEqual(
            output[1:],
            [
                f"{DIAGNOSTIC_PREFIX}runtime_diagnostic_rejected\n",
                f"{DIAGNOSTIC_PREFIX}runtime_diagnostic_rejected\n",
            ],
        )
        self.assertTrue(all(len(line.encode("utf-8")) <= 96 for line in output))
        self.assertNotIn("private conversation text", "".join(output))

    def test_diagnostic_sink_failure_is_contained(self) -> None:
        def fail(_value: str) -> object:
            raise OSError("synthetic private sink error")

        emit_diagnostic("runtime_worker_exception", write=fail)

    def test_trace_is_correlated_bounded_and_content_free(self) -> None:
        output: list[str] = []
        emit_trace(
            "ollama.connected",
            request_id="request-1",
            model="fixture:latest",
            write=output.append,
        )
        emit_trace(
            "ollama.first_chunk",
            request_id="private conversation",
            write=output.append,
        )

        self.assertEqual(
            output,
            [
                f'{TRACE_PREFIX}{{"event":"ollama.connected","requestId":"request-1","model":"fixture:latest"}}\n'
            ],
        )
        self.assertNotIn("private conversation", "".join(output))


if __name__ == "__main__":
    unittest.main()
