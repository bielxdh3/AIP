import { describe, expect, it, vi } from "vitest";
import { openAgentConversations } from "./agent-navigation";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("agent conversation navigation", () => {
  it("opens the selected agent conversation surface", async () => {
    invoke.mockResolvedValue(undefined);
    await openAgentConversations("agt_luma_provisional");
    expect(invoke).toHaveBeenCalledWith("open_agent_conversations", {
      agentId: "agt_luma_provisional",
    });
  });

  it("forwards the current conversation when opening full chat", async () => {
    invoke.mockResolvedValue(undefined);
    await openAgentConversations("agt_luma_provisional", "conversation-luma-2");
    expect(invoke).toHaveBeenCalledWith("open_agent_conversations", {
      agentId: "agt_luma_provisional",
      conversationId: "conversation-luma-2",
    });
  });
});
