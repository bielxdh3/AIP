# AIP

**Agentes Independentes Personalizáveis**

AIP (Agentes Independentes Personalizáveis) is a local-first modular platform for
creating 2D agents with identity, memory, interchangeable models, and future BielOS
integration. The visual product name is **A.I.P.**

## Status

Phase 0 now has a repository implementation for the Windows desktop foundation:

- pnpm workspace;
- React and TypeScript main panel in Portuguese;
- Tauri and Rust application core;
- Rust-owned SQLite migration and two isolated provisional agents;
- managed Python runtime with a versioned NDJSON health handshake;
- transparent always-on-top overlay code paths, persisted drag positions, safe mode,
  deterministic placeholder animation states, and best-effort full-screen detection;
- automated TypeScript, Python, Rust, secret-scan, and CI definitions.

Native Rust checks and interactive Windows overlay behavior still require the Windows
toolchain described in [Windows setup](docs/WINDOWS_SETUP.md). No model, Ollama adapter,
real chat, memory, automation, Android client, or BielOS integration exists yet.

## Supported platform

- Windows 10 64-bit minimum
- Windows 11 64-bit

Linux, macOS, iOS, and Android are not supported by Phase 0.

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

## License

Licensed under the Apache License 2.0. See [LICENSE](LICENSE).
