# Tauri Client Refresh Evidence

日期：2026-05-05
分支：`codex/tauri-client-refresh`
工作区：`/Users/tyone/.codex/worktrees/tauri-client-refresh/upload-download-util`

## 版本更新

- `@tauri-apps/api`: `^2.10.1` -> `^2.11.0`
- `@tauri-apps/cli`: `^2.10.1` -> `^2.11.0`
- `tauri`: `2.10.3` -> `2.11.0`
- `tauri-build`: `2.5.6` -> `2.6.0`
- 新增 `frontend` script: `macos:dmg:ci = CI=true tauri build --bundles dmg`
- 新增 `frontend` script: `check:tauri-client`

## 验证命令

所有通过：

```bash
cd frontend
npm run check:tauri-client
npm run lint
npx tsc -b --noEmit
npm run test
npm run build
node scripts/check-bundle-size.mjs
npx tauri info
CI=true npx tauri build --bundles dmg
npx tauri build --bundles app
```

```bash
cd frontend/src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

说明：本机默认 Node 为 `23.11.0`，`canvas@2.11.2` 没有对应预编译包且本机缺 `pkg-config`，因此本次本地依赖安装使用 Node 22 + `npm ci --ignore-scripts`。CI 的 Node 22 与 system dependencies 仍是权威基线。

## macOS 产物

- DMG: `frontend/src-tauri/target/release/bundle/dmg/UploadDownloadUtil_0.1.0_aarch64.dmg`
- DMG size: `6.0M`
- DMG SHA-256: `65a8ca87cf4c225be4f2400adf1b4f4f3fc84b0ec831ee9185c52d327d52bc0e`
- DMG format: `UDZO`
- DMG verify: `hdiutil verify` reported checksum valid.
- App bundle: `frontend/src-tauri/target/release/bundle/macos/UploadDownloadUtil.app`
- App bundle size: `6.9M`
- App bundle files:
  - `Contents/Info.plist`
  - `Contents/MacOS/app`
  - `Contents/Resources/icon.icns`

## Android 状态

Android client was not generated because the local Android SDK is incomplete.

`npm run android:doctor` output:

```text
JAVA: OK (openjdk version "25.0.2" 2026-01-20 LTS)
ADB: 缺失
ANDROID_SDK_ROOT: /Users/tyone/Library/Android/sdk
platform-tools: 缺失
build-tools: 缺失
platforms: 缺失
ndk: 缺失（Tauri Android 必需）
cmdline-tools: 缺失（建议安装）
Rust Android targets:
aarch64-linux-android
armv7-linux-androideabi
i686-linux-android
x86_64-linux-android
```

Required before Android package generation:

- Android SDK `platform-tools`
- Android SDK `build-tools`
- Android SDK `platforms`
- Android SDK `cmdline-tools`
- Android NDK

## DMG Headless Finding

Plain `npx tauri build` generated the release binary and `.app`, then failed during DMG bundling because `bundle_dmg.sh` entered the Finder AppleScript step. Re-running with `CI=true npx tauri build --bundles dmg` succeeded and generated the verified DMG. See `docs/constraints/C-036-tauri-dmg-headless-ci.md`.
