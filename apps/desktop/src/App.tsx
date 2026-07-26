import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AppSnapshot,
  AgentMemory,
  AgentSimulatedState,
  ConversationMessage,
  PhaseOneConversation,
  ProvisionalAgent,
} from "@aip/contracts";
import AgentSprite from "./components/AgentSprite";
import {
  blockedSendCopy,
  canRequestCancellation,
  messageFailureCopy,
  messageStatusCopy,
  providerStatusCopy,
  requestForAgent,
} from "./conversation-state";
import {
  isNearConversationBottom,
  shouldScrollConversationToBottom,
} from "./conversation-scroll";
import { usePhaseOne } from "./use-phase-one";
import { createListenerRegistration } from "./listener-lifecycle";
import "./App.css";

const runtimeLabels: Record<AppSnapshot["runtime"]["state"], string> = {
  stopped: "Runtime parado",
  starting: "Runtime iniciando",
  ready: "Runtime local pronto",
  unavailable: "Runtime de IA indisponível",
  crashed: "Runtime interrompido",
  safe_mode: "Runtime desativado pelo modo seguro",
};

function AgentButton({
  agent,
  active,
  onSelect,
}: {
  agent: ProvisionalAgent;
  active: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      className={active ? "agent-tab active" : "agent-tab"}
      type="button"
      onClick={onSelect}
    >
      <AgentSprite spriteKey={agent.spriteKey} name={agent.name} />
      <span>{agent.name}</span>
    </button>
  );
}

function MessageItem({ message }: { message: ConversationMessage }) {
  return (
    <article
      className={`chat-message ${message.author}`}
      data-status={message.status}
    >
      <div className="message-heading">
        <strong>{message.author === "user" ? "Você" : "Agente"}</strong>
        <span>{messageStatusCopy(message)}</span>
      </div>
      {message.content ? <p>{message.content}</p> : null}
      {message.status === "failed" ? (
        <small>{messageFailureCopy(message.errorCode)}</small>
      ) : null}
    </article>
  );
}

