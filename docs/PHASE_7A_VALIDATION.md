# Phase 7A validation

## Automated evidence

- Rust compilation covers migration loading and the typed Rust/Tauri boundary.
- Focused repository tests must cover protected-target rejection, bounded deltas and windows, idempotency, rollback, isolation, ineligible sources, malformed candidates, transactional failure, migration preservation, restart persistence, model independence, and sanitized errors before approval.

## Manual Windows checklist

- Open an existing v0.1 database and confirm migration preserves Astra/Luma profiles, chats, memory, overlays, state, and model selection.
- Confirm Astra and Luma trait isolation, Portuguese history/explanations, protected-target rejection, owner-correction persistence, and rollback persistence after restart.
- Confirm a temporary chat creates no cognitive history; safe mode remains usable; and no unexpected CPU or model activity occurs.

## Not yet validated

No live Windows/manual validation is claimed by this implementation task. Phase 7A remains pending review and manual validation.
