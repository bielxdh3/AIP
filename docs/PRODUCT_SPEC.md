# AIP Product Specification

## 1. Product identity

**AIP** means **Agentes Independentes Personalizáveis**.

- Product name: `AIP`
- Stylized visual identity: `A.I.P.`
- Repository: `bielxdh3/AIP`
- License: Apache License 2.0
- Repository visibility: public
- Repository language: English
- User interface language: Portuguese

AIP is a local-first platform for creating customizable 2D agents with independent identities, persistent memory, interchangeable models, evolving behavior, supervised tools, and a future integration path with BielOS.

AIP is implemented as a standalone product first. BielOS integration must happen through stable contracts and APIs after AIP is functional on its own.

## 2. Supported platforms

Planned platforms are limited to:

- Windows 10 64-bit as the initial minimum target;
- Windows 11 64-bit;
- Android in a later phase.

Linux, macOS, and iOS are explicitly out of scope with no current plan to support them.

## 3. Initial users and ownership

The initial implementation is tested only by the Owner.

The v0.1 application uses an implicit local Owner profile without login, PIN, password, or idle lock.

The initial product supports two agents:

1. The Owner's agent.
2. The future girlfriend-owned agent, initially created under the Owner until local accounts and ownership transfer exist.

Later rules:

- only an administrator may create new agents;
- external extensions may only be installed by the administrator;
- agent ownership may be transferred to another local account;
- each account accesses only its own agent, private chats, history, and settings;
- owners may access public conversations between agents.

## 4. Agent identity

Agent identity is independent from the language model.

Changing, removing, or replacing a model must not erase or replace:

- identity;
- name;
- birthday and age;
- appearance;
- memories;
- conversation history;
- personality;
- opinions;
- relationships;
- goals;
- preferences;
- voice;
- state;
- permissions.

Each agent has a unique identifier. Imported duplicates become derived individuals with new identifiers and a new copy birthday while preserving the original creation lineage.

## 5. Initial agent profiles

The Owner's agent is initially:

- technical;
- critical;
- task-oriented;
- context-seeking;
- evidence-sensitive;
- able to keep conflicting claims with source and date.

The girlfriend-oriented agent is initially:

- cute and expressive;
- emotional without claiming real sentience;
- curious;
- childlike in learning style;
- focused on conversation, teaching, games, stories, and gentle questions;
- expected to ask for confirmation when teachings conflict.

These are starting traits, not immutable scripts. Later personality evolution may change preferences, opinions, mannerisms, relationships, and behavior within protected identity boundaries.

## 6. Agent creation

The creation flow asks for:

- name;
- chosen birthday;
- chosen initial age;
- age category or non-human age model;
- species or character type;
- pronouns;
- initial personality traits and intensity values;
- appearance;
- initial screen position;
- default model;
- model explanation written for non-technical users.

Birthday may predate the real creation date. Initial age is owner-selected rather than strictly calculated.

Personality traits are visible and extensible. Initial examples include:

- curiosity;
- sociability;
- criticality;
- spontaneity;
- affection;
- autonomy.

## 7. Models

Models are replaceable components.

Initial model provider:

- Ollama.

Requirements:

- automatically detect installed Ollama models;
- allow an advanced custom Ollama endpoint;
- define a default model per agent;
- allow a temporary override per conversation;
- allow selection from inside a conversation;
- return to the agent default on request;
- explain models in simple language such as faster but more limited, slower but more capable, tool-capable, or memory-heavy;
- record benchmark data when tested;
- allow the administrator to install, disable, and remove models;
- later allow agents to request a model change, requiring permission;
- later allow automatic model downgrade when configured and resources are constrained;
- permit free models even when their licenses are not formally open source.

The reference hardware is a GTX 1060 6 GB, but CPU, iGPU, and future GPU upgrades must remain supported.

Only one heavy generation uses the reference GPU at a time. Multiple agents may remain visually active while model work is queued.

Default model keep-alive is 15 minutes and is configurable.

## 8. Conversations

Each agent supports multiple conversations similar to ChatGPT.

One conversation is marked as the main conversation and has no fixed subject.

Conversation types:

### 8.1 Normal conversation

- persisted in history;
- eligible for automatic memory extraction;
- eligible for summaries;
- associated with the active owner and agent;
- may use the default model or a conversation override.

### 8.2 Private conversation

- persisted;
- visible only to the owner of the agent;
- may generate memories for that agent;
- protected by account/profile boundaries rather than disk encryption in the initial plan.

### 8.3 Temporary conversation

- held only in RAM;
- must disappear completely when closed or after a crash;
- must not persist conversation content, summaries, memories, or learning;
- read-only with respect to external state;
- may converse, search, and use approved read-only tools;
- may not create, edit, move, rename, delete, install, send, or otherwise change external state;
- may retain only minimal non-content action audit according to the retention policy.

