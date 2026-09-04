import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";

const appCss = readFileSync(new URL("./App.css", import.meta.url), "utf8");
type CssRule = { selector: string; body: string };

function matchingBrace(source: string, openingBrace: number) {
  let depth = 0;
  for (let index = openingBrace; index < source.length; index += 1) {
    if (source[index] === "{") depth += 1;
    if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) return index;
    }
  }
  return source.length;
}

function parseRules(source: string, start = 0, end = source.length): CssRule[] {
  const parsed: CssRule[] = [];
  let cursor = start;

  while (cursor < end) {
    const openingBrace = source.indexOf("{", cursor);
    if (openingBrace === -1 || openingBrace >= end) break;
    const closingBrace = Math.min(matchingBrace(source, openingBrace), end);
    const selector = source.slice(cursor, openingBrace).trim();
    const body = source.slice(openingBrace + 1, closingBrace);

    if (selector.startsWith("@")) {
      parsed.push(...parseRules(source, openingBrace + 1, closingBrace));
    } else if (selector.length > 0) {
      parsed.push({ selector, body });
    }

    cursor = closingBrace + 1;
  }

  return parsed;
}

const rules = parseRules(appCss.replace(/\/\*[\s\S]*?\*\//g, ""));

function rulesContaining(selector: string) {
  return rules.filter((rule) => rule.selector.includes(selector));
}

function rulesForSelector(selector: string) {
  return rules.filter((rule) =>
    rule.selector.split(",").some((candidate) => candidate.trim() === selector),
  );
}

function declarations(rule: CssRule) {
  return Array.from(
    rule.body.matchAll(/([\w-]+)\s*:\s*([^;]+)/g),
    (match) => [match[1]?.trim() ?? "", match[2]?.trim() ?? ""] as const,
  );
}

function targetsAssistantMessage(selector: string) {
  return selector.split(",").some((candidate) => {
    const lastCompound =
      candidate
        .trim()
        .split(/\s+|>|\+|~/)
        .at(-1) ?? "";
    return [
      ".chat-message",
      ".chat-message.agent",
      ".chat-message:not(.user)",
    ].includes(lastCompound);
  });
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

  it("keeps transcript layers open and the user prompt as the only bubble", () => {
    const conversationSurfaceRules = rulesForSelector(".conversation-surface");
    expect(conversationSurfaceRules.length).toBeGreaterThan(0);

    for (const rule of conversationSurfaceRules) {
      expect(rule.body).not.toMatch(/(?:background|border|box-shadow)\s*:/);
    }

    const assistantRules = rules.filter((rule) =>
      targetsAssistantMessage(rule.selector),
    );
    expect(assistantRules.length).toBeGreaterThan(0);
    for (const rule of assistantRules) {
      for (const [property, value] of declarations(rule)) {
        if (property === "background" || property === "background-color") {
          expect(value, `${rule.selector} ${property}`).toMatch(
            /^(none|transparent)$/,
          );
        }
        if (
          property === "border" ||
          property === "border-color" ||
          property === "border-style" ||
          property === "border-width"
        ) {
          expect(value, `${rule.selector} ${property}`).toMatch(
            /^(0|0px|none|transparent)$/,
          );
        }
        if (property === "border-radius" || property === "padding") {
          expect(value, `${rule.selector} ${property}`).toMatch(/^(0|0px)$/);
        }
        if (property === "box-shadow") {
          expect(value, `${rule.selector} ${property}`).toMatch(/^(0|none)$/);
        }
      }
    }

    expect(
      assistantRules.some((rule) =>
        declarations(rule).some(
          ([property, value]) =>
            property === "background" && value === "transparent",
        ),
      ),
    ).toBe(true);

    const userRules = rulesForSelector(".chat-message.user");
    expect(userRules.length).toBeGreaterThan(0);
    expect(
      userRules.some((rule) =>
        declarations(rule).some(
          ([property, value]) =>
            property === "background" &&
            value === "var(--color-surface-raised)",
        ),
      ),
    ).toBe(true);
    expect(
      userRules.some((rule) =>
        declarations(rule).some(
          ([property, value]) =>
            property === "border" && value.startsWith("1px solid"),
        ),
      ),
    ).toBe(true);
    for (const expected of [
      ["width", "fit-content"],
      ["max-width", "min(720px, 100%)"],
      ["max-width", "100%"],
      ["border-radius", "var(--radius-lg)"],
      ["justify-self", "end"],
    ] as const) {
      expect(
        userRules.some((rule) =>
          declarations(rule).some(
            ([property, value]) =>
              property === expected[0] && value === expected[1],
          ),
        ),
        `${expected[0]} ${expected[1]}`,
      ).toBe(true);
    }

    expect(
      rulesForSelector(".composer").some((rule) =>
        declarations(rule).some(
          ([property, value]) =>
            property === "background" && value === "var(--color-surface)",
        ),
      ),
    ).toBe(true);
    expect(
      rulesForSelector(".message-history").some((rule) =>
        declarations(rule).some(
          ([property, value]) =>
            property === "background" && value === "var(--color-background)",
        ),
      ),
    ).toBe(true);
  });
});
