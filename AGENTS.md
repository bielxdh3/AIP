# AIP Agent Instructions

AIP means **Agentes Independentes Personalizáveis**. It is a public, local-first desktop platform developed standalone before any BielOS integration. Treat repository documents as the source of truth.

## Required skills

Load `.agents/skills/aip-implementation/SKILL.md` before inspecting files for implementation, bootstrap, feature, refactor, test, or implementation-linked documentation tasks.

Route other work as follows:

- Phase, commit, scope, or completed-result review: `.agents/skills/aip-phase-review/SKILL.md`.
- Security, permissions, process isolation, secrets, privacy, audit, tools, or data protection: `.agents/skills/aip-security-review/SKILL.md`.
- Public repository, secret scanning, release, or publication safety: `.agents/skills/aip-publication-check/SKILL.md`.
- Load Ponytail (`ponytail`) for code generation or refactoring when available.

If a required skill cannot be loaded, report that before implementing. Do not invent a skill unless explicitly requested.

## Architecture and scope

- Keep AIP standalone; do not import BielOS runtime code or modify BielOS.
- Target Windows 10 64-bit; Linux, macOS, and iOS are out of scope. Android is later scope.
- Use the pnpm monorepo with Tauri/Rust for the desktop shell, authoritative SQLite state, persistence, process management, and OS integration; React/TypeScript owns the UI; Python is a replaceable inference runtime only.
- Rust remains the source of truth for persistence. Keep agent identity, memory, and contracts model-independent and versioned.
- The initial tester is the local Owner; v0.1 has no login, PIN, or idle lock, and starts with two agents. Dynamic agent creation is later scope.
- Models are replaceable and may be free without being open source; Ollama is the first adapter, not a permanent dependency. Only one heavy generation may use the reference GPU at a time.
- Runtime failure must not terminate or block the UI, history, settings, or safe mode.
- Prefer newline-delimited JSON-RPC over managed stdio; any future local HTTP service binds only to `127.0.0.1` with ephemeral authentication, and must not replace sufficient stdio IPC.
- Preserve the v0.1 local Owner model and two-agent scope. Do not add future roadmap items without an explicit phase request: BielOS, Android, remote access/APIs, cloud access, voice, autonomous conversations, deep personality evolution, destructive Windows tools, marketplace, vision, export/import, or multiple local accounts.

## Security, privacy, language

- Never expose or commit `.env` contents, credentials, private keys, databases, histories, memories, models, exports, backups, personal data, private paths, BielOS operational details, or media containing personal data.
- Use placeholders in examples. Temporary chat content, summaries, memories, and learning records remain in memory only.
- User-facing UI text is Portuguese; source, identifiers, comments, and documentation are English.
- Preserve security, privacy, data-integrity, accessibility, and true-validation safeguards. Never claim a validation passed unless it actually completed.

## Repository discipline

- Keep changes small, phase-scoped, verifiable, and limited to the request. Do not add dependencies or perform unrelated cleanup/refactors.
- Inspect Git state before claims. Do not reset, discard, amend, rebase, force-push, or push unless explicitly authorized.
- Keep routine output compact and do not expose sensitive data or large logs.
