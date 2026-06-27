# codex-cli 实现技术文档

版本：v1.0
日期：2026-05-11
适用代码：`scripts/codex-cli`
目标读者：第一次接手 `codex-cli` 的维护者、CI 排障者、后续 Agent

## 0. 先读这个结论

`codex-cli` 是一个 Rust CLI crate，核心用途是把 Review 输入转成结构化问题，再让本地 Codex 执行器生成补丁，把补丁应用到目标仓库，并可选写文档、评论 PR、提交和推送。

它不是直接调用 OpenAI API 的 SDK，也不自己实现模型。所有模型调用都通过环境变量 `CODEX_AGENT_COMMAND` 指向的本地命令完成，例如：

```bash
CODEX_AGENT_COMMAND="codex exec --skip-git-repo-check -"
```

当前自动修复主补丁格式是 SEARCH/REPLACE block。unified diff 仍被支持，但只是兼容路径。维护时不要把 unified diff 重新改成默认主路径，除非同步修改约束、测试和文档。

最重要的执行链路是：

```text
src/bin/codex.rs
  -> runtime::{pr_auto_fix_with_options_and_review_data, auto_fix_local_with_review_data}
  -> runtime::run_auto_fix_loop
  -> pipeline::Pipeline
  -> skills::{ReadReviewSkill, DecisionSkill, BatchFixSkill}
  -> skills::{SecurityCheckSkill, QualityScoreSkill, DocumentationSkill}
  -> skills::{DryRunFeedbackSkill, FeedbackSkill}
  -> types::PrAutoFixOutput JSON
```

## 1. 目录与分层

源码目录：

```text
scripts/codex-cli/
├── Cargo.toml
├── AGENTS.md
├── src/
│   ├── lib.rs
│   ├── types.rs
│   ├── llm.rs
│   ├── repo.rs
│   ├── runtime.rs
│   ├── pipeline.rs
│   ├── skills.rs
│   ├── prompts.rs
│   ├── review_json.rs
│   ├── doctor.rs
│   ├── auto_fix_report.rs
│   ├── patch/
│   │   ├── mod.rs
│   │   ├── search_replace.rs
│   │   └── unified_diff.rs
│   └── bin/
│       ├── codex.rs
│       └── codex-auto-fix.rs
├── tests/
├── tools/
└── docs/
```

工程分层按本仓库规则理解：

| 层 | 文件 | 职责 |
| --- | --- | --- |
| Types | `src/types.rs` | Review 数据、输出 JSON、Skill Pack metadata |
| Config / LLM | `src/llm.rs`, `src/prompts.rs` | 本地模型执行器配置、prompt 模板 |
| Repo / IO | `src/repo.rs`, `src/patch/*` | 文件、git、gh、changelog、patch 应用 |
| Service | `src/skills.rs`, `src/review_json.rs`, `src/auto_fix_report.rs` | 业务步骤、Review 解析、失败报告 |
| Runtime | `src/runtime.rs`, `src/pipeline.rs` | 上下文创建、流水线编排、最终输出 |
| UI / CLI | `src/bin/codex.rs`, `src/bin/codex-auto-fix.rs`, `src/doctor.rs` | 参数解析、人类/机器输出、诊断命令 |

## 2. 构建与安装

在 `scripts/codex-cli` 下运行：

```bash
cargo build
cargo test
cargo install --path . --force
```

`Cargo.toml` 关闭了 `autobins`，显式声明两个二进制：

```toml
[[bin]]
name = "codex"
path = "src/bin/codex.rs"

[[bin]]
name = "codex-auto-fix"
path = "src/bin/codex-auto-fix.rs"
```

`src/bin/codex-auto-fix.rs` 只有：

```rust
include!("codex.rs");
```

含义是两个二进制共享同一套 CLI 代码。CI 推荐调用 `codex-auto-fix pr-auto-fix`，这样 `CODEX_AGENT_COMMAND` 可以继续指向真实 Codex CLI 的 `codex exec ...`，减少命名冲突。

安装后先跑：

```bash
codex-auto-fix doctor
codex-auto-fix doctor --json
```

`doctor` 会检查：

- `codex-auto-fix` 是否在 PATH。
- `codex` 是否在 PATH。
- 当前运行二进制是否比源码新。
- `CODEX_AGENT_COMMAND` 是否配置。
- `git`、`gh`、`jq`、`cargo` 是否存在。

如果 `source.freshness` 是 warning，通常修复方式是：

```bash
cd scripts/codex-cli
cargo install --path . --force
```

## 3. CLI 命令入口

入口文件：`src/bin/codex.rs`

使用 `clap::{Parser, Subcommand}` 定义 `Cli` 和 `Commands`。所有命令都在 `main()` 的 `match &cli.command` 中分发。

### 3.1 review

```bash
codex review --path src/lib.rs
codex review --path src/lib.rs --fix
```

实现细节：

1. 创建 `CodexClient::new()`。
2. 用 `repo::read_agents_rules()` 读取当前 git 根目录的 `AGENTS.md`。
3. 读取目标文件内容。
4. 拼 system prompt 和 user prompt。
5. 调 `client.call(...)`。
6. 如果 `--fix` 为 true，则用 `repo::extract_code_block` 从模型输出取第一个代码块并写回原文件。

风险点：

- 这是单文件辅助命令，不保证 stdout 是 JSON。
- `--fix` 是整文件写回，适合本地实验，不是生产 PR 自动修复主路径。

### 3.2 refactor

```bash
codex refactor --path src/lib.rs --strategy modularize
```

实现方式与 `review --fix` 类似：读文件、调用模型、提取代码块、写回原文件。策略只是 prompt 文本，不是代码里的枚举。

### 3.3 doc

```bash
codex doc --path src/lib.rs --kind api
```

读取单文件，调用模型生成 Markdown，直接输出到 stdout，不写文件。

### 3.4 pr-auto-fix

```bash
codex-auto-fix pr-auto-fix \
  --pr-number 123 \
  --review-text "$REVIEW" \
  --max-rounds 2 \
  --yes \
  --repo-root "$GITHUB_WORKSPACE"
```

也兼容旧参数：

```bash
--gemini-review "$REVIEW"
```

主输入三选一：

- `--review-json <path>`：推荐机器输入，跳过模型解析 Review 文本。
- `--review-text <text>`：通用 Review Markdown 文本。
- `--gemini-review <text>`：旧命名兼容。

参数互斥规则：

- `--review-text` 和 `--gemini-review` 不能同时出现。
- 三种输入都缺失时退出码为 2。

