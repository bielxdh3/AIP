// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AgentSimulatedState, ProvisionalAgent } from "@aip/contracts";
import { AgentStateControls, SidebarNavigation } from "./App";

const { invoke, listen } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

const agent = (id: string, name: string, spriteKey: "astra" | "luma") =>
  ({
    id,
    name,
    profileKey: "owner",
    spriteKey,
    position: { x: 0, y: 0 },
    birthday: "2020-01-02",
    fictiveAge: 28,
    ageCategory: "adult",
    species: "agent",
    pronouns: "they/them",
    personalitySummary: "Descrição",
    traitsJson: "{}",
    appearancePreset: spriteKey,
  }) satisfies ProvisionalAgent;

const state: AgentSimulatedState = {
  agentId: "astra",
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

describe("Phase D sidebar and state explanations", () => {
  let root: Root | undefined;
  let container: HTMLDivElement | undefined;

  afterEach(() => {
    if (root !== undefined) act(() => root?.unmount());
    container?.remove();
    root = undefined;
    container = undefined;
    vi.clearAllMocks();
  });

  it("keeps agent navigation compact and leaves global settings behind the brand", () => {
    invoke.mockResolvedValue("");
    listen.mockResolvedValue(vi.fn());
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    const onWorkspace = vi.fn();
    act(() =>
      root?.render(
        <SidebarNavigation
          agents={[
            agent("astra", "Astra", "astra"),
            agent("luma", "Luma", "luma"),
          ]}
          activeAgentId="astra"
          workspace="chat"
          onSelectAgent={vi.fn()}
          onWorkspace={onWorkspace}
          onProfile={vi.fn()}
        />,
      ),
    );

    expect(container.querySelector(".sidebar-navigation")).not.toBeNull();
    expect(container.querySelectorAll(".sidebar-section")).toHaveLength(2);
    expect(container.querySelector(".sidebar-agents")?.tagName).toBe("DETAILS");
    expect(container.querySelector(".sidebar-secondary")?.tagName).toBe(
      "DETAILS",
    );
    expect(
      container.querySelectorAll(".sidebar-agents .agent-tab"),
    ).toHaveLength(2);
    expect(container.querySelector(".sidebar-secondary > nav")).not.toBeNull();
    expect(
      container.querySelector(".sidebar-secondary")?.textContent,
    ).not.toContain("Aplicativo");
    expect(
      container.querySelector(".sidebar-secondary")?.textContent,
    ).not.toContain("Recursos locais");
    expect(
      container.querySelector(".sidebar-secondary")?.textContent,
    ).not.toContain("Configurações");

    const agentsSection =
      container.querySelector<HTMLDetailsElement>(".sidebar-agents");
    if (agentsSection === null) throw new Error("Missing agents section");
    expect(agentsSection.open).toBe(true);
    act(() => agentsSection.querySelector<HTMLElement>("summary")?.click());
    expect(agentsSection.open).toBe(false);
    act(() => agentsSection.querySelector<HTMLElement>("summary")?.click());
    expect(agentsSection.open).toBe(true);

    const stateButton = Array.from(
      container.querySelectorAll<HTMLButtonElement>(
        ".sidebar-secondary > nav button",
      ),
    ).find((button) => button.textContent === "Estado");
    if (stateButton === undefined) throw new Error("Missing state navigation");
    act(() => stateButton.click());
    expect(onWorkspace).toHaveBeenCalledWith("state");
  });

  it("routes an agent selection to the selected agent context", () => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    const onSelectAgent = vi.fn();
    act(() =>
      root?.render(
        <SidebarNavigation
          agents={[
            agent("astra", "Astra", "astra"),
            agent("luma", "Luma", "luma"),
          ]}
          activeAgentId="astra"
          workspace="chat"
          onSelectAgent={onSelectAgent}
          onWorkspace={vi.fn()}
          onProfile={vi.fn()}
        />,
      ),
    );
    const luma = Array.from(
      container.querySelectorAll<HTMLButtonElement>(".agent-tab"),
    ).find((button) => button.textContent?.includes("Luma"));
    if (luma === undefined) throw new Error("Missing Luma agent tab");
    act(() => luma.click());
    expect(onSelectAgent).toHaveBeenCalledWith("luma");
  });

  it("gives every simulated state value and action a readable nearby helper", async () => {
    invoke.mockResolvedValue(state);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () => root?.render(<AgentStateControls agentId="astra" />));

    expect(
      container.querySelectorAll(".state-mode-explanations [data-state-mode]"),
    ).toHaveLength(3);
    expect(container.textContent).toContain("Normal");
    expect(container.textContent).toContain("Sem voz");
    expect(container.textContent).toContain("Silencioso");
    expect(container.querySelectorAll(".state-metric")).toHaveLength(3);
    expect(container.querySelectorAll(".state-action")).toHaveLength(2);
    expect(
      container.querySelectorAll(".readable-helper").length,
    ).toBeGreaterThanOrEqual(9);
    expect(container.querySelector(".state-mode-options")).not.toBeNull();
    expect(
      container.querySelector<HTMLButtonElement>(
        '.state-mode-button[aria-pressed="true"]',
      ),
    ).not.toBeNull();
    expect(container.textContent).toContain("sem remover a suspensão");
  });
});
