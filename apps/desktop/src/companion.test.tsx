// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CompanionControls } from "./App";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("CompanionControls", () => {
  let root: Root | undefined;
  let container: HTMLDivElement | undefined;

  afterEach(() => {
    act(() => root?.unmount());
    container?.remove();
    invoke.mockReset();
  });

  it("renders the local-only metadata boundary and loads read-only records", async () => {
    invoke.mockResolvedValue([]);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);

    await act(async () =>
      root?.render(
        <CompanionControls
          agentId="agt_astra_provisional"
          temporaryChat={false}
          safeMode={false}
        />,
      ),
    );

    expect(container.textContent).toContain("Companion Android local");
    expect(container.textContent).toContain("sem rede, listener, relay");
    expect(container.textContent).toContain(
      "bytes de mídia nunca são persistidos",
    );
    expect(container.textContent).toContain("Protocolo v1");
    expect(invoke).toHaveBeenCalledWith("list_companion_devices", {
      agentId: "agt_astra_provisional",
    });
    expect(invoke).toHaveBeenCalledWith("list_companion_audit", {
      agentId: "agt_astra_provisional",
    });
    expect(invoke).toHaveBeenCalledWith("list_companion_history", {
      agentId: "agt_astra_provisional",
    });
  });

  it("fails closed for temporary chat and safe mode while keeping labels visible", async () => {
    invoke.mockResolvedValue([]);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);

    await act(async () =>
      root?.render(
        <CompanionControls
          agentId="agt_astra_provisional"
          temporaryChat
          safeMode
        />,
      ),
    );

    expect(container.textContent).toContain(
      "Conversa temporária: alterações do companion bloqueadas",
    );
    expect(container.textContent).toContain(
      "Modo seguro: alterações do companion bloqueadas",
    );
    const pairingButton = [...container.querySelectorAll("button")].find(
      (button) => button.textContent === "Solicitar pareamento fixture",
    );
    expect(pairingButton?.disabled).toBe(true);
    expect(invoke).not.toHaveBeenCalledWith(
      "start_companion_pairing",
      expect.anything(),
    );
  });
});
