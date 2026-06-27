# CHANGELOG

本文档记录项目的重要变更，按时间倒序排列。

---

## [未发布] — 2026 年（当前会话）

### 🐛 `/files` 路由刷新滚动位置恢复（2026-05-06）

- 修复刷新浏览器后 `/files` 文件浏览页强制回到顶部、丢失浏览位置的问题。
- 删除 `frontend/src/router/ScrollRestoration.tsx#scopeFor()` 中对 `/files` 的路径排除，让主文件浏览页与 `/settings` 共享同一套 history-entry + URL fallback 持久化。
- `restore(y)` 对非零目标改为 RAF 重试循环（≤120 帧 ≈ 2s，容差 ±2px），首帧文档高度不足时持续重试；`y === 0` 保留原双 RAF 快速路径。
- 监听 `wheel` / `touchmove` / `keydown`，用户一旦手动滚动立即放弃恢复，绝不抢夺用户控制权。
- 与 `useFileListScrollRestoration` 的「按需补页」恢复路径互不冲突：路由层只负责 window scroll；列表层继续按 `hasMore` 拉页。
- 新增两条红绿回归测试：`/files` 同 history key 刷新、`key` 变化刷新；`vitest run src/router/ScrollRestoration.test.tsx` 6/6 通过。
- 扩展 `docs/constraints/C-036-navigation-scroll-restoration.md`：禁止 `scopeFor()` 路径白/黑名单，强制非零目标走 RAF 重试 + 用户输入退出策略。
- 新增 `docs/exec-plans/2026-05-06-files-route-refresh-scroll.{md,json}`。

### 🐛 虚拟列表刷新滚动位置冲突（2026-05-06）

- 修复进入文件夹刷新后滚动位置丢失：延长 `RESTORE_MAX_ATTEMPTS` 从 60 到 120（约 2 秒），给虚拟列表挂载和 `loadMore` 补页足够时间。
- 根因：`ScrollRestoration` 的 RAF 重试预算（1s）不足，虚拟列表 `prefixSums` 计算和 `loadMore` 补页需要额外时间。
- 解决：`RESTORE_MAX_ATTEMPTS = 120`，帧预算延长至约 2 秒，足够虚拟列表挂载 + `loadMore` 补页。
- `tsc -b`、`eslint` 通过。

### Settings 页面外观与体验优化（2026-05-04）

- 新增 Settings 本地 UI helper，统一账户、密码、API Token 表单的 label、input、error、helper、button、panel 样式。
- 设置页快速导航改为数据驱动渲染，并改善桌面/移动端的按钮排列与 focus ring。
- 修复窄屏下邮箱输入 + `Get code` 按钮、API Token 创建标题区的挤压风险。
- Appearance 区块使用 Settings 语义 panel token，选中态更清晰且保持 `dark` / `light` / `purple` 主题值不变。
- 新增 Settings 回归测试，覆盖主题切换、邮箱变更验证码展示、API Token 创建 payload、删除确认打开前不触发删除。
- 新增 `docs/design-docs/2026-05-04-settings-page-optimization.md` 与 `docs/exec-plans/2026-05-04-settings-page-optimization.md`。

### 上传链路完整性补强（2026-05-04）

- 普通上传、秒传、分片完成统一校验 `folder_id` 是否属于当前用户，防止跨用户写入文件夹。
- 后端上传 handler 集成测试增加 `serial_test` 隔离，避免共享测试数据库并发清理导致 upload session / user FK 抖动。
- 前端分片上传支持真正断点续传：本地 session 绑定文件指纹、大小、修改时间、MIME、folder 和文件名，复用前先查询后端 status。
- 每个分片上传发送 `X-Part-SHA256`，后端在记录 uploaded part 前校验 SHA 和该分片期望字节数。
- 秒传跨用户复制改为 storage 层 copy：Local 用文件系统 copy，S3 用 `copy_object`，避免大文件读入内存。
- 分片 complete 成功/失败记录 `chunked_upload_complete` metrics。
- 删除未使用的 `useFileUpload` hook，并将取消与 folder 传递回归测试集中到实际 `UploadDialog` controller。
- 新增 `docs/design-docs/upload-completeness-hardening.md`、`docs/constraints/C-023-upload-resume-integrity.md` 和本任务 exec plan。

### 🐛 图片预览放大拖动（2026-05-03）

- 图片预览放大到 `1x` 以上后，支持通过鼠标、触摸或笔拖动平移，查看被预览框裁掉的边缘区域。
- 缩回 `1x`、切换预览文件、点击 Reset 时会同步清除平移偏移，避免下一张图继承上一张图的位置。
- 新增 `useImagePan` 与 `ImagePreview` 回归测试，并记录永久约束 `docs/constraints/C-019-image-preview-pan-when-zoomed.md`。

### 📚 文档统一整理（2026-05-01）

#### 整理目的

项目文档分散在多个文件中，存在内容重复、结构不清晰的问题。本次整理旨在：
- 统一文档结构，便于快速查找
- 合并重复内容，消除冗余
- 建立文档中心入口

#### 整理内容

**1. 文档中心更新**
- 更新 `docs/README.md` 作为统一文档入口，增加「已归档文档」章节，记录整合情况
- 优化文档结构展示，添加清晰的分类导航
- 更新版本号至 v2.3

**2. 高并发优化文档整合**
- 创建 `docs/HIGH_CONCURRENCY.md`：整合「高并发.md」和「前端降低后端并发压力方案.md」的内容
- 统一前后端高并发优化方案，包含：
  - 前端层：防抖/节流、请求去重、虚拟列表、缓存策略、并发控制
  - 后端层：架构优化、上传/下载优化、列表/搜索优化
  - 监控与可观测性、实施路线图

