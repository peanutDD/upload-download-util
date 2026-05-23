# AI 自动修复永久约束 (Codex-Fix-Constraints)

版本：v1.1 | 最后更新：2026-05-03

## 1. 决策边界 (Decision Boundaries)
1. **优先级限制**: 严禁修复 `Low` 级别问题。AI 必须将精力集中在影响架构稳定性或安全性的 `Medium`、字面量 `Medium+` 及以上问题。
2. **文件黑名单**:
    - 禁止修改锁文件: `Cargo.lock`, `package-lock.json`, `bun.lock`, `pnpm-lock.yaml`。
    - 禁止修改敏感配置: `.env`, `*.secret`, `config/*.yaml`。
    - 禁止修改非代码文件: `docs/*`, `*.md`, `LICENSE`, `.gitignore`。
3. **架构铁律同步**: 修复后的代码必须符合 `AGENTS.md` 中的 15 条黄金规则。

## 2. 操作安全 (Operational Safety)
1. **Git 隔离**: 所有自动修复必须在独立的临时补丁中尝试应用，失败则立即回滚，严禁强制推送 (`push -f`)。
2. **Commit 规范**: 自动修复提交必须触发 CI，禁止使用 `[skip ci]`。为避免重复 Gemini 请求，`gemini-review-kickoff` 只允许跳过 GitHub Actions `codex-auto-fix` 状态机生成的精确前缀 `🤖 codex auto-fix:`；本地/Desktop Codex 发布必须使用 `🤖 codex local auto-fix:` 或提供等价 PR 评论/status-table 流程，不能冒充 workflow-owned 提交。
3. **轮次上限**: 单个 PR 的自动修复轮次严禁超过 `MAX_ROUNDS` (默认为 2)，防止 LLM 陷入无效的自纠缠循环。
4. **未修复必须解释**: 任何 `Medium`、`Medium+`、`High`、`Critical` 问题如果没有自动修复，必须在 PR 评论或最终 JSON 的 `issue_statuses` / `pending_explanations` 中写明原因。
5. **补丁失败重试**: `git apply` 失败时不得立即放弃；必须把失败补丁和最新源码回传给模型，重试生成一次更小的 unified diff。重试仍失败才进入未修复说明。
6. **Pending 不可伪装为 Clean**: `pending_explanations` 非空时，workflow 不得发布“无需修复/建议合并”类评论；必须进入下一轮 Review 或加 `gemini-review-needs-human` 标签。

## 3. 情报处理 (Intelligence Handling)
1. **JSON 强制性**: `read_gemini_review` 必须在 `json_mode` 下运行，且 `temperature` 设为 0。
2. **溯源要求**: 每个生成的补丁必须在解析阶段保留 Gemini 的 `reason` 片段，确保修复动机可追溯。
3. **安全审计 fail-closed**: 安全审计输出必须按严格 JSON 解析；解析失败视为未通过，不允许用字符串包含判断替代结构化解析。
4. **评分可用性**: 质量评分必须区分“真实 0 分”和“评分不可用”；解析失败需重试，仍失败则显式标记不可用。
5. **修复尝试记录**: 每条 issue 的补丁生成、应用成功/失败和失败原因必须结构化记录，供最终评论和人工决策使用。
