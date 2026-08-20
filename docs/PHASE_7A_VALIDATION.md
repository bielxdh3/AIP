# Phase 7A validation

## Status

Phase 7A is approved/DONE for commit `3e591a06129a9d8f27e026490f9bd83028eb2465`.
The approval is based on the automated evidence, the Owner's current manual attestation
recorded below, and the successful CI for PR #3. It does not claim live model extraction or
any Phase 7B–7E behavior.

## Automated evidence

- The exact Phase 7A implementation/build baseline is
  `3e591a06129a9d8f27e026490f9bd83028eb2465`.
- The Phase A documentation baseline is
  `b6adccc754c7ba293d96c057d9e0f3fa71d0a3fd`. CI run
  [32337005681](https://github.com/bielxdh3/AIP/actions/runs/32337005681) completed
  successfully for that documentation commit; both `phase-zero` and `package` passed.
- The implementation/build baseline's earlier CI run
  [30671809304](https://github.com/bielxdh3/AIP/actions/runs/30671809304) also completed
  successfully; both `phase-zero` and `package` passed.
- Rust compilation covers migration loading and the typed Rust/Tauri boundary.
- The focused repository test matrix covers protected-target rejection, bounded deltas and
  windows, idempotency, rollback, isolation, ineligible sources, malformed candidates,
  transactional failure, migration preservation, restart persistence, model independence, and
  sanitized errors.

## Manual Windows checklist

- Open an existing v0.1 database and confirm migration preserves Astra/Luma profiles, chats, memory, overlays, state, and model selection.
- Confirm Astra and Luma trait isolation, Portuguese history/explanations, protected-target rejection, owner-correction persistence, and rollback persistence after restart.
- Confirm a temporary chat creates no cognitive history; safe mode remains usable; and no unexpected CPU or model activity occurs.

## Owner's current manual attestation

The human validation result applies to the exact build prepared from commit:

`3e591a06129a9d8f27e026490f9bd83028eb2465`

Build:

`A.I.P._0.1.0_x64-setup.exe`

SHA-256:

`686F22A24E3A5CF6E5DCDC311212E9BA2CB8D6AB4E5B3F76C8E6CCFD73DB8B74`

Prepared environment record:

- Windows 10 Pro x64
- build 19045
- 100% display scaling
- isolated validation data
- active data preserved by backup/isolation

The Owner's current attestation is:

**all Phase 7A manual checks passed and the tested behavior works.**

This records the Owner's statement as the human validation result. No observations beyond this
attestation are claimed.

## Implemented scope

Phase 7A provides deterministic Rust/SQLite trait events, owner correction, safe explanation projection, compensating rollback, checkpoints, and minimal audit metadata. Ordinary candidates use typed controlled-internal or persisted conversation-message sources; conversation messages are only validated as a future-adapter boundary and do not trigger live model extraction. Evidence identity is independent of idempotency; source identifiers, not conversation content or hidden reasoning, are retained. The Portuguese panel has deterministic mocked coverage for loading, empty history, trait visibility, validation, refreshes, eligible rollback, safe explanations, stable errors, agent switching, late responses, degraded responses, safe mode, accessible labels, and simulated-state wording.

The automated matrix covers policy limits and anti-oscillation, idempotency/evidence identity, corrections, compensating rollback and original-event immutability, source eligibility/redaction, migration/reopen persistence, isolation, contracts, and frontend behavior. The global formatting baseline remains limited to unchanged `apps/desktop/src-tauri/gen/schemas/{acl-manifests,capabilities,desktop-schema,windows-schema}.json`, `apps/desktop/src/{App.css,components/AgentSprite.tsx,conversation-state.test.ts,conversation-state.ts,pixel-document.test.ts,pixel-document.ts,use-phase-one.ts}`, and `pnpm-lock.yaml`. Phase 7A is automated-validation complete and approved/DONE under the Owner attestation recorded above.

## Automated command results

- `pnpm secrets:scan`: passed (117 repository files).
- Touched-file Prettier and `cargo fmt --check`: passed. `pnpm format:check` reports only the 12 unchanged baseline paths above.
- `pnpm lint`, `pnpm typecheck`, contracts tests (5), desktop tests (39), and the desktop production build: passed.
- `cargo check --locked`, Clippy with `-D warnings`, and full Rust/Tauri tests: passed (71 passed, 1 ignored local-Ollama integration test). Migration coverage is included in the Rust suite.

## Boundaries and remaining limitations

No live model extraction is claimed: persisted conversation messages remain a future-adapter
boundary, and Phase 7B–7E are not implemented. The Owner's attestation above is the complete
human validation evidence recorded here; no additional observations are inferred.
