import { useCallback, useEffect, useRef, useState } from "react";
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
  messageStatusCopy,
  providerStatusCopy,
  requestForAgent,
} from "./conversation-state";
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
        <small>
          O histórico local foi preservado. Tente novamente quando o runtime
          estiver disponível.
        </small>
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

  useEffect(() => {
    const history = historyRef.current;
    if (history !== null) history.scrollTop = history.scrollHeight;
  }, [phase?.messages]);

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
    setBusy(true);
    try {
      await invoke("send_phase_one_message", {
        agentId: currentPhase.agent.id,
        conversationId: currentPhase.conversation.id,
        content,
      });
      setDraft("");
      await load();
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
      await load();
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
            <span>Manter modelo carregado</span>
            <select
              value={phase.keepAliveMinutes}
              onChange={(event) =>
                void invoke("update_keep_alive", {
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

      <div className="message-history" ref={historyRef} aria-live="polite">
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

function App() {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [activeAgentId, setActiveAgentId] = useState<string | null>(null);
  const [changingMode, setChangingMode] = useState(false);

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
        {activeAgentId === null ? (
          <section className="conversation-empty">Carregando agentes…</section>
        ) : (
          <ConversationSurface agentId={activeAgentId} />
        )}
      </main>
    </div>
  );
}

export default App;
