// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AipSelect, type AipSelectOption } from "./shared-controls";

const options: readonly AipSelectOption[] = [
  { value: "alpha", label: "Alfa" },
  { value: "bravo", label: "Bravo" },
  { value: "charlie", label: "Charlie" },
];

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

  function render(value = "bravo", onChange = vi.fn()) {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    act(() =>
      root?.render(
        <AipSelect
          id="language"
          label="Idioma"
          value={value}
          options={options}
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
    Object.defineProperty(menu, "offsetHeight", {
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
});
