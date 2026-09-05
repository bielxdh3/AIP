export type PointerPoint = {
  x: number;
  y: number;
};

export function pointerDelta(
  previous: PointerPoint | null,
  current: PointerPoint,
): PointerPoint | null {
  if (
    previous === null ||
    !Number.isFinite(previous.x) ||
    !Number.isFinite(previous.y) ||
    !Number.isFinite(current.x) ||
    !Number.isFinite(current.y)
  ) {
    return null;
  }
  const delta = { x: current.x - previous.x, y: current.y - previous.y };
  return Number.isFinite(delta.x) && Number.isFinite(delta.y) ? delta : null;
}
