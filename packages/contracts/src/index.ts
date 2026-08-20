export const PROTOCOL_VERSION = 1 as const;

export const PHASE_ONE_LIMITS = {
  maxUserMessageBytes: 16_384,
  maxHistoryMessages: 32,
  maxContextBytes: 49_152,
  maxStreamChunkBytes: 8_192,
  maxAssistantOutputBytes: 65_536,
  maxQueueLength: 8,
  maxDiscoveredModels: 64,
} as const;

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
  birthday: string;
  fictiveAge: number;
  ageCategory: string;
  species: string;
  pronouns: string;
  personalitySummary: string;
  traitsJson: string;
  appearancePreset: string;
};

export type AppSnapshot = {
  appVersion: string;
  buildSha: string;
  buildTimestamp: string;
  runtimePackagingMode: string;
  safeMode: boolean;
  databaseReady: boolean;
  migrationVersion: number;
  runtime: RuntimeStatus;
  agents: ProvisionalAgent[];
  onboardingRequired: boolean;
};

export type ProviderState =
  "checking" | "available" | "empty" | "unavailable" | "malformed" | "timeout";

export type OllamaModel = {
  ref: string;
  providerModelId: string;
  displayName: string;
  size: number;
  family: string | null;
  parameterSize: string | null;
  quantization: string | null;
  capabilities: string[];
};

export type ProviderSnapshot = {
  state: ProviderState;
  detailCode: string;
  models: OllamaModel[];
  refreshedAt: number | null;
};

export type ConversationMessageStatus =
  "pending" | "streaming" | "complete" | "failed" | "cancelled";

export type ConversationMessageAuthor = "user" | "agent" | "system";

export type ConversationMessage = {
  id: string;
  conversationId: string;
  agentId: string;
  author: ConversationMessageAuthor;
  content: string;
  modelRef: string | null;
  status: ConversationMessageStatus;
  createdAt: number;
  completedAt: number | null;
  errorCode: string | null;
  branchId: string;
  turnGroupId: string;
};

export type ConversationBranch = {
  id: string;
  parentBranchId: string | null;
  parentMessageId: string | null;
  createdAt: number;
};

export type ConversationTurnVariant = {
  assistantMessageId: string;
  branchId: string;
  turnGroupId: string;
};

export type PhaseOneConversation = {
  id: string;
  agentId: string;
  title: string;
  modelOverrideRef: string | null;
};

export type AgentMemory = {
  id: string;
  agentId: string;
  category: string;
  content: string;
  status: "active" | "archived" | "trashed" | "candidate_rejected";
  confirmationStatus: "confirmed" | "pending" | "rejected";
  confidenceMilli: number;
  importance: number;
  sourceType: string;
  sourceMessageId: string | null;
  sourceConversationId: string | null;
  conflictKey: string | null;
  createdAt: number;
  updatedAt: number;
};

export type AgentSimulatedState = {
  agentId: string;
  sleep: number;
  energy: number;
  mood: number;
  focus: number;
  curiosity: number;
  socialFatigue: number;
  mode: "normal" | "voice_muted" | "silent";
  suspended: boolean;
  wakeNowUntil: number | null;
  lastSimulatedAt: number;
};

