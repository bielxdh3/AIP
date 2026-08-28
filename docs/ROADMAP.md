# AIP Roadmap

## Historical v0.1 phase status

The old Phase 1–5 pending labels described pre-release preparation states. The later
installed-Windows approval recorded in [the v0.1 manual validation record](V0_1_MANUAL_VALIDATION.md)
on 2026-07-30 for packaged SHA `6b5dc1a0a18d3e346d04c6bd89de13775c681434` (CI run
`30474813207`) is the release-level human gate for the v0.1 scope. That record names onboarding,
profile edits, conversation switching, temporary chat, memory workflows, state/mode behavior,
pixel editing, overlays, display scaling, multi-monitor recovery, packaged startup, updates, and
local Ollama interaction. Its recorded result is that the package opened, remained usable, and
closed and reopened successfully; no unrecorded per-step observations are reconstructed here.

Within that release-level evidence boundary, Phases 1–5 are DONE. Earlier phase-specific
validation notes remain historical evidence of preparation and earlier attempts; they are not
new claims about the packaged result or blockers for the post-v0.1 roadmap.

## Roadmap principles

- Build AIP standalone before BielOS integration.
- Keep each phase small enough to review and validate.
- Do not claim a phase is complete without evidence.
- Preserve Windows 10 64-bit as the minimum target.
- Keep the UI functional when AI components are missing.
- Avoid implementing later features merely because an abstraction could support them.
- Stabilize contracts before Android or remote access.

## Phase 0: repository bootstrap and visual shell `[DONE]`

Goal: produce the first executable Windows shell with two provisional agents and resilient runtime boundaries.

Implementation state: approved and complete. The preserved first click-through hotfix failed
manual Windows validation; runtime commit
`a6ccb1badf6aa8a1f317ea1818c247d87f311fe6` passed the corrected manual Windows 11
test at 100% display scaling and its exact-SHA GitHub Actions run.

Deliverables:

- pnpm monorepo;
- Tauri + Rust application;
- React + TypeScript interface;
- Python runtime skeleton;
- managed stdio health handshake;
- shared versioned contracts;
- SQLite migration foundation;
- minimal main panel;
- two provisional 64x64 agent overlays;
- transparent click-through proof;
- drag behavior;
- always-on-top behavior;
- full-screen hiding proof where feasible;
- safe-mode startup;
- runtime unavailable state;
- initial tests and Windows setup documentation.

Excluded:

- Ollama chat;
- real memory;
- full pixel editor;
- autonomous behavior;
- BielOS.

Expected commit: `chore: bootstrap AIP desktop workspace`

## Phase 1: local conversation vertical slice `[DONE — V0.1 INSTALLED-WINDOWS VALIDATION]`

Goal: prove one complete model conversation path without coupling agent identity to the model.

Deliverables:

- Ollama detection;
- installed model discovery;
- default and unavailable statuses;
- one serialized generation queue;
- streaming response;
- persistent messages;
- compact and expanded speech bubble;
- cancel generation;
- 15-minute configurable keep-alive;
- degraded behavior when Ollama or model disappears.

Expected commit: `feat: add local conversation vertical slice`

Implementation and release-level manual validation are recorded in the v0.1 manual validation
record above. The separate Phase 1 validation document explicitly labels its failed and pending
pre-release attempts as historical and records their supersession by the later installed-package
approval; it does not replace the later installed-package approval.

## Phase 2: two-agent creation and isolation `[DONE — V0.1 INSTALLED-WINDOWS VALIDATION]`

Goal: create and persist two independent agents under the implicit Owner.

Deliverables:

- first-run Owner initialization;
- two-agent creation flow;
- identity, birthday, age, species, pronouns, traits, and appearance;
- default model per agent;
- model override per conversation;
- main chat per agent;
- isolation tests for chats, settings, and model selection;
- position persistence.

Expected commit: `feat: add two-agent creation and isolation`

## Phase 3: memory and temporary chat `[DONE — V0.1 INSTALLED-WINDOWS VALIDATION]`

Goal: add initial learning without deep personality evolution.

Deliverables:

