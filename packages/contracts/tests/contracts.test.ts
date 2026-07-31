import { describe, expect, it } from "vitest";
import {
  PROTOCOL_VERSION,
  canTransitionRuntime,
  parseCognitiveEvent,
  parseCognitiveTrait,
  parseHealthResponse,
} from "../src/index";

describe("runtime contracts", () => {
  it("allows deterministic safe-mode transitions", () => {
    expect(canTransitionRuntime("ready", "safe_mode")).toBe(true);
    expect(canTransitionRuntime("safe_mode", "ready")).toBe(false);
    expect(canTransitionRuntime("safe_mode", "starting")).toBe(true);
  });

  it("accepts only the versioned health response", () => {
    expect(
      parseHealthResponse({
        protocolVersion: PROTOCOL_VERSION,
        id: "health-1",
        result: {
          name: "aip-runtime",
          status: "ready",
          protocolVersion: PROTOCOL_VERSION,
        },
      }),
    ).not.toBeNull();

    expect(
      parseHealthResponse({
        protocolVersion: 99,
        id: "health-1",
        result: { name: "aip-runtime", status: "ready", protocolVersion: 99 },
      }),
    ).toBeNull();
  });
});

describe("cognitive contracts", () => {
  it("accepts typed trait and event responses and rejects malformed values", () => {
    expect(
      parseCognitiveTrait({ key: "curiosity", value: 0.5, isProtected: false }),
    ).not.toBeNull();
    expect(parseCognitiveTrait({ key: "curiosity", value: "0.5" })).toBeNull();
    expect(
      parseCognitiveEvent({
        id: "event",
        agentId: "astra",
        kind: "trait_delta",
        traitKey: "curiosity",
        sourceKind: "internal_test",
        reason: "test",
        confidence: 1,
        requestedValue: 0.05,
        appliedDelta: 0.05,
        priorValue: 0.5,
        resultingValue: 0.55,
        status: "applied",
        code: null,
        rollbackOfEventId: null,
        createdAt: 1,
      }),
    ).not.toBeNull();
    expect(parseCognitiveEvent({ id: "event" })).toBeNull();
  });
});
