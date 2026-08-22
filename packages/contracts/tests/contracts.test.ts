import { describe, expect, it } from "vitest";
import {
  PROTOCOL_VERSION,
  canTransitionRuntime,
  parseCognitiveError,
  parseCognitiveEvent,
  parseCognitiveExplanation,
  parseCognitiveTrait,
  parseHealthResponse,
  parseOwnerCorrectionResult,
  parseRollbackResult,
  parseVoiceEmotionHypothesis,
  parseVoiceSettings,
  parseVoiceSynthesisResult,
  parseVoiceTranscriptionResult,
  parseVoiceWakeWordResult,
  parseToolAction,
  parseToolActionInput,
  parseToolAudit,
  parseToolCatalog,
  parseToolSession,
  parseToolSessions,
  parseToolExecutionResult,
  parseToolCompensation,
  parseWorkspaceRoot,
  parseWorkspaceRoots,
  parseWorkspaceRootIdRequest,
  parseWorkspaceRootRequest,
  parseExtensionAudit,
  parseExtensionCatalog,
  parseExtensionManifest,
  parseExtensionPackage,
  parseExtensionExecutionRequest,
  parseExtensionExecutionResult,
  parseExtensionExecutionCancellationRequest,
  parseExtensionProposals,
  parseScreenVisionAnalysisResult,
  parseScreenVisionAudit,
  parseScreenVisionFixture,
  parseScreenVisionFixtures,
  parseScreenVisionHypothesis,
  parseScreenVisionJob,
  parseScreenVisionPrivacy,
  parseScreenVisionSession,
  parseScreenVisionSessions,
  COMPANION_FIXTURE_APP_VERSION,
  COMPANION_FIXTURE_FINGERPRINT,
  COMPANION_FIXTURE_PAIRING_NONCE,
  COMPANION_PROTOCOL_VERSION,
  parseCompanionAudit,
  parseCompanionDevice,
  parseCompanionDevices,
  parseCompanionHistory,
  parseCompanionHistoryRecord,
  parseCompanionKeyRotation,
  parseCompanionProtocolInfo,
  parseCompanionQueue,
  parseCompanionQueueItem,
  parseCompanionQueuePayload,
  parseCompanionRevocation,
  parseCompanionSafetyFlags,
  parseCompanionSession,
  parseCompanionSessionProof,
  parseCompanionSessions,
  GATEWAY_CLOUDFLARE_ACCESS_AUDIENCE_METADATA,
  GATEWAY_CLOUDFLARE_HOSTNAME_METADATA,
  GATEWAY_CLOUDFLARE_TUNNEL_ID_METADATA,
  GATEWAY_FIXTURE_ACCOUNT_ID,
  GATEWAY_FIXTURE_AGENT_ID,
  GATEWAY_FIXTURE_APP_VERSION,
  GATEWAY_FIXTURE_AUTH_PROOF_METADATA,
  GATEWAY_FIXTURE_CLIENT_ID,
  GATEWAY_FIXTURE_EXTERNAL_ACCOUNT_METADATA,
  GATEWAY_FIXTURE_LOCAL_ACCOUNT_ID,
  GATEWAY_FIXTURE_RECOVERY_TARGET,
  GATEWAY_FIXTURE_TRANSFER_INTEGRITY_HASH,
  GATEWAY_MIN_PROTOCOL_VERSION,
  GATEWAY_PROTOCOL_VERSION,
  parseGatewayAccount,
  parseGatewayAccounts,
  parseGatewayAudit,
  parseGatewayProtocolInfo,
  parseGatewayRecovery,
  parseGatewayRecoveries,
  parseGatewayRevocations,
  parseGatewaySession,
  parseGatewaySessionProof,
  parseGatewaySessions,
  parseGatewayTransfer,
  parseGatewayTransfers,
} from "../src/index";

describe("runtime contracts", () => {
  it("allows deterministic safe-mode transitions", () => {
    expect(canTransitionRuntime("ready", "safe_mode")).toBe(true);
    expect(canTransitionRuntime("safe_mode", "ready")).toBe(false);
    expect(canTransitionRuntime("safe_mode", "starting")).toBe(true);
  });

  it("accepts only the versioned health response", () => {
    expect(
      parseHealthResponse({
        protocolVersion: PROTOCOL_VERSION,
        id: "health-1",
        result: {
          name: "aip-runtime",
          status: "ready",
          protocolVersion: PROTOCOL_VERSION,
        },
      }),
    ).not.toBeNull();

    expect(
      parseHealthResponse({
        protocolVersion: 99,
        id: "health-1",
        result: { name: "aip-runtime", status: "ready", protocolVersion: 99 },
      }),
    ).toBeNull();
  });
});

