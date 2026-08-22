# Known limitations

Installed-Windows manual validation passed for the v0.1 package on 2026-07-30. The release-level
approval covers the implemented v0.1 scope, so historical Phases 1–5 are DONE at that evidence
level; earlier phase-specific notes remain historical preparation evidence and are not repeated as
new observations here. AIP v0.1.0 was published from commit `b6f74b3793437a647186dd52eeb950ff4b3fb228`.
A local Ollama integration test is skipped when its required model is not installed. v0.1.0 has
no cloud synchronization, Android client, BielOS integration, released voice feature,
real supervised external tools, released screen vision, or the post-release Phase 7E–7F
cognitive-core conversation integration. The current development checkpoint implements
bounded Phase 7B–7E local paths, the Phase 8 local voice architecture checkpoint, and the
Phase 9 supervised-tools mock checkpoint plus the functional Phase 10 closed declarative
extension runtime and the Phase 11 synthetic metadata-only screen-vision checkpoint; these are not
v0.1 release claims. Phase 7E and 7F
validation reservations are recorded in
[PHASE_7E_VALIDATION.md](PHASE_7E_VALIDATION.md) and [PHASE_7F_VALIDATION.md](PHASE_7F_VALIDATION.md).

## Distribution and platform

- v0.1.0 installers are unsigned.
- v0.1.0 targets Windows 10 and Windows 11 on x64 hardware.

## Ollama startup

- AIP does not automatically start Ollama; the user must start the Ollama application/service or run an Ollama command before using a local model.
- Automatic Ollama detection and explicit start controls are deferred to a future UX phase.

## Phase 8 voice checkpoint

- Phase 8 currently exposes a local Rust/SQLite architecture, Tauri commands, versioned
  contracts, Portuguese controls, and metadata-only fixtures. It does not capture from a
  microphone, persist raw audio, upload data, clone a real person, or run a hidden listener;
  text conversation remains the fallback.
- Real audio-device/model integration, packaged Windows validation, restart/manual runtime
  evidence, and subjective voice-quality validation remain reserved. The checkpoint is not
  release approval. See [PHASE_8_VOICE_SPEC.md](PHASE_8_VOICE_SPEC.md).

## Phase 9 supervised-tools checkpoint

- Phase 9 currently provides only SQLite-backed manifests, fixture-scoped sessions, typed
  preview/approval/confirmation/cancellation/compensation paths, deterministic mock output,
  audit retention, and Portuguese Owner controls. It does not access the host filesystem,
  shell, credentials, calendars, messaging accounts, network, or external providers.
- Safe mode and temporary chat fail closed for tool mutations. Read-only catalog and audit
  inspection remain available for recovery visibility; no hidden execution or permission
  expansion is supported.
- Live provider adapters, real external-effect verification, packaged-Windows runtime
  evidence, and release/manual approval remain reserved. See
  [PHASE_9_TOOLS_SPEC.md](PHASE_9_TOOLS_SPEC.md).

## Phase 10 bounded extensions

- Phase 10 provides SQLite-backed private metadata manifests plus closed
  declarative package execution. Automated validation is complete; extension
  usability and human package review remain pending. Packages never access
  native code, files, network, shell, subprocesses, credentials, or arbitrary
  expressions.
  Local-fixture admission, review-only agent proposals, explicit Owner review
  and activation, re-reviewing updates, rollback, disable, bounded audit
  retention, and Portuguese controls remain available.
- Safe mode and temporary chat fail closed for all durable extension
  mutations. Read-only catalog, proposal, and audit inspection remains
  available. Unsupported SDK and recovery lifecycle values remain metadata-only
  checkpoint states. Manual Windows evidence, extension usability, package
  review, and release approval remain reserved. See
  [PHASE_10_EXTENSIONS_SPEC.md](PHASE_10_EXTENSIONS_SPEC.md).

## Phase 11 screen-vision checkpoint

- Phase 11 provides bounded on-demand Windows capture plus synthetic monitor
  fixtures; raw frames remain transient and the local visual adapter reports
  unavailable when no model is configured.
  Display metadata is capped at 18 records total (two fixtures plus 16 real
  monitors).
  preview/confirmation, per-session permissions, privacy/redaction hooks,
  reference-GPU scheduling, bounded quotas, cancellation, cleanup, audit, and
  Portuguese Owner controls. It does not call a Windows screenshot API, capture
  a desktop/window, create or retain pixels, or persist screenshot bytes.
- No continuous or background capture exists. The checkpoint does not access
  the host filesystem, shell, credentials, browser/accounts, network, remote
  model, or cloud provider. The visual hypothesis is always uncertain,
  non-diagnostic, non-sensitive-attribute, bounded, and non-durable.