function ConversationSurface({ agentId, temporary }: { agentId: string; temporary: boolean }) {
  const { phase, error, load } = usePhaseOne(agentId, temporary);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [cancellingRequestId, setCancellingRequestId] = useState<string | null>(
    null,
  );
  const historyRef = useRef<HTMLDivElement>(null);
  const conversationIdRef = useRef<string | null>(null);
  const followsBottomRef = useRef(true);

  useLayoutEffect(() => {
    const history = historyRef.current;
    const conversationId = phase?.conversation.id;
    if (history === null || conversationId === undefined) return;
    const conversationChanged = conversationIdRef.current !== conversationId;
    if (
      shouldScrollConversationToBottom(
        conversationChanged,
        followsBottomRef.current,
      )
    ) {
      history.scrollTop = history.scrollHeight;
      followsBottomRef.current = true;
    }
    conversationIdRef.current = conversationId;
  }, [phase?.conversation.id, phase?.messages]);

  if (error) {
    return (
      <section className="conversation-empty" role="alert">
        <p>Não foi possível carregar a conversa local.</p>
        <button type="button" onClick={() => void load()}>
          Tentar novamente
        </button>
      </section>
    );
  }
  if (phase === null)
    return (
      <section className="conversation-empty">Carregando conversa…</section>
    );

  const currentPhase = phase;
  const request = requestForAgent(phase.queue, phase.agent.id);
  const blocked = blockedSendCopy(phase.sendBlockedCode);

  async function send() {
    const content = draft.trim();
    if (!content || busy || !currentPhase.canSend) return;
    followsBottomRef.current = true;
    setBusy(true);
    try {
      await invoke(temporary ? "send_temporary_phase_one_message" : "send_phase_one_message", temporary
        ? { agentId: currentPhase.agent.id, content }
        : { agentId: currentPhase.agent.id, conversationId: currentPhase.conversation.id, content });
      setDraft("");
      void load();
    } finally {
      setBusy(false);
    }
  }

  async function refreshModels() {
    await invoke("refresh_ollama_models").catch(() => null);
    await load();
  }

  async function cancelCurrentRequest() {
    if (
      request === null ||
      !canRequestCancellation(request, cancellingRequestId)
    )
      return;
    setCancellingRequestId(request.requestId);
    try {
      await invoke("cancel_phase_one_generation", {
        requestId: request.requestId,
      });
      void load();
    } finally {
      setCancellingRequestId(null);
    }
  }

  return (
    <section
      className="conversation-surface"
      aria-label={`Conversa com ${phase.agent.name}`}
    >
      <header className="conversation-header">
        <div>
          <p className="eyebrow">{temporary ? "Conversa temporária" : "Conversa principal"}</p>
          <h1>{phase.agent.name}</h1>
          <span className={`provider-state ${phase.provider.state}`}>
            {providerStatusCopy(phase)}
          </span>
        </div>
        <div className="conversation-controls">
          <label>
            <span>Modelo local</span>
            <select
              value={phase.selectedModelRef ?? ""}
              onChange={(event) => {
                if (event.target.value) {
                  void invoke("select_phase_one_model", {
                    agentId: currentPhase.agent.id,
                    modelRef: event.target.value,
                  }).then(load);
                }
              }}
            >
              <option value="">Selecione um modelo</option>
              {phase.provider.models.map((model) => (
                <option value={model.ref} key={model.ref}>
                  {model.displayName}
                  {model.parameterSize ? ` · ${model.parameterSize}` : ""}
                  {model.quantization ? ` · ${model.quantization}` : ""}
                </option>
              ))}
              {phase.selectedModelRef !== null &&
              !phase.selectedModelAvailable ? (
                <option value={phase.selectedModelRef}>
                  Modelo salvo (indisponível)
                </option>
              ) : null}
            </select>
          </label>
          {!temporary ? <label>
            <span>Modelo desta conversa</span>
            <select value={phase.modelOverrideRef ?? ""} onChange={(event) => void invoke("set_main_conversation_model_override", { agentId: currentPhase.agent.id, modelRef: event.target.value || null }).then(load)}>
              <option value="">Usar modelo padrão do agente</option>
              {phase.provider.models.map((model) => <option value={model.ref} key={`override-${model.ref}`}>{model.displayName}</option>)}
              {phase.modelOverrideRef !== null && !phase.selectedModelAvailable ? <option value={phase.modelOverrideRef}>Modelo salvo (indisponível)</option> : null}
            </select>
          </label> : null}
          <label>
            <span>Manter modelo carregado</span>
            <select
              value={phase.keepAliveMinutes}
              onChange={(event) =>
                void invoke("update_keep_alive", {
                  agentId: currentPhase.agent.id,
                  minutes: Number(event.target.value),
                }).then(load)
              }
            >
              <option value={0}>Descarregar imediatamente</option>
              <option value={5}>5 minutos</option>
              <option value={15}>15 minutos</option>
              <option value={30}>30 minutos</option>
              <option value={60}>60 minutos</option>
              <option value={120}>120 minutos</option>
            </select>
          </label>
          <button type="button" onClick={() => void refreshModels()}>
            Atualizar modelos
          </button>
        </div>
      </header>

      <div
        className="message-history"
        ref={historyRef}
        aria-live="polite"
        onScroll={() => {
          const history = historyRef.current;
          if (history !== null) {
            followsBottomRef.current = isNearConversationBottom(history);
          }
        }}
      >
        {phase.messages.length === 0 ? (
          <div className="history-placeholder">
            <strong>Esta conversa ainda está vazia.</strong>
            <span>
              Escolha um modelo local e envie uma mensagem para começar.
            </span>
          </div>
        ) : (
          phase.messages.map((message) => (
            <MessageItem key={message.id} message={message} />
          ))
        )}
      </div>

      <footer className="composer">
        {request !== null ? (
          <div className="queue-banner">
            <span>
              {request.active
                ? request.cancellationRequested
                  ? "Cancelando resposta…"
                  : "Gerando resposta…"
                : "Aguardando processamento…"}
            </span>
            <button
              type="button"
              disabled={!canRequestCancellation(request, cancellingRequestId)}
              onClick={() => void cancelCurrentRequest()}
            >
              {request.cancellationRequested ||
              cancellingRequestId === request.requestId
                ? "Cancelando…"
                : "Cancelar"}
            </button>
          </div>
        ) : null}
        <textarea
          value={draft}
          maxLength={16_384}
          placeholder={blocked ?? `Escreva para ${phase.agent.name}`}
          disabled={!phase.canSend || busy}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              void send();
            }
          }}
        />
        <div className="composer-footer">
          <span>{blocked ?? "Enter envia · Shift+Enter cria uma linha"}</span>
          <button
            type="button"
            disabled={!phase.canSend || !draft.trim() || busy}
            onClick={() => void send()}
          >
            Enviar
          </button>
        </div>
      </footer>
    </section>
  );
}

