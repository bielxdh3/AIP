// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { PhaseOneState } from "@aip/contracts";
import { ConversationDraftSurface } from "./App";

const hookState = vi.hoisted(() => ({
  phase: null as unknown,
  error: false,
  load: vi.fn(),
}));
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("./use-phase-one", () => ({
  usePhaseOne: () => hookState,
}));

const loadedPhase = {
  agent: { id: "agent", name: "Astra" },
  conversation: {
    id: "existing",
    agentId: "agent",
    title: "Conversa existente",
    modelOverrideRef: null,
    isPinned: false,
  },
  messages: [],
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

describe("ConversationDraftSurface", () => {
  let root: Root | undefined;
  let container: HTMLDivElement | undefined;

  afterEach(() => {
    if (root !== undefined) act(() => root?.unmount());
    container?.remove();
    root = undefined;
    container = undefined;
    hookState.phase = null;
    hookState.error = false;
    hookState.load.mockReset();
    invoke.mockReset();
  });

  function renderDraft(onPersisted = vi.fn(), onCreated = vi.fn()) {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    act(() => {
      root?.render(
        <ConversationDraftSurface
          agentId="agent"
          onCreated={onCreated}
          onPersisted={onPersisted}
        />,
      );
    });
    return onPersisted;
  }

  it("preserves the first message and retries without creating duplicates", async () => {
    hookState.phase = loadedPhase;
    const created = {
      ...loadedPhase.conversation,
      id: "created",
      title: "Nova conversa",
    };
    let sendAttempts = 0;
    const onPersisted = vi.fn();
    const onCreated = vi.fn();
    invoke.mockImplementation((command: string) => {
      if (command === "create_agent_conversation")
        return Promise.resolve(created);
      if (command === "send_phase_one_message") {
        sendAttempts += 1;
        return sendAttempts === 1
          ? Promise.reject(new Error("send_failed"))
          : Promise.resolve(undefined);
      }
      return Promise.resolve(undefined);
    });
    renderDraft(onPersisted, onCreated);
    const textarea = container?.querySelector<HTMLTextAreaElement>(
      ".conversation-draft-surface .composer textarea",
    );
    if (textarea === null || textarea === undefined)
      throw new Error("Missing draft composer");
    change(textarea, "mensagem para tentar");
    await act(async () =>
      container
        ?.querySelector<HTMLButtonElement>(
          ".conversation-draft-surface .composer-footer > button",
        )
        ?.click(),
    );

    expect(onCreated).toHaveBeenCalledOnce();
    expect(onPersisted).not.toHaveBeenCalled();
    expect(textarea.value).toBe("mensagem para tentar");
    expect(container?.textContent).toContain("rascunho foi preservado");
    const retry = Array.from(container?.querySelectorAll("button") ?? []).find(
      (button) => button.textContent === "Tentar novamente",
    );
    if (retry === undefined) throw new Error("Missing retry button");
    await act(async () => retry.click());

    expect(
      invoke.mock.calls.filter(
        ([command]) => command === "create_agent_conversation",
      ),
    ).toHaveLength(1);
    expect(
      invoke.mock.calls.filter(
        ([command]) => command === "send_phase_one_message",
      ),
    ).toHaveLength(2);
    expect(onPersisted).toHaveBeenCalledOnce();
    expect(textarea.value).toBe("");
  });

  it("can be abandoned without creating a persisted conversation or fake history", () => {
    hookState.phase = loadedPhase;
    renderDraft();

    expect(container?.querySelector(".message-history")?.textContent).toContain(
      "Rascunho ainda não persistido",
    );
    expect(container?.querySelectorAll(".chat-message")).toHaveLength(0);
    act(() => root?.unmount());
    expect(invoke).not.toHaveBeenCalledWith(
      "create_agent_conversation",
      expect.anything(),
    );
  });

  it("persists with a bounded fallback title before sending the first message", async () => {
    hookState.phase = loadedPhase;
    const created = {
      ...loadedPhase.conversation,
      id: "created",
      title: "Nova conversa",
    };
    invoke.mockImplementation((command: string) =>
      command === "create_agent_conversation"
        ? Promise.resolve(created)
        : Promise.resolve(undefined),
    );
    const onPersisted = renderDraft();
    const textarea = container?.querySelector<HTMLTextAreaElement>(
      ".conversation-draft-surface .composer textarea",
    );
    if (textarea === null || textarea === undefined)
      throw new Error("Missing draft composer");
    change(textarea, "primeira mensagem");
    await act(async () =>
      container
        ?.querySelector<HTMLButtonElement>(
          ".conversation-draft-surface .composer-footer > button",
        )
        ?.click(),
    );

    expect(invoke).toHaveBeenNthCalledWith(1, "create_agent_conversation", {
      agentId: "agent",
      title: "Nova conversa",
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "set_active_agent_conversation", {
      agentId: "agent",
      conversationId: "created",
    });
    expect(invoke).toHaveBeenNthCalledWith(3, "send_phase_one_message", {
      agentId: "agent",
      conversationId: "created",
      content: "primeira mensagem",
      policy: {
        mode: "auto",
        excludedModelRefs: [],
        fallbackOnlyModelRefs: [],
        preferredModelRef: null,
      },
    });
    expect(onPersisted).toHaveBeenCalledOnce();
  });

  it("persists an explicitly named empty conversation without sending", async () => {
    hookState.phase = loadedPhase;
    const created = {
      ...loadedPhase.conversation,
      id: "named",
      title: "Notas",
    };
    invoke.mockImplementation((command: string) =>
      command === "create_agent_conversation"
        ? Promise.resolve(created)
        : Promise.resolve(undefined),
    );
    const onPersisted = renderDraft();
    const title = container?.querySelector<HTMLInputElement>(
      ".draft-title-field input",
    );
    if (title === null || title === undefined) throw new Error("Missing title");
    change(title, "Notas");
    await act(async () =>
      Array.from(container?.querySelectorAll("button") ?? [])
        .find((button) => button.textContent === "Salvar nome")
        ?.click(),
    );

    expect(invoke).toHaveBeenCalledWith("create_agent_conversation", {
      agentId: "agent",
      title: "Notas",
    });
    expect(invoke).toHaveBeenCalledWith("set_active_agent_conversation", {
      agentId: "agent",
      conversationId: "named",
    });
    expect(invoke).not.toHaveBeenCalledWith(
      "send_phase_one_message",
      expect.anything(),
    );
    expect(onPersisted).toHaveBeenCalledOnce();
  });
});
