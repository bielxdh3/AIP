// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProvisionalAgent } from "@aip/contracts";
import { ProfileForm } from "./App";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("./use-phase-one", () => ({
  usePhaseOne: () => ({ phase: null, error: false, load: vi.fn() }),
}));

const agent: ProvisionalAgent = {
  id: "agent",
  name: "Astra",
  profileKey: "owner",
  spriteKey: "astra",
  position: { x: 0, y: 0 },
  birthday: "2020-01-02",
  fictiveAge: 28,
  ageCategory: "adult",
  species: "agent",
  pronouns: "they/them",
  personalitySummary: "Descrição inicial",
  traitsJson: JSON.stringify({ curiosity: 50 }),
  appearancePreset: "astra",
};

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

describe("ProfileForm", () => {
  let root: Root | undefined;
  let container: HTMLDivElement | undefined;

  afterEach(() => {
    if (root !== undefined) act(() => root?.unmount());
    container?.remove();
    root = undefined;
    container = undefined;
    vi.clearAllMocks();
  });

  it("keeps dirty fields during snapshot refresh and Cancel restores the baseline", async () => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () =>
      root?.render(<ProfileForm agent={agent} done={vi.fn()} />),
    );
    expect(
      container.querySelector(
        '.profile-default-model [aria-haspopup="listbox"]',
      ),
    ).not.toBeNull();
    expect(container.querySelector(".profile-default-model select")).toBeNull();

    const fields = container.querySelectorAll(".profile-fields > label");
    const age = fields[2]?.querySelector("input") as HTMLInputElement;
    const description = fields[6]?.querySelector(
      "textarea",
    ) as HTMLTextAreaElement;
    const trait = container.querySelector(
      ".trait-grid input",
    ) as HTMLInputElement;
    await act(async () => {
      change(age, "42");
      change(description, "Descrição editada");
      change(trait, "73");
    });

    await act(async () =>
      root?.render(
        <ProfileForm
          agent={{ ...agent, personalitySummary: "Snapshot remoto" }}
          done={vi.fn()}
        />,
      ),
    );
    expect(age.value).toBe("42");
    expect(description.value).toBe("Descrição editada");
    expect(trait.value).toBe("73");

    const cancel = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent === "Cancelar",
    );
    await act(async () => cancel?.click());
    expect(age.value).toBe("28");
    expect(description.value).toBe("Descrição inicial");
    expect(trait.value).toBe("50");
  });

  it("accepts a clean remote snapshot and resets when the agent identity changes", async () => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () =>
      root?.render(<ProfileForm agent={agent} done={vi.fn()} />),
    );

    const updated = {
      ...agent,
      name: "Astra remota",
      personalitySummary: "Descrição remota",
    };
    await act(async () =>
      root?.render(<ProfileForm agent={updated} done={vi.fn()} />),
    );
    expect(
      (
        container.querySelector(
          '.profile-fields input[value="Astra remota"]',
        ) as HTMLInputElement | null
      )?.value,
    ).toBe("Astra remota");
    expect(
      (
        container.querySelector(
          ".profile-fields textarea",
        ) as HTMLTextAreaElement
      ).value,
    ).toBe("Descrição remota");

    await act(async () =>
      root?.render(
        <ProfileForm
          agent={{ ...updated, id: "other", name: "Luma" }}
          done={vi.fn()}
        />,
      ),
    );
    expect(
      (
        container.querySelector(
          '.profile-fields input[value="Luma"]',
        ) as HTMLInputElement | null
      )?.value,
    ).toBe("Luma");
  });

  it("persists traits, fictive age, description and supported select values", async () => {
    invoke.mockResolvedValue(undefined);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () =>
      root?.render(<ProfileForm agent={agent} done={vi.fn()} />),
    );

    const fields = container.querySelectorAll(".profile-fields > label");
    await act(async () => {
      change(fields[2]?.querySelector("input") as HTMLInputElement, "42");
      change(
        fields[6]?.querySelector("textarea") as HTMLTextAreaElement,
        "Nova descrição",
      );
      change(
        container!.querySelector(".trait-grid input") as HTMLInputElement,
        "73",
      );
    });
    await chooseOption(container!, "profile-ageCategory", "child");
    await chooseOption(container!, "profile-species", "human");
    await chooseOption(container!, "profile-pronouns", "ela/dela");
    const save = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent === "Salvar alterações",
    );
    await act(async () => save?.click());

    expect(invoke).toHaveBeenCalledWith(
      "update_agent_profile",
      expect.objectContaining({
        agent: expect.objectContaining({
          fictiveAge: 42,
          personalitySummary: "Nova descrição",
          ageCategory: "child",
          species: "human",
          pronouns: "ela/dela",
          traitsJson: expect.stringContaining('"curiosity":73'),
        }),
      }),
    );
    expect(
      container.querySelectorAll(".profile-fields .aip-select-trigger"),
    ).toHaveLength(3);

    const savedAgent = invoke.mock.calls.find(
      ([command]) => command === "update_agent_profile",
    )?.[1].agent as ProvisionalAgent;
    await act(async () =>
      root?.render(<ProfileForm agent={savedAgent} done={vi.fn()} />),
    );
    expect(
      (
        container.querySelector(
          ".profile-fields textarea",
        ) as HTMLTextAreaElement
      ).value,
    ).toBe("Nova descrição");
    expect((fields[2]?.querySelector("input") as HTMLInputElement).value).toBe(
      "42",
    );
    expect(
      container
        .querySelector<HTMLButtonElement>("#profile-ageCategory-trigger")
        ?.getAttribute("data-value"),
    ).toBe("child");
  });

  it("provides a keyboard-oriented dark calendar hook for the draft birthday", async () => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () =>
      root?.render(<ProfileForm agent={agent} done={vi.fn()} />),
    );

    const trigger = container.querySelector<HTMLButtonElement>(
      ".date-picker-trigger",
    );
    if (trigger === null) throw new Error("Missing date picker trigger");
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
    await act(async () => trigger.click());
    const popover = container.querySelector<HTMLElement>(
      ".date-picker-popover",
    );
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    expect(trigger.getAttribute("aria-controls")).toBe(popover?.id);
    expect(popover?.getAttribute("role")).toBe("dialog");
    expect(
      container!.querySelector('[role="grid"] [aria-selected="true"]'),
    ).not.toBeNull();

    await act(async () =>
      container!
        .querySelector<HTMLButtonElement>('[aria-label="2020-01-15"]')
        ?.click(),
    );
    expect(trigger.textContent).toBe("15/01/2020");

    await act(async () => trigger.click());
    await act(async () =>
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })),
    );
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
    expect(document.activeElement).toBe(trigger);
  });

  it("jumps directly to a month and year without losing the day grid", async () => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () =>
      root?.render(<ProfileForm agent={agent} done={vi.fn()} />),
    );
    const trigger = container.querySelector<HTMLButtonElement>(
      ".date-picker-trigger",
    );
    if (trigger === null) throw new Error("Missing date picker trigger");
    await act(async () => trigger.click());
    const year =
      container.querySelector<HTMLInputElement>('[aria-label="Ano"]');
    if (year === null) throw new Error("Missing year control");
    change(year, "1995");
    expect(
      container.querySelector('[role="grid"]')?.getAttribute("aria-label"),
    ).toMatch(/2020/);
    await act(async () =>
      year.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Enter", bubbles: true }),
      ),
    );
    const month =
      container.querySelector<HTMLSelectElement>('[aria-label="Mês"]');
    if (month === null) throw new Error("Missing month control");
    await act(async () => {
      month.value = "6";
      month.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(
      container.querySelector('[role="grid"]')?.getAttribute("aria-label"),
    ).toMatch(/1995/);
    expect(container.querySelectorAll('[role="gridcell"]').length).toBe(31);
  });

  it("saves custom pronouns and optional human fields without requiring them", async () => {
    invoke.mockResolvedValue(undefined);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () =>
      root?.render(
        <ProfileForm agent={{ ...agent, species: "human" }} done={vi.fn()} />,
      ),
    );
    await chooseOption(container, "profile-pronouns", "custom");
    const customPronouns = Array.from(container.querySelectorAll("label"))
      .find((label) => label.textContent?.includes("Pronomes personalizados"))
      ?.querySelector("input") as HTMLInputElement;
    const gender = Array.from(container.querySelectorAll("label"))
      .find((label) => label.textContent?.includes("Gênero (opcional)"))
      ?.querySelector("input") as HTMLInputElement;
    const sexuality = Array.from(container.querySelectorAll("label"))
      .find((label) => label.textContent?.includes("Sexualidade (opcional)"))
      ?.querySelector("input") as HTMLInputElement;
    change(customPronouns, "ze/zir");
    change(gender, "  não binário  ");
    change(sexuality, "  bissexual e queer  ");
    expect(gender.value).toBe("  não binário  ");
    expect(sexuality.value).toBe("  bissexual e queer  ");
    const save = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent === "Salvar alterações",
    );
    await act(async () => save?.click());
    expect(invoke).toHaveBeenCalledWith(
      "update_agent_profile",
      expect.objectContaining({
        agent: expect.objectContaining({
          pronouns: "ze/zir",
          gender: "não binário",
          sexuality: "bissexual e queer",
        }),
      }),
    );
  });
});
