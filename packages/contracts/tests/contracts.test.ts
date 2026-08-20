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
