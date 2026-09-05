// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AipSelect, FilePicker, type AipSelectOption } from "./shared-controls";

const options: readonly AipSelectOption[] = [
  { value: "alpha", label: "Alfa" },
  { value: "bravo", label: "Bravo" },
  { value: "charlie", label: "Charlie" },
];

const manyOptions: readonly AipSelectOption[] = Array.from(
  { length: 12 },
  (_, index) => ({
    value: `option-${index}`,
    label: `Opção ${index + 1}`,
  }),
);

describe("AipSelect", () => {
  let root: Root | undefined;
  let container: HTMLDivElement | undefined;
  const originalInnerHeight = window.innerHeight;
  const originalInnerWidth = window.innerWidth;

  afterEach(() => {
    if (root !== undefined) act(() => root?.unmount());
    container?.remove();
    root = undefined;
    container = undefined;
    Object.defineProperty(window, "innerHeight", {
      configurable: true,
      value: originalInnerHeight,
    });
    Object.defineProperty(window, "innerWidth", {
      configurable: true,
      value: originalInnerWidth,
    });
    vi.restoreAllMocks();
  });

  function render(
    value = "bravo",
    onChange = vi.fn(),
    selectOptions: readonly AipSelectOption[] = options,
  ) {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    act(() =>
      root?.render(
        <AipSelect
          id="language"
          label="Idioma"
          value={value}
          options={selectOptions}
          onChange={onChange}
        />,
      ),
    );
    return onChange;
  }

  function trigger() {
    const element =
      container?.querySelector<HTMLButtonElement>("#language-trigger");
    if (element === null || element === undefined)
      throw new Error("Missing select trigger");
    return element;
  }

  function press(key: string) {
    trigger().dispatchEvent(
      new KeyboardEvent("keydown", { key, bubbles: true }),
    );
  }

  it("exposes stable listbox semantics and keyboard selection", async () => {
    const onChange = render();
    const button = trigger();
    expect(button.getAttribute("aria-haspopup")).toBe("listbox");
    expect(button.getAttribute("aria-controls")).toBe("language-listbox");
    expect(button.getAttribute("aria-expanded")).toBe("false");

    button.focus();
    await act(async () => press("Enter"));
    const listbox = document.querySelector<HTMLElement>("#language-listbox");
    if (listbox === null) throw new Error("Missing listbox");
    const optionElements = Array.from(
      listbox.querySelectorAll<HTMLElement>('[role="option"]'),
    );
    const optionIds = optionElements.map((option) => option.id);
    expect(button.getAttribute("aria-owns")).toBe("language-listbox");
    expect(button.getAttribute("aria-activedescendant")).toBe(
      "language-option-bravo",
    );
    expect(listbox.getAttribute("aria-labelledby")).toBe("language-label");
    expect(optionElements[1]?.getAttribute("aria-selected")).toBe("true");

    await act(async () => {
      press("ArrowDown");
      press("Home");
      press("End");
      press("ArrowUp");
    });
    expect(button.getAttribute("aria-activedescendant")).toBe(
      "language-option-bravo",
    );
    await act(async () => press("ArrowDown"));
    expect(button.getAttribute("aria-activedescendant")).toBe(
      "language-option-charlie",
    );
    await act(async () => press("Enter"));
    expect(onChange).toHaveBeenCalledWith("charlie");
    expect(button.getAttribute("aria-expanded")).toBe("false");
    expect(document.activeElement).toBe(button);

    await act(async () => button.click());
    expect(
      Array.from(
        document.querySelectorAll<HTMLElement>(
          '#language-listbox [role="option"]',
        ),
      ).map((option) => option.id),
    ).toEqual(optionIds);
    await act(async () => press("Escape"));
    expect(button.getAttribute("aria-expanded")).toBe("false");
  });

  it("closes on outside pointer input and flips above the trigger", async () => {
    Object.defineProperty(window, "innerHeight", {
      configurable: true,
      value: 400,
    });
    Object.defineProperty(window, "innerWidth", {
      configurable: true,
      value: 600,
    });
    render();
    const button = trigger();
    vi.spyOn(button, "getBoundingClientRect").mockReturnValue({
      top: 320,
      bottom: 356,
      left: 80,
      right: 260,
      width: 180,
      height: 36,
      x: 80,
      y: 320,
      toJSON: () => ({}),
    });
    await act(async () => button.click());
    const menu = document.querySelector<HTMLElement>("#language-listbox");
    if (menu === null) throw new Error("Missing listbox");
    Object.defineProperty(menu, "scrollHeight", {
      configurable: true,
      value: 180,
    });
    await act(async () => window.dispatchEvent(new Event("resize")));
    expect(menu.dataset.placement).toBe("above");
    expect(menu.style.top).toBe("136px");
    expect(menu.style.left).toBe("80px");

    const outside = document.createElement("div");
    document.body.append(outside);
    await act(async () =>
      outside.dispatchEvent(new Event("pointerdown", { bubbles: true })),
    );
    expect(button.getAttribute("aria-expanded")).toBe("false");
    outside.remove();
  });

  it("uses unconstrained content height before applying the viewport clamp", async () => {
    Object.defineProperty(window, "innerHeight", {
      configurable: true,
      value: 800,
    });
    Object.defineProperty(window, "innerWidth", {
      configurable: true,
      value: 600,
    });
    render("option-2", vi.fn(), manyOptions);
    const button = trigger();
    vi.spyOn(button, "getBoundingClientRect").mockReturnValue({
      top: 120,
      bottom: 156,
      left: 80,
      right: 260,
      width: 180,
      height: 36,
      x: 80,
      y: 120,
      toJSON: () => ({}),
    });
    await act(async () => button.click());
    const menu = document.querySelector<HTMLElement>("#language-listbox");
    if (menu === null) throw new Error("Missing listbox");
    Object.defineProperty(menu, "scrollHeight", {
      configurable: true,
      value: 420,
    });
    await act(async () => window.dispatchEvent(new Event("resize")));
    expect(Number.parseFloat(menu.style.maxHeight)).toBe(420);
    expect(Number.parseFloat(menu.style.maxHeight)).toBeGreaterThan(96);
  });

  it("flips and clamps the natural menu height near a viewport edge", async () => {
    Object.defineProperty(window, "innerHeight", {
      configurable: true,
      value: 240,
    });
    Object.defineProperty(window, "innerWidth", {
      configurable: true,
      value: 600,
    });
    render("option-2", vi.fn(), manyOptions);
    const button = trigger();
    vi.spyOn(button, "getBoundingClientRect").mockReturnValue({
      top: 160,
      bottom: 196,
      left: 80,
      right: 260,
      width: 180,
      height: 36,
      x: 80,
      y: 160,
      toJSON: () => ({}),
    });
    await act(async () => button.click());
    const menu = document.querySelector<HTMLElement>("#language-listbox");
    if (menu === null) throw new Error("Missing listbox");
    Object.defineProperty(menu, "scrollHeight", {
      configurable: true,
      value: 420,
    });
    await act(async () => window.dispatchEvent(new Event("resize")));
    expect(menu.dataset.placement).toBe("above");
    expect(Number.parseFloat(menu.style.maxHeight)).toBe(152);
    expect(Number.parseFloat(menu.style.maxHeight)).toBeLessThan(420);
    expect(Number.parseFloat(menu.style.top)).toBe(8);
  });

  it("keeps select and file input interactions custom and accessible", () => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    act(() =>
      root?.render(
        <>
          <AipSelect
            id="custom-control"
            label="Opção"
            value="alpha"
            options={options}
            onChange={vi.fn()}
          />
          <FilePicker id="attachment" label="Arquivo" onChange={vi.fn()} />
        </>,
      ),
    );

    expect(container.querySelector("select")).toBeNull();
    const fileInput =
      container.querySelector<HTMLInputElement>('input[type="file"]');
    expect(fileInput).not.toBeNull();
    expect(fileInput?.className).toContain("visually-hidden");
    expect(
      container
        .querySelector<HTMLButtonElement>(".aip-file-picker-action")
        ?.getAttribute("aria-controls"),
    ).toBe("attachment-input");
  });
});
