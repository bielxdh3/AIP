# Phase 7A validation

## Automated evidence

- Rust compilation covers migration loading and the typed Rust/Tauri boundary.
- Focused repository tests must cover protected-target rejection, bounded deltas and windows, idempotency, rollback, isolation, ineligible sources, malformed candidates, transactional failure, migration preservation, restart persistence, model independence, and sanitized errors before approval.

## Manual Windows checklist

- Open an existing v0.1 database and confirm migration preserves Astra/Luma profiles, chats, memory, overlays, state, and model selection.
- Confirm Astra and Luma trait isolation, Portuguese history/explanations, protected-target rejection, owner-correction persistence, and rollback persistence after restart.
- Confirm a temporary chat creates no cognitive history; safe mode remains usable; and no unexpected CPU or model activity occurs.

## Implemented scope

Phase 7A partially provides deterministic Rust/SQLite trait events, owner correction, safe explanation projection, compensating rollback, checkpoints, and minimal audit metadata. Ordinary candidates now use typed controlled-internal or persisted conversation-message sources; conversation messages are only validated as a future adapter boundary and do not trigger live model extraction. Evidence identity is deterministic and independent of idempotency; source identifiers, not conversation content or hidden reasoning, are retained. The frontend exposes Portuguese inspection, explanation, correction, and rollback controls for Astra and Luma.

Focused backend source-eligibility, policy, persistence, rollback, and redaction tests are recorded with this change: the Rust suite has 69 passing tests and one intentionally ignored local-Ollama integration test. The global formatting baseline remains limited to unchanged `apps/desktop/src-tauri/gen/schemas/{acl-manifests,capabilities,desktop-schema,windows-schema}.json`, `apps/desktop/src/{App.css,components/AgentSprite.tsx,conversation-state.test.ts,conversation-state.ts,pixel-document.test.ts,pixel-document.ts,use-phase-one.ts}`, and `pnpm-lock.yaml`. Phase 7A remains incomplete: the full test matrix, additional frontend coverage, and manual Windows validation still remain. The phase is not approved.

## Not yet validated

No live Windows/manual validation is claimed by this implementation task. Phase 7A remains pending review and manual validation.
