// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { PhaseOneState } from "@aip/contracts";
import { ConversationSurface } from "./App";

let phase: PhaseOneState | null = null;

vi.mock("./use-phase-one", () => ({
  usePhaseOne: () => ({ phase, error: false, load: vi.fn() }),
}));

const loadedPhase = {
  agent: { id: "agent", name: "Astra" },
  conversation: {
    id: "conversation",
    agentId: "agent",
    title: "Conversa",
    modelOverrideRef: null,
    isPinned: false,
  },
  messages: [
    {
      id: "assistant",
      conversationId: "conversation",
      agentId: "agent",
      author: "agent",
      content: "Resposta elegível",
      modelRef: "ollama:test",
      status: "complete",
      createdAt: 1,
      completedAt: 1,
      errorCode: null,
      branchId: "main",
      turnGroupId: "turn",
    },
  ],
  branches: [],
  turnVariants: [],
  activeBranchId: "main",
  provider: {
    state: "available",
    detailCode: "provider_available",
    models: [{ ref: "ollama:test" }],
    refreshedAt: 1,
  },
  selectedModelRef: "ollama:test",
  defaultModelRef: "ollama:test",
  modelOverrideRef: null,
  effectiveModelSource: "agent_default",
  selectedModelAvailable: true,
  keepAliveMinutes: 5,
  queue: [],
  canSend: true,
  sendBlockedCode: null,
} as unknown as PhaseOneState;

describe("ConversationSurface", () => {
  let root: Root;
  let container: HTMLDivElement;

  afterEach(() => {
    act(() => root?.unmount());
    container?.remove();
    phase = null;
  });

  it("rerenders from loading without a hook-order error and shows retry", () => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => undefined);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);

    expect(() =>
      act(() =>
        root.render(<ConversationSurface agentId="agent" temporary={false} />),
      ),
    ).not.toThrow();
    expect(container.textContent).toContain("Carregando conversa");

    phase = loadedPhase;
    expect(() =>
      act(() =>
        root.render(<ConversationSurface agentId="agent" temporary={false} />),
      ),
    ).not.toThrow();
    expect(container.textContent).toContain("Resposta elegível");
    expect(container.textContent).toContain("Tentar novamente");
    expect(consoleError).not.toHaveBeenCalledWith(
      expect.stringContaining("Rendered more hooks"),
    );
    consoleError.mockRestore();
  });

  it("hides assistant actions while keeping queue cancellation available", () => {
    phase = {
      ...loadedPhase,
      messages: [
        {
          ...loadedPhase.messages[0],
          status: "streaming",
        },
      ],
      queue: [
        {
          agentId: "agent",
          requestId: "request",
          assistantMessageId: "assistant",
          active: true,
          cancellationRequested: false,
        },
      ],
    } as unknown as PhaseOneState;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);

    act(() => {
      root.render(<ConversationSurface agentId="agent" temporary={false} />);
    });

    expect(container.querySelector(".chat-message .message-actions")).toBeNull();
    expect(container.querySelector(".queue-banner")?.textContent).toContain(
      "Cancelar",
    );
  });
});
