# Known limitations

Installed-Windows manual validation passed for the v0.1 package on 2026-07-30. The release-level
approval covers the implemented v0.1 scope, so historical Phases 1–5 are DONE at that evidence
level; earlier phase-specific notes remain historical preparation evidence and are not repeated as
new observations here. AIP v0.1.0 was published from commit `b6f74b3793437a647186dd52eeb950ff4b3fb228`.
A local Ollama integration test is skipped when its required model is not installed. v0.1.0 has
no cloud synchronization, Android client, BielOS integration, released voice feature,
supervised external tools, extensions, screen vision, or the post-release Phase 7E–7F
cognitive-core conversation integration. The current development checkpoint implements
bounded Phase 7B–7E local paths and the Phase 8 local voice architecture checkpoint; this is
not a v0.1 release claim. Phase 7E and 7F validation reservations are recorded in
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
