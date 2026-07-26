# AIP

**Agentes Independentes Personalizáveis**

AIP (Agentes Independentes Personalizáveis) is a local-first modular platform for
creating 2D agents with identity, memory, interchangeable models, and future BielOS
integration. The visual product name is **A.I.P.**

## Status

Phase 0 is approved and complete. Phases 1 through 5 are implemented locally; the v0.1
candidate is prepared but remains pending the documented Windows and Ollama validation:

- pnpm workspace;
- React and TypeScript main panel in Portuguese;
- Tauri and Rust application core;
- Rust-owned SQLite migration and two isolated provisional agents;
- managed Python runtime with a versioned NDJSON health, discovery, generation, and
  cancellation protocol;
- loopback-only Ollama discovery and streaming chat adapter;
- one provisional main conversation per Astra and Luma, with isolated persistent histories;
- per-agent profiles, multiple persistent conversations, temporary in-memory chat, scoped
  memories, and bounded conversation summaries;
- deterministic states, silent/safe modes, suspension, and wake-now controls;
- per-agent 64×64 pixel documents rendered over the provisional sprites;
- one bounded FIFO generation queue, provisional global model selection, and configurable
  keep-alive;
- Portuguese conversation panel and independent compact/expanded overlay bubbles;
- transparent always-on-top overlay code paths, persisted drag positions, safe mode,
  deterministic placeholder animation states, and best-effort full-screen detection;
- automated TypeScript, Python, Rust, secret-scan, and CI definitions.

Runtime commit `a6ccb1badf6aa8a1f317ea1818c247d87f311fe6` passed the recorded Windows 11
manual test at 100% display scaling and its exact-SHA GitHub Actions run. Windows 10,
non-100% real display scaling, multiple monitors, and installer behavior remain manual
validation limitations. Phase 1 automated tests do not require Ollama, but real discovery,
streaming, queue, cancellation, bubble, restart, and provider-interruption behavior still
require the documented manual smoke test. No model is downloaded automatically. Memory,
tools, voice, Android, autonomous behavior, scheduling, and BielOS integration are not implemented.

## Supported platform

- Windows 10 64-bit minimum
- Windows 11 64-bit

Linux, macOS, iOS, and Android are not supported by Phase 1.

## Stack

- Tauri 2 and Rust for lifecycle, local persistence, process ownership, and overlays
- React 19 and TypeScript for the Portuguese desktop interface
- SQLite through bundled `rusqlite`
- Python 3.11+ as a replaceable managed child runtime
- pnpm workspaces

## Repository layout

```text
apps/desktop/               React UI and Tauri/Rust desktop core
packages/contracts/         Versioned TypeScript contracts and state rules
services/runtime/           Standard-library Python runtime boundary
scripts/                    Public-repository validation helpers
docs/                       Product, architecture, security, and setup documents
```

## Development

Install the [Windows prerequisites](docs/WINDOWS_SETUP.md), then run:

```powershell
pnpm install
python -m venv .venv
.\.venv\Scripts\Activate.ps1
python -m pip install -e ".\services\runtime[dev]"
pnpm check
pnpm dev
```

The application stores its database under the Tauri application-local data directory,
never in the repository. The Python runtime does not access SQLite and opens no network
listener.

## Documentation

- [Product specification](docs/PRODUCT_SPEC.md)
- [Architecture](docs/ARCHITECTURE.md)
- [MVP v0.1](docs/MVP_V0.1.md)
- [Data model](docs/DATA_MODEL.md)
- [UI behavior](docs/UI_BEHAVIOR.md)
- [Security and permissions](docs/SECURITY_AND_PERMISSIONS.md)
- [Roadmap](docs/ROADMAP.md)
- [Windows setup](docs/WINDOWS_SETUP.md)
- [Phase 0 validation](docs/PHASE_0_VALIDATION.md)
- [Phase 1 validation](docs/PHASE_1_VALIDATION.md)
- [v0.1 manual validation](docs/V0_1_MANUAL_VALIDATION.md)
- [Known limitations](docs/KNOWN_LIMITATIONS.md)
- [Candidate checklist](docs/RELEASE_CHECKLIST.md)

## License

Licensed under the Apache License 2.0. See [LICENSE](LICENSE).