PR 模式默认：

- `repo_root` 来自 `--repo-root`，否则 `GITHUB_WORKSPACE`，否则 `.`。
- `enable_pr_comments = !--no-pr-comments`。
- `auto_push = --yes`。
- stdout 只输出最终 JSON。
- 过程日志全部走 stderr。

### 3.5 auto-fix-local

```bash
codex-auto-fix auto-fix-local \
  --repo-root /abs/path/to/repo \
  --review-file /abs/path/to/review.md \
  --max-rounds 2 \
  --yes \
  --disable-changelog
```

本地模式不依赖 GitHub PR，不发 PR 评论。输入可以是：

- `--review-json`
- `--review-text`
- `--review-file`

`--review-text` 和 `--review-file` 不能同时出现。

### 3.6 review-to-json

```bash
codex-auto-fix review-to-json --input review.md --output review.json
```

调用 `review_json::convert_review_file`，把标准化 Review Markdown 转成 `StructuredReview` JSON。该命令不调用模型。

### 3.7 auto-fix-weekly-report

```bash
codex-auto-fix auto-fix-weekly-report --input runs.jsonl --output report.md
```

读取一条 JSON、JSON 数组或 JSONL，按 `apply_fail_reason + file + fallback_used + final_status` 聚合 Top 5 失败样本。

### 3.8 skill-pack

```bash
codex-auto-fix skill-pack list --plugin-root scripts/codex-cli
codex-auto-fix skill-pack run --plugin-root scripts/codex-cli --skill codex-cli-workflow --input-file input.md
```

`list` 扫描 `skills/*/SKILL.md`。`run` 会读取插件根目录的 `AGENTS.md` 和目标 `SKILL.md`，拼成 system prompt 后调用 `CodexClient`。

插件根目录解析优先级：

1. `--plugin-root`
2. `CODEX_SKILL_PACK_ROOT`
3. 从当前目录向上查找 `.claude-plugin/plugin.json`

## 4. 模型执行器

入口文件：`src/llm.rs`

`CodexClient` 内部字段：

```rust
pub struct CodexClient {
    command: Vec<String>,
    timeout: Duration,
}
```

创建流程：

1. `dotenv().ok()` 加载 `.env`。
2. 读取必填环境变量 `CODEX_AGENT_COMMAND`。
3. `parse_command` 用空白拆分命令。
4. `reject_recursive_command` 防止命令指向当前二进制。
5. `agent_timeout` 读取 `CODEX_AGENT_TIMEOUT_SECONDS`，默认 900 秒。

Prompt 构造：

```text
System instructions:
<system_prompt>

User request:
<user_prompt>
```

Prompt 传递方式：

| `CODEX_AGENT_COMMAND` 内容 | 行为 |
| --- | --- |
| 不含占位符 | prompt 写入子进程 stdin |
| 含 `{prompt}` | prompt 替换为一个命令参数 |
| 含 `{prompt_file}` | prompt 写入临时 Markdown 文件，参数替换为文件路径 |

子进程执行细节：

- 使用 `tokio::process::Command`。
- stdout、stderr 都被捕获。
- `kill_on_drop(true)` 防止超时时遗留进程。
- 超时返回 `本地 Codex 命令超时（N 秒）`。
- 非 0 退出码返回 `本地 Codex 命令失败，status=...，stderr=...`。
- 成功时只返回 stdout。

常见修复：

```bash
export CODEX_AGENT_COMMAND="codex exec --skip-git-repo-check -"
export CODEX_AGENT_TIMEOUT_SECONDS=1200
codex-auto-fix doctor
```

如果报递归调用，说明 `CODEX_AGENT_COMMAND` 指到了 `codex-cli` 生成的 `codex` 或 `codex-auto-fix` 本身。改成真实 Codex CLI 的绝对路径或调整 PATH。

GitHub Actions 的 `Codex Auto Fix (本地 Runner)` 在进入长时间
`pr-auto-fix` 前会执行 `codex-auto-fix doctor --json`。只要
`agent.command` 返回 `warning`，workflow 就会停止并提示人工修正
`CODEX_AGENT_COMMAND`，而不是让本地 Runner 递归调用到超时。

## 5. 数据结构与 JSON 契约

入口文件：`src/types.rs`

### 5.1 ReviewIssue

```rust
pub struct ReviewIssue {
    pub file: String,
    pub line: Option<u32>,
    pub severity: String,
    pub description: String,
    pub suggestion: String,
    pub constraints: Vec<String>,
    pub reason: Option<String>,
}
```

含义：

- `file`：仓库相对路径。
- `line`：问题起始行，可为空。
- `severity`：`Critical`、`High`、`Medium+`、`Medium`、`Low` 或带 `Priority` 后缀的等价文本。
- `description`：Review 问题正文。
- `suggestion`：模型或 Review 给出的预期修复。
- `constraints`：比如 `Only modify src/lib.rs`。
- `reason`：原 Review 片段或结构化 issue id，用于溯源。

### 5.2 ReviewData

```rust
pub struct ReviewData {
    pub summary: String,
    pub issues: Vec<ReviewIssue>,
}
```

`ReadReviewSkill` 输出，或由 `review_json::read_review_data_file` 从结构化 JSON 转换而来。

### 5.3 StructuredReview

`review-to-json` 使用的稳定输入格式：

```rust
pub struct StructuredReview {
    pub review_id: String,
    pub summary: String,
    pub issues: Vec<StructuredReviewIssue>,
}
```

`StructuredReviewIssue` 包含：

- `id`
- `severity`
- `file`
- `line`
- `rule`
- `problem`
- `expected`
- `constraints`
- `acceptance`

读入自动修复时会转换为 `ReviewIssue`：

- `expected` 进入 `suggestion`。
- `constraints` 同时保留在 `constraints`，并追加到 `suggestion` 文本。
- `id / rule` 合并进 `reason`。

### 5.4 PrAutoFixOutput

自动修复命令 stdout 的唯一机器可读输出：

```rust
pub struct PrAutoFixOutput {
    pub fixed: bool,
    pub files: Vec<String>,
    pub quality_score: u8,
    pub quality_score_available: bool,
    pub security_passed: bool,
    pub push_blocked: bool,
    pub has_pending: bool,
    pub pending_count: usize,
    pub review_clean: bool,
    pub apply_fail_reason: Option<String>,
    pub retry_count: usize,
    pub fallback_used: bool,
    pub final_status: String,
    pub summary: Option<String>,
    pub review_record_path: Option<String>,
    pub fixed_explanations: Vec<String>,
    pub pending_explanations: Vec<String>,
    pub issue_statuses: Vec<ReviewIssueStatus>,
}
```

