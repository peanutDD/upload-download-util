## CLI 与输出契约

`codex-cli` 提供两个二进制入口：

- `codex-auto-fix`：推荐给 GitHub Actions / runner 使用，避免与真实 Codex GPT-5.5 CLI 冲突。
- `codex`：兼容旧入口，适合确认 PATH 不冲突的本地使用。

命令分为两类：

- 本地单文件辅助：`review` / `refactor` / `doc`
- 自动修复流水线：`pr-auto-fix` / `auto-fix-local`
- 结构化转换：`review-to-json`
- 诊断与运维：`doctor` / `auto-fix-weekly-report`

命令定义见：[codex.rs](file:///Users/tyone/github/upload-download-util/scripts/codex-cli/src/bin/codex.rs)

### 通用约定

- stdout：仅输出最终结果（自动修复命令输出 JSON）
- stderr：输出运行日志（便于 CI 抓取但不污染 JSON）

### doctor

检查本机安装、PATH、依赖和模型执行器配置是否与当前源码同步。

```bash
codex-auto-fix doctor
codex-auto-fix doctor --json
```

`doctor` 会报告：

- 当前运行的二进制路径
- 编译时的 `scripts/codex-cli` 源码目录
- PATH 上的 `codex-auto-fix` / `codex` 是否指向当前二进制
- 当前二进制是否晚于 `src/`、`Cargo.toml`、`Cargo.lock`
- `CODEX_AGENT_COMMAND` 是否配置，且是否误指向 `codex-auto-fix` 自己
- `git`、`gh`、`jq`、`cargo` 是否可用

GitHub Actions 的 `codex-fix` 会在真正启动 `pr-auto-fix` 前运行
`doctor --json`。如果 `agent.command` 是 `warning`，workflow 会 fail-fast，
避免本地 Runner 进入递归调用后看起来像“卡死”。

如果本机安装版落后源码，按提示执行：

```bash
cargo install --path scripts/codex-cli --force
```

### review

审查单个文件；可选 `--fix` 直接把模型返回的代码块写回文件。

```bash
codex review --path src/lib.rs
codex review --path src/lib.rs --fix
```

### refactor

按策略重构单个文件（策略是 prompt 参数）。

```bash
codex refactor --path src/lib.rs --strategy modularize
```

### doc

对单个文件生成 Markdown 文档并输出到 stdout。

```bash
codex doc --path src/lib.rs --kind api
```

### pr-auto-fix

对 PR 的 Review 输入执行自动修复。适用于 GitHub Actions。

```bash
codex-auto-fix pr-auto-fix \
  --pr-number 123 \
  --review-json /tmp/review.json \
  --max-rounds 2 \
  --yes \
  --repo-root "$GITHUB_WORKSPACE"
```

参数说明：

- `--pr-number`：PR 号（用于评论与 changelog 记录）
- `--review-json`：结构化 Review JSON 文件，主修复链路优先使用该输入
- `--review-text`：通用 Review 文本（Markdown 兼容输入，推荐给非结构化输入使用）
- `--gemini-review`：旧兼容入口，等价于 `--review-text`，保留给现有 workflow 回滚路径
- `--max-rounds`：外层 Gemini Review 轮次上限提示；单次命令只处理当前这一条 Gemini Review
- `--yes`：允许提交/推送（未传则 Dry-Run）
- `--repo-root`：仓库根目录（默认取 `GITHUB_WORKSPACE` 或当前目录）
- `--rules-file`：规则文件路径（覆盖 repo_root 下 `AGENTS.md`）
- `--changelog-path`：changelog 路径（相对路径会基于 repo_root 拼接）
- `--disable-changelog`：禁用 changelog 写入
- `--no-pr-comments`：禁用 PR 评论（仍会执行修复/推送）

输出（stdout）：

```json
{
  "fixed": true,
  "files": ["src/a.rs"],
  "quality_score": 95,
  "quality_score_available": true,
  "security_passed": true,
  "push_blocked": false,
  "has_pending": false,
  "pending_count": 0,
  "review_clean": true,
  "summary": "Gemini 对本次 PR 的整体总结",
  "fixed_explanations": [],
  "pending_explanations": [],
  "issue_statuses": [
    {
      "severity": "Medium",
      "file": "src/a.rs",
      "line": 12,
      "description": "Gemini issue text",
      "status": "resolved",
      "explanation": "已自动修复"
    }
  ]
}
```

### auto-fix-local

不依赖 GitHub PR，直接对本地仓库执行自动修复流水线。

```bash
codex-auto-fix auto-fix-local \
  --repo-root /abs/path/to/any-repo \
  --review-text "Medium: fix value" \
  --max-rounds 2 \
  --yes
```

参数说明：

- `--repo-root`：目标仓库根目录（必须是 git 仓库，且能执行 `git apply`）
- `--review-json`：结构化 Review JSON 文件，主修复链路优先使用该输入
- `--review-text`：通用 Review 文本（Markdown 兼容输入）
- `--review-file`：包含 review 文本的文件（Markdown 兼容输入）
- `--yes`：允许提交/推送（未传则 Dry-Run，不会提交/推送）
- `--rules-file / --changelog-path / --disable-changelog`：同 `pr-auto-fix`

输出（stdout）：同 `pr-auto-fix`。

### pr-auto-fix 输出观测字段

stdout 只输出一份 JSON，workflow 可直接用 `jq` 读取。关键字段包括：

- `fixed`：本轮是否产生修复（Dry-Run 下表示本地已改动）。
- `issue_statuses`：每个 `Medium/Medium+/High/Critical` Gemini issue 的一一对应状态，`status` 为 `resolved`、`pending` 或 `blocked`。
- `fixed_explanations`：已自动修复的 `Medium/Medium+/High/Critical` 问题清单。
- `pending_count` / `pending_explanations`：未自动修复的问题数量与原因。
- `apply_fail_reason`：`git apply` 失败分类，可能为 `malformed_diff`、
  `context_mismatch`、`drift`、`unknown` 或 `null`。
- `retry_count`：重试 apply 的次数。
- `fallback_used`：是否成功使用 full-file fallback。
- `final_status`：`clean`、`pending` 或 `needs-human`。

GitHub Actions 默认使用 `USE_REVIEW_JSON=true` 和 `--review-json`。临时回滚时
设置 `USE_REVIEW_JSON=false`，workflow 会跳过 JSON 转换并使用
`--gemini-review "$REVIEW_BODY"`。新接入方应优先使用 `--review-text`。

### review-to-json

把标准化 Gemini Review Markdown 转成稳定 JSON。GitHub Actions 主链路会把该
JSON 作为 `pr-auto-fix --review-json` 输入，避免在修复前再次依赖 LLM 解析 Markdown。

```bash
codex-auto-fix review-to-json \
  --input /tmp/review.md \
  --output /tmp/review.json
```

兼容 shell 入口保留在 `scripts/codex-cli/tools/review_to_json.sh`，用于沿用早期
workflow 片段；该脚本委托给同一个 Rust 子命令，不维护第二套 parser。

输入 Markdown 支持重复的问题块：

```markdown
- Severity: Medium
- File: src/lib.rs
- Line: 42
- Rule: no-unused-branch
- Problem: 条件恒为 true。
- Expected: 基于输入判断，避免死分支。
- Constraints:
  - only modify src/lib.rs
  - no signature change
```

输出文件与 stdout 都是相同的 JSON；过程日志只写 stderr。

```json
{
  "review_id": "",
  "summary": "1 actionable issues",
  "issues": [
    {
      "id": "ISSUE-001",
      "severity": "Medium",
      "file": "src/lib.rs",
      "line": 42,
      "rule": "no-unused-branch",
      "problem": "条件恒为 true。",
      "expected": "基于输入判断，避免死分支。",
      "constraints": ["only modify src/lib.rs", "no signature change"],
      "acceptance": []
    }
  ]
}
```

### auto-fix-weekly-report

把一周内收集到的 `pr-auto-fix` / `auto-fix-local` stdout JSON 聚合成失败样本 Top 5
Markdown 报告。输入可以是 JSONL，也可以是单个 JSON 或 JSON 数组。

```bash
codex-auto-fix auto-fix-weekly-report \
  --input /tmp/auto-fix-results.jsonl \
  --output docs/references/auto-fix-weekly-report.md
```

聚合维度固定为：`apply_fail_reason`、文件路径、`fallback_used`、`final_status`。
没有真实失败样本证明收益前，不默认增加 `git apply --check`。
