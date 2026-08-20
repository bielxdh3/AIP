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