字段解释：

| 字段 | 含义 |
| --- | --- |
| `fixed` | Dry-run 下表示是否应用了本地变更；`--yes` 下还要求未被 push policy 阻塞 |
| `files` | 本轮修改文件，排序去重 |
| `quality_score` | 0 到 100 |
| `quality_score_available` | 质量评分是否成功解析 |
| `security_passed` | prompt-based 安全审计是否通过 |
| `push_blocked` | 当前实现主要由安全或策略状态影响，true 时需要人处理 |
| `has_pending` | 是否存在 Medium 以上未自动解决问题 |
| `pending_count` | 未解决说明数量 |
| `review_clean` | 安全通过、未阻塞、无 pending |
| `apply_fail_reason` | `malformed_diff`、`context_mismatch`、`drift`、`unknown` 等 |
| `retry_count` | patch retry 次数 |
| `fallback_used` | 是否使用完整文件兜底成功 |
| `final_status` | `clean`、`pending`、`needs-human` |
| `summary` | Review 总结 |
| `review_record_path` | ledger 路径，通常是 `docs/auto-review-ledger.md` |
| `fixed_explanations` | 已修复问题说明 |
| `pending_explanations` | 未修复原因 |
| `issue_statuses` | 每条 Medium 以上问题的一对一状态 |

修改该结构是破坏性风险最高的操作之一。新增字段相对安全；删除或改名会破坏 workflow 的 `jq` 解析。

## 6. Runtime 与 Pipeline

入口文件：`src/runtime.rs`, `src/pipeline.rs`

### 6.1 AutoFixOptions

```rust
pub struct AutoFixOptions {
    pub repo_root: String,
    pub rules_file: Option<String>,
    pub changelog_path: Option<String>,
    pub disable_changelog: bool,
    pub enable_pr_comments: bool,
}
```

CLI 参数会被压成这个结构，再传给 runtime。

### 6.2 PR 模式

`pr_auto_fix_with_options_and_review_data` 做这些事：

1. 从 `GITHUB_REPOSITORY` 读仓库名。
2. `repo::read_rules(Some(repo_root), rules_file)` 读取规则。
3. 构造 `SkillContext`。
4. 如果提供了 `--review-json`，把 `ctx.parsed_data` 预置为结构化数据。
5. 调用 `run_auto_fix_loop`。

### 6.3 本地模式

`auto_fix_local_with_review_data` 与 PR 模式几乎相同，区别是：

- `pr_number = 0`
- `repo = ""`
- 通常 `enable_pr_comments = false`

### 6.4 run_auto_fix_loop

实际主流程：

```text
fix_pipeline:
  ReadReviewSkill
  DecisionSkill
  BatchFixSkill

post_fix_pipeline:
  SecurityCheckSkill
  QualityScoreSkill
  DocumentationSkill

feedback_pipeline:
  DryRunFeedbackSkill
  FeedbackSkill
```

执行细节：

1. `ctx.current_round = 1`。
2. 如果 `max_rounds > 1`，只输出提示，当前命令不会在内部请求多轮外部 Review。
3. 执行修复 pipeline。
4. 如果没有产生修复文件，跳过安全、评分、文档。
5. 如果安全检查发现可转换为 review issue 的 finding，会执行一次安全修复补丁轮。
6. 执行 `enforce_review_policy`，确保每条 Medium 以上问题都有 resolved 或 pending 说明。
7. 把安全 finding 加入 pending。
8. 写 `docs/auto-review-ledger.md` 和 per-PR ledger。
9. 执行反馈 pipeline。
10. 汇总 `PrAutoFixOutput`。

### 6.5 Pipeline

`Pipeline` 很薄：

```rust
pub struct Pipeline {
    skills: Vec<Box<dyn Skill>>,
}
```

`run` 只做：

- 按顺序遍历 skill。
- 每个 skill 执行前向 stderr 打印 `🚀 [Skill: name] 正在执行...`。
- 调用 `skill.execute(ctx, client).await?`。

不要把业务判断塞进 `pipeline.rs`。新增步骤时改 `runtime.rs` 的组装顺序。

## 7. SkillContext

入口文件：`src/skills.rs`

`SkillContext` 是流水线共享状态。重要字段如下：

| 字段 | 写入方 | 读取方 | 含义 |
| --- | --- | --- | --- |
| `pr_number` | runtime | feedback、ledger | PR 号，本地模式为 0 |
| `repo` | runtime | ReadReviewSkill | GitHub 仓库名上下文 |
| `repo_root` | runtime | BatchFix、Security、Repo | 目标仓库根目录 |
| `rules_text` | runtime | prompt 生成 | AGENTS 或 rules-file 内容 |
| `raw_input` | CLI/runtime | ReadReview、QualityScore | 原始 Review 文本 |
| `parsed_data` | ReadReview 或 review-json | Decision、policy、feedback | 结构化 Review |
| `selected_issues` | Decision | BatchFix、policy | 允许自动修复的问题 |
| `fixed_files` | BatchFix、Documentation、ledger | Security、Quality、Feedback、output | 本轮修改文件 |
| `fixed_issue_keys` | BatchFix | status 生成 | 已解决 issue key |
| `quality_score` | QualityScore | Feedback、output | 质量评分 |
| `quality_score_available` | QualityScore | Feedback、output | 评分是否可用 |
| `security_passed` | SecurityCheck | runtime、Feedback、output | 安全检查是否通过 |
| `security_findings` | SecurityCheck | runtime、Feedback | 安全问题 |
| `push_blocked` | policy / security path | output、Feedback | 是否阻止自动推送 |
| `auto_push` | CLI | Feedback、output | 是否允许提交/推送 |
| `enable_pr_comments` | CLI | Feedback | 是否发 PR 评论 |
| `changelog_path` | CLI | Documentation | changelog 覆盖路径 |
| `disable_changelog` | CLI | Documentation、ledger | 是否跳过 changelog/ledger |
| `max_rounds` | CLI | runtime log | 外层 workflow 最大轮次提示 |
| `current_round` | runtime | attempts、changelog | 当前处理轮次 |
| `fix_attempts` | BatchFix | policy、output | 每次补丁尝试记录 |
| `pending_explanations` | policy/security | Feedback、output | 未解决原因 |

`FixAttempt` 记录：

```rust
pub struct FixAttempt {
    pub round: u8,
    pub issue_key: String,
    pub file: String,
    pub stage: String,
    pub success: bool,
    pub reason: Option<String>,
}
```

