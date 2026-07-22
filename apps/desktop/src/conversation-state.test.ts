import { describe, expect, it } from "vitest";
import type { PhaseOneEvent, PhaseOneState } from "@aip/contracts";
import {
  applyPhaseOneEvent,
  blockedSendCopy,
  bubblePresentation,
  canRequestCancellation,
  compactPreview,
  createConversationViewState,
  messageStatusCopy,
  providerStatusCopy,
  requestForAgent,
} from "./conversation-state";

function phase(agentId = "astra"): PhaseOneState {
  return {
    agent: {
      id: agentId,
      name: "Astra",
      profileKey: "owner",
      spriteKey: "astra",
      position: { x: 0, y: 0 },
    },
    conversation: {
      id: `conversation-${agentId}`,
      agentId,
      title: "Conversa principal",
    },
    messages: [
      {
        id: "assistant",
        conversationId: `conversation-${agentId}`,
        agentId,
        author: "agent",
        content: "",
        modelRef: "ollama:test",
        status: "streaming",
        createdAt: 1,
        completedAt: null,
        errorCode: null,
      },
    ],
    provider: {
      state: "available",
      detailCode: "provider_available",
      models: [],
      refreshedAt: 1,
    },
    selectedModelRef: "ollama:test",
    selectedModelAvailable: true,
    keepAliveMinutes: 15,
    queue: [
      {
        requestId: "request",
        agentId,
        conversationId: `conversation-${agentId}`,
        assistantMessageId: "assistant",
        position: 0,
        active: true,
        cancellationRequested: false,
      },
    ],
    canSend: true,
    sendBlockedCode: null,
  };
}

function event(
  eventType: PhaseOneEvent["eventType"],
  sequence: number | null,
  content: string | null,
): PhaseOneEvent {
  return {
    protocolVersion: 1,
    eventType,
    requestId: "request",
    agentId: "astra",
    conversationId: "conversation-astra",
    assistantMessageId: "assistant",
    sequence,
    content,
    errorCode: null,
  };
}

describe("conversation event reducer", () => {
  it("appends each ordered streaming chunk exactly once", () => {
    const initial = createConversationViewState(phase());
    const first = applyPhaseOneEvent(
      initial,
      event("generation.chunk", 1, "Olá"),
    );
    const duplicate = applyPhaseOneEvent(
      first,
      event("generation.chunk", 1, "Olá"),
    );
    const outOfOrder = applyPhaseOneEvent(
      first,
      event("generation.chunk", 3, "!"),
    );
    const second = applyPhaseOneEvent(
      first,
      event("generation.chunk", 2, " mundo"),
    );
    expect(first.phase.messages[0]?.content).toBe("Olá");
    expect(duplicate).toBe(first);
    expect(outOfOrder).toBe(first);
    expect(second.phase.messages[0]?.content).toBe("Olá mundo");
  });

  it("ignores another agent and keeps terminal state idempotent", () => {
    const initial = createConversationViewState(phase());
    const wrongAgent = {
      ...event("generation.chunk", 1, "x"),
      agentId: "luma",
    };
    expect(applyPhaseOneEvent(initial, wrongAgent)).toBe(initial);
    const complete = applyPhaseOneEvent(
      initial,
      event("generation.complete", null, null),
    );
    expect(complete.phase.messages[0]?.status).toBe("complete");
    expect(
      applyPhaseOneEvent(complete, event("generation.failed", null, null)),
    ).toBe(complete);
  });

  it("exposes provider, model, cancel and compact bubble states", () => {
    const current = phase();
    expect(providerStatusCopy(current)).toBe("Ollama disponível");
    expect(blockedSendCopy("selected_model_unavailable")).toContain(
      "indisponível",
    );
    expect(requestForAgent(current.queue, "astra")?.active).toBe(true);
    expect(requestForAgent(current.queue, "luma")).toBeNull();
    expect(compactPreview("um\ndois\ntrês\nquatro")).toBe("um\ndois\ntrês…");
  });

  it("reports unavailable persisted selection without switching it", () => {
    const current = phase();
    current.selectedModelAvailable = false;
    expect(providerStatusCopy(current)).toBe("Modelo selecionado indisponível");
    expect(current.selectedModelRef).toBe("ollama:test");
  });

  it("derives compact, expanded and cancel controls from authoritative state", () => {
    const current = phase();
    current.messages[0]!.content = "um\ndois\ntrês\nquatro";
    const active = bubblePresentation(current);
    expect(active.preview).toBe("Gerando resposta…");
    expect(active.fullText).toBe("um\ndois\ntrês\nquatro");
    expect(active.request?.requestId).toBe("request");
    expect(canRequestCancellation(active.request, null)).toBe(true);
    expect(canRequestCancellation(active.request, "request")).toBe(false);
    current.queue[0]!.cancellationRequested = true;
    expect(bubblePresentation(current).preview).toBe("Cancelando resposta…");
    expect(canRequestCancellation(current.queue[0]!, null)).toBe(false);
    current.queue = [];
    const complete = bubblePresentation(current);
    expect(complete.preview).toBe("um\ndois\ntrês…");
    expect(complete.request).toBeNull();
  });

  it("distinguishes provider failure from genuine runtime death", () => {
    const current = phase();
    const failed = current.messages[0]!;
    failed.status = "failed";
    failed.errorCode = "provider_interrupted";
    expect(messageStatusCopy(failed)).toContain("Ollama");
    failed.errorCode = "runtime_process_exit_unexpected";
    expect(messageStatusCopy(failed)).toContain("Runtime local");
  });

  it("keeps simultaneous Astra and Luma bubble reducers isolated", () => {
    const astra = createConversationViewState(phase("astra"));
    const luma = createConversationViewState(phase("luma"));
    const updatedAstra = applyPhaseOneEvent(
      astra,
      event("generation.chunk", 1, "Astra reply"),
    );
    expect(updatedAstra.phase.messages[0]?.content).toBe("Astra reply");
    expect(luma.phase.messages[0]?.content).toBe("");
  });
});
