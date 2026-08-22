# Corrective roadmap gap matrix

Architect evidence for the corrective completion mission. This matrix separates the
previous metadata/checkpoint implementation from the real functional behavior still
safe to add before the final human gate.

| Phase | Original roadmap deliverable | Previous full-roadmap / PR #6 claim | Current real behavior | Missing engineering | Automated evidence currently available | Genuine human-only validation | Classification |
|---|---|---|---|---|---|---|---|
| 0–6 | Windows desktop shell, conversation, agents, memory, modes, editor, packaging | v0.1 baseline released | Installed-Windows validation record exists for the v0.1 scope | No corrective work identified in this mission | Historical CI and repository validation records | Subjective release smoke already recorded; future regression review remains separate | APPROVED |
| 7A | Cognitive event foundation and protected boundaries | Approved foundation | Bounded Rust/SQLite event path with Owner correction and rollback | Later extraction/adapters remain separate | Phase 7A validation record and tests | Product/UX review of later cognition | APPROVED |
| 7B–7F | Opinions, relationships, goals, public conversation, validation/UX | Implemented local paths/checkpoint | Bounded local persistence and controls; no claim of complete runtime integration | Runtime and broader product validation remain | Focused Rust/TS/contract checks | Manual UX/runtime review | FOUNDATION ONLY |
| 8 | Local audio input, STT, TTS, wake word, custom voice | Bounded on-demand local Windows adapter and provider-neutral argv path | Native bounded device enumeration/Owner selection, in-memory capture/playback, replaceable local STT/TTS/wake adapters, guarded runtime commands, and text fallback; no cloud/upload/telemetry/background listener | Supported device/provider availability, packaged/manual hardware checks, quality, and human Owner validation | Rust, contract, and desktop voice/runtime tests | Mic/speaker quality, wake-word behavior, voice naturalness | FUNCTIONAL — HUMAN VALIDATION PENDING |
| 9 | Safe supervised local filesystem and provider-neutral tools | Functional local workspace checkpoint | Opaque configured roots; canonical containment, link/system-root rejection; real metadata inspection and approved bounded moves with preview/approval/second-confirmation/dry-run/audit/rollback; provider-neutral calendar/messaging fixtures | No delete/overwrite/network/shell/telemetry/watcher | Automated Rust/contract/UI gate evidence plus real OS-effect tests | Human Windows root/confirmation UX and live provider validation remain open | FUNCTIONAL — HUMAN VALIDATION PENDING |
| 10 | Capability-bounded extension execution | Closed declarative sandbox checkpoint | Versioned package builder, SHA-256 integrity, Owner-reviewed active execution through the bounded Rust VM/host contract, capabilities, budgets, cancellation, idempotency, update reapproval, rollback, disable, audit, and typed UI/contracts | Extension usability and package review | Rust execution/lifecycle tests, contract, and desktop validation | Extension usability and package review | FUNCTIONAL — AUTOMATED VALIDATION COMPLETE |
| 11 | Explicit Windows capture and local visual adapter | Bounded automated capture/provider checkpoint | Real confirmed Windows GDI capture, bounded transient cleanup, quotas/resource serialization, replaceable local `aip-screen-vision-v1` executable adapter with unavailable degradation, and deterministic fake coverage | Packaged/manual Windows UX, local model installation and visual quality | Capture/provider, parser, lifecycle, and synthetic tests | Privacy/visual quality and supported-device behavior | FUNCTIONAL — HUMAN VALIDATION PENDING |
| 12 | Android project, reproducible APK, pairing, authenticated desktop transport, chat/history/queue/notifications | Metadata-only companion checkpoint; no APK/transport | Functional APK with authenticated local/private explicit-connect client and deterministic loopback | Physical-device/private-LAN validation, release signing | Android JVM tests, lint, assembleDebug, loopback contract tests | Physical-device UX and permission behavior | FUNCTIONAL — HUMAN VALIDATION PENDING |
| 13 | Secure AIP-side gateway transport | Functional authenticated local/private `aip-gateway-v1` framed HMAC TCP transport with Rust/SQLite authority | SQLite accounts/transfers/sessions/recovery/revocation plus signed bounded responses | Private-LAN/hardware/manual validation, remote CI, Cloudflare/BielOS integration | Rust loopback authority lifecycle tests and desktop UI transport lifecycle tests | Cloudflare/BielOS credentials and external ownership | FUNCTIONAL — HUMAN VALIDATION PENDING |

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
