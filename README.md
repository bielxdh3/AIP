# AIP

**Agentes Independentes Personalizáveis**

AIP is a local-first, modular platform for creating customizable 2D agents with independent identities, persistent memory, interchangeable language models, and a future integration path with BielOS.

The product name is **AIP**. The visual identity may use the stylized form **A.I.P.**

## Status

AIP is in specification and bootstrap stage. The first implementation target is a Windows 10 64-bit desktop application tested by the project owner.

## Initial technical direction

- Tauri and Rust for the Windows desktop shell and system integration
- React and TypeScript for the interface
- Python as a replaceable local AI runtime
- SQLite as the authoritative local database
- Ollama as the first model adapter
- pnpm monorepo
- Portuguese user interface
- English source code, internal identifiers, documentation, and repository content

## Product principles

- Local-first and usable without cloud services
- Agent identity must remain independent from the selected model
- The desktop interface must remain usable when Python, Ollama, or a model is unavailable
- Small, phase-scoped implementation increments
- Explicit permissions before state-changing or destructive actions
- Maximum practical use of open-source components
- No BielOS coupling during the standalone AIP implementation
- Windows 10/11 and Android are the only planned platforms

## Documentation

- [Product specification](docs/PRODUCT_SPEC.md)
- [Architecture](docs/ARCHITECTURE.md)
- [MVP v0.1](docs/MVP_V0.1.md)
- [Data model](docs/DATA_MODEL.md)
- [UI behavior](docs/UI_BEHAVIOR.md)
- [Security and permissions](docs/SECURITY_AND_PERMISSIONS.md)
- [Roadmap](docs/ROADMAP.md)
- [Codex Phase 0 prompt](docs/CODEX_PHASE_0_PROMPT.md)

## License

Licensed under the Apache License 2.0. See [LICENSE](LICENSE).