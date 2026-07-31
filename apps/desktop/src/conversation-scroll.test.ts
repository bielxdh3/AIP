import { describe, expect, it } from "vitest";
import {
  isNearConversationBottom,
  shouldScrollConversationToBottom,
} from "./conversation-scroll";

describe("conversation scroll behavior", () => {
  it("opens and switches conversations at the newest message", () => {
    expect(shouldScrollConversationToBottom(true, false)).toBe(true);
  });

  it("follows new messages only while already near the bottom", () => {
    expect(
      isNearConversationBottom({
        scrollTop: 568,
        scrollHeight: 1_000,
        clientHeight: 400,
      }),
    ).toBe(true);
    expect(shouldScrollConversationToBottom(false, true)).toBe(true);
  });

  it("does not force a user who scrolled upward back to the bottom", () => {
    expect(
      isNearConversationBottom({
        scrollTop: 420,
        scrollHeight: 1_000,
        clientHeight: 400,
      }),
    ).toBe(false);
    expect(shouldScrollConversationToBottom(false, false)).toBe(false);
  });
});
