// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  AppSnapshot,
  OllamaModel,
  PhaseOneState,
  ProvisionalAgent,
} from "@aip/contracts";
import { SettingsSurface } from "./App";
import { ThemeProvider } from "./theme";
import {
  MODEL_PREFERENCES_STORAGE_KEY,
  readModelPreferences,
} from "./model-preferences";

const { invoke, listen } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(() => Promise.resolve(() => undefined)),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

const model: OllamaModel = {
  ref: "ollama:llama3",
  providerModelId: "llama3",
  displayName: "Llama 3 local",
  size: 2_000_000_000,
  family: "llama",
  parameterSize: "8B",
  quantization: "Q4_K_M",
  capabilities: ["completion"],
};

const astra: ProvisionalAgent = {
  id: "agt_astra_provisional",
  name: "Astra",
  profileKey: "owner",
  spriteKey: "astra",
  position: { x: 10, y: 20 },
  birthday: "2020-01-02",
  fictiveAge: 28,
  ageCategory: "adult",
  species: "agent",
  pronouns: "they/them",
  personalitySummary: "Astra fixture",
  traitsJson: "{}",
  appearancePreset: "astra",
};

const luma: ProvisionalAgent = {
  ...astra,
  id: "agt_luma_provisional",
  name: "Luma",
  profileKey: "companion",
  spriteKey: "luma",
};

const provider = {
  state: "available" as const,
  detailCode: "provider_available",
  models: [model],
  refreshedAt: 123,
};

function phase(agent: ProvisionalAgent, selectedModelRef: string) {
  return {
    agent,
    provider,
    selectedModelRef,
    defaultModelRef: "ollama:llama3",
  } as unknown as PhaseOneState;
}

const phases = {
  [astra.id]: phase(astra, "ollama:llama3"),
  [luma.id]: phase(luma, "ollama:llama3"),
};

const snapshot: AppSnapshot = {
  appVersion: "0.1.0",
  buildSha: "fixture-build",
  buildTimestamp: "2026-08-31T00:00:00Z",
  runtimePackagingMode: "fixture",
  safeMode: false,
  databaseReady: true,
  migrationVersion: 6,
  runtime: {
    state: "ready",
    protocolVersion: 1,
    detailCode: "runtime_ready",
  },
  agents: [astra, luma],
  onboardingRequired: false,
};

