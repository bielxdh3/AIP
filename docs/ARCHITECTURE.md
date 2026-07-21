# AIP Architecture

## 1. Architecture goals

AIP must:

- run locally on Windows 10 64-bit;
- remain usable when the AI runtime is unavailable;
- keep agent identity independent from models;
- support two agents initially and more later;
- allow future Android and BielOS integration without rewriting the core;
- remain modular and testable;
- operate acceptably on a GTX 1060 6 GB reference GPU;
- avoid exposing unnecessary local network services;
- preserve clear permission and persistence boundaries.

## 2. Monorepo layout

Target layout:

```text
AIP/
├── apps/
│   └── desktop/                 # Tauri + React desktop application
├── services/
│   └── runtime/                 # Python inference runtime
├── packages/
│   ├── contracts/               # Versioned shared schemas and generated types
│   ├── agent-core/              # Deterministic agent-domain logic
│   ├── model-adapters/          # Model-provider abstractions
│   ├── memory/                  # Memory ranking and context assembly interfaces
│   ├── pixel-engine/            # Sprite, animation, attachment, and physics logic
│   ├── ui/                      # Reusable React components
│   └── test-support/            # Shared fixtures and test helpers
├── assets/
│   └── pixel-art/               # Public placeholder and built-in assets
├── docs/
├── scripts/
├── tests/
├── AGENTS.md
├── pnpm-workspace.yaml
├── package.json
├── Cargo.toml
└── pyproject.toml
```

Do not create every package during Phase 0 unless it has a concrete purpose. The layout is a target, not an excuse for empty directories and ceremonial abstractions.

## 3. Runtime boundaries

### 3.1 React and TypeScript

Responsibilities:

- Portuguese user interface;
- main panel;
- creation flow;
- chat surfaces;
- speech bubbles;
- pixel editor;
- settings;
- task and model status presentation;
- rendering deterministic visual state received from Rust.

React must not:

- directly access SQLite;
- directly spawn Python;
- directly call unrestricted shell commands;
- directly manage authoritative permissions;
- directly treat model output as trusted actions.

### 3.2 Rust and Tauri

Rust is the authoritative application core.

Responsibilities:

- application lifecycle;
- transparent overlay windows;
- always-on-top and desktop-only behavior;
- click-through hit testing;
- full-screen detection;
- monitor and window geometry;
- process lifecycle for Python;
- authoritative SQLite access and migrations;
- settings and data-root management;
- agent, chat, memory, state, and model metadata;
- inference queue orchestration;
- safe mode;
- permissions and approvals;
- crash recovery;
- emitting validated events to the UI.

Rust remains functional when Python or Ollama is absent.

### 3.3 Python runtime

Python is a replaceable inference service.

Responsibilities:

- model-provider calls;
- streaming token generation;
- embedding generation when introduced;
- memory-candidate extraction;
- summarization;
- structured response generation;
- later tool planning and emotional-hypothesis classification.

Python must not be the sole owner of:

- agent identity;
- history;
- memory truth;
- permissions;
- audit;
- UI state;
- model selection metadata;
- safe mode.

Python returns candidates and results. Rust validates and persists authoritative state.

### 3.4 Ollama

Ollama is the first model adapter.

Default endpoint:

```text
http://127.0.0.1:11434
```

An advanced custom endpoint may be configured later.

Ollama is optional at application startup. The UI must show an unavailable status without failing.

## 4. Inter-process communication

Initial Rust-to-Python communication uses newline-delimited JSON messages over managed child-process stdin/stdout.

Advantages:

- no exposed TCP port;
- Rust owns process lifecycle;
- straightforward request/response correlation;
- streaming events remain possible;
- runtime crash is detectable;
- easier local security model.

Protocol rules:

- UTF-8 only;
- one JSON object per line;
- explicit protocol version;
- unique request identifiers;
- typed method names;
- structured errors;
- heartbeat or health request;
- bounded message size;
- no arbitrary code execution method;
- no raw shell command method;
- stderr reserved for sanitized diagnostics;
- malformed messages terminate or quarantine the runtime connection safely.

