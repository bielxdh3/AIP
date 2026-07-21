# AIP Agent Instructions

AIP means **Agentes Independentes Personalizáveis**.

AIP is a public, local-first desktop agent platform. It is being implemented as a standalone product before any BielOS integration.

Treat the repository as the source of truth. Do not use the planning chat as the only specification when repository documents exist.

## Core Priorities

1. Keep changes small, phase-scoped, and verifiable.
2. Preserve the separation between agent identity and model runtime.
3. Keep the desktop interface usable when Python, Ollama, or a model is unavailable.
4. Do not implement future phases unless explicitly requested.
5. Do not couple AIP to BielOS during standalone development.
6. Do not expose secrets, user data, private histories, models, exports, or local operational data.
7. Prefer boring, maintainable solutions over clever complexity.
8. Inspect repository state before making claims.
9. Do not claim a validation passed unless it was actually run and completed.
10. Keep user-facing interface text in Portuguese and repository code/documentation in English.

## Required AIP Skill Selection

Every AIP prompt must explicitly name the required AIP skill at the top before general instructions.

Use these skills by task type:

- Implementation, repository bootstrap, feature work, refactors, tests, or documentation tied to an implementation phase:
  `.agents/skills/aip-implementation/SKILL.md`
- Commit review, completed phase review, scope verification, or selecting the next phase:
  `.agents/skills/aip-phase-review/SKILL.md`
- Security, permissions, local process isolation, secrets, temporary chat, audit, tool execution, data protection, or remote-access preparation:
  `.agents/skills/aip-security-review/SKILL.md`
- Public repository review, secret scanning, release preparation, or publication safety:
  `.agents/skills/aip-publication-check/SKILL.md`

If no existing AIP skill matches the task, stop and report exactly:

`No matching AIP skill found for this task.`

Do not invent a new skill unless the user explicitly requests one.

## Product Boundaries

- Initial platform: Windows 10 64-bit.
- Windows 11 may be supported, but Windows 10 remains the minimum target.
- Android is a later planned platform.
- Linux, macOS, and iOS are explicitly out of scope.
- AIP is standalone first. BielOS integration is a later phase.
- The initial tester is the Owner only.
- The v0.1 account model uses an implicit local Owner profile without login, PIN, or idle lock.
- Two agents exist in the initial product scope. Dynamic creation of additional agents is later.
- Agent identity, memory, appearance, relationships, and state must not be stored inside a model-specific format.
- Models are replaceable and may be free without being open source.
- Ollama is the first adapter, not a permanent architectural dependency.

## v0.1 Boundaries

The v0.1 scope is defined in `docs/MVP_V0.1.md`.

Do not add these unless a later phase explicitly requests them:

- BielOS integration;
- Android application;
- Cloudflare Tunnel or Access;
- voice recognition or synthesis;
- voice cloning;
- autonomous conversations between agents;
- deep personality evolution;
- real Windows control or destructive tools;
- extension marketplace;
- screen vision;
- full agent export/import;
- remote APIs;
- multiple local user accounts.

## Architecture Rules

- Use a pnpm monorepo.
- Use Tauri and Rust for the desktop shell, authoritative state, persistence, process management, and OS integration.
- Use React and TypeScript for the UI.
- Use a Python runtime as a replaceable inference service.
- Rust owns authoritative persistence. Python returns inference results and memory candidates but does not directly own the primary database.
- Use SQLite as the authoritative database.
- Prefer newline-delimited JSON-RPC over managed process stdio for Rust-to-Python communication in the initial implementation.
- Runtime failure must not terminate the desktop UI.
- Only one heavy model generation may use the reference GPU at a time.
- Keep contracts versioned and model-agnostic.
- Bind any future local HTTP service to `127.0.0.1` only and require an ephemeral authentication mechanism.
- Do not add a local network server when stdio IPC is sufficient.

## Data and Privacy Rules

Never commit or print:

- `.env` contents;
- API keys or tokens;
- private keys;
- local databases;
- chat histories;
- memory stores;
- model files;
- exports or backups;
- local usernames or private paths unless they are documented placeholders;
- private BielOS operational details;
- screenshots or media containing personal data.

Use placeholders in examples.

Temporary chat must remain in memory only and must not write conversation content, summaries, memories, or learning records to disk.

## Output Rules

Keep output compact.

Do not narrate routine actions such as reading files, editing files, running standard commands, or inspecting diffs.

Only interrupt when:

- a blocking ambiguity requires user input;
- a command fails and changes the plan;
- a security-sensitive issue is found;
- the requested scope cannot be completed.

## Final Response Format

Use only:

- Status
- Files changed
- Behavior changed
- Validations run
- Validations not run
- Remaining limitations

Do not include motivational commentary, apologies, speculation, or long logs.

If a command fails, include only the command, relevant error lines, and the corrective action or limitation.

## Validation Expectations

Use the smallest relevant validation set.

Expected workspace commands after bootstrap:

```bash
pnpm secrets:scan
pnpm lint
pnpm typecheck
pnpm test
pnpm build
pnpm tauri:check
```

For Python runtime changes, also run the configured formatter, linter, type checker, and tests.

For docs-only changes, run only the secret scan when available.

Do not claim remote CI passed unless workflow results were confirmed.

## Codex Behavior

- Work in the currently opened AIP repository. Do not assume a local path.
- The existing BielOS repository may be available at `E:\BielOS`, but AIP must not import BielOS runtime code.
- BielOS `AGENTS.md` and `.agents/skills` may be used as a behavioral baseline only.
- Do not modify the BielOS repository while implementing AIP unless the user explicitly asks.
- Do not create unrelated refactors.
- Do not modify unrelated files.
- Do not add dependencies without clear justification.
- Prefer incremental commits with one coherent purpose.
- Keep the final answer minimal.