// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { OllamaModel } from "@aip/contracts";
import { ModelPicker } from "./App";

const models: OllamaModel[] = [
  {
    ref: "ollama:llama3",
    providerModelId: "llama3",
    displayName: "Llama 3 local",
    size: 2_000_000_000,
    family: "llama",
    parameterSize: "8B",
    quantization: "Q4_K_M",
    capabilities: ["completion"],
  },
  {
    ref: "ollama:vision",
    providerModelId: "vision",
    displayName: "Vision local",
    size: 3_000_000_000,
    family: "clip",
    parameterSize: "7B",
    quantization: "Q5",
    capabilities: ["vision"],
  },
];

let root: Root | undefined;
let container: HTMLDivElement | undefined;

afterEach(() => {
  if (root !== undefined) act(() => root?.unmount());
  container?.remove();
  root = undefined;
  container = undefined;
});

function renderPicker(
  onSelect: (modelRef: string | null) => void | Promise<void>,
  overrides: Partial<React.ComponentProps<typeof ModelPicker>> = {},
) {
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  act(() => {
    root?.render(
      <ModelPicker
        label="Modelo de teste"
        models={models}
        value={null}
        providerState="available"
        defaultOption={{ label: "Usar padrão", detail: "ollama:llama3" }}
        onSelect={onSelect}
        {...overrides}
      />,
    );
  });
}

function changeInput(element: HTMLInputElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(
    Object.getPrototypeOf(element),
    "value",
  )?.set;
  setter?.call(element, value);
  element.dispatchEvent(new Event("input", { bubbles: true }));
}

