# AIP Roadmap

## Roadmap principles

- Build AIP standalone before BielOS integration.
- Keep each phase small enough to review and validate.
- Do not claim a phase is complete without evidence.
- Preserve Windows 10 64-bit as the minimum target.
- Keep the UI functional when AI components are missing.
- Avoid implementing later features merely because an abstraction could support them.
- Stabilize contracts before Android or remote access.

## Phase 0: repository bootstrap and visual shell

Goal: produce the first executable Windows shell with two provisional agents and resilient runtime boundaries.

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

## Phase 1: local conversation vertical slice

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

## Phase 2: two-agent creation and isolation

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

## Phase 3: memory and temporary chat

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

## Phase 4: states, modes, and scheduling

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

Expected commit: `feat: add agent states and application modes`

## Phase 5: pixel editor and overlay behavior

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

## Phase 6: v0.1 stabilization and packaging

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

## Post-v0.1 Phase 7: deeper personality and relationships

Potential scope:

- personality evolution;
- opinions and evidence;
- relationship values;
- hobbies and goals;
- autonomous fictional activities;
- agent-to-agent conversations;
- owner-configurable relationship limits;
- gradual offline conceptual progression.

This phase requires a separate cognitive-core specification before implementation.

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