import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AgentAnimationState, AppSnapshot } from "@aip/contracts";
import AgentSprite from "./components/AgentSprite";
import "./App.css";

export default function Overlay({ agentId }: { agentId: string }) {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [animation, setAnimation] = useState<AgentAnimationState>("idle");
  const agent = useMemo(
    () =>
      snapshot?.agents.find((candidate) => candidate.id === agentId) ?? null,
    [agentId, snapshot],
  );

  useEffect(() => {
    const refresh = () =>
      void invoke<AppSnapshot>("get_app_snapshot")
        .then(setSnapshot)
        .catch(() => null);
    refresh();
    const timer = window.setInterval(refresh, 1000);
    return () => window.clearInterval(timer);
  }, []);

  async function startDrag() {
    setAnimation("dragged");
    try {
      await invoke("start_overlay_drag", { agentId });
    } finally {
      window.setTimeout(() => setAnimation("idle"), 180);
    }
  }

  function showThinkingState() {
    setAnimation("thinking");
    window.setTimeout(() => setAnimation("idle"), 1200);
  }

  if (!agent || snapshot?.safeMode) return null;

  return (
    <main className="overlay-stage" data-animation={animation}>
      <button
        className="overlay-agent"
        type="button"
        aria-label={`${agent.name}, agente provisório. Arraste para mover.`}
        onPointerDown={() => void startDrag()}
        onDoubleClick={showThinkingState}
      >
        <AgentSprite spriteKey={agent.spriteKey} name={agent.name} />
        <span className="overlay-label">{agent.name}</span>
        {animation === "thinking" ? (
          <span
            className="thought-indicator"
            aria-label="Estado de pensamento provisório"
          >
            ···
          </span>
        ) : null}
      </button>
    </main>
  );
}