function ProfileFields({ draft, onChange }: { draft: ProvisionalAgent; onChange: (next: ProvisionalAgent) => void }) {
  return <>
    <label>Nome<input value={draft.name} onChange={(event) => onChange({ ...draft, name: event.target.value })} /></label>
    <label>Data de aniversário<input type="date" value={draft.birthday} onChange={(event) => onChange({ ...draft, birthday: event.target.value })} /></label>
    <label>Idade fictícia<input type="number" min="0" max="10000" value={draft.fictiveAge} onChange={(event) => onChange({ ...draft, fictiveAge: Number(event.target.value) })} /></label>
    <label>Categoria de idade<input value={draft.ageCategory} onChange={(event) => onChange({ ...draft, ageCategory: event.target.value })} /></label>
    <label>Espécie<input value={draft.species} onChange={(event) => onChange({ ...draft, species: event.target.value })} /></label>
    <label>Pronomes<input value={draft.pronouns} onChange={(event) => onChange({ ...draft, pronouns: event.target.value })} /></label>
    <label>Descrição<input value={draft.personalitySummary} onChange={(event) => onChange({ ...draft, personalitySummary: event.target.value })} /></label>
    <label>Traços (JSON)<input value={draft.traitsJson} onChange={(event) => onChange({ ...draft, traitsJson: event.target.value })} /></label>
  </>;
}

function ProfileForm({ agent, done }: { agent: ProvisionalAgent; done: () => void }) {
  const [draft, setDraft] = useState(agent);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => setDraft(agent), [agent]);
  async function save() {
    if (!draft.name.trim() || !draft.birthday || !draft.species.trim() || !draft.pronouns.trim()) {
      setError("Preencha nome, data, espécie e pronomes."); return;
    }
    try {
      await invoke("update_agent_profile", { agent: draft });
      done();
    } catch { setError("Não foi possível salvar o perfil."); }
  }
  return <section className="conversation-empty" aria-label="Perfil do agente">
    <h1>{`Perfil de ${agent.name}`}</h1>
    <ProfileFields draft={draft} onChange={setDraft} />
    {error ? <p role="alert">{error}</p> : null}<button type="button" onClick={() => void save()}>Salvar perfil</button>
  </section>;
}

function OnboardingForm({ agents, done }: { agents: ProvisionalAgent[]; done: () => void }) {
  const [drafts, setDrafts] = useState(agents);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => setDrafts(agents), [agents]);
  const update = (index: number, next: ProvisionalAgent) => setDrafts((current) => current.map((agent, currentIndex) => currentIndex === index ? next : agent));
  async function save() {
    if (drafts.length !== 2 || drafts.some((agent) => !agent.name.trim() || !agent.birthday || !agent.ageCategory.trim() || !agent.species.trim() || !agent.pronouns.trim())) {
      setError("Preencha os campos obrigatórios dos dois agentes."); return;
    }
    try {
      await invoke("complete_phase_two_onboarding", { agents: drafts });
      done();
    } catch { setError("Não foi possível concluir a criação dos perfis."); }
  }
  return <section className="conversation-empty" aria-label="Criação dos perfis">
    <h1>Crie os dois perfis</h1>
    {drafts.map((agent, index) => <fieldset key={agent.id}><legend>{agent.name}</legend><ProfileFields draft={agent} onChange={(next) => update(index, next)} /></fieldset>)}
    {error ? <p role="alert">{error}</p> : null}<button type="button" onClick={() => void save()}>Concluir criação</button>
  </section>;
}

function ConversationList({ agentId, changed }: { agentId: string; changed: () => void }) {
  const [items, setItems] = useState<PhaseOneConversation[]>([]);
  const [title, setTitle] = useState("");
  const load = useCallback(() => void invoke<PhaseOneConversation[]>("list_agent_conversations", { agentId }).then(setItems), [agentId]);
  useEffect(() => { load(); }, [load]);
  async function create() {
    if (!title.trim()) return;
    await invoke("create_agent_conversation", { agentId, title });
    setTitle(""); load();
  }
  async function select(conversationId: string) {
    await invoke("set_active_agent_conversation", { agentId, conversationId });
    changed();
  }
  return <div className="conversation-list" aria-label="Conversas do agente">
    {items.map((item) => <div key={item.id}><button type="button" onClick={() => void select(item.id)}>{item.title}{item.id === items[0]?.id ? " · Principal" : ""}</button>{item.id !== items[0]?.id ? <button type="button" onClick={() => void invoke("archive_agent_conversation", { agentId, conversationId: item.id }).then(load)}>Arquivar</button> : null}</div>)}
    <input value={title} placeholder="Nova conversa" onChange={(event) => setTitle(event.target.value)} />
    <button type="button" onClick={() => void create()}>Criar conversa</button>
  </div>;
}

