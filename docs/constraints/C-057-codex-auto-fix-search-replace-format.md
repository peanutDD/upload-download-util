# C-057: codex-auto-fix SEARCH/REPLACE 主补丁格式

日期：2026-05-09
状态：生效中（permanent）
关联实现：`scripts/codex-cli/src/patch/`、`scripts/codex-cli/src/skills.rs::BatchFixSkill`
关联 ADR：`scripts/codex-cli/docs/exec-plans/auto-fix-patch-reliability.md`（AFPR-001）
姊妹文档：`scripts/codex-cli/docs/constraints/codex-cli-patch-format.md`（落地细节）

## 约束（不可删除）

`codex-auto-fix` 在自动修复链路中：

1. 默认 relaxed workflow（`CODEX_AUTO_FIX_DIRECT_FULL_FILE=true`）**必须**直接请求完整目标文件内容并写入工作区，避免上下文漂移导致 `context_mismatch` 阻塞自动修复；SEARCH/REPLACE Block 是本地/严格/未开启直写模式下的 LLM 主输出格式（`generate_fix_patch` 默认走 `RequestedPatchFormat::Auto`，prompt 由 `search_replace_system_prompt` 构造）。
2. **不允许**回退到「让 LLM 重新生成 unified diff」作为首选重试路径——unified diff 仅作向后兼容输入：当模型自发输出 diff 且 `detect_format` 识别为 `UnifiedDiff` 时按现有 `apply_patch_with_details_in` 路径处理。
3. **禁止**绕过 SR 解析器对 `### File:` 头与 `<<<<<<< SEARCH / ======= / >>>>>>> REPLACE` 三标志的强校验。
4. **禁止**在 SR 应用器中跳过三级匹配（exact → trim_trailing_whitespace → normalize_indent）的歧义检测；多匹配必须返回错误。
5. 未开启完整文件直写时，兜底顺序固定为：**SR 块兜底 → 完整文件覆写**；不允许把"完整 diff 文件"插入这两步之间。开启完整文件直写时，不得在直写前调用 patch apply。
6. 当初始补丁为空、`context_mismatch`、`drift`、`unknown`、重试补丁为空或重试生成失败时，**必须**继续尝试完整文件兜底；这些是可恢复的模型/上下文失败，不得直接中断整轮自动修复。
7. 当 `selected_issues.len() > 0 && fixed_files.is_empty()` 时，**必须**短路 `SecurityCheck / QualityScore / Documentation`，避免对未修复的原始代码评分误导人类审阅者。
8. 默认 workflow 使用 relaxed 模式（`CODEX_AUTO_FIX_STRICT=false`）：SecurityCheck/QualityScore 等软审计只作为提示，不得把已修复的 Gemini Review 问题重新卡成 needs-human；受保护文件、repo 写越界、checkout 精确性和提交前验证仍是硬边界。
9. `git push` / `gh pr comment` 等远程调用必须复用 `checked_output_with_retry`，且 `classify_git_network_error` 至少识别 `Empty reply from server / connection reset / timeout / failed to connect`。

## 为什么

- LLM 直接产 unified diff 时，hunk header 行数计数错误率极高，触发 `git apply: corrupt patch at line N`，无法靠 prompt 规劝消除。
- SR Block 把"修改"表达为「定位锚 + 替换」，去掉了行数计数，错误从语法级降到语义级，可由匹配器精确反馈给模型。
- 失败短路保证 SecurityCheck/QualityScore 永远基于"被实际修改过的代码"，避免出现「补丁全失败但报 42 分」这种误导性记录。
- 自动修复的主目标是清理 Gemini Review 指出的代码问题；软审计和上下文漂移不能喧宾夺主，必须尽可能把控制权交回 Codex/GPT-5.5 继续修。

## 历史教训（不可重复）

| 日期 | 事件 | 教训 |
|------|------|------|
| 2026-05-09 | PR #26 自动修复 6 次连续 `corrupt patch at line` | LLM 重试 unified diff 不会自愈；同一文件多次失败成本极高 |
| 历史多次 | 整文件兜底里仍走 diff 路径 | 兜底必须先 SR 再裸覆写，不能再让 diff 复活 |
| 历史多次 | 全部 patch 失败但 QualityScore=42 写进了 `docs/quality-score.md` | 失败必须短路评分，否则历史分数被污染 |
| 2026-05-14 | `context_mismatch` 后重试补丁为空导致 review 自动修复进入 pending | context mismatch 是可恢复失败；必须继续完整文件兜底，而不是把问题交还给人工 |
| 2026-05-15 | relaxed workflow 仍先走 patch apply，真实 `backend/src/api/webdav.rs` 因 `context_mismatch` 进入重试/兜底日志 | relaxed 自动修复应直接完整文件写入；patch 上下文匹配不应成为 GPT-5.5 修复链路的第一道障碍 |

## 失效条件

仅当出现以下**全部**条件时，方可由人类批准修订本约束：

1. 业内主流 LLM（GPT-5+、Claude Opus 5+、Gemini 3+）在 100 个真实 PR 黄金集上 unified diff 一次性应用成功率 ≥ 99%
2. SR Block 在歧义匹配场景比 diff 更糟（实证数据，非主观判断）
3. 上游 `git apply` 引入 `--llm-mode` 等容错开关并稳定释出

否则视为永久。

## 验证

- `cargo test -p codex-cli --test patch_search_replace`（SR 单测）
- `cargo test -p codex-cli` 全量
- `scripts/codex-cli/docs/constraints/codex-cli-patch-format.md` 中列出的全部 Required Behavior
