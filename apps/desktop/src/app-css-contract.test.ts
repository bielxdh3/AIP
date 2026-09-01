import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";

const appCss = readFileSync(new URL("./App.css", import.meta.url), "utf8");
const rules = Array.from(appCss.matchAll(/([^{}]+)\{([^{}]*)\}/g), (match) => ({
  selector: match[1] ?? "",
  body: match[2] ?? "",
}));

function rulesContaining(selector: string) {
  return rules.filter((rule) => rule.selector.includes(selector));
}

function rulesForSelector(selector: string) {
  return rules.filter((rule) =>
    rule.selector.split(",").some((candidate) => candidate.trim() === selector),
  );
}

describe("App.css semantic foreground contract", () => {
  it("keeps neutral hover states readable and primary fills intentional", () => {
    const neutralSelectors = [
      ".memory-card-actions button:hover",
      ".memory-composer-actions button:hover",
      ".pixel-editor button:hover:not(:disabled)",
      ".aip-modal-actions button:hover",
      ".aip-modal-actions button:focus-visible",
    ];

    for (const selector of neutralSelectors) {
      const matchingRules = rulesContaining(selector);
      expect(matchingRules.length, selector).toBeGreaterThan(0);
      for (const rule of matchingRules) {
        expect(rule.body).not.toMatch(
          /color:\s*var\(--color-(?:on-primary|accent-ink)\)/,
        );
      }
    }

    for (const rule of rules.filter((candidate) =>
      /color:\s*var\(--color-(?:on-primary|accent-ink)\)/.test(candidate.body),
    )) {
      expect(rule.body).toMatch(
        /background:\s*var\(--color-(?:primary|accent|accent-strong)\)/,
      );
    }

    for (const selector of [
      ".memory-composer-actions button:first-child",
      ".profile-actions button",
      ".aip-button-primary",
    ]) {
      const filledRule = rulesContaining(selector).find((rule) =>
        /background:\s*var\(--color-(?:primary|accent|accent-strong)\)/.test(
          rule.body,
        ),
      );
      expect(filledRule, selector).toBeDefined();
      expect(filledRule?.body).toMatch(
        /color:\s*var\(--color-(?:on-primary|accent-ink)\)/,
      );
    }
  });

  it("keeps the outer conversation surface unraised", () => {
    const conversationSurfaceRules = rulesForSelector(".conversation-surface");
    expect(conversationSurfaceRules.length).toBeGreaterThan(0);

    for (const rule of conversationSurfaceRules) {
      expect(rule.body).not.toMatch(/(?:background|border|box-shadow)\s*:/);
    }

    for (const selector of [".composer", ".chat-message"]) {
      expect(
        rulesForSelector(selector).some((rule) =>
          /background\s*:\s*var\(--color-surface(?:-raised)?\)/.test(rule.body),
        ),
        selector,
      ).toBe(true);
    }
  });
});
