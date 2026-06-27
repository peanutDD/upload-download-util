import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const __dirname = dirname(fileURLToPath(import.meta.url));

function readCssCustomProperties(selectorPattern: RegExp) {
  const css = readFileSync(resolve(__dirname, "tokens.css"), "utf8");
  const selector = selectorPattern.exec(css);
  expect(selector).not.toBeNull();

  if (!selector) {
    throw new Error(`Missing CSS selector: ${selectorPattern.source}`);
  }

  const ruleBody = css
    .slice(selector.index + selector[0].length)
    .match(/^[\s\S]*?(?=\s*})/)?.[0];
  expect(ruleBody).toBeDefined();

  const properties = new Map<string, string>();
  for (const [, name, value] of (ruleBody ?? "").matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) {
    properties.set(name, value.trim());
  }

  return properties;
}

function readToken(tokens: Map<string, string>, name: string) {
  const value = tokens.get(name);
  expect(value, name).toBeDefined();
  return value ?? "";
}

describe("terminal theme tokens", () => {
  it("defines blackglass terminal tokens for core surfaces and controls", () => {
    const tokens = readCssCustomProperties(/\[data-theme="terminal"\],\s*\.terminal\s*\{/);

    expect(readToken(tokens, "--surface-page-gradient")).toContain("rgba(var(--rgb-cyan-400)");
    expect(readToken(tokens, "--surface-page-gradient")).toContain("rgba(var(--rgb-emerald-400)");
    expect(readToken(tokens, "--surface-page-gradient")).toContain("rgb(var(--rgb-slate-950))");
    expect(readToken(tokens, "--color-border-medium")).toContain("rgba(var(--rgb-cyan-400)");
    expect(readToken(tokens, "--color-focus-ring")).toContain("rgba(var(--rgb-emerald-300)");
    expect(readToken(tokens, "--glass-bg-strong")).toContain("rgba(var(--rgb-slate-950)");

    expect(readToken(tokens, "--nav-surface-bg")).toContain("rgba(var(--rgb-slate-950)");
    expect(readToken(tokens, "--nav-btn-border")).toContain("rgba(var(--rgb-cyan-400)");
    expect(readToken(tokens, "--filelist-page-bg")).toContain("rgb(var(--rgb-slate-950))");
    expect(readToken(tokens, "--file-card-border")).toContain("rgba(var(--rgb-cyan-400)");
    expect(readToken(tokens, "--upload-backdrop")).toContain("rgba(var(--rgb-slate-950)");
    expect(readToken(tokens, "--upload-progress-bar-bg")).toContain("rgba(var(--rgb-cyan-400)");
    expect(readToken(tokens, "--preview-purple-rgb")).toBe("var(--rgb-cyan-400)");
    expect(readToken(tokens, "--settings-surface-bg")).toContain("rgba(var(--rgb-slate-950)");
    expect(readToken(tokens, "--settings-action-bg")).toContain("rgba(var(--rgb-cyan-400)");
    expect(readToken(tokens, "--trash-page-bg")).toContain("rgba(var(--rgb-emerald-400)");
    expect(readToken(tokens, "--trash-tech-scanline")).toContain("rgba(var(--rgb-emerald-300)");
  });

  it("keeps terminal motion lightweight and CSS-only", () => {
    const css = readFileSync(resolve(__dirname, "tokens.css"), "utf8");

    expect(css).toContain("@keyframes terminal-glow-pulse");
    expect(css).toContain("prefers-reduced-motion: no-preference");
    expect(css).toContain("--terminal-scanline-bg");
  });
});
