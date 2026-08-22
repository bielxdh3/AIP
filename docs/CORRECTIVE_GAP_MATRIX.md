# Corrective roadmap gap matrix

Architect evidence for the corrective completion mission. This matrix separates the
previous metadata/checkpoint implementation from the real functional behavior still
safe to add before the final human gate.

| Phase | Original roadmap deliverable | Previous full-roadmap / PR #6 claim | Current real behavior | Missing engineering | Automated evidence currently available | Genuine human-only validation | Classification |
|---|---|---|---|---|---|---|---|
| 0–6 | Windows desktop shell, conversation, agents, memory, modes, editor, packaging | v0.1 baseline released | Installed-Windows validation record exists for the v0.1 scope | No corrective work identified in this mission | Historical CI and repository validation records | Subjective release smoke already recorded; future regression review remains separate | APPROVED |
| 7A | Cognitive event foundation and protected boundaries | Approved foundation | Bounded Rust/SQLite event path with Owner correction and rollback | Later extraction/adapters remain separate | Phase 7A validation record and tests | Product/UX review of later cognition | APPROVED |
| 7B–7F | Opinions, relationships, goals, public conversation, validation/UX | Implemented local paths/checkpoint | Bounded local persistence and controls; no claim of complete runtime integration | Runtime and broader product validation remain | Focused Rust/TS/contract checks | Manual UX/runtime review | FOUNDATION ONLY |
| 8 | Local audio input, STT, TTS, wake word, custom voice | Bounded on-demand local Windows adapter and provider-neutral argv path | Rust/SQLite metadata, fixture adapters, guarded runtime commands, controls, bounded in-memory capture/playback, and text fallback; no cloud/upload/telemetry/background listener | Supported device/provider availability, packaged/manual hardware checks, quality, and human Owner validation | Existing migration/module/contract/UI/runtime tests | Mic/speaker quality, wake-word behavior, voice naturalness | FOUNDATION ONLY |
| 9 | Safe supervised local filesystem and provider-neutral tools | Functional local workspace checkpoint | Opaque configured roots; canonical containment, link/system-root rejection; metadata-only inspection; bounded move preview/approval/second-confirmation/dry-run/rollback; provider-neutral calendar/messaging fixtures | No delete/overwrite/network/shell/telemetry/watcher | Automated Rust/contract/UI gate evidence | Human Windows root/confirmation UX and live provider validation remain open | FUNCTIONAL — HUMAN VALIDATION PENDING |
| 10 | Capability-bounded extension execution | Metadata-only checkpoint | Closed declarative package builder, SHA-256 integrity, Owner-reviewed active execution, bounded host capabilities, budgets, cancellation, audit, and typed UI/contracts | Extension usability and package review | Rust, contract, and desktop validation | Extension usability and package review | FUNCTIONAL — AUTOMATED VALIDATION COMPLETE |
| 11 | Explicit Windows capture and local visual adapter | Bounded automated capture/provider checkpoint | In-memory capture, cleanup, quotas and unavailable adapter degradation | Packaged/manual Windows UX, local model installation and visual quality | Capture/provider and synthetic lifecycle tests | Privacy/visual quality and supported-device behavior | FUNCTIONAL — AUTOMATED CHECKPOINT |
| 12 | Android project, reproducible APK, pairing, authenticated desktop transport, chat/history/queue/notifications | Metadata-only companion checkpoint; no APK/transport | Functional APK with authenticated local/private explicit-connect client and deterministic loopback | Physical-device/private-LAN validation, release signing | Android JVM tests, lint, assembleDebug, loopback contract tests | Physical-device UX and permission behavior | FUNCTIONAL — RELEASE/HARDWARE CHECKS |
| 13 | Secure AIP-side gateway transport | Metadata-only gateway checkpoint; no listener/socket | SQLite accounts/transfers/sessions/recovery/revocation metadata only | Real authenticated local/private socket, version negotiation, replay/idempotency, limits, audit, loopback client | Existing Rust/contract/UI fixture tests | Optional live Cloudflare/BielOS integration | MISSING |

## Corrective acceptance order

1. Fix and independently review the Windows Rust/Tauri test startup failure before
   merging PR #6.
2. Continue from updated main in separate coherent PRs: functional voice,
   supervised tools, sandboxed extensions, real screen vision, Android plus the
   required desktop transport, and the AIP gateway.
3. Keep BielOS-side implementation and optional live Cloudflare/provider credentials
   as EXTERNAL_DEPENDENCY; they must not be pulled into standalone AIP.
4. Do not change a classification to functional until the primary effect executes
   and has deterministic automated coverage. Human quality checks remain separate.
