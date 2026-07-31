import { describe, expect, it, vi } from "vitest";
import { createListenerRegistration } from "./listener-lifecycle";

describe("listener lifecycle", () => {
  it("cleans an installed listener exactly once", () => {
    const cleanup = vi.fn();
    const registration = createListenerRegistration();
    registration.register(cleanup);
    registration.dispose();
    registration.dispose();
    expect(cleanup).toHaveBeenCalledTimes(1);
  });

  it("immediately cleans a listener resolved after unmount", () => {
    const cleanup = vi.fn();
    const registration = createListenerRegistration();
    registration.dispose();
    registration.register(cleanup);
    expect(cleanup).toHaveBeenCalledTimes(1);
  });
});
