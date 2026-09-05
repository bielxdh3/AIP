import { describe, expect, it } from "vitest";
import { pointerDelta, screenPoint } from "./overlay-drag";

describe("controlled overlay drag geometry", () => {
  it("returns the movement between consecutive pointer samples", () => {
    expect(pointerDelta({ x: 10, y: 20 }, { x: 18, y: 13 })).toEqual({
      x: 8,
      y: -7,
    });
  });

  it("keeps slow incremental movement in stationary screen coordinates", () => {
    const samples = [
      screenPoint({ screenX: 1000, screenY: 500 }),
      screenPoint({ screenX: 1002, screenY: 501 }),
      screenPoint({ screenX: 1004, screenY: 502 }),
      screenPoint({ screenX: 1006, screenY: 503 }),
    ];

    expect(
      samples
        .slice(1)
        .map((sample, index) => pointerDelta(samples[index] ?? null, sample)),
    ).toEqual([
      { x: 2, y: 1 },
      { x: 2, y: 1 },
      { x: 2, y: 1 },
    ]);
  });

  it("extracts screen coordinates without using a moving client origin", () => {
    const sample = {
      screenX: 1440,
      screenY: 812,
      clientX: 12,
      clientY: 4,
    };
    expect(screenPoint(sample)).toEqual({ x: 1440, y: 812 });
  });

  it("fails closed when a pointer sample is unavailable or non-finite", () => {
    expect(pointerDelta(null, { x: 1, y: 1 })).toBeNull();
    expect(pointerDelta({ x: Number.NaN, y: 1 }, { x: 1, y: 1 })).toBeNull();
    expect(
      pointerDelta({ x: 1, y: 1 }, { x: Number.POSITIVE_INFINITY, y: 1 }),
    ).toBeNull();
  });
});
