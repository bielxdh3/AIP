# Phase 7F validation

## Current closure evidence (2026-08-24)

Status: FUNCTIONAL — HUMAN VALIDATION PENDING. Automated validation is complete on
the current host: Rust format/check, 10 focused cognitive tests, desktop typecheck,
contracts typecheck, and desktop Vitest (14 files, 51 tests) pass; `git diff --check`
passes. The closure includes fail-closed memory/conversation source validation,
transactional memory invalidation with opinion/relationship projection recomputation,
deterministic goal/activity expiry, six-dimension relationship visibility, public
conversation budgets/termination/consent checks, and Owner-visible fictional activity
lifecycle controls. Installed-Windows interaction, packaged restart, and subjective
Portuguese review remain the genuine human-only gate. Earlier executor spawn/entrypoint
reservations below are retained as historical evidence.

## Historical correction evidence (retained)

## Current automated result

Status: FUNCTIONAL — AUTOMATED VALIDATION COMPLETE. Opinion provenance, all six
relationship dimensions, goal origin/schedule/evidence, and Owner-visible fictional
activity lifecycle controls are exposed. Installed-Windows interaction and
subjective Portuguese review remain HUMAN VALIDATION PENDING.

## Status

Phase 7F is the integrated validation and documentation checkpoint for the existing
7B–7E work. This document records the integrated 7B–7F correction state without
adding a later-phase protocol, channel, or external-action path. It records what is
locally evidenced and what remains reserved.

The exact implementation target is the resulting `HEAD` of this single correction
commit (parent `0d4aff03b1d7eac0564ffec3dbf5bfc7a36f351c`). The symbolic `HEAD`
reference resolves to the exact commit containing this record. Its scope is the
7B–7D history through
`821d8f9d468e50aedb4ec7b3d42dabcf8162d157`, followed by the Phase 7E commits
`4de625841d6b87ad8821cdc98e947877eb61dbe7`,
`70ba8fe88049c530cdd53c9585bcef3ede408a21`, and
`5ff4807985eb8156f2c59b7644700c43d19c58fc`, the integrated correction at parent
`0d4aff03b1d7eac0564ffec3dbf5bfc7a36f351c`, and this focused blocker correction.
The relevant migration sequence is
`0012_phase7a_cognitive_events.sql` (7A), `0013_phase7b_7d_cognitive_core.sql`
(7B–7D), and `0014_phase7e_7f_conversations.sql` (schema version 14). The companion
record is [Phase 7E validation](PHASE_7E_VALIDATION.md).

## Integrated boundary matrix

| Area                            | Local implementation evidence                                                                                                                                                                                | Validation status                                                                                  |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------- |
| 7B opinions                     | Rust/SQLite opinion, evidence, correction, dispute, supersession, fail-closed source lifecycle, and deterministic invalidation/recalculation paths                                                                 | FUNCTIONAL — AUTOMATED VALIDATION COMPLETE; installed-Windows and subjective Portuguese review pending |
| 7C relationships                | Rust/SQLite bounded six-dimension relationship state, isolation, reset, rollback, and deterministic source-invalidation replay                                                                                       | FUNCTIONAL — AUTOMATED VALIDATION COMPLETE; installed-Windows and subjective Portuguese review pending |
| 7D goals and fictional activity | Owner approval, origin/schedule/evidence, budgets, fictional-only state, and visible activity start/pause/resume/complete/expire/archive controls | FUNCTIONAL — AUTOMATED VALIDATION COMPLETE |
| 7E public conversations         | Typed policy/start/turn/resource/candidate/list/inspect/interrupt/reject paths in Rust, contracts, and the Portuguese panel, with termination reason and consent/revocation boundaries | FUNCTIONAL — AUTOMATED VALIDATION COMPLETE; packaged/manual restart and Owner smoke pending |
| Persistence and recovery        | Migration 0014, SQLite transactions, reopen persistence, and recovery tests are present                                                                                                                      | FUNCTIONAL — AUTOMATED VALIDATION COMPLETE locally; packaged restart remains HUMAN VALIDATION PENDING |
| Security boundary               | Owner/participant/opt-in/revocation/mode/temporary/budget guards; public-only content screening; pending candidate attribution; explicit temporary-chat fields on durable 7A–7D commands                     | Reviewed in source; no hidden/private channel or direct model durable mutation is exposed          |

## Historical validation evidence (retained)

The following older evidence is retained to document the prior executor environment;
the current-host results are recorded above and supersede its blocked-test claims.

## Required validation evidence

The following commands were run against the implementation target or its unchanged
working tree:

- Rust format check and `cargo check --locked` passed.
- Desktop and contracts TypeScript checks passed.
- Focused ESLint passed. Contracts-only Prettier passed.
- Full focused Prettier reported the existing App.tsx and cognitive-panel test
  differences; this checkpoint does not rewrite those product files.
- Focused Rust tests were blocked when Windows could not launch the test binary with
  `STATUS_ENTRYPOINT_NOT_FOUND` (`0xc0000139`).
- Focused Vitest was blocked before test execution by pnpm/Vite `spawn EPERM`.
  The serial/config=false fallback was also unavailable because the installed Vitest
  treated `false` as a config path.
- `pnpm secrets:scan` was attempted and was blocked because its child `git` process
  returned `spawnSync git EPERM`. A read-only tracked-file pattern check found no
  sensitive filename, and the pattern check was not treated as a replacement for the
  official scan.
- `git diff --check` passed before this documentation-only change; it is rerun for the
  final commit.

The correction also makes policy revocation live for existing conversations, maps the
Portuguese panel to the seeded IDs `agt_astra_provisional` and
`agt_luma_provisional`, and closes the Tauri temporary-chat boundary for all durable
7A–7D writes. The fictional-activity backend path is corrected and remains
fictional-only; the panel exposes its typed lifecycle controls.
These results distinguish compilation/static checks from test execution. Existing
source tests cover restart persistence, owner and participant isolation, temporary and
mode guards, public-only content, candidate non-application, heavy-job uniqueness,
bounded termination, and recovery. Their presence is not reported as a passing runtime
result while the test launch reservations remain.

## Manual, runtime, and remote boundaries

Not observed in this checkpoint:

- installed-Windows UI interaction in temporary, safe, silent, and suspended modes;
- an Owner session that opts in both agents, starts/appends/inspects/interrupts a public
  conversation, reserves/completes one bounded heavy job, and rejects a pending
  candidate;
- database restart/reopen behavior exercised in the packaged application;
- live model/runtime recovery behavior; and
- remote CI or a draft-publication review.

The next validation environment must clear the Windows entrypoint and pnpm process
reservations, then record actual focused test and manual results. No result here is a
release approval or a claim that remote CI passed.

## Scope guard

7B–7E are described as implemented local paths only. Phase 7F is validation and
documentation status, not a new conversation or autonomy feature. Voice, supervised
tools, extensions, screen vision, Android, BielOS integration, remote access, telemetry,
and other later phases remain pending.
