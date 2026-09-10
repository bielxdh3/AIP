export const GENERATION_HEARTBEAT_INTERVAL_MS = 500;
export const GENERATION_HEARTBEAT_STALL_MS = 2_000;

export type HeartbeatMeasurement = {
  latencyMs: number;
  stalled: boolean;
};

export function measureGenerationHeartbeat(
  startedAt: number,
  finishedAt: number,
): HeartbeatMeasurement {
  const latencyMs = Math.max(0, finishedAt - startedAt);
  return {
    latencyMs,
    stalled: latencyMs >= GENERATION_HEARTBEAT_STALL_MS,
  };
}
