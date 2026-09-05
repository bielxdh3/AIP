import { describe, expect, it } from "vitest";
import { pointerDelta } from "./overlay-drag";

describe("controlled overlay drag geometry", () => {
  it("returns the movement between consecutive pointer samples", () => {
    expect(pointerDelta({ x: 10, y: 20 }, { x: 18, y: 13 })).toEqual({
      x: 8,
      y: -7,
    });
  });

  it("fails closed when a pointer sample is unavailable or non-finite", () => {
    expect(pointerDelta(null, { x: 1, y: 1 })).toBeNull();
    expect(pointerDelta({ x: Number.NaN, y: 1 }, { x: 1, y: 1 })).toBeNull();
    expect(
      pointerDelta({ x: 1, y: 1 }, { x: Number.POSITIVE_INFINITY, y: 1 }),
    ).toBeNull();
  });
});