describe("local-only gateway contracts", () => {
  const protocol = {
    schemaVersion: 1 as const,
    protocolVersion: GATEWAY_PROTOCOL_VERSION,
    minProtocolVersion: GATEWAY_MIN_PROTOCOL_VERSION,
    transport: "local_loopback_fixture" as const,
    networkListener: false as const,
    cloudflare: {
      provider: "cloudflare_tunnel_access" as const,
      mode: "metadata_only" as const,
      tunnelIdMetadata: GATEWAY_CLOUDFLARE_TUNNEL_ID_METADATA,
      hostnameMetadata: GATEWAY_CLOUDFLARE_HOSTNAME_METADATA,
      accessAudienceMetadata: GATEWAY_CLOUDFLARE_ACCESS_AUDIENCE_METADATA,
      credentialState: "absent" as const,
      networkListener: false as const,
    },
    standaloneFallback: true as const,
  };
  const handshake = {
    schemaVersion: 1 as const,
    protocolVersion: GATEWAY_PROTOCOL_VERSION,
    messageId: "gateway-message-1",
    clientId: GATEWAY_FIXTURE_CLIENT_ID,
    kind: "session" as const,
    sessionId: "session-1",
    nonceMetadata: "fixture:gateway-message/1",
    replayCounter: 1,
    payloadKind: "gateway_session",
  };
  const account = {
    id: GATEWAY_FIXTURE_ACCOUNT_ID,
    ownerUserId: "owner_user",
    localAccountId: GATEWAY_FIXTURE_LOCAL_ACCOUNT_ID,
    externalAccountIdMetadata: GATEWAY_FIXTURE_EXTERNAL_ACCOUNT_METADATA,
    ownershipScope: "owner_only" as const,
    status: "metadata_only" as const,
    metadataOnly: true as const,
    externalEffectPerformed: false as const,
    standaloneFallback: true as const,
    createdAt: 1,
    updatedAt: 1,
  };
  const transfer = {
    id: "transfer-1",
    accountId: GATEWAY_FIXTURE_ACCOUNT_ID,
    sourceAgentId: GATEWAY_FIXTURE_AGENT_ID,
    ownerUserId: "owner_user",
    destinationAccountMetadata: GATEWAY_FIXTURE_EXTERNAL_ACCOUNT_METADATA,
    integrityHash: GATEWAY_FIXTURE_TRANSFER_INTEGRITY_HASH,
    status: "approved" as const,
    authorizationStatus: "owner_approved" as const,
    approvalRequired: true as const,
    metadataOnly: true as const,
    externalEffectPerformed: false as const,
    standaloneFallback: true as const,
    createdAt: 1,
    approvedAt: 1,
    updatedAt: 1,
  };
  const session = {
    id: "session-1",
    accountId: GATEWAY_FIXTURE_ACCOUNT_ID,
    transferId: transfer.id,
    sourceAgentId: GATEWAY_FIXTURE_AGENT_ID,
    ownerUserId: "owner_user",
    clientId: GATEWAY_FIXTURE_CLIENT_ID,
    status: "connected" as const,
    protocolVersion: GATEWAY_PROTOCOL_VERSION,
    appVersion: GATEWAY_FIXTURE_APP_VERSION,
    negotiatedProtocolVersion: GATEWAY_PROTOCOL_VERSION,
    sessionNonceMetadata: "fixture:gateway-session/1",
    authProofMetadata: GATEWAY_FIXTURE_AUTH_PROOF_METADATA,
    lastReplayCounter: 1,
    scope: "administrative_recovery" as const,
    authenticated: true as const,
    localLoopbackOnly: true as const,
    standaloneFallback: true as const,
    connectedAt: 1,
    lastSeenAt: 1,
    disconnectedAt: null,
    protocol,
    handshake,
    updatedAt: 1,
  };

  it("accepts bounded local gateway responses", () => {
    expect(parseGatewayProtocolInfo(protocol)).not.toBeNull();
    expect(parseGatewayAccount(account)).not.toBeNull();
    expect(parseGatewayAccounts([account])).not.toBeNull();
    expect(parseGatewayTransfer(transfer)).not.toBeNull();
    expect(parseGatewayTransfers([transfer])).not.toBeNull();
    expect(
      parseGatewaySessionProof({
        sessionId: session.id,
        transferId: session.transferId,
        clientId: session.clientId,
        sessionNonceMetadata: session.sessionNonceMetadata,
        authProofMetadata: session.authProofMetadata,
        appVersion: session.appVersion,
        protocolVersion: session.protocolVersion,
        messageNonceMetadata: "fixture:gateway-message/2",
        replayCounter: 2,
      }),
    ).not.toBeNull();
    expect(parseGatewaySession(session)).not.toBeNull();
    expect(parseGatewaySessions([session])).not.toBeNull();
    const recovery = {
      id: "recovery-1",
      accountId: GATEWAY_FIXTURE_ACCOUNT_ID,
      transferId: transfer.id,
      sessionId: session.id,
      sourceAgentId: GATEWAY_FIXTURE_AGENT_ID,
      ownerUserId: "owner_user",
      clientId: GATEWAY_FIXTURE_CLIENT_ID,
      kind: "mobile_administrative" as const,
      status: "pending_approval" as const,
      targetMetadata: GATEWAY_FIXTURE_RECOVERY_TARGET,
      approvalRequired: true as const,
      metadataOnly: true as const,
      externalEffectPerformed: false as const,
      createdAt: 1,
      approvedAt: null,
      updatedAt: 1,
    };
    expect(parseGatewayRecovery(recovery)).not.toBeNull();
    expect(parseGatewayRecoveries([recovery])).not.toBeNull();
    expect(
      parseGatewayAudit([
        {
          id: "audit-1",
          accountId: account.id,
          transferId: transfer.id,
          sessionId: session.id,
          recoveryId: recovery.id,
          sourceAgentId: GATEWAY_FIXTURE_AGENT_ID,
          ownerUserId: "owner_user",
          event: "session_connected",
          result: "authenticated",
          code: null,
          summary: "Sessão local autenticada",
          createdAt: 1,
        },
      ]),
    ).not.toBeNull();
    expect(
      parseGatewayRevocations([
        {
          id: "revoke-1",
          accountId: GATEWAY_FIXTURE_ACCOUNT_ID,
          transferId: null,
          sessionId: session.id,
          ownerUserId: "owner_user",
          targetKind: "session",
          targetId: session.id,
          previousStatus: "connected",
          reason: "revogação fixture",
          createdAt: 1,
        },
      ]),
    ).not.toBeNull();
  });

  it("rejects incompatible or unsafe gateway responses", () => {
    expect(
      parseGatewayProtocolInfo({ ...protocol, networkListener: true }),
    ).toBeNull();
    expect(parseGatewaySession({ ...session, privateKey: "no" })).toBeNull();
    expect(
      parseGatewayTransfer({ ...transfer, integrityHash: "sha256:wrong" }),
    ).toBeNull();
    expect(parseGatewaySessions(new Array(33).fill(session))).toBeNull();
  });
});

