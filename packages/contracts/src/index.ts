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
export type VoiceCaptureRuntimeRequest = {
  agentId: string;
  operationId: string;
  idempotencyKey: string;
  durationMs: number;
  temporaryChat: boolean;
};
export type VoiceSynthesisRuntimeRequest = {
  agentId: string;
  operationId: string;
  idempotencyKey: string;
  text: string;
  temporaryChat: boolean;
};
export type VoiceOperationCancellationRequest = {
  agentId: string;
  operationId: string;
};
export type VoiceOperationStatusRequest = {
  agentId: string;
  operationId: string;
};
export type VoiceRuntimeStatus = "started" | "completed" | "cancelled" | "degraded";
export type VoiceOperationStatus = {
  operationId: string;
  agentId: string;
  operation: "transcription" | "synthesis" | "wake_word";
  status: VoiceRuntimeStatus;
  code: string | null;
  providerRef: string | null;
  durationMs: number | null;
  rawAudioPersisted: false;
  listenerActive: false;
  startedAt: number;
  completedAt: number | null;
};
export type VoiceRuntimeTranscriptionResult = {
  operationId: string;
  status: "completed" | "cancelled" | "degraded";
  code: string | null;
  text: string | null;
  confidence: number | null;
  durationMs: number;
  providerRef: string | null;
  source: string;
  metadataOnly: boolean;
  rawAudioPersisted: false;
  textChatFallback: boolean;
};
export type VoiceRuntimeSynthesisResult = {
  operationId: string;
  status: "completed" | "cancelled" | "degraded" | "muted";
  code: string | null;
  voiceRef: string;
  durationMs: number;
  providerRef: string | null;
  source: string;
  metadataOnly: boolean;
  rawAudioPersisted: false;
  textChatFallback: boolean;
};
export type VoiceRuntimeWakeWordResult = {
  operationId: string;
  status: "detected" | "ignored" | "cancelled" | "degraded";
  code: string | null;
  detected: boolean;
  captureDurationMs: number;
  providerRef: string | null;
  source: string;
  listenerActive: false;
  metadataOnly: boolean;
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
  "workspace_mock" | "workspace_local" | "calendar_mock" | "messaging_mock";
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
  "dry_run" | "simulated" | "cancelled" | "compensated" | "executed" | "failed";

export type ToolManifest = {
  toolId: string;
  manifestVersion: 1;
  name: string;
  classification: ToolClassification;
  adapterKind: ToolAdapterKind;
  scopeKind: "workspace" | "workspace_root" | "calendar" | "messaging";
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
export type WorkspaceRoot = {
  id: string;
  enabled: boolean;
  createdAt: number;
  updatedAt: number;
};
export type WorkspaceRootRequest = {
  path: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type WorkspaceRootIdRequest = {
  rootId: string;
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
export type ToolFileMove = { from: string; to: string; sourceIdentity?: string };
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
  changed: boolean;
  untrusted: true;
};
export type ToolCompensation = {
  kind: string;
  available: boolean;
  description: string;
  moves?: ToolCompensationMove[] | null;
};
export type ToolCompensationMove = {
  from: string;
  to: string;
  identity: string;
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
export type ExtensionInstruction =
  | { op: "emit_text"; text: string | null; echoInput: boolean | null }
  | { op: "read_agent_context" }
  | { op: "list_tool_catalog" }
  | { op: "yield" };
export type ExtensionPackage = {
  format: "aip-extension-package/v1";
  entrypoint: "main";
  instructions: ExtensionInstruction[];
  integritySha256: string;
};

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
  package?: ExtensionPackage | null;
};
export type ExtensionExecutionRequest = {
  agentId: string;
  ownerUserId: string;
  extensionId: string;
  revision: number;
  packageHash: string;
  input: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type ExtensionExecutionResult = {
  executionId: string;
  status: "succeeded" | "failed" | "terminated" | "cancelled" | "denied";
  output: string | null;
  error: string | null;
  steps: number;
};
export type ExtensionExecutionCancellationRequest = {
  agentId: string;
  ownerUserId: string;
  executionId: string;
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

export const COMPANION_PROTOCOL_VERSION = 1 as const;
export const COMPANION_MIN_PROTOCOL_VERSION = 1 as const;
export const COMPANION_FIXTURE_DEVICE_ID = "android-fixture-01" as const;
export const COMPANION_FIXTURE_FINGERPRINT =
  "fixture:fingerprint/android-01" as const;
export const COMPANION_FIXTURE_PAIRING_NONCE =
  "fixture:pairing/android-01" as const;
export const COMPANION_FIXTURE_APP_VERSION = "0.1.0-fixture" as const;

export type CompanionPlatform = "android";
export type CompanionDeviceStatus =
  "pairing_requested" | "paired" | "expired" | "revoked";
export type CompanionSessionStatus =
  "connected" | "disconnected" | "revoked" | "expired";
export type CompanionQueueStatus =
  "previewed" | "queued" | "cancelled" | "failed";
export type CompanionMessageKind =
  | "pairing"
  | "session"
  | "queue"
  | "history"
  | "key_rotation"
  | "revocation"
  | "status";
export type CompanionSafetyFlags = {
  metadataOnly: true;
  mediaBytesPersisted: false;
  networkListener: false;
  standaloneFallback: true;
};
export type CompanionQueuePayload =
  | { kind: "text"; text: string }
  | {
      kind: "audio";
      mimeType: string;
      durationMs: number;
      byteLength: number;
    }
  | {
      kind: "image";
      mimeType: string;
      width: number;
      height: number;
      byteLength: number;
    }
  | { kind: "file"; fileName: string; mimeType: string; byteLength: number }
  | { kind: "task"; title: string; summary: string };
export type CompanionProtocolInfo = {
  schemaVersion: 1;
  protocolVersion: typeof COMPANION_PROTOCOL_VERSION;
  minProtocolVersion: typeof COMPANION_MIN_PROTOCOL_VERSION;
  platform: CompanionPlatform;
  appVersion: typeof COMPANION_FIXTURE_APP_VERSION;
  transport: "tauri_command_fixture";
  networkListener: false;
  standaloneFallback: true;
};
export type CompanionProtocolMessage = {
  schemaVersion: 1;
  protocolVersion: typeof COMPANION_PROTOCOL_VERSION;
  messageId: string;
  deviceId: string;
  platform: CompanionPlatform;
  appVersion: typeof COMPANION_FIXTURE_APP_VERSION;
  kind: CompanionMessageKind;
  sessionId: string | null;
  nonceMetadata: string;
  replayCounter: number;
  payloadKind: string;
};
export type CompanionDevice = {
  id: string;
  agentId: string;
  ownerUserId: string;
  deviceId: string;
  platform: CompanionPlatform;
  appVersion: typeof COMPANION_FIXTURE_APP_VERSION;
  protocolVersion: typeof COMPANION_PROTOCOL_VERSION;
  status: CompanionDeviceStatus;
  fingerprint: string;
  pairingNonceMetadata: string;
  keyVersion: number;
  pairingExpiresAt: number | null;
  pairedAt: number | null;
  revokedAt: number | null;
  lastSeenAt: number | null;
  compatible: true;
  standaloneFallback: true;
  createdAt: number;
  updatedAt: number;
};
export type CompanionSession = {
  id: string;
  deviceId: string;
  agentId: string;
  ownerUserId: string;
  status: CompanionSessionStatus;
  protocolVersion: typeof COMPANION_PROTOCOL_VERSION;
  appVersion: typeof COMPANION_FIXTURE_APP_VERSION;
  negotiatedProtocolVersion: typeof COMPANION_PROTOCOL_VERSION;
  keyFingerprint: string;
  sessionNonceMetadata: string;
  lastReplayCounter: number;
  connectedAt: number;
  lastSeenAt: number;
  disconnectedAt: number | null;
  protocol: CompanionProtocolInfo;
  handshake: CompanionProtocolMessage;
  updatedAt: number;
};
export type CompanionQueueItem = {
  id: string;
  deviceId: string;
  sessionId: string;
  agentId: string;
  ownerUserId: string;
  kind: CompanionQueuePayload["kind"];
  status: CompanionQueueStatus;
  payload: CompanionQueuePayload;
  summary: string;
  metadataOnly: true;
  mediaBytesPersisted: false;
  approvalRequired: true;
  retryCount: number;
  errorCode: string | null;
  createdAt: number;
  previewedAt: number;
  approvedAt: number | null;
  cancelledAt: number | null;
  updatedAt: number;
};
export type CompanionHistoryRecord = {
  id: string;
  deviceId: string | null;
  sessionId: string | null;
  agentId: string;
  ownerUserId: string;
  direction: string;
  kind: string;
  summary: string;
  metadataOnly: true;
  mediaBytesPersisted: false;
  createdAt: number;
};
export type CompanionAuditRecord = {
  id: string;
  deviceId: string | null;
  sessionId: string | null;
  queueId: string | null;
  agentId: string;
  ownerUserId: string;
  event: string;
  result: string;
  code: string | null;
  summary: string;
  createdAt: number;
};
export type CompanionKeyRotation = {
  id: string;
  deviceId: string;
  agentId: string;
  ownerUserId: string;
  oldFingerprint: string;
  newFingerprint: string;
  oldKeyVersion: number;
  newKeyVersion: number;
  nonceMetadata: string;
  status: "completed";
  reason: string;
  createdAt: number;
};
export type CompanionRevocation = {
  id: string;
  deviceId: string;
  agentId: string;
  ownerUserId: string;
  previousStatus: CompanionDeviceStatus;
  reason: string;
  createdAt: number;
};
export type CompanionPairingRequest = {
  agentId: string;
  ownerUserId: string;
  deviceId: string;
  platform: CompanionPlatform;
  appVersion: typeof COMPANION_FIXTURE_APP_VERSION;
  protocolVersion: typeof COMPANION_PROTOCOL_VERSION;
  fingerprint: string;
  pairingNonceMetadata: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type CompanionPairingConfirmationRequest = {
  agentId: string;
  ownerUserId: string;
  deviceId: string;
  fingerprint: string;
  pairingNonceMetadata: string;
  confirmed: true;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type CompanionSessionRequest = {
  agentId: string;
  ownerUserId: string;
  deviceId: string;
  appVersion: typeof COMPANION_FIXTURE_APP_VERSION;
  protocolVersion: typeof COMPANION_PROTOCOL_VERSION;
  fingerprint: string;
  pairingNonceMetadata: string;
  messageNonceMetadata: string;
  replayCounter: number;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type CompanionSessionProof = {
  sessionId: string;
  deviceId: string;
  sessionNonceMetadata: string;
  keyFingerprint: string;
  appVersion: typeof COMPANION_FIXTURE_APP_VERSION;
  protocolVersion: typeof COMPANION_PROTOCOL_VERSION;
  messageNonceMetadata: string;
  replayCounter: number;
};
export type CompanionReconnectRequest = {
  agentId: string;
  ownerUserId: string;
  proof: CompanionSessionProof;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type CompanionQueuePreviewRequest = {
  agentId: string;
  ownerUserId: string;
  proof: CompanionSessionProof;
  payload: CompanionQueuePayload;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type CompanionQueueDecisionRequest = {
  agentId: string;
  ownerUserId: string;
  proof: CompanionSessionProof;
  queueId: string;
  approved: true;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type CompanionQueueActionRequest = {
  agentId: string;
  ownerUserId: string;
  proof: CompanionSessionProof;
  queueId: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type CompanionDeviceActionRequest = {
  agentId: string;
  ownerUserId: string;
  deviceId: string;
  reason: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};

export const GATEWAY_PROTOCOL_VERSION = 1 as const;
export const GATEWAY_MIN_PROTOCOL_VERSION = 1 as const;
export const GATEWAY_FIXTURE_AGENT_ID = "agt_luma_provisional" as const;
export const GATEWAY_FIXTURE_ACCOUNT_ID = "gateway-account-owner" as const;
export const GATEWAY_FIXTURE_LOCAL_ACCOUNT_ID = "aip-owner-local" as const;
export const GATEWAY_FIXTURE_EXTERNAL_ACCOUNT_METADATA =
  "fixture:external-account/bielos-owner" as const;
export const GATEWAY_FIXTURE_CLIENT_ID = "mobile-admin-fixture-01" as const;
export const GATEWAY_FIXTURE_APP_VERSION = "0.1.0-gateway-fixture" as const;
export const GATEWAY_FIXTURE_AUTH_PROOF_METADATA =
  "fixture:auth/mobile-admin-01" as const;
export const GATEWAY_FIXTURE_TRANSFER_INTEGRITY_HASH =
  "sha256:fixture/girlfriend-agent-v1" as const;
export const GATEWAY_FIXTURE_RECOVERY_TARGET =
  "fixture:recovery/owner-access" as const;
export const GATEWAY_CLOUDFLARE_TUNNEL_ID_METADATA =
  "fixture:tunnel/aip-gateway" as const;
export const GATEWAY_CLOUDFLARE_HOSTNAME_METADATA = "example.invalid" as const;
export const GATEWAY_CLOUDFLARE_ACCESS_AUDIENCE_METADATA =
  "fixture:access/aip-owner" as const;

export type GatewayAccountStatus = "metadata_only" | "revoked";
export type GatewayTransferStatus = "previewed" | "approved" | "revoked";
export type GatewaySessionStatus =
  "connected" | "disconnected" | "revoked" | "expired";
export type GatewayRecoveryStatus = "pending_approval" | "approved" | "revoked";
export type GatewayMessageKind = "session" | "recovery";
export type GatewayCloudflareMetadata = {
  provider: "cloudflare_tunnel_access";
  mode: "metadata_only";
  tunnelIdMetadata: typeof GATEWAY_CLOUDFLARE_TUNNEL_ID_METADATA;
  hostnameMetadata: typeof GATEWAY_CLOUDFLARE_HOSTNAME_METADATA;
  accessAudienceMetadata: typeof GATEWAY_CLOUDFLARE_ACCESS_AUDIENCE_METADATA;
  credentialState: "absent";
  networkListener: false;
};
export type GatewayProtocolInfo = {
  schemaVersion: 1;
  protocolVersion: typeof GATEWAY_PROTOCOL_VERSION;
  minProtocolVersion: typeof GATEWAY_MIN_PROTOCOL_VERSION;
  transport: "local_loopback_fixture";
  networkListener: false;
  cloudflare: GatewayCloudflareMetadata;
  standaloneFallback: true;
};
export type GatewayProtocolMessage = {
  schemaVersion: 1;
  protocolVersion: typeof GATEWAY_PROTOCOL_VERSION;
  messageId: string;
  clientId: string;
  kind: GatewayMessageKind;
  sessionId: string;
  nonceMetadata: string;
  replayCounter: number;
  payloadKind: string;
};
export type GatewayAccount = {
  id: string;
  ownerUserId: string;
  localAccountId: typeof GATEWAY_FIXTURE_LOCAL_ACCOUNT_ID;
  externalAccountIdMetadata: typeof GATEWAY_FIXTURE_EXTERNAL_ACCOUNT_METADATA;
  ownershipScope: "owner_only";
  status: GatewayAccountStatus;
  metadataOnly: true;
  externalEffectPerformed: false;
  standaloneFallback: true;
  createdAt: number;
  updatedAt: number;
};
export type GatewayTransfer = {
  id: string;
  accountId: typeof GATEWAY_FIXTURE_ACCOUNT_ID;
  sourceAgentId: typeof GATEWAY_FIXTURE_AGENT_ID;
  ownerUserId: string;
  destinationAccountMetadata: typeof GATEWAY_FIXTURE_EXTERNAL_ACCOUNT_METADATA;
  integrityHash: typeof GATEWAY_FIXTURE_TRANSFER_INTEGRITY_HASH;
  status: GatewayTransferStatus;
  authorizationStatus: "pending_owner_approval" | "owner_approved" | "revoked";
  approvalRequired: true;
  metadataOnly: true;
  externalEffectPerformed: false;
  standaloneFallback: true;
  createdAt: number;
  approvedAt: number | null;
  updatedAt: number;
};
export type GatewaySessionProof = {
  sessionId: string;
  transferId: string;
  clientId: typeof GATEWAY_FIXTURE_CLIENT_ID;
  sessionNonceMetadata: string;
  authProofMetadata: typeof GATEWAY_FIXTURE_AUTH_PROOF_METADATA;
  appVersion: typeof GATEWAY_FIXTURE_APP_VERSION;
  protocolVersion: typeof GATEWAY_PROTOCOL_VERSION;
  messageNonceMetadata: string;
  replayCounter: number;
};
export type GatewaySession = {
  id: string;
  accountId: typeof GATEWAY_FIXTURE_ACCOUNT_ID;
  transferId: string;
  sourceAgentId: typeof GATEWAY_FIXTURE_AGENT_ID;
  ownerUserId: string;
  clientId: typeof GATEWAY_FIXTURE_CLIENT_ID;
  status: GatewaySessionStatus;
  protocolVersion: typeof GATEWAY_PROTOCOL_VERSION;
  appVersion: typeof GATEWAY_FIXTURE_APP_VERSION;
  negotiatedProtocolVersion: typeof GATEWAY_PROTOCOL_VERSION;
  sessionNonceMetadata: string;
  authProofMetadata: typeof GATEWAY_FIXTURE_AUTH_PROOF_METADATA;
  lastReplayCounter: number;
  scope: "administrative_recovery";
  authenticated: true;
  localLoopbackOnly: true;
  standaloneFallback: true;
  connectedAt: number;
  lastSeenAt: number;
  disconnectedAt: number | null;
  protocol: GatewayProtocolInfo;
  handshake: GatewayProtocolMessage;
  updatedAt: number;
};
export type GatewayRecovery = {
  id: string;
  accountId: typeof GATEWAY_FIXTURE_ACCOUNT_ID;
  transferId: string;
  sessionId: string;
  sourceAgentId: typeof GATEWAY_FIXTURE_AGENT_ID;
  ownerUserId: string;
  clientId: typeof GATEWAY_FIXTURE_CLIENT_ID;
  kind: "mobile_administrative";
  status: GatewayRecoveryStatus;
  targetMetadata: typeof GATEWAY_FIXTURE_RECOVERY_TARGET;
  approvalRequired: true;
  metadataOnly: true;
  externalEffectPerformed: false;
  createdAt: number;
  approvedAt: number | null;
  updatedAt: number;
};
export type GatewayAuditRecord = {
  id: string;
  accountId: string | null;
  transferId: string | null;
  sessionId: string | null;
  recoveryId: string | null;
  sourceAgentId: typeof GATEWAY_FIXTURE_AGENT_ID;
  ownerUserId: string;
  event: string;
  result: string;
  code: string | null;
  summary: string;
  createdAt: number;
};
export type GatewayRevocation = {
  id: string;
  accountId: typeof GATEWAY_FIXTURE_ACCOUNT_ID;
  transferId: string | null;
  sessionId: string | null;
  ownerUserId: string;
  targetKind: "transfer" | "session";
  targetId: string;
  previousStatus: string;
  reason: string;
  createdAt: number;
};
export type GatewayTransferRequest = {
  agentId: typeof GATEWAY_FIXTURE_AGENT_ID;
  ownerUserId: string;
  destinationAccountMetadata: typeof GATEWAY_FIXTURE_EXTERNAL_ACCOUNT_METADATA;
  integrityHash: typeof GATEWAY_FIXTURE_TRANSFER_INTEGRITY_HASH;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type GatewayTransferApprovalRequest = {
  agentId: typeof GATEWAY_FIXTURE_AGENT_ID;
  ownerUserId: string;
  transferId: string;
  approved: true;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type GatewaySessionRequest = {
  agentId: typeof GATEWAY_FIXTURE_AGENT_ID;
  ownerUserId: string;
  transferId: string;
  clientId: typeof GATEWAY_FIXTURE_CLIENT_ID;
  appVersion: typeof GATEWAY_FIXTURE_APP_VERSION;
  protocolVersion: typeof GATEWAY_PROTOCOL_VERSION;
  authProofMetadata: typeof GATEWAY_FIXTURE_AUTH_PROOF_METADATA;
  messageNonceMetadata: string;
  replayCounter: number;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type GatewayReconnectRequest = {
  agentId: typeof GATEWAY_FIXTURE_AGENT_ID;
  ownerUserId: string;
  proof: GatewaySessionProof;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type GatewayRecoveryRequest = {
  agentId: typeof GATEWAY_FIXTURE_AGENT_ID;
  ownerUserId: string;
  proof: GatewaySessionProof;
  recoveryKind: "mobile_administrative";
  targetMetadata: typeof GATEWAY_FIXTURE_RECOVERY_TARGET;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type GatewayRecoveryApprovalRequest = {
  agentId: typeof GATEWAY_FIXTURE_AGENT_ID;
  ownerUserId: string;
  proof: GatewaySessionProof;
  recoveryId: string;
  approved: true;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type GatewaySessionActionRequest = {
  agentId: typeof GATEWAY_FIXTURE_AGENT_ID;
  ownerUserId: string;
  sessionId: string;
  reason: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};
export type GatewayTransferActionRequest = {
  agentId: typeof GATEWAY_FIXTURE_AGENT_ID;
  ownerUserId: string;
  transferId: string;
  reason: string;
  idempotencyKey: string;
  temporaryChat: boolean;
};

function isGatewayReference(value: unknown, maximum = 256): value is string {
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

function isGatewayTimestamp(value: unknown): value is number {
  return cognitiveNumber(value) && Number.isSafeInteger(value) && value >= 0;
}

function isGatewayInteger(
  value: unknown,
  minimum = 0,
  maximum = Number.MAX_SAFE_INTEGER,
): value is number {
  return (
    cognitiveNumber(value) &&
    Number.isSafeInteger(value) &&
    value >= minimum &&
    value <= maximum
  );
}

function hasGatewayForbiddenField(candidate: Record<string, unknown>): boolean {
  return Object.keys(candidate).some((key) =>
    /raw|bytes|privateKey|secret|token|password|credentialValue|shell|command|relayUrl|listenerAddress|python|bielos/i.test(
      key,
    ),
  );
}

function isGatewayTransferStatus(
  value: unknown,
): value is GatewayTransferStatus {
  return value === "previewed" || value === "approved" || value === "revoked";
}

function isGatewaySessionStatus(value: unknown): value is GatewaySessionStatus {
  return (
    value === "connected" ||
    value === "disconnected" ||
    value === "revoked" ||
    value === "expired"
  );
}

function isGatewayRecoveryStatus(
  value: unknown,
): value is GatewayRecoveryStatus {
  return (
    value === "pending_approval" || value === "approved" || value === "revoked"
  );
}

export function parseGatewayCloudflareMetadata(
  value: unknown,
): GatewayCloudflareMetadata | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasGatewayForbiddenField(candidate) &&
    candidate.provider === "cloudflare_tunnel_access" &&
    candidate.mode === "metadata_only" &&
    candidate.tunnelIdMetadata === GATEWAY_CLOUDFLARE_TUNNEL_ID_METADATA &&
    candidate.hostnameMetadata === GATEWAY_CLOUDFLARE_HOSTNAME_METADATA &&
    candidate.accessAudienceMetadata ===
      GATEWAY_CLOUDFLARE_ACCESS_AUDIENCE_METADATA &&
    candidate.credentialState === "absent" &&
    candidate.networkListener === false
    ? (candidate as unknown as GatewayCloudflareMetadata)
    : null;
}

export function parseGatewayProtocolInfo(
  value: unknown,
): GatewayProtocolInfo | null {
  const candidate = toolRecord(value);
  const cloudflare = parseGatewayCloudflareMetadata(candidate?.cloudflare);
  return candidate !== null &&
    !hasGatewayForbiddenField(candidate) &&
    candidate.schemaVersion === 1 &&
    candidate.protocolVersion === GATEWAY_PROTOCOL_VERSION &&
    candidate.minProtocolVersion === GATEWAY_MIN_PROTOCOL_VERSION &&
    candidate.transport === "local_loopback_fixture" &&
    candidate.networkListener === false &&
    candidate.standaloneFallback === true &&
    cloudflare !== null
    ? (candidate as unknown as GatewayProtocolInfo)
    : null;
}

export function parseGatewayProtocolMessage(
  value: unknown,
): GatewayProtocolMessage | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasGatewayForbiddenField(candidate) &&
    candidate.schemaVersion === 1 &&
    candidate.protocolVersion === GATEWAY_PROTOCOL_VERSION &&
    isGatewayReference(candidate.messageId, 128) &&
    candidate.clientId === GATEWAY_FIXTURE_CLIENT_ID &&
    (candidate.kind === "session" || candidate.kind === "recovery") &&
    isGatewayReference(candidate.sessionId, 128) &&
    isGatewayReference(candidate.nonceMetadata, 256) &&
    isGatewayInteger(candidate.replayCounter, 1) &&
    isGatewayReference(candidate.payloadKind, 64)
    ? (candidate as unknown as GatewayProtocolMessage)
    : null;
}

export function parseGatewayAccount(value: unknown): GatewayAccount | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasGatewayForbiddenField(candidate) &&
    isGatewayReference(candidate.id, 128) &&
    isGatewayReference(candidate.ownerUserId, 96) &&
    candidate.localAccountId === GATEWAY_FIXTURE_LOCAL_ACCOUNT_ID &&
    candidate.externalAccountIdMetadata ===
      GATEWAY_FIXTURE_EXTERNAL_ACCOUNT_METADATA &&
    candidate.ownershipScope === "owner_only" &&
    (candidate.status === "metadata_only" || candidate.status === "revoked") &&
    candidate.metadataOnly === true &&
    candidate.externalEffectPerformed === false &&
    candidate.standaloneFallback === true &&
    isGatewayTimestamp(candidate.createdAt) &&
    isGatewayTimestamp(candidate.updatedAt)
    ? (candidate as unknown as GatewayAccount)
    : null;
}

export function parseGatewayTransfer(value: unknown): GatewayTransfer | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasGatewayForbiddenField(candidate) &&
    isGatewayReference(candidate.id, 128) &&
    candidate.accountId === GATEWAY_FIXTURE_ACCOUNT_ID &&
    candidate.sourceAgentId === GATEWAY_FIXTURE_AGENT_ID &&
    isGatewayReference(candidate.ownerUserId, 96) &&
    candidate.destinationAccountMetadata ===
      GATEWAY_FIXTURE_EXTERNAL_ACCOUNT_METADATA &&
    candidate.integrityHash === GATEWAY_FIXTURE_TRANSFER_INTEGRITY_HASH &&
    isGatewayTransferStatus(candidate.status) &&
    (candidate.authorizationStatus === "pending_owner_approval" ||
      candidate.authorizationStatus === "owner_approved" ||
      candidate.authorizationStatus === "revoked") &&
    candidate.approvalRequired === true &&
    candidate.metadataOnly === true &&
    candidate.externalEffectPerformed === false &&
    candidate.standaloneFallback === true &&
    isGatewayTimestamp(candidate.createdAt) &&
    (candidate.approvedAt === null ||
      isGatewayTimestamp(candidate.approvedAt)) &&
    isGatewayTimestamp(candidate.updatedAt)
    ? (candidate as unknown as GatewayTransfer)
    : null;
}

export function parseGatewaySessionProof(
  value: unknown,
): GatewaySessionProof | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasGatewayForbiddenField(candidate) &&
    isGatewayReference(candidate.sessionId, 128) &&
    isGatewayReference(candidate.transferId, 128) &&
    candidate.clientId === GATEWAY_FIXTURE_CLIENT_ID &&
    isGatewayReference(candidate.sessionNonceMetadata, 256) &&
    candidate.authProofMetadata === GATEWAY_FIXTURE_AUTH_PROOF_METADATA &&
    candidate.appVersion === GATEWAY_FIXTURE_APP_VERSION &&
    candidate.protocolVersion === GATEWAY_PROTOCOL_VERSION &&
    isGatewayReference(candidate.messageNonceMetadata, 256) &&
    isGatewayInteger(candidate.replayCounter, 1)
    ? (candidate as unknown as GatewaySessionProof)
    : null;
}

export function parseGatewaySession(value: unknown): GatewaySession | null {
  const candidate = toolRecord(value);
  const protocol = parseGatewayProtocolInfo(candidate?.protocol);
  const handshake = parseGatewayProtocolMessage(candidate?.handshake);
  return candidate !== null &&
    !hasGatewayForbiddenField(candidate) &&
    isGatewayReference(candidate.id, 128) &&
    candidate.accountId === GATEWAY_FIXTURE_ACCOUNT_ID &&
    isGatewayReference(candidate.transferId, 128) &&
    candidate.sourceAgentId === GATEWAY_FIXTURE_AGENT_ID &&
    isGatewayReference(candidate.ownerUserId, 96) &&
    candidate.clientId === GATEWAY_FIXTURE_CLIENT_ID &&
    isGatewaySessionStatus(candidate.status) &&
    candidate.protocolVersion === GATEWAY_PROTOCOL_VERSION &&
    candidate.appVersion === GATEWAY_FIXTURE_APP_VERSION &&
    candidate.negotiatedProtocolVersion === GATEWAY_PROTOCOL_VERSION &&
    isGatewayReference(candidate.sessionNonceMetadata, 256) &&
    candidate.authProofMetadata === GATEWAY_FIXTURE_AUTH_PROOF_METADATA &&
    isGatewayInteger(candidate.lastReplayCounter, 1) &&
    candidate.scope === "administrative_recovery" &&
    candidate.authenticated === true &&
    candidate.localLoopbackOnly === true &&
    candidate.standaloneFallback === true &&
    isGatewayTimestamp(candidate.connectedAt) &&
    isGatewayTimestamp(candidate.lastSeenAt) &&
    (candidate.disconnectedAt === null ||
      isGatewayTimestamp(candidate.disconnectedAt)) &&
    protocol !== null &&
    handshake !== null &&
    handshake.kind === "session" &&
    handshake.sessionId === candidate.id &&
    isGatewayTimestamp(candidate.updatedAt)
    ? (candidate as unknown as GatewaySession)
    : null;
}

export function parseGatewayRecovery(value: unknown): GatewayRecovery | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasGatewayForbiddenField(candidate) &&
    isGatewayReference(candidate.id, 128) &&
    candidate.accountId === GATEWAY_FIXTURE_ACCOUNT_ID &&
    isGatewayReference(candidate.transferId, 128) &&
    isGatewayReference(candidate.sessionId, 128) &&
    candidate.sourceAgentId === GATEWAY_FIXTURE_AGENT_ID &&
    isGatewayReference(candidate.ownerUserId, 96) &&
    candidate.clientId === GATEWAY_FIXTURE_CLIENT_ID &&
    candidate.kind === "mobile_administrative" &&
    isGatewayRecoveryStatus(candidate.status) &&
    candidate.targetMetadata === GATEWAY_FIXTURE_RECOVERY_TARGET &&
    candidate.approvalRequired === true &&
    candidate.metadataOnly === true &&
    candidate.externalEffectPerformed === false &&
    isGatewayTimestamp(candidate.createdAt) &&
    (candidate.approvedAt === null ||
      isGatewayTimestamp(candidate.approvedAt)) &&
    isGatewayTimestamp(candidate.updatedAt)
    ? (candidate as unknown as GatewayRecovery)
    : null;
}

export function parseGatewayAuditRecord(
  value: unknown,
): GatewayAuditRecord | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasGatewayForbiddenField(candidate) &&
    isGatewayReference(candidate.id, 128) &&
    (candidate.accountId === null ||
      isGatewayReference(candidate.accountId, 128)) &&
    (candidate.transferId === null ||
      isGatewayReference(candidate.transferId, 128)) &&
    (candidate.sessionId === null ||
      isGatewayReference(candidate.sessionId, 128)) &&
    (candidate.recoveryId === null ||
      isGatewayReference(candidate.recoveryId, 128)) &&
    candidate.sourceAgentId === GATEWAY_FIXTURE_AGENT_ID &&
    isGatewayReference(candidate.ownerUserId, 96) &&
    isGatewayReference(candidate.event, 96) &&
    isGatewayReference(candidate.result, 96) &&
    (candidate.code === null || isGatewayReference(candidate.code, 96)) &&
    isGatewayReference(candidate.summary, 512) &&
    isGatewayTimestamp(candidate.createdAt)
    ? (candidate as unknown as GatewayAuditRecord)
    : null;
}

export function parseGatewayRevocation(
  value: unknown,
): GatewayRevocation | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasGatewayForbiddenField(candidate) &&
    isGatewayReference(candidate.id, 128) &&
    candidate.accountId === GATEWAY_FIXTURE_ACCOUNT_ID &&
    (candidate.transferId === null ||
      isGatewayReference(candidate.transferId, 128)) &&
    (candidate.sessionId === null ||
      isGatewayReference(candidate.sessionId, 128)) &&
    isGatewayReference(candidate.ownerUserId, 96) &&
    (candidate.targetKind === "transfer" ||
      candidate.targetKind === "session") &&
    isGatewayReference(candidate.targetId, 128) &&
    isGatewayReference(candidate.previousStatus, 64) &&
    isGatewayReference(candidate.reason, 512) &&
    isGatewayTimestamp(candidate.createdAt)
    ? (candidate as unknown as GatewayRevocation)
    : null;
}

function parseGatewayArray<T>(
  value: unknown,
  parser: (item: unknown) => T | null,
  maximum: number,
): T[] | null {
  if (!Array.isArray(value) || value.length > maximum) return null;
  const parsed = value.map(parser);
  return parsed.every((item): item is T => item !== null) ? parsed : null;
}

export function parseGatewayAccounts(value: unknown): GatewayAccount[] | null {
  return parseGatewayArray(value, parseGatewayAccount, 4);
}

export function parseGatewayTransfers(
  value: unknown,
): GatewayTransfer[] | null {
  return parseGatewayArray(value, parseGatewayTransfer, 16);
}

export function parseGatewaySessions(value: unknown): GatewaySession[] | null {
  return parseGatewayArray(value, parseGatewaySession, 32);
}

export function parseGatewayRecoveries(
  value: unknown,
): GatewayRecovery[] | null {
  return parseGatewayArray(value, parseGatewayRecovery, 64);
}

export function parseGatewayAudit(value: unknown): GatewayAuditRecord[] | null {
  return parseGatewayArray(value, parseGatewayAuditRecord, 100);
}

export function parseGatewayRevocations(
  value: unknown,
): GatewayRevocation[] | null {
  return parseGatewayArray(value, parseGatewayRevocation, 64);
}

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
  | "workspace_root_unavailable"
  | "workspace_root_invalid"
  | "workspace_root_limit"
  | "workspace_path_unavailable"
  | "workspace_path_invalid"
  | "workspace_destination_exists"
  | "workspace_move_failed"
  | "workspace_move_partial"
  | "workspace_source_identity_unavailable"
  | "workspace_source_identity_mismatch"
  | "workspace_compensation_unavailable"
  | "workspace_compensation_failed"
  | "action_compensation_failed"
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
  | "screen_vision_idempotency_invalid"
  | "companion_agent_invalid"
  | "companion_approval_required"
  | "companion_audit_oversized"
  | "companion_authentication_failed"
  | "companion_blocked_safe_mode"
  | "companion_blocked_suspended"
  | "companion_blocked_temporary"
  | "companion_cancelled"
  | "companion_device_already_paired"
  | "companion_device_invalid"
  | "companion_device_limit"
  | "companion_device_not_found"
  | "companion_device_revoked"
  | "companion_fingerprint_invalid"
  | "companion_fixture_invalid"
  | "companion_history_oversized"
  | "companion_idempotency_conflict"
  | "companion_idempotency_invalid"
  | "companion_key_rotation_invalid"
  | "companion_nonce_invalid"
  | "companion_owner_required"
  | "companion_pairing_confirmation_required"
  | "companion_pairing_expired"
  | "companion_pairing_invalid"
  | "companion_pairing_required"
  | "companion_payload_invalid"
  | "companion_payload_oversized"
  | "companion_protocol_incompatible"
  | "companion_queue_invalid"
  | "companion_queue_limit"
  | "companion_queue_not_found"
  | "companion_queue_state_invalid"
  | "companion_replay_rejected"
  | "companion_request_oversized"
  | "companion_retry_limit"
  | "companion_revocation_not_found"
  | "companion_rotation_not_found"
  | "companion_session_invalid"
  | "companion_session_not_found"
  | "companion_session_unavailable"
  | "companion_text_invalid";
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
    "workspace_root_unavailable",
    "workspace_root_invalid",
    "workspace_root_limit",
    "workspace_path_unavailable",
    "workspace_path_invalid",
    "workspace_destination_exists",
    "workspace_move_failed",
    "workspace_move_partial",
    "workspace_source_identity_unavailable",
    "workspace_source_identity_mismatch",
    "workspace_compensation_unavailable",
    "workspace_compensation_failed",
    "action_compensation_failed",
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
    "companion_agent_invalid",
    "companion_approval_required",
    "companion_audit_oversized",
    "companion_authentication_failed",
    "companion_blocked_safe_mode",
    "companion_blocked_suspended",
    "companion_blocked_temporary",
    "companion_cancelled",
    "companion_device_already_paired",
    "companion_device_invalid",
    "companion_device_limit",
    "companion_device_not_found",
    "companion_device_revoked",
    "companion_fingerprint_invalid",
    "companion_fixture_invalid",
    "companion_history_oversized",
    "companion_idempotency_conflict",
    "companion_idempotency_invalid",
    "companion_key_rotation_invalid",
    "companion_nonce_invalid",
    "companion_owner_required",
    "companion_pairing_confirmation_required",
    "companion_pairing_expired",
    "companion_pairing_invalid",
    "companion_pairing_required",
    "companion_payload_invalid",
    "companion_payload_oversized",
    "companion_protocol_incompatible",
    "companion_queue_invalid",
    "companion_queue_limit",
    "companion_queue_not_found",
    "companion_queue_state_invalid",
    "companion_replay_rejected",
    "companion_request_oversized",
    "companion_retry_limit",
    "companion_revocation_not_found",
    "companion_rotation_not_found",
    "companion_session_invalid",
    "companion_session_not_found",
    "companion_session_unavailable",
    "companion_text_invalid",
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

function toolBoundedText(value: unknown, maximum: number): value is string {
  return typeof value === "string" && value.length > 0 && value.length <= maximum &&
    !/[\u0000-\u001f\u007f]/.test(value);
}

function toolBoundedArray(value: unknown, maximum: number, itemMaximum: number): value is string[] {
  return Array.isArray(value) && value.length <= maximum && value.every((item) => toolBoundedText(item, itemMaximum));
}

function toolBoundedId(value: unknown, maximum = 128): value is string {
  return toolBoundedText(value, maximum);
}

function isToolClassification(value: unknown): value is ToolClassification {
  return value === "read_only" || value === "state_changing";
}

function isToolAdapterKind(value: unknown): value is ToolAdapterKind {
  return (
    value === "workspace_mock" ||
    value === "workspace_local" ||
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
  return ["dry_run", "simulated", "cancelled", "compensated", "executed", "failed"].includes(
    value as string,
  );
}

export function parseToolActionInput(value: unknown): ToolActionInput | null {
  const candidate = toolRecord(value);
  if (!candidate || !cognitiveString(candidate.kind)) return null;
  switch (candidate.kind) {
    case "workspaceInspect":
      return toolBoundedArray(candidate.relativePaths, 32, 512)
        ? (candidate as unknown as ToolActionInput)
        : null;
    case "workspaceOrganize": {
      if (!Array.isArray(candidate.moves) || candidate.moves.length === 0 || candidate.moves.length > 32) return null;
      const moves = candidate.moves.every((move) => {
        const item = toolRecord(move);
        return (
          item !== null &&
          toolBoundedText(item.from, 512) &&
          toolBoundedText(item.to, 512) &&
          (item.sourceIdentity === undefined || toolBoundedText(item.sourceIdentity, 128))
        );
      });
      return moves ? (candidate as unknown as ToolActionInput) : null;
    }
    case "calendarList":
      return toolBoundedText(candidate.date, 32)
        ? (candidate as unknown as ToolActionInput)
        : null;
    case "calendarCreate":
      return toolBoundedText(candidate.title, 160) &&
        toolBoundedText(candidate.date, 32) &&
        toolBoundedText(candidate.start, 16) &&
        toolBoundedText(candidate.end, 16)
        ? (candidate as unknown as ToolActionInput)
        : null;
    case "messagingPreview":
    case "messagingSend":
      return toolBoundedText(candidate.recipient, 160) &&
        toolBoundedText(candidate.body, 2048)
        ? (candidate as unknown as ToolActionInput)
        : null;
    default:
      return null;
  }
}

export function parseToolManifest(value: unknown): ToolManifest | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    toolBoundedId(candidate.toolId, 96) &&
    candidate.manifestVersion === 1 &&
    toolBoundedText(candidate.name, 120) &&
    isToolClassification(candidate.classification) &&
    isToolAdapterKind(candidate.adapterKind) &&
    ["workspace", "workspace_root", "calendar", "messaging"].includes(
      candidate.scopeKind as string,
    ) &&
    typeof candidate.requiresSecondConfirmation === "boolean" &&
    toolBoundedArray(candidate.capabilities, 16, 64) &&
    cognitiveNumber(candidate.updatedAt)
    ? (candidate as unknown as ToolManifest)
    : null;
}

export function parseToolSessionPermission(
  value: unknown,
): ToolSessionPermission | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    toolBoundedId(candidate.toolId, 96) &&
    isToolPermission(candidate.permission)
    ? (candidate as unknown as ToolSessionPermission)
    : null;
}

export function parseToolSession(value: unknown): ToolSession | null {
  const candidate = toolRecord(value);
  if (!candidate || !Array.isArray(candidate.permissions) || candidate.permissions.length === 0 || candidate.permissions.length > 12) return null;
  const permissions = candidate.permissions.map(parseToolSessionPermission);
  return toolBoundedId(candidate.id) &&
    toolBoundedId(candidate.agentId, 96) &&
    toolBoundedText(candidate.scopeRef, 128) &&
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
    toolBoundedText(candidate.output, 4096) &&
    typeof candidate.changed === "boolean" &&
    candidate.untrusted === true
    ? (candidate as unknown as ToolExecutionResult)
    : null;
}

function boundedToolPath(value: unknown): value is string {
  return toolBoundedText(value, 512);
}

function boundedOpaqueRootId(value: unknown): value is string {
  return toolBoundedText(value, 64) && /^[A-Za-z0-9._:-]+$/.test(value);
}

function boundedIdempotency(value: unknown): value is string {
  return toolBoundedText(value, 128);
}

export function parseWorkspaceRoot(value: unknown): WorkspaceRoot | null {
  const candidate = toolRecord(value);
  return candidate !== null && boundedOpaqueRootId(candidate.id) &&
    typeof candidate.enabled === "boolean" && cognitiveNumber(candidate.createdAt) &&
    cognitiveNumber(candidate.updatedAt)
    ? (candidate as unknown as WorkspaceRoot) : null;
}

export function parseWorkspaceRoots(value: unknown): WorkspaceRoot[] | null {
  if (!Array.isArray(value) || value.length > 64) return null;
  const roots = value.map(parseWorkspaceRoot);
  return roots.every((root): root is WorkspaceRoot => root !== null) ? roots : null;
}

export function parseWorkspaceRootRequest(value: unknown): WorkspaceRootRequest | null {
  const candidate = toolRecord(value);
  return candidate !== null && boundedToolPath(candidate.path) && boundedIdempotency(candidate.idempotencyKey) &&
    typeof candidate.temporaryChat === "boolean"
    ? (candidate as unknown as WorkspaceRootRequest) : null;
}

export function parseWorkspaceRootIdRequest(value: unknown): WorkspaceRootIdRequest | null {
  const candidate = toolRecord(value);
  return candidate !== null && boundedOpaqueRootId(candidate.rootId) && boundedIdempotency(candidate.idempotencyKey) &&
    typeof candidate.temporaryChat === "boolean"
    ? (candidate as unknown as WorkspaceRootIdRequest) : null;
}

export function parseToolCompensation(value: unknown): ToolCompensation | null {
  const candidate = toolRecord(value);
  if (candidate === null || !toolBoundedText(candidate.kind, 64) ||
    typeof candidate.available !== "boolean" || !toolBoundedText(candidate.description, 1024)) return null;
  if (candidate.moves === undefined || candidate.moves === null) return candidate as unknown as ToolCompensation;
  if (!Array.isArray(candidate.moves) || candidate.moves.length > 32) return null;
  const moves = candidate.moves.every((move) => {
    const item = toolRecord(move);
    return item !== null && toolBoundedText(item.from, 512) && toolBoundedText(item.to, 512) &&
      toolBoundedText(item.identity, 128);
  });
  return moves
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
    toolBoundedId(candidate.id) &&
    toolBoundedId(candidate.sessionId) &&
    toolBoundedId(candidate.agentId, 96) &&
    toolBoundedId(candidate.toolId, 96) &&
    isToolClassification(candidate.classification) &&
    parseToolActionInput(candidate.input) !== null &&
    toolBoundedText(candidate.summary, 512) &&
    toolBoundedArray(candidate.affectedResources, 64, 512) &&
    toolBoundedText(candidate.exactEffect, 1024) &&
    isToolActionStatus(candidate.status) &&
    typeof candidate.dryRun === "boolean" &&
    typeof candidate.requiresOwnerApproval === "boolean" &&
    typeof candidate.requiresSecondConfirmation === "boolean" &&
    typeof candidate.ownerApproved === "boolean" &&
    typeof candidate.secondConfirmed === "boolean" &&
    (candidate.result === null || result !== null) &&
    (candidate.compensation === null || compensation !== null) &&
    (candidate.code === null || toolBoundedText(candidate.code, 96)) &&
    cognitiveNumber(candidate.createdAt) &&
    cognitiveNumber(candidate.updatedAt)
    ? (candidate as unknown as ToolAction)
    : null;
}

export function parseToolAuditRecord(value: unknown): ToolAuditRecord | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    toolBoundedId(candidate.id) &&
    (candidate.actionId === null || toolBoundedId(candidate.actionId)) &&
    (candidate.sessionId === null || toolBoundedId(candidate.sessionId)) &&
    toolBoundedId(candidate.agentId, 96) &&
    (candidate.toolId === null || toolBoundedId(candidate.toolId, 96)) &&
    toolBoundedText(candidate.event, 64) &&
    toolBoundedText(candidate.result, 64) &&
    (candidate.code === null || toolBoundedText(candidate.code, 96)) &&
    toolBoundedText(candidate.summary, 2048) &&
    cognitiveNumber(candidate.createdAt)
    ? (candidate as unknown as ToolAuditRecord)
    : null;
}

export function parseToolCatalog(value: unknown): ToolManifest[] | null {
  if (!Array.isArray(value) || value.length > 16) return null;
  const manifests = value.map(parseToolManifest);
  return manifests.every(
    (manifest): manifest is ToolManifest => manifest !== null,
  )
    ? manifests
    : null;
}

export function parseToolSessions(value: unknown): ToolSession[] | null {
  if (!Array.isArray(value) || value.length > 32) return null;
  const sessions = value.map(parseToolSession);
  return sessions.every((session): session is ToolSession => session !== null)
    ? sessions
    : null;
}

export function parseToolAudit(value: unknown): ToolAuditRecord[] | null {
  if (!Array.isArray(value) || value.length > 100) return null;
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
const MAX_EXTENSION_PACKAGE_INSTRUCTIONS = 32;
const MAX_EXTENSION_PACKAGE_TEXT = 4096;
const MAX_EXTENSION_EXECUTION_INPUT = 4096;
const MAX_EXTENSION_EXECUTION_OUTPUT = 8192;

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

function isExtensionOutput(value: unknown): value is string {
  return typeof value === "string" && value.length <= MAX_EXTENSION_EXECUTION_OUTPUT &&
    !Array.from(value).some((character) => character.charCodeAt(0) < 32 || character.charCodeAt(0) === 127);
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

function extensionKeys(value: Record<string, unknown>, keys: string[]): boolean {
  return Object.keys(value).every((key) => keys.includes(key));
}

function parseExtensionInstruction(value: unknown): ExtensionInstruction | null {
  const candidate = toolRecord(value);
  if (candidate === null || typeof candidate.op !== "string") return null;
  if (candidate.op === "read_agent_context" || candidate.op === "list_tool_catalog" || candidate.op === "yield") {
    return Object.keys(candidate).length === 1 ? (candidate as ExtensionInstruction) : null;
  }
  return candidate.op === "emit_text" && extensionKeys(candidate, ["op", "text", "echoInput"]) &&
    (candidate.text === null || isExtensionBoundedText(candidate.text, MAX_EXTENSION_PACKAGE_TEXT)) &&
    (candidate.echoInput === null || typeof candidate.echoInput === "boolean") &&
    (candidate.text !== null || candidate.echoInput === true)
    ? (candidate as ExtensionInstruction)
    : null;
}

export function parseExtensionPackage(value: unknown): ExtensionPackage | null {
  const candidate = toolRecord(value);
  if (candidate === null || !extensionKeys(candidate, ["format", "entrypoint", "instructions", "integritySha256"])) return null;
  const instructions = Array.isArray(candidate.instructions) && candidate.instructions.length > 0 && candidate.instructions.length <= MAX_EXTENSION_PACKAGE_INSTRUCTIONS
    ? candidate.instructions.map(parseExtensionInstruction) : null;
  if (instructions === null || instructions.some((instruction) => instruction === null)) return null;
  const encoded = instructions.map((instruction) => JSON.stringify(instruction));
  return candidate.format === "aip-extension-package/v1" && candidate.entrypoint === "main" &&
    new Set(encoded).size === encoded.length && typeof candidate.integritySha256 === "string" && /^[0-9a-f]{64}$/.test(candidate.integritySha256)
    ? (candidate as unknown as ExtensionPackage) : null;
}

export function parseExtensionManifest(
  value: unknown,
): ExtensionManifest | null {
  const candidate = toolRecord(value);
  const capabilities = parseExtensionCapabilities(candidate?.capabilities);
  const packageValue = candidate?.package === null ? null : parseExtensionPackage(candidate?.package);
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
    candidate.untrusted === true &&
    (candidate.package === null || candidate.package === undefined || packageValue !== null)
    ? (candidate as unknown as ExtensionManifest)
    : null;
}

export function parseExtensionExecutionRequest(value: unknown): ExtensionExecutionRequest | null {
  const candidate = toolRecord(value);
  return candidate !== null && extensionKeys(candidate, ["agentId", "ownerUserId", "extensionId", "revision", "packageHash", "input", "idempotencyKey", "temporaryChat"]) &&
    isExtensionAgentId(candidate.agentId) && isExtensionAgentId(candidate.ownerUserId) && isExtensionId(candidate.extensionId) &&
    isExtensionRevision(candidate.revision) && typeof candidate.packageHash === "string" && /^[0-9a-f]{64}$/.test(candidate.packageHash) &&
    isExtensionBoundedText(candidate.input, MAX_EXTENSION_EXECUTION_INPUT) && isExtensionBoundedText(candidate.idempotencyKey, 128) && typeof candidate.temporaryChat === "boolean"
    ? (candidate as unknown as ExtensionExecutionRequest) : null;
}

export function parseExtensionExecutionResult(value: unknown): ExtensionExecutionResult | null {
  const candidate = toolRecord(value);
  return candidate !== null && extensionKeys(candidate, ["executionId", "status", "output", "error", "steps"]) &&
    isExtensionRecordId(candidate.executionId) && ["succeeded", "failed", "terminated", "cancelled", "denied"].includes(candidate.status as string) &&
    (candidate.output === null || isExtensionOutput(candidate.output)) &&
    (candidate.error === null || isExtensionBoundedText(candidate.error, 512)) && typeof candidate.steps === "number" && Number.isSafeInteger(candidate.steps) && candidate.steps >= 0 && candidate.steps <= 32
    ? (candidate as unknown as ExtensionExecutionResult) : null;
}

export function parseExtensionExecutionCancellationRequest(value: unknown): ExtensionExecutionCancellationRequest | null {
  const candidate = toolRecord(value);
  return candidate !== null && extensionKeys(candidate, ["agentId", "ownerUserId", "executionId"]) && isExtensionAgentId(candidate.agentId) && isExtensionAgentId(candidate.ownerUserId) && isExtensionRecordId(candidate.executionId)
    ? (candidate as unknown as ExtensionExecutionCancellationRequest) : null;
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

const MAX_COMPANION_TEXT = 16_384;
const MAX_COMPANION_REFERENCE = 192;
const MAX_COMPANION_RETRY_COUNT = 8;
const MAX_COMPANION_MEDIA_METADATA_BYTES = 100_000_000;

function isCompanionBoundedText(
  value: unknown,
  maximum: number,
): value is string {
  return (
    cognitiveString(value) &&
    value.length > 0 &&
    value.length <= maximum &&
    !Array.from(value).some((character) => {
      const code = character.charCodeAt(0);
      return code < 32 && ![9, 10, 13].includes(code);
    })
  );
}

function isCompanionReference(
  value: unknown,
  maximum = MAX_COMPANION_REFERENCE,
): value is string {
  return (
    isCompanionBoundedText(value, maximum) &&
    !value.includes("..") &&
    !value.includes("\\") &&
    /^[A-Za-z0-9:._/-]+$/.test(value)
  );
}

function isCompanionTimestamp(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function isCompanionInteger(
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

function isCompanionProtocolVersion(value: unknown): value is 1 {
  return value === COMPANION_PROTOCOL_VERSION;
}

function hasCompanionForbiddenField(
  candidate: Record<string, unknown>,
): boolean {
  return [
    "rawBytes",
    "bytes",
    "mediaBytes",
    "audioBytes",
    "imageBytes",
    "fileBytes",
    "pixelData",
    "imageData",
    "networkUrl",
    "relay",
    "credentials",
    "privateKey",
    "accessToken",
    "capturePath",
    "filePath",
  ].some((key) => key in candidate);
}

function isCompanionMessageKind(value: unknown): value is CompanionMessageKind {
  return [
    "pairing",
    "session",
    "queue",
    "history",
    "key_rotation",
    "revocation",
    "status",
  ].includes(value as string);
}

function isCompanionDeviceStatus(
  value: unknown,
): value is CompanionDeviceStatus {
  return ["pairing_requested", "paired", "expired", "revoked"].includes(
    value as string,
  );
}

function isCompanionSessionStatus(
  value: unknown,
): value is CompanionSessionStatus {
  return ["connected", "disconnected", "revoked", "expired"].includes(
    value as string,
  );
}

function isCompanionQueueStatus(value: unknown): value is CompanionQueueStatus {
  return ["previewed", "queued", "cancelled", "failed"].includes(
    value as string,
  );
}

function isCompanionMime(value: unknown, prefix: string): value is string {
  return (
    isCompanionReference(value, 96) &&
    value.includes("/") &&
    (prefix === "" || value.startsWith(prefix))
  );
}

function isCompanionFileName(value: unknown): value is string {
  return (
    isCompanionBoundedText(value, 192) &&
    value !== "." &&
    value !== ".." &&
    !value.includes("/") &&
    !value.includes("\\") &&
    !value.includes(":")
  );
}

export function parseCompanionSafetyFlags(
  value: unknown,
): CompanionSafetyFlags | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    candidate.metadataOnly === true &&
    candidate.mediaBytesPersisted === false &&
    candidate.networkListener === false &&
    candidate.standaloneFallback === true
    ? (candidate as unknown as CompanionSafetyFlags)
    : null;
}

export function parseCompanionProtocolInfo(
  value: unknown,
): CompanionProtocolInfo | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasCompanionForbiddenField(candidate) &&
    candidate.schemaVersion === 1 &&
    isCompanionProtocolVersion(candidate.protocolVersion) &&
    candidate.minProtocolVersion === COMPANION_MIN_PROTOCOL_VERSION &&
    candidate.platform === "android" &&
    candidate.appVersion === COMPANION_FIXTURE_APP_VERSION &&
    candidate.transport === "tauri_command_fixture" &&
    candidate.networkListener === false &&
    candidate.standaloneFallback === true
    ? (candidate as unknown as CompanionProtocolInfo)
    : null;
}

export function parseCompanionProtocolMessage(
  value: unknown,
): CompanionProtocolMessage | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasCompanionForbiddenField(candidate) &&
    candidate.schemaVersion === 1 &&
    isCompanionProtocolVersion(candidate.protocolVersion) &&
    isCompanionReference(candidate.messageId) &&
    isCompanionReference(candidate.deviceId, 96) &&
    candidate.platform === "android" &&
    candidate.appVersion === COMPANION_FIXTURE_APP_VERSION &&
    isCompanionMessageKind(candidate.kind) &&
    (candidate.sessionId === null ||
      isCompanionReference(candidate.sessionId, 128)) &&
    isCompanionReference(candidate.nonceMetadata) &&
    isCompanionInteger(candidate.replayCounter, 1, Number.MAX_SAFE_INTEGER) &&
    isCompanionReference(candidate.payloadKind, 32)
    ? (candidate as unknown as CompanionProtocolMessage)
    : null;
}

export function parseCompanionQueuePayload(
  value: unknown,
): CompanionQueuePayload | null {
  const candidate = toolRecord(value);
  if (candidate === null || hasCompanionForbiddenField(candidate)) return null;
  switch (candidate.kind) {
    case "text":
      return isCompanionBoundedText(candidate.text, MAX_COMPANION_TEXT)
        ? (candidate as unknown as CompanionQueuePayload)
        : null;
    case "audio":
      return isCompanionMime(candidate.mimeType, "audio/") &&
        isCompanionInteger(candidate.durationMs, 1, 300_000) &&
        isCompanionInteger(
          candidate.byteLength,
          0,
          MAX_COMPANION_MEDIA_METADATA_BYTES,
        )
        ? (candidate as unknown as CompanionQueuePayload)
        : null;
    case "image":
      return isCompanionMime(candidate.mimeType, "image/") &&
        isCompanionInteger(candidate.width, 1, 8_192) &&
        isCompanionInteger(candidate.height, 1, 8_192) &&
        isCompanionInteger(
          candidate.byteLength,
          0,
          MAX_COMPANION_MEDIA_METADATA_BYTES,
        )
        ? (candidate as unknown as CompanionQueuePayload)
        : null;
    case "file":
      return isCompanionFileName(candidate.fileName) &&
        isCompanionMime(candidate.mimeType, "") &&
        isCompanionInteger(
          candidate.byteLength,
          0,
          MAX_COMPANION_MEDIA_METADATA_BYTES,
        )
        ? (candidate as unknown as CompanionQueuePayload)
        : null;
    case "task":
      return isCompanionBoundedText(candidate.title, 256) &&
        isCompanionBoundedText(candidate.summary, 2_048)
        ? (candidate as unknown as CompanionQueuePayload)
        : null;
    default:
      return null;
  }
}

export function parseCompanionDevice(value: unknown): CompanionDevice | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasCompanionForbiddenField(candidate) &&
    isCompanionReference(candidate.id, 128) &&
    isCompanionReference(candidate.agentId, 96) &&
    isCompanionReference(candidate.ownerUserId, 96) &&
    isCompanionReference(candidate.deviceId, 96) &&
    candidate.platform === "android" &&
    candidate.appVersion === COMPANION_FIXTURE_APP_VERSION &&
    isCompanionProtocolVersion(candidate.protocolVersion) &&
    isCompanionDeviceStatus(candidate.status) &&
    isCompanionReference(candidate.fingerprint) &&
    isCompanionReference(candidate.pairingNonceMetadata) &&
    isCompanionInteger(candidate.keyVersion, 1, 32) &&
    (candidate.pairingExpiresAt === null ||
      isCompanionTimestamp(candidate.pairingExpiresAt)) &&
    (candidate.pairedAt === null || isCompanionTimestamp(candidate.pairedAt)) &&
    (candidate.revokedAt === null ||
      isCompanionTimestamp(candidate.revokedAt)) &&
    (candidate.lastSeenAt === null ||
      isCompanionTimestamp(candidate.lastSeenAt)) &&
    candidate.compatible === true &&
    candidate.standaloneFallback === true &&
    isCompanionTimestamp(candidate.createdAt) &&
    isCompanionTimestamp(candidate.updatedAt)
    ? (candidate as unknown as CompanionDevice)
    : null;
}

export function parseCompanionSessionProof(
  value: unknown,
): CompanionSessionProof | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasCompanionForbiddenField(candidate) &&
    isCompanionReference(candidate.sessionId, 128) &&
    isCompanionReference(candidate.deviceId, 96) &&
    isCompanionReference(candidate.sessionNonceMetadata) &&
    isCompanionReference(candidate.keyFingerprint) &&
    candidate.appVersion === COMPANION_FIXTURE_APP_VERSION &&
    isCompanionProtocolVersion(candidate.protocolVersion) &&
    isCompanionReference(candidate.messageNonceMetadata) &&
    isCompanionInteger(candidate.replayCounter, 1, Number.MAX_SAFE_INTEGER)
    ? (candidate as unknown as CompanionSessionProof)
    : null;
}

export function parseCompanionSession(value: unknown): CompanionSession | null {
  const candidate = toolRecord(value);
  const protocol = parseCompanionProtocolInfo(candidate?.protocol);
  const handshake = parseCompanionProtocolMessage(candidate?.handshake);
  return candidate !== null &&
    !hasCompanionForbiddenField(candidate) &&
    isCompanionReference(candidate.id, 128) &&
    isCompanionReference(candidate.deviceId, 96) &&
    isCompanionReference(candidate.agentId, 96) &&
    isCompanionReference(candidate.ownerUserId, 96) &&
    isCompanionSessionStatus(candidate.status) &&
    isCompanionProtocolVersion(candidate.protocolVersion) &&
    candidate.appVersion === COMPANION_FIXTURE_APP_VERSION &&
    candidate.negotiatedProtocolVersion === COMPANION_PROTOCOL_VERSION &&
    isCompanionReference(candidate.keyFingerprint) &&
    isCompanionReference(candidate.sessionNonceMetadata) &&
    isCompanionInteger(
      candidate.lastReplayCounter,
      1,
      Number.MAX_SAFE_INTEGER,
    ) &&
    isCompanionTimestamp(candidate.connectedAt) &&
    isCompanionTimestamp(candidate.lastSeenAt) &&
    (candidate.disconnectedAt === null ||
      isCompanionTimestamp(candidate.disconnectedAt)) &&
    protocol !== null &&
    protocol.protocolVersion === candidate.protocolVersion &&
    handshake !== null &&
    handshake.kind === "session" &&
    handshake.sessionId === candidate.id &&
    handshake.deviceId === candidate.deviceId &&
    handshake.protocolVersion === candidate.protocolVersion &&
    isCompanionTimestamp(candidate.updatedAt)
    ? (candidate as unknown as CompanionSession)
    : null;
}

export function parseCompanionQueueItem(
  value: unknown,
): CompanionQueueItem | null {
  const candidate = toolRecord(value);
  const payload = parseCompanionQueuePayload(candidate?.payload);
  return candidate !== null &&
    !hasCompanionForbiddenField(candidate) &&
    isCompanionReference(candidate.id, 128) &&
    isCompanionReference(candidate.deviceId, 96) &&
    isCompanionReference(candidate.sessionId, 128) &&
    isCompanionReference(candidate.agentId, 96) &&
    isCompanionReference(candidate.ownerUserId, 96) &&
    payload !== null &&
    candidate.kind === payload.kind &&
    isCompanionQueueStatus(candidate.status) &&
    isCompanionBoundedText(candidate.summary, 512) &&
    candidate.metadataOnly === true &&
    candidate.mediaBytesPersisted === false &&
    candidate.approvalRequired === true &&
    isCompanionInteger(candidate.retryCount, 0, MAX_COMPANION_RETRY_COUNT) &&
    (candidate.errorCode === null ||
      isCompanionReference(candidate.errorCode)) &&
    isCompanionTimestamp(candidate.createdAt) &&
    isCompanionTimestamp(candidate.previewedAt) &&
    (candidate.approvedAt === null ||
      isCompanionTimestamp(candidate.approvedAt)) &&
    (candidate.cancelledAt === null ||
      isCompanionTimestamp(candidate.cancelledAt)) &&
    isCompanionTimestamp(candidate.updatedAt)
    ? (candidate as unknown as CompanionQueueItem)
    : null;
}

export function parseCompanionHistoryRecord(
  value: unknown,
): CompanionHistoryRecord | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasCompanionForbiddenField(candidate) &&
    isCompanionReference(candidate.id, 128) &&
    (candidate.deviceId === null ||
      isCompanionReference(candidate.deviceId, 96)) &&
    (candidate.sessionId === null ||
      isCompanionReference(candidate.sessionId, 128)) &&
    isCompanionReference(candidate.agentId, 96) &&
    isCompanionReference(candidate.ownerUserId, 96) &&
    isCompanionReference(candidate.direction, 32) &&
    isCompanionReference(candidate.kind, 32) &&
    isCompanionBoundedText(candidate.summary, 512) &&
    candidate.metadataOnly === true &&
    candidate.mediaBytesPersisted === false &&
    isCompanionTimestamp(candidate.createdAt)
    ? (candidate as unknown as CompanionHistoryRecord)
    : null;
}

export function parseCompanionAuditRecord(
  value: unknown,
): CompanionAuditRecord | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasCompanionForbiddenField(candidate) &&
    isCompanionReference(candidate.id, 128) &&
    (candidate.deviceId === null ||
      isCompanionReference(candidate.deviceId, 96)) &&
    (candidate.sessionId === null ||
      isCompanionReference(candidate.sessionId, 128)) &&
    (candidate.queueId === null ||
      isCompanionReference(candidate.queueId, 128)) &&
    isCompanionReference(candidate.agentId, 96) &&
    isCompanionReference(candidate.ownerUserId, 96) &&
    isCompanionReference(candidate.event, 64) &&
    isCompanionReference(candidate.result, 64) &&
    (candidate.code === null || isCompanionReference(candidate.code, 96)) &&
    isCompanionBoundedText(candidate.summary, 512) &&
    isCompanionTimestamp(candidate.createdAt)
    ? (candidate as unknown as CompanionAuditRecord)
    : null;
}

export function parseCompanionKeyRotation(
  value: unknown,
): CompanionKeyRotation | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasCompanionForbiddenField(candidate) &&
    isCompanionReference(candidate.id, 128) &&
    isCompanionReference(candidate.deviceId, 96) &&
    isCompanionReference(candidate.agentId, 96) &&
    isCompanionReference(candidate.ownerUserId, 96) &&
    isCompanionReference(candidate.oldFingerprint) &&
    isCompanionReference(candidate.newFingerprint) &&
    isCompanionInteger(candidate.oldKeyVersion, 1, 32) &&
    candidate.newKeyVersion === candidate.oldKeyVersion + 1 &&
    isCompanionReference(candidate.nonceMetadata) &&
    candidate.status === "completed" &&
    isCompanionBoundedText(candidate.reason, 512) &&
    isCompanionTimestamp(candidate.createdAt)
    ? (candidate as unknown as CompanionKeyRotation)
    : null;
}

export function parseCompanionRevocation(
  value: unknown,
): CompanionRevocation | null {
  const candidate = toolRecord(value);
  return candidate !== null &&
    !hasCompanionForbiddenField(candidate) &&
    isCompanionReference(candidate.id, 128) &&
    isCompanionReference(candidate.deviceId, 96) &&
    isCompanionReference(candidate.agentId, 96) &&
    isCompanionReference(candidate.ownerUserId, 96) &&
    isCompanionDeviceStatus(candidate.previousStatus) &&
    isCompanionBoundedText(candidate.reason, 512) &&
    isCompanionTimestamp(candidate.createdAt)
    ? (candidate as unknown as CompanionRevocation)
    : null;
}

function parseCompanionArray<T>(
  value: unknown,
  parser: (item: unknown) => T | null,
  maximum: number,
): T[] | null {
  if (!Array.isArray(value) || value.length > maximum) return null;
  const parsed = value.map(parser);
  return parsed.every((item): item is T => item !== null) ? parsed : null;
}

export function parseCompanionDevices(
  value: unknown,
): CompanionDevice[] | null {
  return parseCompanionArray(value, parseCompanionDevice, 4);
}

export function parseCompanionSessions(
  value: unknown,
): CompanionSession[] | null {
  return parseCompanionArray(value, parseCompanionSession, 32);
}

export function parseCompanionQueue(
  value: unknown,
): CompanionQueueItem[] | null {
  return parseCompanionArray(value, parseCompanionQueueItem, 16);
}

export function parseCompanionHistory(
  value: unknown,
): CompanionHistoryRecord[] | null {
  return parseCompanionArray(value, parseCompanionHistoryRecord, 100);
}

export function parseCompanionAudit(
  value: unknown,
): CompanionAuditRecord[] | null {
  return parseCompanionArray(value, parseCompanionAuditRecord, 100);
}

export function parseCompanionKeyRotations(
  value: unknown,
): CompanionKeyRotation[] | null {
  return parseCompanionArray(value, parseCompanionKeyRotation, 32);
}

export function parseCompanionRevocations(
  value: unknown,
): CompanionRevocation[] | null {
  return parseCompanionArray(value, parseCompanionRevocation, 32);
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
