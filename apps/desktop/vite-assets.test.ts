import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import viteConfig from "./vite.config";

describe("desktop public assets", () => {
  it("maps the Vite public directory to the authoritative Tauri icon", async () => {
    const config = await (typeof viteConfig === "function"
      ? viteConfig({ command: "build", mode: "test" })
      : viteConfig);
    const publicDir = config.publicDir;
    expect(publicDir).toBe("src-tauri/icons");
    expect(existsSync(resolve(process.cwd(), publicDir, "icon.ico"))).toBe(
      true,
    );
  });
});
