import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { PhaseOneEvent, PhaseOneState } from "@aip/contracts";
import {
  applyPhaseOneEvent,
  createConversationViewState,
  type ConversationViewState,
} from "./conversation-state";
import { createListenerRegistration } from "./listener-lifecycle";

export function usePhaseOne(agentId: string | null) {
  const [view, setView] = useState<ConversationViewState | null>(null);
  const [error, setError] = useState(false);
  const loadRevision = useRef(0);

  const load = useCallback(async () => {
    if (agentId === null) return;
    const revision = ++loadRevision.current;
    try {
      const phase = await invoke<PhaseOneState>("get_phase_one_state", {
        agentId,
      });
      if (revision !== loadRevision.current) return;
      setView(createConversationViewState(phase));
      setError(false);
    } catch {
      if (revision !== loadRevision.current) return;
      setError(true);
    }
  }, [agentId]);

  useEffect(() => {
    setView(null);
    void load();
  }, [load]);

  useEffect(() => {
    const registration = createListenerRegistration();
    void listen<PhaseOneEvent>("phase-one-event", (incoming) => {
      const event = incoming.payload;
      setView((current) =>
        current === null ? current : applyPhaseOneEvent(current, event),
      );
      if (
        event.eventType === "state.changed" ||
        event.eventType === "generation.complete" ||
        event.eventType === "generation.failed" ||
        event.eventType === "generation.cancelled"
      ) {
        void load();
      }
    }).then(registration.register);
    return registration.dispose;
  }, [load]);

  return { view, phase: view?.phase ?? null, error, load };
}
