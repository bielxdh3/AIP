export const PROTOCOL_VERSION = 1 as const;

export type RuntimeState =
  "stopped" | "starting" | "ready" | "unavailable" | "crashed" | "safe_mode";

export type RuntimeStatus = {
  state: RuntimeState;
  protocolVersion: number | null;
  detailCode: string;
};

export type AgentAnimationState = "idle" | "dragged" | "thinking";

export type AgentPosition = {
  x: number;
  y: number;
};

export type OverlayInteractiveRegion = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export type ProvisionalAgent = {
  id: string;
  name: string;
  profileKey: "owner" | "companion";
  spriteKey: "astra" | "luma";
  position: AgentPosition;
};

export type AppSnapshot = {
  appVersion: string;
  safeMode: boolean;
  databaseReady: boolean;
  migrationVersion: number;
  runtime: RuntimeStatus;
  agents: ProvisionalAgent[];
};

export type HealthRequest = {
  protocolVersion: typeof PROTOCOL_VERSION;
  id: string;
  method: "runtime.health";
  params: Record<string, never>;
};

export type HealthResponse = {
  protocolVersion: typeof PROTOCOL_VERSION;
  id: string;
  result: {
    name: "aip-runtime";
    status: "ready";
    protocolVersion: typeof PROTOCOL_VERSION;
  };
};

const runtimeTransitions: Record<RuntimeState, readonly RuntimeState[]> = {
  stopped: ["starting", "safe_mode"],
  starting: ["ready", "unavailable", "crashed", "stopped", "safe_mode"],
  ready: ["crashed", "stopped", "safe_mode"],
  unavailable: ["starting", "stopped", "safe_mode"],
  crashed: ["starting", "stopped", "safe_mode"],
  safe_mode: ["stopped", "starting"],
};

export function canTransitionRuntime(
  from: RuntimeState,
  to: RuntimeState,
): boolean {
  return runtimeTransitions[from].includes(to);
}

export function parseHealthResponse(value: unknown): HealthResponse | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }

  const candidate = value as Partial<HealthResponse>;
  if (
    candidate.protocolVersion !== PROTOCOL_VERSION ||
    typeof candidate.id !== "string" ||
    candidate.id.length === 0 ||
    typeof candidate.result !== "object" ||
    candidate.result === null
  ) {
    return null;
  }

  const result = candidate.result as Partial<HealthResponse["result"]>;
  return result.name === "aip-runtime" &&
    result.status === "ready" &&
    result.protocolVersion === PROTOCOL_VERSION
    ? (candidate as HealthResponse)
    : null;
}
