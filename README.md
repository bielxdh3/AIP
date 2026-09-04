<div align="center">

# A.I.P.

**Agentes Independentes Personalizáveis**

Local-first 2D agents with their own identity, memory, conversations, model, and persistent fictional state.

[![Status](https://img.shields.io/badge/status-active%20development-orange)](#project-status)
[![Version](https://img.shields.io/badge/stable%20baseline-v0.1.0-blue)](#project-status)
[![Platform](https://img.shields.io/badge/platform-Windows-0078D4)](#requirements)
[![Desktop](https://img.shields.io/badge/desktop-Tauri%202-FFC131)](#technology)
[![License](https://img.shields.io/badge/license-Apache%202.0-green)](LICENSE)

AIP is a Windows desktop platform for running expressive local agents without turning their identity, memories, or private conversations into a cloud dependency.

</div>

> [!IMPORTANT]
> **v0.1.0 is the stable reviewed baseline.** Post-v0.1 cognitive-core work remains under active development and must not be presented as part of the stable release until it is reviewed and merged.

## The idea at a glance

```text
                         ┌──────────────────────┐
                         │      You speak       │
                         │  text · controls     │
                         └──────────┬───────────┘
                                    │
                         ┌──────────▼───────────┐
                         │    Desktop agent     │
                         │ overlay · identity   │
                         │ mood · conversations │
                         └──────────┬───────────┘
                                    │ Tauri commands
                    ┌───────────────▼────────────────┐
                    │         Rust desktop core       │
                    │ lifecycle · permissions · state │
                    │ queue · migrations · processes  │
                    └───────────┬───────────┬─────────┘
                                │           │
                     persistent │           │ managed NDJSON
                                │           │
                    ┌───────────▼──────┐ ┌──▼─────────────────┐
                    │ SQLite database  │ │ Python runtime      │
                    │ agents · memory  │ │ replaceable backend │
                    │ chats · settings │ └──┬─────────────────┘
                    └──────────────────┘    │ loopback only
                                           ▼
                                  ┌───────────────────┐
                                  │ Local model host  │
                                  │      Ollama       │
                                  └───────────────────┘
```

The desktop core owns application state and persistence. The Python service is a managed, replaceable inference boundary rather than the owner of the primary database.

## Why AIP exists

Most assistants are presented as disposable chat windows. AIP explores a different model: agents that feel persistent and distinct while keeping the user in control of local data, models, permissions, and behavior.

Each agent can have its own:

- identity and profile;
- conversation history;
- scoped memories and summaries;
- selected local model;
- fictional state such as energy, focus, or mood;
- overlay position and interface preferences.

## Project status

The current stable baseline already includes:

- [x] two isolated provisional agents;
- [x] persistent and temporary conversations;
- [x] local Ollama discovery and streaming generation;
- [x] scoped memories and bounded summaries;
- [x] deterministic fictional states;
- [x] configurable model selection and generation queue;
- [x] compact and expanded desktop overlays;
- [x] Rust-owned SQLite migrations and application state;
- [x] versioned Rust-to-Python NDJSON protocol;
- [x] TypeScript, Python, Rust, secret-scan, and CI validation.

> [!NOTE]
> AIP is functional software under active development, not a finished consumer product. Packaging, visual polish, model management, and platform validation are still evolving.

The long-term context and memory target is defined in [docs/CONTEXT_MEMORY_ARCHITECTURE.md](docs/CONTEXT_MEMORY_ARCHITECTURE.md). It specifies semantic compaction, supersession, temporary-context lifecycles, profile projections, graph-assisted retrieval, and dynamic token-budgeted context compilation. It is a normative roadmap target, not a claim that the stable release already implements that full architecture.

## Technology

| Layer | Responsibility | Technology |
|---|---|---|
| Desktop shell | Windows lifecycle, windows, overlays, process ownership | Tauri 2 + Rust |
| Interface | Agent UI, chats, settings, visual state | React 19 + TypeScript |
| Persistence | Agents, conversations, memories, settings, migrations | SQLite + `rusqlite` |
| Runtime boundary | Replaceable local inference service | Python 3.11+ |
| Model host | Local discovery and generation | Ollama |
| Workspace | Packages, scripts, validation | pnpm workspaces |

## Requirements

- Windows 10 64-bit or Windows 11 64-bit;
- Node.js 22 or newer;
- pnpm 11;
- Rust toolchain required by Tauri;
- Python 3.11 or newer;
- Ollama for the current local-model workflow.

Linux, macOS, and iOS are not supported by the current stable release. Phase 12 provides a functional Android APK with authenticated local/private explicit-connect transport and deterministic loopback coverage; Phase 13 provides a functional authenticated local/private `aip-gateway-v1` TCP checkpoint with Rust/SQLite authority and loopback validation. Physical-device/private-LAN, manual recovery/permission, remote CI, Cloudflare/BielOS and release-signing checks remain reserved; neither phase claims a stable release.

## Install

[Download the current stable v0.1.0 MSI](https://github.com/bielxdh3/AIP/releases/download/v0.1.0/A.I.P._0.1.0_x64_en-US.msi)

The active 0.2.1 line is unreleased development. The stable download above remains the reviewed v0.1.0 release asset.

## Quick start

### 1. Clone the repository

```powershell
git clone https://github.com/bielxdh3/AIP.git
cd AIP
```

### 2. Install JavaScript dependencies

```powershell
pnpm install
```

### 3. Prepare the Python runtime

```powershell
python -m venv .venv
.\.venv\Scripts\Activate.ps1
python -m pip install --upgrade pip
python -m pip install -e ".\services\runtime[dev]"
```

### 4. Validate the workspace

```powershell
pnpm check
```

### 5. Start the desktop app

Start Ollama separately, then run:

```powershell
pnpm dev
```

For the complete Windows environment setup, read [docs/WINDOWS_SETUP.md](docs/WINDOWS_SETUP.md).

## Repository map

```text
AIP/
├── apps/
│   └── desktop/             React interface + Tauri/Rust desktop core
├── packages/
│   └── contracts/           Shared contracts and deterministic state rules
├── services/
│   └── runtime/             Managed Python inference boundary
├── benchmarks/              Performance and evaluation material
├── scripts/                 Validation and repository tooling
├── docs/                    Product, architecture, security, and setup docs
├── SECURITY.md              Vulnerability reporting policy
└── README.md
```

## Validation

The main validation command combines secret scanning, linting, type checking, tests, builds, Python checks, and the Tauri/Rust check:

```powershell
pnpm check
```

Useful focused commands:

| Command | Purpose |
|---|---|
| `pnpm secrets:scan` | Detect secrets and unsafe repository artifacts |
| `pnpm lint` | Lint TypeScript and JavaScript |
| `pnpm typecheck` | Type-check workspace packages |
| `pnpm test` | Run workspace tests |
| `pnpm build` | Build contracts and desktop interface |
| `pnpm python:check` | Format, lint, type-check, and test the Python runtime |
| `pnpm tauri:check` | Validate the Tauri/Rust desktop core |

> [!WARNING]
> CI cannot honestly prove Windows overlay behavior, installer behavior, GPU/model compatibility, or installed-runtime behavior. Those areas require real Windows validation.

## Privacy and security model

AIP is local-first, but “local” does not automatically mean “safe.” The project treats the following boundaries as explicit responsibilities:

- the Rust core owns the primary database and managed runtime lifecycle;
- Ollama discovery is loopback-only in the current design;
- the Python runtime opens no network listener;
- real conversations, memories, databases, model files, and secrets must stay outside Git;
- external tools and future extensions must use explicit permission boundaries;
- temporary in-memory conversations must not silently become persistent memory;
- logs and diagnostics must avoid leaking private user content.

See [SECURITY.md](SECURITY.md) and [docs/SECURITY_AND_PERMISSIONS.md](docs/SECURITY_AND_PERMISSIONS.md).

## Current limitations

- Ollama must currently be started separately;
- installers are unsigned;
- the visual design is still being refined;
- no production model is downloaded automatically;
- screen vision now has a bounded on-demand Windows capture/provider checkpoint; local model installation and packaged/manual visual validation remain prerequisites; BielOS integration remains future work; Phase 10 extensions are now limited closed declarative packages with explicit Owner review;
- Android and other desktop operating systems are not part of the stable baseline.

The evidence boundary is documented in [docs/KNOWN_LIMITATIONS.md](docs/KNOWN_LIMITATIONS.md) and [docs/V0_1_MANUAL_VALIDATION.md](docs/V0_1_MANUAL_VALIDATION.md).

## Roadmap

- [ ] Refine the visual identity and pixel-art agent system
- [ ] Improve local model discovery and lifecycle management
- [ ] Add voice input and per-agent voice output
- [ ] Add supervised screen understanding and external tools
- [ ] Extend the cognitive core without breaking deterministic boundaries
- [ ] Implement the bounded semantic context and long-term memory compiler defined in `docs/CONTEXT_MEMORY_ARCHITECTURE.md`
- [ ] Build the Android companion experience
- [ ] Define an approved BielOS integration boundary
- [ ] Produce signed, reproducible Windows installers

## Documentation

- [Product specification](docs/PRODUCT_SPEC.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Context and memory architecture](docs/CONTEXT_MEMORY_ARCHITECTURE.md)
- [Phase H versioning and shipped behavior](docs/PHASE_H_VERSIONING_AND_SHIPPED_BEHAVIOR.md)
- [MVP v0.1](docs/MVP_V0.1.md)
- [Data model](docs/DATA_MODEL.md)
- [Security and permissions](docs/SECURITY_AND_PERMISSIONS.md)
- [Roadmap](docs/ROADMAP.md)
- [Windows setup](docs/WINDOWS_SETUP.md)
- [Cognitive-core specification](docs/COGNITIVE_CORE_SPEC.md)

## License

Licensed under the [Apache License 2.0](LICENSE).

## Disclaimer

AIP is an independent experimental project. Its fictional states and personalities are interface and simulation features; they are not evidence of consciousness, emotion, or sentience.
