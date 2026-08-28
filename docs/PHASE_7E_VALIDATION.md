# Phase 7E validation

## Current closure evidence (2026-08-24)

Status: PRODUCTIZED — AUTOMATED VALIDATION COMPLETE; INSTALLED-WINDOWS HUMAN VALIDATION PENDING. On the current host,
`cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check`,
`cargo check --locked --manifest-path apps/desktop/src-tauri/Cargo.toml`, and
`cargo test --locked --manifest-path apps/desktop/src-tauri/Cargo.toml --lib cognitive::tests -- --test-threads=1`
pass (10 focused cognitive tests). `pnpm --filter @aip/desktop typecheck` and
`pnpm --filter @aip/desktop test -- --run` pass (14 files, 51 tests), and the
Owner-visible panel covers evidence provenance, all six relationship dimensions,
goal schedule/expiry, and fictional activity lifecycle. Conversation termination
reason, per-purpose opt-in/revocation, bounded budgets, and candidate-pending
behavior remain covered by the Rust conversation path.

Installed-Windows interaction, packaged restart, and subjective Portuguese review
are HUMAN VALIDATION PENDING. The earlier executor `spawn EPERM` and entrypoint
reservations below are retained as historical evidence and are superseded by the
current-host Vitest/Rust results.

## Historical executor evidence (retained)

`cargo test --locked --manifest-path apps/desktop/src-tauri/Cargo.toml cognitive --quiet`
passed with 13 tests. `cargo check --locked --manifest-path apps/desktop/src-tauri/Cargo.toml`
and `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check` passed.
Desktop Vitest was attempted on the current host and remained blocked by Windows
`spawn EPERM`; no frontend runtime pass is claimed.

## Status and exact scope

Phase 7E is implemented as a local Rust/SQLite path on the development branch. This
record is evidence for the implementation and static review; it is not a claim of
successful Windows runtime, manual, or remote-CI validation.

The exact implementation target under validation is the resulting `HEAD` of this
single correction commit (parent `0d4aff03b1d7eac0564ffec3dbf5bfc7a36f351c` on
`feat/phase-7b-7f-cognitive-core`). The symbolic `HEAD` reference resolves to the
exact commit containing this record.

- The 7B–7D baseline is the sequence `0fba15d079f7362e2c017771dc6050733b24958e`,
  `94d26c5088e8c64c3a5b5f12e7c82867dbca8ed4`,
  `e84284b8b90d90ed32b6fc21e6977c0ca1effd49`, and
  `821d8f9d468e50aedb4ec7b3d42dabcf8162d157`.
- The Phase 7E commits are `4de625841d6b87ad8821cdc98e947877eb61dbe7`
  (wiring), `70ba8fe88049c530cdd53c9585bcef3ede408a21` (boundary hardening),
  and `5ff4807985eb8156f2c59b7644700c43d19c58fc` (typed UI path), followed by
  the integrated correction at parent `0d4aff03b1d7eac0564ffec3dbf5bfc7a36f351c`
  and this focused blocker correction.
- The relevant migration sequence is `0012_phase7a_cognitive_events.sql` (7A),
  `0013_phase7b_7d_cognitive_core.sql` (7B–7D), and
  `0014_phase7e_7f_conversations.sql` (schema version 14). Database initialization
  applies them through the existing migration runner; 0014 persists policies,
  conversations, public turns, candidates, resource jobs, and the one-running-heavy-job
  constraint.

## Authoritative behavior reviewed

Rust owns validation and SQLite owns durable state. The public path has no private or
hidden agent channel. Public turns are bounded data, and the Rust boundary rejects
hidden-reasoning, private-channel, and complete-prompt material by inspecting keys and
bounded string values without executing or expanding the content. Candidate JSON is
attributed to its completed public conversation and remains `pending`; this phase has
no direct candidate-to-opinion, relationship, or goal application path.

The reviewed boundaries are:

- Both participating agents must explicitly opt in to the same stated purpose. The
  request and stored conversation are owner-scoped. Different owners cannot access
  the durable state; participant membership is enforced for public turns, candidate
  emission, and resource reservation/completion. Listing, inspection, interruption,
  and candidate rejection are owner-scoped Owner controls rather than private agent
  channels.
- Temporary chat, safe mode, silent mode, and suspended-agent state block autonomous
  conversation, candidate, and resource work. Owner interruption and candidate
  rejection remain explicit owner controls, while read-only inspection remains
  owner-scoped; every durable 7A–7D Tauri command also requires an explicit
  `temporaryChat` field and checks the active temporary-chat state, while the UI
  removes durable controls from temporary chat.
