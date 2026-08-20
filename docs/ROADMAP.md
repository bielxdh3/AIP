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
record above. The separate Phase 1 validation document retains historical failed and pending
pre-release attempts; it does not replace the later installed-package approval.

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
recorded in [its validation record](PHASE_7A_VALIDATION.md); later slices remain unstarted.

- **Phase 7A — cognitive event foundation and protected/evolvable boundaries [APPROVED — DONE]:** typed, bounded ordinary candidates, owner correction, safe explanation, and latest-event compensating rollback. The exact commit, CI, and Owner attestation are recorded in the validation record. Live extraction and all Phase 7B–7E capabilities remain excluded.
  Owner-scoped event processing, typed source eligibility, trait limits, auditability, and temporary-chat exclusion. Conversation-source validation is a future-adapter boundary only; live extraction remains excluded.
- **Phase 7B — opinions and evidence:** sourced, inspectable opinions with correction,
  dispute, and supersession workflows.
- **Phase 7C — relationships:** bounded per-subject relationship dimensions, event history,
  limits, reset, and rollback.
- **Phase 7D — goals and fictional activities:** approval-bound durable goals and explicitly
  fictional, budgeted activity state with no external action.
- **Phase 7E — bounded agent-to-agent conversation:** visible, purpose-bound interactions
  with hard resource budgets and deferred candidate processing.
- **Phase 7F — integrated validation and UX hardening:** Portuguese explanations, restart
  behavior, safety controls, and full cross-boundary validation.

## Post-v0.1 Phase 8: voice

Potential scope:

- local speech recognition;
- lightweight wake word;
- speech synthesis;
- custom voice and consent flow;
- base-voice protection;
- emotional-hypothesis classification;
- voice-muted and silent-mode completion.

## Post-v0.1 Phase 9: supervised tools

Potential scope:

- tool manifest;
- granular session permissions;
- action preview;
- approval and forced-execution flow;
- read-only and state-changing separation;
- file organization tools;
- calendar and messaging tools;
- audit retention;
- safe rollback.

## Post-v0.1 Phase 10: extensions

Potential scope:

- extension SDK;
- sandbox;
- private catalog;
- agent-created extensions;
- administrator-selected third-party extensions;
- permission-aware updates;
- rollback and ratings.

## Post-v0.1 Phase 11: screen vision

Potential scope:

- on-demand screenshot only;
- explicit user request;
- separate visual model loaded on demand;
- no continuous screen analysis;
- privacy controls;
- resource scheduling.

## Post-v0.1 Phase 12: Android client

Potential scope:

- BielOS APK agent module;
- floating icon;
- text and voice conversation;
- notifications;
- read-only offline history;
- offline message queue;
- approval flow;
- authenticated connection to the PC.

## Post-v0.1 Phase 13: BielOS integration

Potential scope:

- versioned AIP gateway;
- BielOS accounts and ownership;
- transfer of girlfriend-oriented agent;
- Cloudflare Tunnel and Access;
- mobile administrative recovery;
- preserved AIP standalone operation;
- no direct exposure of Python runtime internals.

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