常见 `stage`：

- `patch_generation`
- `patch_apply`
- `patch_generation_retry`
- `patch_apply_retry`
- `file_replacement_fallback`

## 8. 各 Skill 实现细节

### 8.1 ReadReviewSkill

职责：把 Markdown Review 解析成 `ReviewData`。

如果 `ctx.parsed_data` 已经有值，说明用户传了 `--review-json`，直接跳过模型解析。

否则调用私有函数 `read_gemini_review`：

- system prompt 要求严格 JSON。
- JSON schema 包含 `summary` 和 `issues`。
- 支持去掉 ```json 包裹。
- 解析失败会返回 `解析 Gemini Review JSON 失败`，并附带模型原始输出。

排障：

- 如果这里失败，优先改输入为 `--review-json`，用 `review-to-json` 生成稳定 JSON。
- 如果必须解析 Markdown，检查模型输出是否混入解释文字、尾随逗号或代码块不完整。

### 8.2 DecisionSkill

职责：筛出允许自动修复的问题。

核心函数：`decide_fix_or_skip`

默认策略：

- 允许严重级别：`Critical,High,Medium+,Medium`。
- `Medium Priority` 等带 `Priority` 后缀的文本会归一化。
- `Medium+` 在 allowed 只有 `medium` 时也视为允许。
- 保护文件包含 `Cargo.lock`、`package-lock.json`、`bun.lock`、`Cargo.toml`、`package.json`、`pyproject.toml`、`.env`。
- 默认排除 `docs/` 和 `*.md`。

相关环境变量：

```bash
CODEX_ALLOWED_SEVERITIES="Critical,High,Medium+,Medium"
CODEX_EXCLUDE_DOCS=true
CODEX_PROTECTED_FILES="extra.lock,.env.local"
```

如果 review 明明有问题但输出 `未选择自动修复问题`，先检查：

1. severity 是否被 `review_severity_token` 归一化后不在 allowed 集合。
2. 文件是否命中 protected 列表。
3. 文件是否是 docs 或 Markdown。

### 8.3 BatchFixSkill

职责：对 `selected_issues` 逐条生成补丁并应用。

每条 issue 的流程：

1. 计算 `issue_key = file:line:severity:description`。
2. 调 `generate_fix_patch`。
3. 如果模型返回空或格式未知，记录失败，继续下一条。
4. 调 `apply_generated_patch`。
5. 成功时把文件加入 `fixed_files`，把 issue key 加入 `fixed_issue_keys`，记录成功 attempt。
6. 失败时按原因重试。
7. 如果失败是 `malformed_diff`，直接进入完整文件兜底。
8. 否则调用 `generate_retry_fix_patch` 生成更小补丁。
9. 重试仍失败时进入完整文件兜底。

注意：单条 issue 失败不会中断整个 pipeline，这是为了让其他问题仍可被修复。

### 8.4 generate_fix_patch

补丁格式由 `CODEX_PATCH_FORMAT` 决定：

| 值 | 行为 |
| --- | --- |
| `auto` | 当前等价于 SEARCH/REPLACE 主路径 |
| `search_replace`, `search-replace`, `sr` | 强制 SEARCH/REPLACE |
| `unified_diff`, `unified-diff`, `diff` | 强制 unified diff |

默认 SEARCH/REPLACE prompt 来自 `prompts::search_replace_system_prompt`，硬约束包括：

- 必须先输出 `### File: <allowed-file>`。
- 每个变更使用 `<<<<<<< SEARCH`、`=======`、`>>>>>>> REPLACE`。
- SEARCH 块必须来自当前源码。
- `max blocks` 来自 `CODEX_SR_MAX_BLOCKS`，默认 5。
- 禁止 unified diff。
- 禁止解释文字。
- 禁止修改 allowed file 以外文件。

模型输出如果包含 Markdown 代码块，会先用 `repo::extract_code_block` 提取。

### 8.5 apply_full_file_fallback

完整文件兜底是最后手段，路径必须通过保护规则。

先检查 `full_file_fallback_block_reason`：

- 如果命中 protected 文件，拒绝。
- 如果路径不在允许前缀，拒绝。

默认允许前缀：

```text
src/,backend/src/,frontend/src/,scripts/,.github/scripts/
```

可用环境变量覆盖：

```bash
CODEX_FULL_FILE_FALLBACK_ALLOWED_PREFIXES="src/,backend/src/"
```

兜底并不是立即整文件写回，而是先尝试让模型生成 SEARCH/REPLACE 兜底：

1. `generate_replacement_via_search_replace`
2. 若可应用，记录 `file_replacement_fallback` 成功
3. 否则调用 `generate_replacement_file`
4. 确认返回内容不是 patch，不为空，且与原文不同
5. `repo::write_repo_file` 写回完整文件

### 8.6 SecurityCheckSkill

职责：对 `fixed_files` 做 prompt-based 安全审计。

每个文件：

1. `repo::read_repo_file` 读内容。
2. 要求模型只输出 JSON：

```json
{"passed": true, "reason": "原因"}
```

3. `parse_security_audit` 清理并解析。
4. 解析失败按不通过处理，reason 包含原始输出。
5. 任一文件不通过时 `ctx.security_passed = false`。

这是软审计，不替代真实 SAST。当前 runtime 会尝试把安全 finding 转成 High issue，进行一次安全修复轮。

### 8.7 QualityScoreSkill

职责：让模型根据 AGENTS 规则给修复打 0 到 100 分。

解析规则：

- 直接数字如 `87` 可接受。
- JSON `{"score": 91, "reason": "ok"}` 可接受。
- 大于 100 拒绝。
- 首次解析失败会重试一次。
- 两次失败则 `quality_score_available = false`，但 pipeline 不崩溃。

### 8.8 DocumentationSkill

职责：写 changelog。

跳过条件：

- `ctx.disable_changelog = true`
- `ctx.fixed_files.is_empty()`

写入路径：

- 如果传了 `--changelog-path`，使用该路径。
- 相对路径会拼到 `repo_root`。
- 否则默认查找 `<repo_root>/docs/CHANGELOG.md`。
- 默认 changelog 不存在时静默跳过。

写入后会把 changelog 相对路径加入 `fixed_files`，确保后续提交包含文档记录。

### 8.9 DryRunFeedbackSkill

触发条件：

- `auto_push = false`
- `fixed_files` 非空
- `enable_pr_comments = true`

行为：发 PR 评论，告诉人类补丁已经本地应用但未推送。

