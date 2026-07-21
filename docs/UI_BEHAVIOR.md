# AIP UI Behavior

## 1. Language and style

- All user-facing interface text is Portuguese.
- Source code, internal identifiers, documentation, and localization keys are English.
- The visual identity uses `AIP` as the product name and may display `A.I.P.` in branding.
- Agent visuals use a clear 8-bit-inspired pixel-art style.
- Placeholder art must be original and clearly replaceable.
- Do not copy the planning-chat reference image or other third-party assets.

## 2. Application surfaces

AIP has three initial desktop surfaces:

1. Main application panel.
2. Transparent agent overlay windows.
3. Expandable speech bubbles attached to agents.

The main application remains usable when overlays are hidden or safe mode is active.

## 3. Main application panel

Initial navigation:

- Início
- Conversas
- Agentes
- Memórias
- Aparência
- Modelos
- Estados
- Configurações
- Diagnóstico

The navigation may be simplified during Phase 0, but routes and component boundaries should not assume a single screen forever.

### 3.1 Home

Shows:

- two agent cards;
- runtime status;
- Ollama/model status when implemented;
- current application mode;
- recent conversations;
- resource summary when available;
- safe mode entry;
- clear degraded-mode warnings.

### 3.2 Conversations

Shows:

- conversation list grouped by agent;
- main conversation marker;
- private conversation marker;
- temporary chat entry point;
- active model and conversation override;
- message history;
- generation status;
- cancel generation;
- create, rename, archive, and select conversations.

### 3.3 Agents

Shows:

- identity;
- owner;
- birthday and age;
- species and pronouns;
- starting traits;
- current state;
- default model;
- appearance preview;
- monitor and position preferences;
- current mode.

v0.1 supports two agents only.

### 3.4 Memories

Shows:

- memory category;
- content;
- source;
- confidence;
- confirmation status;
- date;
- active, archived, or trashed state;
- edit, archive, restore, or delete controls;
- conflict indicators.

Automatic memory candidates must be distinguishable from confirmed facts.

### 3.5 Appearance

Contains:

- layered preview;
- part selection;
- color options;
- animation preview;
- pixel editor;
- attachment-point editor;
- import PNG;
- save and revert.

### 3.6 Models

Shows:

- discovered Ollama models;
- availability;
- non-technical explanation;
- capabilities;
- observed local benchmark data;
- default agent assignment;
- current conversation overrides;
- keep-alive setting;
- advanced endpoint setting.

No model is presented as objectively best. Benchmark data is labeled as local observation.

### 3.7 Diagnostics

Shows:

- application version;
- database migration status;
- Python runtime status;
- protocol version;
- Ollama connectivity;
- model availability;
- sanitized recent errors;
- safe mode controls;
- data paths;
- validation or repair actions when implemented.

Never display secrets or raw private prompt content in diagnostics.

## 4. Agent creation flow

The first-run flow creates exactly two agents.

Steps:

1. Welcome and local-first explanation.
2. Owner initialization without login credentials.
3. First agent identity.
4. First agent traits.
5. First agent appearance.
6. First agent model selection if available.
7. Second agent identity.
8. Second agent traits.
9. Second agent appearance.
10. Second agent model selection if available.
11. Review.
12. Create.

The model step must support:

- no model available;
- skip and configure later;
- short Portuguese explanations such as:
  - `Mais rápido, porém mais limitado.`
  - `Mais lento, mas tende a responder melhor.`
  - `Pode consumir mais memória.`
  - `Não está disponível neste computador.`

The flow must not imply that a model is the identity of the agent.

## 5. Overlay window behavior

### 5.1 Visibility

Default:

- agent is visible after normal startup;
- agent stays above normal application windows;
- agent hides during detected full-screen applications;
- safe mode hides all agent overlays;
- manually hiding an agent does not stop its background state unless silent or suspended policy says otherwise.

Configurable:

- always on top;
- desktop-only mode;
- full-screen hiding;
- visible monitor;
- per-agent or global settings.

### 5.2 Click-through

- Transparent regions pass clicks to the application below.
- Opaque character pixels and active bubble regions accept input.
- Hit testing must use deterministic geometry or alpha masks.
- The overlay must not create a large invisible rectangle that blocks desktop interaction.
- Phase 0 uses a 128 alpha threshold for sprite masks. Decorative shadows do not expand the
  interactive shape.

### 5.3 Mouse actions

- Single click: open or focus speech bubble.
- Double click: open main application on the relevant agent or conversation.
- Right click: open quick menu.
- Drag: move agent.
- Drop: apply configured gravity and surface behavior.

### 5.4 Quick menu

Portuguese items:

- `Conversar`
- `Silenciar`
- `Dormir`
- `Personalizar`
- `Trocar modelo`
- `Ocultar`
- `Abrir painel`

Future actions must respect permissions and modes.

## 6. Motion and physics

Initial deterministic states:

- idle;
- walk;
- dragged;
- fall or settle according to gravity mode;
- sleep;
- talk;
- happy;
- irritated;
- thinking.

Configurable gravity examples:

- disabled;
- settle on nearest valid surface;
- fall to taskbar or screen bottom;
- return to preferred position.

Configurable visual surfaces:

- screen bottom;
- taskbar;
- window edges;
- desktop icons when detectable;
- another agent;
- fixed user-defined zones.

Physics requirements:

- simple and deterministic;
- low CPU cost;
- frame-rate independent updates;
- no LLM required;
- collision response must avoid permanent off-screen trapping;
- user can recover position from settings.

## 7. Screen position preferences

Each agent may develop or use a preferred position.

