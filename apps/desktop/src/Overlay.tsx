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
} from "./overlay-gesture";
import {
  buildInteractiveRegions,
  elementBounds,
  readSpriteMask,
  type SpriteMask,
} from "./overlay-input";
import { requestForAgent } from "./conversation-state";
import { usePhaseOne } from "./use-phase-one";
import "./App.css";

export default function Overlay({ agentId }: { agentId: string }) {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [dragging, setDragging] = useState(false);
  const [spriteMask, setSpriteMask] = useState<SpriteMask | null>(null);
  const { phase } = usePhaseOne(agentId);
  const spriteRef = useRef<HTMLImageElement>(null);
  const labelRef = useRef<HTMLSpanElement>(null);
  const thoughtRef = useRef<HTMLSpanElement>(null);
  const gestureRef = useRef(initialOverlayGestureState);
  const agent = useMemo(
    () =>
      snapshot?.agents.find((candidate) => candidate.id === agentId) ?? null,
    [agentId, snapshot],
  );
  const request = phase === null ? null : requestForAgent(phase.queue, agentId);
  const thinking = request?.active === true;
  const animation: AgentAnimationState = dragging
    ? "dragged"
    : thinking
      ? "thinking"
      : "idle";
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
  }, [agentId, overlayActive, spriteMask, thinking]);

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
      void invoke("set_overlay_interactive_regions", {
        agentId,
        regions: [],
      }).catch(() => null);
    },
    [agentId],
  );

  async function startDrag(button: HTMLButtonElement, pointerId: number) {
    if (button.hasPointerCapture(pointerId))
      button.releasePointerCapture(pointerId);
    setDragging(true);
    try {
      await invoke("start_overlay_drag", { agentId });
    } finally {
      gestureRef.current = cancelGesture(gestureRef.current);
      setDragging(false);
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
    if (result.action === "start_drag")
      void startDrag(event.currentTarget, event.pointerId);
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
    if (result.action === "click") {
      void invoke("set_overlay_bubble_visible", { agentId, visible: true });
    } else if (result.action === "double_click") {
      void invoke("open_main_conversation", { agentId });
    }
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
        aria-label={`${agent.name}. Clique para conversar ou arraste para mover.`}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={handlePointerCancel}
      >
        <AgentSprite
          ref={spriteRef}
          agentId={agent.id}
          spriteKey={agent.spriteKey}
          name={agent.name}
          onLoad={(image) => setSpriteMask(readSpriteMask(image))}
        />
        <span ref={labelRef} className="overlay-label">
          {agent.name}
        </span>
        {thinking ? (
          <span
            ref={thoughtRef}
            className="thought-indicator"
            aria-label="Gerando resposta"
          >
            ···
          </span>
        ) : null}
      </button>
    </main>
  );
}