本地模式默认 `enable_pr_comments = false`，所以不会调用 `gh pr comment`。

### 8.10 FeedbackSkill

如果 `fixed_files` 非空：

- `auto_push = false` 时直接返回，因为 DryRun 已处理。
- `auto_push = true` 时调用 `repo::commit_and_push_in`。
- 构造 PR 评论或 stderr 输出，包含安全状态、质量评分、修复尝试摘要、已修复文件和 issue 状态表。

如果 `fixed_files` 为空但有 `parsed_data`：

- 没有 pending 时评论“未发现需要自动修复的高优先级问题”。
- 有 pending 时评论未解决说明。

## 9. Patch 子系统

入口文件：`src/patch/mod.rs`, `src/patch/search_replace.rs`, `src/patch/unified_diff.rs`

### 9.1 格式检测

`detect_format` 返回：

- `Empty`
- `SearchReplace`
- `UnifiedDiff`
- `Mixed`
- `Unknown`

规则：

- 包含 `<<<<<<< SEARCH` 视为 SEARCH/REPLACE。
- 包含 `diff --git ` 视为 unified diff。
- 两者都有是 `Mixed`，当前应用时优先走 SEARCH/REPLACE。

### 9.2 SEARCH/REPLACE 解析

格式必须是：

```text
### File: src/example.rs
<<<<<<< SEARCH
旧代码
=======
新代码
>>>>>>> REPLACE
```

`parse_search_replace_blocks` 先调用 `validate_file_header`：

- 必须存在 `### File:`。
- 文件必须等于当前 allowed file。

解析状态机：

```text
Outside
  -> 看到 <<<<<<< SEARCH 后进入 Search
Search
  -> 看到 ======= 后进入 Replace
Replace
  -> 看到 >>>>>>> REPLACE 后保存 block 并回到 Outside
```

错误：

- 缺 `=======`：`malformed SEARCH/REPLACE block: missing ======= separator`
- 缺结束标记：`malformed SEARCH/REPLACE block: missing >>>>>>> REPLACE marker`
- 没有 block：`no SEARCH/REPLACE blocks found`
- block 数超过 `CODEX_SR_MAX_BLOCKS`：拒绝

### 9.3 SEARCH 匹配

`find_unique_match` 三层匹配：

1. Exact：原始字节完全匹配。
2. TrimTrailingWhitespace：逐行去掉行尾空白后匹配。
3. NormalizeIndent：归一化缩进并去掉行尾空白后匹配。

每层必须唯一匹配。

常见错误：

- `SEARCH block is ambiguous: N exact matches`
- `SEARCH block is ambiguous: N trimmed-line matches`
- `SEARCH block is ambiguous: N normalized-indent matches`
- `SEARCH block was not found in the allowed file`

### 9.4 多 block 应用

`apply_search_replace_in` 对每个 block 先在原始文件上定位，收集 `PlannedEdit`：

```rust
struct PlannedEdit {
    start: usize,
    end: usize,
    replace: String,
    order: usize,
}
```

所有 edit 按 `(start, end, order)` 排序，然后 `reject_overlapping_edits` 检查重叠。

允许多个插入点相同的空 SEARCH 插入；不允许普通替换范围重叠。

最终一次性构造新字符串并写回文件，避免第一个 block 修改后影响第二个 block 的偏移。

### 9.5 unified diff 兼容路径

`apply_patch_with_details_in` 先 `validate_unified_diff_for_file`，再写临时 patch 并执行：

```bash
git -C <repo_root> apply --whitespace=fix <tmp.patch>
```

preflight 检查：

- patch 非空。
- 必须以 `diff --git ` 开头。
- 只能有一个 `diff --git` header。
- header、`---`、`+++` 都必须指向当前 allowed file。
- 必须有 `@@` hunk。
- hunk header 必须可解析。
- hunk body 的 old/new 行数必须与 header 一致。
- hunk 内不能出现新的 file header。

失败分类：

| 分类 | 触发 |
| --- | --- |
| `malformed_diff` | corrupt patch、malformed、unrecognized input、patch fragment without header |
| `context_mismatch` | patch does not apply、patch failed |
| `drift` | no such file、does not exist、not in index |
| `unknown` | 其他 |

## 10. Repo / IO 副作用

入口文件：`src/repo.rs`

### 10.1 规则读取

`read_rules(repo_root, rules_file)` 优先级：

1. 显式 `rules_file`
2. `<repo_root>/AGENTS.md`
3. 内置兜底：`严格遵循项目架构铁律和 TDD 铁律。`

如果修复行为没有遵守项目规则，先检查实际传入的 `--repo-root` 和 `--rules-file`。

### 10.2 文件读写

```rust
read_repo_file(repo_root, path)
write_repo_file(repo_root, path, content)
```

这两个函数直接 `Path::new(repo_root).join(path)`。调用方必须保证 `path` 是安全的仓库相对路径。

### 10.3 changelog

`build_changelog_entry` 生成稳定格式：

```text
#### 🤖 Codex Auto-Fix (PR #123, round 1) — ts=...

- 安全扫描：通过
- 质量评分：95 / 100
- 变更文件：
  - `src/a.rs`
```

`update_changelog` 插入策略：

1. 如果已有 `### 🤖 AI 自动修复`，插在该标题后面。
2. 否则如果有 `## [未发布]`，在其下创建小节。
3. 否则追加到文末。

### 10.4 ledger

`append_auto_review_ledger_in` 写两个文件：

- `docs/auto-review-ledger.md`
- `docs/auto-review-ledgers/pr-<number>.md` 或本地模式 `docs/auto-review-ledgers/local.md`

写入内容包括：

- PR / local 标签。
- round。
- unix timestamp。
- summary。
- 修改文件。
- 每条 Medium 以上问题的状态表。

### 10.5 commit and push

`commit_and_push_in(repo_root, fixed_files, push)`：

1. 构造提交信息：

```text
🤖 codex auto-fix: 修复 N 个文件 (基于 Gemini Review)
```

2. `git -C <repo_root> add <fixed_files...>`
3. `git -C <repo_root> commit -m <msg>`
4. 如果 `push = true`，执行 `git push origin HEAD`

推送兜底：

- 如果 `CODEX_PUBLISH_VIA_GH_API=true`，跳过 git push，直接用 GitHub API 发布当前提交。
- 如果 git push 因网络错误三次失败，也尝试 GitHub API fallback。

可重试网络错误包括：

- Empty reply from server
- failed to connect
- connection reset
- timeout
- unexpected disconnect
- HTTP/2 stream
- TLS

