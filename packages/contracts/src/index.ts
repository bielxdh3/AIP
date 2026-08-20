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

export type VoiceMode = "normal" | "voice_muted" | "silent";
export type VoiceConsentState = "not_granted" | "granted" | "revoked";
export type VoiceSettings = {
  agentId: string;
  schemaVersion: 1;
  baseVoiceId: string;
  baseVoiceProtected: true;
  customVoiceRef: string | null;
  customVoiceConsent: VoiceConsentState;
  recognitionModelRef: string | null;
  synthesisModelRef: string | null;
  inputDeviceRef: string | null;
  outputDeviceRef: string | null;
  mode: VoiceMode;
  voiceMuted: boolean;
  silent: boolean;
  suspended: boolean;
  updatedAt: number;
};
export type VoiceSettingsRequest = {
  agentId: string;
  recognitionModelRef: string | null;
  synthesisModelRef: string | null;
  inputDeviceRef: string | null;
  outputDeviceRef: string | null;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type CustomVoiceConsentRequest = {
  agentId: string;
  granted: boolean;
  customVoiceRef: string | null;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type VoiceTranscriptionRequest = {
  agentId: string;
  fixtureId: string;
  temporaryChat: boolean;
};
export type VoiceSynthesisRequest = {
  agentId: string;
  text: string;
  temporaryChat: boolean;
};
export type VoiceWakeWordRequest = {
  agentId: string;
  fixtureId: string;
  temporaryChat: boolean;
};
export type VoiceEmotionHypothesisRequest = {
  text: string;
  temporaryChat: boolean;
};
export type VoiceTranscriptionResult = {
  status: "ready" | "degraded";
  code: string | null;
  fixtureId: string;
  text: string | null;
  confidence: number | null;
  metadataOnly: true;
  rawAudioPersisted: false;
  textChatFallback: boolean;
};
export type VoiceSynthesisResult = {
  status: "ready" | "degraded" | "muted";
  code: string | null;
  voiceRef: string;
  durationMs: number;
  metadataOnly: true;
  rawAudioPersisted: false;
  textChatFallback: boolean;
};
export type VoiceWakeWordResult = {
  status: "detected" | "ignored" | "degraded";
  code: string | null;
  fixtureId: string;
  detected: boolean;
  listenerActive: false;
  metadataOnly: true;
};
export type VoiceEmotionHypothesisResult = {
  label: "neutral" | "positive" | "concerned";
  confidence: number;
  uncertain: true;
  diagnostic: false;
  source: string;
};

export type ToolClassification = "read_only" | "state_changing";
export type ToolAdapterKind =
  "workspace_mock" | "calendar_mock" | "messaging_mock";
export type ToolPermission =
  "preview" | "execute_read_only" | "execute_state_changing";
export type ToolSessionStatus = "active" | "cancelled" | "closed";
export type ToolActionStatus =
  | "previewed"
  | "approved"
  | "confirmed"
  | "dry_run"
  | "executed"
  | "cancelled"
  | "failed"
  | "compensated"
  | "rejected";
export type ToolResultStatus =
  "dry_run" | "simulated" | "cancelled" | "compensated";

export type ToolManifest = {
  toolId: string;
  manifestVersion: 1;
  name: string;
  classification: ToolClassification;
  adapterKind: ToolAdapterKind;
  scopeKind: "workspace" | "calendar" | "messaging";
  requiresSecondConfirmation: boolean;
  capabilities: string[];
  updatedAt: number;
};
export type ToolSessionPermission = {
  toolId: string;
  permission: ToolPermission;
};
export type ToolSessionRequest = {
  agentId: string;
  scopeRef: string;
  permissions: ToolSessionPermission[];
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ToolSession = {
  id: string;
  agentId: string;
  scopeRef: string;
  status: ToolSessionStatus;
  permissions: ToolSessionPermission[];
  createdAt: number;
  updatedAt: number;
};
export type ToolFileMove = { from: string; to: string };
export type ToolActionInput =
  | { kind: "workspaceInspect"; relativePaths: string[] }
  | { kind: "workspaceOrganize"; moves: ToolFileMove[] }
  | { kind: "calendarList"; date: string }
  | {
      kind: "calendarCreate";
      title: string;
      date: string;
      start: string;
      end: string;
    }
  | { kind: "messagingPreview"; recipient: string; body: string }
  | { kind: "messagingSend"; recipient: string; body: string };
export type ToolActionPreviewRequest = {
  agentId: string;
  sessionId: string;
  toolId: string;
  input: ToolActionInput;
  dryRun: boolean;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ToolActionDecisionRequest = {
  agentId: string;
  actionId: string;
  approved: boolean;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ToolActionConfirmationRequest = {
  agentId: string;
  actionId: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ToolActionExecutionRequest = {
  agentId: string;
  actionId: string;
  dryRun: boolean;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ToolActionCancellationRequest = {
  agentId: string;
  actionId: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ToolSessionCancellationRequest = {
  agentId: string;
  sessionId: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ToolExecutionResult = {
  status: ToolResultStatus;
  output: string;
  changed: false;
  untrusted: true;
};
export type ToolCompensation = {
  kind: string;
  available: boolean;
  description: string;
};
export type ToolAction = {
  id: string;
  sessionId: string;
  agentId: string;
  toolId: string;
  classification: ToolClassification;
  input: ToolActionInput;
  summary: string;
  affectedResources: string[];
  exactEffect: string;
  status: ToolActionStatus;
  dryRun: boolean;
  requiresOwnerApproval: boolean;
  requiresSecondConfirmation: boolean;
  ownerApproved: boolean;
  secondConfirmed: boolean;
  result: ToolExecutionResult | null;
  compensation: ToolCompensation | null;
  code: string | null;
  createdAt: number;
  updatedAt: number;
};
export type ToolAuditRecord = {
  id: string;
  actionId: string | null;
  sessionId: string | null;
  agentId: string;
  toolId: string | null;
  event: string;
  result: string;
  code: string | null;
  summary: string;
  createdAt: number;
};

export const EXTENSION_SDK_VERSION = "aip-extension-sdk/v1" as const;
export type ExtensionCapability =
  "agent_context" | "tool_catalog" | "owner_review";
export type ExtensionSandboxPolicy = "metadata_only";
export type ExtensionAdmissionPolicy = "local_fixture_only";
export type ExtensionSourceKind = "administrator_selected" | "agent_created";
export type ExtensionCatalogScope = "private_local";
export type ExtensionLifecycle =
  | "review_required"
  | "approved"
  | "active"
  | "disabled"
  | "rejected"
  | "recovery_required";
export type ExtensionReviewStatus = "pending" | "approved" | "rejected";
export type ExtensionProposalStatus =
  "pending" | "approved" | "rejected" | "withdrawn";
export type ExtensionPermissionStatus = "pending" | "approved" | "denied";

export type ExtensionManifest = {
  extensionId: string;
  manifestVersion: 1;
  extensionVersion: string;
  sdkVersion: string;
  name: string;
  sandboxPolicy: ExtensionSandboxPolicy;
  admissionPolicy: ExtensionAdmissionPolicy;
  capabilities: ExtensionCapability[];
  localFixtureRef: string | null;
  untrusted: true;
};
export type ExtensionPermissionRequest = {
  capability: ExtensionCapability;
  status: ExtensionPermissionStatus;
};
export type ExtensionCatalogEntry = {
  extensionId: string;
  catalogScope: ExtensionCatalogScope;
  sourceKind: ExtensionSourceKind;
  lifecycle: ExtensionLifecycle;
  reviewStatus: ExtensionReviewStatus;
  manifest: ExtensionManifest;
  currentRevision: number;
  activeRevision: number | null;
  approvedCapabilities: ExtensionCapability[];
  compatible: boolean;
  untrusted: true;
  updatedAt: number;
};
export type ExtensionProposal = {
  id: string;
  extensionId: string;
  revision: number;
  sourceKind: ExtensionSourceKind;
  proposerAgentId: string | null;
  status: ExtensionProposalStatus;
  reviewStatus: ExtensionReviewStatus;
  manifest: ExtensionManifest;
  requestedCapabilities: ExtensionCapability[];
  approvedCapabilities: ExtensionCapability[];
  permissions: ExtensionPermissionRequest[];
  compatible: boolean;
  reviewReason: string | null;
  createdAt: number;
  updatedAt: number;
};
export type ExtensionAuditRecord = {
  id: string;
  extensionId: string | null;
  proposalId: string | null;
  revision: number | null;
  agentId: string;
  event: string;
  result: string;
  code: string | null;
  summary: string;
  createdAt: number;
};
export type ExtensionProposalRequest = {
  agentId: string;
  ownerUserId: string;
  sourceKind: ExtensionSourceKind;
  proposerAgentId: string | null;
  manifest: ExtensionManifest;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ExtensionAgentProposalRequest = {
  agentId: string;
  ownerUserId: string;
  manifest: ExtensionManifest;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ExtensionUpdateRequest = {
  agentId: string;
  ownerUserId: string;
  extensionId: string;
  sourceKind: ExtensionSourceKind;
  proposerAgentId: string | null;
  manifest: ExtensionManifest;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ExtensionReviewRequest = {
  agentId: string;
  ownerUserId: string;
  proposalId: string;
  approved: boolean;
  approvedCapabilities: ExtensionCapability[];
  reason: string | null;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ExtensionActivationRequest = {
  agentId: string;
  ownerUserId: string;
  extensionId: string;
  proposalId: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ExtensionRollbackRequest = {
  agentId: string;
  ownerUserId: string;
  extensionId: string;
  targetRevision: number;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ExtensionDisableRequest = {
  agentId: string;
  ownerUserId: string;
  extensionId: string;
  reason: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};

export type ScreenVisionPermission = "capture_fixture" | "analyze_fixture";
export type ScreenVisionSessionStatus = "active" | "cancelled" | "closed";
export type ScreenVisionJobStatus =
  | "previewed"
  | "queued"
  | "running"
  | "completed"
  | "cancelled"
  | "failed"
  | "cleaned";
export type ScreenVisionModelLifecycle =
  "not_loaded" | "loading" | "ready" | "running" | "unloaded" | "unavailable";
export type ScreenVisionCleanupStatus = "pending" | "complete";
export type ScreenVisionRedactionKind =
  "exclude_sensitive_regions" | "exclude_text_like_regions";
export type ScreenVisionRedactionRule = {
  kind: ScreenVisionRedactionKind;
  enabled: boolean;
};
export type ScreenVisionPrivacyPolicy = {
  excludeSensitiveContent: true;
  redactionRules: ScreenVisionRedactionRule[];
};
export type ScreenVisionFixture = {
  fixtureId: string;
  monitorId: string;
  displayName: string;
  width: number;
  height: number;
  scale: number;
  synthetic: true;
  metadataOnly: true;
};
export type ScreenVisionPreview = {
  fixtureId: string;
  monitorId: string;
  displayName: string;
  width: number;
  height: number;
  synthetic: true;
  metadataOnly: true;
  confirmationRequired: true;
  redactionRuleCount: number;
};
export type ScreenVisionSession = {
  id: string;
  agentId: string;
  ownerUserId: string;
  monitorId: string;
  fixtureId: string;
  status: ScreenVisionSessionStatus;
  permissions: ScreenVisionPermission[];
  privacy: ScreenVisionPrivacyPolicy;
  maxJobs: number;
  maxDurationMs: number;
  createdAt: number;
  updatedAt: number;
  closedAt: number | null;
};
export type ScreenVisionJob = {
  id: string;
  sessionId: string;
  agentId: string;
  ownerUserId: string;
  monitorId: string;
  fixtureId: string;
  modelFixtureId: "fixture:visual-model/screen-neutral-v1";
  resourceKey: "reference-gpu";
  resourceStatus: "available" | "reserved" | "released";
  status: ScreenVisionJobStatus;
  terminalStatus:
    "completed" | "cancelled" | "failed" | "expired" | "cleaned" | null;
  modelLifecycle: ScreenVisionModelLifecycle;
  modelLoadedAt: number | null;
  modelRunAt: number | null;
  modelCleanupAt: number | null;
  cleanupStatus: ScreenVisionCleanupStatus;
  preview: ScreenVisionPreview;
  privacy: ScreenVisionPrivacyPolicy;
  frameMetadataPresent: boolean;
  resultDurable: false;
  errorCode: string | null;
  createdAt: number;
  queuedAt: number | null;
  runningAt: number | null;
  completedAt: number | null;
  cleanedAt: number | null;
  updatedAt: number;
};
export type ScreenVisionHypothesis = {
  text: string;
  confidence: number;
  uncertain: true;
  diagnostic: false;
  durable: false;
  sensitiveAttributeInferred: false;
  source: string;
};
export type ScreenVisionAnalysisResult = {
  job: ScreenVisionJob;
  hypothesis: ScreenVisionHypothesis;
  outputBounded: true;
  screenshotBytesPersisted: false;
};
export type ScreenVisionAuditRecord = {
  id: string;
  sessionId: string | null;
  jobId: string | null;
  agentId: string;
  event: string;
  result: string;
  code: string | null;
  summary: string;
  createdAt: number;
};
export type ScreenVisionSessionRequest = {
  agentId: string;
  ownerUserId: string;
  monitorId: string;
  fixtureId: string;
  permissions: ScreenVisionPermission[];
  privacy: ScreenVisionPrivacyPolicy;
  maxJobs: number;
  maxDurationMs: number;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ScreenVisionJobPreviewRequest = {
  agentId: string;
  ownerUserId: string;
  sessionId: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ScreenVisionJobConfirmationRequest = {
  agentId: string;
  ownerUserId: string;
  jobId: string;
  confirmed: boolean;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ScreenVisionJobCancellationRequest = {
  agentId: string;
  ownerUserId: string;
  jobId: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ScreenVisionJobCleanupRequest = {
  agentId: string;
  ownerUserId: string;
  jobId: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ScreenVisionSessionCancellationRequest = {
  agentId: string;
  ownerUserId: string;
  sessionId: string;
  idempotencyKey: string;
  temporaryChat: boolean;
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
  temporaryChat: boolean;
};
export type OpinionEvidenceCorrectionRequest = {
  agentId: string;
  evidenceId: string;
  claimValue: string;
  reason: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type CognitiveOpinionStatusRequest = {
  agentId: string;
  opinionId: string;
  status: "disputed" | "superseded" | "archived" | "rejected";
  reason: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type CognitiveOpinionRecalculationRequest = {
  agentId: string;
  opinionId: string;
  reason: string;
  idempotencyKey: string;
  temporaryChat: boolean;
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
  temporaryChat: boolean;
};
export type RelationshipResetRequest = {
  agentId: string;
  relationshipId: string;
  reason: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type RelationshipRollbackRequest = {
  agentId: string;
  eventId: string;
  idempotencyKey: string;
  temporaryChat: boolean;
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
  temporaryChat: boolean;
};
export type CognitiveGoalApprovalRequest = {
  agentId: string;
  goalId: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type CognitiveGoalStatusRequest = {
  agentId: string;
  goalId: string;
  status: Exclude<CognitiveGoalStatus, "proposed">;
  completionEvidence: string | null;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type FictionalActivityStatus =
  "active" | "paused" | "completed" | "expired" | "archived";
export type FictionalActivity = {
  id: string;
  goalId: string;
  agentId: string;
  activityType: string;
  status: FictionalActivityStatus;
  fictionalOnly: true;
  budgetUnits: number;
  startedAt: number;
  endedAt: number | null;
  createdAt: number;
};
export type FictionalActivityRequest = {
  agentId: string;
  goalId: string;
  activityType: string;
  budgetUnits: number;
  durationMs: number;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type FictionalActivityStatusRequest = {
  agentId: string;
  activityId: string;
  status: FictionalActivityStatus;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ConversationPolicy = {
  agentId: string;
  purpose: string;
  optedIn: boolean;
  maxTurns: number;
  maxTokens: number;
  maxDurationMs: number;
  maxRepetitions: number;
  resourceBudget: number;
  revokedAt: number | null;
  updatedAt: number;
};
export type ConversationPolicyRequest = {
  agentId: string;
  purpose: string;
  optedIn: boolean;
  maxTurns: number;
  maxTokens: number;
  maxDurationMs: number;
  maxRepetitions: number;
  resourceBudget: number;
  temporaryChat: boolean;
};
export type ConversationStartRequest = {
  initiatorAgentId: string;
  participantAgentId: string;
  purpose: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type AgentConversationStatus =
  "active" | "completed" | "cancelled" | "suspended" | "rejected";
export type AgentConversationSummary = {
  id: string;
  initiatorAgentId: string;
  participantAgentId: string;
  purpose: string;
  status: AgentConversationStatus;
  maxTurns: number;
  maxTokens: number;
  maxDurationMs: number;
  maxRepetitions: number;
  resourceBudget: number;
  turnCount: number;
  tokenCount: number;
  loopCount: number;
  terminationReason: string | null;
  createdAt: number;
  updatedAt: number;
  completedAt: number | null;
};
export type PublicConversationTurn = {
  id: string;
  conversationId: string;
  speakerAgentId: string;
  turnIndex: number;
  content: string;
  sourceKind: "owner" | "model_candidate";
  createdAt: number;
};
export type AgentConversationInspection = {
  conversation: AgentConversationSummary;
  turns: PublicConversationTurn[];
};
export type PublicConversationTurnRequest = {
  agentId: string;
  conversationId: string;
  speakerAgentId: string;
  content: string;
  sourceKind: "owner" | "model_candidate";
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ConversationInterruptRequest = {
  agentId: string;
  conversationId: string;
  reason: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type CognitiveCandidateRequest = {
  agentId: string;
  conversationId: string;
  candidateKind: "opinion" | "relationship" | "goal";
  candidateJson: string;
  idempotencyKey: string;
};
export type CognitiveCandidate = {
  id: string;
  conversationId: string;
  agentId: string;
  candidateKind: "opinion" | "relationship" | "goal";
  candidateJson: string;
  sourceReference: string;
  status: "pending" | "applied" | "rejected";
  createdAt: number;
};
export type CognitiveCandidateRejectionRequest = {
  agentId: string;
  candidateId: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type HeavyGenerationRequest = {
  agentId: string;
  conversationId: string;
  priority: number;
  budgetUnits: number;
  idempotencyKey: string;
};
export type CognitiveResourceJobStatus =
  "queued" | "running" | "completed" | "cancelled" | "failed";
export type CognitiveResourceJob = {
  id: string;
  agentId: string;
  conversationId: string | null;
  jobKind: string;
  heavy: boolean;
  priority: number;
  budgetUnits: number;
  status: CognitiveResourceJobStatus;
  errorCode: string | null;
  createdAt: number;
  startedAt: number | null;
  endedAt: number | null;
};
export type ResourceJobCompletionRequest = {
  agentId: string;
  jobId: string;
  status: "completed" | "cancelled" | "failed";
  errorCode: string | null;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type OwnerCorrectionRequest = {
  agentId: string;
  traitKey: string;
  value: number;
  reason: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type RollbackRequest = {
  agentId: string;
  eventId: string;
  idempotencyKey: string;
  temporaryChat: boolean;
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
  | "invalid_activity"
  | "invalid_activity_budget"
  | "activity_not_found"
  | "conversation_temporary_blocked"
  | "conversation_purpose_invalid"
  | "conversation_budget_invalid"
  | "conversation_opt_in_required"
  | "conversation_participant_invalid"
  | "conversation_blocked_safe_mode"
  | "conversation_blocked_silent"
  | "conversation_blocked_suspended"
  | "conversation_not_found"
  | "conversation_not_active"
  | "conversation_not_completed"
  | "conversation_turn_invalid"
  | "conversation_turn_limit"
  | "conversation_token_limit"
  | "conversation_duration_limit"
  | "conversation_candidate_invalid"
  | "candidate_not_found"
  | "candidate_already_decided"
  | "heavy_generation_busy"
  | "invalid_resource_status"
  | "resource_job_not_found"
  | "persistence_failed"
  | "voice_settings_not_found"
  | "voice_reference_invalid"
  | "voice_consent_invalid"
  | "voice_blocked_silent"
  | "voice_blocked_suspended"
  | "voice_fixture_unavailable"
  | "voice_device_unavailable"
  | "voice_model_unavailable"
  | "voice_device_or_model_unavailable"
  | "voice_input_invalid"
  | "voice_muted"
  | "tools_blocked_temporary"
  | "tools_blocked_safe_mode"
  | "tool_not_found"
  | "tool_manifest_invalid"
  | "tool_scope_invalid"
  | "tool_permission_invalid"
  | "tool_permission_denied"
  | "tool_session_limit"
  | "tool_session_not_found"
  | "tool_session_cancelled"
  | "tool_input_invalid"
  | "tool_output_oversized"
  | "tool_audit_oversized"
  | "tool_action_not_found"
  | "tool_action_invalid"
  | "tool_action_not_executable"
  | "tool_action_already_completed"
  | "tool_action_not_approvable"
  | "tool_approval_not_required"
  | "tool_approval_required"
  | "tool_action_rejected"
  | "tool_action_cancelled"
  | "tool_confirmation_not_required"
  | "tool_confirmation_required"
  | "tool_action_not_confirmable"
  | "tool_compensation_unavailable"
  | "extensions_blocked_temporary"
  | "extensions_blocked_safe_mode"
  | "extension_already_exists"
  | "extension_not_found"
  | "extension_proposal_not_found"
  | "extension_revision_not_found"
  | "extension_manifest_invalid"
  | "extension_manifest_oversized"
  | "extension_sdk_incompatible"
  | "extension_id_invalid"
  | "extension_identity_invalid"
  | "extension_version_invalid"
  | "extension_text_invalid"
  | "extension_fixture_invalid"
  | "extension_sandbox_invalid"
  | "extension_admission_denied"
  | "extension_untrusted_required"
  | "extension_capability_invalid"
  | "extension_capability_expansion"
  | "extension_source_invalid"
  | "extension_revision_invalid"
  | "extension_proposal_invalid"
  | "extension_review_required"
  | "extension_review_reason_required"
  | "extension_permission_invalid"
  | "extension_permission_required"
  | "extension_update_requires_review"
  | "extension_rollback_unavailable"
  | "extension_audit_oversized"
  | "extension_owner_required"
  | "extension_proposal_self_review"
  | "extension_request_oversized"
  | "extension_result_oversized"
  | "extension_idempotency_invalid"
  | "screen_vision_blocked_temporary"
  | "screen_vision_blocked_safe_mode"
  | "screen_vision_blocked_suspended"
  | "screen_vision_owner_required"
  | "screen_vision_agent_invalid"
  | "screen_vision_session_not_found"
  | "screen_vision_job_not_found"
  | "screen_vision_fixture_invalid"
  | "screen_vision_permission_invalid"
  | "screen_vision_privacy_invalid"
  | "screen_vision_quota_invalid"
  | "screen_vision_session_limit"
  | "screen_vision_job_limit"
  | "screen_vision_session_cancelled"
  | "screen_vision_confirmation_required"
  | "screen_vision_job_invalid"
  | "screen_vision_resource_busy"
  | "screen_vision_request_oversized"
  | "screen_vision_result_oversized"
  | "screen_vision_audit_oversized"
  | "screen_vision_idempotency_invalid";
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
    "invalid_activity",
    "invalid_activity_budget",
    "activity_not_found",
    "conversation_temporary_blocked",
    "conversation_purpose_invalid",
    "conversation_budget_invalid",
    "conversation_opt_in_required",
    "conversation_participant_invalid",
    "conversation_blocked_safe_mode",
    "conversation_blocked_silent",
    "conversation_blocked_suspended",
    "conversation_not_found",
    "conversation_not_active",
    "conversation_not_completed",
    "conversation_turn_invalid",
    "conversation_turn_limit",
    "conversation_token_limit",
    "conversation_duration_limit",
    "conversation_candidate_invalid",
    "candidate_not_found",
    "candidate_already_decided",
    "heavy_generation_busy",
    "invalid_resource_status",
    "resource_job_not_found",
    "persistence_failed",
    "voice_settings_not_found",
    "voice_reference_invalid",
    "voice_consent_invalid",
    "voice_blocked_silent",
    "voice_blocked_suspended",
    "voice_fixture_unavailable",
    "voice_device_unavailable",
    "voice_model_unavailable",
    "voice_device_or_model_unavailable",
    "voice_input_invalid",
    "voice_muted",
    "tools_blocked_temporary",
    "tools_blocked_safe_mode",
    "tool_not_found",
    "tool_manifest_invalid",
    "tool_scope_invalid",
    "tool_permission_invalid",
    "tool_permission_denied",
    "tool_session_limit",
    "tool_session_not_found",
    "tool_session_cancelled",
    "tool_input_invalid",
    "tool_output_oversized",
    "tool_audit_oversized",
    "tool_action_not_found",
    "tool_action_invalid",
    "tool_action_not_executable",
    "tool_action_already_completed",
    "tool_action_not_approvable",
    "tool_approval_not_required",
    "tool_approval_required",
    "tool_action_rejected",
    "tool_action_cancelled",
    "tool_confirmation_not_required",
    "tool_confirmation_required",
    "tool_action_not_confirmable",
    "tool_compensation_unavailable",
    "extensions_blocked_temporary",
    "extensions_blocked_safe_mode",
    "extension_already_exists",
    "extension_not_found",
    "extension_proposal_not_found",
    "extension_revision_not_found",
    "extension_manifest_invalid",
    "extension_manifest_oversized",
    "extension_sdk_incompatible",
    "extension_id_invalid",
    "extension_identity_invalid",
    "extension_version_invalid",
    "extension_text_invalid",
    "extension_fixture_invalid",
    "extension_sandbox_invalid",
    "extension_admission_denied",
    "extension_untrusted_required",
    "extension_capability_invalid",
    "extension_capability_expansion",
    "extension_source_invalid",
    "extension_revision_invalid",
    "extension_proposal_invalid",
    "extension_review_required",
    "extension_review_reason_required",
    "extension_permission_invalid",
    "extension_permission_required",
    "extension_update_requires_review",
    "extension_rollback_unavailable",
    "extension_audit_oversized",
    "screen_vision_blocked_temporary",
    "screen_vision_blocked_safe_mode",
    "screen_vision_blocked_suspended",
    "screen_vision_owner_required",
    "screen_vision_agent_invalid",
    "screen_vision_session_not_found",
    "screen_vision_job_not_found",
    "screen_vision_fixture_invalid",
    "screen_vision_permission_invalid",
    "screen_vision_privacy_invalid",
    "screen_vision_quota_invalid",
    "screen_vision_session_limit",
    "screen_vision_job_limit",
    "screen_vision_session_cancelled",
    "screen_vision_confirmation_required",
    "screen_vision_job_invalid",
    "screen_vision_resource_busy",
    "screen_vision_request_oversized",
    "screen_vision_result_oversized",
    "screen_vision_audit_oversized",
    "screen_vision_idempotency_invalid",
  ];
  return codes.includes(candidate.code as CognitiveErrorCode) &&
    cognitiveString(candidate.message)
    ? (candidate as CognitiveErrorResponse)
    : null;
}

export function parseVoiceSettings(value: unknown): VoiceSettings | null {
  if (typeof value !== "object" || value === null) return null;
  const candidate = value as Partial<VoiceSettings>;
  return cognitiveString(candidate.agentId) &&
    candidate.schemaVersion === 1 &&
    cognitiveString(candidate.baseVoiceId) &&
    candidate.baseVoiceProtected === true &&
    (candidate.customVoiceRef === null ||
      cognitiveString(candidate.customVoiceRef)) &&
    ["not_granted", "granted", "revoked"].includes(
      candidate.customVoiceConsent ?? "",
    ) &&
    (candidate.recognitionModelRef === null ||
      cognitiveString(candidate.recognitionModelRef)) &&
    (candidate.synthesisModelRef === null ||
      cognitiveString(candidate.synthesisModelRef)) &&
    (candidate.inputDeviceRef === null ||
      cognitiveString(candidate.inputDeviceRef)) &&
    (candidate.outputDeviceRef === null ||
      cognitiveString(candidate.outputDeviceRef)) &&
    ["normal", "voice_muted", "silent"].includes(candidate.mode ?? "") &&
    typeof candidate.voiceMuted === "boolean" &&
    typeof candidate.silent === "boolean" &&
    typeof candidate.suspended === "boolean" &&
    cognitiveNumber(candidate.updatedAt)
    ? (candidate as VoiceSettings)
    : null;
}

export function parseVoiceTranscriptionResult(
  value: unknown,
): VoiceTranscriptionResult | null {
  if (typeof value !== "object" || value === null) return null;
  const candidate = value as Partial<VoiceTranscriptionResult>;
  return ["ready", "degraded"].includes(candidate.status ?? "") &&
    (candidate.code === null || cognitiveString(candidate.code)) &&
    cognitiveString(candidate.fixtureId) &&
    (candidate.text === null || cognitiveString(candidate.text)) &&
    (candidate.confidence === null || cognitiveNumber(candidate.confidence)) &&
    candidate.metadataOnly === true &&
    candidate.rawAudioPersisted === false &&
    typeof candidate.textChatFallback === "boolean"
    ? (candidate as VoiceTranscriptionResult)
    : null;
}

export function parseVoiceSynthesisResult(
  value: unknown,
): VoiceSynthesisResult | null {
  if (typeof value !== "object" || value === null) return null;
  const candidate = value as Partial<VoiceSynthesisResult>;
  return ["ready", "degraded", "muted"].includes(candidate.status ?? "") &&
    (candidate.code === null || cognitiveString(candidate.code)) &&
    cognitiveString(candidate.voiceRef) &&
    cognitiveNumber(candidate.durationMs) &&
    candidate.metadataOnly === true &&
    candidate.rawAudioPersisted === false &&
    typeof candidate.textChatFallback === "boolean"
    ? (candidate as VoiceSynthesisResult)
    : null;
}

export function parseVoiceWakeWordResult(
  value: unknown,
): VoiceWakeWordResult | null {
  if (typeof value !== "object" || value === null) return null;
  const candidate = value as Partial<VoiceWakeWordResult>;
  return ["detected", "ignored", "degraded"].includes(candidate.status ?? "") &&
    (candidate.code === null || cognitiveString(candidate.code)) &&
    cognitiveString(candidate.fixtureId) &&
    typeof candidate.detected === "boolean" &&
    candidate.listenerActive === false &&
    candidate.metadataOnly === true
    ? (candidate as VoiceWakeWordResult)
    : null;
}

export function parseVoiceEmotionHypothesis(
  value: unknown,
): VoiceEmotionHypothesisResult | null {
  if (typeof value !== "object" || value === null) return null;
  const candidate = value as Partial<VoiceEmotionHypothesisResult>;
  return ["neutral", "positive", "concerned"].includes(candidate.label ?? "") &&
    cognitiveNumber(candidate.confidence) &&
    candidate.uncertain === true &&
    candidate.diagnostic === false &&
    cognitiveString(candidate.source)
    ? (candidate as VoiceEmotionHypothesisResult)
    : null;
}

function toolRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function toolStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every(cognitiveString);
}

function isToolClassification(value: unknown): value is ToolClassification {
  return value === "read_only" || value === "state_changing";
}

function isToolAdapterKind(value: unknown): value is ToolAdapterKind {
  return (
    value === "workspace_mock" ||
    value === "calendar_mock" ||
    value === "messaging_mock"
  );
}

function isToolPermission(value: unknown): value is ToolPermission {
  return (
    value === "preview" ||
    value === "execute_read_only" ||
    value === "execute_state_changing"
  );
}

function isToolActionStatus(value: unknown): value is ToolActionStatus {
  return [
    "previewed",
    "approved",
    "confirmed",
    "dry_run",
    "executed",
    "cancelled",
    "failed",
    "compensated",
    "rejected",
  ].includes(value as string);
}

function isToolResultStatus(value: unknown): value is ToolResultStatus {
  return ["dry_run", "simulated", "cancelled", "compensated"].includes(
    value as string,
  );
}

export function parseToolActionInput(value: unknown): ToolActionInput | null {
  const candidate = toolRecord(value);
  if (!candidate || !cognitiveString(candidate.kind)) return null;
  switch (candidate.kind) {
    case "workspaceInspect":
      return toolStringArray(candidate.relativePaths)
        ? (candidate as unknown as ToolActionInput)
        : null;
    case "workspaceOrganize": {
      if (!Array.isArray(candidate.moves)) return null;
      const moves = candidate.moves.every((move) => {
        const item = toolRecord(move);
        return (
          item !== null &&
          cognitiveString(item.from) &&
          cognitiveString(item.to)
        );
      });
      return moves ? (candidate as unknown as ToolActionInput) : null;
    }
    case "calendarList":
      return cognitiveString(candidate.date)
        ? (candidate as unknown as ToolActionInput)
        : null;
    case "calendarCreate":
      return cognitiveString(candidate.title) &&
        cognitiveString(candidate.date) &&
        cognitiveString(candidate.start) &&
        cognitiveString(candidate.end)
        ? (candidate as unknown as ToolActionInput)
        : null;
    case "messagingPreview":
    case "messagingSend":
      return cognitiveString(candidate.recipient) &&
        cognitiveString(candidate.body)
        ? (candidate as unknown as ToolActionInput)
        : null;
    default:
      return null;
  }
}

export function parseToolManifest(value: unknown): ToolManifest | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    cognitiveString(candidate.toolId) &&
    candidate.manifestVersion === 1 &&
    cognitiveString(candidate.name) &&
    isToolClassification(candidate.classification) &&
    isToolAdapterKind(candidate.adapterKind) &&
    ["workspace", "calendar", "messaging"].includes(
      candidate.scopeKind as string,
    ) &&
    typeof candidate.requiresSecondConfirmation === "boolean" &&
    toolStringArray(candidate.capabilities) &&
    cognitiveNumber(candidate.updatedAt)
    ? (candidate as unknown as ToolManifest)
    : null;
}

export function parseToolSessionPermission(
  value: unknown,
): ToolSessionPermission | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    cognitiveString(candidate.toolId) &&
    isToolPermission(candidate.permission)
    ? (candidate as unknown as ToolSessionPermission)
    : null;
}

export function parseToolSession(value: unknown): ToolSession | null {
  const candidate = toolRecord(value);
  if (!candidate || !Array.isArray(candidate.permissions)) return null;
  const permissions = candidate.permissions.map(parseToolSessionPermission);
  return cognitiveString(candidate.id) &&
    cognitiveString(candidate.agentId) &&
    cognitiveString(candidate.scopeRef) &&
    ["active", "cancelled", "closed"].includes(candidate.status as string) &&
    permissions.every(
      (permission): permission is ToolSessionPermission => permission !== null,
    ) &&
    cognitiveNumber(candidate.createdAt) &&
    cognitiveNumber(candidate.updatedAt)
    ? (candidate as unknown as ToolSession)
    : null;
}

export function parseToolExecutionResult(
  value: unknown,
): ToolExecutionResult | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    isToolResultStatus(candidate.status) &&
    cognitiveString(candidate.output) &&
    candidate.changed === false &&
    candidate.untrusted === true
    ? (candidate as unknown as ToolExecutionResult)
    : null;
}

export function parseToolCompensation(value: unknown): ToolCompensation | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    cognitiveString(candidate.kind) &&
    typeof candidate.available === "boolean" &&
    cognitiveString(candidate.description)
    ? (candidate as unknown as ToolCompensation)
    : null;
}

export function parseToolAction(value: unknown): ToolAction | null {
  const candidate = toolRecord(value);
  const result =
    candidate?.result === null
      ? null
      : parseToolExecutionResult(candidate?.result);
  const compensation =
    candidate?.compensation === null
      ? null
      : parseToolCompensation(candidate?.compensation);
  return candidate !== null &&
    cognitiveString(candidate.id) &&
    cognitiveString(candidate.sessionId) &&
    cognitiveString(candidate.agentId) &&
    cognitiveString(candidate.toolId) &&
    isToolClassification(candidate.classification) &&
    parseToolActionInput(candidate.input) !== null &&
    cognitiveString(candidate.summary) &&
    toolStringArray(candidate.affectedResources) &&
    cognitiveString(candidate.exactEffect) &&
    isToolActionStatus(candidate.status) &&
    typeof candidate.dryRun === "boolean" &&
    typeof candidate.requiresOwnerApproval === "boolean" &&
    typeof candidate.requiresSecondConfirmation === "boolean" &&
    typeof candidate.ownerApproved === "boolean" &&
    typeof candidate.secondConfirmed === "boolean" &&
    (candidate.result === null || result !== null) &&
    (candidate.compensation === null || compensation !== null) &&
    (candidate.code === null || cognitiveString(candidate.code)) &&
    cognitiveNumber(candidate.createdAt) &&
    cognitiveNumber(candidate.updatedAt)
    ? (candidate as unknown as ToolAction)
    : null;
}

export function parseToolAuditRecord(value: unknown): ToolAuditRecord | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    cognitiveString(candidate.id) &&
    (candidate.actionId === null || cognitiveString(candidate.actionId)) &&
    (candidate.sessionId === null || cognitiveString(candidate.sessionId)) &&
    cognitiveString(candidate.agentId) &&
    (candidate.toolId === null || cognitiveString(candidate.toolId)) &&
    cognitiveString(candidate.event) &&
    cognitiveString(candidate.result) &&
    (candidate.code === null || cognitiveString(candidate.code)) &&
    cognitiveString(candidate.summary) &&
    cognitiveNumber(candidate.createdAt)
    ? (candidate as unknown as ToolAuditRecord)
    : null;
}

export function parseToolCatalog(value: unknown): ToolManifest[] | null {
  if (!Array.isArray(value)) return null;
  const manifests = value.map(parseToolManifest);
  return manifests.every(
    (manifest): manifest is ToolManifest => manifest !== null,
  )
    ? manifests
    : null;
}

export function parseToolSessions(value: unknown): ToolSession[] | null {
  if (!Array.isArray(value)) return null;
  const sessions = value.map(parseToolSession);
  return sessions.every((session): session is ToolSession => session !== null)
    ? sessions
    : null;
}

export function parseToolAudit(value: unknown): ToolAuditRecord[] | null {
  if (!Array.isArray(value)) return null;
  const records = value.map(parseToolAuditRecord);
  return records.every((record): record is ToolAuditRecord => record !== null)
    ? records
    : null;
}

function isExtensionCapability(value: unknown): value is ExtensionCapability {
  return (
    value === "agent_context" ||
    value === "tool_catalog" ||
    value === "owner_review"
  );
}

const MAX_EXTENSION_ID_LENGTH = 96;
const MAX_EXTENSION_VERSION_LENGTH = 32;
const MAX_EXTENSION_SDK_LENGTH = 64;
const MAX_EXTENSION_NAME_LENGTH = 160;
const MAX_EXTENSION_CAPABILITIES = 8;
const MAX_EXTENSION_FIXTURE_LENGTH = 160;
const MAX_EXTENSION_RECORD_ID_LENGTH = 128;
const MAX_EXTENSION_AGENT_ID_LENGTH = 96;
const MAX_EXTENSION_REASON_LENGTH = 512;
const MAX_EXTENSION_AUDIT_TEXT_LENGTH = 2048;

function isExtensionBoundedText(
  value: unknown,
  maximum: number,
): value is string {
  return (
    cognitiveString(value) &&
    value.length > 0 &&
    value.length <= maximum &&
    !Array.from(value).some((character) => {
      const code = character.charCodeAt(0);
      return code < 32 || code === 127;
    })
  );
}

function isExtensionId(value: unknown): value is string {
  return (
    isExtensionBoundedText(value, MAX_EXTENSION_ID_LENGTH) &&
    !value.startsWith(".") &&
    !value.endsWith(".") &&
    /^[a-z0-9._-]+$/.test(value)
  );
}

function isExtensionVersion(value: unknown): value is string {
  return (
    isExtensionBoundedText(value, MAX_EXTENSION_VERSION_LENGTH) &&
    /^(0|[1-9]\d{0,5})\.(0|[1-9]\d{0,5})\.(0|[1-9]\d{0,5})$/.test(value)
  );
}

function isExtensionFixtureRef(value: unknown): value is string {
  return (
    isExtensionBoundedText(value, MAX_EXTENSION_FIXTURE_LENGTH) &&
    value.startsWith("fixture:extension/") &&
    !value.includes("..") &&
    !value.includes("\\") &&
    /^[A-Za-z0-9._:/-]+$/.test(value)
  );
}

function isExtensionRecordId(value: unknown): value is string {
  return isExtensionBoundedText(value, MAX_EXTENSION_RECORD_ID_LENGTH);
}

function isExtensionAgentId(value: unknown): value is string {
  return isExtensionBoundedText(value, MAX_EXTENSION_AGENT_ID_LENGTH);
}

function isExtensionRevision(value: unknown): value is number {
  return (
    typeof value === "number" &&
    Number.isSafeInteger(value) &&
    value >= 1 &&
    value <= 2_147_483_647
  );
}

function isExtensionTimestamp(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function parseExtensionCapabilities(
  value: unknown,
): ExtensionCapability[] | null {
  if (
    !Array.isArray(value) ||
    value.length > MAX_EXTENSION_CAPABILITIES ||
    !value.every(isExtensionCapability)
  ) {
    return null;
  }
  return new Set(value).size === value.length
    ? (value as ExtensionCapability[])
    : null;
}

function hasSameExtensionCapabilities(
  left: ExtensionCapability[],
  right: ExtensionCapability[],
): boolean {
  return (
    left.length === right.length &&
    left.every((capability) => right.includes(capability))
  );
}

export function parseExtensionManifest(
  value: unknown,
): ExtensionManifest | null {
  const candidate = toolRecord(value);
  const capabilities = parseExtensionCapabilities(candidate?.capabilities);
  if (
    candidate !== null &&
    [
      "code",
      "entrypoint",
      "source",
      "path",
      "url",
      "command",
      "module",
      "script",
      "binary",
      "package",
      "network",
      "hostAccess",
      "shell",
      "credentials",
      "executable",
      "runtime",
    ].some((key) => key in candidate)
  ) {
    return null;
  }
  return candidate !== null &&
    isExtensionId(candidate.extensionId) &&
    candidate.manifestVersion === 1 &&
    isExtensionVersion(candidate.extensionVersion) &&
    isExtensionBoundedText(candidate.sdkVersion, MAX_EXTENSION_SDK_LENGTH) &&
    candidate.sdkVersion === EXTENSION_SDK_VERSION &&
    isExtensionBoundedText(candidate.name, MAX_EXTENSION_NAME_LENGTH) &&
    candidate.sandboxPolicy === "metadata_only" &&
    candidate.admissionPolicy === "local_fixture_only" &&
    capabilities !== null &&
    (candidate.localFixtureRef === null ||
      isExtensionFixtureRef(candidate.localFixtureRef)) &&
    candidate.untrusted === true
    ? (candidate as unknown as ExtensionManifest)
    : null;
}

export function parseExtensionPermissionRequest(
  value: unknown,
): ExtensionPermissionRequest | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    isExtensionCapability(candidate.capability) &&
    ["pending", "approved", "denied"].includes(candidate.status as string)
    ? (candidate as unknown as ExtensionPermissionRequest)
    : null;
}

export function parseExtensionCatalogEntry(
  value: unknown,
): ExtensionCatalogEntry | null {
  const candidate = toolRecord(value);
  const manifest = parseExtensionManifest(candidate?.manifest);
  const approvedCapabilities = parseExtensionCapabilities(
    candidate?.approvedCapabilities,
  );
  return candidate !== null &&
    isExtensionId(candidate.extensionId) &&
    candidate.catalogScope === "private_local" &&
    ["administrator_selected", "agent_created"].includes(
      candidate.sourceKind as string,
    ) &&
    [
      "review_required",
      "approved",
      "active",
      "disabled",
      "rejected",
      "recovery_required",
    ].includes(candidate.lifecycle as string) &&
    ["pending", "approved", "rejected"].includes(
      candidate.reviewStatus as string,
    ) &&
    manifest !== null &&
    manifest.extensionId === candidate.extensionId &&
    isExtensionRevision(candidate.currentRevision) &&
    (candidate.activeRevision === null ||
      isExtensionRevision(candidate.activeRevision)) &&
    approvedCapabilities !== null &&
    approvedCapabilities.every((capability) =>
      manifest.capabilities.includes(capability),
    ) &&
    typeof candidate.compatible === "boolean" &&
    candidate.compatible === (manifest.sdkVersion === EXTENSION_SDK_VERSION) &&
    candidate.untrusted === true &&
    isExtensionTimestamp(candidate.updatedAt)
    ? (candidate as unknown as ExtensionCatalogEntry)
    : null;
}

export function parseExtensionProposal(
  value: unknown,
): ExtensionProposal | null {
  const candidate = toolRecord(value);
  const manifest = parseExtensionManifest(candidate?.manifest);
  const requestedCapabilities = parseExtensionCapabilities(
    candidate?.requestedCapabilities,
  );
  const approvedCapabilities = parseExtensionCapabilities(
    candidate?.approvedCapabilities,
  );
  const permissions = Array.isArray(candidate?.permissions)
    ? candidate.permissions.map(parseExtensionPermissionRequest)
    : null;
  return candidate !== null &&
    isExtensionRecordId(candidate.id) &&
    isExtensionId(candidate.extensionId) &&
    isExtensionRevision(candidate.revision) &&
    ["administrator_selected", "agent_created"].includes(
      candidate.sourceKind as string,
    ) &&
    (candidate.sourceKind === "administrator_selected"
      ? candidate.proposerAgentId === null
      : isExtensionAgentId(candidate.proposerAgentId)) &&
    ["pending", "approved", "rejected", "withdrawn"].includes(
      candidate.status as string,
    ) &&
    ["pending", "approved", "rejected"].includes(
      candidate.reviewStatus as string,
    ) &&
    manifest !== null &&
    manifest.extensionId === candidate.extensionId &&
    requestedCapabilities !== null &&
    hasSameExtensionCapabilities(
      requestedCapabilities,
      manifest.capabilities,
    ) &&
    approvedCapabilities !== null &&
    approvedCapabilities.every((capability) =>
      requestedCapabilities.includes(capability),
    ) &&
    permissions !== null &&
    permissions.length <= MAX_EXTENSION_CAPABILITIES &&
    permissions.every(
      (permission): permission is ExtensionPermissionRequest =>
        permission !== null,
    ) &&
    new Set(permissions.map((permission) => permission.capability)).size ===
      permissions.length &&
    permissions.every((permission) =>
      requestedCapabilities.includes(permission.capability),
    ) &&
    typeof candidate.compatible === "boolean" &&
    candidate.compatible === (manifest.sdkVersion === EXTENSION_SDK_VERSION) &&
    (candidate.reviewReason === null ||
      isExtensionBoundedText(
        candidate.reviewReason,
        MAX_EXTENSION_REASON_LENGTH,
      )) &&
    isExtensionTimestamp(candidate.createdAt) &&
    isExtensionTimestamp(candidate.updatedAt)
    ? (candidate as unknown as ExtensionProposal)
    : null;
}

export function parseExtensionAuditRecord(
  value: unknown,
): ExtensionAuditRecord | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    isExtensionRecordId(candidate.id) &&
    (candidate.extensionId === null || isExtensionId(candidate.extensionId)) &&
    (candidate.proposalId === null ||
      isExtensionRecordId(candidate.proposalId)) &&
    (candidate.revision === null || isExtensionRevision(candidate.revision)) &&
    isExtensionAgentId(candidate.agentId) &&
    isExtensionBoundedText(candidate.event, 64) &&
    isExtensionBoundedText(candidate.result, 64) &&
    (candidate.code === null || isExtensionBoundedText(candidate.code, 96)) &&
    isExtensionBoundedText(
      candidate.summary,
      MAX_EXTENSION_AUDIT_TEXT_LENGTH,
    ) &&
    isExtensionTimestamp(candidate.createdAt)
    ? (candidate as unknown as ExtensionAuditRecord)
    : null;
}

export function parseExtensionCatalog(
  value: unknown,
): ExtensionCatalogEntry[] | null {
  if (!Array.isArray(value) || value.length > 64) return null;
  const entries = value.map(parseExtensionCatalogEntry);
  return entries.every(
    (entry): entry is ExtensionCatalogEntry => entry !== null,
  )
    ? entries
    : null;
}

export function parseExtensionProposals(
  value: unknown,
): ExtensionProposal[] | null {
  if (!Array.isArray(value) || value.length > 64) return null;
  const proposals = value.map(parseExtensionProposal);
  return proposals.every(
    (proposal): proposal is ExtensionProposal => proposal !== null,
  )
    ? proposals
    : null;
}

export function parseExtensionAudit(
  value: unknown,
): ExtensionAuditRecord[] | null {
  if (!Array.isArray(value) || value.length > 100) return null;
  const records = value.map(parseExtensionAuditRecord);
  return records.every(
    (record): record is ExtensionAuditRecord => record !== null,
  )
    ? records
    : null;
}

const SCREEN_VISION_MODEL_FIXTURE_ID = "fixture:visual-model/screen-neutral-v1";
const SCREEN_VISION_RESOURCE_KEY = "reference-gpu";
const MAX_SCREEN_VISION_TEXT = 512;
const MAX_SCREEN_VISION_RESULT_TEXT = 1_024;

function isScreenVisionBoundedText(
  value: unknown,
  maximum: number,
): value is string {
  return (
    cognitiveString(value) &&
    value.length > 0 &&
    value.length <= maximum &&
    !Array.from(value).some((character) => {
      const code = character.charCodeAt(0);
      return code < 32 || code === 127;
    })
  );
}

function isScreenVisionReference(
  value: unknown,
  maximum: number,
): value is string {
  return (
    isScreenVisionBoundedText(value, maximum) &&
    !value.includes("..") &&
    !value.includes("\\") &&
    /^[A-Za-z0-9:._/-]+$/.test(value)
  );
}

function isScreenVisionTimestamp(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function isScreenVisionBoundedInteger(
  value: unknown,
  minimum: number,
  maximum: number,
): value is number {
  return (
    typeof value === "number" &&
    Number.isSafeInteger(value) &&
    value >= minimum &&
    value <= maximum
  );
}

function hasScreenVisionForbiddenField(
  candidate: Record<string, unknown>,
): boolean {
  return [
    "screenshot",
    "screenshotBytes",
    "pixelData",
    "imageBytes",
    "imageData",
    "pixels",
    "capturePath",
    "filePath",
    "networkUrl",
    "remoteModelRef",
  ].some((key) => key in candidate);
}

function isScreenVisionPermission(
  value: unknown,
): value is ScreenVisionPermission {
  return value === "capture_fixture" || value === "analyze_fixture";
}

function parseScreenVisionPermissions(
  value: unknown,
): ScreenVisionPermission[] | null {
  if (
    !Array.isArray(value) ||
    value.length !== 2 ||
    !value.every(isScreenVisionPermission)
  ) {
    return null;
  }
  const permissions = value as ScreenVisionPermission[];
  return new Set(permissions).size === 2 &&
    permissions.includes("capture_fixture") &&
    permissions.includes("analyze_fixture")
    ? permissions
    : null;
}

export function parseScreenVisionPrivacy(
  value: unknown,
): ScreenVisionPrivacyPolicy | null {
  const candidate = toolRecord(value);
  const redactionRules = Array.isArray(candidate?.redactionRules)
    ? candidate.redactionRules.map((rule) => {
        const parsed = toolRecord(rule);
        return parsed !== null &&
          ["exclude_sensitive_regions", "exclude_text_like_regions"].includes(
            parsed.kind as string,
          ) &&
          typeof parsed.enabled === "boolean"
          ? (parsed as unknown as ScreenVisionRedactionRule)
          : null;
      })
    : null;
  return candidate !== null &&
    candidate.excludeSensitiveContent === true &&
    redactionRules !== null &&
    redactionRules.length > 0 &&
    redactionRules.length <= 8 &&
    redactionRules.every(
      (rule): rule is ScreenVisionRedactionRule =>
        rule !== null && rule.enabled,
    ) &&
    redactionRules.some((rule) => rule.kind === "exclude_sensitive_regions")
    ? (candidate as unknown as ScreenVisionPrivacyPolicy)
    : null;
}

export function parseScreenVisionFixture(
  value: unknown,
): ScreenVisionFixture | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasScreenVisionForbiddenField(candidate) &&
    isScreenVisionReference(candidate.fixtureId, 160) &&
    isScreenVisionReference(candidate.monitorId, 64) &&
    isScreenVisionBoundedText(candidate.displayName, 160) &&
    isScreenVisionBoundedInteger(candidate.width, 1, 16_384) &&
    isScreenVisionBoundedInteger(candidate.height, 1, 16_384) &&
    cognitiveNumber(candidate.scale) &&
    candidate.scale > 0 &&
    candidate.scale <= 4 &&
    candidate.synthetic === true &&
    candidate.metadataOnly === true
    ? (candidate as unknown as ScreenVisionFixture)
    : null;
}

export function parseScreenVisionPreview(
  value: unknown,
): ScreenVisionPreview | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasScreenVisionForbiddenField(candidate) &&
    isScreenVisionReference(candidate.fixtureId, 160) &&
    isScreenVisionReference(candidate.monitorId, 64) &&
    isScreenVisionBoundedText(candidate.displayName, 160) &&
    isScreenVisionBoundedInteger(candidate.width, 1, 16_384) &&
    isScreenVisionBoundedInteger(candidate.height, 1, 16_384) &&
    candidate.synthetic === true &&
    candidate.metadataOnly === true &&
    candidate.confirmationRequired === true &&
    isScreenVisionBoundedInteger(candidate.redactionRuleCount, 1, 8)
    ? (candidate as unknown as ScreenVisionPreview)
    : null;
}

export function parseScreenVisionSession(
  value: unknown,
): ScreenVisionSession | null {
  const candidate = toolRecord(value);
  const permissions = parseScreenVisionPermissions(candidate?.permissions);
  const privacy = parseScreenVisionPrivacy(candidate?.privacy);
  return candidate !== null &&
    !hasScreenVisionForbiddenField(candidate) &&
    isScreenVisionReference(candidate.id, 128) &&
    isScreenVisionReference(candidate.agentId, 96) &&
    isScreenVisionReference(candidate.ownerUserId, 96) &&
    isScreenVisionReference(candidate.monitorId, 64) &&
    isScreenVisionReference(candidate.fixtureId, 160) &&
    ["active", "cancelled", "closed"].includes(candidate.status as string) &&
    permissions !== null &&
    privacy !== null &&
    isScreenVisionBoundedInteger(candidate.maxJobs, 1, 8) &&
    isScreenVisionBoundedInteger(candidate.maxDurationMs, 100, 15_000) &&
    isScreenVisionTimestamp(candidate.createdAt) &&
    isScreenVisionTimestamp(candidate.updatedAt) &&
    (candidate.closedAt === null || isScreenVisionTimestamp(candidate.closedAt))
    ? (candidate as unknown as ScreenVisionSession)
    : null;
}

export function parseScreenVisionJob(value: unknown): ScreenVisionJob | null {
  const candidate = toolRecord(value);
  const preview = parseScreenVisionPreview(candidate?.preview);
  const privacy = parseScreenVisionPrivacy(candidate?.privacy);
  return candidate !== null &&
    !hasScreenVisionForbiddenField(candidate) &&
    isScreenVisionReference(candidate.id, 128) &&
    isScreenVisionReference(candidate.sessionId, 128) &&
    isScreenVisionReference(candidate.agentId, 96) &&
    isScreenVisionReference(candidate.ownerUserId, 96) &&
    isScreenVisionReference(candidate.monitorId, 64) &&
    isScreenVisionReference(candidate.fixtureId, 160) &&
    candidate.modelFixtureId === SCREEN_VISION_MODEL_FIXTURE_ID &&
    candidate.resourceKey === SCREEN_VISION_RESOURCE_KEY &&
    ["available", "reserved", "released"].includes(
      candidate.resourceStatus as string,
    ) &&
    [
      "previewed",
      "queued",
      "running",
      "completed",
      "cancelled",
      "failed",
      "cleaned",
    ].includes(candidate.status as string) &&
    (candidate.terminalStatus === null ||
      ["completed", "cancelled", "failed", "expired", "cleaned"].includes(
        candidate.terminalStatus as string,
      )) &&
    [
      "not_loaded",
      "loading",
      "ready",
      "running",
      "unloaded",
      "unavailable",
    ].includes(candidate.modelLifecycle as string) &&
    (candidate.modelLoadedAt === null ||
      isScreenVisionTimestamp(candidate.modelLoadedAt)) &&
    (candidate.modelRunAt === null ||
      isScreenVisionTimestamp(candidate.modelRunAt)) &&
    (candidate.modelCleanupAt === null ||
      isScreenVisionTimestamp(candidate.modelCleanupAt)) &&
    ["pending", "complete"].includes(candidate.cleanupStatus as string) &&
    preview !== null &&
    preview.fixtureId === candidate.fixtureId &&
    preview.monitorId === candidate.monitorId &&
    privacy !== null &&
    typeof candidate.frameMetadataPresent === "boolean" &&
    candidate.resultDurable === false &&
    (candidate.errorCode === null ||
      isScreenVisionBoundedText(candidate.errorCode, 96)) &&
    isScreenVisionTimestamp(candidate.createdAt) &&
    (candidate.queuedAt === null ||
      isScreenVisionTimestamp(candidate.queuedAt)) &&
    (candidate.runningAt === null ||
      isScreenVisionTimestamp(candidate.runningAt)) &&
    (candidate.completedAt === null ||
      isScreenVisionTimestamp(candidate.completedAt)) &&
    (candidate.cleanedAt === null ||
      isScreenVisionTimestamp(candidate.cleanedAt)) &&
    isScreenVisionTimestamp(candidate.updatedAt)
    ? (candidate as unknown as ScreenVisionJob)
    : null;
}

export function parseScreenVisionHypothesis(
  value: unknown,
): ScreenVisionHypothesis | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasScreenVisionForbiddenField(candidate) &&
    isScreenVisionBoundedText(candidate.text, MAX_SCREEN_VISION_RESULT_TEXT) &&
    isScreenVisionBoundedInteger(candidate.confidence, 0, 100) &&
    candidate.uncertain === true &&
    candidate.diagnostic === false &&
    candidate.durable === false &&
    candidate.sensitiveAttributeInferred === false &&
    isScreenVisionBoundedText(candidate.source, 160)
    ? (candidate as unknown as ScreenVisionHypothesis)
    : null;
}

export function parseScreenVisionAnalysisResult(
  value: unknown,
): ScreenVisionAnalysisResult | null {
  const candidate = toolRecord(value);
  const job = parseScreenVisionJob(candidate?.job);
  const hypothesis = parseScreenVisionHypothesis(candidate?.hypothesis);
  return candidate !== null &&
    !hasScreenVisionForbiddenField(candidate) &&
    job !== null &&
    hypothesis !== null &&
    candidate.outputBounded === true &&
    candidate.screenshotBytesPersisted === false
    ? (candidate as unknown as ScreenVisionAnalysisResult)
    : null;
}

export function parseScreenVisionAuditRecord(
  value: unknown,
): ScreenVisionAuditRecord | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasScreenVisionForbiddenField(candidate) &&
    isScreenVisionReference(candidate.id, 128) &&
    (candidate.sessionId === null ||
      isScreenVisionReference(candidate.sessionId, 128)) &&
    (candidate.jobId === null ||
      isScreenVisionReference(candidate.jobId, 128)) &&
    isScreenVisionReference(candidate.agentId, 96) &&
    isScreenVisionBoundedText(candidate.event, 64) &&
    isScreenVisionBoundedText(candidate.result, 64) &&
    (candidate.code === null ||
      isScreenVisionBoundedText(candidate.code, 96)) &&
    isScreenVisionBoundedText(candidate.summary, MAX_SCREEN_VISION_TEXT) &&
    isScreenVisionTimestamp(candidate.createdAt)
    ? (candidate as unknown as ScreenVisionAuditRecord)
    : null;
}

export function parseScreenVisionFixtures(
  value: unknown,
): ScreenVisionFixture[] | null {
  if (!Array.isArray(value) || value.length > 2) return null;
  const fixtures = value.map(parseScreenVisionFixture);
  return fixtures.every(
    (fixture): fixture is ScreenVisionFixture => fixture !== null,
  )
    ? fixtures
    : null;
}

export function parseScreenVisionSessions(
  value: unknown,
): ScreenVisionSession[] | null {
  if (!Array.isArray(value) || value.length > 32) return null;
  const sessions = value.map(parseScreenVisionSession);
  return sessions.every(
    (session): session is ScreenVisionSession => session !== null,
  )
    ? sessions
    : null;
}

export function parseScreenVisionJobs(
  value: unknown,
): ScreenVisionJob[] | null {
  if (!Array.isArray(value) || value.length > 64) return null;
  const jobs = value.map(parseScreenVisionJob);
  return jobs.every((job): job is ScreenVisionJob => job !== null)
    ? jobs
    : null;
}

export function parseScreenVisionAudit(
  value: unknown,
): ScreenVisionAuditRecord[] | null {
  if (!Array.isArray(value) || value.length > 100) return null;
  const records = value.map(parseScreenVisionAuditRecord);
  return records.every(
    (record): record is ScreenVisionAuditRecord => record !== null,
  )
    ? records
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
