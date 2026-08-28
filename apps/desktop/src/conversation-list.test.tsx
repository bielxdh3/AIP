// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ConversationList } from "./App";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const current = [
  { id: "main", title: "Conversa inicial", isPinned: true },
  { id: "extra", title: "Conversa secundária", isPinned: false },
];
const archived = [{ id: "old", title: "Conversa arquivada", isPinned: false }];

let root: Root | undefined;
let container: HTMLDivElement | undefined;

function change(element: HTMLInputElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(
    Object.getPrototypeOf(element),
    "value",
  )?.set;
  setter?.call(element, value);
  element.dispatchEvent(new Event("input", { bubbles: true }));
}

afterEach(() => {
  if (root !== undefined) act(() => root?.unmount());
  container?.remove();
  root = undefined;
  container = undefined;
  vi.clearAllMocks();
});

describe("ConversationList regressions", () => {
  it("exposes scoped semantic hooks and accessible creation controls", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_agent_conversations")
        return Promise.resolve(current);
      if (command === "list_archived_agent_conversations")
        return Promise.resolve(archived);
      return Promise.resolve(undefined);
    });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);

    await act(async () => {
      root?.render(<ConversationList agentId="agent" changed={vi.fn()} />);
      await Promise.resolve();
    });

    const list = container.querySelector(".conversation-list");
    const input = container.querySelector<HTMLInputElement>(
      ".conversation-list-input",
    );
    expect(list?.getAttribute("role")).toBe("region");
    expect(list?.getAttribute("aria-label")).toBe("Conversas do agente");
    expect(container.querySelectorAll(".conversation-list-item")).toHaveLength(
      2,
    );
    expect(container.textContent).not.toContain("Conversa arquivada");
    expect(input?.type).toBe("text");
    expect(input?.getAttribute("aria-label")).toBe("Título da nova conversa");
    expect(container.querySelector(".conversation-list-create")).not.toBeNull();
    expect(
      container.querySelector(".conversation-archive-management"),
    ).not.toBeNull();
    expect(container.querySelectorAll(".conversation-actions")).toHaveLength(2);

    if (input === null) throw new Error("Missing conversation title input");
    change(input, "Nova conversa");
    const create = container.querySelector<HTMLButtonElement>(
      ".conversation-list-create",
    );
    if (create === null) throw new Error("Missing create button");
    await act(async () => create.click());
    expect(invoke).toHaveBeenCalledWith("create_agent_conversation", {
      agentId: "agent",
      title: "Nova conversa",
    });
  });

  it("opens archived management only on request", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "list_agent_conversations")
        return Promise.resolve(current);
      if (command === "list_archived_agent_conversations")
        return Promise.resolve(archived);
      return Promise.resolve(undefined);
    });
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);

    await act(async () => {
      root?.render(<ConversationList agentId="agent" changed={vi.fn()} />);
      await Promise.resolve();
    });

    const manage = container.querySelector<HTMLButtonElement>(
      ".conversation-archive-management",
    );
    if (manage === null) throw new Error("Missing archive management button");
    await act(async () => manage.click());
    expect(invoke).toHaveBeenCalledWith("list_archived_agent_conversations", {
      agentId: "agent",
    });
    expect(container.textContent).toContain("Conversa arquivada");
  });
});
