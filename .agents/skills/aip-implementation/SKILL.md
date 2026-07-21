---
name: aip-implementation
description: Use for AIP repository bootstrap, phase-scoped implementation, feature work, refactors, tests, and implementation-linked documentation.
---

# AIP Implementation Skill

Use this skill for implementing AIP phases.

Keep output compact. Do not narrate routine inspection or progress.

## Required Process

1. Inspect `AGENTS.md` and the relevant specification documents.
2. Confirm the current branch and repository state.
3. Resolve the requested phase into explicit acceptance criteria.
4. Inspect only files relevant to the phase.
5. Implement the smallest coherent change that satisfies the acceptance criteria.
6. Add or update tests for behavior that can be tested.
7. Update documentation when architecture or user-visible behavior changes.
8. Run the smallest relevant validation set.
9. Inspect the final diff for scope creep, secrets, generated files, and accidental local data.
10. Do not declare completion when required validation failed or was not run.

## Architecture Constraints

- AIP remains standalone until a later explicit BielOS integration phase.
- Rust owns authoritative persistence, process management, and system integration.
- React and TypeScript own the user interface.
- Python is a replaceable inference runtime, not the source of truth for agent identity or persistence.
- Runtime failure must not crash or block access to the UI, history, settings, or safe mode.
- Agent identity and memory must remain model-independent.
- Use versioned contracts between layers.
- Keep temporary chat content in memory only.
- Keep all user-facing UI text in Portuguese.
- Keep source code, internal identifiers, comments, and documentation in English.

## Scope Control

Do not implement later roadmap items because they seem convenient.

Do not add:

- remote access;
- Android code;
- voice;
- autonomous agent conversations;
- destructive tools;
- BielOS imports;
- extension marketplace;
- screen vision;
- broad plugin systems;

unless the requested phase explicitly includes them.

## Dependency Rules

- Prefer standard library or already approved workspace dependencies.
- Explain any new dependency in the final response.
- Avoid large frameworks for small deterministic behavior.
- Pin or lock dependencies through the repository's normal package managers.

## Validation Guidance

Use relevant commands only. Expected commands after bootstrap include:

```bash
pnpm secrets:scan
pnpm lint
pnpm typecheck
pnpm test
pnpm build
pnpm tauri:check
```

Run Python validations for runtime changes.

For docs-only work, do not run build commands unless explicitly requested.

## Commit Guidance

Prefer one coherent commit per phase with a conventional message such as:

- `chore: bootstrap AIP workspace`
- `feat: add desktop agent overlay shell`
- `feat: add local chat persistence`
- `test: cover agent state transitions`

Do not commit generated local databases, model files, build outputs, secrets, exports, or user data.

## Final Response Format

Use only:

- Status
- Files changed
- Behavior changed
- Validations run
- Validations not run
- Remaining limitations