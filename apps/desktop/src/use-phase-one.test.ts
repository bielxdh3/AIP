import { describe, expect, it } from "vitest";
import type { PhaseOneState } from "@aip/contracts";
import { createConversationViewState } from "./conversation-state";
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
});