### 10.6 gh API fallback

fallback 大致流程：

1. 获取 repo：`GITHUB_REPOSITORY` 或 `gh repo view`。
2. 获取当前分支：`git branch --show-current` 或 `GITHUB_HEAD_REF`。
3. 读取远端 ref 的 commit sha。
4. 读取 base tree sha。
5. 用当前 HEAD 的 changed files 构造 Git tree entries。
6. POST 新 tree。
7. POST 新 commit。
8. PATCH 远端 branch ref。

如果 fallback 失败，检查：

- `gh auth status`
- `GITHUB_TOKEN` 权限
- 当前分支是否可确定
- 分支保护是否阻止更新

### 10.7 PR 评论

`post_comment(pr_number, body)` 调：

```bash
gh pr comment <pr_number> --body <body>
```

同样使用网络重试包装。

## 11. review_json 实现

入口文件：`src/review_json.rs`

用途：把标准 Markdown Review 或 inline comment 风格文本转成稳定 JSON，避免每次都让模型解析大段 Markdown。

### 11.1 标准字段解析

支持形如：

```markdown
- severity: Medium
- file: src/lib.rs
- line: 42
- rule: no-panic
- problem: panic may crash runner
- expected: return Result instead
- constraints:
  - only modify src/lib.rs
```

`parse_structured_review` 内部维护 `PendingIssue`。只有 `severity`、`file`、`line`、`problem`、`expected` 都存在时才会生成 issue。

### 11.2 inline comment 解析

支持形如：

~~~markdown
### src/lib.rs:42
![medium](...)
comment body

```suggestion
replacement
```
~~~

解析逻辑：

- `parse_inline_heading` 从 `### file:line` 取文件和行号。
- `parse_severity_badge` 从图片 alt 中识别 severity。
- suggestion 代码块进入 `expected`。
- 非 suggestion 文本进入 `problem`。

### 11.3 输出

`convert_review_file`：

1. 读 input Markdown。
2. `parse_structured_review`。
3. 为 issue 填 `ISSUE-001`、`ISSUE-002`。
4. 写 pretty JSON 到 output。
5. stderr 输出 `ok: wrote ... with N issues`。
6. stdout 返回 JSON 文本。

## 12. doctor 实现

入口文件：`src/doctor.rs`

`build_report` 返回：

```rust
pub struct DoctorReport {
    pub package: &'static str,
    pub version: &'static str,
    pub status: CheckStatus,
    pub current_exe: String,
    pub manifest_dir: String,
    pub reinstall_hint: String,
    pub checks: Vec<DoctorCheck>,
}
```

状态只有：

- `ok`
- `warning`

任何 check warning，整体就是 warning。

核心 checks：

- `path.codex-auto-fix`
- `path.codex`
- `source.freshness`
- `agent.command`
- `dependency.git`
- `dependency.gh`
- `dependency.jq`
- `dependency.cargo`

`source.freshness` 会比较当前二进制 mtime 和 `Cargo.toml`、`Cargo.lock`、`src/**` 最新 mtime。如果二进制更老，就提示重装。

## 13. 环境变量总表

| 变量 | 默认值 | 使用位置 | 作用 |
| --- | --- | --- | --- |
| `CODEX_AGENT_COMMAND` | 无，必填 | `llm.rs` | 本地 Codex 执行命令 |
| `CODEX_AGENT_TIMEOUT_SECONDS` | `900` | `llm.rs` | 单次模型调用超时 |
| `CODEX_ALLOWED_SEVERITIES` | `Critical,High,Medium+,Medium` | `skills.rs` | 自动修复 severity 过滤 |
| `CODEX_EXCLUDE_DOCS` | `true` | `skills.rs` | 是否排除 `docs/` 和 `*.md` |
| `CODEX_PROTECTED_FILES` | 追加到内置列表 | `skills.rs` | 额外保护路径片段 |
| `CODEX_PATCH_FORMAT` | `auto` | `skills.rs` | patch prompt 格式 |
| `CODEX_SR_MAX_BLOCKS` | `5` | `search_replace.rs`, `skills.rs` | SEARCH/REPLACE block 上限 |
| `CODEX_FULL_FILE_FALLBACK_ALLOWED_PREFIXES` | `src/,backend/src/,frontend/src/,scripts/,.github/scripts/` | `skills.rs` | 整文件兜底允许前缀 |
| `CODEX_SKILL_PACK_ROOT` | 无 | `codex.rs` | Skill Pack 根目录 |
| `CODEX_PUBLISH_VIA_GH_API` | `false` | `repo.rs` | 强制 GitHub API 发布提交 |
| `CODEX_REMOTE_RETRY_SLEEP_SECONDS` | attempt * 10 | `repo.rs` | 远端命令重试等待秒数 |
| `GITHUB_WORKSPACE` | `.` fallback | `runtime.rs`, `codex.rs` | PR 模式 repo root |
| `GITHUB_REPOSITORY` | 空字符串 | `runtime.rs`, `repo.rs` | GitHub repo full name |
| `GITHUB_HEAD_REF` | 无 | `repo.rs` | API fallback 分支名兜底 |

## 14. stdout / stderr 契约

自动化命令：

- `pr-auto-fix`
- `auto-fix-local`

必须保持：

- stdout 只输出最终 JSON。
- 过程日志、错误、进度全部走 stderr。

原因：GitHub Actions 常用 `jq` 解析 stdout。一旦 stdout 混入日志，workflow 会把结果当成非法 JSON。

辅助命令：

- `review`
- `refactor`
- `doc`
- `doctor`
- `skill-pack list`
- `skill-pack run`
- `review-to-json`
- `auto-fix-weekly-report`

这些可以输出人类可读文本，其中 `doctor --json` 和 `skill-pack list --json` 是结构化输出。

## 15. 故障排查手册

### 15.1 `CODEX_AGENT_COMMAND` 未配置

症状：

```text
请设置 CODEX_AGENT_COMMAND，例如：codex exec --skip-git-repo-check -
```

修复：

```bash
export CODEX_AGENT_COMMAND="codex exec --skip-git-repo-check -"
codex-auto-fix doctor
```

如果在 GitHub Actions，确认 env 写在调用步骤同一作用域。

### 15.2 本地 Codex 命令递归

症状：

```text
CODEX_AGENT_COMMAND 指向了当前 codex-cli 二进制，会导致递归调用
```

