import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const __dirname = dirname(fileURLToPath(import.meta.url));

function readCssCustomProperties(fileName: string, selectorPattern: RegExp) {
  const css = readFileSync(resolve(__dirname, fileName), "utf8");
  const selector = selectorPattern.exec(css);
  expect(selector).not.toBeNull();

  if (!selector) {
    throw new Error(`Missing CSS selector: ${selectorPattern.source}`);
  }

  const ruleBody = css.slice(selector.index + selector[0].length).match(/^[\s\S]*?(?=\s*})/)?.[0];
  expect(ruleBody).toBeDefined();

  const properties = new Map<string, string>();
  for (const [, name, value] of (ruleBody ?? "").matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) {
    properties.set(name, value.trim());
  }

  return properties;
}

function readToken(tokens: Map<string, string>, name: string) {
  const value = tokens.get(name);
  expect(value).toBeDefined();
  return value ?? "";
}

function expectTokenValueToUseRgbTokens(value: string, rgbTokens: string[]) {
  for (const rgbToken of rgbTokens) {
    expect(value).toContain(`rgba(var(${rgbToken})`);
  }
}

describe("purple theme tokens", () => {
  it("uses a Nebula Bloom page surface with star-dust accents", () => {
    const tokens = readCssCustomProperties("tokens.css", /\[data-theme="purple"\]\s*,\s*\.purple\s*\{/);
    const surface = readToken(tokens, "--surface-page-gradient");

    expect(surface).toMatch(/\bradial-gradient\s*\(/);
    expectTokenValueToUseRgbTokens(surface, [
      "--rgb-purple-500",
      "--rgb-fuchsia-500",
      "--rgb-rose-400",
      "--rgb-cyan-400",
      "--rgb-white",
    ]);
  });

  it("replaces green chrome accents with violet, rose, and cyan star light", () => {
    const tokens = readCssCustomProperties("tokens.css", /\[data-theme="purple"\]\s*,\s*\.purple\s*\{/);
    const navBottomLine = readToken(tokens, "--nav-bottom-line");
    const footerShimmer = readToken(tokens, "--footer-shimmer-bg");
    const ctaPrimary = readToken(tokens, "--cta-primary-bg");

    expect(navBottomLine).not.toContain("--rgb-emerald");
    expect(footerShimmer).not.toContain("--rgb-emerald");
    expect(ctaPrimary).not.toContain("--rgb-emerald");
    expectTokenValueToUseRgbTokens(navBottomLine, ["--rgb-fuchsia-500", "--rgb-rose-400", "--rgb-cyan-400"]);
    expectTokenValueToUseRgbTokens(footerShimmer, ["--rgb-fuchsia-500", "--rgb-rose-400", "--rgb-cyan-400"]);
    expectTokenValueToUseRgbTokens(ctaPrimary, ["--rgb-purple-500", "--rgb-fuchsia-500", "--rgb-rose-400"]);
  });

  it("gives shared glass surfaces the dreamier purple nebula palette", () => {
    const tokens = readCssCustomProperties("tokens.css", /\[data-theme="purple"\]\s*,\s*\.purple\s*\{/);
    const filelistToolbar = readToken(tokens, "--filelist-toolbar-btn-bg");
    const uploadGlow = [
      readToken(tokens, "--upload-backdrop-glow-1"),
      readToken(tokens, "--upload-backdrop-glow-2"),
      readToken(tokens, "--upload-backdrop-glow-3"),
    ].join(" ");
    const dialogPrimary = readToken(tokens, "--dialog-primary-btn-bg");

    expectTokenValueToUseRgbTokens(filelistToolbar, ["--rgb-fuchsia-500", "--rgb-rose-400"]);
    expectTokenValueToUseRgbTokens(uploadGlow, ["--rgb-purple-500", "--rgb-fuchsia-500", "--rgb-rose-400"]);
    expectTokenValueToUseRgbTokens(dialogPrimary, ["--rgb-purple-500", "--rgb-fuchsia-500", "--rgb-rose-400"]);
  });
});
