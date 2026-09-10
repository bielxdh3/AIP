import { describe, expect, it } from "vitest";
import type { PhaseOneState } from "@aip/contracts";
import {
  applyPhaseOneEvent,
  createConversationViewState,
} from "./conversation-state";
import { loadIsCurrent, mergeLoadedPhase } from "./use-phase-one";

describe("Phase One load revisions", () => {
  it("ignores a load that began before an applied stream event", () => {
    const startedRevision = 1;
    const revisionAfterStreamEvent = startedRevision + 1;

    expect(loadIsCurrent(startedRevision, revisionAfterStreamEvent)).toBe(
      false,
    );
    expect(
      loadIsCurrent(revisionAfterStreamEvent, revisionAfterStreamEvent),
    ).toBe(true);
  });

  it("preserves active request sequences across a same-conversation refresh", () => {
    const phase = {
      agent: { id: "agent" },
      conversation: { id: "conversation" },
      queue: [
        {
          agentId: "agent",
          requestId: "request",
          assistantMessageId: "assistant",
          active: true,
          cancellationRequested: false,
        },
      ],
    } as unknown as PhaseOneState;
    const current = createConversationViewState(phase);
    current.lastSequenceByRequest.request = 1;

    expect(mergeLoadedPhase(current, phase).lastSequenceByRequest).toEqual({
      request: 1,
    });
    expect(
      mergeLoadedPhase(current, {
        ...phase,
        queue: [],
      }).lastSequenceByRequest,
    ).toEqual({});
    expect(
      mergeLoadedPhase(current, {
        ...phase,
        conversation: {
          ...phase.conversation,
          id: "other-conversation",
        },
      }).lastSequenceByRequest,
    ).toEqual({});
  });

  it("does not let a stale reload rewind an active streamed assistant", () => {
    const message = {
      id: "assistant",
      conversationId: "conversation",
      agentId: "agent",
      author: "agent",
      content: "AB",
      status: "streaming",
    };
    const queueEntry = {
      agentId: "agent",
      requestId: "request",
      conversationId: "conversation",
      assistantMessageId: "assistant",
      active: true,
      cancellationRequested: false,
    };
    const currentPhase = {
      agent: { id: "agent" },
      conversation: { id: "conversation" },
      messages: [message],
      queue: [queueEntry],
    } as unknown as PhaseOneState;
    const current = createConversationViewState(currentPhase);
    current.lastSequenceByRequest.request = 2;
    const stalePhase = {
      ...currentPhase,
      messages: [{ ...message, content: "A", status: "streaming" }],
    } as unknown as PhaseOneState;

    const merged = mergeLoadedPhase(current, stalePhase);
    expect(merged.phase.messages[0]?.content).toBe("AB");
    expect(merged.phase.messages[0]?.status).toBe("streaming");
    expect(merged.lastSequenceByRequest).toEqual({ request: 2 });

    const terminal = mergeLoadedPhase(current, {
      ...currentPhase,
      messages: [{ ...message, content: "AB", status: "complete" }],
      queue: [],
    } as unknown as PhaseOneState);
    expect(terminal.phase.messages[0]?.content).toBe("AB");
    expect(terminal.phase.messages[0]?.status).toBe("complete");
    expect(terminal.phase.queue).toEqual([]);
    expect(terminal.lastSequenceByRequest).toEqual({});
  });

  it("keeps the event order monotonic across interleaved stale snapshots", () => {
    const baseMessage = {
      id: "assistant",
      conversationId: "conversation",
      agentId: "agent",
      author: "agent",
      content: "",
      status: "streaming",
    };
    const baseQueueEntry = {
      agentId: "agent",
      requestId: "request",
      conversationId: "conversation",
      assistantMessageId: "assistant",
      active: true,
      cancellationRequested: false,
    };
    const makePhase = (content: string, status = "streaming") =>
      ({
        agent: { id: "agent" },
        conversation: { id: "conversation" },
        messages: [{ ...baseMessage, content, status }],
        queue: status === "streaming" ? [baseQueueEntry] : [],
      }) as unknown as PhaseOneState;
    const makeChunk = (sequence: number) => ({
      protocolVersion: 1 as const,
      eventType: "generation.chunk" as const,
      requestId: "request",
      agentId: "agent",
      conversationId: "conversation",
      assistantMessageId: "assistant",
      sequence,
      content: "x",
      errorCode: null,
    });

    let current = createConversationViewState(makePhase(""));
    for (let sequence = 1; sequence <= 10; sequence += 1) {
      current = applyPhaseOneEvent(current, makeChunk(sequence));
    }

    current = mergeLoadedPhase(current, makePhase("x".repeat(5)));
    expect(current.phase.messages[0]?.content).toHaveLength(10);
    expect(current.lastSequenceByRequest.request).toBe(10);

    for (let sequence = 11; sequence <= 20; sequence += 1) {
      current = applyPhaseOneEvent(current, makeChunk(sequence));
    }
    current = mergeLoadedPhase(current, makePhase("x".repeat(15)));
    expect(current.phase.messages[0]?.content).toHaveLength(20);
    expect(current.lastSequenceByRequest.request).toBe(20);

    current = applyPhaseOneEvent(current, {
      ...makeChunk(20),
      eventType: "generation.complete",
      content: "",
    });

    const staleGenerating = mergeLoadedPhase(
      current,
      makePhase("x".repeat(15), "streaming"),
    );
    expect(staleGenerating.phase.messages[0]?.content).toHaveLength(20);
    expect(staleGenerating.phase.messages[0]?.status).toBe("complete");
    expect(staleGenerating.phase.queue).toEqual([]);
    expect(staleGenerating.lastSequenceByRequest).toEqual({});

    const staleTerminal = mergeLoadedPhase(
      current,
      makePhase("x".repeat(15), "complete"),
    );
    expect(staleTerminal.phase.messages[0]?.content).toHaveLength(20);
    expect(staleTerminal.phase.messages[0]?.status).toBe("complete");
  });
});
