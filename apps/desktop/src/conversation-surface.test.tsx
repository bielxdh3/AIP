// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { PhaseOneState } from "@aip/contracts";
import { ConversationSurface } from "./App";

let phase: PhaseOneState | null = null;
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

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
  const onActiveConversationChange = vi.fn();

  afterEach(() => {
    act(() => root?.unmount());
    container?.remove();
    phase = null;
    vi.clearAllMocks();
  });

  function renderSurface(temporary = false, onToggleTemporary?: () => void) {
    act(() => {
      root.render(
        <ConversationSurface
          agentId="agent"
          temporary={temporary}
          onToggleTemporary={onToggleTemporary}
          onActiveConversationChange={onActiveConversationChange}
        />,
      );
    });
  }

  function change(element: HTMLTextAreaElement, value: string) {
    const setter = Object.getOwnPropertyDescriptor(
      HTMLTextAreaElement.prototype,
      "value",
    )?.set;
    setter?.call(element, value);
    element.dispatchEvent(new Event("input", { bubbles: true }));
  }

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
        root.render(
          <ConversationSurface
            agentId="agent"
            temporary={false}
            onActiveConversationChange={onActiveConversationChange}
          />,
        ),
      ),
    ).not.toThrow();
    expect(container.textContent).toContain("Carregando conversa");

    phase = loadedPhase;
    expect(() =>
      act(() =>
        root.render(
          <ConversationSurface
            agentId="agent"
            temporary={false}
            onActiveConversationChange={onActiveConversationChange}
          />,
        ),
      ),
    ).not.toThrow();
    expect(container.textContent).toContain("Resposta elegível");
    expect(onActiveConversationChange).toHaveBeenCalledWith("conversation");
    expect(container.textContent).toContain("Tentar novamente");
    expect(
      container.querySelector(
        '.conversation-model-selector [aria-haspopup="listbox"]',
      ),
    ).not.toBeNull();
    expect(
      container.querySelector(
        '.conversation-model-selector [aria-haspopup="listbox"]',
      )?.textContent,
    ).toContain("Modelo: Automático");
    expect(
      container.querySelector(".conversation-model-selector")?.textContent,
    ).toContain("Equilibrado");
    expect(
      container.querySelector(".conversation-model-selector select"),
    ).toBeNull();
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

    renderSurface();

    expect(
      container.querySelector(".chat-message .message-actions"),
    ).toBeNull();
    expect(container.querySelector(".queue-banner")?.textContent).toContain(
      "Cancelar",
    );
    expect(
      container.querySelector(".generation-status.shiny-text")?.textContent,
    ).toBe("Gerando resposta…");

    phase = {
      ...loadedPhase,
      queue: [
        {
          agentId: "agent",
          requestId: "request",
          assistantMessageId: "assistant",
          active: true,
          cancellationRequested: true,
        },
      ],
    } as unknown as PhaseOneState;
    renderSurface();
    expect(container.querySelector(".generation-status.shiny-text")).toBeNull();
    expect(container.querySelector(".generation-status")?.textContent).toBe(
      "Cancelando resposta…",
    );
  });

  it("keeps the draft editable but blocks send and Enter during generation", async () => {
    phase = {
      ...loadedPhase,
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
    renderSurface();

    const textarea =
      container.querySelector<HTMLTextAreaElement>(".composer textarea")!;
    const send = container.querySelector<HTMLButtonElement>(
      ".composer-footer > button",
    )!;
    expect(textarea.disabled).toBe(false);
    expect(send.disabled).toBe(true);
    await act(async () => {
      change(textarea, "rascunho para depois");
      textarea.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Enter", bubbles: true }),
      );
    });
    expect(invoke).not.toHaveBeenCalledWith(
      "send_phase_one_message",
      expect.anything(),
    );
  });

  it("allows a new send after cancellation removes the queued request", async () => {
    invoke.mockResolvedValue(undefined);
    phase = {
      ...loadedPhase,
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
    renderSurface();
    await act(async () =>
      container
        .querySelector<HTMLButtonElement>(".queue-banner button")
        ?.click(),
    );
    expect(invoke).toHaveBeenCalledWith("cancel_phase_one_generation", {
      requestId: "request",
    });

    phase = loadedPhase;
    renderSurface();
    const textarea =
      container.querySelector<HTMLTextAreaElement>(".composer textarea")!;
    change(textarea, "próxima mensagem");
    await act(async () =>
      container
        .querySelector<HTMLButtonElement>(".composer-footer > button")
        ?.click(),
    );
    expect(invoke).toHaveBeenCalledWith("send_phase_one_message", {
      agentId: "agent",
      conversationId: "conversation",
      content: "próxima mensagem",
      policy: {
        mode: "auto",
        excludedModelRefs: [],
        fallbackOnlyModelRefs: [],
        preferredModelRef: null,
      },
    });
  });

  it("passes the bounded routing policy to temporary sends", async () => {
    phase = loadedPhase;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    renderSurface(true);
    expect(onActiveConversationChange).not.toHaveBeenCalled();

    const textarea =
      container.querySelector<HTMLTextAreaElement>(".composer textarea")!;
    change(textarea, "mensagem temporária");
    await act(async () =>
      container
        .querySelector<HTMLButtonElement>(".composer-footer > button")
        ?.click(),
    );

    expect(invoke).toHaveBeenCalledWith("send_temporary_phase_one_message", {
      agentId: "agent",
      content: "mensagem temporária",
      policy: {
        mode: "auto",
        excludedModelRefs: [],
        fallbackOnlyModelRefs: [],
        preferredModelRef: null,
      },
    });
  });

  it("keeps provider recovery and temporary controls inside the composer", () => {
    phase = {
      ...loadedPhase,
      provider: { ...loadedPhase.provider, state: "unavailable" },
      selectedModelAvailable: false,
      canSend: false,
      sendBlockedCode: "provider_unavailable",
    } as unknown as PhaseOneState;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    const onToggleTemporary = vi.fn();
    renderSurface(false, onToggleTemporary);

    expect(
      container.querySelector(".conversation-header .provider-state"),
    ).toBeNull();
    expect(
      container.querySelector(".composer .provider-state")?.textContent,
    ).toContain("Ollama indisponível");
    expect(
      container.querySelector(".provider-recovery")?.textContent,
    ).toContain("Abra o Ollama");
    const temporaryControl = container.querySelector<HTMLButtonElement>(
      ".composer-actions .temporary-control",
    );
    expect(temporaryControl?.getAttribute("aria-label")).toBe(
      "Iniciar conversa temporária",
    );
    expect(temporaryControl?.title).toBe("Iniciar conversa temporária");
    expect(
      container.querySelector<HTMLButtonElement>(".composer-footer > button")
        ?.disabled,
    ).toBe(true);
  });
});