原因：PATH 上的 `codex` 是本 crate 构建出来的 `codex`，不是实际 Codex CLI。
`CODEX_AGENT_COMMAND=codex-auto-fix ...` 或
`cargo run --bin codex-auto-fix ...` 会被 `doctor` 直接标记为递归配置。

修复：

```bash
which codex
which codex-auto-fix
export CODEX_AGENT_COMMAND="/abs/path/to/real/codex exec --skip-git-repo-check -"
```

CI 中优先调用 `codex-auto-fix` 二进制，避免和真实 `codex` 命令同名。

### 15.3 模型调用超时

症状：

```text
本地 Codex 命令超时（900 秒）
```

修复：

```bash
export CODEX_AGENT_TIMEOUT_SECONDS=1800
```

同时检查 prompt 是否过大。大 Review 建议先用 `review-to-json` 做结构化输入。

### 15.4 Review JSON 解析失败

症状：

```text
解析 Gemini Review JSON 失败
```

修复路径：

1. 保存原 Review 到 `review.md`。
2. 运行：

```bash
codex-auto-fix review-to-json --input review.md --output review.json
```

3. 改用：

```bash
codex-auto-fix auto-fix-local --repo-root "$PWD" --review-json review.json
```

如果 `review-to-json` 输出 0 issues，检查 Markdown 是否包含标准字段或 inline heading。

### 15.5 没有选中任何问题

症状：

输出 JSON：

```json
{
  "fixed": false,
  "security_passed": true,
  "quality_score_available": false,
  "final_status": "clean"
}
```

或日志：

```text
跳过：本轮未选择自动修复问题
```

检查：

- severity 是否是 Low。
- 文件是否在 `docs/` 或以 `.md` 结尾。
- 文件是否命中 protected 列表。
- `CODEX_ALLOWED_SEVERITIES` 是否覆盖得太窄。

临时允许文档修复：

```bash
CODEX_EXCLUDE_DOCS=false codex-auto-fix auto-fix-local ...
```

### 15.6 模型没有返回 patch

症状：

```text
模型未返回可应用的 SEARCH/REPLACE 或 unified diff
```

原因：

- 模型输出解释文字，没有 patch。
- 输出了普通代码块。
- SEARCH/REPLACE 缺 marker。

修复：

- 检查 `CODEX_PATCH_FORMAT`，建议保持默认 `auto` 或显式 `search_replace`。
- 缩小 Review issue，让 suggestion 更具体。
- 如果是 prompt 问题，修改 `src/prompts.rs` 并补测试。

### 15.7 `missing SEARCH/REPLACE file header`

原因：模型没输出：

```text
### File: <allowed-file>
```

修复：

- 优先调整 prompt，强调第一行必须是文件头。
- 检查模型是否把 patch 包在说明文字中。`extract_code_block` 只取第一个代码块，如果第一个不是 patch，会失败。

### 15.8 `does not match allowed file`

原因：模型试图改非当前 issue 文件。

修复：

- 不要放宽这个限制，除非设计上允许跨文件修复。
- 如果确实需要跨文件修复，应把 Review 拆成多个 issue，每个 issue 单文件处理。

### 15.9 `SEARCH block is ambiguous`

原因：SEARCH 片段在目标文件出现多次。

修复：

- 重试 prompt 已要求增加上下文。
- 如果仍失败，说明模型给的 SEARCH 太短。人工修复时应复制更多上下文，至少包含函数签名或唯一邻近代码。

代码入口：

- `search_replace.rs::find_unique_match`
- `skills.rs::generate_retry_fix_patch`

### 15.10 `SEARCH block was not found`

原因：

- 源码漂移。
- 模型复制了旧代码。
- 行尾、缩进归一化后仍找不到。

修复：

```bash
git -C <repo_root> diff
git -C <repo_root> status
```

确认 Review 对应的是当前 checkout。CI 中要确保 checkout 到 PR head，而不是 base branch。

### 15.11 `SEARCH/REPLACE blocks overlap`

原因：同一次 patch 的多个 block 修改范围重叠。

修复：

- 让模型合并为一个更大的 block。
- 或拆成两条不同 issue，不要在同一 patch 里重复改同一段。

### 15.12 unified diff `malformed_diff`

原因：

- 缺 `diff --git`。
- hunk header 格式不完整。
- hunk 行数与 header 不一致。
- patch 指向了其他文件。

修复：

- 默认不要强制 `CODEX_PATCH_FORMAT=unified_diff`。
- 如果必须使用 unified diff，先用 `validate_unified_diff_for_file` 的错误信息定位格式问题。

### 15.13 `context_mismatch`

原因：diff patch 上下文与当前文件不匹配。

修复：

- 更新 checkout 到 review 对应 commit。
- 改用 SEARCH/REPLACE。
- 检查是否已有前一条 issue 修改了同一段。

### 15.14 `drift`

原因：文件不存在、不在 index 或路径漂移。

修复：

```bash
git -C <repo_root> ls-files | rg '<file>'
```

如果 review 指向重命名前的文件，只能人工确认。

### 15.15 安全检查失败

症状：

```json
"security_passed": false
```

并且 `pending_explanations` 中有 `[Security]`。

修复：

1. 查看 `security_findings` 对应文件。
2. 如果 finding 可修复，确认安全修复轮是否产生 patch。
3. 如果是误报，人工 review 后可以调整安全 prompt 或后续策略，但不要直接忽略。

### 15.16 质量评分不可用

症状：

```json
"quality_score_available": false
```

原因：

- 模型输出非数字且非 JSON。
- JSON 里 `score` 超过 100。
- 两次解析都失败。

修复：

- 检查 stderr 的解析失败原因。
- 调整 `QualityScoreSkill` system prompt。
- 保持失败不阻塞主流程，避免评分服务影响修复。

### 15.17 changelog 写入失败

症状：`DocumentationSkill` 返回文件写入错误。

检查：

- `--changelog-path` 是否存在权限问题。
- 相对路径是否应基于 `repo_root`。
- 默认 `docs/CHANGELOG.md` 是否存在。

需要跳过：

```bash
--disable-changelog
```

### 15.18 PR 评论失败

症状：

```text
DryRunFeedbackSkill: PR 评论发布失败
```

检查：

```bash
gh auth status
gh pr view <pr-number>
```

如果本地模式不想评论，使用 `auto-fix-local` 或 PR 模式加：

```bash
--no-pr-comments
```

### 15.19 git push 失败

先看是否是网络错误。如果是网络错误，代码会重试三次，再尝试 API fallback。

手动检查：

```bash
git -C <repo_root> status
git -C <repo_root> branch --show-current
git -C <repo_root> remote -v
```

