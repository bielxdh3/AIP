# AIP MVP v0.1

## 1. Goal

AIP v0.1 proves that two independent local agents can exist in a resilient Windows desktop application with separate identities, pixel-art overlays, local conversations, interchangeable models, initial memory, and persistent state.

The v0.1 release is tested only by the Owner.

## 2. Required platform

- Windows 10 64-bit minimum
- Windows 11 compatibility when practical
- CPU-only operation allowed
- iGPU operation allowed
- NVIDIA GPU acceleration allowed through the selected model runtime
- GTX 1060 6 GB used as the reference dedicated GPU

No Android or BielOS integration is included in v0.1.

## 3. Required technology

- pnpm monorepo
- Tauri
- Rust
- React
- TypeScript
- Python runtime managed by the desktop application
- SQLite authoritative database
- Ollama first model adapter
- Apache-2.0 license

## 4. Required v0.1 capabilities

### 4.1 Application resilience

- The main application opens without Python.
- The main application opens without Ollama.
- The main application opens without any installed model.
- Runtime or model failure does not close the UI.
- History and settings remain available in degraded mode.
- Safe mode can start without the Python runtime.

### 4.2 Owner profile

- One implicit local Owner profile.
- No login, password, PIN, or idle lock.
- The data model must support future accounts and ownership transfer without requiring a v0.1 user-facing account system.

### 4.3 Two agents

- Exactly two agents may be created in the initial flow.
- Both agents are initially owned by the Owner.
- Each agent has an independent identifier, profile, appearance, chats, memories, state, and model preference.
- Cross-agent data access must be explicit and never accidental.
- Dynamic creation of additional agents is not included.

### 4.4 Agent creation flow

The creation flow supports:

- name;
- birthday;
- selected initial age;
- age type or category;
- species;
- pronouns;
- initial traits with visible intensity values;
- initial appearance;
- initial screen position;
- default model selection;
- simple model explanations for non-technical users.

The flow may use provisional assets and model descriptions.

### 4.5 Pixel-art overlay

- 64x64 base artwork.
- Integer scaling without blur.
- Two agents visible simultaneously.
- Transparent overlay windows.
- Always-on-top mode.
- Optional desktop-only mode.
- Automatic hiding during full-screen applications where Windows detection is reliable.
- Click-through outside interactive character and bubble regions.
- Drag and drop.
- Configurable gravity behavior.
- Simple collision behavior.
- Configurable use of taskbar, window edges, desktop icons, and other agent as visual surfaces where technically feasible.
- Single click opens speech bubble.
- Double click opens main application.
- Right click opens quick menu.

### 4.6 Initial animations

Required animation states:

- idle;
- walk;
- dragged;
- sleep;
- talk;
- happy;
- irritated;
- thinking.

Placeholder art is acceptable.

### 4.7 Pixel editor

Initial editor supports:

- 64x64 canvas;
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
- PNG import;
- fully transparent or fully opaque normal pixels;
- attachment points;
- saving and loading a versioned source document.

Initial editable categories:

- body;
- hair;
- clothing;
- accessory;
- color.

The data format must allow more categories later.

### 4.8 Speech bubbles

- Bubble preview appears over each character.
- Default preview is three lines.
- Line limit is configurable.
- Long replies show a concise preview.
- Click expands the bubble.
- Expanded bubble shows the complete reply.
- Expanded bubble allows text reply without opening the main application.
- Thinking state animates both bubble and character.
- Multiple bubbles may be open simultaneously.
- Heavy model generation is serialized when required.

### 4.9 Conversations

- Multiple chats per agent.
- One main chat per agent with no fixed topic.
- Create, rename, select, and archive conversations.
- Separate message history per agent and conversation.
- Conversation-level model override.
- Streaming model response when supported.
- Cancel in-progress generation.
- Clear status for model unavailable, loading, busy, failed, or ready.

### 4.10 Temporary chat

- Temporary chat content exists only in RAM.
- Closing the chat removes it.
- Application exit or crash removes it.
- No message, summary, memory, or learning content is written to SQLite or files.
- Temporary chat is read-only with respect to external state.
- v0.1 has no state-changing tools, but the policy must be represented in contracts and tests.

### 4.11 Model system

- Detect installed Ollama models.
- Show model availability and basic capabilities.
- Select default model per agent.
- Override model per conversation.
- Return conversation to agent default.
- Show a non-technical model explanation.
- Advanced custom Ollama endpoint may be stored in settings.
- Default keep-alive is 15 minutes and configurable.
- Only one heavy generation runs at once on reference hardware.
- UI and animation remain responsive while generation is queued.
- No default model is permanently hardcoded before post-test selection.

### 4.12 Model benchmarking

Store optional benchmark observations:

- model identifier;
- provider;
- hardware summary;
- load time;
- time to first token;
- tokens per second;
- peak or observed RAM;
- peak or observed VRAM when available;
- context size used;
- qualitative owner rating;
- test date.

Benchmarks must be clearly labeled as local observations, not universal specifications.

### 4.13 Initial memory

