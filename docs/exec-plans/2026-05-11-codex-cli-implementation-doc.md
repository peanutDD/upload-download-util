# codex-cli Implementation Documentation Exec Plan

日期：2026-05-11
状态：完成

## 目标

新增一份面向维护者和排障者的 `scripts/codex-cli` 完整实现技术文档，覆盖当前 Rust CLI 的入口、模块分层、数据契约、自动修复流水线、补丁应用、环境变量、故障修复路径和验证方式。

## 假设

- 本任务只改文档和导航，不改 `codex-cli` 运行逻辑。
- 文档以当前源码为准；如现有旧文档描述与源码不一致，新文档明确现状。
- 目标读者不熟悉实现细节，需要能按文档定位问题、修复问题、运行验证。

## 风险

- 文档过长容易与源码漂移；通过引用明确文件路径、函数名和输出字段降低漂移成本。
- 现有文档存在旧路径说明，新文档需要避免扩大重复维护面。
- 文档变更无法通过 TDD 证明行为正确；验证改用 `cargo fmt --check`、`cargo test` 和文档路径检查。

## 依赖

- `scripts/codex-cli/Cargo.toml`
- `scripts/codex-cli/src/bin/codex.rs`
- `scripts/codex-cli/src/lib.rs`
- `scripts/codex-cli/src/types.rs`
- `scripts/codex-cli/src/llm.rs`
- `scripts/codex-cli/src/repo.rs`
- `scripts/codex-cli/src/runtime.rs`
- `scripts/codex-cli/src/pipeline.rs`
- `scripts/codex-cli/src/skills.rs`
- `scripts/codex-cli/src/patch/*`
- `scripts/codex-cli/src/review_json.rs`
- `scripts/codex-cli/src/doctor.rs`
- `scripts/codex-cli/src/auto_fix_report.rs`
- `scripts/codex-cli/docs/*`

## 步骤

1. 完成：读取仓库与 `scripts/codex-cli` 局部规则。
2. 完成：读取当前源码入口和核心模块，确认实际实现。
3. 完成：新增 `scripts/codex-cli/docs/implementation.md`。
4. 完成：在 `scripts/codex-cli/docs/README.md` 增加导航。
5. 完成：运行格式/测试验证。
6. 完成：更新 `docs/quality-score.md`。

## 验证

- `awk 'BEGIN{n=0} /^```/{n++} END{print n}' scripts/codex-cli/docs/implementation.md` 输出 `150`，三反引号围栏成对。
- `cargo fmt --check` 通过。
- `cargo test` 通过。
