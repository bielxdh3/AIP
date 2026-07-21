import { describe, expect, it } from "vitest";
import {
  PROTOCOL_VERSION,
  canTransitionRuntime,
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
