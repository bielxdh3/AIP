import { describe, expect, it } from "vitest";

import {
  GENERATION_HEARTBEAT_STALL_MS,
  measureGenerationHeartbeat,
} from "./generation-liveness";

describe("generation liveness", () => {
  it("keeps normal heartbeat latency non-stalled", () => {
    expect(measureGenerationHeartbeat(100, 240)).toEqual({
      latencyMs: 140,
      stalled: false,
    });
  });

  it("marks a heartbeat at the stall threshold", () => {
    expect(
      measureGenerationHeartbeat(100, 100 + GENERATION_HEARTBEAT_STALL_MS),
    ).toEqual({
      latencyMs: GENERATION_HEARTBEAT_STALL_MS,
      stalled: true,
    });
  });
});
