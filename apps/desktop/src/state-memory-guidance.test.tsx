// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AgentSimulatedState } from "@aip/contracts";
import { AgentStateControls, MemoryWorkspace } from "./App";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const state: AgentSimulatedState = {
  agentId: "agent",
  sleep: 20,
  energy: 80,
  mood: 70,
  focus: 70,
  curiosity: 70,
  socialFatigue: 20,
  mode: "normal",
  suspended: false,
  wakeNowUntil: null,
  lastSimulatedAt: 1,
};

async function chooseOption(container: HTMLElement, id: string, value: string) {
  const trigger = container.querySelector<HTMLButtonElement>(`#${id}-trigger`);
  if (trigger === null) throw new Error(`Missing ${id} trigger`);
  await act(async () => trigger.click());
  const option = document.body.querySelector<HTMLElement>(
    `.aip-select-option[data-value="${value}"]`,
  );
  if (option === null) throw new Error(`Missing ${value} option`);
  await act(async () => option.click());
}

describe("State and memory guidance", () => {
  let root: Root | undefined;
  let container: HTMLDivElement | undefined;

  afterEach(() => {
    if (root !== undefined) act(() => root?.unmount());
    container?.remove();
    root = undefined;
    container = undefined;
    vi.clearAllMocks();
  });

  it("explains simulated state and suspension actions", async () => {
    invoke.mockResolvedValue(state);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () => root?.render(<AgentStateControls agentId="agent" />));
    expect(container.textContent).toContain("valores fictícios simulados");
    expect(container.textContent).toContain("silencia a voz sintetizada");
    expect(container.textContent).toContain(
      "alterações de configurações de voz",
    );
    expect(container.textContent).toContain("não são medições de saúde");
    expect(container.textContent).toContain("pausa o avanço simulado");
    expect(container.textContent).toContain("sem remover a suspensão");
    expect(container.textContent).toContain("Acordar agora");
  });

  it("shows contextual memory count, category help, and proposal semantics", async () => {
    invoke.mockResolvedValue([]);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () => root?.render(<MemoryWorkspace agentId="agent" />));
    expect(container.textContent).toContain("0 memórias ativas");
    expect(container.textContent).toContain("pertencem somente a este agente");
    await chooseOption(container, "memory-status", "archived");
    expect(invoke).toHaveBeenLastCalledWith(
      "search_agent_memories",
      expect.objectContaining({ status: "archived" }),
    );
    await chooseOption(container, "memory-category", "rule");
    expect(container.textContent).toContain("Uma regra explícita");
    expect(container.textContent).toContain("candidata pendente");
  });
});
