// @vitest-environment jsdom
import { act, createElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import { describe, expect, it } from "vitest";
import { afterEach, vi } from "vitest";
import AgentSprite, { pixelOverlays } from "./AgentSprite";

const { invoke, listen } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(() => Promise.resolve(() => undefined)),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

let root: Root | undefined;
let container: HTMLDivElement | undefined;

afterEach(() => {
  if (root !== undefined) act(() => root?.unmount());
  container?.remove();
  root = undefined;
  container = undefined;
  vi.clearAllMocks();
});

describe("pixelOverlays", () => {
  it("uses visible valid pixels and rejects malformed source", () => {
    expect(
      pixelOverlays(
        '{"layers":[{"pixels":[[1,2,"#fff"],[2,3,"#11223380"],[3,4,"#12345"],[64,0,"#000"]]},{"visible":false,"pixels":[[3,4,"#000"]]}]}',
      ),
    ).toEqual([
      { x: 1, y: 2, color: "#fff" },
      { x: 2, y: 3, color: "#11223380" },
    ]);
    expect(pixelOverlays("not json")).toEqual([]);
  });

  it("renders custom alpha pixels as SVG fill attributes", async () => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    invoke.mockResolvedValue('{"layers":[{"pixels":[[4,5,"#11223380"]]}]}');
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);

    await act(async () => {
      root?.render(
        createElement(AgentSprite, {
          agentId: "agent",
          spriteKey: "astra",
          name: "Astra",
        }),
      );
      await Promise.resolve();
    });

    const rect = container.querySelector(".agent-sprite-custom rect");
    expect(rect?.getAttribute("x")).toBe("4");
    expect(rect?.getAttribute("y")).toBe("5");
    expect(rect?.getAttribute("fill")).toBe("#11223380");
    expect(rect?.getAttribute("color")).toBeNull();
  });
});