describe("cognitive contracts", () => {
  const event = {
    id: "event",
    agentId: "astra",
    kind: "trait_delta",
    traitKey: "curiosity",
    sourceKind: "controlled_internal",
    sourceReference: "processor:evidence",
    reason: "test",
    confidence: 1,
    requestedValue: 0.05,
    appliedDelta: 0.05,
    priorValue: 0.5,
    resultingValue: 0.55,
    status: "applied",
    code: null,
    rollbackOfEventId: null,
    createdAt: 1,
  };

  it("accepts typed trait, event, explanation, correction and rollback responses", () => {
    expect(
      parseCognitiveTrait({ key: "curiosity", value: 0.5, isProtected: false }),
    ).not.toBeNull();
    expect(parseCognitiveTrait({ key: "curiosity", value: "0.5" })).toBeNull();
    expect(parseCognitiveEvent(event)).not.toBeNull();
    expect(
      parseCognitiveExplanation({ event, traitLabel: "Curiosidade" }),
    ).not.toBeNull();
    expect(
      parseOwnerCorrectionResult({ ...event, kind: "owner_correction" }),
    ).not.toBeNull();
    expect(parseRollbackResult({ ...event, kind: "rollback" })).not.toBeNull();
  });

  it("rejects malformed cognitive payloads without rendering raw fields", () => {
    expect(parseCognitiveEvent({ id: "event" })).toBeNull();
    expect(
      parseCognitiveEvent({ ...event, confidence: Number.NaN }),
    ).toBeNull();
    expect(parseCognitiveEvent({ ...event, kind: "unknown" })).toBeNull();
    expect(parseCognitiveEvent({ ...event, status: "pending" })).toBeNull();
    expect(parseCognitiveEvent({ ...event, sourceKind: 1 })).toBeNull();
    expect(
      parseCognitiveEvent({ ...event, sourceReference: undefined }),
    ).toBeNull();
    expect(
      parseCognitiveExplanation({ event: { id: "event" }, traitLabel: "x" }),
    ).toBeNull();
    expect(
      parseCognitiveEvent({ ...event, rawPayload: "do not render" }),
    ).not.toBeNull();
  });

  it("accepts every stable cognitive error code", () => {
    for (const code of [
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
      "persistence_failed",
    ]) {
      expect(
        parseCognitiveError({ code, message: "Mensagem segura" }),
      ).not.toBeNull();
    }
    expect(parseCognitiveError({ code: "unknown", message: "x" })).toBeNull();
  });
});

describe("voice contracts", () => {
  const settings = {
    agentId: "agt_astra_provisional",
    schemaVersion: 1 as const,
    baseVoiceId: "aip-base-v1",
    baseVoiceProtected: true as const,
    customVoiceRef: null,
    customVoiceConsent: "not_granted" as const,
    recognitionModelRef: null,
    synthesisModelRef: null,
    inputDeviceRef: null,
    outputDeviceRef: null,
    mode: "normal" as const,
    voiceMuted: false,
    silent: false,
    suspended: false,
    updatedAt: 1,
  };

  it("accepts metadata-only settings and degraded results", () => {
    expect(parseVoiceSettings(settings)).not.toBeNull();
    expect(
      parseVoiceTranscriptionResult({
        status: "degraded",
        code: "voice_device_unavailable",
        fixtureId: "fixture:hello",
        text: null,
        confidence: null,
        metadataOnly: true,
        rawAudioPersisted: false,
        textChatFallback: true,
      }),
    ).not.toBeNull();
    expect(
      parseVoiceSynthesisResult({
        status: "muted",
        code: "voice_muted",
        voiceRef: "aip-base-v1",
        durationMs: 0,
        metadataOnly: true,
        rawAudioPersisted: false,
        textChatFallback: true,
      }),
    ).not.toBeNull();
  });

  it("rejects raw-audio or diagnostic voice payloads", () => {
    expect(
      parseVoiceTranscriptionResult({
        status: "ready",
        code: null,
        fixtureId: "fixture:hello",
        text: "Olá",
        confidence: 0.9,
        metadataOnly: true,
        rawAudioPersisted: true,
        textChatFallback: false,
      }),
    ).toBeNull();
    expect(
      parseVoiceWakeWordResult({
        status: "detected",
        code: null,
        fixtureId: "fixture:wake-aip",
        detected: true,
        listenerActive: true,
        metadataOnly: true,
      }),
    ).toBeNull();
    expect(
      parseVoiceEmotionHypothesis({
        label: "positive",
        confidence: 0.9,
        uncertain: false,
        diagnostic: true,
        source: "model",
      }),
    ).toBeNull();
  });
});

