import { describe, expect, it } from "vitest";
import {
  beginGesture,
  cancelGesture,
  endGesture,
  initialOverlayGestureState,
  moveGesture,
} from "./overlay-gesture";
import { screenPoint } from "./overlay-drag";

describe("overlay gesture state", () => {
  it("keeps the first stationary press as a click", () => {
    const pressed = beginGesture(initialOverlayGestureState, 1, 10, 10);
    expect(endGesture(pressed, 1, 100).action).toBe("click");
  });

  it("uses one screen-coordinate basis for threshold and drag deltas", () => {
    const start = screenPoint({ screenX: 1010, screenY: 410 });
    const pressed = beginGesture(
      initialOverlayGestureState,
      1,
      start.x,
      start.y,
    );
    const belowThreshold = screenPoint({ screenX: 1013, screenY: 413 });
    expect(
      moveGesture(pressed, 1, belowThreshold.x, belowThreshold.y).action,
    ).toBe("none");
    const movedPoint = screenPoint({ screenX: 1020, screenY: 410 });
    const moved = moveGesture(pressed, 1, movedPoint.x, movedPoint.y);
    expect(moved.action).toBe("start_drag");
    expect(endGesture(moved.state, 1, 100).action).toBe("none");
  });

  it("turns a second click within the interval into full-chat navigation", () => {
    const first = endGesture(
      beginGesture(initialOverlayGestureState, 1, 0, 0),
      1,
      100,
    );
    const secondPress = beginGesture(first.state, 2, 0, 0);
    expect(endGesture(secondPress, 2, 400).action).toBe("double_click");
  });

  it("cancels drag state without triggering navigation", () => {
    const pressed = beginGesture(initialOverlayGestureState, 1, 0, 0);
    const moved = moveGesture(pressed, 1, 10, 0);
    const cancelled = cancelGesture(moved.state);
    expect(cancelled.dragging).toBe(false);
    expect(cancelled.pointerId).toBeNull();
  });

  it("clears the pointer after a drag ends", () => {
    const pressed = beginGesture(initialOverlayGestureState, 1, 0, 0);
    const moved = moveGesture(pressed, 1, 10, 0);
    const ended = endGesture(moved.state, 1, 100);
    expect(ended.action).toBe("none");
    expect(ended.state.pointerId).toBeNull();
    expect(ended.state.dragging).toBe(false);
  });
});
