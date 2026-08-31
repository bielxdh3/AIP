// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  DEFAULT_MODEL_PREFERENCES,
  MODEL_PREFERENCES_STORAGE_KEY,
  normalizeModelPreferences,
  readModelPreferences,
  routingPolicyPayload,
  useModelPreferences,
  writeModelPreferences,
} from "./model-preferences";

describe("model preferences", () => {
  let root: Root | undefined;
  let container: HTMLDivElement | undefined;

  beforeEach(() => {
    window.localStorage.clear();
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
  });

  afterEach(() => {
    if (root !== undefined) act(() => root?.unmount());
    container?.remove();
    root = undefined;
    container = undefined;
    window.localStorage.clear();
  });

  it("normalizes bounded, unique refs and supported policy modes", () => {
    const boundedRefs = Array.from(
      { length: 64 },
      (_, index) => `ollama:model-${index}`,
    );
    expect(
      normalizeModelPreferences({
        hiddenModelRefs: [...boundedRefs, "ollama:overflow"],
        excludedModelRefs: ["ollama:excluded", "ollama:excluded", 42, ""],
        fallbackOnlyModelRefs: ["ollama:fallback", "ollama:fallback"],
        preferredModelRef: "ollama:preferred",
        policyMode: "quality",
      }),
    ).toEqual({
      hiddenModelRefs: boundedRefs,
      excludedModelRefs: ["ollama:excluded"],
      fallbackOnlyModelRefs: ["ollama:fallback"],
      preferredModelRef: "ollama:preferred",
      policyMode: "quality",
    });

    expect(
      normalizeModelPreferences({
        hiddenModelRefs: ["line\nbreak", "x".repeat(209)],
        preferredModelRef: "line\nbreak",
        policyMode: "unsupported",
      }),
    ).toEqual(DEFAULT_MODEL_PREFERENCES);
  });

  it("reads and writes normalized preferences to local storage", () => {
    window.localStorage.setItem(
      MODEL_PREFERENCES_STORAGE_KEY,
      JSON.stringify({
        preferredModelRef: "ollama:stored",
        policyMode: "manual",
      }),
    );
    expect(readModelPreferences()).toEqual({
      ...DEFAULT_MODEL_PREFERENCES,
      preferredModelRef: "ollama:stored",
      policyMode: "manual",
    });

    expect(
      writeModelPreferences({
        ...DEFAULT_MODEL_PREFERENCES,
        hiddenModelRefs: ["ollama:hidden", "ollama:hidden"],
        policyMode: "speed",
      }),
    ).toEqual({
      ...DEFAULT_MODEL_PREFERENCES,
      hiddenModelRefs: ["ollama:hidden"],
      policyMode: "speed",
    });
    expect(
      JSON.parse(
        window.localStorage.getItem(MODEL_PREFERENCES_STORAGE_KEY) ?? "null",
      ),
    ).toEqual({
      ...DEFAULT_MODEL_PREFERENCES,
      hiddenModelRefs: ["ollama:hidden"],
      policyMode: "speed",
    });
  });

  it("maps bounded preferences to the Rust routing policy payload", () => {
    expect(
      routingPolicyPayload({
        ...DEFAULT_MODEL_PREFERENCES,
        hiddenModelRefs: ["ollama:hidden"],
        excludedModelRefs: ["ollama:excluded"],
        fallbackOnlyModelRefs: ["ollama:fallback"],
        preferredModelRef: "ollama:preferred",
        policyMode: "quality",
      }),
    ).toEqual({
      mode: "quality",
      excludedModelRefs: ["ollama:excluded"],
      fallbackOnlyModelRefs: ["ollama:fallback"],
      preferredModelRef: "ollama:preferred",
    });
  });

  it("updates the hook state and persisted value through its updater", async () => {
    function Probe() {
      const [preferences, updatePreferences] = useModelPreferences();
      return (
        <>
          <output data-policy={preferences.policyMode} />
          <button
            type="button"
            onClick={() =>
              updatePreferences((current) => ({
                ...current,
                preferredModelRef: "ollama:preferred",
                policyMode: "manual",
              }))
            }
          >
            Atualizar
          </button>
        </>
      );
    }

    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () => root?.render(<Probe />));
    expect(container.querySelector("output")?.dataset.policy).toBe("auto");

    await act(async () =>
      container?.querySelector<HTMLButtonElement>("button")?.click(),
    );
    expect(container.querySelector("output")?.dataset.policy).toBe("manual");
    expect(readModelPreferences()).toMatchObject({
      preferredModelRef: "ollama:preferred",
      policyMode: "manual",
    });
  });
});
