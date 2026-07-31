# Known limitations

Installed-Windows manual validation passed for the v0.1 package on 2026-07-30. A local Ollama integration test is skipped when its required model is not installed. No cloud synchronization, Android client, BielOS integration, voice, external tools, or public release is included.

## Ollama startup

- AIP does not automatically start Ollama; the user must start the Ollama application/service or run an Ollama command before using a local model.
- Automatic Ollama detection and explicit start controls are deferred to a future UX phase.

## Deferred usability and agent features

- General, Owner profile, Agents, and Models settings still need their own focused functional UX pass. Safe mode and diagnostics are the currently implemented settings controls; backup/export remains unavailable.
- The default controls and conversation management layout need a cohesive visual-design pass. This is intentionally separate from generation reliability work.
- The current visual design is functional but unattractive; a dedicated visual redesign is deferred.
- Simulated energy, mood, and sleep currently have limited visible effects and require further manual validation.
- The pixel editor remains layer-based. A future semantic sprite system should define reusable head, torso, arms, hands, legs, feet, hair, clothing, accessories, attachment joints, and safe animation poses without changing a user-created identity.
- Automatic memory candidates remain subject to manual validation before broader learning behavior is expanded; low-value and temporary content must not be consolidated automatically.
