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
Phase 9 supervised-tools mock checkpoint plus the Phase 10 metadata-only extension
checkpoint and the Phase 11 synthetic metadata-only screen-vision checkpoint; these are not
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

## Phase 10 extensions checkpoint

- Phase 10 currently provides only SQLite-backed, private metadata manifests,
  local-fixture admission, review-only agent proposals, explicit Owner review
  and activation, re-reviewing updates, rollback, disable, bounded audit
  retention, and Portuguese controls. It does not load, compile, interpret, or
  execute extension code.
- Network fetch, shell, host-filesystem access, credentials, remote code
  execution, public marketplace behavior, hidden execution, package
  integrity/signature verification, ratings, and real plugin behavior are not
  implemented.
- Safe mode and temporary chat fail closed for all durable extension
  mutations. Read-only catalog, proposal, and audit inspection remains
  available. Unsupported SDK and recovery lifecycle values are metadata-only
  checkpoint states; no package recovery or execution is attempted. Runtime/
  package validation, manual Windows evidence, and release approval remain
  reserved. See
  [PHASE_10_EXTENSIONS_SPEC.md](PHASE_10_EXTENSIONS_SPEC.md).

## Phase 11 screen-vision checkpoint

- Phase 11 currently provides only synthetic monitor fixtures, metadata-only
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
