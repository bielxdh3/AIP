# Phase 8 voice specification

## Status and scope

Phase 8 includes a bounded on-demand local Windows runtime: native input/output device
enumeration and Owner selection, waveIn/waveOut capture/playback, replaceable local
provider-neutral STT/TTS/wake-word argv paths, and guarded Portuguese Tauri controls.
Supported-device/provider availability, packaged/manual hardware checks, quality, and human
Owner validation remain open.

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
voice is protected; a custom reference can only be selected through explicit consent. The safe
path carries bounded fixture or local custom references to the existing local TTS provider and
never imports or persists raw personal samples.

The registered Tauri commands are:

- `get_voice_settings`;
- `update_voice_settings`;
- `set_custom_voice_consent`;
- `transcribe_voice_fixture`;
- `synthesize_voice_fixture`;
- `detect_voice_wake_word_fixture`;
- `transcribe_voice_local`;
- `synthesize_voice_local`;
- `detect_voice_wake_word_local`;
- `cancel_voice_operation`;
- `get_voice_operation_status`;
- `list_voice_devices`;
- `classify_voice_emotion`.

The TypeScript contracts preserve fixture parsers and add explicit runtime request/result/status
types. The Portuguese `VoiceControls` panel exposes clearly labeled fixture checks and
on-demand local actions without making text chat depend on voice availability.
Runtime examples use `local:wavein:0`, `local:waveout:0`, `local:stt:provider`, and
`local:tts:provider`. Native enumeration returns bounded stable device records when Windows
hardware is available; missing hardware/providers remain explicit degraded states.

## Safety and privacy invariants

- Temporary chat is read-only for durable voice settings and consent. The Rust command
  boundary checks both the request flag and active temporary-chat state.
- Silent and suspended agents fail closed for voice settings/consent mutations. Wake-word
  handling returns ignored/degraded metadata and never activates a listener. Voice-muted
  synthesis returns a muted result with text fallback.
- On-demand capture/playback uses bounded in-memory Windows wave adapters and a local,
  provider-neutral argv path only; there is no background listener. Device loss, cancellation,
  timeout, and model absence degrade safely. Raw audio persistence,
  upload, network calls, and telemetry do not exist.
- Fixture and local references are bounded and character-validated. No real-person voice
  cloning path is implemented; custom consent never imports or persists raw voice samples.
- Emotion classification is a bounded text heuristic presented as uncertain and
  non-diagnostic. It is never a fact, diagnosis, or identity claim.
- Fixture results explicitly report `metadataOnly: true`, `rawAudioPersisted: false`, and
  text fallback where voice is unavailable or muted.

## Local behavior matrix

| Path          | Implemented checkpoint behavior                                                       | Not yet evidenced                                         |
| ------------- | ------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| Transcription | On-demand bounded local Windows capture invokes a configured provider; unavailable devices/models degrade. | Supported availability, packaged checks, and accuracy. |
| Synthesis     | On-demand bounded local Windows playback invokes a configured provider; unavailable devices/models degrade. | Supported availability, packaged checks, and quality. |
| Wake word     | On-demand bounded local check invokes a configured provider; `listenerActive` remains false. | Provider/device behavior and Owner validation. |
| Custom voice  | Bounded fixture/local reference with explicit durable consent and revocation; no sample import/persistence. | Real voice assets, cloning, or personal voice samples.    |
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