- manual memories;
- automatic memory candidates;
- categories and confidence;
- source references;
- conflict representation;
- recent summaries;
- searchable history;
- temporary chat held only in RAM;
- tests proving temporary content is not persisted.

Expected commit: `feat: add initial memory and temporary chat`

## Phase 4: states and modes `[DONE — V0.1 INSTALLED-WINDOWS VALIDATION]`

Goal: introduce deterministic fictional state and robust application modes.

Deliverables:

- sleep, energy, and mood;
- deterministic state transitions;
- wake-now control;
- offline elapsed-time application;
- normal, voice-muted, silent, and safe modes;
- suspension semantics;
- queue priority and basic resource settings;
- clear UI status.

Local scheduling is intentionally deferred because calendar integration is outside v0.1.

Expected commit: `feat: add agent states and application modes`

## Phase 5: pixel editor and overlay behavior `[DONE — V0.1 INSTALLED-WINDOWS VALIDATION]`

Goal: replace provisional appearance handling with the initial complete visual toolset.

Deliverables:

- versioned 64x64 source format;
- layers;
- palette;
- pencil, eraser, fill, eyedropper, selection, mirror, undo, redo, zoom;
- PNG import;
- attachment points;
- animation preview;
- configurable gravity;
- simple collision;
- taskbar, window-edge, icon, and agent surfaces where feasible;
- right-click quick menu;
- multi-monitor recovery.

Expected commit: `feat: add pixel editor and overlay physics`

## Phase 6: v0.1 stabilization and packaging `[DONE]`

Goal: produce a testable public Windows v0.1 package.

Deliverables:

- Windows 10 packaging;
- migration and restart tests;
- reference hardware benchmark pass;
- secret scan;
- installer content inspection;
- public documentation;
- known limitations;
- release checklist;
- honest manual validation record.

Expected commit: `release: prepare AIP v0.1`

Installed-Windows manual validation was approved on 2026-07-30 for packaged SHA
`6b5dc1a0a18d3e346d04c6bd89de13775c681434` (CI run `30474813207`).

## Post-v0.1 baseline

AIP v0.1.0 was published from commit `b6f74b3793437a647186dd52eeb950ff4b3fb228`.
The release and its documented manual-validation record are the authoritative baseline for
post-v0.1 work. Implemented v0.1 behavior remains subject to its published limitations;
future work must not be represented as released behavior.

## Post-v0.1 Phase 7: cognitive core

The implementation contract is [the cognitive-core specification](COGNITIVE_CORE_SPEC.md).
The Phase 7 product decisions are resolved and PR #2 is merged. Phase 7A is complete as
recorded in [its validation record](PHASE_7A_VALIDATION.md); the current development checkpoint
covers bounded 7B–7D controls and the Phase 7E public conversation path. Phase 7F is the
validation/documentation checkpoint recorded in [its validation record](PHASE_7F_VALIDATION.md);
runtime, manual, and remote reservations remain explicit there. This is not a v0.1 release claim.

- **Phase 7A — cognitive event foundation and protected/evolvable boundaries [APPROVED — DONE]:** typed, bounded ordinary candidates, owner correction, safe explanation, and latest-event compensating rollback. The exact commit, CI, and Owner attestation are recorded in the validation record. Live extraction and later Phase 7E–7F capabilities remain excluded.
  Owner-scoped event processing, typed source eligibility, trait limits, auditability, and temporary-chat exclusion. Conversation-source validation is a future-adapter boundary only; live extraction remains excluded.
- **Phase 7B — opinions and evidence [PRODUCTIZED — HUMAN VALIDATION PENDING]:** sourced,
  inspectable opinions with correction, dispute, supersession, fail-closed source lifecycle,
  and Owner-visible provenance. Automated Rust/TS validation is complete; installed-Windows
  and subjective Portuguese review remain pending.
- **Phase 7C — relationships [PRODUCTIZED — HUMAN VALIDATION PENDING]:** bounded six-
  dimension relationship state, event history, limits, reset, rollback, and deterministic
  projection recomputation after source invalidation.
- **Phase 7D — goals and fictional activities [PRODUCTIZED — HUMAN VALIDATION PENDING]:**
  approval-bound fictional goals with due/expiry semantics and explicitly fictional,
  budgeted activity lifecycle controls with no external action.