**3. 功能开关文档整合**
- 创建 `docs/FEATURE_FLAGS.md`：整合「开关矩阵与接口边界.md」的内容
- 定义完整的 Feature Flags 矩阵和 Provider 接口边界

**4. 代码审查文档整合**
- 扩展 `docs/CODE_REVIEW_GUIDE.md`，整合以下文档内容：
  - 开始使用指南（原 CODE_REVIEW_START.md）
  - 操作指南（原 CODE_REVIEW_USAGE.md）
  - 审查报告模板（原 CODE_REVIEW_REPORT.md）
- 添加章节：§12 开始使用、§13 操作指南、§14 审查报告模板

#### 文档结构优化

| 原文档 | 状态 | 整合到 |
|--------|------|--------|
| `高并发.md` | 保留 | `HIGH_CONCURRENCY.md` |
| `前端降低后端并发压力方案.md` | 保留 | `HIGH_CONCURRENCY.md` |
| `开关矩阵与接口边界.md` | 保留 | `FEATURE_FLAGS.md` |
| `CODE_REVIEW_START.md` | 保留 | `CODE_REVIEW_GUIDE.md` §12 |
| `CODE_REVIEW_USAGE.md` | 保留 | `CODE_REVIEW_GUIDE.md` §13 |
| `CODE_REVIEW_REPORT.md` | 保留 | `CODE_REVIEW_GUIDE.md` §14 |

> **说明**：为保持向后兼容性，原有文档均保留，新内容优先在整合后的文档中维护。

---

### 🧱 前端组件代码拆分（2026-04-30）

#### 问题背景

前端代码中存在多个超大文件（>500 行），职责混杂，不符合单一职责原则，难以维护和扩展：
- `UploadDialog.tsx` - 909 行（上传弹窗、拖拽、URL 上传、进度列表混杂）
- `FilePreviewContent.tsx` - 650+ 行（图片、视频、音频、PDF、Markdown 预览混杂）
- `FileListContent.tsx` - 1000+ 行（列表、分组、选择、操作混杂）

#### 拆分策略

采用渐进式重构，按功能领域拆分为独立组件，保持视觉风格和功能完全不变。

#### 新增组件

**上传模块** (`frontend/src/components/files/upload/`)
- `UploadDropzone.tsx` - 拖拽上传组件（含文件选择、拖拽状态、视觉效果）
- `UrlUploadForm.tsx` - URL 上传表单组件（含 URL 验证、下载、错误处理）
- `UploadProgressList.tsx` - 上传进度列表组件（含统计信息、警告、文件项）

**预览模块** (`frontend/src/components/files/preview/`)
- `ImagePreview.tsx` - 图片预览组件（含加载状态、错误处理）
- `AudioPreview.tsx` - 音频预览组件（含播放器控件）

**列表模块** (`frontend/src/components/files/list/`)
- `GroupSelectCheckbox.tsx` - 分组全选复选框组件（含全选/取消全选/混合状态）

#### 拆分效果

| 文件 | 拆分前 | 拆分后 | 减少 |
|------|--------|--------|------|
| `UploadDialog.tsx` | 909 行 | 554 行 | -39% |
| `FilePreviewContent.tsx` | 650+ 行 | 584 行 | -10% |
| `FileListContent.tsx` | 1000+ 行 | 900+ 行 | -10% |

#### 验收结果

| 验收项 | 状态 |
|--------|------|
| `npm run build` | ✅ 通过 |
| `npx tsc --noEmit` | ✅ 无类型错误 |
| `npm run lint` | ✅ 无 lint 错误 |
| 视觉一致性 | ✅ 完全保持原样 |
| 功能完整性 | ✅ 所有功能正常 |

#### 关键原则

1. **零视觉变更**：所有 CSS 类、data-oid 属性完整保留
2. **零功能回归**：所有业务逻辑、状态管理、事件处理完整保留
3. **单一职责**：每个组件只负责一个明确的功能领域
4. **渐进式**：每次只拆分一个文件，验证通过后再继续

---

### 🐛 Bug 修复（预览功能）

#### 1. 修复图片预览放大缩小旋转重置功能失效

**问题描述**

预览页的放大、缩小、旋转、重置按钮点击后，图片没有任何变化。

**根本原因**

`FilePreview.tsx` 中定义的 `zoom` 和 `rotation` 状态没有正确传递到 `ImagePreview` 组件。`ImagePreview` 组件有自己的 `imageTransformRef`，但无法获取到父组件的状态值。

**修复方案**

- 在 `ImagePreviewProps` 接口中添加 `zoom` 和 `rotation` 属性
- 使用 CSS 变量 + Tailwind 任意值语法 `transform-[scale(var(--preview-zoom))_rotate(var(--preview-rotation))]` 应用变换
- 更新 `FilePreviewContentProps` 接口，传递 `zoom` 和 `rotation` 到 `ImagePreview`
- 清理 `FilePreview.tsx` 中不再需要的 `imageTransformRef` 和 `useEffect`

**影响范围**

- `frontend/src/components/files/preview/ImagePreview.tsx`
- `frontend/src/components/files/preview/FilePreviewContent.tsx`
- `frontend/src/components/files/preview/FilePreview.tsx`

#### 2. 修复视频预览循环播放功能失效

**问题描述**

视频预览工具栏的循环播放按钮点击后，视频不会循环播放。

