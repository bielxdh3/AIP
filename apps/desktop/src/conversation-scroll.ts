export type ConversationScrollMetrics = {
  clientHeight: number;
  scrollHeight: number;
  scrollTop: number;
};

const BOTTOM_TOLERANCE_PX = 32;

export function isNearConversationBottom({
  clientHeight,
  scrollHeight,
  scrollTop,
}: ConversationScrollMetrics): boolean {
  return scrollHeight - scrollTop - clientHeight <= BOTTOM_TOLERANCE_PX;
}

export function shouldScrollConversationToBottom(
  conversationChanged: boolean,
  followsBottom: boolean,
): boolean {
  return conversationChanged || followsBottom;
}