describe("supervised tool contracts", () => {
  const manifest = {
    toolId: "calendar.create_event",
    manifestVersion: 1 as const,
    name: "Calendar fixture event",
    classification: "state_changing" as const,
    adapterKind: "calendar_mock" as const,
    scopeKind: "calendar" as const,
    requiresSecondConfirmation: true,
    capabilities: ["preview", "create", "compensate"],
    updatedAt: 1,
  };
  const session = {
    id: "session-1",
    agentId: "astra",
    scopeRef: "fixture:calendar/owner",
    status: "active" as const,
    permissions: [
      { toolId: manifest.toolId, permission: "preview" as const },
      {
        toolId: manifest.toolId,
        permission: "execute_state_changing" as const,
      },
    ],
    createdAt: 1,
    updatedAt: 1,
  };
  const action = {
    id: "action-1",
    sessionId: session.id,
    agentId: session.agentId,
    toolId: manifest.toolId,
    classification: manifest.classification,
    input: {
      kind: "calendarCreate" as const,
      title: "Revisão",
      date: "2026-08-20",
      start: "10:00",
      end: "11:00",
    },
    summary: "Criar evento fixture",
    affectedResources: ["fixture:calendar/owner/2026-08-20"],
    exactEffect: "Somente mock local.",
    status: "approved" as const,
    dryRun: false,
    requiresOwnerApproval: true,
    requiresSecondConfirmation: true,
    ownerApproved: true,
    secondConfirmed: false,
    result: null,
    compensation: null,
    code: null,
    createdAt: 1,
    updatedAt: 1,
  };

  it("accepts manifest, session, preview, action and audit payloads", () => {
    expect(parseToolCatalog([manifest])).not.toBeNull();
    expect(parseToolSession(session)).not.toBeNull();
    expect(parseToolSessions([session])).not.toBeNull();
    expect(parseToolActionInput(action.input)).not.toBeNull();
    expect(parseToolAction(action)).not.toBeNull();
    expect(
      parseToolAudit([
        {
          id: "audit-1",
          actionId: action.id,
          sessionId: session.id,
          agentId: session.agentId,
          toolId: manifest.toolId,
          event: "action_previewed",
          result: "previewed",
          code: null,
          summary: "Prévia registrada.",
          createdAt: 1,
        },
      ]),
    ).not.toBeNull();
    expect(parseToolExecutionResult({ status: "executed", output: "moved", changed: true, untrusted: true })).not.toBeNull();
    expect(parseToolCompensation({ kind: "workspace_move", available: true, description: "bounded", moves: [{ from: "a.txt", to: "b.txt", identity: "win:1:2" }] })).not.toBeNull();
    expect(parseWorkspaceRoot({ id: "wrt_opaque", enabled: true, createdAt: 1, updatedAt: 2 })).not.toBeNull();
    expect(parseWorkspaceRootRequest({ path: "C:/workspace", idempotencyKey: "root-1", temporaryChat: false })).not.toBeNull();
    expect(parseWorkspaceRootIdRequest({ rootId: "wrt_opaque", idempotencyKey: "root-2", temporaryChat: false })).not.toBeNull();
  });

  it("rejects unsafe or malformed tool payloads", () => {
    expect(
      parseToolAction({
        ...action,
        result: {
          status: "simulated",
          output: "mock",
          changed: true,
          untrusted: false,
        },
      }),
    ).toBeNull();
    expect(parseToolExecutionResult({ status: "executed", output: "moved", changed: "yes", untrusted: true })).toBeNull();
    expect(parseToolExecutionResult({ status: "unknown", output: "moved", changed: true, untrusted: true })).toBeNull();
    expect(parseToolCompensation({ kind: "workspace_move", available: true, description: "bounded", moves: [{ from: "a.txt\u0000", to: "b.txt", identity: "win:1:2" }] })).toBeNull();
    expect(parseToolActionInput({ kind: "workspaceOrganize", moves: [{ from: "a.txt", to: "b.txt", sourceIdentity: "unix:1:2" }] })).not.toBeNull();
    expect(parseToolActionInput({ kind: "workspaceOrganize", moves: [{ from: "a.txt", to: "b.txt", sourceIdentity: "x".repeat(129) }] })).toBeNull();
    expect(parseToolActionInput({ kind: "workspaceOrganize", moves: [{ from: "a.txt", to: "b.txt", sourceIdentity: "unix:\u00001:2" }] })).toBeNull();
    expect(parseToolAction({ ...action, summary: "x".repeat(513) })).toBeNull();
    expect(parseToolAction({ ...action, affectedResources: Array.from({ length: 65 }, (_, index) => `r${index}`) })).toBeNull();
    expect(parseToolActionInput({ kind: "workspaceOrganize", moves: Array.from({ length: 33 }, () => ({ from: "a.txt", to: "b.txt" })) })).toBeNull();
    expect(parseToolSession({ ...session, permissions: Array.from({ length: 13 }, (_, index) => ({ toolId: `tool-${index}`, permission: "preview" as const })) })).toBeNull();
    const audit = {
      id: "audit-1",
      actionId: action.id,
      sessionId: session.id,
      agentId: session.agentId,
      toolId: manifest.toolId,
      event: "action_previewed",
      result: "previewed",
      code: null,
      summary: "Prévia registrada.",
      createdAt: 1,
    };
    expect(parseToolAudit([audit])).not.toBeNull();
    expect(parseToolAudit([{ ...audit, actionId: "a".repeat(129) }])).toBeNull();
    expect(parseToolAudit([{ ...audit, sessionId: "session\u0000id" }])).toBeNull();
    expect(parseWorkspaceRoots(Array.from({ length: 65 }, (_, index) => ({ id: `wrt-${index}`, enabled: true, createdAt: 1, updatedAt: 1 })))).toBeNull();
    expect(parseToolCatalog(Array.from({ length: 17 }, () => manifest))).toBeNull();
    expect(parseToolSessions(Array.from({ length: 33 }, () => session))).toBeNull();
    expect(parseToolAudit(Array.from({ length: 101 }, () => audit))).toBeNull();
    expect(
      parseToolActionInput({ kind: "shell", command: "whoami" }),
    ).toBeNull();
    expect(
      parseToolSessions([session, { ...session, status: "unknown" }]),
    ).toBeNull();
    expect(
      parseCognitiveError({
        code: "tools_blocked_safe_mode",
        message: "Ferramentas bloqueadas no modo seguro",
      }),
    ).not.toBeNull();
  });
});