**根本原因**

虽然 `loop` 属性已正确绑定到 `<video>` 元素，但当 `loop` 状态变化时，React 的属性更新可能没有正确同步到 DOM，特别是在使用 HLS.js 加载视频时。

**修复方案**

在 `VideoPreview` 组件中添加 `useEffect`，当 `loop` 属性变化时，直接操作 DOM 元素更新 `loop` 属性：

```typescript
useEffect(() => {
  if (videoRef.current) {
    videoRef.current.loop = loop;
  }
}, [loop, videoRef]);
```

**影响范围**

- `frontend/src/components/files/preview/VideoPreview.tsx`

---

### 🚀 后端依赖升级与 OpenTelemetry 追踪集成（2026-04-28）

#### 依赖版本升级

| Crate | 当前版本 | 升级后版本 | Rust 版本要求 | 说明 |
|-------|---------|-----------|--------------|------|
| axum | 0.7 | 0.8 | 1.75+ | 改进 extractor、error handling |
| aws-sdk-s3 | 1.0 | 1.82+ | 1.70+ | 大量 bug 修复和性能改进 |
| aws-config | 1.0 | 1.6+ | 1.70+ | 同上 |
| jsonwebtoken | 9.2 | 9.3 | 1.65+ | 安全补丁 |
| moka | 0.12 | 0.12.10+ | 1.65+ | 内存管理改进 |
| image | 0.25 | 0.25.6+ | 1.65+ | CVE 修复 |

#### OpenTelemetry 分布式追踪

- 更新到 opentelemetry 0.28 版本
- 配置 OTLP 导出器（默认端点：`http://localhost:4317`）
- 支持多种采样策略（always_on、always_off、parentbased、traceidbased）
- trace_id 在请求间传播
- 集成 Jaeger（已在 docker-compose.yml 中定义）

#### axum 0.8 兼容性修复

**移除 async_trait 宏**
- axum 0.8 原生支持 async trait，移除了 `axum::async_trait` 导出
- 修改文件：`src/extractors/admin.rs`、`src/extractors/auth.rs`

**路由语法变更**
- 参数路由：`/:id` → `/{id}`（21 处）
- 通配符路由：`/*path` → `{*path}`（1 处）
- 修改文件：`src/api/admin.rs`、`src/api/files.rs`、`src/api/folders.rs`、`src/api/organizations.rs`、`src/api/api_token.rs`、`src/api/share.rs`

#### 验收结果

| 验收项 | 状态 |
|--------|------|
| `cargo check` | ✅ 通过 |
| `cargo clippy --all-targets --all-features -- -D warnings` | ✅ 无警告 |
| `cargo test` | ✅ 全部通过（30+ 测试） |
| `cargo build --profile dist` | ✅ 成功（21MB 二进制） |

#### 运行说明

启动 Jaeger 后，可通过以下环境变量启用追踪：

```bash
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export OTEL_SERVICE_NAME=file-storage-backend
cargo run
```

Jaeger UI 访问地址：http://localhost:16686

#### 详细文档

完整的技术细节和实施步骤请参考：[docs/BACKEND_UPGRADE_2026-04-28.md](./BACKEND_UPGRADE_2026-04-28.md)

---

### 🤖 AI 自动修复

#### 🤖 Codex Auto-Fix (PR #32, round 1) — ts=1778722807

- 安全扫描：发现潜在风险
- 质量评分：0 / 100
- 变更文件：
  - `backend/src/api/webdav.rs`
  - `backend/src/handlers/files/fulltext_search.rs`
  - `backend/src/services/fulltext_search.rs`
  - `docs/CHANGELOG.md`

#### 🤖 Codex Auto-Fix (PR #32, round 1) — ts=1778722475

- 安全扫描：发现潜在风险
- 质量评分：38 / 100
- 变更文件：
  - `backend/src/api/webdav.rs`
  - `backend/src/handlers/files/fulltext_search.rs`
  - `backend/src/services/fulltext_search.rs`

#### 🤖 Codex Auto-Fix (PR #32, round 1) — ts=1778712651

- 安全扫描：发现潜在风险
- 质量评分：41 / 100
- 变更文件：
  - `backend/src/api/webdav.rs`
  - `backend/src/handlers/files/fulltext_search.rs`
  - `backend/src/services/file/list.rs`
  - `backend/src/services/fulltext_search.rs`
  - `frontend/src/components/settings/WebDavAccessSection.tsx`

#### 🤖 Codex Auto-Fix (PR #29, round 1) — ts=1778525990

- 安全扫描：发现潜在风险
- 质量评分：52 / 100
- 变更文件：
  - `docs/CHANGELOG.md`
  - `scripts/codex-cli/src/repo.rs`
  - `scripts/codex-cli/src/review_ledger.rs`

#### 🤖 Codex Auto-Fix (PR #29, round 1) — ts=1778525198

- 安全扫描：发现潜在风险
- 质量评分：52 / 100
- 变更文件：
  - `scripts/codex-cli/src/repo.rs`
  - `scripts/codex-cli/src/review_ledger.rs`
#### 🤖 Codex Auto-Fix (PR #28, round 1) — ts=1778520011

- 安全扫描：通过
- 质量评分：25 / 100
- 变更文件：
  - `scripts/codex-cli/src/repo.rs`

#### 🤖 Codex Auto-Fix (PR #27, round 1) — ts=1778485870

- 安全扫描：通过
- 质量评分：63 / 100
- 变更文件：
  - `docs/CHANGELOG.md`
  - `scripts/codex-cli/src/repo.rs`

