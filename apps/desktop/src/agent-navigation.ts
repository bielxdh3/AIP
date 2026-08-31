import { invoke } from "@tauri-apps/api/core";

export const OPEN_AGENT_CONVERSATIONS_EVENT = "open-agent-conversations";

export type OpenAgentConversationsPayload = {
  agentId: string;
  conversationId: string;
};

export function openAgentConversations(
  agentId: string,
  conversationId?: string,
) {
  return invoke(
    "open_agent_conversations",
    conversationId === undefined ? { agentId } : { agentId, conversationId },
  );
}