## 9. Memory

Automatic memory saving is required.

Initial memory system includes:

- fixed agent profile;
- confirmed facts from the owner;
- recent conversation summary;
- full searchable history;
- manually added memories;
- automatically extracted memory candidates.

Memory categories include:

- permanent;
- preference;
- fact;
- rule;
- emotional memory.

Memory rules:

- direct owner statements may be saved automatically;
- inference must not be stored as fact;
- internet findings remain provisional until confirmed;
- conflicting information is retained with source, date, and context;
- memories may be viewed, corrected, archived, restored, or deleted;
- rarely used important memories remain stored;
- redundant or low-value context may be consolidated or archived;
- full history remains available even when summaries are used for model context.

Later memory tiers are:

- active;
- permanent;
- archive;
- trash.

## 10. Personality, opinions, and relationships

Deep personality evolution is later than v0.1, but the architecture must support it.

Agents may later:

- develop preferences and opinions;
- disagree with owners;
- change opinions through conversation rather than direct editing;
- form hobbies and goals;
- remember emotional associations with dates, clothing, music, and conversations;
- form relationships such as friendship, trust, irritation, rivalry, and admiration;
- have visible relationship values;
- be affected by conversations with other agents;
- form opinions about models, extensions, applications, clothing, and people mentioned in conversations.

Opinions about real people must distinguish verified fact, reported experience, and impression.

Public agent-to-agent conversations are visible to both owners. Agents do not have formal hidden private chats from owners, though their personality may cause them to avoid or resist answering a direct question.

## 11. Fictional state

v0.1 includes initial support for:

- sleep;
- energy;
- mood.

Later state may include:

- hunger;
- curiosity;
- focus;
- social fatigue.

State may affect:

- visual animation;
- writing style;
- willingness to accept non-urgent requests;
- frequency of spontaneous conversation;
- choice of fictional activities.

Agents may choose fictional activities such as sleeping, resting, playing, or reading to recover state.

When the PC is off, time-based state and conceptual goals continue and are applied gradually after startup.

Suspension pauses activities, goals, and fictional states, but age continues.

An `wake now` control temporarily overrides sleep and fatigue.

## 12. Desktop visual system

Agents use 2D pixel art inspired by an 8-bit aesthetic:

- 64x64 base sprites;
- integer scaling;
- large visible pixels;
- thick dark outlines;
- compact palettes;
- opaque or fully transparent normal pixels;
- lightweight frame animation.

The visual reference from the planning conversation is inspiration only. Do not copy third-party art assets.

The visual system supports layered parts:

- body;
- face and expressions;
- hair;
- upper clothing;
- lower clothing;
- shoes;
- accessories;
- held objects;
- effects.

Parts use attachment points such as head, hands, back, and feet.

The initial editor supports:

- pencil;
- eraser;
- fill;
- eyedropper;
- selection;
- mirror;
- undo and redo;
- layers;
- palette;
- zoom;
- animation preview;
- PNG import.

Initial animation states:

- idle;
- walk;
- dragged;
- sleep;
- talk;
- happy;
- irritated.

## 13. Desktop overlay behavior

Agents are visible until hidden.

Configurable behaviors include:

- always on top;
- desktop-only mode;
- hide when a full-screen application is active;
- transparent-window click-through;
- drag and drop;
- gravity behavior;
- simple collisions;
- screen and monitor boundaries;
- walking on taskbar, window edges, desktop icons, or another agent;
- preferred screen position;
- observing the cursor;
- future running, jumping, sitting, and playing.

Click behavior:

- single click opens or focuses the speech bubble;
- double click opens the main AIP panel;
- right click opens a quick menu.

Quick menu actions:

- conversar;
- silenciar;
- dormir;
- personalizar;
- trocar modelo;
- ocultar;
- abrir painel.

## 14. Speech bubbles

Short responses, greetings, status messages, and warnings appear in comic-style speech bubbles over the character.

Requirements:

- default maximum preview of three lines;
- configurable line limit;
- click to expand;
- full reply available in expanded view;
- reply input available without opening the main application;
- button to open the full conversation;
- thinking animation in both bubble and character;
- multiple bubbles may be visible simultaneously;
- heavy model replies are queued, so one agent speaks after the other when required.

Example statuses:

- `Modelo indisponível`;
- `Carregando modelo…`;
- `Processamento mais lento`;
- `Memória registrada`;
- `Modo silencioso ativo`.

## 15. Modes

### Normal mode

- voice later enabled;
- spontaneous activity allowed;
- agent-to-agent conversation later allowed;
- background goals later allowed when resources permit.

### Voice-muted mode

- no audible speech;
- visual and text interaction remain active;
- agents may later notify the owner or converse silently.

