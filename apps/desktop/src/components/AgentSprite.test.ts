import { describe, expect, it } from "vitest";
import { pixelOverlays } from "./AgentSprite";

describe("pixelOverlays", () => {
  it("uses visible valid pixels and rejects malformed source", () => {
    expect(
      pixelOverlays(
        '{"layers":[{"pixels":[[1,2,"#fff"],[64,0,"#000"]]},{"visible":false,"pixels":[[3,4,"#000"]]}]}',
      ),
    ).toEqual([{ x: 1, y: 2, color: "#fff" }]);
    expect(pixelOverlays("not json")).toEqual([]);
  });
});