强制使用 API fallback：

```bash
export CODEX_PUBLISH_VIA_GH_API=true
```

如果是分支保护或权限不足，必须人工处理。

### 15.20 stdout 不是合法 JSON

症状：workflow 中 `jq` 失败。

原因：某处把日志写到了 stdout。

修复：

- 自动化路径里所有进度日志必须用 `eprintln!`。
- `println!` 只能输出最终 JSON。
- 检查新增代码是否在 `pr-auto-fix` 或 `auto-fix-local` 路径中打印了非 JSON。

## 16. 修改代码时应该改哪里

### 16.1 新增输出 JSON 字段

修改：

- `src/types.rs::PrAutoFixOutput`
- `src/runtime.rs::run_auto_fix_loop` 构造 output 的位置
- 对应测试，通常在 `tests/e2e_auto_fix.rs` 或 runtime 单测
- workflow 文档，如果 Actions 会读取新字段

注意：新增字段通常兼容；删除或改名不兼容。

### 16.2 修改 severity 策略

修改：

- `src/types.rs::review_severity_token`
- `src/types.rs::is_review_severity_medium_or_higher`
- `src/types.rs::review_severity_matches_allowed`
- `src/skills.rs::decide_fix_or_skip`
- 单测：`types.rs` 和 `skills.rs` 内相关测试

同步文档：

- `scripts/codex-cli/docs/configuration.md`
- 本文档环境变量和 DecisionSkill 章节

### 16.3 修改 patch 格式

修改：

- `src/prompts.rs`
- `src/patch/mod.rs`
- `src/patch/search_replace.rs`
- `src/patch/unified_diff.rs`
- `src/skills.rs::generate_fix_patch`
- `src/skills.rs::generate_retry_fix_patch`

必须跑：

```bash
cargo test --test patch_search_replace
cargo test --test e2e_auto_fix
cargo test
```

### 16.4 新增一个 Skill

步骤：

1. 在 `src/skills.rs` 新增 struct。
2. 实现 `Skill` trait。
3. 明确读写 `SkillContext` 哪些字段。
4. 在 `src/runtime.rs` 的合适 pipeline 中 `.with_skill(Box::new(NewSkill))`。
5. 增加单测，至少覆盖成功路径和失败路径。
6. 如果会产生外部副作用，确保日志走 stderr。

不要把新业务逻辑写进 `pipeline.rs`。

### 16.5 新增 CLI 参数

修改：

- `src/bin/codex.rs` 的对应 enum variant。
- match 分支里把参数传入 `AutoFixOptions` 或新的 runtime 参数。
- 如果影响行为，更新 `docs/references/cli.md` 和本文档。
- 如果影响自动化输出，更新 workflow 测试或 e2e。

### 16.6 修改提交/推送行为

修改：

- `src/repo.rs::commit_and_push_in`
- 相关 API fallback 函数
- `repo.rs` 内 git/gh fake script 单测

必须非常谨慎，因为这里有外部可见副作用。

## 17. 测试地图

推荐基础验证：

```bash
cd scripts/codex-cli
cargo fmt --check
cargo test
```

重点测试文件：

| 文件 | 覆盖内容 |
| --- | --- |
| `tests/e2e_auto_fix.rs` | 本地自动修复端到端、fake agent、JSON 输出、安全阻塞 |
| `tests/patch_search_replace.rs` | SEARCH/REPLACE 解析、匹配、模糊匹配、重叠拒绝 |
| `tests/review_to_json.rs` | Markdown Review 到 JSON |
| `tests/doctor.rs` | doctor 输出 |
| `tests/auto_fix_report.rs` | 周报聚合 |
| `tests/workflow_state.rs` | workflow 状态相关规则 |
| `tests/gemini_watchdog.rs` | Gemini review watchdog 约束 |
| `tests/review_governance_docs.rs` | review 治理文档约束 |

源码内单测覆盖：

- `types.rs`：severity token 和 allowed severity。
- `llm.rs`：命令解析、prompt 拼接、timeout。
- `repo.rs`：changelog、ledger、git push retry、API fallback。
- `runtime.rs`：post-fix skip、pending policy。
- `skills.rs`：安全解析、质量评分、issue status。
- `unified_diff.rs`：preflight 和失败分类。

## 18. 本地最小调试流程

准备 fake review：

```markdown
- severity: Medium
- file: src/lib.rs
- line: 1
- rule: demo
- problem: value should return 2
- expected: change return value to 2
- constraints:
  - only modify src/lib.rs
```

转 JSON：

```bash
codex-auto-fix review-to-json --input review.md --output review.json
```

本地 dry run：

```bash
codex-auto-fix auto-fix-local \
  --repo-root "$PWD" \
  --review-json review.json \
  --disable-changelog
```

允许提交：

```bash
codex-auto-fix auto-fix-local \
  --repo-root "$PWD" \
  --review-json review.json \
  --disable-changelog \
  --yes
```

看结果：

```bash
git status
git diff
```

如果只是想验证 patch 子系统，不要调用模型，优先跑：

```bash
cargo test --test patch_search_replace
```

## 19. 不要打破的实现约束

- 自动化 stdout 只能是最终 JSON。
- 过程日志必须走 stderr。
- 默认 patch 主路径是 SEARCH/REPLACE。
- 每条 issue 默认只允许修改一个 allowed file。
- protected 文件不能进入完整文件兜底。
- Review 里 Medium/Medium+/High/Critical 问题必须有一对一状态。
- 模型 JSON 解析失败时要给出原始输出或可定位错误。
- `CODEX_AGENT_COMMAND` 不能递归指向当前二进制。
- GitHub push 网络失败可重试，可 API fallback，但权限和分支保护不能绕过。
- 文档、ledger、changelog 是可追溯性的一部分，不能无声失败。

## 20. 读源码顺序

第一次排障建议按这个顺序读：

1. `src/bin/codex.rs`：确认命令参数如何进 runtime。
2. `src/runtime.rs`：确认 pipeline 顺序和最终 JSON。
3. `src/skills.rs`：确认失败发生在哪个 Skill。
4. `src/patch/mod.rs` 和 `src/patch/search_replace.rs`：确认 patch 为什么不能应用。
5. `src/repo.rs`：确认文件、git、gh、changelog 副作用。
6. `src/llm.rs`：确认模型执行器命令和超时。
7. `src/types.rs`：确认输出字段和 severity 归一化。
8. `tests/e2e_auto_fix.rs`：找最接近的行为测试，复制后改成回归测试。
