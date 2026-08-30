// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { PhaseOneState } from "@aip/contracts";
import Bubble from "./Bubble";

let phase: PhaseOneState | null = null;
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("./use-phase-one", () => ({
  usePhaseOne: () => ({ phase, error: false, load: vi.fn() }),
}));

class TestResizeObserver {
  observe() {}
  disconnect() {}
}

const loadedPhase = {
  agent: { id: "agent", name: "Astra" },
  conversation: { id: "conversation" },
  messages: [],
  queue: [],
  canSend: true,
  sendBlockedCode: null,
  provider: { state: "available", models: [] },
} as unknown as PhaseOneState;

const queuedPhase = {
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

describe("Bubble composer", () => {
  let root: Root | undefined;
  let container: HTMLDivElement | undefined;

  afterEach(() => {
    if (root !== undefined) act(() => root?.unmount());
    container?.remove();
    root = undefined;
    container = undefined;
    phase = null;
    vi.clearAllMocks();
  });

  function renderBubble() {
    act(() => root?.render(<Bubble agentId="agent" />));
  }

  function change(element: HTMLTextAreaElement, value: string) {
    const setter = Object.getOwnPropertyDescriptor(
      HTMLTextAreaElement.prototype,
      "value",
    )?.set;
    setter?.call(element, value);
    element.dispatchEvent(new Event("input", { bubbles: true }));
  }

  it("keeps drafting available, blocks send during a request, then sends after cancellation", async () => {
    vi.stubGlobal("ResizeObserver", TestResizeObserver);
    invoke.mockImplementation((command: string) =>
      command === "get_app_snapshot"
        ? Promise.resolve({ safeMode: false })
        : Promise.resolve(undefined),
    );
    phase = queuedPhase;
    container = document.createElement("div");
    const view = container;
    document.body.append(view);
    root = createRoot(view);
    renderBubble();
    await act(async () =>
      view.querySelector<HTMLButtonElement>(".bubble-title")?.click(),
    );

    const textarea = view.querySelector<HTMLTextAreaElement>(
      ".bubble-composer textarea",
    )!;
    const send = view.querySelector<HTMLButtonElement>(
      ".bubble-composer button",
    )!;
    expect(textarea.disabled).toBe(false);
    expect(send.disabled).toBe(true);
    change(textarea, "rascunho no balão");
    await act(async () =>
      textarea.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Enter", bubbles: true }),
      ),
    );
    expect(invoke).not.toHaveBeenCalledWith(
      "send_phase_one_message",
      expect.anything(),
    );

    await act(async () =>
      view.querySelector<HTMLButtonElement>(".bubble-cancel")?.click(),
    );
    expect(invoke).toHaveBeenCalledWith("cancel_phase_one_generation", {
      requestId: "request",
    });

    phase = loadedPhase;
    renderBubble();
    change(textarea, "mensagem seguinte");
    await act(async () => send.click());
    expect(invoke).toHaveBeenCalledWith("send_phase_one_message", {
      agentId: "agent",
      conversationId: "conversation",
      content: "mensagem seguinte",
    });
  });
});
