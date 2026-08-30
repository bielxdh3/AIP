// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PixelDocumentEditor } from "./App";

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
    {
      id: "hair",
      name: "Hair",
      visible: true,
      locked: false,
      pixels: [[2, 2, "#222222"]],
    },
  ],
  attachmentPoints: {},
  semanticParts: {},
});

type ParsedSource = {
  layers: Array<{ pixels: [number, number, string][] }>;
};

let root: Root | undefined;
let container: HTMLDivElement | undefined;
let context: {
  fillStyle: string;
  fillRect: ReturnType<typeof vi.fn>;
  strokeStyle: string;
  lineWidth: number;
  strokeRect: ReturnType<typeof vi.fn>;
};

function pointerEvent(
  type: string,
  x: number,
  y: number,
  buttons = 1,
  pointerId = 1,
) {
  const event = new Event(type, { bubbles: true });
  Object.defineProperties(event, {
    buttons: { value: buttons },
    offsetX: { value: x * 4 },
    offsetY: { value: y * 4 },
    pointerId: { value: pointerId },
  });
  return event;
}

function button(label: string): HTMLButtonElement {
  const match = Array.from(container?.querySelectorAll("button") ?? []).find(
    (candidate) => candidate.textContent?.trim() === label,
  );
  if (!(match instanceof HTMLButtonElement))
    throw new Error(`Missing button: ${label}`);
  return match;
}

async function renderEditor() {
  invoke.mockResolvedValue(source);
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  await act(async () => {
    root?.render(<PixelDocumentEditor agentId="agent" />);
    await Promise.resolve();
  });
  const canvas = container.querySelector("canvas");
  if (!(canvas instanceof HTMLCanvasElement)) throw new Error("Missing canvas");
  Object.defineProperty(canvas, "clientWidth", {
    configurable: true,
    value: 256,
  });
  Object.defineProperty(canvas, "clientHeight", {
    configurable: true,
    value: 256,
  });
  canvas.setPointerCapture = vi.fn();
  canvas.releasePointerCapture = vi.fn();
  return canvas;
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
  context = {
    fillStyle: "",
    fillRect: vi.fn(),
    strokeStyle: "",
    lineWidth: 0,
    strokeRect: vi.fn(),
  };
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(
    () => context as unknown as CanvasRenderingContext2D,
  );
});

describe("PixelDocumentEditor regressions", () => {
  it("requires explicit confirmation before deleting a layer", async () => {
    await renderEditor();
    const textarea = container?.querySelector<HTMLTextAreaElement>(
      '[aria-label="Documento de pixel art"]',
    );
    if (textarea === null || textarea === undefined)
      throw new Error("Missing source editor");

    await act(async () => button("Excluir").click());
    expect(container?.querySelector('[role="dialog"]')).not.toBeNull();
    expect(JSON.parse(textarea.value).layers).toHaveLength(2);

    await act(async () => button("Cancelar").click());
    expect(container?.querySelector('[role="dialog"]')).toBeNull();
    expect(JSON.parse(textarea.value).layers).toHaveLength(2);

    await act(async () => button("Excluir").click());
    await act(async () => button("Excluir camada").click());
    expect(JSON.parse(textarea.value).layers).toHaveLength(1);
  });

  it("keeps Pencil edits isolated and supports undo/redo", async () => {
    const canvas = await renderEditor();

    await act(async () =>
      canvas.dispatchEvent(pointerEvent("pointerdown", 3, 3)),
    );
    await act(async () =>
      canvas.dispatchEvent(pointerEvent("pointerup", 3, 3, 0)),
    );

    const textarea = container?.querySelector<HTMLTextAreaElement>(
      '[aria-label="Documento de pixel art"]',
    );
    if (textarea === null || textarea === undefined)
      throw new Error("Missing source editor");
    const edited = JSON.parse(textarea.value) as ParsedSource;
    expect(edited.layers[0]!.pixels).toContainEqual([3, 3, "#57d8bd"]);
    expect(edited.layers[1]!.pixels).toEqual([[2, 2, "#222222"]]);

    await act(async () => button("Desfazer").click());
    const undone = JSON.parse(textarea.value) as ParsedSource;
    expect(undone.layers[0]!.pixels).toEqual([[1, 1, "#111111"]]);
    expect(undone.layers[1]!.pixels).toEqual([[2, 2, "#222222"]]);

    await act(async () => button("Refazer").click());
    const redone = JSON.parse(textarea.value) as ParsedSource;
    expect(redone.layers[0]!.pixels).toContainEqual([3, 3, "#57d8bd"]);
    expect(redone.layers[1]!.pixels).toEqual([[2, 2, "#222222"]]);
  });

  it("uses a fresh pointer-down anchor after end and cancel", async () => {
    const canvas = await renderEditor();
    await act(async () => button("Selecionar").click());

    await act(async () =>
      canvas.dispatchEvent(pointerEvent("pointerdown", 10, 12)),
    );
    await act(async () =>
      canvas.dispatchEvent(pointerEvent("pointermove", 14, 15)),
    );
    expect(
      context.strokeRect.mock.calls[context.strokeRect.mock.calls.length - 1],
    ).toEqual([40.5, 48.5, 19, 15]);
    await act(async () =>
      canvas.dispatchEvent(pointerEvent("pointerup", 14, 15, 0)),
    );

    await act(async () =>
      canvas.dispatchEvent(pointerEvent("pointerdown", 20, 22)),
    );
    await act(async () =>
      canvas.dispatchEvent(pointerEvent("pointermove", 18, 19)),
    );
    expect(
      context.strokeRect.mock.calls[context.strokeRect.mock.calls.length - 1],
    ).toEqual([72.5, 76.5, 11, 15]);
    await act(async () =>
      canvas.dispatchEvent(pointerEvent("pointercancel", 18, 19, 0)),
    );

    await act(async () =>
      canvas.dispatchEvent(pointerEvent("pointerdown", 5, 6)),
    );
    await act(async () =>
      canvas.dispatchEvent(pointerEvent("pointermove", 7, 8)),
    );
    expect(
      context.strokeRect.mock.calls[context.strokeRect.mock.calls.length - 1],
    ).toEqual([20.5, 24.5, 11, 11]);
  });
});
