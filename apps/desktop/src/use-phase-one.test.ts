import { describe, expect, it } from "vitest";
import { loadIsCurrent } from "./use-phase-one";

describe("Phase One load revisions", () => {
  it("ignores a load that began before an applied stream event", () => {
    const startedRevision = 1;
    const revisionAfterStreamEvent = startedRevision + 1;

    expect(loadIsCurrent(startedRevision, revisionAfterStreamEvent)).toBe(
      false,
    );
    expect(
      loadIsCurrent(revisionAfterStreamEvent, revisionAfterStreamEvent),
    ).toBe(true);
  });
});
