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

afterEach(() => {
  if (root !== undefined) act(() => root?.unmount());
  container?.remove();
  root = undefined;
  container = undefined;
  vi.clearAllMocks();
});

describe("ConversationList regressions", () => {
  it("requires explicit confirmation before deleting a conversation", async () => {
    invoke.mockImplementation((command: string) =>
      command === "list_agent_conversations"
        ? Promise.resolve(current)
        : Promise.resolve(undefined),
    );
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);

    await act(async () => {
      root?.render(<ConversationList agentId="agent" changed={vi.fn()} />);
      await Promise.resolve();
    });

    const deleteButton = Array.from(
      container.querySelectorAll<HTMLButtonElement>(".conversation-actions button"),
    ).find((button) => button.textContent === "Excluir");
    if (deleteButton === undefined) throw new Error("Missing delete button");
    await act(async () => deleteButton.click());
    const dialog = container.querySelector('[role="dialog"]');
    expect(dialog).not.toBeNull();
    expect(dialog?.textContent).toContain("Conversa inicial");
    expect(document.activeElement?.textContent).toBe("Cancelar");
    expect(invoke).not.toHaveBeenCalledWith("delete_agent_conversation", {
      agentId: "agent",
      conversationId: "main",
    });

    await act(async () =>
      dialog?.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
      ),
    );
    expect(container.querySelector('[role="dialog"]')).toBeNull();

    await act(async () => deleteButton.click());
    const confirm = Array.from(container.querySelectorAll("button")).find(
      (button) => button.textContent === "Excluir conversa",
    );
    if (confirm === undefined) throw new Error("Missing confirmation button");
    await act(async () => {
      confirm.click();
      await Promise.resolve();
    });
    expect(invoke).toHaveBeenCalledWith("delete_agent_conversation", {
      agentId: "agent",
      conversationId: "main",
    });
  });

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
    const onNewDraft = vi.fn();

    await act(async () => {
      root?.render(
        <ConversationList
          agentId="agent"
          changed={vi.fn()}
          onNewDraft={onNewDraft}
        />,
      );
      await Promise.resolve();
    });

    const list = container.querySelector(".conversation-list");
    expect(list?.getAttribute("role")).toBe("region");
    expect(list?.getAttribute("aria-label")).toBe("Conversas do agente");
    expect(list?.classList.contains("conversation-list")).toBe(true);
    expect(container.querySelectorAll(".conversation-list-item")).toHaveLength(
      2,
    );
    expect(container.textContent).not.toContain("Conversa arquivada");
    const create = container.querySelector<HTMLButtonElement>(
      ".conversation-list-create",
    );
    expect(create?.textContent).toBe("Nova conversa");
    expect(
      container.querySelector(".conversation-archive-management"),
    ).not.toBeNull();
    expect(container.querySelectorAll(".conversation-actions")).toHaveLength(2);

    if (create === null) throw new Error("Missing create button");
    await act(async () => create.click());
    expect(onNewDraft).toHaveBeenCalledOnce();
    expect(invoke).not.toHaveBeenCalledWith(
      "create_agent_conversation",
      expect.anything(),
    );
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

  it("notifies the parent before selecting an existing conversation", async () => {
    invoke.mockImplementation((command: string) =>
      command === "list_agent_conversations"
        ? Promise.resolve(current)
        : Promise.resolve(undefined),
    );
    const onSelectExisting = vi.fn();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () => {
      root?.render(
        <ConversationList
          agentId="agent"
          changed={vi.fn()}
          onSelectExisting={onSelectExisting}
        />,
      );
      await Promise.resolve();
    });

    await act(async () =>
      container
        ?.querySelector<HTMLButtonElement>(".conversation-list-select")
        ?.click(),
    );
    expect(onSelectExisting).toHaveBeenCalledOnce();
    expect(invoke).toHaveBeenCalledWith("set_active_agent_conversation", {
      agentId: "agent",
      conversationId: "main",
    });
  });

  it("closes the action menu before focusing the rename editor", async () => {
    invoke.mockImplementation((command: string) =>
      command === "list_agent_conversations"
        ? Promise.resolve(current)
        : Promise.resolve([]),
    );
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () => {
      root?.render(<ConversationList agentId="agent" changed={vi.fn()} />);
      await Promise.resolve();
    });

    const menu = container.querySelector(
      ".conversation-actions",
    ) as HTMLDetailsElement;
    await act(async () => menu.querySelector("summary")?.click());
    expect(menu.open).toBe(true);
    await act(async () =>
      Array.from(menu.querySelectorAll("button"))
        .find((button) => button.textContent === "Renomear")
        ?.click(),
    );
    expect(menu.open).toBe(false);
    expect(container.querySelector(".conversation-rename")).not.toBeNull();
    expect(
      container.querySelector(".conversation-rename-actions"),
    ).not.toBeNull();
    expect(document.activeElement).toBe(
      container.querySelector(".conversation-rename input"),
    );
  });
});