- After a conversation is created, append, candidate emission, resource reservation,
  and resource completion re-read both participant policies in their transaction.
  A false opt-in or non-null revocation blocks the operation, including an otherwise
  idempotent replay.
- Purpose, turns, tokens, duration, repetitions, public-turn content, candidate JSON,
  per-job resource units, and cumulative conversation resource usage are bounded.
  Adjacent echo/repetition detection, budget exhaustion, owner interruption, and
  errors terminate or prevent further work. A uniqueness constraint permits only one
  heavy generation at a time.
- Running resource work is recovered on database restart, idempotency keys replay the
  existing result where supported, and public turns/candidates/resources remain
  inspectable after reopen. No hidden reasoning, complete prompt, or private-channel
  record is persisted.

Focused Rust tests for these invariants exist in `conversation.rs`, including opt-in
and owner isolation, mode and temporary blocking, bounded termination, interruption,
one-heavy-job uniqueness, restart recovery, pending candidates, content screening,
participant membership, and cumulative resource-budget/complete guards. The typed
command-path test in `lib.rs` covers the registered Tauri wrappers. The focused
cognitive-panel test covers the owner-visible UI path, pending-only rejection,
temporary suppression, safe Portuguese error rendering, and the authoritative seeded
participant IDs. The fictional-activity Rust/contracts path is bounded and guarded;
the Portuguese panel exposes Owner-visible start, pause, resume, complete, expire,
and archive controls for fictional activities.

## Historical pre-host evidence (retained)

The table below records the earlier environment reservations without changing their
historical meaning; it is not the current-host validation result.

## Automated evidence

| Command                                                                                                                                      | Result   | Evidence boundary                                                                                                                        |
| -------------------------------------------------------------------------------------------------------------------------------------------- | -------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check`                                                                        | Passed   | Rust formatting completed locally.                                                                                                       |
| `cargo check --locked --manifest-path apps/desktop/src-tauri/Cargo.toml`                                                                     | Passed   | Rust/Tauri compilation completed; existing dead-code warnings were emitted, with no compile error.                                       |
| `cargo test --locked --manifest-path apps/desktop/src-tauri/Cargo.toml conversation --quiet`                                                 | Blocked  | The Windows test binary reached launch and failed with `STATUS_ENTRYPOINT_NOT_FOUND` (`0xc0000139`); no Rust test case is called passed. |
| `pnpm --filter @aip/desktop exec tsc --noEmit`                                                                                               | Passed   | Desktop TypeScript typecheck completed.                                                                                                  |
| `pnpm --filter @aip/contracts exec tsc -p tsconfig.json --noEmit`                                                                            | Passed   | Contracts TypeScript typecheck completed.                                                                                                |
| `pnpm exec eslint apps/desktop/src/App.tsx apps/desktop/src/cognitive-panel.test.tsx packages/contracts/src/index.ts --max-warnings=0`       | Passed   | Focused lint completed.                                                                                                                  |
| `pnpm exec prettier --check packages/contracts/src/index.ts`                                                                                 | Passed   | Contracts formatting completed.                                                                                                          |
| `pnpm exec prettier --check apps/desktop/src/App.tsx apps/desktop/src/cognitive-panel.test.tsx packages/contracts/src/index.ts`              | Reserved | Existing App/test formatting differences remain; no product formatting change is part of this validation checkpoint.                     |
| `pnpm --filter @aip/desktop exec vitest run src/cognitive-panel.test.tsx`                                                                    | Blocked  | Vite/Vitest startup failed before test execution with `spawn EPERM`.                                                                     |
| `pnpm --filter @aip/desktop exec vitest run src/cognitive-panel.test.tsx --config=false --pool=threads --no-file-parallelism --maxWorkers=1` | Not run  | The installed Vitest interpreted `false` as a config path (`Cannot resolve entry module false`); this did not produce test evidence.     |
| `git diff --check`                                                                                                                           | Passed   | The implementation baseline had no whitespace errors.                                                                                    |

## Runtime and publication boundary

No installed-Windows package, live model generation, restart session, Owner manual
attestation, or remote CI result was observed for this checkpoint. The Windows
`STATUS_ENTRYPOINT_NOT_FOUND` reservation and the pnpm/Vitest `spawn EPERM`
reservation must be cleared in a suitable environment before claiming runtime test
completion. Prettier differences remain an explicit baseline reservation. Phase 7E
therefore has a validated local implementation path, not a release or publication
approval.