#### 🤖 Codex Auto-Fix (PR #27, round 1) — ts=1778485161

- 安全扫描：发现潜在风险
- 质量评分：63 / 100
- 变更文件：
  - `scripts/codex-cli/src/repo.rs`

#### 🤖 Codex Auto-Fix (PR #27, round 1) — ts=1778483936

- 安全扫描：发现潜在风险
- 质量评分：68 / 100
- 变更文件：
  - `docs/CHANGELOG.md`
  - `scripts/codex-cli/src/repo.rs`

#### 🤖 Codex Auto-Fix (PR #27, round 1) — ts=1778483184

- 安全扫描：发现潜在风险
- 质量评分：68 / 100
- 变更文件：
  - `scripts/codex-cli/src/repo.rs`

#### 🤖 Codex Auto-Fix (PR #26, round 1) — ts=1778473007

- 安全扫描：发现潜在风险
- 质量评分：22 / 100
- 变更文件：
  - `backend/migrations/035_rewrite_active_file_name_unique_indexes.sql`
  - `backend/src/services/file/delete.rs`
  - `backend/tests/migration_governance_tests.rs`
  - `docs/CHANGELOG.md`
  - `frontend/src/components/files/grid/FileCard.tsx`

#### 🤖 Codex Auto-Fix (PR #26, round 1) — ts=1778471910

- 安全扫描：发现潜在风险
- 质量评分：28 / 100
- 变更文件：
  - `backend/migrations/035_rewrite_active_file_name_unique_indexes.sql`
  - `backend/src/services/file/delete.rs`
  - `backend/tests/migration_governance_tests.rs`
  - `frontend/src/components/files/grid/FileCard.tsx`

#### 🤖 Codex Auto-Fix (PR #26, round 1) — ts=1778469810

- 安全扫描：通过
- 质量评分：25 / 100
- 变更文件：
  - `backend/migrations/034_add_files_deleted_at.sql`
  - `backend/src/services/file/delete.rs`
  - `frontend/src/pages/Trash.tsx`

#### 🤖 Codex Auto-Fix (PR #26, round 1) — ts=1778382774

- 安全扫描：发现潜在风险
- 质量评分：62 / 100
- 变更文件：
  - `backend/migrations/034_add_files_deleted_at.sql`
  - `backend/src/bin/worker.rs`
  - `backend/src/services/file/delete.rs`
  - `frontend/src/pages/Trash.tsx`

#### 🤖 Codex Auto-Fix (PR #26, round 1) — ts=1778380921

- 安全扫描：发现潜在风险
- 质量评分：58 / 100
- 变更文件：
  - `backend/src/bin/worker.rs`
  - `backend/src/services/file/delete.rs`
  - `frontend/src/utils/dragAutoScroll.ts`

#### 🤖 Codex Auto-Fix (PR #15, round 1) — ts=1777980848

- 安全扫描：通过
- 质量评分：72 / 100
- 变更文件：
  - `frontend/src/components/files/useFileList.ts`
  - `frontend/src/styles/tokens.css`

#### 🤖 Codex Auto-Fix (PR #15, round 1) — ts=1777980174

- 安全扫描：通过
- 质量评分：72 / 100
- 变更文件：
  - `frontend/src/components/files/useFileList.ts`
#### 🤖 Codex Auto-Fix (PR #14, round 1) — ts=1777975852

- 安全扫描：通过
- 质量评分：0 / 100
- 变更文件：
  - `frontend/src/components/layout/NavBar.tsx`
  - `scripts/codex-cli/src/skills.rs`

#### 🤖 Codex Auto-Fix (PR #14, round 1) — ts=1777972130

- 安全扫描：通过
- 质量评分：76 / 100
- 变更文件：
  - `frontend/src/styles/lightThemeTokens.test.ts`
  - `frontend/src/styles/nav.css`
  - `frontend/src/styles/platform.css`

#### 🤖 Codex Auto-Fix (PR #14, round 1) — ts=1777971721

- 安全扫描：通过
- 质量评分：76 / 100
- 变更文件：
  - `frontend/src/styles/lightThemeTokens.test.ts`
  - `frontend/src/styles/nav.css`
  - `frontend/src/styles/tokens.css`

#### 0. codex pr-auto-fix 引入“变更入档”模块化 Skill

**变更内容**

- `codex pr-auto-fix` 的 Pipeline 新增 `DocumentationSkill`：每次自动修复会把 PR 号、轮次、变更文件清单、安全扫描结果、质量评分写入 `docs/CHANGELOG.md`
- 增加 `DryRunFeedbackSkill`：本地未传 `--yes` 时，自动在 PR 留评论说明“已生成但未推送”，避免静默停留在本地
- `scripts/codex-cli` 代码完成模块化解耦：从单文件实现拆分为 `src/lib.rs + config/llm/repo/skills/pipeline/runtime`，二进制入口迁移至 `src/bin/codex.rs`
- Pipeline 已支持插拔式扩展（如安全审计 `SecurityCheckSkill`、质量评分 `QualityScoreSkill`），并在 PR 评论中展示结果

**收益**

- 修复链路可追溯：任何一次 AI 自动变更都能在同一份 Changelog 中复盘
- Skill 可复用：文档记录能力可以被复用到其他命令/流水线（例如批量重构、发布前自检）

---

### 🧱 架构调整

#### 1. 拆分前端 `App.tsx`，降低入口复杂度

**变更内容**

