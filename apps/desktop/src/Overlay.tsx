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
import AgentSprite, { type PixelOverlay } from "./components/AgentSprite";
import {
  beginGesture,
  cancelGesture,
  endGesture,
  initialOverlayGestureState,
  moveGesture,
} from "./overlay-gesture";
import { pointerDelta, type PointerPoint } from "./overlay-drag";
import {
  buildInteractiveRegions,
  elementBounds,
  readSpriteMask,
  type SpriteMask,
} from "./overlay-input";
import { requestForAgent } from "./conversation-state";
import { openAgentConversations } from "./agent-navigation";
import { usePhaseOne } from "./use-phase-one";
import "./App.css";

export default function Overlay({ agentId }: { agentId: string }) {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [dragging, setDragging] = useState(false);
  const [spriteMask, setSpriteMask] = useState<SpriteMask | null>(null);
  const [customPixels, setCustomPixels] = useState<PixelOverlay[]>([]);
  const { phase } = usePhaseOne(agentId);
  const spriteRef = useRef<HTMLImageElement>(null);
  const labelRef = useRef<HTMLSpanElement>(null);
  const thoughtRef = useRef<HTMLSpanElement>(null);
  const gestureRef = useRef(initialOverlayGestureState);
  const lastDragPointRef = useRef<PointerPoint | null>(null);
  const dragCommandRef = useRef<Promise<unknown>>(Promise.resolve());
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
          customPixels,
        )
      : [];
    void invoke("set_overlay_interactive_regions", { agentId, regions }).catch(
      () => null,
    );
  }, [agentId, customPixels, overlayActive, spriteMask]);

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
    return () => {
      if (animationFrame !== null) window.cancelAnimationFrame(animationFrame);
      observer.disconnect();
      window.removeEventListener("resize", scheduleReport);
    };
  }, [reportInteractiveRegions]);

  useLayoutEffect(() => {
    reportInteractiveRegions();
  }, [reportInteractiveRegions, thinking]);

  useEffect(
    () => () => {
      void invoke("set_overlay_interactive_regions", {
        agentId,
        regions: [],
      }).catch(() => null);
    },
    [agentId],
  );

  function handlePointerDown(event: React.PointerEvent<HTMLButtonElement>) {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    lastDragPointRef.current = { x: event.clientX, y: event.clientY };
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
    if (result.action === "start_drag") setDragging(true);
    if (!result.state.dragging || result.state.pointerId !== event.pointerId) {
      return;
    }
    const current = { x: event.clientX, y: event.clientY };
    const delta = pointerDelta(lastDragPointRef.current, current);
    lastDragPointRef.current = current;
    if (delta === null || (delta.x === 0 && delta.y === 0)) return;
    dragCommandRef.current = dragCommandRef.current
      .catch(() => undefined)
      .then(() =>
        invoke("move_overlay", {
          agentId,
          deltaX: delta.x,
          deltaY: delta.y,
        }),
      )
      .catch(() => null);
  }

  function handlePointerUp(event: React.PointerEvent<HTMLButtonElement>) {
    const ownsPointer = gestureRef.current.pointerId === event.pointerId;
    const result = endGesture(
      gestureRef.current,
      event.pointerId,
      performance.now(),
    );
    gestureRef.current = result.state;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    if (ownsPointer) {
      lastDragPointRef.current = null;
      setDragging(false);
    }
    if (result.action === "click") {
      void invoke("set_overlay_bubble_visible", { agentId, visible: true });
    } else if (result.action === "double_click") {
      void openAgentConversations(agentId);
    }
  }

  function handlePointerCancel(event: React.PointerEvent<HTMLButtonElement>) {
    const ownsPointer = gestureRef.current.pointerId === event.pointerId;
    gestureRef.current = cancelGesture(gestureRef.current);
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    if (ownsPointer) {
      lastDragPointRef.current = null;
      setDragging(false);
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
          onPixelsChange={setCustomPixels}
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