describe("metadata-only extension contracts", () => {
  const manifest = {
    extensionId: "fixture.notes",
    manifestVersion: 1 as const,
    extensionVersion: "1.0.0",
    sdkVersion: "aip-extension-sdk/v1",
    name: "Notas locais fixture",
    sandboxPolicy: "metadata_only" as const,
    admissionPolicy: "local_fixture_only" as const,
    capabilities: ["tool_catalog" as const, "owner_review" as const],
    localFixtureRef: "fixture:extension/notes",
    untrusted: true as const,
    package: null,
  };

  const proposal = {
    id: "proposal-1",
    extensionId: manifest.extensionId,
    revision: 1,
    sourceKind: "agent_created" as const,
    proposerAgentId: "astra",
    status: "pending" as const,
    reviewStatus: "pending" as const,
    manifest,
    requestedCapabilities: manifest.capabilities,
    approvedCapabilities: [],
    permissions: manifest.capabilities.map((capability) => ({
      capability,
      status: "pending" as const,
    })),
    compatible: true,
    reviewReason: null,
    createdAt: 1,
    updatedAt: 1,
  };

  it("accepts versioned untrusted proposals and catalog records", () => {
    expect(parseExtensionManifest(manifest)).not.toBeNull();
    expect(parseExtensionProposals([proposal])).not.toBeNull();
    expect(
      parseExtensionCatalog([
        {
          extensionId: manifest.extensionId,
          catalogScope: "private_local",
          sourceKind: "agent_created",
          lifecycle: "review_required",
          reviewStatus: "pending",
          manifest,
          currentRevision: 1,
          activeRevision: null,
          approvedCapabilities: [],
          compatible: true,
          untrusted: true,
          updatedAt: 1,
        },
      ]),
    ).not.toBeNull();
    expect(
      parseExtensionAudit([
        {
          id: "audit-1",
          extensionId: manifest.extensionId,
          proposalId: proposal.id,
          revision: 1,
          agentId: "astra",
          event: "proposal_created",
          result: "pending_review",
          code: null,
          summary: "Proposta criada.",
          createdAt: 1,
        },
      ]),
    ).not.toBeNull();
  });

  it("rejects code-like, trusted, incompatible and unknown payloads", () => {
    expect(
      parseExtensionManifest({
        ...manifest,
        untrusted: false,
      }),
    ).toBeNull();
    expect(
      parseExtensionManifest({
        ...manifest,
        sdkVersion: "future-sdk",
      }),
    ).toBeNull();
    expect(
      parseExtensionManifest({
        ...manifest,
        extensionId: "Not.Valid",
      }),
    ).toBeNull();
    expect(
      parseExtensionManifest({
        ...manifest,
        extensionVersion: "1.0.01",
      }),
    ).toBeNull();
    expect(
      parseExtensionManifest({
        ...manifest,
        capabilities: ["tool_catalog", "tool_catalog"],
      }),
    ).toBeNull();
    expect(
      parseExtensionManifest({
        ...manifest,
        localFixtureRef: "fixture:extension/../private",
      }),
    ).toBeNull();
    expect(
      parseExtensionManifest({
        ...manifest,
        code: "return fetch('https://example.invalid')",
      }),
    ).toBeNull();
    expect(
      parseExtensionManifest({
        ...manifest,
        sandboxPolicy: "host_process",
      }),
    ).toBeNull();
    expect(
      parseExtensionProposals([{ ...proposal, status: "active" }]),
    ).toBeNull();
  });
});

