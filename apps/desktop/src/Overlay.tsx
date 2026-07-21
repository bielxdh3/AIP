import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AgentAnimationState, AppSnapshot } from "@aip/contracts";
import AgentSprite from "./components/AgentSprite";
import {
  beginGesture,
  cancelGesture,
  endGesture,
  initialOverlayGestureState,
  moveGesture,
  THOUGHT_DURATION_MS,
} from "./overlay-gesture";
import {
  buildInteractiveRegions,
  elementBounds,
  readSpriteMask,
  type SpriteMask,
} from "./overlay-input";
import "./App.css";

export default function Overlay({ agentId }: { agentId: string }) {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [animation, setAnimation] = useState<AgentAnimationState>("idle");
  const [spriteMask, setSpriteMask] = useState<SpriteMask | null>(null);
  const spriteRef = useRef<HTMLImageElement>(null);
  const labelRef = useRef<HTMLSpanElement>(null);
  const thoughtRef = useRef<HTMLSpanElement>(null);
  const gestureRef = useRef(initialOverlayGestureState);
  const thoughtTimerRef = useRef<number | null>(null);
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
      ? buildInteractiveRegions(
          spriteMask,
          elementBounds(spriteRef.current),
          elementBounds(labelRef.current),
          elementBounds(thoughtRef.current),
        )
      : [];
    void invoke("set_overlay_interactive_regions", { agentId, regions }).catch(
      () => null,
    );
  }, [agentId, overlayActive, spriteMask, animation]);

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
    reportInteractiveRegions();
    return () => {
      if (animationFrame !== null) window.cancelAnimationFrame(animationFrame);
      observer.disconnect();
      window.removeEventListener("resize", scheduleReport);
    };
  }, [reportInteractiveRegions]);

  useEffect(
    () => () => {
      if (thoughtTimerRef.current !== null) {
        window.clearTimeout(thoughtTimerRef.current);
      }
      void invoke("set_overlay_interactive_regions", {
        agentId,
        regions: [],
      }).catch(() => null);
    },
    [agentId],
  );

  function showThinkingState() {
    if (thoughtTimerRef.current !== null) {
      window.clearTimeout(thoughtTimerRef.current);
    }
    setAnimation("thinking");
    thoughtTimerRef.current = window.setTimeout(() => {
      thoughtTimerRef.current = null;
      setAnimation("idle");
    }, THOUGHT_DURATION_MS);
  }

  async function startDrag(button: HTMLButtonElement, pointerId: number) {
    if (thoughtTimerRef.current !== null) {
      window.clearTimeout(thoughtTimerRef.current);
      thoughtTimerRef.current = null;
    }
    if (button.hasPointerCapture(pointerId))
      button.releasePointerCapture(pointerId);
    setAnimation("dragged");
    try {
      await invoke("start_overlay_drag", { agentId });
    } finally {
      gestureRef.current = cancelGesture(gestureRef.current);
      setAnimation("idle");
    }
  }

  function handlePointerDown(event: React.PointerEvent<HTMLButtonElement>) {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    gestureRef.current = beginGesture(
      gestureRef.current,
      event.pointerId,
      event.clientX,
      event.clientY,
    );
  }

  function handlePointerMove(event: React.PointerEvent<HTMLButtonElement>) {
    const result = moveGesture(
      gestureRef.current,
      event.pointerId,
      event.clientX,
      event.clientY,
    );
    gestureRef.current = result.state;
    if (result.action === "start_drag") {
      void startDrag(event.currentTarget, event.pointerId);
    }
  }

  function handlePointerUp(event: React.PointerEvent<HTMLButtonElement>) {
    const result = endGesture(
      gestureRef.current,
      event.pointerId,
      performance.now(),
    );
    gestureRef.current = result.state;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    if (result.action === "thought") showThinkingState();
  }

  function handlePointerCancel(event: React.PointerEvent<HTMLButtonElement>) {
    gestureRef.current = cancelGesture(gestureRef.current);
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  }

  if (!agent || snapshot?.safeMode) return null;

  return (
    <main className="overlay-stage" data-animation={animation}>
      <button
        className="overlay-agent"
        type="button"
        aria-label={`${agent.name}, agente provisório. Arraste para mover.`}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={handlePointerCancel}
      >
        <AgentSprite
          ref={spriteRef}
          spriteKey={agent.spriteKey}
          name={agent.name}
          onLoad={(image) => setSpriteMask(readSpriteMask(image))}
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
