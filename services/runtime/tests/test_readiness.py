from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from aip_runtime.ollama import ProviderError
from aip_runtime.readiness import OllamaRuntimeManager, _start_ollama


class FakeClient:
    def __init__(self, health_results: list[object], discovery: object | None = None) -> None:
        self.health_results = health_results
        self.discovery = discovery if discovery is not None else []
        self.health_calls = 0
        self.discover_calls = 0

    def health(self) -> None:
        self.health_calls += 1
        result = self.health_results.pop(0) if self.health_results else None
        if isinstance(result, ProviderError):
            raise result

    def discover(self) -> list[dict[str, object]]:
        self.discover_calls += 1
        if isinstance(self.discovery, ProviderError):
            raise self.discovery
        return self.discovery  # type: ignore[return-value]


class FakeProcess:
    def __init__(self, *, exits_on_terminate: bool = True) -> None:
        self.exits_on_terminate = exits_on_terminate
        self.terminated = False
        self.killed = False
        self.wait_calls = 0

    def poll(self) -> int | None:
        return 0 if self.killed or (self.terminated and self.exits_on_terminate) else None

    def terminate(self) -> None:
        self.terminated = True

    def wait(self, timeout: float | None = None) -> int:
        del timeout
        self.wait_calls += 1
        if self.wait_calls == 1 and not self.exits_on_terminate:
            raise subprocess.TimeoutExpired("ollama", 2.0)
        return 0

    def kill(self) -> None:
        self.killed = True


class ReadinessTests(unittest.TestCase):
    def test_healthy_existing_provider_is_reused(self) -> None:
        client = FakeClient([None], [{"ref": "ollama:test"}])
        factory_calls: list[Path] = []
        manager = OllamaRuntimeManager(
            client,
            environ={},
            process_factory=lambda path: factory_calls.append(path) or FakeProcess(),
        )  # type: ignore[arg-type]

        readiness = manager.ensure_ready()
        self.assertEqual(readiness.source, "existing")
        self.assertFalse(factory_calls)
        self.assertEqual(manager.discover(), [{"ref": "ollama:test"}])
        self.assertEqual(client.health_calls, 2)

    def test_explicit_executable_allows_autostart_and_tracks_process(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            executable = Path(directory) / "ollama.exe"
            executable.touch()
            process = FakeProcess()
            factory_calls: list[Path] = []
            client = FakeClient([ProviderError("provider_unavailable"), None])
            manager = OllamaRuntimeManager(
                client,
                environ={"AIP_OLLAMA_EXECUTABLE": str(executable)},
                process_factory=lambda path: factory_calls.append(path) or process,
                readiness_timeout=0,
            )  # type: ignore[arg-type]

            readiness = manager.ensure_ready()
            self.assertEqual(readiness.source, "started")
            self.assertEqual(factory_calls, [executable])
            self.assertTrue(manager.started_process)
            manager.shutdown()
            self.assertTrue(process.terminated)

    def test_bounded_config_path_allows_explicit_autostart(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executable = root / "ollama.exe"
            executable.touch()
            config = root / "ollama.json"
            config.write_text(json.dumps({"executable": str(executable)}), encoding="utf-8")
            client = FakeClient([ProviderError("provider_unavailable"), None])
            paths: list[Path] = []
            manager = OllamaRuntimeManager(
                client,
                environ={"AIP_OLLAMA_CONFIG": str(config)},
                process_factory=lambda path: paths.append(path) or FakeProcess(),
                readiness_timeout=0,
            )  # type: ignore[arg-type]

            manager.ensure_ready()
            self.assertEqual(paths, [executable])

    def test_no_config_does_not_start_after_unavailable_health(self) -> None:
        client = FakeClient([ProviderError("provider_unavailable")])
        factory_called = False

        def factory(_path: Path) -> FakeProcess:
            nonlocal factory_called
            factory_called = True
            return FakeProcess()

        manager = OllamaRuntimeManager(client, environ={}, process_factory=factory)  # type: ignore[arg-type]
        with self.assertRaisesRegex(ProviderError, "provider_unavailable"):
            manager.ensure_ready()
        self.assertFalse(factory_called)

    def test_timeout_terminates_only_the_started_process(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            executable = Path(directory) / "ollama.exe"
            executable.touch()
            process = FakeProcess(exits_on_terminate=False)
            client = FakeClient(
                [ProviderError("provider_unavailable"), ProviderError("provider_timeout")]
            )
            manager = OllamaRuntimeManager(
                client,
                environ={"AIP_OLLAMA_EXECUTABLE": str(executable)},
                process_factory=lambda _path: process,
                monotonic=iter((0.0, 1.0)).__next__,
                readiness_timeout=0.5,
            )  # type: ignore[arg-type]

            with self.assertRaisesRegex(ProviderError, "provider_timeout"):
                manager.ensure_ready()
            self.assertTrue(process.terminated)
            self.assertTrue(process.killed)
            self.assertFalse(manager.started_process)

    def test_malformed_inventory_is_not_reported_as_ready(self) -> None:
        client = FakeClient([None], ProviderError("provider_malformed"))
        manager = OllamaRuntimeManager(client, environ={})  # type: ignore[arg-type]

        with self.assertRaisesRegex(ProviderError, "provider_malformed"):
            manager.discover()
        self.assertEqual(client.discover_calls, 1)

    def test_started_provider_is_reused_without_a_second_process(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            executable = Path(directory) / "ollama.exe"
            executable.touch()
            process = FakeProcess()
            factory_calls: list[Path] = []
            client = FakeClient([ProviderError("provider_unavailable"), None, None])
            manager = OllamaRuntimeManager(
                client,
                environ={"AIP_OLLAMA_EXECUTABLE": str(executable)},
                process_factory=lambda path: factory_calls.append(path) or process,
                readiness_timeout=0,
            )  # type: ignore[arg-type]

            self.assertEqual(manager.ensure_ready().source, "started")
            self.assertEqual(manager.ensure_ready().source, "started")
            self.assertEqual(factory_calls, [executable])

    def test_autostart_uses_serve_without_shell(self) -> None:
        executable = Path("C:/Program Files/Ollama/ollama.exe")
        with patch("aip_runtime.readiness.subprocess.Popen") as popen:
            _start_ollama(executable)
        popen.assert_called_once()
        args, kwargs = popen.call_args
        self.assertEqual(args[0], [str(executable), "serve"])
        self.assertFalse(kwargs["shell"])

    def test_shutdown_does_not_touch_an_existing_provider_process(self) -> None:
        client = FakeClient([None])
        manager = OllamaRuntimeManager(client, environ={})  # type: ignore[arg-type]
        manager.ensure_ready()
        manager.shutdown()
        self.assertFalse(manager.started_process)


if __name__ == "__main__":
    unittest.main()