describe("bounded extension execution contracts", () => {
  const packageValue = {
    format: "aip-extension-package/v1" as const,
    entrypoint: "main" as const,
    instructions: [
      { op: "emit_text" as const, text: "ok", echoInput: null },
      { op: "read_agent_context" as const },
      { op: "list_tool_catalog" as const },
      { op: "yield" as const },
    ],
    integritySha256: "0".repeat(64),
  };

  it("accepts bounded executable package and result payloads", () => {
    expect(parseExtensionPackage(packageValue)).not.toBeNull();
    expect(parseExtensionExecutionRequest({
      agentId: "agt_astra_provisional",
      ownerUserId: "usr_owner_local",
      extensionId: "fixture.notes",
      revision: 1,
      packageHash: packageValue.integritySha256,
      input: "bounded",
      idempotencyKey: "execute-1",
      temporaryChat: false,
    })).not.toBeNull();
    expect(parseExtensionExecutionResult({
      executionId: "execution-1",
      status: "succeeded",
      output: "agent_id:agt_astra_provisional",
      error: null,
      steps: 4,
    })).not.toBeNull();
    expect(parseExtensionExecutionCancellationRequest({
      agentId: "agt_astra_provisional",
      ownerUserId: "usr_owner_local",
      executionId: "execution-1",
    })).not.toBeNull();
  });

  it("rejects malformed, unknown, duplicate, and oversized execution payloads", () => {
    expect(parseExtensionPackage({ ...packageValue, integritySha256: "z".repeat(64) })).toBeNull();
    expect(parseExtensionPackage({ ...packageValue, instructions: [{ op: "unknown" }] })).toBeNull();
    expect(parseExtensionPackage({ ...packageValue, instructions: [{ op: "yield" }, { op: "yield" }] })).toBeNull();
    expect(parseExtensionPackage({ ...packageValue, instructions: Array.from({ length: 33 }, () => ({ op: "yield" as const })) })).toBeNull();
    expect(parseExtensionExecutionResult({ executionId: "execution-1", status: "unknown", output: null, error: null, steps: 0 })).toBeNull();
    expect(parseExtensionExecutionResult({ executionId: "execution-1", status: "failed", output: "x".repeat(8193), error: null, steps: 1 })).toBeNull();
    expect(parseExtensionExecutionRequest({ agentId: "agent", ownerUserId: "owner", extensionId: "fixture.notes", revision: 1, packageHash: "0".repeat(64), input: "x".repeat(4097), idempotencyKey: "key", temporaryChat: false, extra: true })).toBeNull();
  });
});

