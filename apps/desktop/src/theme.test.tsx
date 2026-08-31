// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import {
  THEME_STORAGE_KEY,
  ThemeControls,
  ThemeProvider,
  contrastRatio,
  normalizeThemePreferences,
  readableForeground,
  useTheme,
} from "./theme";

type QueryState = {
  matches: boolean;
  listeners: Set<() => void>;
  addEventListener: (type: string, listener: () => void) => void;
  removeEventListener: (type: string, listener: () => void) => void;
};

function changeColor(element: HTMLInputElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(
    HTMLInputElement.prototype,
    "value",
  )?.set;
  setter?.call(element, value);
  element.dispatchEvent(new Event("input", { bubbles: true }));
  element.dispatchEvent(new Event("change", { bubbles: true }));
}

async function chooseOption(container: HTMLElement, id: string, value: string) {
  const trigger = container.querySelector<HTMLButtonElement>(`#${id}-trigger`);
  if (trigger === null) throw new Error(`Missing ${id} trigger`);
  await act(async () => trigger.click());
  const option = document.body.querySelector<HTMLElement>(
    `.aip-select-option[data-value="${value}"]`,
  );
  if (option === null) throw new Error(`Missing ${value} option`);
  await act(async () => option.click());
}

function StateProbe() {
  const { preferences, resolvedMode, reducedMotion } = useTheme();
  return (
    <output
      data-font={preferences.font}
      data-mode={preferences.mode}
      data-radius={preferences.radius}
      data-reduced-motion={reducedMotion}
      data-resolved-mode={resolvedMode}
    />
  );
}

describe("theme foundations", () => {
  let root: Root | undefined;
  let container: HTMLDivElement | undefined;
  let systemLight = false;
  let reducedMotion = false;
  let queries: Map<string, QueryState>;

  beforeEach(() => {
    window.localStorage.clear();
    systemLight = false;
    reducedMotion = false;
    queries = new Map();
    const matchMedia = vi.fn((query: string): MediaQueryList => {
      const state: QueryState = {
        matches:
          query === "(prefers-color-scheme: light)"
            ? systemLight
            : reducedMotion,
        addEventListener: (_type, listener) => {
          state.listeners.add(listener);
        },
        removeEventListener: (_type, listener) => {
          state.listeners.delete(listener);
        },
        listeners: new Set<() => void>(),
      };
      queries.set(query, state);
      return state as unknown as MediaQueryList;
    });
    vi.stubGlobal("matchMedia", matchMedia);
  });

  afterEach(() => {
    if (root !== undefined) act(() => root?.unmount());
    container?.remove();
    root = undefined;
    container = undefined;
    vi.unstubAllGlobals();
  });

  function render(children: ReactNode) {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    act(() => root?.render(<ThemeProvider>{children}</ThemeProvider>));
  }

  it("supports dark, light, and system modes", async () => {
    render(
      <>
        <ThemeControls />
        <StateProbe />
      </>,
    );
    expect(document.documentElement.dataset.theme).toBe("dark");
    if (container === undefined) throw new Error("Missing theme container");
    await chooseOption(container, "theme-mode", "light");
    expect(document.documentElement.dataset.theme).toBe("light");
    await chooseOption(container, "theme-mode", "dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
    systemLight = true;
    await chooseOption(container, "theme-mode", "system");
    const query = queries.get("(prefers-color-scheme: light)")!;
    query.matches = true;
    await act(async () => query.listeners.forEach((listener) => listener()));
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("starts with the safe Times New Roman foundation", () => {
    render(<StateProbe />);
    expect(document.documentElement.style.getPropertyValue("--font-ui")).toBe(
      '"Times New Roman", Times, serif',
    );
    expect(document.documentElement.style.getPropertyValue("--space-7")).toBe(
      "32px",
    );
    expect(
      document.documentElement.style.getPropertyValue("--color-surface-soft"),
    ).toBe("#292b2f");
  });

  it("follows system appearance changes and keeps readable custom colors", async () => {
    systemLight = false;
    render(<ThemeControls />);
    if (container === undefined) throw new Error("Missing theme container");
    await chooseOption(container, "theme-mode", "system");
    expect(document.documentElement.dataset.theme).toBe("dark");

    systemLight = true;
    const query = queries.get("(prefers-color-scheme: light)")!;
    query.matches = true;
    await act(async () => query.listeners.forEach((listener) => listener()));
    expect(document.documentElement.dataset.theme).toBe("light");

    expect(readableForeground("#ffffff")).toBe("#241d14");
    expect(readableForeground("#000000")).toBe("#fffaf1");
    expect(
      contrastRatio("#ffffff", readableForeground("#ffffff")),
    ).toBeGreaterThanOrEqual(4.5);
    expect(
      contrastRatio("#000000", readableForeground("#000000")),
    ).toBeGreaterThanOrEqual(4.5);
    const colors = container?.querySelectorAll<HTMLInputElement>(
      'input[type="color"]',
    );
    if (colors === undefined || colors.length < 2)
      throw new Error("Missing theme color controls");
    await act(async () => {
      changeColor(colors[0]!, "#ffffff");
      changeColor(colors[1]!, "#000000");
    });
    expect(
      document.documentElement.style.getPropertyValue("--color-primary"),
    ).toBe("#ffffff");
    expect(
      document.documentElement.style.getPropertyValue("--color-on-primary"),
    ).toBe("#241d14");
    expect(
      document.documentElement.style.getPropertyValue("--color-on-secondary"),
    ).toBe("#fffaf1");
  });

  it("persists allowlisted radius and font preferences", async () => {
    render(<ThemeControls />);
    if (container === undefined) throw new Error("Missing theme container");
    await chooseOption(container, "theme-radius", "soft");
    await chooseOption(container, "theme-font", "atkinson");
    expect(document.documentElement.style.getPropertyValue("--radius-md")).toBe(
      "12px",
    );
    expect(
      document.documentElement.style.getPropertyValue("--font-ui"),
    ).toContain("Atkinson Hyperlegible");
    expect(window.localStorage.getItem(THEME_STORAGE_KEY)).toContain(
      '"radius":"soft"',
    );
    expect(window.localStorage.getItem(THEME_STORAGE_KEY)).toContain(
      '"font":"atkinson"',
    );
  });

  it("sets reduced-motion tokens and can restore full motion", async () => {
    reducedMotion = true;
    render(<StateProbe />);
    expect(document.documentElement.dataset.motion).toBe("reduced");
    expect(
      document.documentElement.style.getPropertyValue("--motion-fast"),
    ).toBe("0ms");

    reducedMotion = false;
    const query = queries.get("(prefers-reduced-motion: reduce)")!;
    query.matches = false;
    await act(async () => query.listeners.forEach((listener) => listener()));
    expect(document.documentElement.dataset.motion).toBe("full");
    expect(
      document.documentElement.style.getPropertyValue("--motion-fast"),
    ).toBe("120ms");
  });

  it("rejects invalid stored colors without accepting arbitrary font stacks", () => {
    expect(
      normalizeThemePreferences({
        primaryColor: "url(javascript:bad)",
        secondaryColor: "#abc",
        font: 'url("remote-font")',
      }),
    ).toMatchObject({
      primaryColor: "#d0aa72",
      secondaryColor: "#efd09b",
      font: "times",
    });
  });
});