- **Phase 7E — bounded agent-to-agent conversation [PRODUCTIZED — HUMAN VALIDATION PENDING]:**
  visible, purpose-bound public interactions with hard resource budgets, termination reason,
  consent/revocation checks, and deferred candidate processing. See [the Phase 7E validation
  record](PHASE_7E_VALIDATION.md).
- **Phase 7F — integrated validation and UX hardening [PRODUCTIZED — HUMAN VALIDATION PENDING]:**
  current automated validation, Portuguese UI boundaries, persistence/recovery tests, safety
  controls, and honest documentation. Installed-Windows interaction and subjective Portuguese
  review remain the sole human gate; see [the Phase 7F validation record](PHASE_7F_VALIDATION.md).

Current corrective evidence (2026-08-24): Rust source lifecycle validation,
transactional memory invalidation/recalculation, deterministic goal/activity expiry,
and focused cognitive tests are green. Desktop Vitest is green in the current host
(14 files, 51 tests). Installed-Windows interaction and subjective Portuguese review
remain HUMAN VALIDATION PENDING; the earlier executor `spawn EPERM` report is
historical environment evidence, not the current host result.

## Post-v0.1 Phase 8: voice [EXTERNAL-PREREQUISITE PRODUCTIZED — HUMAN VALIDATION PENDING]

The local runtime/effects checkpoint is productized in Rust/SQLite, Tauri commands,
versioned contracts, native Windows wave-device enumeration, bounded in-memory capture/playback,
replaceable local STT/TTS argv adapters, wake-word routing, and Portuguese Owner controls.
Missing devices/models degrade to text fallback; no cloud path, raw-audio persistence, or hidden
listener exists. Hardware quality, packaged restart behavior, and manual Owner validation remain
pending.

Reserved validation covers real audio devices and speech models, packaged Windows
behavior, and subjective voice quality. The checkpoint keeps captured PCM only in bounded
memory and has no raw-audio persistence, upload/network path, real-person cloning, or hidden
listener. See the
[Phase 8 voice specification](PHASE_8_VOICE_SPEC.md).

## Post-v0.1 Phase 9: supervised tools `[EXTERNAL-PREREQUISITE PRODUCTIZED — HUMAN VALIDATION PENDING]`

The current checkpoint implements the bounded local architecture described in
[PHASE_9_TOOLS_SPEC.md](PHASE_9_TOOLS_SPEC.md):

- versioned catalog with read-only/state-changing classification;
- granular fixture-scoped sessions and permissions;
- exact action preview, dry-run, Owner approval, and manifest-bound second confirmation;
- deterministic workspace, calendar, and messaging adapter mocks plus real bounded local
  workspace inspection and approved move/compensation effects;
- bounded untrusted output, cancellation, compensation metadata, and 30-day audit retention;
- temporary-chat and safe-mode fail-closed controls with Portuguese Owner UI.

The local effects are confined to explicit Owner-configured roots with canonical containment,
link/system-directory rejection, preview, approval, second confirmation, dry-run, audit, and
compensation. Shell, credentials, network, and external-provider mutation remain disabled;
packaged-Windows UX and live provider credentials remain reserved validation work.

## Post-v0.1 Phase 10: extensions [PRODUCTIZED — AUTOMATED VALIDATION COMPLETE; HUMAN PACKAGE REVIEW PENDING]

The current checkpoint implements the bounded local runtime and effects described in
[PHASE_10_EXTENSIONS_SPEC.md](PHASE_10_EXTENSIONS_SPEC.md):

- versioned, untrusted metadata manifests with local-fixture admission;
- private catalog and review-only agent proposals;
- explicit Owner capability review and activation;
- updates that disable the current revision and force re-review;
- explicit rollback, disable, bounded audit retention, and Portuguese controls;
- safe-mode and temporary-chat fail-closed mutation gates.

This executes only closed declarative packages through Rust; it does not load,
compile, fetch, or execute native extension code. Human extension usability,
package review, and release approval remain pending. It
does not access the network, shell, host filesystem, credentials, remote code,
or a public marketplace. Human package review, real plugin usability, ratings,
packaged-Windows evidence, and release approval remain reserved.

