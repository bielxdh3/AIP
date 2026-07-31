---
name: aip-implementation
description: Use for AIP bootstrap, phase-scoped implementation, feature work, refactors, tests, and implementation-linked documentation.
---

# AIP implementation

Load this skill before any implementation inspection or edit. Define acceptance criteria before editing.

## Focused process

1. Read only the relevant specification and confirm branch/worktree state.
2. Start with targeted searches for files, symbols, and references.
3. Inspect only files related to the acceptance criteria; do not scan the repository without explicit need.
4. Implement the smallest coherent change. Do not add future-phase work, opportunistic cleanup, or unrelated refactoring.
5. Run only the smallest validation capable of proving the change; do not validate the whole workspace for an isolated change.
6. Review the final diff for scope, secrets, generated files, and local data, then stop when acceptance criteria are met.

## Output and inspection discipline

- Do not reread unchanged files without a concrete reason.
- Use bounded excerpts instead of printing large files in full.
- Never print lockfiles, generated files, builds, databases, or complete logs.
- Examine command summaries, errors, and relevant lines only.
- Do not repeat `git status`, `git diff`, searches, tests, builds, or equivalent inspections without new evidence.
- Do not narrate routine actions or progress. Keep the final response short and factual.

## AIP invariants

Preserve the repository rules: AIP remains separate from BielOS; Rust/Tauri owns authoritative persistence, process management, and system integration; React/TypeScript owns UI; Python is replaceable inference only; runtime failure must not block the UI; identity, memory, and contracts remain model-independent and versioned; temporary chat stays in memory; UI text is Portuguese and source/documentation are English; security, privacy, data-integrity, and accessibility safeguards remain intact.

Do not add remote access, Android, voice, autonomous conversations, destructive tools, marketplace, vision, export/import, multiple accounts, or other later-scope features unless explicitly requested.

## Completion

Do not declare completion when required validation failed or was not run. Do not claim tests or CI without evidence.
