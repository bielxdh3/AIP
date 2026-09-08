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
});