- 将原先集中在 `frontend/src/App.tsx` 中的 `QueryClient` 配置、React Query Devtools、认证守卫和路由定义拆分到独立模块：
  - `frontend/src/providers/QueryProvider.tsx`
  - `frontend/src/providers/AuthProvider.tsx`
  - `frontend/src/router/AppRouter.tsx`
- `App.tsx` 现在仅保留应用装配职责：`QueryProvider`、`BrowserCompatibilityWarning`、`ErrorBoundary`、`AuthProvider`、`AppRouter`

**收益**

- 降低单文件复杂度，便于维护
- 路由、认证、数据层职责边界更清晰
- 后续继续拆分 Devtools、Bootstrap、PWA 等逻辑更容易

---

#### 2. 拆分前端全局启动逻辑（bootstrap）

**变更内容**

- 将 `frontend/src/main.tsx` 中的启动逻辑拆分为独立模块：
  - `frontend/src/bootstrap/sentry.ts`
  - `frontend/src/bootstrap/preconnect.ts`
  - `frontend/src/bootstrap/errorTracking.ts`
- `main.tsx` 只保留样式引入、`App` 渲染和基础启动装配

**收益**

- 入口文件职责单一
- 便于后续继续抽离 `vitals`、PWA、平台初始化逻辑

---

#### 3. Tauri（Rust 侧）引入 Repository + Service 分层骨架

**变更内容**

- 为 `frontend/src-tauri/src/lib.rs` 补齐“装配层”职责：初始化后端 base url、构建 HTTP client、注入 `AppState`、注册 `invoke_handler`
- 增加最小纵切（以 health 为例）：
  - `commands/health.rs`：Tauri command `backend_health`
  - `services/backend_service.rs`：业务层入口
  - `repositories/backend_repository.rs`：对 backend 的 HTTP 调用封装（`GET /health`）
  - `models/error.rs`：`AppError`（支持 `Result<T, AppError>` 返回给前端）
- 后端地址支持通过环境变量覆盖：
  - `UPLOAD_DOWNLOAD_UTIL_API_BASE_URL`（默认 `http://localhost:3000`）

**收益**

- `lib.rs` 不再承担业务/IO 细节，职责稳定为“Runtime 装配”
- 与 backend 的 Types → Repo → Service → Runtime 分层理念对齐，便于继续扩展文件上传/下载等命令

---

### 🎨 UI / 主题改造

#### 3. 全面实现前端界面的动态主题（Light/Dark/Purple）

**变更内容**

- 彻底移除了前端代码中大量硬编码的颜色类名（如 `text-slate-900`、`bg-emerald-500` 等），替换为语义化的 CSS 变量（如 `var(--settings-title)`、`var(--upload-text)` 等）。
- **上传模块**：上传弹窗（`UploadDialog`）、拖拽区（`UploadDropzone`）、URL 上传表单及文件列表项（`UploadFileItem`）全面适配三种主题，进度条和错误气泡颜色均动态化。
- **设置模块**：设置页（`Settings`）及其各个子区块（如用户信息、存储空间、修改密码、API Token 等）的背景、边框、文字、按钮全部改用 `--settings-*` 变量驱动。
- **预览模块**：文件预览页（`FilePreviewContent`）的加载态、错误态、不支持状态的 UI 以及 Markdown 预览（`MarkdownPreview`）的样式去除了明暗硬编码，统一由 `--preview-*` 主题变量控制。
- **文件网格**：文件夹卡片（`FolderCard`）与下拉菜单配色接入了文件列表相关的玻璃态主题变量。
- **样式定义**：在 `frontend/src/styles/tokens.css` 和 `index.css` 中补齐并扩展了针对 `light`、`dark` 和 `purple` 主题的上传、设置、预览及文件列表语义变量映射。

**收益**

- 三种主题（浅色、深色、紫色）切换时，颜色不再依赖组件内的 Tailwind 颜色硬编码，视觉表现完全一致且准确。
- 浅色模式可以正确显示深字白底，深色/紫色主题保持独立且协调的视觉风格。
- 后续调整主题视觉时只需修改 CSS token，无需再逐个排查修改 React 组件。

---

#### 4. 修复上传与预览 UI 的多处主题显示问题

**已处理问题**

- 上传弹窗浅色模式背景仍显示紫色
- 上传文件项在浅色模式下仍保留深色/紫色块
- 文件预览页浅色模式下文字和图标显示为黑色/不统一
- 批量操作栏图标在不同主题下对比度不稳定
- `All Files` 区域统计文字和若干提示信息颜色不统一

**影响范围**

- `frontend/src/components/files/upload/UploadDialog.tsx`
- `frontend/src/components/files/upload/UploadFileItem.tsx`
- `frontend/src/components/files/preview/FilePreview.tsx`
- `frontend/src/components/files/preview/FilePreviewContent.tsx`
- `frontend/src/components/files/list/FileListBatchActions.tsx`
- `frontend/src/components/files/list/FileListContent.tsx`

---

### 🧹 工程治理

#### 5. 调整 ESLint 忽略规则，排除生成目录

**变更内容**

- 在 `frontend/eslint.config.js` 中补充忽略目录：
  - `dist`
  - `coverage`
  - `src-tauri/target`
  - `src-tauri/gen`
  - `.vite`

---

### ⚙️ 后端重构与工程优化

#### 6. 后端配置模块化重构

**变更内容**

- **配置拆分**：将巨大的 `backend/src/config.rs`（23KB+）拆分为 `backend/src/config/` 目录下的多个子模块：
  - `auth.rs`, `database.rs`, `redis.rs`, `oauth.rs`, `server.rs`, `storage.rs`, `tasks.rs`, `cache.rs`, `search.rs`, `rate_limit.rs`