describe("ModelPicker", () => {
  it("opens, filters metadata, selects and closes after selection", async () => {
    const onSelect = vi.fn().mockResolvedValue(undefined);
    renderPicker(onSelect);
    const trigger = container?.querySelector<HTMLButtonElement>(
      '[aria-haspopup="listbox"]',
    );
    if (trigger === null || trigger === undefined)
      throw new Error("Missing trigger");

    await act(async () => trigger.click());
    const search = document.body.querySelector<HTMLInputElement>(
      'input[type="search"]',
    );
    if (search === null || search === undefined)
      throw new Error("Missing search");
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    const listbox =
      document.body.querySelector<HTMLElement>('[role="listbox"]');
    const initialSearch = document.body.querySelector<HTMLInputElement>(
      'input[type="search"]',
    );
    if (listbox === null || listbox === undefined)
      throw new Error("Missing listbox");
    if (initialSearch === null || initialSearch === undefined)
      throw new Error("Missing search");
    expect(trigger.getAttribute("aria-controls")).toBe(listbox.id);
    expect(initialSearch.getAttribute("aria-controls")).toBe(listbox.id);
    const initialActiveId = initialSearch.getAttribute("aria-activedescendant");
    expect(initialActiveId).not.toBeNull();
    expect(
      document.getElementById(initialActiveId!)?.getAttribute("data-active"),
    ).toBe("true");
    const visionBeforeFilter = Array.from(
      document.body.querySelectorAll<HTMLElement>('[role="option"]') ?? [],
    ).find((candidate) => candidate.textContent?.includes("Vision local"));

    await act(async () => {
      changeInput(search, "vision");
    });
    expect(
      Array.from(document.body.querySelectorAll('[role="option"]') ?? []).map(
        (option) => option.textContent,
      ),
    ).toEqual([expect.stringContaining("Vision local")]);

    const option = document.body.querySelector<HTMLElement>('[role="option"]');
    if (option === null || option === undefined)
      throw new Error("Missing option");
    expect(initialSearch.getAttribute("aria-activedescendant")).toBe(option.id);
    expect(option.getAttribute("data-active")).toBe("true");
    expect(visionBeforeFilter?.id).toBe(option.id);
    await act(async () => option.click());
    expect(onSelect).toHaveBeenCalledWith("ollama:vision");
    expect(document.body.querySelector('[role="listbox"]')).toBeNull();

    await act(async () => trigger.click());
    await act(async () =>
      document.dispatchEvent(
        new PointerEvent("pointerdown", { bubbles: true }),
      ),
    );
    expect(document.body.querySelector('[role="listbox"]')).toBeNull();
  });

  it("shows provider errors and keeps an unavailable selection visible", async () => {
    const onSelect = vi.fn();
    renderPicker(onSelect, {
      models: [],
      value: "ollama:missing",
      providerState: "timeout",
      disabled: false,
    });
    const trigger = container?.querySelector<HTMLButtonElement>(
      '[aria-haspopup="listbox"]',
    );
    if (trigger === null || trigger === undefined)
      throw new Error("Missing trigger");
    expect(trigger.textContent).toContain("ollama:missing");
    expect(trigger.textContent).toContain("Indisponível");
    expect(container?.textContent).toContain("O Ollama não respondeu a tempo");

    await act(async () => trigger.click());
    const unavailable = document.body.querySelector<HTMLElement>(
      '[role="option"][aria-disabled="true"]',
    );
    if (unavailable === null || unavailable === undefined)
      throw new Error("Missing unavailable option");
    expect(unavailable.textContent).toContain("indisponível");
    await act(async () => unavailable.click());
    expect(onSelect).not.toHaveBeenCalled();

    const search = document.body.querySelector<HTMLInputElement>(
      'input[type="search"]',
    );
    if (search === null || search === undefined)
      throw new Error("Missing search");
    await act(async () =>
      search.dispatchEvent(
        new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }),
      ),
    );
    await act(async () =>
      search.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Enter", bubbles: true }),
      ),
    );
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("supports Arrow, Enter and Escape from the search field", async () => {
    const onSelect = vi.fn().mockResolvedValue(undefined);
    renderPicker(onSelect);
    const trigger = container?.querySelector<HTMLButtonElement>(
      '[aria-haspopup="listbox"]',
    );
    if (trigger === null || trigger === undefined)
      throw new Error("Missing trigger");
    await act(async () =>
      trigger.dispatchEvent(
        new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }),
      ),
    );
    const search = document.body.querySelector<HTMLInputElement>(
      'input[type="search"]',
    );
    if (search === null || search === undefined)
      throw new Error("Missing search");
    expect(document.activeElement).toBe(search);
    await act(async () =>
      search.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Enter", bubbles: true }),
      ),
    );
    expect(onSelect).toHaveBeenCalledWith(null);

    await act(async () => trigger.click());
    const reopenedSearch = document.body.querySelector<HTMLInputElement>(
      'input[type="search"]',
    );
    if (reopenedSearch === null || reopenedSearch === undefined)
      throw new Error("Missing reopened search");
    await act(async () =>
      reopenedSearch.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
      ),
    );
    expect(document.body.querySelector('[role="listbox"]')).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });

  it("portals the menu to body and repositions it when the viewport changes", async () => {
    const onSelect = vi.fn().mockResolvedValue(undefined);
    renderPicker(onSelect);
    const trigger = container?.querySelector<HTMLButtonElement>(
      '[aria-haspopup="listbox"]',
    );
    if (trigger === null || trigger === undefined)
      throw new Error("Missing trigger");
    trigger.getBoundingClientRect = () =>
      ({
        x: 700,
        y: 100,
        top: 100,
        right: 780,
        bottom: 136,
        left: 700,
        width: 80,
        height: 36,
        toJSON: () => ({}),
      }) as DOMRect;

    await act(async () => trigger.click());
    const portal = document.body.querySelector<HTMLElement>(
      '[data-model-picker-portal="true"]',
    );
    if (portal === null) throw new Error("Missing model picker portal");
    expect(portal.parentElement).toBe(document.body);
    expect(portal.style.visibility).toBe("visible");
    expect(portal.style.left).toBe("700px");
    Object.defineProperty(portal, "scrollHeight", {
      configurable: true,
      value: 420,
    });
    await act(async () => window.dispatchEvent(new Event("resize")));
    expect(Number.parseFloat(portal.style.maxHeight)).toBe(420);

    trigger.getBoundingClientRect = () =>
      ({
        x: 900,
        y: 600,
        top: 600,
        right: 980,
        bottom: 636,
        left: 900,
        width: 80,
        height: 36,
        toJSON: () => ({}),
      }) as DOMRect;
    await act(async () => window.dispatchEvent(new Event("resize")));
    expect(Number.parseFloat(portal.style.left)).toBe(
      window.innerWidth - Number.parseFloat(portal.style.width) - 8,
    );
    expect(portal.getAttribute("data-placement")).toBe("above");

    await act(async () =>
      document.dispatchEvent(
        new PointerEvent("pointerdown", { bubbles: true }),
      ),
    );
    expect(document.body.querySelector('[role="listbox"]')).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });
});
