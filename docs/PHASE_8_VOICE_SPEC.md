# Phase 8 voice specification

## Status and scope

Phase 8 is implemented as a local architecture checkpoint. The checkpoint proves the
authoritative state model, guarded Tauri boundaries, versioned contracts, Portuguese UI
controls, metadata-only fixtures, and text fallback. It is implementation-ready for a
future local audio adapter, but it is not a claim that real audio devices or speech models
are integrated.

The implementation remains standalone, local-first, Owner-scoped, and limited to the two
seeded agents. It does not add remote access, cloud processing, autonomous conversations,
voice uploads, or Phase 9 tools.

## Authoritative design

Rust and SQLite own voice state. Migration `0015_phase8_voice.sql` creates:

- `agent_voice_settings`, one Owner-scoped row per seeded agent, with schema version `1`,
  immutable base voice `aip-base-v1`, optional local model/device references, consent state,
  and timestamps;
- `voice_mutation_events`, bounded idempotency records for settings and custom-voice
  consent changes.

Database mapping validates the schema version and base voice against the Rust constants before
returning a settings object. SQLite checks provide the durable constraint as well. The base
voice is protected; a custom reference can only be selected through explicit consent and the
checkpoint accepts only synthetic `fixture:custom-*` consent references.

The registered Tauri commands are:

- `get_voice_settings`;
- `update_voice_settings`;
- `set_custom_voice_consent`;
- `transcribe_voice_fixture`;
- `synthesize_voice_fixture`;
- `detect_voice_wake_word_fixture`;
- `classify_voice_emotion`.

The TypeScript contracts parse settings and all fixture results at the UI boundary. The
Portuguese `VoiceControls` panel exposes local references, consent, fixture checks, and
degraded-state explanations without making text chat depend on voice availability.

## Safety and privacy invariants

- Temporary chat is read-only for durable voice settings and consent. The Rust command
  boundary checks both the request flag and active temporary-chat state.
- Silent and suspended agents fail closed for voice settings/consent mutations. Wake-word
  handling returns ignored/degraded metadata and never activates a listener. Voice-muted
  synthesis returns a muted result with text fallback.
- No microphone capture, raw audio persistence, audio table, upload, network call, telemetry
  path, or hidden listener exists in this checkpoint.
- Fixture and local references are bounded and character-validated. No real-person voice
  cloning path is implemented; custom consent is synthetic-fixture-only.
- Emotion classification is a bounded text heuristic presented as uncertain and
  non-diagnostic. It is never a fact, diagnosis, or identity claim.
- Fixture results explicitly report `metadataOnly: true`, `rawAudioPersisted: false`, and
  text fallback where voice is unavailable or muted.

## Local behavior matrix

| Path          | Implemented checkpoint behavior                                                       | Not yet evidenced                                         |
| ------------- | ------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| Transcription | Bounded fixtures return text/confidence or a degraded code.                           | Real microphone/device capture and speech model accuracy. |
| Synthesis     | Bounded text returns metadata-only duration/voice reference or degraded/muted status. | Real output device playback and subjective quality.       |
| Wake word     | Fixture detection is explicit; `listenerActive` is always false.                      | Any background listener or hardware integration.          |
| Custom voice  | Synthetic fixture reference with explicit durable consent and revocation.             | Real voice assets, cloning, or personal voice samples.    |
| Emotion       | Uncertain, non-diagnostic text hypothesis.                                            | Audio emotion inference or clinical interpretation.       |

## Validation boundary

The implementation is validated by source review, Rust formatting/checks, Clippy, TypeScript,
focused ESLint, focused docs/contracts formatting, whitespace checks, and the repository
secrets scan when those commands are available. Focused Rust/Vitest execution may remain
blocked by the Windows test-binary or pnpm/Vite environment; a blocked run is not reported as
passed.

Reserved evidence before calling Phase 8 a release-ready voice feature:

- real local audio-device and speech-model integration on supported Windows hardware;
- packaged Windows startup, restart, degraded-mode, temporary, silent, muted, and suspended
  manual checks;
- objective device/model error recovery and persistence checks in the packaged application;
- subjective Owner review of intelligibility, latency, voice consistency, and comfort;
- review that no raw audio, upload, hidden listener, or real-person cloning path was added.

This checkpoint does not authorize or implement Phase 9 supervised tools.
