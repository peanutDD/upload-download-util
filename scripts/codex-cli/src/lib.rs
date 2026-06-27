//! `codex-cli` 的可复用库代码。
//!
//! 二进制入口位于 `src/bin/codex.rs`，这里只保留可测试、可复用的模块：
//! - LLM 调用
//! - Repo/IO（git/gh/changelog/comment）
//! - Skill 实现与 Pipeline 编排

pub mod auto_fix_report;
pub mod doctor;
pub mod llm;
pub mod patch;
pub mod pipeline;
pub mod prompts;
pub mod repo;
pub mod review_json;
pub mod review_ledger;
pub mod runtime;
pub mod skills;
pub mod types;