export type CognitiveTrait = {
  key: string;
  value: number;
  isProtected: boolean;
};
export type CognitiveEventSummary = {
  id: string;
  agentId: string;
  kind: "trait_delta" | "owner_correction" | "rollback";
  traitKey: string;
  sourceKind: string;
  sourceReference: string | null;
  reason: string;
  confidence: number;
  requestedValue: number;
  appliedDelta: number | null;
  priorValue: number;
  resultingValue: number;
  status: "applied" | "rejected" | "rolled_back";
  code: string | null;
  rollbackOfEventId: string | null;
  createdAt: number;
};
export type CognitiveEventExplanation = {
  event: CognitiveEventSummary;
  traitLabel: string;
};
export type OpinionEvidence = {
  id: string;
  opinionId: string;
  sourceKind: string;
  classification: string;
  stance: number;
  claimKey: string;
  claimValue: string;
  sourceReference: string | null;
  attribution: string | null;
  confidence: number;
  status: "active" | "disputed" | "superseded" | "rejected";
  createdAt: number;
};
export type CognitiveOpinion = {
  id: string;
  agentId: string;
  subjectType: string;
  subjectRef: string;
  stance: number;
  confidence: number;
  status: "active" | "disputed" | "superseded" | "archived" | "rejected";
  reason: string;
  createdAt: number;
  updatedAt: number;
  evidence: OpinionEvidence[];
};
export type OpinionCandidateRequest = {
  agentId: string;
  subjectType: string;
  subjectRef: string;
  stance: number;
  confidence: number;
  sourceKind: string;
  classification: string;
  claimKey: string;
  claimValue: string;
  sourceReference: string | null;
  attribution: string | null;
  reason: string;
  idempotencyKey: string;
};
export type OpinionEvidenceCorrectionRequest = {
  agentId: string;
  evidenceId: string;
  claimValue: string;
  reason: string;
  idempotencyKey: string;
};
export type RelationshipValues = {
  familiarity: number;
  trust: number;
  affinity: number;
  admiration: number;
  irritation: number;
  reliabilityExpectation: number;
};
export type RelationshipDeltas = RelationshipValues;
export type RelationshipEvent = {
  id: string;
  relationshipId: string;
  eventId: string;
  deltas: RelationshipDeltas;
  prior: RelationshipValues;
  resulting: RelationshipValues;
  sourceKind: string;
  sourceReference: string | null;
  confidence: number;
  reason: string;
  status: "applied" | "superseded" | "rolled_back";
  createdAt: number;
};
export type RelationshipState = {
  id: string;
  agentId: string;
  subjectType: string;
  subjectRef: string;
  values: RelationshipValues;
  updatedAt: number;
  events: RelationshipEvent[];
};
export type RelationshipCandidateRequest = {
  agentId: string;
  subjectType: string;
  subjectRef: string;
  deltas: RelationshipDeltas;
  sourceKind: string;
  sourceReference: string | null;
  confidence: number;
  reason: string;
  idempotencyKey: string;
};
export type CognitiveGoalStatus =
  | "proposed"
  | "active"
  | "suspended"
  | "completed"
  | "cancelled"
  | "archived"
  | "rejected";
export type CognitiveGoal = {
  id: string;
  agentId: string;
  title: string;
  description: string;
  origin: "owner" | "agent_proposal";
  fictionalOnly: true;
  priority: number;
  status: CognitiveGoalStatus;
  budgetUnits: number;
  dueAt: number | null;
  expiresAt: number | null;
  completionEvidence: string | null;
  parentGoalId: string | null;
  createdAt: number;
  updatedAt: number;
};
export type GoalRequest = {
  agentId: string;
  title: string;
  description: string;
  priority: number;
  budgetUnits: number;
  dueAt: number | null;
  expiresAt: number | null;
  parentGoalId: string | null;
  idempotencyKey: string;
};
export type OwnerCorrectionRequest = {
  agentId: string;
  traitKey: string;
  value: number;
  reason: string;
  idempotencyKey: string;
};
export type RollbackRequest = {
  agentId: string;
  eventId: string;
  idempotencyKey: string;
};
export type CognitiveErrorCode =
  | "agent_not_found"
  | "event_not_found"
  | "invalid_idempotency_key"
  | "invalid_reason"
  | "invalid_value"
  | "operation_unavailable"
  | "oscillation_blocked"
  | "rate_limit_event"
  | "rate_limit_window"
  | "rollback_not_allowed"
  | "source_ineligible"
  | "source_not_found"
  | "ownership_mismatch"
  | "trait_not_found"
  | "protected_trait"
  | "idempotency_conflict"
  | "duplicate_evidence"
  | "rollback_conflict"
  | "invalid_classification"
  | "invalid_evidence"
  | "attribution_required"
  | "internet_fact_unverified"
  | "inference_not_fact"
  | "real_person_uncertain"
  | "defamation_blocked"
  | "invalid_status"
  | "evidence_not_found"
  | "evidence_not_active"
  | "invalid_subject"
  | "relationship_not_found"
  | "relationship_delta_limit"
  | "relationship_rate_limit"
  | "manipulation_blocked"
  | "invalid_goal"
  | "external_action_blocked"
  | "invalid_goal_budget"
  | "invalid_goal_schedule"
  | "goal_not_found"
  | "goal_loop_blocked"
  | "invalid_transition"
  | "persistence_failed";
export type CognitiveErrorResponse = {
  code: CognitiveErrorCode;
  message: string;
};

function cognitiveNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}
function cognitiveString(value: unknown): value is string {
  return typeof value === "string";
}
export function parseCognitiveTrait(value: unknown): CognitiveTrait | null {
  if (typeof value !== "object" || value === null) return null;
  const candidate = value as Partial<CognitiveTrait>;
  return typeof candidate.key === "string" &&
    cognitiveNumber(candidate.value) &&
    typeof candidate.isProtected === "boolean"
    ? (candidate as CognitiveTrait)
    : null;
}
export function parseCognitiveEvent(
  value: unknown,
): CognitiveEventSummary | null {
  if (typeof value !== "object" || value === null) return null;
  const candidate = value as Partial<CognitiveEventSummary>;
  return cognitiveString(candidate.id) &&
    cognitiveString(candidate.agentId) &&
    ["trait_delta", "owner_correction", "rollback"].includes(
      candidate.kind ?? "",
    ) &&
    cognitiveString(candidate.traitKey) &&
    cognitiveString(candidate.sourceKind) &&
    (candidate.sourceReference === null ||
      cognitiveString(candidate.sourceReference)) &&
    cognitiveString(candidate.reason) &&
    (candidate.appliedDelta === null ||
      cognitiveNumber(candidate.appliedDelta)) &&
    cognitiveNumber(candidate.priorValue) &&
    cognitiveNumber(candidate.resultingValue) &&
    cognitiveNumber(candidate.requestedValue) &&
    cognitiveNumber(candidate.confidence) &&
    ["applied", "rejected", "rolled_back"].includes(candidate.status ?? "") &&
    (candidate.code === null || cognitiveString(candidate.code)) &&
    (candidate.rollbackOfEventId === null ||
      cognitiveString(candidate.rollbackOfEventId)) &&
    cognitiveNumber(candidate.createdAt)
    ? (candidate as CognitiveEventSummary)
    : null;
}
export function parseCognitiveExplanation(
  value: unknown,
): CognitiveEventExplanation | null {
  if (typeof value !== "object" || value === null) return null;
  const candidate = value as Partial<CognitiveEventExplanation>;
  return parseCognitiveEvent(candidate.event) !== null &&
    cognitiveString(candidate.traitLabel)
    ? (candidate as CognitiveEventExplanation)
    : null;
}
export function parseOwnerCorrectionResult(
  value: unknown,
): CognitiveEventSummary | null {
  return parseCognitiveEvent(value);
}
export function parseRollbackResult(
  value: unknown,
): CognitiveEventSummary | null {
  return parseCognitiveEvent(value);
}
export function parseCognitiveError(
  value: unknown,
): CognitiveErrorResponse | null {
  if (typeof value !== "object" || value === null) return null;
  const candidate = value as Partial<CognitiveErrorResponse>;
  const codes: CognitiveErrorCode[] = [
    "agent_not_found",
    "event_not_found",
    "invalid_idempotency_key",
    "invalid_reason",
    "invalid_value",
    "operation_unavailable",
    "oscillation_blocked",
    "rate_limit_event",
    "rate_limit_window",
    "rollback_not_allowed",
    "source_ineligible",
    "source_not_found",
    "ownership_mismatch",
    "trait_not_found",
    "protected_trait",
    "idempotency_conflict",
    "duplicate_evidence",
    "rollback_conflict",
    "invalid_classification",
    "invalid_evidence",
    "attribution_required",
    "internet_fact_unverified",
    "inference_not_fact",
    "real_person_uncertain",
    "defamation_blocked",
    "invalid_status",
    "evidence_not_found",
    "evidence_not_active",
    "invalid_subject",
    "relationship_not_found",
    "relationship_delta_limit",
    "relationship_rate_limit",
    "manipulation_blocked",
    "invalid_goal",
    "external_action_blocked",
    "invalid_goal_budget",
    "invalid_goal_schedule",
    "goal_not_found",
    "goal_loop_blocked",
    "invalid_transition",
    "persistence_failed",
  ];
  return codes.includes(candidate.code as CognitiveErrorCode) &&
    cognitiveString(candidate.message)
    ? (candidate as CognitiveErrorResponse)
    : null;
}

export type QueueEntry = {
  requestId: string;
  agentId: string;
  conversationId: string;
  assistantMessageId: string;
  position: number;
  active: boolean;
  cancellationRequested: boolean;
};

export type PhaseOneState = {
  agent: ProvisionalAgent;
  conversation: PhaseOneConversation;
  messages: ConversationMessage[];
  branches: ConversationBranch[];
  turnVariants: ConversationTurnVariant[];
  activeBranchId: string | null;
  provider: ProviderSnapshot;
  selectedModelRef: string | null;
  defaultModelRef: string | null;
  modelOverrideRef: string | null;
  effectiveModelSource:
    "agent_default" | "conversation_override" | "temporary_override";
  selectedModelAvailable: boolean;
  keepAliveMinutes: number;
  queue: QueueEntry[];
  canSend: boolean;
  sendBlockedCode: string | null;
};

export type SendMessageResult = {
  requestId: string;
  conversationId: string;
  userMessageId: string;
  assistantMessageId: string;
};

export type PhaseOneEvent = {
  protocolVersion: typeof PROTOCOL_VERSION;
  eventType:
    | "state.changed"
    | "generation.started"
    | "generation.chunk"
    | "generation.complete"
    | "generation.failed"
    | "generation.cancelled";
  requestId: string | null;
  agentId: string | null;
  conversationId: string | null;
  assistantMessageId: string | null;
  sequence: number | null;
  content: string | null;
  errorCode: string | null;
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
