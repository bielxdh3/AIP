// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { PhaseOneState } from "@aip/contracts";
import Bubble from "./Bubble";
import {
  BUBBLE_NATIVE_CLOSE_EVENT,
  BUBBLE_NATIVE_OPEN_EVENT,
} from "./overlay-events";

let phase: PhaseOneState | null = null;
const phasesByAgent = new Map<string, PhaseOneState>();
const { invoke, listen, usePhaseOne } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  usePhaseOne: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));
vi.mock("./use-phase-one", () => ({ usePhaseOne }));

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

const astraPhase = {
  ...loadedPhase,
  agent: { id: "astra", name: "Astra" },
  conversation: { id: "astra-conversation" },
} as unknown as PhaseOneState;

const lumaPhase = {
  ...loadedPhase,
  agent: { id: "luma", name: "Luma" },
  conversation: { id: "luma-conversation" },
} as unknown as PhaseOneState;

type LifecycleEventCallback = (event: {
  payload: { agentId: string };
}) => void;

describe("Bubble composer", () => {
  let root: Root | undefined;
  let container: HTMLDivElement | undefined;
  const listeners = new Map<string, LifecycleEventCallback[]>();

  afterEach(() => {
    if (root !== undefined) act(() => root?.unmount());
    container?.remove();
    root = undefined;
    container = undefined;
    phase = null;
    phasesByAgent.clear();
    listeners.clear();
    vi.clearAllMocks();
  });

  function renderBubbles(agentIds: readonly string[] = ["agent"]) {
    usePhaseOne.mockImplementation((agentId: string) => ({
      phase: phasesByAgent.get(agentId) ?? phase,
      error: false,
      load: vi.fn(),
    }));
    listen.mockImplementation(
      (event: string, callback: LifecycleEventCallback) => {
        const callbacks = listeners.get(event) ?? [];
        callbacks.push(callback);
        listeners.set(event, callbacks);
        return Promise.resolve(() => {
          const remaining = (listeners.get(event) ?? []).filter(
            (candidate) => candidate !== callback,
          );
          if (remaining.length > 0) listeners.set(event, remaining);
          else listeners.delete(event);
        });
      },
    );
    act(() =>
      root?.render(
        <>
          {agentIds.map((agentId) => (
            <Bubble key={agentId} agentId={agentId} />
          ))}
        </>,
      ),
    );
  }

  function renderBubble() {
    renderBubbles();
  }

  function emitLifecycle(event: string, agentId: string) {
    for (const callback of listeners.get(event) ?? []) {
      callback({ payload: { agentId } });
    }
  }

  async function flushListenerRegistration() {
    await act(async () => {
      await Promise.resolve();
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

  function latestGeometry() {
    return [...invoke.mock.calls]
      .reverse()
      .find(([command]) => command === "set_overlay_bubble_geometry")?.[1] as
      { width: number; height: number } | undefined;
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
    await act(async () => {
      await Promise.resolve();
    });
    expect(invoke).toHaveBeenCalledWith("set_overlay_interactive_regions", {
      agentId: "agent",
      regions: [],
    });
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
      policy: {
        mode: "auto",
        excludedModelRefs: [],
        fallbackOnlyModelRefs: [],
        preferredModelRef: null,
      },
    });
  });

  it("minimizes to a complete small bubble and restores the expanded state", async () => {
    vi.stubGlobal("ResizeObserver", TestResizeObserver);
    invoke.mockImplementation((command: string) =>
      command === "get_app_snapshot"
        ? Promise.resolve({ safeMode: false })
        : Promise.resolve(undefined),
    );
    phase = loadedPhase;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    renderBubble();

    await act(async () =>
      container?.querySelector<HTMLButtonElement>(".bubble-title")?.click(),
    );
    expect(container?.querySelector(".agent-bubble")?.className).toContain(
      "expanded",
    );
    expect(latestGeometry()).toMatchObject({ width: 380 });

    const textarea = container?.querySelector<HTMLTextAreaElement>(
      ".bubble-composer textarea",
    );
    if (textarea === null || textarea === undefined)
      throw new Error("Missing bubble composer");
    change(textarea, "rascunho preservado");
    await act(async () =>
      container?.querySelector<HTMLButtonElement>(".bubble-minimize")?.click(),
    );
    expect(container?.querySelector(".agent-bubble")?.className).toContain(
      "minimized",
    );
    expect(latestGeometry()).toMatchObject({ width: 196 });
    expect(
      container?.querySelector(".bubble-minimized-preview"),
    ).not.toBeNull();
    expect(container?.textContent).toContain("Astra");

    await act(async () =>
      container?.querySelector<HTMLButtonElement>(".bubble-restore")?.click(),
    );
    expect(container?.querySelector(".agent-bubble")?.className).toContain(
      "expanded",
    );
    expect(latestGeometry()).toMatchObject({ width: 380 });
    expect(
      container?.querySelector<HTMLTextAreaElement>(".bubble-composer textarea")
        ?.value,
    ).toBe("rascunho preservado");
  });

  it("reopens as a compact bubble after repeated close and restore cycles", async () => {
    vi.stubGlobal("ResizeObserver", TestResizeObserver);
    invoke.mockImplementation((command: string) =>
      command === "get_app_snapshot"
        ? Promise.resolve({ safeMode: false })
        : Promise.resolve(undefined),
    );
    phase = loadedPhase;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    renderBubble();

    await act(async () =>
      container?.querySelector<HTMLButtonElement>(".bubble-title")?.click(),
    );
    await act(async () =>
      container?.querySelector<HTMLButtonElement>(".bubble-minimize")?.click(),
    );
    await act(async () =>
      container?.querySelector<HTMLButtonElement>(".bubble-close")?.click(),
    );
    expect(container.querySelector(".agent-bubble")?.className).toContain(
      "compact",
    );
    expect(invoke).toHaveBeenCalledWith("set_overlay_bubble_visible", {
      agentId: "agent",
      visible: false,
    });

    await act(async () => emitLifecycle(BUBBLE_NATIVE_OPEN_EVENT, "agent"));
    expect(container.querySelector(".agent-bubble")?.className).toContain(
      "compact",
    );
    expect(latestGeometry()).toMatchObject({ width: 380 });
    await act(async () =>
      container?.querySelector<HTMLButtonElement>(".bubble-title")?.click(),
    );
    expect(container.querySelector(".agent-bubble")?.className).toContain(
      "expanded",
    );
  });

  it("opens the same agent conversation in the full workspace", async () => {
    vi.stubGlobal("ResizeObserver", TestResizeObserver);
    invoke.mockImplementation((command: string) =>
      command === "get_app_snapshot"
        ? Promise.resolve({ safeMode: false })
        : Promise.resolve(undefined),
    );
    phase = loadedPhase;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    renderBubble();
    await act(async () =>
      container?.querySelector<HTMLButtonElement>(".bubble-title")?.click(),
    );
    await act(async () =>
      container?.querySelector<HTMLButtonElement>(".bubble-open-chat")?.click(),
    );
    expect(invoke).toHaveBeenCalledWith("open_agent_conversations", {
      agentId: "agent",
      conversationId: "conversation",
    });
  });

  it("returns to compact presentation and clears regions after native close", async () => {
    vi.stubGlobal("ResizeObserver", TestResizeObserver);
    invoke.mockImplementation((command: string) =>
      command === "get_app_snapshot"
        ? Promise.resolve({ safeMode: false })
        : Promise.resolve(undefined),
    );
    phase = loadedPhase;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    renderBubble();

    await act(async () =>
      container?.querySelector<HTMLButtonElement>(".bubble-title")?.click(),
    );
    expect(container.querySelector(".agent-bubble")?.className).toContain(
      "expanded",
    );
    await act(async () => emitLifecycle(BUBBLE_NATIVE_CLOSE_EVENT, "agent"));
    expect(container.querySelector(".agent-bubble")?.className).toContain(
      "compact",
    );
    expect(invoke).toHaveBeenCalledWith("set_overlay_interactive_regions", {
      agentId: "agent",
      regions: [],
    });

    await act(async () => emitLifecycle(BUBBLE_NATIVE_OPEN_EVENT, "agent"));
    expect(container.querySelector(".agent-bubble")?.className).toContain(
      "compact",
    );
    expect(invoke).toHaveBeenCalledWith("set_overlay_bubble_geometry", {
      agentId: "agent",
      width: 380,
      height: 128,
    });
  });

  it("isolates native close and open lifecycle events between Astra and Luma", async () => {
    vi.stubGlobal("ResizeObserver", TestResizeObserver);
    invoke.mockImplementation((command: string) =>
      command === "get_app_snapshot"
        ? Promise.resolve({ safeMode: false })
        : Promise.resolve(undefined),
    );
    phasesByAgent.set("astra", astraPhase);
    phasesByAgent.set("luma", lumaPhase);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    renderBubbles(["astra", "luma"]);
    await flushListenerRegistration();

    const bubbles = Array.from(
      container.querySelectorAll<HTMLElement>(".agent-bubble"),
    );
    expect(bubbles).toHaveLength(2);
    bubbles.forEach((bubble) => {
      bubble.getBoundingClientRect = () =>
        ({
          x: 8,
          y: 8,
          top: 8,
          right: 372,
          bottom: 120,
          left: 8,
          width: 364,
          height: 112,
          toJSON: () => ({}),
        }) as DOMRect;
    });

    await act(async () => {
      bubbles[0]?.querySelector<HTMLButtonElement>(".bubble-title")?.click();
      bubbles[1]?.querySelector<HTMLButtonElement>(".bubble-title")?.click();
    });
    expect(bubbles[0]?.className).toContain("expanded");
    expect(bubbles[1]?.className).toContain("expanded");

    await act(async () => emitLifecycle(BUBBLE_NATIVE_CLOSE_EVENT, "astra"));
    expect(bubbles[0]?.className).toContain("compact");
    expect(bubbles[1]?.className).toContain("expanded");
    expect(invoke).toHaveBeenCalledWith("set_overlay_interactive_regions", {
      agentId: "luma",
      regions: [expect.any(Object)],
    });

    await act(async () => emitLifecycle(BUBBLE_NATIVE_OPEN_EVENT, "astra"));
    expect(bubbles[0]?.className).toContain("compact");
    expect(
      [...invoke.mock.calls].reverse().find(
        ([command, args]) =>
          command === "set_overlay_bubble_geometry" &&
          (args as { agentId?: string } | undefined)?.agentId === "astra",
      )?.[1],
    ).toMatchObject({ agentId: "astra", width: 380, height: 128 });

    await act(async () =>
      bubbles[0]?.querySelector<HTMLButtonElement>(".bubble-title")?.click(),
    );
    expect(bubbles[0]?.className).toContain("expanded");

    await act(async () => emitLifecycle(BUBBLE_NATIVE_CLOSE_EVENT, "luma"));
    expect(bubbles[0]?.className).toContain("expanded");
    expect(bubbles[1]?.className).toContain("compact");

    await act(async () => emitLifecycle(BUBBLE_NATIVE_OPEN_EVENT, "luma"));
    expect(bubbles[1]?.className).toContain("compact");
    expect(
      [...invoke.mock.calls].reverse().find(
        ([command, args]) =>
          command === "set_overlay_bubble_geometry" &&
          (args as { agentId?: string } | undefined)?.agentId === "luma",
      )?.[1],
    ).toMatchObject({ agentId: "luma", width: 380, height: 128 });
  });
});