Example envelope:

```json
{
  "protocolVersion": 1,
  "id": "req_01J...",
  "method": "model.generate",
  "params": {
    "agentId": "agt_01J...",
    "conversationId": "cnv_01J...",
    "modelRef": "ollama:qwen-example",
    "messages": []
  }
}
```

Streaming events use the request identifier:

```json
{
  "protocolVersion": 1,
  "event": "model.token",
  "requestId": "req_01J...",
  "data": {
    "text": "Olá"
  }
}
```

## 5. Authoritative persistence

SQLite is owned through the Rust data layer.

Reasons:

- the UI can access history while Python is unavailable;
- one migration system owns schema evolution;
- Python cannot silently rewrite identity or permissions;
- temporary chat can be kept outside persistence by design;
- audit and consistency rules remain deterministic.

Recommended Rust library: SQLx with checked migrations, subject to implementation validation.

Database access rules:

- WAL mode may be used after testing Windows behavior;
- foreign keys must be enabled;
- destructive schema changes require explicit migration planning;
- migrations run before normal application state loads;
- migration failure opens recovery or safe mode rather than silently resetting data;
- no model-generated SQL;
- no raw user content in diagnostic logs.

## 6. Storage layout

Application-controlled local files:

```text
%LOCALAPPDATA%\AIP\
├── config\
├── database\
├── logs\
├── runtime\
├── cache\
└── temp\
```

Configurable user data root:

```text
%USERPROFILE%\AIP-Data\
├── agents\
├── models\
├── memories\
├── histories\
├── media\
├── assets\
├── exports\
└── backups\
```

The actual implementation may keep relational history and memory metadata in SQLite while storing large binary assets in the data root.

Path rules:

- all configured paths are canonicalized;
- paths are not accepted from model output without validation;
- no path traversal;
- local data is excluded from Git;
- moving a data root uses a transactional migration plan;
- model storage may be configured separately.

## 7. Domain modules

### 7.1 Agent core

Deterministic domain logic for:

- agent identity;
- ownership;
- starting traits;
- birthday and age;
- state transitions;
- mode transitions;
- model preferences;
- screen preferences;
- later opinions, goals, and relationships.

The agent core must not depend on Ollama-specific types.

### 7.2 Conversation service

Responsibilities:

- multiple chats per agent;
- main chat designation;
- private chat flags;
- temporary in-memory sessions;
- message persistence;
- model override per conversation;
- recent-context retrieval;
- streaming state;
- cancellation;
- concise bubble preview generation.

### 7.3 Memory service

Initial responsibilities:

- manual memories;
- automatic memory candidates;
- confirmation and correction metadata;
- source message references;
- memory categories;
- active-memory retrieval;
- recent conversation summaries.

Later responsibilities:

- embeddings;
- semantic retrieval;
- consolidation;
- archive and trash;
- conflict ranking;
- relationship and opinion evidence.

### 7.4 Model service

Responsibilities:

- provider registry;
- installed-model discovery;
- model capabilities;
- default model per agent;
- model override per conversation;
- benchmark metadata;
- generation queue;
- cancellation;
- 15-minute default keep-alive;
- unavailable and degraded statuses.

### 7.5 Pixel engine

Responsibilities:

- 64x64 source sprites;
- integer-scale rendering metadata;
- layers and attachment points;
- animation state machine;
- deterministic simple physics;
- collision and surface configuration;
- monitor boundaries;
- cursor observation;
- preferred positions;
- editor document format.

Physics and animation must not require an LLM.

### 7.6 Resource scheduler

Responsibilities:

- one heavy inference at a time on reference hardware;
- active-chat priority;
- owner-command priority after active chat;
- agent-specific priorities;
- pause and resume;
- model load/unload;
- system-load awareness in later phases;
- clear user-facing status.

## 8. Event model

Rust publishes validated application events to React.

Examples:

