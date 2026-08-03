# AIP

**Agentes Independentes Personalizáveis** is a local-first desktop platform for creating 2D agents with separate identities, memories, conversations, models, and persistent state.

AIP is designed to run locally on Windows using a Tauri/Rust desktop core, a React and TypeScript interface, SQLite persistence, and a replaceable Python inference runtime. The visual product name is **A.I.P.**

> **Stable baseline:** v0.1.0. Post-v0.1 cognitive-core work is under active development and is not part of the stable baseline until reviewed and merged.

## Current capabilities

- two isolated provisional agents with independent profiles and histories;
- multiple persistent conversations and temporary in-memory chat;
- local Ollama discovery and streaming generation;
- scoped memories and bounded conversation summaries;
- deterministic fictional states and safe/silent controls;
- configurable local model selection and generation queue;
- compact and expanded desktop overlays with persisted positions;
- Rust-owned SQLite migrations and application state;
- managed, versioned Rust-to-Python NDJSON runtime protocol;
- automated TypeScript, Python, Rust, secret-scan, and CI validation.

## Platform support

- Windows 10 64-bit minimum;
- Windows 11 64-bit.

Linux, macOS, iOS, and Android are not supported by the current stable release.

## Technology

- Tauri 2 and Rust for lifecycle, persistence, process ownership, and overlays;
- React 19 and TypeScript for the desktop interface;
- SQLite through `rusqlite`;
- Python 3.11+ as a replaceable managed runtime;
- pnpm workspaces;
- Ollama through loopback-only local discovery.

## Repository layout

```text
apps/desktop/               React UI and Tauri/Rust desktop core
packages/contracts/         Shared versioned contracts and state rules
services/runtime/           Managed Python runtime boundary
scripts/                    Validation and repository tooling
docs/                       Product, architecture, security, and setup docs
```

## Development

Read the [Windows setup guide](docs/WINDOWS_SETUP.md), then run:

```powershell
pnpm install
python -m venv .venv
.\.venv\Scripts\Activate.ps1
python -m pip install -e ".\services\runtime[dev]"
pnpm check
pnpm dev
```

The application stores its database in the Tauri application-local data directory, not in the repository. The Python runtime does not own the primary database and opens no network listener.

## Validation

Use the repository commands appropriate to the change, including:

```powershell
pnpm secrets:scan
pnpm check
pnpm test
pnpm build
```

Platform-dependent overlay, packaging, and installed-runtime behavior require honest Windows validation and must not be claimed from CI alone.

## Current limitations

- Ollama must currently be started separately;
- installers are unsigned;
- the visual design is still being refined;
- Android, voice, screen vision, supervised external tools, extensions, and BielOS integration are future work;
- no production model is downloaded automatically.

See [Known limitations](docs/KNOWN_LIMITATIONS.md) and [v0.1 manual validation](docs/V0_1_MANUAL_VALIDATION.md) for the current evidence boundary.

## Documentation

- [Product specification](docs/PRODUCT_SPEC.md)
- [Architecture](docs/ARCHITECTURE.md)
- [MVP v0.1](docs/MVP_V0.1.md)
- [Data model](docs/DATA_MODEL.md)
- [Security and permissions](docs/SECURITY_AND_PERMISSIONS.md)
- [Roadmap](docs/ROADMAP.md)
- [Windows setup](docs/WINDOWS_SETUP.md)
- [Cognitive-core specification](docs/COGNITIVE_CORE_SPEC.md)

## Security and privacy

AIP is local-first, but local software can still expose private conversations, memories, models, or files through careless logging, committed artifacts, unsafe extensions, or broad external-tool permissions. Real user data, secrets, databases, model files, build output, and private BielOS material must remain outside Git.

See [SECURITY.md](SECURITY.md) for responsible vulnerability reporting.

## License

Licensed under the Apache License 2.0. See [LICENSE](LICENSE).