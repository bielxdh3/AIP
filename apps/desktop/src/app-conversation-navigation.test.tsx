// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { PhaseOneState } from "@aip/contracts";
import { OPEN_AGENT_CONVERSATIONS_EVENT } from "./agent-navigation";
import App from "./App";

type EventCallback = (event: { payload: unknown }) => void;

const { invoke, listen, usePhaseOne } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  usePhaseOne: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));
vi.mock("./use-phase-one", () => ({ usePhaseOne }));

const conversations = [
  {
    id: "main",
    agentId: "agent",
    title: "Conversa inicial",
    modelOverrideRef: null,
    isPinned: true,
  },
  {
    id: "extra",
    agentId: "agent",
    title: "Conversa secundária",
    modelOverrideRef: null,
    isPinned: false,
  },
];

const phase = {
  agent: { id: "agent", name: "Astra" },
  conversation: conversations[0],
  messages: [],
  branches: [],
  turnVariants: [],
  activeBranchId: "main",
  provider: {
    state: "available",
    detailCode: "provider_available",
    models: [],
    refreshedAt: 1,
  },
  selectedModelRef: null,
  defaultModelRef: null,
  modelOverrideRef: null,
  effectiveModelSource: "agent_default",
  selectedModelAvailable: false,
  keepAliveMinutes: 5,
  queue: [],
  canSend: false,
  sendBlockedCode: "provider_empty",
} as unknown as PhaseOneState;
let currentPhase = phase;

const snapshot = {
  appVersion: "0.2.0",
  buildSha: "test",
  buildTimestamp: "test",
  runtimePackagingMode: "managed",
  safeMode: false,
  databaseReady: true,
  migrationVersion: 1,
  runtime: { state: "ready", protocolVersion: 1, detailCode: "ready" },
  agents: [
    {
      id: "agent",
      name: "Astra",
      profileKey: "owner",
      spriteKey: "astra",
      position: { x: 0, y: 0 },
      birthday: "2000-01-01",
      fictiveAge: 26,
      ageCategory: "adult",
      species: "humana",
      pronouns: "ela/dela",
      personalitySummary: "Resumo",
      traitsJson: "{}",
      appearancePreset: "default",
    },
  ],
  onboardingRequired: false,
};

describe("App conversation navigation integration", () => {
  let root: Root | undefined;
  let container: HTMLDivElement | undefined;
  let listeners: Map<string, EventCallback[]>;

  afterEach(() => {
    if (root !== undefined) act(() => root?.unmount());
    container?.remove();
    root = undefined;
    container = undefined;
    listeners.clear();
    vi.clearAllMocks();
  });

  async function renderApp() {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    listeners = new Map();
    listen.mockImplementation((event: string, callback: EventCallback) => {
      const callbacks = listeners.get(event) ?? [];
      callbacks.push(callback);
      listeners.set(event, callbacks);
      return Promise.resolve(vi.fn());
    });
    invoke.mockImplementation((command: string) => {
      if (command === "get_app_snapshot") return Promise.resolve(snapshot);
      if (command === "list_agent_conversations")
        return Promise.resolve(conversations);
      if (command === "load_pixel_document") return Promise.resolve("{}");
      return Promise.resolve(undefined);
    });
    currentPhase = phase;
    usePhaseOne.mockImplementation(() => ({
      phase: currentPhase,
      error: false,
      load: vi.fn(),
    }));
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () => {
      root?.render(<App />);
      await Promise.resolve();
      await Promise.resolve();
      await Promise.resolve();
    });
  }

  function activeRow() {
    return container?.querySelector<HTMLElement>(
      '.conversation-list-item[data-active="true"]',
    );
  }

  it("renders the restored PhaseOneState conversation as the active row", async () => {
    await renderApp();

    expect(
      activeRow()?.querySelector(".conversation-list-select")?.textContent,
    ).toContain("Conversa inicial");
    expect(
      activeRow()
        ?.querySelector(".conversation-list-select")
        ?.getAttribute("aria-current"),
    ).toBe("page");
  });

  it("moves selection only after successful activation and preserves it on failure", async () => {
    await renderApp();
    const select = (title: string) =>
      Array.from(
        container?.querySelectorAll<HTMLButtonElement>(
          ".conversation-list-select",
        ) ?? [],
      ).find((button) => button.textContent?.includes(title));

    invoke.mockImplementation(
      (command: string, args?: { conversationId?: string }) => {
        if (command === "set_active_agent_conversation") {
          const selected = conversations.find(
            (conversation) => conversation.id === args?.conversationId,
          );
          if (selected !== undefined)
            currentPhase = { ...currentPhase, conversation: selected };
          return Promise.resolve();
        }
        return command === "list_agent_conversations"
          ? Promise.resolve(conversations)
          : Promise.resolve(undefined);
      },
    );
    await act(async () => {
      select("secundária")?.click();
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(
      activeRow()?.querySelector(".conversation-list-select")?.textContent,
    ).toContain("Conversa secundária");

    invoke.mockImplementation((command: string) => {
      if (command === "set_active_agent_conversation") {
        return Promise.reject(new Error("activation_failed"));
      }
      if (command === "list_agent_conversations")
        return Promise.resolve(conversations);
      return Promise.resolve(undefined);
    });
    await act(async () => {
      select("inicial")?.click();
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(
      activeRow()?.querySelector(".conversation-list-select")?.textContent,
    ).toContain("Conversa secundária");
  });

  it("uses the overlay conversation and clears selection when starting a draft", async () => {
    await renderApp();
    const open = listeners.get(OPEN_AGENT_CONVERSATIONS_EVENT)?.[0];
    if (open === undefined) throw new Error("Missing conversation listener");

    await act(async () =>
      open({
        payload: { agentId: "agent", conversationId: "extra" },
      }),
    );
    expect(
      activeRow()?.querySelector(".conversation-list-select")?.textContent,
    ).toContain("Conversa secundária");

    const create = container?.querySelector<HTMLButtonElement>(
      ".conversation-list-create",
    );
    if (create === undefined || create === null)
      throw new Error("Missing draft control");
    await act(async () => create.click());
    expect(activeRow()).toBeNull();
    expect(container?.textContent).toContain("Rascunho local");
  });
});