- **嵌套结构**：引入了基于 `config` crate 的嵌套 Struct 支持，配置路径更清晰（如 `config.auth.jwt_secret`）。
- **向后兼容**：通过 `Environment` 别名映射，确保原有的平铺环境变量（如 `DATABASE_URL`, `REDIS_URL`）在不修改 `.env` 的情况下依然有效。
- **Redis 纠偏**：纠正了 Redis 配置路径，将其从 `database.redis_url` 移至顶级的 `redis.url`。

**收益**

- 极大提升了配置代码的可维护性和可读性。
- 职责分离，不同功能的配置互不干扰。

---

#### 7. 健康检查与稳定性增强

**变更内容**

- **超时控制**：在 `backend/src/handlers/health.rs` 的 `readiness_check` 端点中，为数据库、Redis 和存储后端的检查添加了显式的超时控制（`tokio::time::timeout`）。
- **超时设置**：数据库(2s)、Redis(2s)、存储后端(5s)，避免因依赖项挂起导致探测阻塞。
- **详细日志**：增强了健康检查失败时的日志记录，能够区分是连接错误还是响应超时。

---

#### 8. Docker 与部署优化

**变更内容**

- **镜像升级**：将后端 `Dockerfile` 的 Rust 基础镜像从 `1.75` 升级至 `1.80-slim`。
- **构建优化**：改用 `slim` 变体显著减小镜像体积，并手动补齐了必要的构建依赖（`pkg-config`, `libssl-dev`）。
- **忽略文件更新**：更新了 `.gitignore`、`.dockerignore` 和 `.vercelignore`，将 `actions-runner/` 目录纳入忽略范围，防止敏感或无关文件上传。

**修复效果**

- `eslint` 不再扫描 Tauri/Rust 生成产物
- 之前大量由二进制/打包输出引起的 Parsing Error 已消失

**当前状态**

- `npm run build` 通过
- `npm run lint` 仍存在少量存量问题（非本次拆分引入）：
  - `frontend/src/components/files/preview/FilePreviewContent.tsx`
  - `frontend/src/components/files/useFileList.ts`

---

### 🐛 Bug 修复

#### 0. 修复文件列表缓存反序列化类型不匹配

**问题描述**

- Redis 缓存写入时存的是 `Vec<FileResponse>` 形状，但读取时按 `FileListResult(Vec<File>)` 反序列化，导致命中缓存时反序列化失败（等价于“缓存永远不生效”）
- 缓存 JSON 曾包含 `page` / `limit` 等额外字段，读取侧若按精确结构反序列化会失败

**修复方案**

- 读取缓存时改为反序列化为与响应一致的结构（`CachedFileListResponse { files: Vec<FileResponse>, ... }`）
- 写入缓存时仅保存缓存所需字段（`files/total/next_cursor`），避免结构漂移

**影响范围**

- `backend/src/services/file/list.rs`

---

#### 1. 修复文件夹列表消失问题（`useFileList.ts`）

**问题描述**

页面上已存在的文件夹全部消失，刷新后才重新显示。

**根本原因**

`useFileList.ts` 中存在经典的 JavaScript 变量提升（hoisting）陷阱。原代码使用 `let` 先声明 `displayFolders` 为空数组，随后在下方才用 `useMemo` 真正赋值，但 `outFolders` 的 `useMemo` 在 `displayFolders` 还是初始空数组时就已经捕获了该引用：

```typescript
// ❌ 原代码（有问题）
let displayFolders: Folder[] = [];          // 1. 声明为空数组
const outFolders = useMemo(
  () => displayFolders.filter(...),          // 2. 此时 displayFolders 是空数组引用
  [displayFolders, pendingDeleteFolderIds],  //    依赖数组里的也是空数组，永远不会更新
);
// ...（中间有大量其他 Hook）
displayFolders = useMemo(() => { ... });    // 3. 重新赋值，但 outFolders 感知不到
```

由于 `outFolders` 的 `useMemo` 依赖数组里的 `displayFolders` 始终是初始空数组的引用，导致 `outFolders` 永远为空数组，最终 `displayFolders: outFolders` 传给渲染层时文件夹全部消失。

**修复方案**

将 `displayFolders` 改为 `const`，并确保 `outFolders` 在 `displayFolders` 赋值之后声明：

```typescript
// ✅ 修复后
const displayFolders = useMemo(() => { ... }, [...]);  // 先计算

const outFolders = useMemo(                             // 再过滤
  () => displayFolders.filter(...),
  [displayFolders, pendingDeleteFolderIds],
);
```

同时将 `outFoldersRef` 改为正规的 `useRef<Folder[]>`，并将回滚检测的 `useEffect` 移到 `displayFolders` 赋值之后，结构更清晰。

**影响范围**

- `frontend/src/components/files/useFileList.ts`

---

#### 2. 修复第二次删除文件后 UI 不立即更新问题（`useFileMutations.ts`）

**问题描述**

删除第一个文件时，文件会立即从页面消失（正常）。但删除第二个文件时，文件不会立即消失，只有刷新页面后才消失。后台数据确实已被正确删除。

**根本原因**

乐观更新（Optimistic Update）与 React Query 的后台 refetch 之间存在竞态条件（Race Condition）：

**第一次删除**（正常）：
1. `onMutate` → `cancelQueries` 取消所有进行中的请求 → 从缓存移除文件 → UI 立即消失 ✅
2. `onSettled` → `invalidateQueries` 标记 stale + **立即触发后台 refetch**
3. refetch 返回（不含已删文件）→ 缓存更新 ✅

