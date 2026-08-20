// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ScreenVisionControls } from "./App";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const fixture = {
  fixtureId: "fixture:screen/monitor-1/desktop-neutral-v1",
  monitorId: "monitor-1",
  displayName: "Monitor sintético 1",
  width: 1280,
  height: 720,
  scale: 1,
  synthetic: true,
  metadataOnly: true,
};

const session = {
  id: "session-1",
  agentId: "agt_astra_provisional",
  ownerUserId: "usr_owner_local",
  monitorId: fixture.monitorId,
  fixtureId: fixture.fixtureId,
  status: "active",
  permissions: ["capture_fixture", "analyze_fixture"],
  privacy: {
    excludeSensitiveContent: true,
    redactionRules: [{ kind: "exclude_sensitive_regions", enabled: true }],
  },
  maxJobs: 4,
  maxDurationMs: 5_000,
  createdAt: 1,
  updatedAt: 1,
  closedAt: null,
};

describe("ScreenVisionControls", () => {
  let root: Root | undefined;
  let container: HTMLDivElement | undefined;

  afterEach(() => {
    act(() => root?.unmount());
    container?.remove();
    invoke.mockReset();
  });

  it("renders Portuguese metadata-only controls and sends Owner-scoped session data", async () => {
    let sessions: unknown[] = [];
    invoke.mockImplementation((command: string) => {
      if (command === "list_screen_vision_fixtures")
        return Promise.resolve([fixture]);
      if (command === "list_screen_vision_sessions")
        return Promise.resolve(sessions);
      if (command === "list_screen_vision_jobs") return Promise.resolve([]);
      if (command === "list_screen_vision_audit") return Promise.resolve([]);
      if (command === "create_screen_vision_session") {
        sessions = [session];
        return Promise.resolve(session);
      }
      return Promise.resolve(null);
    });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);

    await act(async () =>
      root?.render(
        <ScreenVisionControls
          agentId="agt_astra_provisional"
          temporaryChat={false}
          safeMode={false}
        />,
      ),
    );
    expect(container.textContent).toContain("Visão de tela (fixture)");
    expect(container.textContent).toContain("não há captura do Windows");
    expect(container.textContent).toContain("Monitor sintético 1");

    const createButton = [...container.querySelectorAll("button")].find(
      (button) => button.textContent === "Criar sessão limitada",
    );
    expect(createButton).not.toBeUndefined();
    await act(async () => createButton?.click());
    expect(invoke).toHaveBeenCalledWith(
      "create_screen_vision_session",
      expect.objectContaining({
        agentId: "agt_astra_provisional",
        ownerUserId: "usr_owner_local",
        monitorId: "monitor-1",
        fixtureId: fixture.fixtureId,
        permissions: ["capture_fixture", "analyze_fixture"],
        privacy: {
          excludeSensitiveContent: true,
          redactionRules: [
            { kind: "exclude_sensitive_regions", enabled: true },
          ],
        },
        temporaryChat: false,
      }),
    );
  });

  it("keeps read-only history visible while disabling mutation controls in temporary chat and safe mode", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_screen_vision_fixtures")
        return Promise.resolve([fixture]);
      return Promise.resolve([]);
    });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);

    await act(async () =>
      root?.render(
        <ScreenVisionControls
          agentId="agt_astra_provisional"
          temporaryChat
          safeMode
        />,
      ),
    );
    expect(container.textContent).toContain(
      "Conversa temporária: alterações de visão bloqueadas.",
    );
    expect(container.textContent).toContain(
      "Modo seguro: alterações de visão bloqueadas.",
    );
    const createButton = [...container.querySelectorAll("button")].find(
      (button) => button.textContent === "Criar sessão limitada",
    );
    expect(createButton?.disabled).toBe(true);
    expect(invoke).toHaveBeenCalledWith("list_screen_vision_fixtures");
    expect(invoke).toHaveBeenCalledWith("list_screen_vision_sessions", {
      agentId: "agt_astra_provisional",
    });
    expect(invoke).not.toHaveBeenCalledWith(
      "create_screen_vision_session",
      expect.anything(),
    );
  });
});