## Post-v0.1 Phase 11: screen vision `[EXTERNAL-PREREQUISITE PRODUCTIZED — HUMAN VALIDATION PENDING]`

The current checkpoint implements the bounded local architecture described in
[PHASE_11_SCREEN_VISION_SPEC.md](PHASE_11_SCREEN_VISION_SPEC.md):

- explicit local Owner identity and confirmation;
- synthetic monitor fixture selection and on-demand Windows display discovery;
- per-session capture/analyze permissions, privacy policy, and redaction hooks;
- preview before confirmation, one-job reference-GPU scheduling, and quotas;
- on-demand synthetic model-fixture lifecycle with automatic cleanup and a replaceable local
  `aip-screen-vision-v1` executable adapter for confirmed real displays;
- cancellation, bounded uncertain non-diagnostic hypothesis, and audit history;
- Rust/SQLite authoritative temporary-chat and safe-mode fail-closed gates;
- Portuguese Owner-facing Screen Vision controls and versioned contracts.

This checkpoint captures a confirmed Windows display only in bounded transient
memory; it does not retain pixels or screenshot bytes, analyze continuously, run in the background,
access the host filesystem/shell/credentials, use a network or remote model, or
persist visual state. The local adapter reports unavailable when its explicitly configured
provider/model is missing. Windows packaging evidence,
privacy/visual UX validation, and release approval remain reserved work.

## Post-v0.1 Phase 12: Android companion `[PRODUCTIZED FOR DEBUG COMPANION — HUMAN DEVICE VALIDATION PENDING]`

The current checkpoint implements the bounded local architecture described in
[PHASE_12_ANDROID_SPEC.md](PHASE_12_ANDROID_SPEC.md):

- versioned `aip-companion-v1` protocol metadata and synthetic Android fixture;
- Owner-scoped pairing/confirmation, protocol negotiation, replay protection,
  reconnect, key rotation, and revocation;
- bounded read-only history and metadata-only text/audio/image/file/task queue;
- explicit preview, Owner approval, cancellation, retry, and audit;
- Rust/SQLite authority with safe-mode and temporary-chat fail-closed gates;
- Portuguese Companion controls and versioned response parsers.

The Android project builds a debug APK and tests the bounded authenticated
local/private protocol and explicit-connect client. Deterministic loopback is
covered; physical-device UX, private-LAN smoke testing, and release signing
remain reserved checks. No relay or external account is claimed.

## Post-v0.1 Phase 13: gateway boundary `[BACKEND-ONLY — LOOPBACK CHECKPOINT; HUMAN PRIVATE-LAN CLIENT WORKFLOW PENDING]`

The current checkpoint implements the bounded local architecture described in
[PHASE_13_GATEWAY_SPEC.md](PHASE_13_GATEWAY_SPEC.md):

- real bounded `aip-gateway-v1` framed HMAC TCP transport with signed errors,
  replay protection, private bind policy, and explicit start/stop lifecycle;
- Owner-scoped transfer preview/approval, session proof/replay checks,
  administrative recovery approval, revocation, and audit;
- Rust/SQLite authority with safe-mode and temporary-chat fail-closed gates;
- Portuguese desktop Gateway controls with explicit Owner-confirmed listener
  lifecycle and transient pairing display; deterministic loopback TCP/SQLite
  authority tests.

This does not integrate BielOS accounts, transfer a real agent, provide a
public relay or tunnel, or use Cloudflare credentials. Private-LAN/hardware
manual validation, recovery UX, release signing, remote CI, external ownership
exchange, and remote/mobile delivery remain separately authorized future scope.

## Deferred research

Research without implementation commitment:

- model routing and automatic downgrade;
- alternate local runtimes;
- advanced embeddings and retrieval;
- secure full agent package export/import;
- physically bundled models in exports;
- derived-agent lineage;
- long-term backup versioning;
- more capable hardware profiles.

## Phase review rule

After every phase:

1. inspect the commit and diff;
2. compare against the phase scope;
3. run or confirm validations;
4. record limitations;
5. select the next phase only after the current one is approved.

Use `.agents/skills/aip-phase-review/SKILL.md` for phase review.