v0.1 stores and exposes the preference but does not implement deep autonomous preference evolution.

Rules:

- drag updates current position;
- owner may set preferred position;
- agent may return to preferred position according to configuration;
- multi-monitor coordinates are normalized and validated;
- disconnected monitors trigger safe repositioning on an available display;
- position is restored after restart.

## 8. Speech bubbles

### 8.1 Compact state

- attached visually to the agent;
- default maximum preview of three lines;
- configurable line limit;
- does not extend outside screen bounds;
- may show a concise preview of a long response;
- supports short statuses and greetings;
- supports thinking animation.

### 8.2 Expanded state

- shows full message;
- allows scrolling;
- includes text input;
- sends a reply to the current conversation;
- includes open-full-chat control;
- includes cancel-generation control while active;
- may include model status;
- remains anchored or safely detached when space is limited.

### 8.3 Simultaneous bubbles

- both agents may display bubbles simultaneously;
- only one heavy generation is processed at a time on reference hardware;
- queued agent shows a clear waiting or thinking state;
- one agent completing a reply must not close the other agent's bubble.

### 8.4 Status messages

Examples:

- `Modelo indisponível`
- `Carregando modelo…`
- `Aguardando processamento…`
- `Processamento mais lento`
- `Memória registrada`
- `Modo silencioso ativo`
- `Runtime indisponível`
- `Modo seguro ativo`

Statuses must not falsely appear as model-generated dialogue.

## 9. Conversation interface

### 9.1 Conversation list

- grouped or filterable by agent;
- main conversation pinned;
- private indicator;
- archived section;
- temporary chat visually separated;
- last updated time;
- model override indicator.

### 9.2 Message states

- pending;
- streaming;
- complete;
- failed;
- cancelled.

Failure UI shows a concise reason and retry option when safe.

### 9.3 Model selection

At conversation level:

- current model control;
- inherit agent default;
- temporary override;
- simple capability and performance explanation;
- clear unavailable state;
- no automatic identity reset.

## 10. Temporary chat UI

Temporary chat must be visually unmistakable.

Required indicators:

- `Chat temporário`
- `Não será salvo`
- `Não cria memórias`
- `Somente ações de leitura`

On close:

- ask for confirmation only when useful;
- destroy in-memory content;
- return no recent-chat card;
- leave no searchable history.

The interface must not offer state-changing tools in temporary chat.

## 11. Memory UI behavior

When automatic extraction creates a memory candidate:

- optionally show a small status in the bubble;
- do not interrupt every conversation with a modal;
- show candidates in the memory panel;
- distinguish confirmed owner statement from inference;
- allow correction without editing original history;
- retain source link when the source chat exists.

A saved memory message may say `Memória registrada` only when persistence succeeded.

## 12. Modes

### Normal

- normal visual behavior;
- manual conversation;
- later spontaneous behavior.

### Voice-muted

- represented in UI even before voice exists;
- text and visual interaction remain available.

### Silent

- no spontaneous behavior;
- direct text interaction remains available in v0.1;
- later direct voice input receives voice response according to product rules.

### Safe

- overlays disappear;
- model controls disabled or marked unavailable;
- main administrative panel remains available;
- recovery actions are emphasized;
- no decorative animation implies that agents are active.

## 13. Accessibility and usability

- UI scaling must respect Windows scaling.
- Pixel art uses integer scaling inside the agent renderer where possible.
- Main panel text must remain readable at common scaling factors.
- Keyboard navigation is required for main panel forms and chat.
- Motion reduction setting should be planned and may be included early if inexpensive.
- Bubble animation must not prevent reading.
- Color is not the only status indicator.
- Destructive controls require text labels and confirmation in later phases.

## 14. Error behavior

The UI must never silently invent success.

Examples:

- Runtime missing: `Runtime de IA indisponível.`
- Ollama missing: `Ollama não foi encontrado.`
- Model missing: `Modelo indisponível.`
- Database migration failed: enter recovery or safe mode.
- Overlay unsupported behavior: disable the specific option and explain the limitation.
- Full-screen detection uncertain: do not claim it worked without manual validation.

## 15. Phase 0 UI limits

Phase 0 may use:

- a minimal main panel;
- provisional original sprites;
- mocked speech-bubble content;
- fake runtime health responses;
- limited settings;
- basic drag behavior;
- simple safe-mode toggle.

Phase 0 must not fake:

- working Ollama chat;
- saved memory;
- model benchmarks;
- reliable window-edge surfaces;
- full pixel editor;
- autonomous behavior.

Placeholders must be labeled as placeholders in code or documentation, not presented as finished functionality.

## 16. Phase 0 implementation evidence

The implemented main panel exposes only home, agent, and diagnostic anchors. It labels both
agents as provisional, reports runtime degradation, exposes a persisted safe-mode control,
and does not present model, chat, or memory functionality as available.

The overlay uses original 64x64 SVG pixel assets with integer scaling, persisted positions,
transparent windows, always-on-top configuration, and idle/dragged/thinking placeholders.
The failed first hotfix used full sprite rectangles and cursor-polled whole-window input
toggling. The second correction rasterizes sprite alpha into compact logical regions, adds
opaque label and thought rectangles, and installs one native Win32 window region per overlay.
Native drag begins only after pointer movement crosses a small threshold, preserving click
and thought triggering. Runtime commit `a6ccb1badf6aa8a1f317ea1818c247d87f311fe6`
passed the Phase 0 manual Windows 11 checklist at 100% display scaling. The visible thought
indicator is draggable but has no separate Phase 0 button action.
