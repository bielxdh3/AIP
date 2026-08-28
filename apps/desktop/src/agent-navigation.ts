import { invoke } from "@tauri-apps/api/core";

export const OPEN_AGENT_CONVERSATIONS_EVENT = "open-agent-conversations";

export function openAgentConversations(agentId: string) {
  return invoke("open_agent_conversations", { agentId });
}
