import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AppSnapshot } from "@aip/contracts";
import {
  blockedSendCopy,
  bubblePresentation,
  canDraftConversationMessage,
  canSendConversationMessage,
  canRequestCancellation,
} from "./conversation-state";
import {
  buildBubbleInteractiveRegions,
  bubbleTargetGeometry,
  elementBounds,
} from "./overlay-input";
import {
  BUBBLE_NATIVE_CLOSE_EVENT,
  BUBBLE_NATIVE_OPEN_EVENT,
  type BubbleNativeLifecyclePayload,
} from "./overlay-events";
import { createListenerRegistration } from "./listener-lifecycle";
import { routingPolicyPayload, useModelPreferences } from "./model-preferences";
import { usePhaseOne } from "./use-phase-one";
import { openAgentConversations } from "./agent-navigation";
import "./App.css";

export default function Bubble({ agentId }: { agentId: string }) {
  const { phase, error, load } = usePhaseOne(agentId);
  const [modelPreferences] = useModelPreferences();
  const [expanded, setExpanded] = useState(false);
  const [minimized, setMinimized] = useState(false);
  const [nativeVisible, setNativeVisible] = useState(true);
  const [draft, setDraft] = useState("");
  const [safeMode, setSafeMode] = useState(false);
  const [cancellingRequestId, setCancellingRequestId] = useState<string | null>(
    null,
  );
  const [sending, setSending] = useState(false);
  const bubbleRef = useRef<HTMLElement>(null);
  const geometryRevision = useRef(0);

  const reportRegion = useCallback(() => {
    const bounds = elementBounds(bubbleRef.current);
    const regions = buildBubbleInteractiveRegions(
      nativeVisible && !safeMode,
      bounds,
    );
    const revision = geometryRevision.current + 1;
    geometryRevision.current = revision;
    const installRegions = () => {
      if (geometryRevision.current !== revision) return;
      void invoke("set_overlay_interactive_regions", {
        agentId,
        regions,
      }).catch(() => null);
    };
    if (!nativeVisible) {
      installRegions();
      return;
    }
    const presentation = minimized
      ? "minimized"
      : expanded
        ? "expanded"
        : "compact";
    const geometry = bubbleTargetGeometry(presentation, bounds);
    void invoke("set_overlay_bubble_geometry", {
      agentId,
      width: geometry.width,
      height: geometry.height,
    })
      .then(installRegions)
      .catch(installRegions);
  }, [agentId, expanded, minimized, nativeVisible, safeMode]);

  useEffect(() => {
    const closeRegistration = createListenerRegistration();
    const openRegistration = createListenerRegistration();
    void listen<BubbleNativeLifecyclePayload>(
      BUBBLE_NATIVE_CLOSE_EVENT,
      (event) => {
        if (event.payload?.agentId !== agentId) return;
        setNativeVisible(false);
        setExpanded(false);
        setMinimized(false);
        geometryRevision.current += 1;
      },
    ).then(closeRegistration.register);
    void listen<BubbleNativeLifecyclePayload>(
      BUBBLE_NATIVE_OPEN_EVENT,
      (event) => {
        if (event.payload?.agentId !== agentId) return;
        setNativeVisible(true);
        geometryRevision.current += 1;
      },
    ).then(openRegistration.register);
    return () => {
      closeRegistration.dispose();
      openRegistration.dispose();
    };
  }, [agentId]);

  useEffect(() => {
    const refresh = () =>
      void invoke<AppSnapshot>("get_app_snapshot")
        .then((snapshot) => setSafeMode(snapshot.safeMode))
        .catch(() => setSafeMode(true));
    refresh();
    const timer = window.setInterval(refresh, 1000);
    return () => window.clearInterval(timer);
  }, []);

  useLayoutEffect(() => {
    const element = bubbleRef.current;
    if (element === null) return;
    const observer = new ResizeObserver(reportRegion);
    observer.observe(element);
    reportRegion();
    window.addEventListener("resize", reportRegion);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", reportRegion);
    };
  }, [reportRegion]);

  useLayoutEffect(() => {
    reportRegion();
  }, [expanded, minimized, reportRegion]);

  useEffect(
    () => () => {
      void invoke("set_overlay_interactive_regions", {
        agentId,
        regions: [],
      }).catch(() => null);
    },
    [agentId],
  );

  if (safeMode) return null;

  if (error || phase === null) {
    return (
      <main ref={bubbleRef} className="agent-bubble compact" role="status">
        <div className="bubble-heading">
          <strong>
            {error ? "Runtime indisponível" : "Carregando conversa…"}
          </strong>
          <CloseBubble
            agentId={agentId}
            onClose={() => {
              setNativeVisible(false);
              setExpanded(false);
              setMinimized(false);
            }}
          />
        </div>
      </main>
    );
  }

  const currentPhase = phase;
  const presentation = bubblePresentation(currentPhase);
  const request = presentation.request;
  const status = presentation.preview;
  const blocked = blockedSendCopy(currentPhase.sendBlockedCode);
  const canSend = canSendConversationMessage(currentPhase);
  const canDraft = canDraftConversationMessage(currentPhase);
  const routingPolicy = routingPolicyPayload(modelPreferences);

  async function send() {
    const content = draft.trim();
    if (!content || sending || !canSend) return;
    setSending(true);
    try {
      await invoke("send_phase_one_message", {
        agentId,
        conversationId: currentPhase.conversation.id,
        content,
        policy: routingPolicy,
      });
      setDraft("");
      await load();
    } finally {
      setSending(false);
    }
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
    <main
      ref={bubbleRef}
      className={`agent-bubble ${minimized ? "minimized" : expanded ? "expanded" : "compact"}`}
    >
      <div className="bubble-heading">
        <button
          className="bubble-title"
          type="button"
          onClick={() => {
            if (minimized) {
              setMinimized(false);
              setExpanded(true);
            } else {
              setExpanded((value) => !value);
            }
          }}
        >
          <strong>{phase.agent.name}</strong>
          <small className="readable-helper">
            {phase.conversation.title ?? "Conversa"}
          </small>
        </button>
        <div className="bubble-heading-actions">
          {minimized ? (
            <button
              className="bubble-restore"
              type="button"
              onClick={() => {
                setMinimized(false);
                setExpanded(true);
              }}
            >
              Restaurar
            </button>
          ) : expanded ? (
            <button
              className="bubble-minimize"
              type="button"
              aria-label="Minimizar balão"
              onClick={() => setMinimized(true)}
            >
              −
            </button>
          ) : null}
          <CloseBubble
            agentId={agentId}
            onClose={() => {
              setNativeVisible(false);
              setExpanded(false);
              setMinimized(false);
            }}
          />
        </div>
      </div>

      {minimized ? (
        <p className="bubble-minimized-preview">{status}</p>
      ) : !expanded ? (
        <button
          className="bubble-preview"
          type="button"
          onClick={() => setExpanded(true)}
        >
          {status}
        </button>
      ) : (
        <>
          <div className="bubble-message-scroll">
            {presentation.fullText ? (
              <p>{presentation.fullText}</p>
            ) : (
              <p className="bubble-muted readable-helper">{status}</p>
            )}
          </div>
          {request !== null ? (
            <button
              className="bubble-cancel"
              type="button"
              disabled={!canRequestCancellation(request, cancellingRequestId)}
              onClick={() => void cancelCurrentRequest()}
            >
              {request.cancellationRequested ||
              cancellingRequestId === request.requestId
                ? "Cancelando resposta…"
                : "Cancelar resposta"}
            </button>
          ) : null}
          <div className="bubble-composer">
            <textarea
              value={draft}
              maxLength={16_384}
              disabled={!canDraft || sending}
              placeholder={blocked ?? "Responder…"}
              onChange={(event) => setDraft(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && !event.shiftKey) {
                  event.preventDefault();
                  void send();
                }
              }}
            />
            <button
              type="button"
              disabled={!canSend || !draft.trim() || sending}
              onClick={() => void send()}
            >
              Enviar
            </button>
          </div>
          <button
            className="bubble-open-chat"
            type="button"
            onClick={() =>
              void openAgentConversations(agentId, currentPhase.conversation.id)
            }
          >
            Abrir conversa completa
          </button>
        </>
      )}
    </main>
  );
}

function CloseBubble({
  agentId,
  onClose,
}: {
  agentId: string;
  onClose: () => void;
}) {
  return (
    <button
      className="bubble-close"
      type="button"
      aria-label="Fechar balão"
      onClick={() => {
        onClose();
        void invoke("set_overlay_bubble_visible", { agentId, visible: false });
      }}
    >
      ×
    </button>
  );
}
