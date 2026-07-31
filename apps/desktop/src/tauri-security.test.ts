import { describe, expect, it } from "vitest";
import config from "../src-tauri/tauri.conf.json";

describe("Tauri image policy", () => {
  it("allows bundled inline agent sprites in development and production", () => {
    expect(config.app.security.csp).toContain("img-src 'self' data:");
    expect(config.app.security.devCsp).toContain("img-src 'self' data:");
  });
});
