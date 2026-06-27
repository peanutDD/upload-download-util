## codex-cli 文档

`codex-cli` 是一个可复用的“自动化 Review → 生成补丁 → 应用补丁 → 可选推送/反馈”的命令行工具。

它既可以在 GitHub Actions 里对 PR 的 Gemini Review 执行自动修复，也可以脱离 GitHub，直接对任意本地仓库执行同样的修复流水线。

### 核心特性：完全本地执行架构

**安全性提升**

- 不依赖任何外部 API（如 OpenAI/GPT）
- 所有 LLM 调用完全通过本地 Codex 命令执行
- 代码和数据 100% 保留在本地环境
- 避免网络传输中的数据泄露风险

**性能与稳定性**

- 本地执行延迟更低
- 不受外部服务限流影响
- 可以充分利用本地硬件加速

**成本效益**

- 无需支付 API 调用费用
- 充分利用已有的计算资源

**本地 Codex GPT-5.5 架构**
- 所有 LLM 任务通过 `CODEX_AGENT_COMMAND` 配置的本地命令执行
- 支持三种 prompt 输入方式：
  - 无占位符：完整 prompt 写入 stdin
  - `{prompt}`：prompt 作为单个命令参数
  - `{prompt_file}`：prompt 写入临时文件，命令接收路径

### 快速开始

**前置依赖**

- Rust（用于构建）
- `git`（用于 `git apply` / `git add` / `git commit` / `git push`）
- （可选）`gh`（用于 PR 评论：`gh pr comment`）

**环境变量**

- `CODEX_AGENT_COMMAND`：必填，本地 Codex 执行命令，例如 `codex exec --skip-git-repo-check -`
- `CODEX_AGENT_TIMEOUT_SECONDS`：可选，单次 Codex 调用超时，默认 `900`

**构建**

```bash
cargo build --release
```

### 使用方式

**GitHub PR 模式（供 Actions 调用）**

```bash
./target/release/codex-auto-fix pr-auto-fix \
  --pr-number 123 \
  --gemini-review "$GEMINI_REVIEW" \
  --max-rounds 2 \
  --yes \
  --repo-root "$GITHUB_WORKSPACE"
```

**本地仓库模式（跨仓库复用）**

```bash
./target/release/codex-auto-fix auto-fix-local \
  --repo-root /abs/path/to/any-repo \
  --review-file /abs/path/to/review.md \
  --max-rounds 2 \
  --yes
```

### 文档导航

**目录结构**

- `design-docs/`：架构与设计文档（为什么这么设计）
- `references/`：使用与配置参考（怎么用）
- `integrations/`：外部集成（CI/Actions）
- `constraints/`：永久约束（不会变/不该被打破）
- `exec-plans/`：执行计划样例/模板（改动可追溯）

**推荐阅读顺序**

- [实现技术文档](implementation.md)
- [架构与模块](design-docs/architecture.md)
- [文件详解](file-inventory.md)
- [Pipeline / Skills 设计](design-docs/pipeline.md)
- [CLI 与输出契约](references/cli.md)
- [配置与策略（规则/Changelog/过滤）](references/configuration.md)
- [自动 Review 使用说明](references/auto-review-usage.md)
- [Skill Pack 自动加载（零同步）](references/development.md#skill-pack-自动加载零同步)
- [GitHub Actions 集成](integrations/github-actions.md)
- [开发与发布](references/development.md)
- [安全与边界](design-docs/security.md)
- [故障排查](references/troubleshooting.md)
- [永久约束](constraints/README.md)
