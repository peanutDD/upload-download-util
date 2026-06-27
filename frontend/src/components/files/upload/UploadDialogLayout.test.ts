import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const css = readFileSync(
  "src/components/files/upload/UploadDialog.css",
  "utf8",
);
const dialogSource = readFileSync(
  "src/components/files/upload/UploadDialog.tsx",
  "utf8",
);
const dropzoneSource = readFileSync(
  "src/components/files/upload/UploadDropzone.tsx",
  "utf8",
);
const urlUploadSource = readFileSync(
  "src/components/files/upload/UrlUploadForm.tsx",
  "utf8",
);
const tokensCss = readFileSync("src/styles/tokens.css", "utf8");

function ruleBody(selector: string) {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = css.match(new RegExp(`${escapedSelector}\\s*\\{([\\s\\S]*?)\\n\\}`));
  return match?.[1] ?? "";
}

function compact(cssRule: string) {
  return cssRule.replace(/\s+/g, " ").trim();
}

describe("UploadDialog mobile layout", () => {
  it("keeps a safe dynamic-viewport gap around the upload window on mobile browsers", () => {
    const backdrop = compact(ruleBody(".uploadDialogCyberBackdrop"));
    const surface = compact(ruleBody(".uploadDialogCyberSurface"));

    expect(backdrop).toContain("--upload-dialog-viewport-gap");
    expect(backdrop).toContain("--upload-dialog-grid-step: clamp(");
    expect(backdrop).toContain("--upload-dialog-backdrop-blur: clamp(");
    expect(backdrop).toContain("height: 100dvh");
    expect(backdrop).toContain("box-sizing: border-box");
    expect(backdrop).toContain(
      "backdrop-filter: blur(var(--upload-dialog-backdrop-blur)) saturate(1.24)",
    );
    expect(backdrop).toContain(
      "padding-top: calc(var(--upload-dialog-viewport-gap) + env(safe-area-inset-top))",
    );
    expect(backdrop).toContain(
      "padding-right: calc(var(--upload-dialog-viewport-gap) + env(safe-area-inset-right))",
    );
    expect(backdrop).toContain(
      "padding-bottom: calc(var(--upload-dialog-viewport-gap) + env(safe-area-inset-bottom))",
    );
    expect(backdrop).toContain(
      "padding-left: calc(var(--upload-dialog-viewport-gap) + env(safe-area-inset-left))",
    );
    expect(surface).toContain(
      "max-height: calc(100dvh - var(--upload-dialog-vertical-gap))",
    );
    expect(surface).toContain(
      "max-width: min(42rem, calc(100dvw - var(--upload-dialog-horizontal-gap)))",
    );
  });
});

describe("UploadDialog purple ready-state buttons", () => {
  it("scopes the bid-style hover treatment to purple mode when files are ready", () => {
    expect(dialogSource).toContain("data-ready-to-upload={hasFiles}");

    const compactCss = compact(css);

    expect(compactCss).toContain(
      '[data-theme="purple"] .uploadDialogCyberFooter[data-ready-to-upload="true"] .uploadDialogCyberPrimaryBtn:not(:disabled):hover',
    );
    expect(compactCss).toContain(
      ".purple .uploadDialogCyberFooter[data-ready-to-upload=\"true\"] .uploadDialogCyberPrimaryBtn:not(:disabled):hover",
    );
    expect(compactCss).toContain("background: linear-gradient(");
    expect(compactCss).toContain("rgba(var(--rgb-fuchsia-500), 0.96) 0%");
    expect(compactCss).toContain("rgba(var(--rgb-purple-500), 0.98) 52%");
    expect(compactCss).toContain("rgba(var(--rgb-purple-400), 0.96) 100%");

    expect(compactCss).toContain(
      '[data-theme="purple"] .uploadDialogCyberFooter[data-ready-to-upload="true"] .uploadDialogCyberSecondaryBtn:not(:disabled):hover',
    );
    expect(compactCss).toContain("background: rgba(var(--rgb-white), 0.02)");
    expect(compactCss).toContain("border-color: rgba(var(--rgb-fuchsia-500), 0.95)");

    [
      "--rgb-fuchsia-500",
      "--rgb-purple-500",
      "--rgb-purple-400",
      "--rgb-purple-950",
      "--rgb-white",
    ].forEach((token) => {
      expect(tokensCss).toContain(token);
    });
  });
});

describe("UploadDialog empty-state button palette", () => {
  it("gives cancel, attach, select files, and URL upload their own semantic classes", () => {
    expect(dialogSource).toContain("uploadDialogCancelBtn");
    expect(dialogSource).toContain("uploadDialogAttachBtn");
    expect(dropzoneSource).toContain("uploadDialogSelectFilesBtn");
    expect(urlUploadSource).toContain("uploadDialogUrlUploadBtn");
  });

  it("defines dark, light, and purple palettes for the empty upload state", () => {
    const compactCss = compact(css);

    [
      "--upload-empty-cancel-bg",
      "--upload-empty-cancel-border",
      "--upload-empty-attach-bg",
      "--upload-empty-attach-border",
      "--upload-select-files-bg",
      "--upload-select-files-border",
      "--upload-url-upload-bg",
      "--upload-url-upload-border",
    ].forEach((token) => {
      expect(compactCss).toContain(token);
    });

    expect(compactCss).toContain(
      '.uploadDialogCyberFooter[data-ready-to-upload="false"] .uploadDialogCancelBtn',
    );
    expect(compactCss).toContain(
      '.uploadDialogCyberFooter[data-ready-to-upload="false"] .uploadDialogAttachBtn:disabled',
    );
    expect(compactCss).toContain(".uploadDialogSelectFilesBtn");
    expect(compactCss).toContain(".uploadDialogUrlUploadBtn");

    expect(compactCss).toContain("[data-theme=\"light\"] .uploadDialogCyberSurface");
    expect(compactCss).toContain("[data-theme=\"purple\"] .uploadDialogCyberSurface");

    [
      "--rgb-emerald-500",
      "--rgb-malachite-500",
      "--rgb-cyan-400",
      "--rgb-purple-500",
      "--rgb-fuchsia-500",
      "--rgb-slate-950",
      "--rgb-white",
    ].forEach((token) => {
      expect(tokensCss).toContain(token);
    });
  });

  it("keeps the light select-files action clean and luminous", () => {
    const compactCss = compact(css);

    expect(compactCss).toContain("rgba(var(--rgb-white), 0.94)");
    expect(compactCss).toContain("rgba(var(--rgb-cyan-400), 0.28)");
    expect(compactCss).toContain("--upload-select-files-text: rgba(var(--rgb-slate-900), 0.92)");
    expect(compactCss).not.toContain(
      "--upload-select-files-bg: linear-gradient( 135deg, rgba(var(--rgb-slate-950), 0.94), rgba(var(--rgb-emerald-500), 0.78) )",
    );
  });
});