- `agent.created`
- `agent.updated`
- `agent.state.changed`
- `agent.animation.changed`
- `conversation.created`
- `message.created`
- `message.streaming`
- `message.completed`
- `memory.candidate.created`
- `memory.saved`
- `model.status.changed`
- `runtime.status.changed`
- `mode.changed`
- `safeMode.entered`
- `safeMode.exited`

Events include versioned payloads and stable identifiers.

## 9. Model-independent context pipeline

Long-term response assembly:

```text
User input
  -> ownership and conversation validation
  -> mode and permission validation
  -> recent conversation window
  -> fixed agent identity
  -> selected relevant memories
  -> fictional state
  -> active goals and relationships
  -> tool availability and permissions
  -> model adapter
  -> structured response validation
  -> UI text and animation event
  -> memory-candidate extraction
  -> authoritative persistence
```

v0.1 implements only the relevant subset:

- user input;
- agent identity and starting traits;
- recent conversation;
- initial memory retrieval;
- sleep, energy, and mood;
- selected model;
- response;
- automatic memory candidate.

## 10. Queueing

Inference queue item fields include:

- request ID;
- agent ID;
- conversation ID;
- priority class;
- selected model;
- created time;
- cancellation state;
- streaming state;
- resource estimate when available.

Initial priority order:

1. active user conversation;
2. active user command;
3. required memory extraction or summarization tied to that conversation;
4. background maintenance.

Autonomous agent conversations and goals are later priorities.

## 11. Safe mode

Safe mode must be owned by Rust and not depend on Python or Ollama.

Safe mode behavior:

- do not start or use model runtime;
- do not display agent overlay windows;
- disable extensions and autonomous work;
- keep main administrative window available;
- keep history and settings readable;
- allow diagnostics and recovery;
- allow model/runtime configuration repair;
- never silently delete user data.

Startup may offer safe mode after repeated runtime or migration failures.

## 12. Temporary chat architecture

Temporary chat state lives only in process memory.

Requirements:

- no database row for messages;
- no summary persistence;
- no memory candidate persistence;
- no telemetry containing content;
- no crash dump containing content when avoidable;
- no autosave file;
- no external state-changing tool;
- session disappears when closed or process exits.

The UI must visibly distinguish temporary chat.

## 13. Pixel-art document format

The editor stores source documents separately from rendered PNGs.

Suggested versioned format:

```json
{
  "schemaVersion": 1,
  "canvas": { "width": 64, "height": 64 },
  "palette": ["#00000000", "#1A1A1AFF"],
  "layers": [],
  "attachmentPoints": {},
  "animations": {}
}
```

Normal pixels are either fully transparent or fully opaque.

Generated PNG previews are derived artifacts. Source layer data remains authoritative.

## 14. Testing strategy

### Unit tests

- agent state transitions;
- mode transitions;
- queue ordering;
- model selection resolution;
- memory candidate rules;
- temporary chat non-persistence;
- pixel attachment and collision logic;
- path validation.

### Integration tests

- Rust starts and stops a fake Python runtime;
- runtime crash leaves UI state available;
- SQLite migrations and restart persistence;
- model unavailable flow;
- safe mode startup;
- full-screen hide behavior where testable;
- two-agent separation.

### UI tests

- creation flow;
- bubble expansion;
- model unavailable status;
- multiple chats;
- temporary-chat indicators;
- basic editor tools.

### Manual Windows validation

- transparent overlay;
- click-through;
- always-on-top;
- F11/full-screen hiding;
- multi-monitor behavior;
- drag, gravity, and collision;
- Tauri packaging on Windows 10.

## 15. Future integration boundary

A later AIP gateway may expose authenticated local APIs for BielOS and Android.

It must be added as a separate boundary, not by exposing internal Rust commands or the Python runtime directly.

Future integration uses:

- versioned API contracts;
- authenticated sessions;
- explicit ownership;
- read/write permission separation;
- Cloudflare infrastructure only at the remote-access layer;
- local model and primary data on the PC.

No future integration code belongs in Phase 0 or the first vertical slice.