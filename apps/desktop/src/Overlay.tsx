import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  AgentAnimationState,
  AppSnapshot,
  OverlayInteractiveRegion,
} from "@aip/contracts";
import AgentSprite from "./components/AgentSprite";
import "./App.css";

export default function Overlay({ agentId }: { agentId: string }) {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [animation, setAnimation] = useState<AgentAnimationState>("idle");
  const [spriteLoaded, setSpriteLoaded] = useState(false);
  const spriteRef = useRef<HTMLImageElement>(null);
  const labelRef = useRef<HTMLSpanElement>(null);
  const thoughtRef = useRef<HTMLSpanElement>(null);
  const agent = useMemo(
    () =>
      snapshot?.agents.find((candidate) => candidate.id === agentId) ?? null,
    [agentId, snapshot],
  );
  const overlayActive = agent !== null && snapshot?.safeMode === false;

  useEffect(() => {
    const refresh = () =>
      void invoke<AppSnapshot>("get_app_snapshot")
        .then(setSnapshot)
        .catch(() => null);
    refresh();
    const timer = window.setInterval(refresh, 1000);
    return () => window.clearInterval(timer);
  }, []);

  const reportInteractiveRegions = useCallback(() => {
    const regions = overlayActive
      ? [spriteRef.current, labelRef.current, thoughtRef.current]
          .map(toInteractiveRegion)
          .filter(
            (region): region is OverlayInteractiveRegion => region !== null,
          )
      : [];

    void invoke("set_overlay_interactive_regions", { agentId, regions }).catch(
      () => null,
    );
  }, [agentId, overlayActive, spriteLoaded, animation]);

  useLayoutEffect(() => {
    let animationFrame: number | null = null;
    const scheduleReport = () => {
      if (animationFrame !== null) window.cancelAnimationFrame(animationFrame);
      animationFrame = window.requestAnimationFrame(() => {
        animationFrame = null;
        reportInteractiveRegions();
      });
    };
    const elements = [
      spriteRef.current,
      labelRef.current,
      thoughtRef.current,
    ].filter((element): element is HTMLElement => element !== null);
    const observer = new ResizeObserver(scheduleReport);
    elements.forEach((element) => observer.observe(element));
    window.addEventListener("resize", scheduleReport);
    scheduleReport();

    return () => {
      if (animationFrame !== null) window.cancelAnimationFrame(animationFrame);
      observer.disconnect();
      window.removeEventListener("resize", scheduleReport);
    };
  }, [reportInteractiveRegions]);

  useEffect(
    () => () => {
      void invoke("set_overlay_interactive_regions", {
        agentId,
        regions: [],
      }).catch(() => null);
    },
    [agentId],
  );

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
        <AgentSprite
          ref={spriteRef}
          spriteKey={agent.spriteKey}
          name={agent.name}
          onLoad={() => setSpriteLoaded(true)}
        />
        <span ref={labelRef} className="overlay-label">
          {agent.name}
        </span>
        {animation === "thinking" ? (
          <span
            ref={thoughtRef}
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

function toInteractiveRegion(
  element: HTMLElement | null,
): OverlayInteractiveRegion | null {
  if (element === null) return null;
  const bounds = element.getBoundingClientRect();
  if (
    ![bounds.x, bounds.y, bounds.width, bounds.height].every(Number.isFinite) ||
    bounds.width <= 0 ||
    bounds.height <= 0
  ) {
    return null;
  }
  return {
    x: bounds.x,
    y: bounds.y,
    width: bounds.width,
    height: bounds.height,
  };
}
