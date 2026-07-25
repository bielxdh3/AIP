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
  ConversationMessage,
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

function ConversationSurface({ agentId }: { agentId: string }) {
  const { phase, error, load } = usePhaseOne(agentId);
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
      await invoke("send_phase_one_message", {
        agentId: currentPhase.agent.id,
        conversationId: currentPhase.conversation.id,
        content,
      });
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
          <p className="eyebrow">Conversa principal</p>
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
          <label>
            <span>Modelo desta conversa</span>
            <select value={phase.modelOverrideRef ?? ""} onChange={(event) => void invoke("set_main_conversation_model_override", { agentId: currentPhase.agent.id, modelRef: event.target.value || null }).then(load)}>
              <option value="">Usar modelo padrão do agente</option>
              {phase.provider.models.map((model) => <option value={model.ref} key={`override-${model.ref}`}>{model.displayName}</option>)}
              {phase.modelOverrideRef !== null && !phase.selectedModelAvailable ? <option value={phase.modelOverrideRef}>Modelo salvo (indisponível)</option> : null}
            </select>
          </label>
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

function App() {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [activeAgentId, setActiveAgentId] = useState<string | null>(null);
  const [changingMode, setChangingMode] = useState(false);
  const [editingAgentId, setEditingAgentId] = useState<string | null>(null);

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
          <ConversationSurface agentId={activeAgentId} />
        )}
      </main>
    </div>
  );
}

export default App;
