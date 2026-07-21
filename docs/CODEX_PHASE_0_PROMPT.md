# Codex Prompt — Phase 0

Copy the prompt below into Codex. Replace the first context placeholder with the `@` mention for the AIP planning chat if available.

---

`@<AIP planning chat>`

Required skill: `.agents/skills/aip-implementation/SKILL.md`

## Task

Bootstrap AIP Phase 0: repository workspace, resilient desktop shell, managed Python runtime skeleton, SQLite foundation, and the first two provisional pixel-agent overlays.

## Repository

Work in the currently opened `bielxdh3/AIP` repository.

Do not assume a local AIP path. Confirm the repository root and current Git state before changing files.

The BielOS repository may be available locally at `E:\BielOS`. Its `AGENTS.md` and `.agents/skills` were used as a behavioral baseline. The AIP repository now contains its own adapted `AGENTS.md` and skills. Follow the AIP files as authoritative. Do not modify BielOS and do not copy BielOS runtime code.

## Source of truth

Read and follow:

- `AGENTS.md`
- `.agents/skills/aip-implementation/SKILL.md`
- `docs/PRODUCT_SPEC.md`
- `docs/ARCHITECTURE.md`
- `docs/MVP_V0.1.md`
- `docs/DATA_MODEL.md`
- `docs/UI_BEHAVIOR.md`
- `docs/SECURITY_AND_PERMISSIONS.md`
- `docs/ROADMAP.md`

The referenced planning chat provides intent and historical context. Repository documents contain the resolved decisions and override older chat statements if they differ.

## Phase scope

Implement only Phase 0 from `docs/MVP_V0.1.md` and `docs/ROADMAP.md`.

### Required deliverables

1. Initialize a pnpm monorepo.
2. Add a Tauri desktop application using Rust, React, and TypeScript.
3. Keep all repository code, internal identifiers, comments, and documentation in English.
4. Keep all visible user-interface text in Portuguese.
5. Add a Rust application-core skeleton that owns:
   - application lifecycle;
   - safe-mode state;
   - runtime lifecycle state;
   - authoritative settings boundary;
   - initial SQLite connection and migration entry point.
6. Add a Python runtime skeleton under `services/runtime`.
7. Implement a minimal versioned NDJSON request/response health handshake over managed stdin/stdout between Rust and Python.
8. The Rust application must start, monitor, and stop the Python child process.
9. Runtime absence, startup failure, malformed response, or crash must not close the desktop UI.
10. Add a small shared contracts package for protocol types or schemas. Do not create a large abstract framework.
11. Add an initial SQLite migration framework and a minimal schema sufficient for:
    - implicit Owner initialization;
    - two provisional agent records;
    - basic agent positions;
    - application settings or safe-mode state where appropriate.
12. Add a minimal main panel showing:
    - AIP identity;
    - safe-mode status;
    - Python runtime status;
    - a clear degraded warning when runtime is unavailable;
    - two provisional agent cards.
13. Add two original provisional 64x64 pixel-art agent assets. Do not copy the planning-chat reference image or third-party art.
14. Add the first overlay proof:
    - transparent background;
    - always-on-top behavior;
    - two agents visible simultaneously;
    - opaque character region interactive;
    - transparent regions click through where supported;
    - drag and position update;
    - position restored after application restart;
    - hide overlays when safe mode is active;
    - attempt full-screen hiding through a small isolated Windows integration, but do not claim it is reliable unless manually validated.
15. Add minimal deterministic animation states or placeholders for:
    - idle;
    - dragged;
    - thinking.
16. Add automated tests for deterministic logic that does not require a live desktop:
    - safe-mode transitions;
    - runtime status transitions;
    - protocol parsing and malformed-message rejection;
    - two-agent data separation;
    - position persistence or repository logic.
17. Add setup and run instructions for Windows 10 64-bit.
18. Add scripts for the relevant validation commands and a basic secret scan appropriate for a public repository.

## Technical decisions

- Use pnpm.
- Use a workspace structure consistent with `docs/ARCHITECTURE.md`.
- Use Tauri, Rust, React, TypeScript, Python, and SQLite.
- Rust owns authoritative persistence.
- Python is a replaceable inference runtime and must not own the primary database.
- Initial Rust-to-Python IPC is NDJSON over managed stdio.
- Do not expose a local TCP server in Phase 0.
- Use versioned protocol envelopes and request identifiers.
- Keep the interface usable when Python is absent.
- Prefer a small working vertical shell over broad scaffolding.
- Keep Windows 10 64-bit as the minimum target.
- Choose current stable dependency versions that support Windows 10 and document any important compatibility decision.
- Do not hardcode a production model.

## Explicitly do not implement

Do not add:

- Ollama integration;
- real model generation;
- real chat;
- automatic memory extraction;
- full memory UI;
- temporary chat implementation;
- full pixel editor;
- advanced physics;
- window-edge or desktop-icon walking beyond isolated interfaces or TODO documentation;
- autonomous conversations;
- autonomous goals;
- voice;
- screen vision;
- real Windows control tools;
- extensions;
- Android;
- Cloudflare;
- BielOS integration;
- remote APIs;
- multiple user accounts;
- dynamic creation of more than two agents;
- unrelated refactors or speculative abstractions.

Do not present mocked behavior as completed functionality.

## Acceptance criteria

The phase is complete only when:

1. The monorepo installs with documented commands.
2. The desktop application can be built or checked in the available environment.
3. The main panel renders Portuguese UI text.
4. The application remains open when the Python runtime is missing or intentionally fails.
5. Safe mode starts without launching Python and hides overlay agents.
6. Two provisional agents are stored separately and shown simultaneously outside safe mode.
7. Dragged positions persist after restart through the authoritative Rust/SQLite path.
8. The IPC health handshake is versioned, tested, and rejects malformed messages safely.
9. Transparent overlay and click-through code paths exist and are manually documented if the environment cannot validate them.
10. No secrets, databases, model files, generated build output, user data, or private BielOS data are committed.
11. Documentation accurately separates automated validation from manual Windows validation.
12. The final diff contains no Phase 1 or later implementation.

## Validation

Run the smallest relevant complete set available after bootstrap, expected to include equivalents of:

```bash
pnpm install
pnpm secrets:scan
pnpm lint
pnpm typecheck
pnpm test
pnpm build
pnpm tauri:check
```

Run the configured Python formatter/linter/type checker/tests for the runtime skeleton.

If Windows GUI behavior cannot be validated in the execution environment, state exactly what was not validated. Do not claim transparent click-through, full-screen hiding, packaging, or Windows 10 execution succeeded without evidence.

## Git and commit

- Keep the change phase-scoped.
- Inspect the final diff.
- Do not commit generated artifacts or local data.
- Expected commit message: `chore: bootstrap AIP desktop workspace`
- Do not push, merge, or modify BielOS unless explicitly authorized by the user or required by the active Codex environment workflow.

## Final response

Use the repository-required format only:

- Status
- Files changed
- Behavior changed
- Validations run
- Validations not run
- Remaining limitations

Keep it compact.

---