describe("metadata-only screen vision contracts", () => {
  const fixture = {
    fixtureId: "fixture:screen/monitor-1/desktop-neutral-v1",
    monitorId: "monitor-1",
    displayName: "Monitor sintético 1",
    width: 1280,
    height: 720,
    scale: 1,
    synthetic: true as const,
    metadataOnly: true as const,
  };
  const privacy = {
    excludeSensitiveContent: true as const,
    redactionRules: [
      { kind: "exclude_sensitive_regions" as const, enabled: true },
    ],
  };
  const preview = {
    fixtureId: fixture.fixtureId,
    monitorId: fixture.monitorId,
    displayName: fixture.displayName,
    width: fixture.width,
    height: fixture.height,
    synthetic: true as const,
    metadataOnly: true as const,
    confirmationRequired: true as const,
    redactionRuleCount: 1,
  };
  const session = {
    id: "session-1",
    agentId: "agt_astra_provisional",
    ownerUserId: "usr_owner_local",
    monitorId: fixture.monitorId,
    fixtureId: fixture.fixtureId,
    status: "active" as const,
    permissions: ["capture_fixture", "analyze_fixture"] as const,
    privacy,
    maxJobs: 4,
    maxDurationMs: 5_000,
    createdAt: 1,
    updatedAt: 1,
    closedAt: null,
  };
  const job = {
    id: "job-1",
    sessionId: session.id,
    agentId: session.agentId,
    ownerUserId: session.ownerUserId,
    monitorId: fixture.monitorId,
    fixtureId: fixture.fixtureId,
    modelFixtureId: "fixture:visual-model/screen-neutral-v1" as const,
    resourceKey: "reference-gpu" as const,
    resourceStatus: "released" as const,
    status: "cleaned" as const,
    terminalStatus: "completed" as const,
    modelLifecycle: "unloaded" as const,
    modelLoadedAt: 2,
    modelRunAt: 2,
    modelCleanupAt: 2,
    cleanupStatus: "complete" as const,
    preview,
    privacy,
    frameMetadataPresent: false,
    resultDurable: false as const,
    errorCode: null,
    createdAt: 1,
    queuedAt: 2,
    runningAt: 2,
    completedAt: 2,
    cleanedAt: 2,
    updatedAt: 2,
  };
  const hypothesis = {
    text: "Hipótese incerta: confirme visualmente.",
    confidence: 42,
    uncertain: true as const,
    diagnostic: false as const,
    durable: false as const,
    sensitiveAttributeInferred: false as const,
    source: "synthetic_fixture_visual_model",
  };
  const result = {
    job,
    hypothesis,
    outputBounded: true as const,
    screenshotBytesPersisted: false as const,
  };

  it("accepts bounded synthetic fixtures, lifecycle records, results and audit", () => {
    expect(parseScreenVisionFixture(fixture)).not.toBeNull();
    expect(parseScreenVisionFixtures([fixture])).not.toBeNull();
    expect(parseScreenVisionPrivacy(privacy)).not.toBeNull();
    expect(parseScreenVisionSession(session)).not.toBeNull();
    expect(parseScreenVisionSessions([session])).not.toBeNull();
    expect(parseScreenVisionJob(job)).not.toBeNull();
    expect(parseScreenVisionHypothesis(hypothesis)).not.toBeNull();
    expect(parseScreenVisionAnalysisResult(result)).not.toBeNull();
    expect(
      parseScreenVisionAudit([
        {
          id: "audit-1",
          sessionId: session.id,
          jobId: job.id,
          agentId: session.agentId,
          event: "job_completed",
          result: "synthetic",
          code: null,
          summary: "Fixture executada sob demanda.",
          createdAt: 2,
        },
      ]),
    ).not.toBeNull();
    expect(
      parseCognitiveError({
        code: "screen_vision_resource_busy",
        message: "Recurso visual ocupado",
      }),
    ).not.toBeNull();
  });

  it("rejects pixels, unsafe privacy, durable visual state and certainty", () => {
    expect(
      parseScreenVisionFixture({
        ...fixture,
        fixtureId: "display:primary",
        monitorId: "display-primary",
        displayName: "Tela principal do Windows",
        synthetic: false,
        metadataOnly: false,
      }),
    ).not.toBeNull();
    expect(
      parseScreenVisionPrivacy({
        ...privacy,
        excludeSensitiveContent: false,
      }),
    ).toBeNull();
    expect(
      parseScreenVisionAnalysisResult({
        ...result,
        screenshotBytes: "not allowed",
      }),
    ).toBeNull();
    expect(
      parseScreenVisionAnalysisResult({
        ...result,
        job: { ...job, resultDurable: true },
      }),
    ).toBeNull();
    expect(
      parseScreenVisionHypothesis({ ...hypothesis, uncertain: false }),
    ).toBeNull();
    expect(parseScreenVisionFixtures([fixture, fixture, fixture, fixture])).not.toBeNull();
    expect(parseScreenVisionFixtures(Array.from({ length: 18 }, () => fixture))).not.toBeNull();
    expect(parseScreenVisionFixtures(Array.from({ length: 19 }, () => fixture))).toBeNull();
  });
});