- Rust/SQLite owns Owner validation, permissions, safe-mode and temporary-chat
  gates, lifecycle, quotas, and cleanup; the React controls cannot bypass them.
  Read-only Screen Vision history/audit can remain visible for recovery, but
  durable mutations fail closed in safe mode and temporary chat.
- Real screen adapters, real visual models, Windows packaging/manual evidence,
  privacy and visual-UX acceptance, performance under real workloads, and
  release approval remain reserved. See
  [PHASE_11_SCREEN_VISION_SPEC.md](PHASE_11_SCREEN_VISION_SPEC.md).

## Phase 12 Android companion checkpoint

- Phase 12 is a local metadata-only protocol checkpoint. It includes a
  synthetic Android fixture, Rust/SQLite pairing and session state, nonce/replay
  checks, compatibility negotiation, rotation/revocation, bounded read-only
  history, metadata-only outgoing queue, explicit Owner approval, audit, and
  Portuguese controls. It is not an APK or a mobile release.
- No listener, relay, tunnel, network, Android account, real credential/key
  material, host filesystem, shell, Python internals, or media bytes are used.
  Audio/image/file queue entries retain only bounded descriptive metadata and
  always report `mediaBytesPersisted: false`.
- Temporary chat and safe mode fail closed for pairing/session/queue/rotation/
  revocation mutations while history and audit remain read-only. React cannot
  bypass the Rust authority. Transport cryptography, Android lifecycle and
  permission UX, notifications, real voice/media, packaged-device tests,
  recovery, and release approval remain reserved. See
  [PHASE_12_ANDROID_SPEC.md](PHASE_12_ANDROID_SPEC.md).

## Phase 13 gateway checkpoint

- Phase 13 is a local metadata-only gateway checkpoint. It includes a
  synthetic protocol/account fixture, Owner-scoped transfer and approval,
  session proof/replay checks, administrative recovery approval, revocation,
  bounded audit, and Portuguese desktop controls. It is not BielOS
  integration or a remote-access release.
- Cloudflare Tunnel/Access values are metadata only. No listener, relay,
  tunnel, network, external account, credential, `.env`, host filesystem,
  shell, or Python-runtime path is used, and no external effect is performed.
- Rust/SQLite owns ownership, authentication, replay, lifecycle, approval,
  revocation, idempotency, safe mode, and temporary-chat gates. Read-only
  protocol/state/audit visibility can remain available while mutations fail
  closed; React cannot bypass those controls.
- Real BielOS ownership exchange, transfer of a real agent, transport
  cryptography, Cloudflare credentials, remote/mobile recovery, packaged
  gateway validation, and release approval remain reserved. See
  [PHASE_13_GATEWAY_SPEC.md](PHASE_13_GATEWAY_SPEC.md).

## Deferred usability and agent features

- Phase 7A is approved/DONE for commit `3e591a06129a9d8f27e026490f9bd83028eb2465`. The exact
  automated evidence and the Owner's current manual attestation are recorded in
  [PHASE_7A_VALIDATION.md](PHASE_7A_VALIDATION.md). It provides bounded deterministic trait
  events, explicit owner correction, inspection, latest-event compensating rollback, and typed
  source-eligibility validation. Persisted conversation sources are validated only for a future
  adapter; no model extraction is connected. Phase 7B–7D opinion, relationship, and
  fictional-goal controls and the bounded public Phase 7E conversation path are present in
  the current development checkpoint. Phase 7F records validation/documentation status;
  Windows runtime, focused test execution, manual evidence, and remote CI remain reserved.
  Later phases remain pending.

- The v0.1 installed-Windows approval is release-level evidence for historical Phases 1–5, not a
  reconstruction of missing phase-by-phase observations. Those phases are not current release
  blockers, while their earlier validation notes remain useful historical evidence.

- General, Owner profile, Agents, and Models settings still need their own focused functional UX pass. Safe mode and diagnostics are the currently implemented settings controls; backup/export remains unavailable.
- The default controls and conversation management layout need a cohesive visual-design pass. This is intentionally separate from generation reliability work.
- The current visual design is functional but unattractive; a dedicated visual redesign is deferred.
- Simulated energy, mood, and sleep currently have limited visible effects and require further manual validation.
- The pixel editor remains layer-based. A future semantic sprite system should define reusable head, torso, arms, hands, legs, feet, hair, clothing, accessories, attachment joints, and safe animation poses without changing a user-created identity.
- Automatic memory candidates remain subject to manual validation before broader learning behavior is expanded; low-value and temporary content must not be consolidated automatically.
