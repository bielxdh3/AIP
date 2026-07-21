export const DRAG_THRESHOLD_PX = 6;
export const DOUBLE_CLICK_INTERVAL_MS = 450;
export const THOUGHT_DURATION_MS = 3000;

export type OverlayGestureAction = "none" | "click" | "start_drag" | "thought";

export type OverlayGestureState = {
  pointerId: number | null;
  startX: number;
  startY: number;
  dragging: boolean;
  lastClickAt: number | null;
};

export const initialOverlayGestureState: OverlayGestureState = {
  pointerId: null,
  startX: 0,
  startY: 0,
  dragging: false,
  lastClickAt: null,
};

export function beginGesture(
  state: OverlayGestureState,
  pointerId: number,
  x: number,
  y: number,
): OverlayGestureState {
  return { ...state, pointerId, startX: x, startY: y, dragging: false };
}

export function moveGesture(
  state: OverlayGestureState,
  pointerId: number,
  x: number,
  y: number,
): { state: OverlayGestureState; action: OverlayGestureAction } {
  if (state.pointerId !== pointerId || state.dragging)
    return { state, action: "none" };
  if (Math.hypot(x - state.startX, y - state.startY) < DRAG_THRESHOLD_PX) {
    return { state, action: "none" };
  }
  return { state: { ...state, dragging: true }, action: "start_drag" };
}

export function endGesture(
  state: OverlayGestureState,
  pointerId: number,
  now: number,
): { state: OverlayGestureState; action: OverlayGestureAction } {
  if (state.pointerId !== pointerId) return { state, action: "none" };
  if (state.dragging) {
    return {
      state: { ...state, pointerId: null, dragging: false },
      action: "none",
    };
  }
  const doubleClick =
    state.lastClickAt !== null &&
    now - state.lastClickAt <= DOUBLE_CLICK_INTERVAL_MS;
  return {
    state: {
      ...state,
      pointerId: null,
      lastClickAt: doubleClick ? null : now,
    },
    action: doubleClick ? "thought" : "click",
  };
}

export function cancelGesture(state: OverlayGestureState): OverlayGestureState {
  return { ...state, pointerId: null, dragging: false };
}