### Silent mode

- no spontaneous actions;
- no autonomous work;
- no autonomous agent-to-agent conversation;
- if directly called by voice, respond by voice;
- if directly called by text, respond by text.

### Safe mode

- models disabled;
- Python runtime optional or disabled;
- extensions and autonomous work disabled;
- agent overlays removed from the screen;
- administrative UI, history, settings, diagnostics, and recovery remain available;
- future authenticated mobile recovery may remain available while agent network access is blocked.

Safe mode may later activate manually, from mobile, during startup, after repeated failures, or after suspicious behavior.

## 16. Resource management

AIP must support:

- CPU, RAM, GPU, VRAM, and task-time limits;
- one heavy generation at a time on reference hardware;
- active chat as the highest priority;
- Owner commands after active chat;
- background tasks at minimal resource use;
- task pause when system load rises;
- visible warning when local processing is busy;
- configurable model unload after 15 minutes by default;
- continued UI and animation while inference is queued;
- future automatic model downgrade when enabled.

The girlfriend-oriented agent prioritizes conversation. The Owner agent prioritizes tasks, without interrupting an active conversation unnecessarily.

## 17. Tools, permissions, and approvals

Real tools are later than the first implementation phase.

Long-term rules:

- agents may research freely when allowed;
- state-changing actions require explicit permission;
- destructive or external actions require individual approval;
- approvals show exact action, reason, affected files/accounts/programs, current state, expected result, preview, rollback availability, and approve/deny/modify controls;
- permission grants for extensions are session-scoped;
- the Owner agent may request approval from mobile;
- the girlfriend-oriented agent has more restrictive computer control;
- forced execution requires a second explicit confirmation;
- the agent records continued disagreement when forced;
- forcing is available to the owner of that agent, not only the administrator;
- absolute prohibitions cannot be overridden, including deliberate system destruction, credential exposure, and illegal actions.

## 18. Extensions

Later AIP supports extensions that:

- declare granular permissions;
- run first in a sandbox when newly created or externally installed;
- show source, permissions, impact, tests, and rollback information;
- may be created by agents but require approval before activation;
- are shared between agents only with authorization;
- may be rated per agent, version, task, and model;
- update automatically when no new permissions are requested;
- require approval when an update adds permissions;
- may come from selected third parties approved by the administrator.

## 19. Voice and emotional inference

Later voice support includes:

- local speech recognition when practical;
- lightweight wake-word detection;
- speech synthesis;
- custom or cloned voice with consent;
- configurable tone, speed, expression, accent, apparent age, and intensity;
- immutable base voice unless the owner authorizes a change.

Emotional inference from text or voice is always a hypothesis. It must never be represented as fact, diagnosis, or certainty.

## 20. Android and BielOS integration

Android and BielOS integration are later phases.

Planned Android behavior:

- BielOS APK module for agents;
- floating Android icon;
- text and voice conversation;
- notifications, including optional spoken audio;
- Owner approval requests;
- read-only synchronized history while PC is offline;
- offline queue for text, audio, images, files, and task requests;
- confirmation before processing the offline queue after the PC reconnects;
- ambiguous or expired requests require clarification.

Remote connectivity should use trusted infrastructure such as Cloudflare Tunnel and Access while keeping model execution and primary data on the PC.

## 21. Export, import, and backup

Later export creates one package containing everything, including:

- identity;
- memories;
- full history;
- private data and secrets;
- models;
- extensions;
- voice;
- appearance;
- media;
- relationships;
- goals;
- opinions;
- configuration;
- lineage.

Export requires an additional confirmation because the package contains private data.

Import previews all included data, extensions, models, permissions, and risks before acceptance.

Imported copies become derived individuals. The original is notified when possible.

Storage planning assumes active mirrored storage may be used, but RAID 1 is not treated as a backup. A separate versioned backup destination is required.

## 22. Audit and retention

Long-term audit records include:

- requester;
- approver;
- agent;
- model;
- tool;
- permissions;
- affected resources;
- result;
- rollback information.

Audit is retained for 30 days and then deleted automatically.

Temporary-chat audit must not contain conversation content.

## 23. Open-source and dependency policy

AIP should use the maximum practical amount of open-source software.

Trusted proprietary infrastructure may be used when justified, especially for later remote access.

Models only need to be free, replaceable, and compatible. They do not need to be formally open source.

## 24. Product non-goals for the initial implementation

The initial implementation is not:

- a cloud SaaS;
- a replacement for BielOS;
- a general unrestricted desktop automation agent;
- a medical or psychological diagnostic system;
- a sentient system;
- a cross-platform desktop framework demonstration;
- a full extension marketplace;
- a multi-user remote service.

The authoritative v0.1 scope is defined in `MVP_V0.1.md`.