- Fixed agent profile.
- Manual memory entry.
- Automatic memory candidate extraction from normal conversations.
- Categories: permanent, preference, fact, rule, emotional memory.
- Source message reference.
- Created date.
- Confidence and confirmation status.
- Owner may view, edit, archive, restore, or delete memory.
- Direct owner statements may be saved automatically.
- Inferences are not saved as confirmed facts.
- Internet information remains provisional.
- Conflicting memories may coexist with source and date.
- Full chat history remains searchable.
- Recent conversation summary may be stored.

Deep semantic retrieval and advanced personality evolution are not required for v0.1.

### 4.14 Initial personality

- Starting trait values are stored and visible.
- Traits influence prompt/context assembly in a basic deterministic way.
- Traits are not deeply self-modifying in v0.1.
- The Owner agent and girlfriend-oriented agent may use different starting presets.

### 4.15 Fictional state

Required states:

- sleep;
- energy;
- mood.

Requirements:

- state is persisted;
- state affects animation;
- state may affect response style in a bounded way;
- user can inspect state;
- user can adjust configuration;
- `wake now` is available;
- state transitions are deterministic outside model generation;
- suspension pauses fictional state but not age;
- offline elapsed time may be applied gradually after startup.

### 4.16 Modes

Required modes:

- normal;
- voice-muted placeholder state;
- silent;
- safe.

v0.1 has no real voice, but the mode must exist in the data and UI model.

Silent mode:

- blocks spontaneous activity;
- direct text interaction remains allowed.

Safe mode:

- does not start the model runtime;
- hides agent overlays;
- keeps administrative UI, history, settings, and recovery available.

### 4.17 Settings

At minimum:

- data paths;
- Ollama endpoint advanced setting;
- model keep-alive;
- overlay always-on-top;
- desktop-only mode;
- full-screen hiding;
- bubble line limit;
- gravity behavior;
- collision behavior;
- allowed visual surfaces;
- per-agent monitor behavior;
- safe-mode startup option;
- basic resource limits.

## 5. Explicitly excluded from v0.1

- Android application;
- BielOS integration;
- Cloudflare;
- speech recognition;
- speech synthesis;
- voice cloning;
- wake word;
- screen capture or vision;
- real Windows control;
- file organization tools;
- sending messages or email;
- calendar integration;
- extension creation or marketplace;
- autonomous conversations between agents;
- autonomous goals;
- deep evolving personality;
- full relationship simulation;
- agent export/import;
- backup implementation;
- remote access;
- multiple user accounts;
- agent ownership transfer;
- unrestricted model APIs.

## 6. Implementation phases

### Phase 0: repository bootstrap and visual shell

Deliver:

- pnpm workspace;
- Tauri + React + TypeScript desktop app;
- Rust application core skeleton;
- Python runtime skeleton with health handshake;
- versioned contract package;
- SQLite migration framework;
- two provisional 64x64 agents;
- transparent overlay proof;
- drag behavior;
- always-on-top behavior;
- full-screen hiding proof where feasible;
- basic main panel;
- safe-mode boot path;
- degraded runtime status;
- tests for deterministic core behavior;
- Windows setup documentation.

Do not include Ollama, real chat, memory extraction, or full editor in Phase 0.

### Phase 1: local conversation vertical slice

Deliver:

- Ollama discovery;
- model selection;
- one serialized generation queue;
- chat message persistence;
- streaming response;
- bubble preview and expansion;
- runtime/model unavailable behavior;
- 15-minute keep-alive.

### Phase 2: agent creation and separation

Deliver:

- Owner initialization;
- two-agent creation flow;
- independent profiles;
- traits;
- model default per agent;
- conversation override;
- strict isolation tests.

### Phase 3: memory and temporary chat

Deliver:

- manual memory;
- automatic memory candidates;
- summaries;
- memory review UI;
- temporary in-memory chat;
- non-persistence tests.

### Phase 4: states and modes

Deliver:

- sleep, energy, mood;
- deterministic transitions;
- wake-now control;
- silent and voice-muted state;
- robust safe mode;
- offline elapsed-time application.

### Phase 5: pixel editor and richer overlay

Deliver:

- complete initial pixel editor toolset;
- layer format;
- attachment points;
- animation preview;
- configurable physics and surfaces;
- right-click quick menu;
- multiple monitors.

### Phase 6: stabilization and v0.1 packaging

Deliver:

- Windows 10 packaging;
- migration testing;
- performance testing on reference hardware;
- public release checks;
- installer validation;
- documentation;
- known limitations.

## 7. Global acceptance criteria

v0.1 is accepted only when:

1. The application starts on Windows 10 64-bit.
2. Two agents can exist with separate data.
3. The UI remains usable without Python, Ollama, or a model.
4. A model can be changed without changing agent identity.
5. One heavy inference runs at a time.
6. Chats persist correctly after restart.
7. Temporary chat does not persist after close or crash.
8. Memory from one agent does not leak to the other.
9. Safe mode starts without model runtime or overlays.
10. Placeholder agents can be dragged and remain interactive.
11. Speech bubbles support compact and expanded interaction.
12. Initial pixel-art documents can be edited, saved, and loaded.
13. No secrets, databases, model files, histories, or personal data are committed.
14. Required validations are documented and pass for the release scope.
15. Known manual Windows validations are recorded honestly.