describe("local-only Android companion contracts", () => {
  const protocol = {
    schemaVersion: 1 as const,
    protocolVersion: COMPANION_PROTOCOL_VERSION,
    minProtocolVersion: 1 as const,
    platform: "android" as const,
    appVersion: COMPANION_FIXTURE_APP_VERSION,
    transport: "tauri_command_fixture" as const,
    networkListener: false as const,
    standaloneFallback: true as const,
  };
  const device = {
    id: "device-1",
    agentId: "agt_astra_provisional",
    ownerUserId: "usr_owner_local",
    deviceId: "android-fixture-01",
    platform: "android" as const,
    appVersion: COMPANION_FIXTURE_APP_VERSION,
    protocolVersion: COMPANION_PROTOCOL_VERSION,
    status: "paired" as const,
    fingerprint: COMPANION_FIXTURE_FINGERPRINT,
    pairingNonceMetadata: COMPANION_FIXTURE_PAIRING_NONCE,
    keyVersion: 1,
    pairingExpiresAt: null,
    pairedAt: 1,
    revokedAt: null,
    lastSeenAt: 1,
    compatible: true as const,
    standaloneFallback: true as const,
    createdAt: 1,
    updatedAt: 1,
  };
  const proof = {
    sessionId: "session-1",
    deviceId: device.deviceId,
    sessionNonceMetadata: "fixture:session/android-fixture-01/one",
    keyFingerprint: device.fingerprint,
    appVersion: COMPANION_FIXTURE_APP_VERSION,
    protocolVersion: COMPANION_PROTOCOL_VERSION,
    messageNonceMetadata: "fixture:message/queue-1",
    replayCounter: 2,
  };
  const session = {
    id: proof.sessionId,
    deviceId: device.deviceId,
    agentId: device.agentId,
    ownerUserId: device.ownerUserId,
    status: "connected" as const,
    protocolVersion: COMPANION_PROTOCOL_VERSION,
    appVersion: COMPANION_FIXTURE_APP_VERSION,
    negotiatedProtocolVersion: COMPANION_PROTOCOL_VERSION,
    keyFingerprint: device.fingerprint,
    sessionNonceMetadata: proof.sessionNonceMetadata,
    lastReplayCounter: 1,
    connectedAt: 1,
    lastSeenAt: 1,
    disconnectedAt: null,
    protocol,
    handshake: {
      schemaVersion: 1 as const,
      protocolVersion: COMPANION_PROTOCOL_VERSION,
      messageId: "session-handshake:session-1",
      deviceId: device.deviceId,
      platform: "android" as const,
      appVersion: COMPANION_FIXTURE_APP_VERSION,
      kind: "session" as const,
      sessionId: proof.sessionId,
      nonceMetadata: "fixture:message/connect-1",
      replayCounter: 1,
      payloadKind: "session",
    },
    updatedAt: 1,
  };
  const payload = { kind: "text" as const, text: "mensagem fixture" };
  const queue = {
    id: "queue-1",
    deviceId: device.deviceId,
    sessionId: session.id,
    agentId: device.agentId,
    ownerUserId: device.ownerUserId,
    kind: "text" as const,
    status: "previewed" as const,
    payload,
    summary: "Texto: mensagem fixture",
    metadataOnly: true as const,
    mediaBytesPersisted: false as const,
    approvalRequired: true as const,
    retryCount: 0,
    errorCode: null,
    createdAt: 1,
    previewedAt: 1,
    approvedAt: null,
    cancelledAt: null,
    updatedAt: 1,
  };

  it("accepts protocol, proof, metadata queue, lifecycle, rotation and audit records", () => {
    expect(parseCompanionProtocolInfo(protocol)).not.toBeNull();
    expect(parseCompanionDevice(device)).not.toBeNull();
    expect(parseCompanionDevices([device])).not.toBeNull();
    expect(parseCompanionSessionProof(proof)).not.toBeNull();
    expect(parseCompanionSession(session)).not.toBeNull();
    expect(parseCompanionSessions([session])).not.toBeNull();
    expect(parseCompanionQueuePayload(payload)).not.toBeNull();
    expect(parseCompanionQueueItem(queue)).not.toBeNull();
    expect(parseCompanionQueue([queue])).not.toBeNull();
    expect(
      parseCompanionHistoryRecord({
        id: "history-1",
        deviceId: device.deviceId,
        sessionId: session.id,
        agentId: device.agentId,
        ownerUserId: device.ownerUserId,
        direction: "outgoing",
        kind: "text",
        summary: "Item criado",
        metadataOnly: true,
        mediaBytesPersisted: false,
        createdAt: 1,
      }),
    ).not.toBeNull();
    expect(
      parseCompanionHistory([
        {
          id: "history-1",
          deviceId: device.deviceId,
          sessionId: session.id,
          agentId: device.agentId,
          ownerUserId: device.ownerUserId,
          direction: "outgoing",
          kind: "text",
          summary: "Item criado",
          metadataOnly: true,
          mediaBytesPersisted: false,
          createdAt: 1,
        },
      ]),
    ).not.toBeNull();
    expect(
      parseCompanionAudit([
        {
          id: "audit-1",
          deviceId: device.deviceId,
          sessionId: session.id,
          queueId: queue.id,
          agentId: device.agentId,
          ownerUserId: device.ownerUserId,
          event: "queue_previewed",
          result: "approval_required",
          code: null,
          summary: "Prévia aguardando aprovação",
          createdAt: 1,
        },
      ]),
    ).not.toBeNull();
    expect(
      parseCompanionKeyRotation({
        id: "rotation-1",
        deviceId: device.deviceId,
        agentId: device.agentId,
        ownerUserId: device.ownerUserId,
        oldFingerprint: device.fingerprint,
        newFingerprint: `${device.fingerprint}-key-v2`,
        oldKeyVersion: 1,
        newKeyVersion: 2,
        nonceMetadata: `${device.pairingNonceMetadata}-nonce-v2`,
        status: "completed",
        reason: "rotação fixture",
        createdAt: 1,
      }),
    ).not.toBeNull();
    expect(
      parseCompanionRevocation({
        id: "revocation-1",
        deviceId: device.deviceId,
        agentId: device.agentId,
        ownerUserId: device.ownerUserId,
        previousStatus: "paired",
        reason: "revogação fixture",
        createdAt: 1,
      }),
    ).not.toBeNull();
  });

  it("rejects network, raw media, incompatible versions and unsafe flags", () => {
    expect(
      parseCompanionProtocolInfo({ ...protocol, networkListener: true }),
    ).toBeNull();
    expect(
      parseCompanionSafetyFlags({
        metadataOnly: true,
        mediaBytesPersisted: true,
        networkListener: false,
        standaloneFallback: true,
      }),
    ).toBeNull();
    expect(
      parseCompanionQueuePayload({ ...payload, rawBytes: "no" }),
    ).toBeNull();
    expect(
      parseCompanionQueueItem({ ...queue, metadataOnly: false }),
    ).toBeNull();
    expect(
      parseCompanionSessionProof({ ...proof, protocolVersion: 99 }),
    ).toBeNull();
    expect(
      parseCompanionDevices([device, device, device, device, device]),
    ).toBeNull();
  });
});
