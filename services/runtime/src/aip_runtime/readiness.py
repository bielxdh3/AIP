"""Bounded local Ollama readiness and owned-process lifecycle."""

from __future__ import annotations

import json
import os
import subprocess
import time
from collections.abc import Callable, Mapping
from contextlib import suppress
from dataclasses import dataclass
from pathlib import Path
from typing import Protocol

from .ollama import OllamaClient, ProviderError

MAX_CONFIG_BYTES = 16_384
MAX_PATH_LENGTH = 260
READINESS_TIMEOUT_SECONDS = 5.0
READINESS_POLL_SECONDS = 0.05
SHUTDOWN_TIMEOUT_SECONDS = 2.0
ALLOWED_EXECUTABLE_NAMES = frozenset(("ollama", "ollama.exe"))


class ProcessLike(Protocol):
    def poll(self) -> int | None: ...

    def terminate(self) -> None: ...

    def wait(self, timeout: float | None = None) -> int: ...

    def kill(self) -> None: ...


ProcessFactory = Callable[[Path], ProcessLike]


@dataclass(frozen=True)
class ProviderReadiness:
    provider: str
    state: str
    source: str


class OllamaRuntimeManager:
    def __init__(
        self,
        client: OllamaClient | None = None,
        *,
        environ: Mapping[str, str] | None = None,
        process_factory: ProcessFactory | None = None,
        monotonic: Callable[[], float] = time.monotonic,
        sleep: Callable[[float], None] = time.sleep,
        readiness_timeout: float = READINESS_TIMEOUT_SECONDS,
    ) -> None:
        self._client = client or OllamaClient()
        self._environ = dict(os.environ if environ is None else environ)
        self._process_factory = process_factory or _start_ollama
        self._monotonic = monotonic
        self._sleep = sleep
        self._readiness_timeout = max(0.0, readiness_timeout)
        self._process: ProcessLike | None = None
        self._started_process = False

    def discover(self) -> list[dict[str, object]]:
        self.ensure_ready()
        return self._client.discover()

    def ensure_ready(self) -> ProviderReadiness:
        first_error = self._probe_health()
        if first_error is None:
            source = "started" if self._started_process else "existing"
            return ProviderReadiness("ollama", "ready", source)

        if self._process is not None:
            if self._process.poll() is None:
                return self._wait_for_health(first_error, "started")
            self._forget_process()

        config = _load_config(self._environ)
        if config is None:
            raise first_error
        try:
            process = self._process_factory(config)
        except (OSError, ValueError, subprocess.SubprocessError) as error:
            raise ProviderError("provider_start_failed") from error
        self._process = process
        self._started_process = True
        try:
            return self._wait_for_health(first_error, "started")
        except ProviderError:
            self.shutdown()
            raise

    def shutdown(self) -> None:
        if not self._started_process or self._process is None:
            return
        process = self._process
        self._forget_process()
        if process.poll() is not None:
            return
        try:
            process.terminate()
            process.wait(timeout=SHUTDOWN_TIMEOUT_SECONDS)
        except subprocess.TimeoutExpired:
            if process.poll() is None:
                process.kill()
                with suppress(OSError, subprocess.SubprocessError):
                    process.wait(timeout=SHUTDOWN_TIMEOUT_SECONDS)
        except (OSError, subprocess.SubprocessError):
            pass

    def _probe_health(self) -> ProviderError | None:
        try:
            self._client.health()
        except ProviderError as error:
            return error
        except TimeoutError:
            return ProviderError("provider_timeout")
        except OSError:
            return ProviderError("provider_unavailable")
        except Exception:
            return ProviderError("provider_internal_error")
        return None

    def _wait_for_health(self, last_error: ProviderError, source: str) -> ProviderReadiness:
        deadline = self._monotonic() + self._readiness_timeout
        while True:
            error = self._probe_health()
            if error is None:
                return ProviderReadiness("ollama", "ready", source)
            last_error = error
            remaining = deadline - self._monotonic()
            if remaining <= 0:
                raise last_error
            self._sleep(min(READINESS_POLL_SECONDS, remaining))

    @property
    def started_process(self) -> bool:
        return self._started_process

    def _forget_process(self) -> None:
        self._process = None
        self._started_process = False


def _start_ollama(executable: Path) -> ProcessLike:
    return subprocess.Popen(
        [str(executable), "serve"],
        shell=False,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def _load_config(environ: Mapping[str, str]) -> Path | None:
    executable = environ.get("AIP_OLLAMA_EXECUTABLE")
    config_path = environ.get("AIP_OLLAMA_CONFIG")
    if executable is not None:
        return _validate_executable(executable)
    if config_path is None:
        return None

    path = _bounded_path(config_path)
    try:
        raw = path.read_bytes()
    except OSError as error:
        raise ProviderError("provider_config_invalid") from error
    if len(raw) > MAX_CONFIG_BYTES:
        raise ProviderError("provider_config_invalid")
    try:
        config = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError, RecursionError) as error:
        raise ProviderError("provider_config_invalid") from error
    if not isinstance(config, dict):
        raise ProviderError("provider_config_invalid")
    executable = config.get("executable")
    if not isinstance(executable, str):
        raise ProviderError("provider_config_invalid")
    return _validate_executable(executable)


def _validate_executable(value: str) -> Path:
    path = _bounded_path(value)
    if path.name.casefold() not in ALLOWED_EXECUTABLE_NAMES:
        raise ProviderError("provider_config_invalid")
    try:
        path = path.resolve(strict=True)
    except OSError as error:
        raise ProviderError("provider_config_invalid") from error
    if path.name.casefold() not in ALLOWED_EXECUTABLE_NAMES or not path.is_file():
        raise ProviderError("provider_config_invalid")
    return path


def _bounded_path(value: str) -> Path:
    if not value or len(value) > MAX_PATH_LENGTH or any(ord(character) < 32 for character in value):
        raise ProviderError("provider_config_invalid")
    path = Path(value)
    if not path.is_absolute():
        raise ProviderError("provider_config_invalid")
    return path
