// @vitest-environment jsdom
import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  LocalCapabilitiesSurface,
  PixelDocumentEditor,
  ScreenVisionControls,
} from "./App";
import { FilePicker } from "./shared-controls";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const source = JSON.stringify({
  layers: [
    {
      id: "body",
      name: "Body",
      visible: true,
      locked: false,
      pixels: [[1, 1, "#111111"]],
    },
  ],
  attachmentPoints: {},
});

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

let root: Root | undefined;
let container: HTMLDivElement | undefined;

function mount(element: ReactNode) {
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  act(() => root?.render(element));
}

afterEach(() => {
  if (root !== undefined) act(() => root?.unmount());
  container?.remove();
  root = undefined;
  container = undefined;
  vi.restoreAllMocks();
  vi.clearAllMocks();
});

beforeEach(() => {
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(
    () =>
      ({
        fillStyle: "",
        fillRect: vi.fn(),
        strokeStyle: "",
        lineWidth: 0,
        strokeRect: vi.fn(),
      }) as unknown as CanvasRenderingContext2D,
  );
});

describe("Phase D Pixel and local resources", () => {
  it("organizes the Pixel toolbar and keeps import as a styled PNG action", async () => {
    invoke.mockResolvedValue(source);
    mount(<PixelDocumentEditor agentId="agent" />);
    await act(async () => await Promise.resolve());

    expect(
      [
        ...(container?.querySelectorAll<HTMLElement>("[data-tool-group]") ??
          []),
      ].map((group) => group.dataset.toolGroup),
    ).toEqual(["drawing", "transform-view", "history", "file"]);
    expect(
      container?.querySelector('[data-tool-group="drawing"]')?.textContent,
    ).toContain("Lápis");
    expect(
      container?.querySelector('[data-tool-group="transform-view"]')
        ?.textContent,
    ).toContain("Espelhar");
    expect(
      container?.querySelector('[data-tool-group="history"]')?.textContent,
    ).toContain("Desfazer");
    expect(
      container?.querySelector('[data-tool-group="file"]')?.textContent,
    ).toContain("Salvar arte");
    expect(
      container
        ?.querySelector('[data-aip-file-picker="pixel-import"] input')
        ?.classList.contains("visually-hidden"),
    ).toBe(true);
    expect(container?.querySelector("#pixel-zoom-trigger")).not.toBeNull();
  });

  it("shows a clear filename and exposes an accessible file action", () => {
    const onChange = vi.fn();
    mount(
      <FilePicker
        id="test-file"
        label="Importar PNG"
        accept="image/png"
        buttonLabel="Escolher PNG"
        description="PNG de até 1 MB."
        onChange={onChange}
      />,
    );
    const input =
      container?.querySelector<HTMLInputElement>("#test-file-input");
    const action = container?.querySelector<HTMLButtonElement>(
      ".aip-file-picker-action",
    );
    if (input === null || input === undefined) throw new Error("Missing input");
    if (action === null || action === undefined)
      throw new Error("Missing action");
    const click = vi.spyOn(input, "click");
    act(() => action.click());
    expect(click).toHaveBeenCalledOnce();

    const file = new File(["png"], "sprite.png", { type: "image/png" });
    Object.defineProperty(input, "files", {
      configurable: true,
      value: [file],
    });
    act(() => input.dispatchEvent(new Event("change", { bubbles: true })));
    expect(onChange).toHaveBeenCalledWith(file);
    expect(container?.textContent).toContain("sprite.png");
    expect(action.getAttribute("aria-describedby")).toContain(
      "test-file-filename",
    );
    expect(input.value).toBe("");
  });

  it("uses AipSelect for zoom and keeps Screen Vision fields grouped", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_screen_vision_fixtures")
        return Promise.resolve([fixture]);
      if (command === "list_screen_vision_sessions") return Promise.resolve([]);
      if (command === "list_screen_vision_jobs") return Promise.resolve([]);
      if (command === "list_screen_vision_audit") return Promise.resolve([]);
      return Promise.resolve(null);
    });
    mount(
      <ScreenVisionControls
        agentId="agent"
        temporaryChat={false}
        safeMode={false}
      />,
    );
    await act(async () => await Promise.resolve());

    expect(container?.querySelectorAll(".screen-vision-fieldset")).toHaveLength(
      2,
    );
    expect(container?.querySelectorAll(".screen-vision-checkbox")).toHaveLength(
      5,
    );
    expect(
      container?.querySelectorAll(
        ".screen-vision-number-fields input[type=number]",
      ),
    ).toHaveLength(2);
    expect(
      container?.querySelectorAll(".screen-vision-controls .aip-select"),
    ).toHaveLength(3);
    expect(
      container?.querySelectorAll(".readable-helper").length,
    ).toBeGreaterThanOrEqual(3);
  });

  it("keeps local resources as spaced status and capability blocks", async () => {
    invoke.mockImplementation((command: string) => {
      if (
        command.startsWith("list_") ||
        command === "get_local_capability_snapshot"
      ) {
        return Promise.resolve([]);
      }
      return Promise.resolve(undefined);
    });
    mount(
      <LocalCapabilitiesSurface
        agentId="agent"
        snapshot={null}
        safeMode={false}
        temporaryChat={false}
      />,
    );
    await act(async () => await Promise.resolve());

    expect(
      container?.querySelector(".local-capabilities-surface"),
    ).not.toBeNull();
    expect(container?.querySelector(".local-status-center")).not.toBeNull();
    expect(container?.querySelector(".local-capability-panels")).not.toBeNull();
    expect(
      container?.querySelector(".local-capability-note.readable-helper"),
    ).not.toBeNull();
  });
});
