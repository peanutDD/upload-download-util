import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { validateTauriClientConfig } from "./check-tauri-client-config.mjs";

const readProjectFile = (path) => readFileSync(join(process.cwd(), path), "utf8");
const readJson = (path) => JSON.parse(readProjectFile(path));

describe("tauri client packaging config", () => {
  it("keeps Tauri packages and packaging invariants aligned with the approved baseline", () => {
    const packageJson = readJson("package.json");
    const versions = validateTauriClientConfig({
      packageJson,
      cargoToml: readProjectFile("src-tauri/Cargo.toml"),
      tauriConfig: readJson("src-tauri/tauri.conf.json"),
      androidConfig: readJson("src-tauri/tauri.android.conf.json"),
      capability: readJson("src-tauri/capabilities/default.json"),
    });

    expect(versions).toEqual({
      api: "^2.11.0",
      cli: "^2.11.0",
      tauri: "2.11.0",
      tauriBuild: "2.6.0",
    });
    expect(packageJson.scripts["macos:dmg:ci"]).toBe("CI=true tauri build --bundles dmg");
  });
});