**第二次删除**（有问题）：
1. 第一次删除触发的 refetch **仍在进行中**
2. `onMutate` → `cancelQueries` → 但此时 refetch 已发出，`cancelQueries` 无法取消已在途的请求
3. 从缓存移除第二个文件 → UI 立即消失（看起来正常）
4. **第一次删除的 refetch 返回**，带回包含第二个文件的旧数据 → **覆盖乐观更新** ❌
5. 第二个文件重新出现在页面上

**修复方案**

将 `onSettled` 中的 `invalidateQueries` 改为 `refetchType: 'none'`，仅标记缓存为 stale 但不立即触发后台 refetch，彻底消除竞态：

```typescript
// ❌ 原代码：标记 stale 并立即触发后台 refetch（会与乐观更新竞争）
onSettled: () => {
  void queryClient.invalidateQueries({ queryKey: ['files'] });
}

// ✅ 修复后：仅标记 stale，不触发 refetch
// 下次窗口聚焦/用户导航时才会自动重新获取真实数据
onSettled: () => {
  void queryClient.invalidateQueries({ queryKey: ['files'], refetchType: 'none' });
}
```

此修复同时应用于 `deleteFile`、`batchDeleteFiles`、`deleteFolder` 三个 mutation。

**影响范围**

- `frontend/src/hooks/files/useFileMutations.ts`

---

#### 3. 修复文件移动到目标文件夹时的同名冲突导致 500 错误

**问题描述**

当文件移动到另一个文件夹，而目标文件夹已存在同名文件时，接口触发 PostgreSQL 唯一约束（`23505`），最终返回 `500 Internal Server Error`。

**修复方案**

- 在执行移动前，先查询两类冲突：
  - 待移动文件集合内部是否存在同名文件
  - 目标文件夹中是否已存在同名文件
- 命中冲突时直接返回可读的验证错误，而不是让数据库约束抛出 500

**影响范围**

- `backend/src/repositories/folders.rs`
- `backend/src/services/folder.rs`

---

### ✨ 新功能 / 性能优化

#### 6. 引入 `@chenglou/pretext` — DOM-free 文本测量，精确虚拟列表行高

**包简介**

`@chenglou/pretext` 是一个**不依赖 DOM 的纯计算文本排版库**，可在不创建任何 DOM 节点的情况下精确计算多行文本的行数和高度。相比传统的"创建隐藏 DOM 节点 → 读取 offsetHeight"方案，它：

- 零 DOM 操作，无布局抖动（Layout Thrashing）
- 纯同步计算，可在渲染前完成所有高度预测
- 内部使用字形宽度数据进行精确排版模拟
- 适合在 Web Worker 或 SSR 环境中使用

**使用场景**

虚拟列表（`VirtualizedFileGrid` / `VirtualizedMixedGrid`）需要在渲染前就知道每一行的精确高度，才能正确计算滚动位置和可见范围。原来使用固定行高（`VIRTUAL_GRID_ROW_HEIGHT = 360px`），导致：

- 文件名较长时卡片被裁剪
- 文件名较短时行间有大量空白
- 滚动位置计算不准确，出现跳动

**实现细节**

新增 `frontend/src/utils/pretextMeasure.ts`，封装了以下能力：

| 函数 | 作用 |
|------|------|
| `measureLineCount(text, font, maxWidth, lineHeight)` | 计算文本在指定宽度下的实际行数 |
| `computeFileCardHeight(filename, cardWidth, vw)` | 计算文件卡片的精确高度（含缩略图、文件名、元信息） |
| `computeFolderCardHeight(name, cardWidth, vw)` | 计算文件夹卡片的精确高度 |
| `buildRowModel(items, columns, containerWidth, vw)` | 构建整个网格的行高模型，返回 `rowHeights` 和前缀和数组 `prefixSums` |
| `findStartRow(prefixSums, scrollTop)` | 二分查找当前滚动位置对应的起始行（O(log n)） |
| `findEndRow(prefixSums, scrollBottom)` | 二分查找当前滚动位置对应的结束行（O(log n)） |
| `clearPretextMeasureCache()` | 清除内部 LRU 缓存（最大 2000 条） |

字体大小使用 CSS `clamp()` 等价的纯 JS 计算，与实际渲染保持一致：

```typescript
// 文件名字体大小：clamp(0.38rem, 1.3vw, 0.58rem)
export function fileNameFontSizePx(vw: number): number {
  return Math.max(0.38 * 16, Math.min(0.58 * 16, 0.013 * vw));
}
```

**对页面的提升效果**

1. **虚拟列表滚动精准**：行高精确计算后，滚动到任意位置都不会出现内容跳动或空白
2. **卡片不再被裁剪**：长文件名（2行）的卡片高度自动扩展，短文件名（1行）的卡片紧凑排列
3. **混合网格支持**：文件夹卡片和文件卡片高度不同，`VirtualizedMixedGrid` 可以同时处理两种高度，同一行取最大值
4. **性能无损**：所有计算同步完成，无 DOM 操作，不触发浏览器重排；内部 LRU 缓存避免重复计算相同文件名

**新增测试**

`frontend/src/utils/pretextMeasure.test.ts` 覆盖了所有核心函数：

- 字体大小 clamp 边界值（min/max/mid）
- 空字符串、空白字符串、零宽度的边界处理
- 卡片高度公式验证
- 行高增长和上限约束
- `buildRowModel` 的单调性和前缀和正确性
- `findStartRow` / `findEndRow` 的二分查找正确性

