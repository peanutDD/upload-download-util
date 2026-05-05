# C-036 - Tauri DMG Headless CI Builds

版本：v1.0 | 最后更新：2026-05-05

## 约束

在 Codex、CI、SSH 或其他非交互式 macOS 会话中生成 Tauri DMG 时，必须使用 CI/headless 路径：

```bash
cd frontend
CI=true npx tauri build --bundles dmg
```

本仓库固定入口：

```bash
cd frontend
npm run macos:dmg:ci
```

## 原因

Tauri 的 DMG bundler 会调用 `create-dmg`，默认会运行 Finder AppleScript 来调整 DMG 背景和图标位置。在非交互式会话中该 AppleScript 可能挂住或失败，导致 `bundle_dmg.sh` 退出且只留下 `rw.*.dmg` 临时镜像。

设置 `CI=true` 后，Tauri 会生成可安装 DMG，并跳过 Finder 美化步骤。产物缺少自定义图标位置/背景，但安装语义不变。

## 复发信号

- `npx tauri build` 输出 `Running bundle_dmg.sh` 后长时间无输出。
- `target/release/bundle/*` 中只剩 `rw.*.dmg`，没有最终 `UploadDownloadUtil_*.dmg`。
- `hdiutil info` 显示临时读写镜像仍挂载在 `/Volumes/dmg.*`。

## 处理

1. 清理挂住的 `osascript`/`bundle_dmg.sh` 进程。
2. 通过 `hdiutil detach` 卸载残留 `/dev/disk*` 或 `/Volumes/dmg.*`。
3. 重新运行 `npm run macos:dmg:ci`。
4. 用 `hdiutil verify <dmg>` 和 `shasum -a 256 <dmg>` 记录证据。
