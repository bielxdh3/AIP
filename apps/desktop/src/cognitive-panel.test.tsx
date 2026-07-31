// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CognitivePanel } from "./App";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const event = {
  id: "event-1",
  agentId: "astra",
  kind: "trait_delta" as const,
  traitKey: "curiosity",
  sourceKind: "controlled_internal",
  sourceReference: "processor:evidence",
  reason: "Evidência aprovada",
  confidence: 1,
  requestedValue: 0.05,
  appliedDelta: 0.05,
  priorValue: 0.5,
  resultingValue: 0.55,
  status: "applied" as const,
  code: null,
  rollbackOfEventId: null,
  createdAt: 1,
  rawPayload: "ignore this internal payload",
};

const traits = [
  { key: "protected_identity", value: 0.5, isProtected: true },
  { key: "curiosity", value: 0.5, isProtected: false },
];

function change(
  element: HTMLInputElement | HTMLTextAreaElement,
  value: string,
) {
  const setter = Object.getOwnPropertyDescriptor(
    Object.getPrototypeOf(element),
    "value",
  )?.set;
  setter?.call(element, value);
  element.dispatchEvent(new Event("input", { bubbles: true }));
}

describe("CognitivePanel", () => {
  let root: Root;
  let container: HTMLDivElement;

  afterEach(() => {
    act(() => root?.unmount());
    container?.remove();
    invoke.mockReset();
  });

  it("renders safe Portuguese cognitive controls and refreshes corrections, rollback and explanation", async () => {
    (
      globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }
    ).IS_REACT_ACT_ENVIRONMENT = true;
    invoke.mockImplementation((command: string) => {
      if (command === "list_cognitive_traits") return Promise.resolve(traits);
      if (command === "list_cognitive_events")
        return Promise.resolve([
          event,
          { ...event, id: "rejected", status: "rejected" },
        ]);
      if (command === "explain_cognitive_event")
        return Promise.resolve({ event, traitLabel: "Curiosidade" });
      return Promise.resolve(event);
    });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);

    expect(container.textContent).toBe("");
    await act(async () => root.render(<CognitivePanel agentId="astra" />));
    expect(container.textContent).toContain("Valores cognitivos simulados");
    expect(container.textContent).toContain("não representam emoções reais");
    expect(container.textContent).toContain(
      "protected_identity: 0.50 — protegido",
    );
    expect(container.textContent).toContain("curiosity: 0.50 — evolutivo");
    expect(container.querySelectorAll('[aria-label^="Reverter"]').length).toBe(
      1,
    );
    expect(container.querySelector('[aria-label^="Corrigir"]')).not.toBeNull();
    expect(container.querySelector('[aria-label^="Explicar"]')).not.toBeNull();

    const correction = container.querySelector(
      '[aria-label^="Corrigir"]',
    ) as HTMLButtonElement;
    await act(async () => correction.click());
    expect(container.textContent).toContain("Informe o motivo da correção.");

    const input = container.querySelector("input") as HTMLInputElement;
    const reason = container.querySelector("textarea") as HTMLTextAreaElement;
    await act(async () => {
      change(input, "2");
      change(reason, "Motivo válido");
    });
    await act(async () => correction.click());
    expect(container.textContent).toContain("Informe um valor entre 0 e 1.");

    await act(async () => change(input, "0.6"));
    await act(async () => correction.click());
    expect(invoke).toHaveBeenCalledWith(
      "create_owner_trait_correction",
      expect.objectContaining({
        agentId: "astra",
        value: 0.6,
        reason: "Motivo válido",
      }),
    );
    expect(invoke).toHaveBeenCalledWith("list_cognitive_traits", {
      agentId: "astra",
    });
    expect(container.textContent).toContain("Correção aplicada.");

    await act(async () =>
      (
        container.querySelector('[aria-label^="Explicar"]') as HTMLButtonElement
      ).click(),
    );
    expect(container.textContent).toContain(
      "Curiosidade: 0.50 → 0.55. Evidência aprovada",
    );
    expect(container.textContent).not.toContain("processor:evidence");
    expect(container.textContent).not.toContain("ignore this internal payload");
    await act(async () =>
      (
        container.querySelector('[aria-label^="Reverter"]') as HTMLButtonElement
      ).click(),
    );
    expect(invoke).toHaveBeenCalledWith(
      "rollback_cognitive_event",
      expect.objectContaining({ eventId: "event-1" }),
    );
    expect(container.textContent).toContain("Reversão aplicada.");
  });

  it("ignores late responses after switching agents and survives unavailable commands", async () => {
    let resolveAstra: ((value: typeof traits) => void) | undefined;
    invoke.mockImplementation((command: string, args: { agentId: string }) => {
      if (args.agentId === "astra") {
        return new Promise((resolve) => {
          resolveAstra = resolve;
        });
      }
      if (command === "list_cognitive_traits")
        return Promise.resolve([
          { key: "autonomy", value: 0.7, isProtected: false },
        ]);
      return Promise.resolve([]);
    });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () => root.render(<CognitivePanel agentId="astra" />));
    await act(async () => root.render(<CognitivePanel agentId="luma" />));
    await act(async () => resolveAstra?.(traits));
    expect(container.textContent).toContain("autonomy: 0.70");
    expect(container.textContent).not.toContain("curiosity: 0.50");
  });

  it("shows loading, keeps empty history safe, and survives safe-mode failures", async () => {
    let resolveTraits: ((value: typeof traits) => void) | undefined;
    let resolveEvents: ((value: (typeof event)[]) => void) | undefined;
    invoke.mockImplementation(
      (command: string) =>
        new Promise((resolve) => {
          if (command === "list_cognitive_traits") resolveTraits = resolve;
          else resolveEvents = resolve;
        }),
    );
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    act(() => root.render(<CognitivePanel agentId="astra" />));
    expect(container.textContent).toContain("Carregando valores cognitivos");
    await act(async () => {
      resolveTraits?.(traits);
      resolveEvents?.([]);
    });
    expect(container.textContent).toContain("Histórico recente");
    expect(container.querySelector('[aria-label^="Reverter"]')).toBeNull();

    invoke.mockRejectedValue("operation_unavailable");
    await act(async () => root.render(<CognitivePanel agentId="luma" />));
    expect(container.textContent).toContain(
      "Não foi possível carregar os valores cognitivos.",
    );
  });

  it("maps stable backend errors to Portuguese copy", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_cognitive_traits") return Promise.resolve(traits);
      if (command === "list_cognitive_events") return Promise.resolve([]);
      return Promise.reject("protected_trait");
    });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () => root.render(<CognitivePanel agentId="astra" />));
    await act(async () =>
      change(
        container.querySelector("textarea") as HTMLTextAreaElement,
        "Motivo válido",
      ),
    );
    await act(async () =>
      (
        container.querySelector('[aria-label^="Corrigir"]') as HTMLButtonElement
      ).click(),
    );
    expect(container.textContent).toContain("Este traço é protegido.");
  });
});