**影响范围**

- `frontend/package.json`（新增依赖 `@chenglou/pretext: ^0.0.3`）
- `frontend/src/utils/pretextMeasure.ts`（新增）
- `frontend/src/utils/pretextMeasure.test.ts`（新增）
- `frontend/src/components/files/grid/VirtualizedMixedGrid.tsx`（接入 pretext 行高计算）
- `frontend/src/components/files/grid/VirtualizedFileGrid.tsx`（接入 pretext 行高计算）

---

### � 后端改进与可观测性

#### 7. 引入 OpenTelemetry 分布式追踪（tracing-opentelemetry）

**问题描述**

随着异步任务（如 HLS 视频转码、GIF 预览生成、ZIP 大文件打包）逻辑变复杂，单机日志无法有效跨任务/跨进程追踪耗时瓶颈和请求上下文，难以进行性能排查。

**解决方案**

引入 `tracing-opentelemetry`，基于 W3C Trace Context 为请求和后台 Worker 建立全链路分布式追踪：
- 在 `backend/src/tracing.rs` 中初始化 OpenTelemetry OTLP 导出器（gRPC）。
- 升级依赖并解决版本冲突：`opentelemetry` (0.27) / `opentelemetry-otlp` (0.27) / `tracing-opentelemetry` (0.28)。
- 统一 `main.rs` 和 `worker.rs` 的追踪初始化流程。
- 为核心后台任务 `run_hls_worker`、`run_gif_preview_worker`、`batch_download_zip` 等函数加上 `#[tracing::instrument]` 宏，注入 `trace_id`、`user_id`、`file_id` 和 `task_id`。
- 支持环境变量配置 `OTEL_EXPORTER_OTLP_ENDPOINT` 与 `OTEL_TRACES_SAMPLER`（动态采样策略）。

**收益**

- 在可观测性平台（如 Jaeger / Zipkin）中能清晰呈现每个 HTTP 请求或异步转码任务的瀑布流。
- 排查高并发情况下的资源争用或失败链路变得极其直观。
- 完善了项目 `docs/TOP_TECH.md`、`docs/OPTIMIZATION_SUMMARY.md` 中的可观测性说明文档。

---

### �📋 变更汇总

| 文件 | 类型 | 说明 |
|------|------|------|
| `frontend/src/components/files/useFileList.ts` | 🐛 Bug Fix | 修复 `displayFolders` 变量提升导致文件夹列表消失 |
| `frontend/src/hooks/files/useFileMutations.ts` | 🐛 Bug Fix | 修复第二次删除文件 UI 不立即更新的竞态问题 |
| `frontend/src/utils/pretextMeasure.ts` | ✨ New | DOM-free 文本测量工具，为虚拟列表提供精确行高 |
| `frontend/src/utils/pretextMeasure.test.ts` | ✅ Test | pretextMeasure 单元测试 |
| `frontend/src/components/files/grid/VirtualizedMixedGrid.tsx` | ✨ Enhance | 接入 pretext 精确行高计算 |
| `frontend/src/components/files/grid/VirtualizedFileGrid.tsx` | ✨ Enhance | 接入 pretext 精确行高计算 |
| `frontend/package.json` | 📦 Dep | 新增 `@chenglou/pretext ^0.0.3` |

---

### 🔬 技术细节：React Query 乐观更新最佳实践

本次修复揭示了一个使用 React Query 乐观更新时的常见陷阱，记录如下供参考：

**错误模式**

```typescript
onMutate: async (id) => {
  await queryClient.cancelQueries({ queryKey: ['items'] }); // 取消进行中的查询
  // 乐观更新缓存...
},
onSettled: () => {
  // ❌ 立即触发 refetch，会与乐观更新竞争
  void queryClient.invalidateQueries({ queryKey: ['items'] });
}
```

**问题**：`invalidateQueries` 默认的 `refetchType: 'active'` 会立即触发所有活跃查询的 refetch。如果上一次操作的 refetch 仍在进行中，`cancelQueries` 无法取消它，该 refetch 完成后会用旧数据覆盖乐观更新。

**正确模式（本项目采用）**

```typescript
onSettled: () => {
  // ✅ 仅标记 stale，不立即 refetch
  // 下次窗口聚焦/路由切换时自动同步真实数据
  void queryClient.invalidateQueries({ queryKey: ['items'], refetchType: 'none' });
}
```

**权衡**：使用 `refetchType: 'none'` 后，删除成功但数据同步会延迟到下次自然触发（窗口聚焦、切换路由等）。对于删除操作，这是可接受的，因为乐观更新已经提供了即时的视觉反馈，且真实数据在服务端已经正确删除。

---

## 历史版本

> 更早的变更记录请参阅 Git 提交历史（`git log --oneline`）。
> 主要里程碑包括：
>
> - `fc51472` feat(虚拟化网格): 实现精确行高计算与即时删除隐藏
> - `611c36c` feat(security): 添加常量时间比较工具防止时序攻击
> - `ce8852d` fix: 修复 macOS 应用沙盒配置以支持视频硬件解码
> - `47d7a27` feat(macos): 为 Tauri 应用添加 macOS 原生窗口样式支持
> - `ac997cd` feat(desktop): 添加 Tauri 桌面应用支持
> - `f1b6308` feat(frontend): 添加端到端测试支持并重构设置组件
> - `e533066` feat(theme): 新增紫色主题并重构样式为 CSS 变量