function MemoryList({ agentId }: { agentId: string }) {
  const [items, setItems] = useState<AgentMemory[]>([]);
  const [content, setContent] = useState("");
  const [category, setCategory] = useState("preference");
  const load = useCallback(() => void invoke<AgentMemory[]>("list_agent_memories", { agentId }).then(setItems), [agentId]);
  useEffect(() => { load(); }, [load]);
  async function save(confirmed = true) {
    if (!content.trim()) return;
    await invoke("create_agent_memory", { agentId, category, content, confirmed });
    setContent(""); load();
  }
  return <div className="memory-list" aria-label="Memórias do agente">
    <strong>Memórias</strong>
    {items.filter((item) => item.status === "active").map((item) => <p key={item.id}><small>{item.category}</small> {item.content}</p>)}
    <input value={category} onChange={(event) => setCategory(event.target.value)} aria-label="Categoria da memória" />
    <input value={content} onChange={(event) => setContent(event.target.value)} placeholder="Nova memória" />
    <button type="button" onClick={() => void save()}>Salvar memória</button>
    <button type="button" onClick={() => void save(false)}>
      Propor memÃ³ria
    </button>
  </div>;
}

function AgentStateControls({ agentId }: { agentId: string }) {
  const [state, setState] = useState<AgentSimulatedState | null>(null);
  const [saving, setSaving] = useState(false);
  const load = useCallback(
    () =>
      void invoke<AgentSimulatedState>("get_agent_simulated_state", { agentId })
        .then(setState)
        .catch(() => setState(null)),
    [agentId],
  );

  useEffect(() => {
    load();
  }, [load]);

  async function update(action: () => Promise<unknown>) {
    if (saving) return;
    setSaving(true);
    try {
      await action();
      load();
    } finally {
      setSaving(false);
    }
  }

  if (state === null) return null;
  return (
    <section className="agent-state-controls" aria-label="Estado do agente">
      <strong>Estado</strong>
      <label>
        Modo
        <select
          value={state.mode}
          disabled={saving}
          onChange={(event) =>
            void update(() =>
              invoke("set_agent_simulated_mode", {
                agentId,
                mode: event.target.value,
              }),
            )
          }
        >
          <option value="normal">Normal</option>
          <option value="voice_muted">Sem voz</option>
          <option value="silent">Silencioso</option>
          <option value="safe">Seguro</option>
        </select>
      </label>
      <small>
        Energia {state.energy}% · Humor {state.mood}% · Sono {state.sleep}%
      </small>
      <button
        type="button"
        disabled={saving}
        onClick={() =>
          void update(() =>
            invoke("set_agent_suspension", {
              agentId,
              suspended: !state.suspended,
            }),
          )
        }
      >
        {state.suspended ? "Retomar agente" : "Suspender agente"}
      </button>
      <button
        type="button"
        disabled={saving || !state.suspended}
        onClick={() => void update(() => invoke("wake_agent_now", { agentId }))}
      >
        Acordar agora
      </button>
    </section>
  );
}

function PixelDocumentEditor({ agentId }: { agentId: string }) {
  const [source, setSource] = useState("");
  const [error, setError] = useState<string | null>(null);
  useEffect(() => { void invoke<string>("load_pixel_document", { agentId }).then(setSource).catch(() => setError("Não foi possível abrir a arte.")); }, [agentId]);
  async function save() {
    try { await invoke("save_pixel_document", { agentId, sourceJson: source }); setError(null); }
    catch { setError("A arte precisa ter camadas e pontos de encaixe válidos."); }
  }
  return <details className="pixel-editor"><summary>Editor de pixel art (64×64)</summary><textarea value={source} onChange={(event) => setSource(event.target.value)} aria-label="Documento de pixel art" /><button type="button" onClick={() => void save()}>Salvar arte</button>{error ? <p role="alert">{error}</p> : null}</details>;
}

