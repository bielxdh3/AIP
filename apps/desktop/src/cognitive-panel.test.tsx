// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { CognitiveGoal } from "@aip/contracts";
import { CognitivePanel, CognitivePanelGate } from "./App";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const event = {
  id: "event-1",
  agentId: "agt_astra_provisional",
  kind: "trait_delta" as const,
  traitKey: "curiosity",
  sourceKind: "controlled_internal",
  sourceReference: "processor:evidence",
  reason: "Evidência aprovada",
  confidence: 1,
  requestedValue: 0.05,
  appliedDelta: 0.05,
  priorValue: 0.5,
  resultingValue: 0.55,
  status: "applied" as const,
  code: null,
  rollbackOfEventId: null,
  createdAt: 1,
  rawPayload: "ignore this internal payload",
};

const traits = [
  { key: "protected_identity", value: 0.5, isProtected: true },
  { key: "curiosity", value: 0.5, isProtected: false },
];

function change(
  element: HTMLInputElement | HTMLTextAreaElement,
  value: string,
) {
  const setter = Object.getOwnPropertyDescriptor(
    Object.getPrototypeOf(element),
    "value",
  )?.set;
  setter?.call(element, value);
  element.dispatchEvent(new Event("input", { bubbles: true }));
}

describe("CognitivePanel", () => {
  let root: Root;
  let container: HTMLDivElement;

  afterEach(() => {
    act(() => root?.unmount());
    container?.remove();
    invoke.mockReset();
  });

  it("renders safe Portuguese cognitive controls and refreshes corrections, rollback and explanation", async () => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    invoke.mockImplementation((command: string) => {
      if (command === "list_cognitive_traits") return Promise.resolve(traits);
      if (command === "list_cognitive_events")
        return Promise.resolve([
          event,
          { ...event, id: "rejected", status: "rejected" },
        ]);
      if (
        command === "list_cognitive_opinions" ||
        command === "list_cognitive_relationships" ||
        command === "list_cognitive_goals"
      )
        return Promise.resolve([]);
      if (command === "explain_cognitive_event")
        return Promise.resolve({ event, traitLabel: "Curiosidade" });
      return Promise.resolve(event);
    });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);

    expect(container.textContent).toBe("");
    await act(async () =>
      root.render(<CognitivePanel agentId="agt_astra_provisional" />),
    );
    expect(container.textContent).toContain("Valores cognitivos simulados");
    expect(container.textContent).toContain("não representam emoções reais");
    expect(container.textContent).toContain(
      "protected_identity: 0.50 — protegido",
    );
    expect(container.textContent).toContain("curiosity: 0.50 — evolutivo");
    expect(container.querySelectorAll('[aria-label^="Reverter"]').length).toBe(
      1,
    );
    expect(container.querySelector('[aria-label^="Corrigir"]')).not.toBeNull();
    expect(container.querySelector('[aria-label^="Explicar"]')).not.toBeNull();

    const correction = container.querySelector(
      '[aria-label^="Corrigir"]',
    ) as HTMLButtonElement;
    await act(async () => correction.click());
    expect(container.textContent).toContain("Informe o motivo da correção.");

    const input = container.querySelector("input") as HTMLInputElement;
    const reason = container.querySelector("textarea") as HTMLTextAreaElement;
    await act(async () => {
      change(input, "2");
      change(reason, "Motivo válido");
    });
    await act(async () => correction.click());
    expect(container.textContent).toContain("Informe um valor entre 0 e 1.");

    await act(async () => change(input, "0.6"));
    await act(async () => correction.click());
    expect(invoke).toHaveBeenCalledWith(
      "create_owner_trait_correction",
      expect.objectContaining({
        agentId: "agt_astra_provisional",
        value: 0.6,
        reason: "Motivo válido",
        temporaryChat: false,
      }),
    );
    expect(invoke).toHaveBeenCalledWith("list_cognitive_traits", {
      agentId: "agt_astra_provisional",
    });
    expect(container.textContent).toContain("Correção aplicada.");

    await act(async () =>
      (
        container.querySelector('[aria-label^="Explicar"]') as HTMLButtonElement
      ).click(),
    );
    expect(container.textContent).toContain(
      "Curiosidade: 0.50 → 0.55. Evidência aprovada",
    );
    expect(container.textContent).not.toContain("processor:evidence");
    expect(container.textContent).not.toContain("ignore this internal payload");
    await act(async () =>
      (
        container.querySelector('[aria-label^="Reverter"]') as HTMLButtonElement
      ).click(),
    );
    expect(invoke).toHaveBeenCalledWith(
      "rollback_cognitive_event",
      expect.objectContaining({
        eventId: "event-1",
        temporaryChat: false,
      }),
    );
    expect(container.textContent).toContain("Reversão aplicada.");
  });

  it("ignores late responses after switching agents and survives unavailable commands", async () => {
    let resolveAstra: ((value: typeof traits) => void) | undefined;
    invoke.mockImplementation((command: string, args: { agentId: string }) => {
      if (args.agentId === "agt_astra_provisional") {
        return new Promise((resolve) => {
          resolveAstra = resolve;
        });
      }
      if (command === "list_cognitive_traits")
        return Promise.resolve([
          { key: "autonomy", value: 0.7, isProtected: false },
        ]);
      return Promise.resolve([]);
    });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () =>
      root.render(<CognitivePanel agentId="agt_astra_provisional" />),
    );
    await act(async () =>
      root.render(<CognitivePanel agentId="agt_luma_provisional" />),
    );
    await act(async () => resolveAstra?.(traits));
    expect(container.textContent).toContain("autonomy: 0.70");
    expect(container.textContent).not.toContain("curiosity: 0.50");
  });

  it("shows loading, keeps empty history safe, and survives safe-mode failures", async () => {
    let resolveTraits: ((value: typeof traits) => void) | undefined;
    let resolveEvents: ((value: (typeof event)[]) => void) | undefined;
    invoke.mockImplementation(
      (command: string) =>
        new Promise((resolve) => {
          if (command === "list_cognitive_traits") resolveTraits = resolve;
          else if (command === "list_cognitive_events") resolveEvents = resolve;
          else resolve([]);
        }),
    );
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    act(() =>
      root.render(<CognitivePanel agentId="agt_astra_provisional" />),
    );
    expect(container.textContent).toContain("Carregando valores cognitivos");
    await act(async () => {
      resolveTraits?.(traits);
      resolveEvents?.([]);
    });
    expect(container.textContent).toContain("Histórico recente");
    expect(container.querySelector('[aria-label^="Reverter"]')).toBeNull();

    invoke.mockRejectedValue("operation_unavailable");
    await act(async () =>
      root.render(<CognitivePanel agentId="agt_luma_provisional" />),
    );
    expect(container.textContent).toContain(
      "Não foi possível carregar os valores cognitivos.",
    );
  });

  it("maps stable backend errors to Portuguese copy", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_cognitive_traits") return Promise.resolve(traits);
      if (command === "list_cognitive_events") return Promise.resolve([]);
      if (
        command === "list_cognitive_opinions" ||
        command === "list_cognitive_relationships" ||
        command === "list_cognitive_goals"
      )
        return Promise.resolve([]);
      return Promise.reject("protected_trait");
    });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () =>
      root.render(<CognitivePanel agentId="agt_astra_provisional" />),
    );
    await act(async () =>
      change(
        container.querySelector("textarea") as HTMLTextAreaElement,
        "Motivo válido",
      ),
    );
    await act(async () =>
      (
        container.querySelector('[aria-label^="Corrigir"]') as HTMLButtonElement
      ).click(),
    );
    expect(container.textContent).toContain("Este traço é protegido.");
  });

  it("lists and mutates the Portuguese 7B-7D surface with bounded commands", async () => {
    const opinion = {
      id: "opinion-1",
      agentId: "agt_astra_provisional",
      subjectType: "topic",
      subjectRef: "tema",
      stance: 0.4,
      confidence: 0.8,
      status: "active" as const,
      reason: "Motivo",
      createdAt: 1,
      updatedAt: 1,
      evidence: [
        {
          id: "evidence-1",
          opinionId: "opinion-1",
          sourceKind: "owner_testimony",
          classification: "verified_fact",
          stance: 0.4,
          claimKey: "owner_claim",
          claimValue: "Evidência segura",
          sourceReference: null,
          attribution: null,
          confidence: 0.8,
          status: "active" as const,
          createdAt: 1,
        },
      ],
    };
    const relationship = {
      id: "relationship-1",
      agentId: "agt_astra_provisional",
      subjectType: "agent",
      subjectRef: "agt_related",
      values: {
        familiarity: 0.5,
        trust: 0.55,
        affinity: 0.5,
        admiration: 0.5,
        irritation: 0,
        reliabilityExpectation: 0.5,
      },
      updatedAt: 1,
      events: [
        {
          id: "relationship-event-1",
          relationshipId: "relationship-1",
          eventId: "core-event-1",
          deltas: {
            familiarity: 0,
            trust: 0.05,
            affinity: 0,
            admiration: 0,
            irritation: 0,
            reliabilityExpectation: 0,
          },
          prior: {
            familiarity: 0.5,
            trust: 0.5,
            affinity: 0.5,
            admiration: 0.5,
            irritation: 0,
            reliabilityExpectation: 0.5,
          },
          resulting: {
            familiarity: 0.5,
            trust: 0.55,
            affinity: 0.5,
            admiration: 0.5,
            irritation: 0,
            reliabilityExpectation: 0.5,
          },
          sourceKind: "owner_testimony",
          sourceReference: null,
          confidence: 0.8,
          reason: "Motivo",
          status: "applied" as const,
          createdAt: 1,
        },
      ],
    };
    let goal: CognitiveGoal = {
      id: "goal-1",
      agentId: "agt_astra_provisional",
      title: "Objetivo de teste",
      description: "Um objetivo sem ação externa",
      origin: "agent_proposal" as const,
      fictionalOnly: true as const,
      priority: 50,
      status: "proposed" as const,
      budgetUnits: 10,
      dueAt: null,
      expiresAt: null,
      completionEvidence: null,
      parentGoalId: null,
      createdAt: 1,
      updatedAt: 1,
    };
    let rejectOpinionStatus = false;
    invoke.mockImplementation((command: string) => {
      if (command === "list_cognitive_traits") return Promise.resolve(traits);
      if (command === "list_cognitive_events") return Promise.resolve([]);
      if (command === "list_cognitive_opinions")
        return Promise.resolve([opinion]);
      if (command === "list_cognitive_relationships")
        return Promise.resolve([relationship]);
      if (command === "list_cognitive_goals") return Promise.resolve([goal]);
      if (command === "list_fictional_activities")
        return Promise.resolve([
          {
            id: "activity-1",
            goalId: "goal-1",
            agentId: "agt_astra_provisional",
            activityType: "fictional-reading",
            status: "active" as const,
            fictionalOnly: true as const,
            budgetUnits: 1,
            startedAt: 1,
            endedAt: null,
            createdAt: 1,
          },
        ]);
      if (command === "set_cognitive_opinion_status") {
        return rejectOpinionStatus
          ? Promise.reject("ownership_mismatch")
          : Promise.resolve(opinion);
      }
      if (
        command === "correct_cognitive_opinion_evidence" ||
        command === "recalculate_cognitive_opinion" ||
        command === "reset_cognitive_relationship" ||
        command === "rollback_cognitive_relationship"
      )
        return Promise.resolve(opinion);
      if (command === "create_owner_cognitive_goal")
        return Promise.resolve({
          ...goal,
          origin: "owner" as const,
          status: "active" as const,
        });
      if (command === "approve_cognitive_goal") {
        goal = { ...goal, status: "active" };
        return Promise.resolve(goal);
      }
      if (command === "update_cognitive_goal_status") {
        goal = {
          ...goal,
          status: "completed",
          completionEvidence: "Concluído em estado fictício",
        };
        return Promise.resolve(goal);
      }
      return Promise.resolve(opinion);
    });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () =>
      root.render(<CognitivePanel agentId="agt_astra_provisional" />),
    );
    expect(container.textContent).toContain("Núcleo cognitivo");
    expect(container.textContent).toContain("tema: posição 0.40");
    expect(container.textContent).toContain("Evidência segura");
    expect(container.textContent).toContain("Objetivo de teste");
    expect(container.textContent).toContain("fictional-reading — active");
    expect(container.textContent).toContain("familiaridade");
    expect(container.textContent).toContain("fonte:");

    const inputs = container.querySelectorAll("input");
    const textareas = container.querySelectorAll("textarea");
    await act(async () => {
      change(inputs[1] as HTMLInputElement, "novo tema");
      change(textareas[1] as HTMLTextAreaElement, "Nova evidência");
      change(textareas[2] as HTMLTextAreaElement, "Novo motivo");
    });
    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find((button) => button.textContent === "Propor opinião")
        ?.click(),
    );
    expect(invoke).toHaveBeenCalledWith(
      "propose_cognitive_opinion",
      expect.objectContaining({
        agentId: "agt_astra_provisional",
        subjectRef: "novo tema",
        sourceKind: "owner_testimony",
        sourceReference: null,
        temporaryChat: false,
      }),
    );

    change(
      container.querySelectorAll("textarea")[3] as HTMLTextAreaElement,
      "Owner corrigiu a evidência",
    );
    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find((button) => button.textContent === "Corrigir evidência")
        ?.click(),
    );
    change(
      container.querySelectorAll("textarea")[4] as HTMLTextAreaElement,
      "Evidência corrigida",
    );
    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find(
          (button) => button.textContent === "Confirmar correção da evidência",
        )
        ?.click(),
    );
    expect(invoke).toHaveBeenCalledWith(
      "correct_cognitive_opinion_evidence",
      expect.objectContaining({
        agentId: "agt_astra_provisional",
        evidenceId: "evidence-1",
        claimValue: "Evidência corrigida",
        temporaryChat: false,
      }),
    );

    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find(
          (button) => button.textContent === "Marcar opinião como disputada",
        )
        ?.click(),
    );
    expect(invoke).toHaveBeenCalledWith(
      "set_cognitive_opinion_status",
      expect.objectContaining({
        agentId: "agt_astra_provisional",
        opinionId: "opinion-1",
        status: "disputed",
        temporaryChat: false,
      }),
    );
    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find((button) => button.textContent === "Recalcular opinião")
        ?.click(),
    );
    expect(invoke).toHaveBeenCalledWith(
      "recalculate_cognitive_opinion",
      expect.objectContaining({
        agentId: "agt_astra_provisional",
        opinionId: "opinion-1",
        temporaryChat: false,
      }),
    );

    const relationshipReason = Array.from(container.querySelectorAll("label"))
      .find((label) =>
        label.textContent?.startsWith(
          "Motivo da redefinição do relacionamento",
        ),
      )
      ?.querySelector("textarea");
    change(
      relationshipReason as HTMLTextAreaElement,
      "Owner pediu redefinição",
    );
    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find((button) => button.textContent === "Redefinir relacionamento")
        ?.click(),
    );
    expect(invoke).toHaveBeenCalledWith(
      "reset_cognitive_relationship",
      expect.objectContaining({
        agentId: "agt_astra_provisional",
        relationshipId: "relationship-1",
        temporaryChat: false,
      }),
    );
    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find((button) => button.textContent === "Reverter último evento")
        ?.click(),
    );
    expect(invoke).toHaveBeenCalledWith(
      "rollback_cognitive_relationship",
      expect.objectContaining({
        agentId: "agt_astra_provisional",
        eventId: "core-event-1",
        temporaryChat: false,
      }),
    );

    const goalTitleInput = Array.from(container.querySelectorAll("label"))
      .find((label) => label.textContent?.startsWith("Título do objetivo"))
      ?.querySelector("input");
    const goalDescriptionInput = Array.from(container.querySelectorAll("label"))
      .find((label) => label.textContent?.startsWith("Descrição do objetivo"))
      ?.querySelector("textarea");
    change(goalTitleInput as HTMLInputElement, "Objetivo do Owner");
    change(
      goalDescriptionInput as HTMLTextAreaElement,
      "Objetivo sem ação externa",
    );
    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find((button) => button.textContent === "Criar objetivo do Owner")
        ?.click(),
    );
    expect(invoke).toHaveBeenCalledWith(
      "create_owner_cognitive_goal",
      expect.objectContaining({
        agentId: "agt_astra_provisional",
        title: "Objetivo do Owner",
        temporaryChat: false,
      }),
    );

    rejectOpinionStatus = true;
    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find(
          (button) => button.textContent === "Marcar opinião como disputada",
        )
        ?.click(),
    );
    expect(container.textContent).toContain(
      "Este registro pertence a outro agente.",
    );

    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find((button) => button.textContent === "Aprovar objetivo")
        ?.click(),
    );
    expect(invoke).toHaveBeenCalledWith(
      "approve_cognitive_goal",
      expect.objectContaining({
        agentId: "agt_astra_provisional",
        goalId: "goal-1",
        temporaryChat: false,
      }),
    );
    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find((button) => button.textContent === "Concluir objetivo")
        ?.click(),
    );
    expect(invoke).toHaveBeenCalledWith(
      "update_cognitive_goal_status",
      expect.objectContaining({
        agentId: "agt_astra_provisional",
        goalId: "goal-1",
        status: "completed",
        temporaryChat: false,
      }),
    );
    expect(container.textContent).toContain("estado fictício");
  });

  it(
    "invokes bounded public conversation and resource commands with pending-only candidates",
    async () => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    let conversation = {
      id: "conversation-1",
      initiatorAgentId: "agt_astra_provisional",
      participantAgentId: "agt_luma_provisional",
      purpose: "planejamento fictício",
      status: "active" as const,
      maxTurns: 12,
      maxTokens: 2048,
      maxDurationMs: 300000,
      maxRepetitions: 2,
      resourceBudget: 20,
      turnCount: 0,
      tokenCount: 0,
      loopCount: 0,
      terminationReason: null,
      createdAt: 1,
      updatedAt: 1,
      completedAt: null,
    };
    const policy = {
      agentId: "agt_astra_provisional",
      purpose: "planejamento fictício",
      optedIn: true,
      maxTurns: 12,
      maxTokens: 2048,
      maxDurationMs: 300000,
      maxRepetitions: 2,
      resourceBudget: 20,
      revokedAt: null,
      updatedAt: 1,
    };
    const pendingCandidate = {
      id: "candidate-pending",
      conversationId: conversation.id,
      agentId: "agt_astra_provisional",
      candidateKind: "opinion" as const,
      candidateJson: '{"subject":"tema","stance":0.2}',
      sourceReference: "conversation-1",
      status: "pending" as const,
      createdAt: 1,
    };
    const rejectedCandidate = {
      ...pendingCandidate,
      id: "candidate-rejected",
      status: "rejected" as const,
    };
    let candidates = [pendingCandidate, rejectedCandidate];
    const resourceJob = {
      id: "resource-job-1",
      agentId: "agt_astra_provisional",
      conversationId: conversation.id,
      jobKind: "heavy_generation",
      heavy: true,
      priority: 50,
      budgetUnits: 1,
      status: "running" as const,
      errorCode: null,
      createdAt: 1,
      startedAt: 1,
      endedAt: null,
    };
    const inspection = () => ({
      conversation,
      turns: [
        {
          id: "turn-1",
          conversationId: conversation.id,
          speakerAgentId: "agt_astra_provisional",
          turnIndex: 0,
          content: "Turno público seguro",
          sourceKind: "owner" as const,
          createdAt: 1,
        },
      ],
    });

    invoke.mockImplementation(
      (command: string, args: Record<string, unknown>) => {
        if (
          command === "list_cognitive_traits" ||
          command === "list_cognitive_events" ||
          command === "list_cognitive_opinions" ||
          command === "list_cognitive_relationships" ||
          command === "list_cognitive_goals"
        )
          return Promise.resolve([]);
        if (command === "list_agent_conversation_policies")
          return Promise.resolve([
            policy,
            { ...policy, agentId: "agt_luma_provisional" },
          ]);
        if (command === "list_cognitive_conversations")
          return Promise.resolve([conversation]);
        if (command === "list_cognitive_candidates")
          return Promise.resolve(candidates);
        if (command === "set_agent_conversation_policy")
          return Promise.resolve({
            ...policy,
            agentId: String(args.agentId),
            optedIn: Boolean(args.optedIn),
          });
        if (command === "start_agent_conversation") {
          conversation = { ...conversation, id: "conversation-started" };
          return Promise.resolve(conversation);
        }
        if (command === "inspect_agent_conversation")
          return Promise.resolve(inspection());
        if (command === "append_public_conversation_turn")
          return Promise.resolve(inspection());
        if (command === "reserve_heavy_generation")
          return Promise.resolve({
            ...resourceJob,
            conversationId: conversation.id,
          });
        if (command === "complete_resource_job")
          return Promise.resolve({
            ...resourceJob,
            status: "completed" as const,
            endedAt: 2,
          });
        if (command === "reject_cognitive_candidate") {
          candidates = [rejectedCandidate];
          return Promise.resolve(rejectedCandidate);
        }
        return Promise.resolve([]);
      },
    );
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () =>
      root.render(<CognitivePanel agentId="agt_astra_provisional" />),
    );

    expect(
      Array.from(container.querySelectorAll("button")).filter(
        (button) => button.textContent === "Rejeitar candidato",
      ),
    ).toHaveLength(1);
    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find((button) => button.textContent === "Salvar autorização pública")
        ?.click(),
    );
    expect(invoke).toHaveBeenCalledWith(
      "set_agent_conversation_policy",
      expect.objectContaining({
        agentId: "agt_astra_provisional",
        temporaryChat: false,
      }),
    );
    expect(invoke).toHaveBeenCalledWith(
      "set_agent_conversation_policy",
      expect.objectContaining({
        agentId: "agt_luma_provisional",
        temporaryChat: false,
      }),
    );

    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find((button) => button.textContent === "Iniciar conversa pública")
        ?.click(),
    );
    expect(invoke).toHaveBeenCalledWith(
      "start_agent_conversation",
      expect.objectContaining({
        initiatorAgentId: "agt_astra_provisional",
        participantAgentId: "agt_luma_provisional",
        temporaryChat: false,
      }),
    );

    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find((button) => button.textContent === "Inspecionar turnos públicos")
        ?.click(),
    );
    expect(invoke).toHaveBeenCalledWith(
      "inspect_agent_conversation",
      expect.objectContaining({ agentId: "agt_astra_provisional" }),
    );
    const turn = Array.from(container.querySelectorAll("label"))
      .find((label) => label.textContent?.startsWith("Turno público"))
      ?.querySelector("textarea");
    await act(async () =>
      change(turn as HTMLTextAreaElement, "Turno do Owner"),
    );
    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find((button) => button.textContent === "Registrar turno público")
        ?.click(),
    );
    expect(invoke).toHaveBeenCalledWith(
      "append_public_conversation_turn",
      expect.objectContaining({
        content: "Turno do Owner",
        sourceKind: "owner",
        temporaryChat: false,
      }),
    );

    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find((button) => button.textContent === "Reservar geração pesada")
        ?.click(),
    );
    expect(invoke).toHaveBeenCalledWith(
      "reserve_heavy_generation",
      expect.objectContaining({
        priority: 50,
        budgetUnits: 1,
      }),
    );
    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find((button) => button.textContent === "Concluir trabalho pesado")
        ?.click(),
    );
    expect(invoke).toHaveBeenCalledWith(
      "complete_resource_job",
      expect.objectContaining({
        status: "completed",
        temporaryChat: false,
      }),
    );

    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find((button) => button.textContent === "Interromper conversa pública")
        ?.click(),
    );
    expect(invoke).toHaveBeenCalledWith(
      "interrupt_agent_conversation",
      expect.objectContaining({
        agentId: "agt_astra_provisional",
        temporaryChat: false,
      }),
    );

    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find((button) => button.textContent === "Rejeitar candidato")
        ?.click(),
    );
    expect(invoke).toHaveBeenCalledWith(
      "reject_cognitive_candidate",
      expect.objectContaining({ candidateId: "candidate-pending" }),
    );
    expect(
      Array.from(container.querySelectorAll("button")).filter(
        (button) => button.textContent === "Rejeitar candidato",
      ),
    ).toHaveLength(0);
      expect(
        invoke.mock.calls.some(
          ([command]) => command === "emit_cognitive_candidate",
        ),
      ).toBe(false);
    },
  );

  it("renders safe Portuguese copy for a public conversation backend error", async () => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    invoke.mockImplementation((command: string) => {
      if (
        command === "list_cognitive_traits" ||
        command === "list_cognitive_events" ||
        command === "list_cognitive_opinions" ||
        command === "list_cognitive_relationships" ||
        command === "list_cognitive_goals"
      )
        return Promise.resolve([]);
      if (command === "list_agent_conversation_policies")
        return Promise.resolve([]);
      if (command === "list_cognitive_conversations")
        return Promise.resolve([]);
      if (command === "list_cognitive_candidates") return Promise.resolve([]);
      if (command === "start_agent_conversation")
        return Promise.reject("conversation_opt_in_required");
      return Promise.resolve([]);
    });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () =>
      root.render(<CognitivePanel agentId="agt_astra_provisional" />),
    );
    await act(async () =>
      Array.from(container.querySelectorAll("button"))
        .find((button) => button.textContent === "Iniciar conversa pública")
        ?.click(),
    );
    expect(container.textContent).toContain(
      "Os dois agentes precisam autorizar este propósito explicitamente.",
    );
  });

  it("suppresses durable cognitive controls during temporary chat", async () => {
    invoke.mockImplementation((command: string) => {
      throw new Error(`unexpected temporary-chat command: ${command}`);
    });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () =>
      root.render(
        <CognitivePanelGate agentId="agt_astra_provisional" temporaryChat />,
      ),
    );

    expect(container.textContent).toContain("Conversa temporária ativa");
    expect(container.textContent).toContain("somente para leitura");
    expect(container.textContent).not.toContain("Núcleo cognitivo");
    expect(container.querySelector("button")).toBeNull();
    expect(invoke).not.toHaveBeenCalled();
  });
});
