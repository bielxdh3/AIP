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

const terminalStatuses = new Set(["complete", "failed", "cancelled"]);

export function loadIsCurrent(
  startedRevision: number,
  currentRevision: number,
): boolean {
  return startedRevision === currentRevision;
}

export function mergeLoadedPhase(
  current: ConversationViewState | null,
  phase: PhaseOneState,
): ConversationViewState {
  const next = createConversationViewState(phase);
  if (
    current === null ||
    current.phase.agent.id !== phase.agent.id ||
    current.phase.conversation.id !== phase.conversation.id
  ) {
    return next;
  }
  // Some lightweight callers only provide identity/queue fields. Keep the old sequence
  // behavior for those snapshots while the real command always supplies full messages.
  if (
    !Array.isArray(phase.messages) ||
    !Array.isArray(current.phase.messages)
  ) {
    const activeRequestIds = new Set(
      phase.queue.map((entry) => entry.requestId),
    );
    return {
      phase,
      lastSequenceByRequest: Object.fromEntries(
        Object.entries(current.lastSequenceByRequest).filter(([requestId]) =>
          activeRequestIds.has(requestId),
        ),
      ),
    };
  }

  const currentQueueByMessage = new Map(
    current.phase.queue.map((entry) => [entry.assistantMessageId, entry]),
  );
  const preservedRequestIds = new Set<string>();
  const mergedMessages = phase.messages.map((loadedMessage) => {
    const currentMessage = current.phase.messages.find(
      (candidate) => candidate.id === loadedMessage.id,
    );
    const currentEntry = currentQueueByMessage.get(loadedMessage.id);
    if (currentMessage === undefined || currentEntry === undefined) {
      return loadedMessage;
    }
    const contentIsMonotonic =
      currentMessage.content.length >= loadedMessage.content.length &&
      currentMessage.content.startsWith(loadedMessage.content);
    const loadedIsTerminal = terminalStatuses.has(loadedMessage.status);
    if (
      !terminalStatuses.has(currentMessage.status) &&
      contentIsMonotonic &&
      (!loadedIsTerminal ||
        currentMessage.content.length > loadedMessage.content.length)
    ) {
      preservedRequestIds.add(currentEntry.requestId);
      return currentMessage;
    }
    return loadedMessage;
  });
  const mergedQueue = phase.queue.map((entry) =>
    preservedRequestIds.has(entry.requestId)
      ? (current.phase.queue.find(
          (candidate) => candidate.requestId === entry.requestId,
        ) ?? entry)
      : entry,
  );
  for (const entry of current.phase.queue) {
    if (
      preservedRequestIds.has(entry.requestId) &&
      !mergedQueue.some((candidate) => candidate.requestId === entry.requestId)
    ) {
      mergedQueue.push(entry);
    }
  }
  const mergedPhase = {
    ...phase,
    messages: mergedMessages,
    queue: mergedQueue,
  };
  const activeRequestIds = new Set(mergedQueue.map((entry) => entry.requestId));
  return {
    phase: mergedPhase,
    lastSequenceByRequest: Object.fromEntries(
      Object.entries(current.lastSequenceByRequest).filter(([requestId]) =>
        activeRequestIds.has(requestId),
      ),
    ),
  };
}

export function usePhaseOne(agentId: string | null, temporary = false) {
  const [view, setView] = useState<ConversationViewState | null>(null);
  const [error, setError] = useState(false);
  const loadRevision = useRef(0);

  const load = useCallback(async () => {
    if (agentId === null) return;
    const revision = ++loadRevision.current;
    try {
      const phase = await invoke<PhaseOneState>(
        temporary ? "get_temporary_phase_one_state" : "get_phase_one_state",
        {
          agentId,
        },
      );
      if (!loadIsCurrent(revision, loadRevision.current)) return;
      setView((current) => mergeLoadedPhase(current, phase));
      setError(false);
    } catch {
      if (!loadIsCurrent(revision, loadRevision.current)) return;
      setError(true);
    }
  }, [agentId, temporary]);

  useEffect(() => {
    setView(null);
    void load();
  }, [load]);

  useEffect(() => {
    const registration = createListenerRegistration();
    void listen<PhaseOneEvent>("phase-one-event", (incoming) => {
      const event = incoming.payload;
      setView((current) => {
        if (current === null) return current;
        const next = applyPhaseOneEvent(current, event);
        if (next !== current) loadRevision.current += 1;
        return next;
      });
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