function App() {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [activeAgentId, setActiveAgentId] = useState<string | null>(null);
  const [changingMode, setChangingMode] = useState(false);
  const [editingAgentId, setEditingAgentId] = useState<string | null>(null);
  const [conversationRevision, setConversationRevision] = useState(0);
  const [temporaryChat, setTemporaryChat] = useState(false);

  const loadSnapshot = useCallback(async () => {
    const next = await invoke<AppSnapshot>("get_app_snapshot");
    setSnapshot(next);
    setActiveAgentId((current) => current ?? next.agents[0]?.id ?? null);
  }, []);

  useEffect(() => {
    void loadSnapshot();
    const timer = window.setInterval(() => void loadSnapshot(), 1500);
    return () => window.clearInterval(timer);
  }, [loadSnapshot]);

  useEffect(() => {
    const registration = createListenerRegistration();
    void listen<string>("open-conversation", (event) =>
      setActiveAgentId(event.payload),
    ).then(registration.register);
    return registration.dispose;
  }, []);

  async function toggleSafeMode() {
    if (!snapshot || changingMode) return;
    setChangingMode(true);
    try {
      setSnapshot(
        await invoke<AppSnapshot>("set_safe_mode", {
          enabled: !snapshot.safeMode,
        }),
      );
    } finally {
      setChangingMode(false);
    }
  }

  return (
    <div className="app-shell conversation-layout">
      <aside className="sidebar" aria-label="Navegação principal">
        <div className="brand-mark" aria-label="A.I.P.">
          <span className="brand-glyph">AI</span>
          <div>
            <strong>A.I.P.</strong>
            <small>Conversa local</small>
          </div>
        </div>
        <p className="sidebar-label">Conversas</p>
        <div className="agent-tabs">
          {snapshot?.agents.map((agent) => (
            <AgentButton
              key={agent.id}
              agent={agent}
              active={agent.id === activeAgentId}
              onSelect={() => setActiveAgentId(agent.id)}
            />
          ))}
        </div>
        {activeAgentId ? <ConversationList agentId={activeAgentId} changed={() => setConversationRevision((value) => value + 1)} /> : null}
        {activeAgentId ? <button type="button" onClick={() => setTemporaryChat((current) => !current)}>{temporaryChat ? "Voltar à conversa salva" : "Abrir conversa temporária"}</button> : null}
        {activeAgentId ? <MemoryList agentId={activeAgentId} /> : null}
        {activeAgentId ? <AgentStateControls agentId={activeAgentId} /> : null}
        {activeAgentId ? <PixelDocumentEditor agentId={activeAgentId} /> : null}
        {snapshot?.agents.map((agent) => <button key={`profile-${agent.id}`} type="button" onClick={() => setEditingAgentId(agent.id)}>Perfil de {agent.name}</button>)}
        <button
          className={snapshot?.safeMode ? "mode-button active" : "mode-button"}
          type="button"
          disabled={!snapshot || changingMode}
          onClick={() => void toggleSafeMode()}
        >
          {snapshot?.safeMode ? "Sair do modo seguro" : "Ativar modo seguro"}
        </button>
        <div className="sidebar-footer">
          <span className="local-dot" aria-hidden="true" />
          {snapshot
            ? runtimeLabels[snapshot.runtime.state]
            : "Verificando runtime"}
        </div>
      </aside>
      <main className="conversation-main">
        {snapshot?.runtime.state === "unavailable" ||
        snapshot?.runtime.state === "crashed" ? (
          <div className="runtime-banner" role="status">
            <span>
              {runtimeLabels[snapshot.runtime.state]}. O histórico continua
              disponível.
            </span>
            <button
              type="button"
              onClick={() => void invoke("retry_phase_one_runtime")}
            >
              Tentar novamente
            </button>
          </div>
        ) : null}
        {snapshot?.onboardingRequired && snapshot.agents.length === 2 ? (
          <OnboardingForm agents={snapshot.agents} done={() => { setEditingAgentId(null); void loadSnapshot(); }} />
        ) : editingAgentId !== null && snapshot?.agents.find((agent) => agent.id === editingAgentId) ? (
          <ProfileForm agent={snapshot.agents.find((agent) => agent.id === editingAgentId)!} done={() => { setEditingAgentId(null); void loadSnapshot(); }} />
        ) : activeAgentId === null ? (
          <section className="conversation-empty">Carregando agentes…</section>
        ) : (
          <ConversationSurface key={`${activeAgentId}-${conversationRevision}-${temporaryChat}`} agentId={activeAgentId} temporary={temporaryChat} />
        )}
      </main>
    </div>
  );
}

export default App;
