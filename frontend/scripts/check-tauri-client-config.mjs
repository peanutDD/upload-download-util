export function validateTauriClientConfig({ packageJson, cargoToml, tauriConfig, androidConfig, capability }) {
  const api = packageJson.dependencies?.["@tauri-apps/api"];
  const cli = packageJson.devDependencies?.["@tauri-apps/cli"];
  const tauri = cargoToml.match(/^tauri\s*=\s*\{?\s*version\s*=\s*"([^"]+)"/m)?.[1];
  const tauriBuild = cargoToml.match(/^tauri-build\s*=\s*\{?\s*version\s*=\s*"([^"]+)"/m)?.[1];

  if (api !== "^2.11.0") {
    throw new Error(`@tauri-apps/api must be ^2.11.0, got ${api}`);
  }
  if (cli !== "^2.11.0") {
    throw new Error(`@tauri-apps/cli must be ^2.11.0, got ${cli}`);
  }
  if (tauri !== "2.11.0") {
    throw new Error(`tauri crate must be 2.11.0, got ${tauri}`);
  }
  if (tauriBuild !== "2.6.0") {
    throw new Error(`tauri-build crate must be 2.6.0, got ${tauriBuild}`);
  }
  if (!tauriConfig.app?.security?.capabilities?.includes("default")) {
    throw new Error("desktop capability default missing");
  }
  if (androidConfig.identifier !== "com.uploaddownloadutil.mobile") {
    throw new Error("android identifier changed");
  }
  if (!capability.permissions?.includes("core:default")) {
    throw new Error("core:default capability missing");
  }

  return { api, cli, tauri, tauriBuild };
}