describe("SettingsSurface model policy", () => {
  let root: Root | undefined;
  let container: HTMLDivElement | undefined;

  beforeEach(() => {
    window.localStorage.clear();
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    invoke.mockImplementation(
      (command: string, args?: { agentId?: string }) => {
        if (command === "get_phase_one_state") {
          return Promise.resolve(phases[args?.agentId ?? ""]);
        }
        if (command === "load_pixel_document")
          return Promise.resolve('{"layers":[]}');
        return Promise.resolve(undefined);
      },
    );
  });

  afterEach(() => {
    if (root !== undefined) act(() => root?.unmount());
    container?.remove();
    root = undefined;
    container = undefined;
    window.localStorage.clear();
    invoke.mockReset();
    listen.mockClear();
  });

  async function settle() {
    await new Promise<void>((resolve) => window.setTimeout(resolve, 0));
  }

  async function renderSurface(
    onProfile = vi.fn(),
    onDefaultModel = vi.fn(),
    onWorkspace = vi.fn(),
  ) {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () => {
      root?.render(
        <ThemeProvider>
          <SettingsSurface
            snapshot={snapshot}
            changingMode={false}
            onToggleSafeMode={vi.fn()}
            activeAgentId={astra.id}
            onProfile={onProfile}
            onDefaultModel={onDefaultModel}
            onWorkspace={onWorkspace}
          />
        </ThemeProvider>,
      );
      await settle();
    });
    return { onProfile, onDefaultModel, onWorkspace };
  }

  async function openSection(section: string) {
    const button = Array.from(
      container?.querySelectorAll<HTMLButtonElement>(".settings-nav button") ??
        [],
    ).find((candidate) => candidate.textContent === section);
    if (button === undefined) throw new Error(`Missing ${section} section`);
    await act(async () => {
      button.click();
      await settle();
    });
  }

  it("renders truthful provider/model metadata and agent shortcuts", async () => {
    const callbacks = await renderSurface();
    await openSection("Agentes");

    const agentCard = container?.querySelector<HTMLElement>(
      `[data-agent-id="${astra.id}"]`,
    );
    expect(agentCard?.textContent).toContain("Astra");
    expect(agentCard?.textContent).toContain("Disponível");
    expect(agentCard?.textContent).toContain("ollama:llama3");
    expect(agentCard?.textContent).toContain("Selecionado");

    const button = (label: string) =>
      Array.from(
        agentCard?.querySelectorAll<HTMLButtonElement>("button") ?? [],
      ).find((candidate) => candidate.textContent === label);
    await act(async () => button("Abrir perfil")?.click());
    await act(async () => button("Modelo padrão")?.click());
    await act(async () => button("Aparência")?.click());
    await act(async () => button("Estado")?.click());
    expect(callbacks.onProfile).toHaveBeenCalledWith(astra.id);
    expect(callbacks.onDefaultModel).toHaveBeenCalledWith(astra.id);
    expect(callbacks.onWorkspace).toHaveBeenNthCalledWith(1, "appearance");
    expect(callbacks.onWorkspace).toHaveBeenNthCalledWith(2, "state");

    await openSection("Modelos");
    const modelCard = container?.querySelector<HTMLElement>(
      `[data-model-ref="${model.ref}"]`,
    );
    const text = modelCard?.textContent ?? "";
    expect(text).toContain("Llama 3 local");
    expect(text).toContain("Provedor: Ollama");
    expect(text).toContain("Ref. do modeloollama:llama3");
    expect(text).toContain("Ref. no provedorllama3");
    expect(text).toContain("Famíliallama");
    expect(text).toContain("Parâmetros8B");
    expect(text).toContain("QuantizaçãoQ4_K_M");
    expect(text).toContain("CargaNão informada pelo snapshot do provedor");
    expect(text).toContain("Provedor disponível");
  });

  it("persists hide/show, Auto eligibility, preferred, fallback and policy choices", async () => {
    await renderSurface();
    await openSection("Modelos");
    const modelCard = container?.querySelector<HTMLElement>(
      `[data-model-ref="${model.ref}"]`,
    );
    if (modelCard === null || modelCard === undefined)
      throw new Error("Missing model card");

    const hide = modelCard.querySelector<HTMLInputElement>(
      `[aria-label="Mostrar ${model.displayName} nos seletores"]`,
    );
    const auto = modelCard.querySelector<HTMLInputElement>(
      `[aria-label="Elegível no Auto: ${model.displayName}"]`,
    );
    const fallback = modelCard.querySelector<HTMLInputElement>(
      `[aria-label="Usar ${model.displayName} apenas como fallback"]`,
    );
    if (hide === null || auto === null || fallback === null)
      throw new Error("Missing model policy controls");

    await act(async () => hide.click());
    expect(hide.checked).toBe(false);
    expect(modelCard.classList.contains("hidden")).toBe(true);
    await act(async () => hide.click());
    expect(hide.checked).toBe(true);
    expect(modelCard.classList.contains("hidden")).toBe(false);

    await act(async () => auto.click());
    expect(auto.checked).toBe(false);
    await act(async () => auto.click());
    expect(auto.checked).toBe(true);

    await act(async () => fallback.click());
    const preferred = Array.from(modelCard.querySelectorAll("button")).find(
      (candidate) => candidate.textContent === "Definir como preferido",
    );
    await act(async () => preferred?.click());

    expect(fallback.checked).toBe(true);
    expect(modelCard.textContent).toContain("Preferido");
    expect(readModelPreferences()).toEqual({
      hiddenModelRefs: [],
      excludedModelRefs: [],
      fallbackOnlyModelRefs: [model.ref],
      preferredModelRef: model.ref,
      policyMode: "auto",
    });

    const policyTrigger = container?.querySelector<HTMLButtonElement>(
      "#settings-model-policy-trigger",
    );
    if (policyTrigger === null || policyTrigger === undefined)
      throw new Error("Missing model policy trigger");
    await act(async () => policyTrigger.click());
    const manual = document.body.querySelector<HTMLElement>(
      '.aip-select-option[data-value="manual"]',
    );
    if (manual === null) throw new Error("Missing manual policy option");
    await act(async () => manual.click());

    expect(readModelPreferences()).toMatchObject({ policyMode: "manual" });
    expect(
      JSON.parse(
        window.localStorage.getItem(MODEL_PREFERENCES_STORAGE_KEY) ?? "null",
      ),
    ).toMatchObject({
      hiddenModelRefs: [],
      excludedModelRefs: [],
      fallbackOnlyModelRefs: [model.ref],
      preferredModelRef: model.ref,
      policyMode: "manual",
    });
  });
});
