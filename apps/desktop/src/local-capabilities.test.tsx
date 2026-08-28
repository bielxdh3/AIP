// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LocalCapabilityStatusCenter } from "./App";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("LocalCapabilityStatusCenter", () => {
  let root: Root | undefined;
  let container: HTMLDivElement | undefined;

  afterEach(() => {
    act(() => root?.unmount());
    container?.remove();
    invoke.mockReset();
  });

  it("opens and focuses the details destination for capability cards", async () => {
    invoke.mockResolvedValue(undefined);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);

    await act(async () =>
      root?.render(
        <LocalCapabilityStatusCenter
          agentId="agt_luma_provisional"
          snapshot={null}
          safeMode={false}
          temporaryChat={false}
        />,
      ),
    );

    const runtimeCard = [
      ...container.querySelectorAll<HTMLAnchorElement>(".local-status-card"),
    ].find((card) => card.textContent?.includes("Runtime"));
    const runtimePanel = container.querySelector<HTMLDetailsElement>(
      "#local-capability-runtime",
    );
    expect(runtimeCard?.getAttribute("aria-controls")).toBe(
      "local-capability-runtime",
    );
    expect(runtimePanel?.open).toBe(false);

    await act(async () => runtimeCard?.click());
    expect(runtimePanel?.open).toBe(true);
    expect(document.activeElement).toBe(runtimePanel?.querySelector("summary"));
    expect(container.textContent).toContain("Configuração:");

    const screenPanel = document.createElement("details");
    screenPanel.id = "local-capability-screen-vision";
    const screenSummary = document.createElement("summary");
    screenSummary.textContent = "Visão de tela";
    screenPanel.append(screenSummary);
    container.append(screenPanel);
    const visualCard = [
      ...container.querySelectorAll<HTMLAnchorElement>(".local-status-card"),
    ].find((card) => card.textContent?.includes("Provedor visual"));

    await act(async () => visualCard?.click());
    expect(screenPanel.open).toBe(true);
    expect(document.activeElement).toBe(screenSummary);
  });
});
