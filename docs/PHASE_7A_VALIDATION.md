# Phase 7A validation

## Automated evidence

- Rust compilation covers migration loading and the typed Rust/Tauri boundary.
- Focused repository tests must cover protected-target rejection, bounded deltas and windows, idempotency, rollback, isolation, ineligible sources, malformed candidates, transactional failure, migration preservation, restart persistence, model independence, and sanitized errors before approval.

## Manual Windows checklist

- Open an existing v0.1 database and confirm migration preserves Astra/Luma profiles, chats, memory, overlays, state, and model selection.
- Confirm Astra and Luma trait isolation, Portuguese history/explanations, protected-target rejection, owner-correction persistence, and rollback persistence after restart.
- Confirm a temporary chat creates no cognitive history; safe mode remains usable; and no unexpected CPU or model activity occurs.

## Implemented scope

Phase 7A provides deterministic Rust/SQLite trait events, owner correction, safe explanation projection, compensating rollback, checkpoints, and minimal audit metadata. Ordinary candidates use typed controlled-internal or persisted conversation-message sources; conversation messages are only validated as a future-adapter boundary and do not trigger live model extraction. Evidence identity is independent of idempotency; source identifiers, not conversation content or hidden reasoning, are retained. The Portuguese panel has deterministic mocked coverage for loading, empty history, trait visibility, validation, refreshes, eligible rollback, safe explanations, stable errors, agent switching, late responses, degraded responses, safe mode, accessible labels, and simulated-state wording.

The automated matrix covers policy limits and anti-oscillation, idempotency/evidence identity, corrections, compensating rollback and original-event immutability, source eligibility/redaction, migration/reopen persistence, isolation, contracts, and frontend behavior. The global formatting baseline remains limited to unchanged `apps/desktop/src-tauri/gen/schemas/{acl-manifests,capabilities,desktop-schema,windows-schema}.json`, `apps/desktop/src/{App.css,components/AgentSprite.tsx,conversation-state.test.ts,conversation-state.ts,pixel-document.test.ts,pixel-document.ts,use-phase-one.ts}`, and `pnpm-lock.yaml`. Phase 7A is automated-validation complete but not approved.

## Automated command results

- `pnpm secrets:scan`: passed (117 repository files).
- Touched-file Prettier and `cargo fmt --check`: passed. `pnpm format:check` reports only the 12 unchanged baseline paths above.
- `pnpm lint`, `pnpm typecheck`, contracts tests (5), desktop tests (39), and the desktop production build: passed.
- `cargo check --locked`, Clippy with `-D warnings`, and full Rust/Tauri tests: passed (71 passed, 1 ignored local-Ollama integration test). Migration coverage is included in the Rust suite.

## Not yet validated

No live extraction or Windows/manual validation is claimed by this implementation task. Manual Windows validation and approval remain pending.
