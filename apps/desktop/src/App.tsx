import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppSnapshot, ProvisionalAgent } from "@aip/contracts";
import AgentSprite from "./components/AgentSprite";
import "./App.css";

const runtimeLabels: Record<AppSnapshot["runtime"]["state"], string> = {
  stopped: "Runtime parado",
  starting: "Runtime iniciando",
  ready: "Runtime local pronto",
  unavailable: "Runtime de IA indisponível",
  crashed: "Runtime interrompido",
  safe_mode: "Runtime desativado pelo modo seguro",
};

function AgentCard({ agent }: { agent: ProvisionalAgent }) {
  return (
    <article className="agent-card">
      <div className="agent-portrait" aria-hidden="true">
        <AgentSprite spriteKey={agent.spriteKey} name={agent.name} />
      </div>
      <div className="agent-copy">
        <div className="agent-heading">
          <div>
            <p className="eyebrow">Agente provisório</p>
            <h3>{agent.name}</h3>
          </div>
          <span className="status-pill">Em espera</span>
        </div>
        <p className="agent-description">
          {agent.profileKey === "owner"
            ? "Perfil técnico inicial, separado do runtime de IA."
            : "Perfil expressivo inicial, separado do runtime de IA."}
        </p>
        <dl className="agent-meta">
          <div>
            <dt>Posição</dt>
            <dd>
              {Math.round(agent.position.x)}, {Math.round(agent.position.y)}
            </dd>
          </div>
          <div>
            <dt>Animação</dt>
            <dd>Ocioso</dd>
          </div>
        </dl>
      </div>
    </article>
  );
}

function App() {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [error, setError] = useState(false);
  const [changingMode, setChangingMode] = useState(false);

  const loadSnapshot = useCallback(async () => {
    try {
      const next = await invoke<AppSnapshot>("get_app_snapshot");
      setSnapshot(next);
      setError(false);
    } catch {
      setError(true);
    }
  }, []);

  useEffect(() => {
    void loadSnapshot();
    const timer = window.setInterval(() => void loadSnapshot(), 1500);
    return () => window.clearInterval(timer);
  }, [loadSnapshot]);

  async function toggleSafeMode() {
    if (!snapshot || changingMode) return;
    setChangingMode(true);
    try {
      const next = await invoke<AppSnapshot>("set_safe_mode", {
        enabled: !snapshot.safeMode,
      });
      setSnapshot(next);
      setError(false);
    } catch {
      setError(true);
    } finally {
      setChangingMode(false);
    }
  }

  return (
    <div className="app-shell">
      <aside className="sidebar" aria-label="Navegação principal">
        <div className="brand-mark" aria-label="A.I.P.">
          <span className="brand-glyph">AI</span>
          <div>
            <strong>A.I.P.</strong>
            <small>Fundação local</small>
          </div>
        </div>
        <nav>
          <a className="nav-item active" href="#inicio" aria-current="page">
            <span aria-hidden="true">01</span> Início
          </a>
          <a className="nav-item" href="#agentes">
            <span aria-hidden="true">02</span> Agentes
          </a>
          <a className="nav-item" href="#diagnostico">
            <span aria-hidden="true">03</span> Diagnóstico
          </a>
        </nav>
        <div className="sidebar-footer">
          <span className="local-dot" aria-hidden="true" />
          Execução local
        </div>
      </aside>

      <main className="main-panel" id="inicio">
        <header className="page-header">
          <div>
            <p className="eyebrow">Agentes Independentes Personalizáveis</p>
            <h1>Visão geral</h1>
            <p className="page-summary">
              Shell inicial para dois agentes locais e limites de runtime
              resilientes.
            </p>
          </div>
          <button
            className={
              snapshot?.safeMode ? "mode-button active" : "mode-button"
            }
            type="button"
            disabled={!snapshot || changingMode}
            onClick={() => void toggleSafeMode()}
          >
            <span className="mode-indicator" aria-hidden="true" />
            {snapshot?.safeMode ? "Sair do modo seguro" : "Ativar modo seguro"}
          </button>
        </header>

        {error ? (
          <section className="notice notice-error" role="alert">
            Não foi possível consultar o núcleo local. Reinicie o aplicativo em
            modo seguro.
          </section>
        ) : null}

        {snapshot && snapshot.runtime.state !== "ready" ? (
          <section className="notice" role="status">
            <strong>{runtimeLabels[snapshot.runtime.state]}.</strong>
            <span>
              O painel e os dados locais continuam disponíveis; nenhum modelo
              foi iniciado.
            </span>
          </section>
        ) : null}

        {snapshot && !snapshot.databaseReady ? (
          <section className="notice notice-error" role="alert">
            A base local não pôde ser inicializada. O aplicativo permanece em
            modo seguro.
          </section>
        ) : null}

        <section className="status-grid" aria-label="Estado da fundação">
          <article className="status-card">
            <p>Aplicativo</p>
            <strong>Fase 0</strong>
            <span>Versão {snapshot?.appVersion ?? "0.1.0"}</span>
          </article>
          <article className="status-card">
            <p>Dados locais</p>
            <strong>
              {snapshot
                ? snapshot.databaseReady
                  ? "Disponíveis"
                  : "Indisponíveis"
                : "Verificando"}
            </strong>
            <span>Migração {snapshot?.migrationVersion ?? 0}</span>
          </article>
          <article className="status-card">
            <p>Runtime Python</p>
            <strong>
              {snapshot ? runtimeLabels[snapshot.runtime.state] : "Verificando"}
            </strong>
            <span>Protocolo {snapshot?.runtime.protocolVersion ?? "—"}</span>
          </article>
          <article className="status-card">
            <p>Modo atual</p>
            <strong>{snapshot?.safeMode ? "Seguro" : "Normal"}</strong>
            <span>
              {snapshot?.safeMode ? "Overlays ocultos" : "Overlays ativos"}
            </span>
          </article>
        </section>

        <section className="section-block" id="agentes">
          <div className="section-heading">
            <div>
              <p className="eyebrow">Identidades locais</p>
              <h2>Agentes provisórios</h2>
            </div>
            <span className="section-count">
              {snapshot?.agents.length ?? 0} de 2
            </span>
          </div>
          <div className="agent-grid">
            {snapshot?.agents.map((agent) => (
              <AgentCard agent={agent} key={agent.id} />
            ))}
            {!snapshot ? (
              <div
                className="agent-card skeleton"
                aria-label="Carregando agentes"
              />
            ) : null}
          </div>
        </section>

        <section className="diagnostic-strip" id="diagnostico">
          <div>
            <p className="eyebrow">Diagnóstico</p>
            <h2>Limites ativos</h2>
          </div>
          <ul>
            <li>SQLite sob autoridade do núcleo Rust</li>
            <li>Runtime isolado por entrada e saída gerenciadas</li>
            <li>Nenhum servidor de rede local</li>
            <li>Nenhum modelo ou conversa real nesta fase</li>
          </ul>
        </section>
      </main>
    </div>
  );
}

export default App;
