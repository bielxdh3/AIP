import { describe, expect, it } from "vitest";
import {
  beginGesture,
  cancelGesture,
  endGesture,
  initialOverlayGestureState,
  moveGesture,
  THOUGHT_DURATION_MS,
} from "./overlay-gesture";

describe("overlay gesture state", () => {
  it("keeps the thought indicator available for direct interaction", () => {
    expect(THOUGHT_DURATION_MS).toBeGreaterThanOrEqual(2000);
  });

  it("keeps the first stationary press as a click", () => {
    const pressed = beginGesture(initialOverlayGestureState, 1, 10, 10);
    expect(endGesture(pressed, 1, 100).action).toBe("click");
  });

  it("starts drag only after movement crosses the threshold", () => {
    const pressed = beginGesture(initialOverlayGestureState, 1, 10, 10);
    expect(moveGesture(pressed, 1, 13, 13).action).toBe("none");
    const moved = moveGesture(pressed, 1, 20, 10);
    expect(moved.action).toBe("start_drag");
    expect(endGesture(moved.state, 1, 100).action).toBe("none");
  });

  it("turns a second click within the interval into thought", () => {
    const first = endGesture(
      beginGesture(initialOverlayGestureState, 1, 0, 0),
      1,
      100,
    );
    const secondPress = beginGesture(first.state, 2, 0, 0);
    expect(endGesture(secondPress, 2, 400).action).toBe("thought");
  });

  it("cancels drag state without triggering thought", () => {
    const pressed = beginGesture(initialOverlayGestureState, 1, 0, 0);
    const moved = moveGesture(pressed, 1, 10, 0);
    expect(cancelGesture(moved.state).dragging).toBe(false);
  });
});
