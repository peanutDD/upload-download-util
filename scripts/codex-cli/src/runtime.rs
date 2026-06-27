use crate::llm::CodexClient;
use crate::pipeline::Pipeline;
use crate::repo;
use crate::review_ledger;
use crate::skills::{
    BatchFixSkill, DecisionSkill, DocumentationSkill, DryRunFeedbackSkill, FeedbackSkill,
    QualityScoreSkill, ReadReviewSkill, SecurityCheckSkill, SkillContext, SkillContextInit,
    fixed_explanations, review_issue_key,
};
use crate::types::{PrAutoFixOutput, ReviewData, ReviewIssue, is_review_severity_medium_or_higher};
use std::env;

#[derive(Debug, Clone)]
pub struct AutoFixOptions {
    pub repo_root: String,
    pub rules_file: Option<String>,
    pub changelog_path: Option<String>,
    pub disable_changelog: bool,
    pub enable_pr_comments: bool,
}

/// GitHub Actions 的主入口：对指定 PR 的 Gemini Review 进行解析并尝试自动修复。
///
/// 输出约定：
/// - 返回值为 JSON 字符串（供 workflow 用 `jq` 解析）
/// - 运行过程中的日志应由各 Skill 输出到 stderr，避免污染 JSON
pub async fn pr_auto_fix(
    pr_number: u32,
    gemini_review: &str,
    max_rounds: u8,
    yes: bool,
    client: &CodexClient,
) -> Result<String, Box<dyn std::error::Error>> {
    let repo_root = env::var("GITHUB_WORKSPACE")
        .ok()
        .or_else(|| repo::git_repo_root().ok())
        .unwrap_or_else(|| ".".to_string());
    let options = AutoFixOptions {
        repo_root,
        rules_file: None,
        changelog_path: None,
        disable_changelog: false,
        enable_pr_comments: true,
    };
    pr_auto_fix_with_options(pr_number, gemini_review, max_rounds, yes, options, client).await
}

pub async fn pr_auto_fix_with_options(
    pr_number: u32,
    gemini_review: &str,
    max_rounds: u8,
    yes: bool,
    options: AutoFixOptions,
    client: &CodexClient,
) -> Result<String, Box<dyn std::error::Error>> {
    pr_auto_fix_with_options_and_review_data(
        pr_number,
        gemini_review,
        max_rounds,
        yes,
        options,
        None,
        client,
    )
    .await
}

pub async fn pr_auto_fix_with_options_and_review_data(
    pr_number: u32,
    gemini_review: &str,
    max_rounds: u8,
    yes: bool,
    options: AutoFixOptions,
    initial_review_data: Option<ReviewData>,
    client: &CodexClient,
) -> Result<String, Box<dyn std::error::Error>> {
    let repo_name = env::var("GITHUB_REPOSITORY").unwrap_or_default();
    let rules_text = repo::read_rules(Some(&options.repo_root), options.rules_file.as_deref());
    let mut ctx = SkillContext::new(SkillContextInit {
        pr_number,
        repo: repo_name,
        repo_root: options.repo_root,
        rules_text,
        raw_input: gemini_review.to_string(),
        rounds: max_rounds,
        auto_push: yes,
        enable_pr_comments: options.enable_pr_comments,
        changelog_path: options.changelog_path,
        disable_changelog: options.disable_changelog,
    });
    ctx.parsed_data = initial_review_data;
    run_auto_fix_loop(ctx, client).await
}

pub async fn auto_fix_local(
    review_text: &str,
    max_rounds: u8,
    yes: bool,
    options: AutoFixOptions,
    client: &CodexClient,
) -> Result<String, Box<dyn std::error::Error>> {
    auto_fix_local_with_review_data(review_text, max_rounds, yes, options, None, client).await
}

pub async fn auto_fix_local_with_review_data(
    review_text: &str,
    max_rounds: u8,
    yes: bool,
    options: AutoFixOptions,
    initial_review_data: Option<ReviewData>,
    client: &CodexClient,
) -> Result<String, Box<dyn std::error::Error>> {
    let rules_text = repo::read_rules(Some(&options.repo_root), options.rules_file.as_deref());
    let mut ctx = SkillContext::new(SkillContextInit {
        pr_number: 0,
        repo: String::new(),
        repo_root: options.repo_root,
        rules_text,
        raw_input: review_text.to_string(),
        rounds: max_rounds,
        auto_push: yes,
        enable_pr_comments: options.enable_pr_comments,
        changelog_path: options.changelog_path,
        disable_changelog: options.disable_changelog,
    });
    ctx.parsed_data = initial_review_data;
    run_auto_fix_loop(ctx, client).await
}

async fn run_auto_fix_loop(
    mut ctx: SkillContext,
    client: &CodexClient,
) -> Result<String, Box<dyn std::error::Error>> {
    let fix_pipeline = Pipeline::new()
        .with_skill(Box::new(ReadReviewSkill))
        .with_skill(Box::new(DecisionSkill))
        .with_skill(Box::new(BatchFixSkill));
    let post_fix_pipeline = Pipeline::new()
        .with_skill(Box::new(SecurityCheckSkill))
        .with_skill(Box::new(QualityScoreSkill))
        .with_skill(Box::new(DocumentationSkill));

    ctx.current_round = 1;
    if ctx.max_rounds > 1 {
        eprintln!(
            "🔁 [AutoFix] 单次处理当前 Gemini Review；外层 PR 标签负责最多 {} 轮 Review",
            ctx.max_rounds
        );
    }
    fix_pipeline.run(&mut ctx, client).await?;
    if should_skip_post_fix_checks(&ctx) {
        mark_post_fix_checks_skipped(&mut ctx);
        eprintln!(
            "⏭️ [AutoFix] 跳过 SecurityCheck/QualityScore/Documentation: {}",
            ctx.quality_score_reason.as_deref().unwrap_or("无后续检查")
        );
    } else {
        post_fix_pipeline.run(&mut ctx, client).await?;
    }
    if remediate_security_findings_once(&mut ctx, client).await? {
        post_fix_pipeline.run(&mut ctx, client).await?;
    }

    enforce_review_policy(&mut ctx);
    record_security_findings_as_pending(&mut ctx);

    let summary = ctx.parsed_data.as_ref().map(|d| d.summary.clone());
    let source_fixed = review_ledger::source_fix_created(&ctx);
    let review_record_path = review_ledger::append_from_context(&mut ctx, summary.clone())?;

    let feedback_pipeline = Pipeline::new()
        .with_skill(Box::new(DryRunFeedbackSkill))
        .with_skill(Box::new(FeedbackSkill));
    feedback_pipeline.run(&mut ctx, client).await?;

    let mut files = ctx.fixed_files.clone();
    files.sort();
    files.dedup();
    let pending_count = ctx.pending_explanations.len();
    let has_pending = pending_count > 0;
    let review_clean =
        (ctx.security_passed || !auto_fix_strict()) && !ctx.push_blocked && !has_pending;
    let final_status = final_status(ctx.push_blocked, has_pending, review_clean);

    let output = PrAutoFixOutput {
        fixed: if ctx.auto_push {
            source_fixed && !ctx.push_blocked
        } else {
            source_fixed
        },
        files,
        quality_score: ctx.quality_score,
        quality_score_available: ctx.quality_score_available,
        security_passed: ctx.security_passed,
        push_blocked: ctx.push_blocked,
        has_pending,
        pending_count,
        review_clean,
        apply_fail_reason: apply_fail_reason(&ctx.fix_attempts),
        retry_count: retry_count(&ctx.fix_attempts),
        fallback_used: fallback_used(&ctx.fix_attempts),
        final_status,
        summary,
        review_record_path,
        fixed_explanations: fixed_explanations(&ctx),
        issue_statuses: review_ledger::issue_statuses(&ctx),
        pending_explanations: ctx.pending_explanations,
    };

    Ok(serde_json::to_string(&output)?)
}

async fn remediate_security_findings_once(
    ctx: &mut SkillContext,
    client: &CodexClient,
) -> Result<bool, Box<dyn std::error::Error>> {
    if ctx.security_passed || ctx.security_findings.is_empty() {
        return Ok(false);
    }

    let issues = security_findings_as_review_issues(&ctx.security_findings);
    if issues.is_empty() {
        return Ok(false);
    }

    eprintln!(
        "🛡️ [AutoFix] SecurityCheck 发现可修复问题，进入安全修复补丁轮: {} 条",
        issues.len()
    );

    ctx.selected_issues = issues;
    let fixed_before = ctx.fixed_files.len();
    let security_fix_pipeline = Pipeline::new().with_skill(Box::new(BatchFixSkill));
    security_fix_pipeline.run(ctx, client).await?;

    Ok(ctx.fixed_files.len() > fixed_before)
}

fn security_findings_as_review_issues(findings: &[String]) -> Vec<ReviewIssue> {
    findings
        .iter()
        .filter_map(|finding| {
            let (file, reason) = finding.split_once(": ")?;
            let file = file.trim();
            if file.is_empty() || !file.contains('/') {
                return None;
            }

            Some(ReviewIssue {
                file: file.to_string(),
                line: None,
                severity: "High".to_string(),
                description: format!("SecurityCheck finding: {}", reason.trim()),
                suggestion: "修复 SecurityCheck 发现的安全风险，并保持改动限于当前文件。"
                    .to_string(),
                constraints: vec![format!("Only modify `{file}`")],
                reason: Some(finding.clone()),
            })
        })
        .collect()
}

fn record_security_findings_as_pending(ctx: &mut SkillContext) {
    if !auto_fix_strict() {
        return;
    }

    if ctx.security_passed || ctx.security_findings.is_empty() {
        return;
    }

    for finding in &ctx.security_findings {
        let explanation = format!("[Security] SecurityCheck 未自动修复：{}", finding);
        if !ctx.pending_explanations.contains(&explanation) {
            ctx.pending_explanations.push(explanation);
        }
    }
}

fn apply_fail_reason(attempts: &[crate::skills::FixAttempt]) -> Option<String> {
    attempts
        .iter()
        .filter(|attempt| {
            !attempt.success
                && matches!(attempt.stage.as_str(), "patch_apply" | "patch_apply_retry")
        })
        .filter_map(|attempt| attempt.reason.as_deref())
        .find_map(|reason| {
            ["malformed_diff", "context_mismatch", "drift", "unknown"]
                .iter()
                .find(|classification| reason.contains(**classification))
                .map(|classification| classification.to_string())
        })
}

fn retry_count(attempts: &[crate::skills::FixAttempt]) -> usize {
    attempts
        .iter()
        .filter(|attempt| attempt.stage == "patch_apply_retry")
        .count()
}

fn fallback_used(attempts: &[crate::skills::FixAttempt]) -> bool {
    attempts.iter().any(|attempt| {
        matches!(
            attempt.stage.as_str(),
            "file_replacement_fallback" | "file_replacement_direct"
        ) && attempt.success
    })
}

pub(crate) fn should_skip_post_fix_checks(ctx: &SkillContext) -> bool {
    ctx.fixed_files.is_empty()
}

pub(crate) fn mark_post_fix_checks_skipped(ctx: &mut SkillContext) {
    if ctx.selected_issues.is_empty() {
        ctx.security_passed = true;
        ctx.quality_score_reason = Some("跳过：本轮未选择自动修复问题".to_string());
    } else {
        ctx.security_passed = false;
        ctx.quality_score_reason = Some("跳过：本轮未产生可验证的修复文件".to_string());
    }
    ctx.quality_score = 0;
    ctx.quality_score_available = false;
    ctx.security_findings.clear();
}

fn final_status(push_blocked: bool, has_pending: bool, review_clean: bool) -> String {
    if push_blocked {
        "needs-human"
    } else if has_pending {
        "pending"
    } else if review_clean {
        "clean"
    } else {
        "needs-human"
    }
    .to_string()
}

fn auto_fix_strict() -> bool {
    env::var("CODEX_AUTO_FIX_STRICT")
        .map(|value| {
            let value = value.to_ascii_lowercase();
            matches!(value.as_str(), "1" | "true" | "yes" | "strict")
        })
        .unwrap_or(false)
}

pub(crate) fn enforce_review_policy(ctx: &mut SkillContext) {
    ctx.pending_explanations.clear();
    let fixed: std::collections::HashSet<String> = ctx
        .fix_attempts
        .iter()
        .filter(|a| a.success)
        .map(|a| a.issue_key.clone())
        .chain(ctx.fixed_issue_keys.iter().cloned())
        .collect();
    let selected: std::collections::HashSet<String> =
        ctx.selected_issues.iter().map(review_issue_key).collect();

    let Some(data) = &ctx.parsed_data else {
        return;
    };

    for issue in data
        .issues
        .iter()
        .filter(|i| is_review_severity_medium_or_higher(&i.severity))
    {
        let issue_key = review_issue_key(issue);
        if fixed.contains(&issue_key) {
            continue;
        }

        let latest_attempt = ctx
            .fix_attempts
            .iter()
            .rev()
            .find(|a| a.issue_key == issue_key && !a.success)
            .and_then(|a| a.reason.as_deref());
        let reason = latest_attempt.unwrap_or_else(|| {
            if selected.contains(&issue_key) {
                "未产生可应用补丁"
            } else {
                "策略过滤或受保护路径，未自动修复"
            }
        });
        ctx.pending_explanations.push(format!(
            "[{}] `{}`:{} 未自动修复：{}",
            issue.severity,
            issue.file,
            issue.line.unwrap_or(0),
            reason
        ));
    }

    if !ctx.pending_explanations.is_empty() && !ctx.enable_pr_comments {
        eprintln!(
            "⚠️ 存在未发布到 PR 的未修复说明: {} 条",
            ctx.pending_explanations.len()
        );
    }
}

pub async fn run_skill_pack_skill(
    plugin_root: &str,
    skill: &str,
    input: &str,
    client: &CodexClient,
) -> Result<String, Box<dyn std::error::Error>> {
    let agents_rules = repo::read_skill_pack_agents_rules(plugin_root);
    let resolved = repo::resolve_skill_pack_skill(plugin_root, skill)?;

    let skill_body = resolved
        .body
        .replace("${CLAUDE_PLUGIN_ROOT}", plugin_root)
        .trim()
        .to_string();
    let skill_name = resolved
        .meta
        .name
        .as_deref()
        .unwrap_or(resolved.meta.id.as_str());

    let system_prompt = format!(
        "你是一个可执行 Skill Pack 的资深软件工程师。\n\n\
必须严格遵守以下 AGENTS.md 规则：\n\n{}\n\n\
现在执行 Skill：{}\n\
Skill Id：{}\n\n\
Skill 指令如下（来自 SKILL.md）：\n\n{}\n",
        agents_rules, skill_name, resolved.meta.id, skill_body
    );
    let user_prompt = input;
    client.call(&system_prompt, user_prompt).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::{FixAttempt, SkillContextInit};
    use crate::types::{ReviewData, ReviewIssue};

    fn test_ctx() -> SkillContext {
        SkillContext::new(SkillContextInit {
            pr_number: 1,
            repo: "owner/repo".to_string(),
            repo_root: ".".to_string(),
            rules_text: "rules".to_string(),
            raw_input: "review".to_string(),
            rounds: 2,
            auto_push: false,
            enable_pr_comments: false,
            changelog_path: None,
            disable_changelog: true,
        })
    }

    #[test]
    fn post_fix_checks_skip_when_no_fix_was_created_for_selected_issues() {
        let mut ctx = test_ctx();
        ctx.selected_issues.push(ReviewIssue {
            file: "src/a.rs".to_string(),
            line: Some(10),
            severity: "Medium".to_string(),
            description: "bug".to_string(),
            suggestion: "fix".to_string(),
            constraints: Vec::new(),
            reason: None,
        });

        assert!(should_skip_post_fix_checks(&ctx));
        mark_post_fix_checks_skipped(&mut ctx);

        assert!(!ctx.security_passed);
        assert!(!ctx.quality_score_available);
        assert_eq!(
            ctx.quality_score_reason.as_deref(),
            Some("跳过：本轮未产生可验证的修复文件")
        );
    }

    #[test]
    fn post_fix_checks_run_when_at_least_one_fix_exists() {
        let mut ctx = test_ctx();
        ctx.selected_issues.push(ReviewIssue {
            file: "src/a.rs".to_string(),
            line: Some(10),
            severity: "Medium".to_string(),
            description: "bug".to_string(),
            suggestion: "fix".to_string(),
            constraints: Vec::new(),
            reason: None,
        });
        ctx.fixed_files.push("src/a.rs".to_string());

        assert!(!should_skip_post_fix_checks(&ctx));
    }

    #[test]
    fn post_fix_checks_skip_cleanly_when_no_issue_was_selected() {
        let mut ctx = test_ctx();

        assert!(should_skip_post_fix_checks(&ctx));
        mark_post_fix_checks_skipped(&mut ctx);

        assert!(ctx.security_passed);
        assert!(!ctx.quality_score_available);
        assert_eq!(
            ctx.quality_score_reason.as_deref(),
            Some("跳过：本轮未选择自动修复问题")
        );
    }

    #[test]
    fn enforce_review_policy_records_pending_explanations_when_comments_disabled() {
        let mut ctx = test_ctx();
        ctx.parsed_data = Some(ReviewData {
            summary: "summary".to_string(),
            issues: vec![ReviewIssue {
                file: "src/a.rs".to_string(),
                line: Some(10),
                severity: "Medium".to_string(),
                description: "bug".to_string(),
                suggestion: "fix".to_string(),
                constraints: Vec::new(),
                reason: Some("review text".to_string()),
            }],
        });
        ctx.fix_attempts.push(FixAttempt {
            round: 1,
            issue_key: "src/a.rs:10:Medium:bug".to_string(),
            file: "src/a.rs".to_string(),
            stage: "patch_generation".to_string(),
            success: false,
            reason: Some("empty patch".to_string()),
        });

        enforce_review_policy(&mut ctx);

        assert_eq!(ctx.pending_explanations.len(), 1);
        assert!(ctx.pending_explanations[0].contains("empty patch"));
    }

    #[test]
    fn enforce_review_policy_records_literal_medium_plus_as_pending() {
        let mut ctx = test_ctx();
        ctx.parsed_data = Some(ReviewData {
            summary: "summary".to_string(),
            issues: vec![ReviewIssue {
                file: "src/a.rs".to_string(),
                line: Some(10),
                severity: "Medium+".to_string(),
                description: "bug".to_string(),
                suggestion: "fix".to_string(),
                constraints: Vec::new(),
                reason: Some("review text".to_string()),
            }],
        });

        enforce_review_policy(&mut ctx);

        assert_eq!(ctx.pending_explanations.len(), 1);
        assert!(ctx.pending_explanations[0].contains("[Medium+]"));
    }

    #[test]
    fn enforce_review_policy_tracks_same_file_issues_independently() {
        let mut ctx = test_ctx();
        ctx.parsed_data = Some(ReviewData {
            summary: "summary".to_string(),
            issues: vec![
                ReviewIssue {
                    file: "src/a.rs".to_string(),
                    line: Some(10),
                    severity: "Medium".to_string(),
                    description: "first".to_string(),
                    suggestion: "fix".to_string(),
                    constraints: Vec::new(),
                    reason: None,
                },
                ReviewIssue {
                    file: "src/a.rs".to_string(),
                    line: Some(20),
                    severity: "Medium".to_string(),
                    description: "second".to_string(),
                    suggestion: "fix".to_string(),
                    constraints: Vec::new(),
                    reason: None,
                },
            ],
        });
        ctx.fix_attempts.push(FixAttempt {
            round: 1,
            issue_key: "src/a.rs:10:Medium:first".to_string(),
            file: "src/a.rs".to_string(),
            stage: "patch_apply".to_string(),
            success: true,
            reason: None,
        });
        ctx.fix_attempts.push(FixAttempt {
            round: 1,
            issue_key: "src/a.rs:20:Medium:second".to_string(),
            file: "src/a.rs".to_string(),
            stage: "patch_generation".to_string(),
            success: false,
            reason: Some("empty patch".to_string()),
        });

        enforce_review_policy(&mut ctx);

        assert_eq!(ctx.pending_explanations.len(), 1);
        assert!(ctx.pending_explanations[0].contains("`:20"));
        assert!(ctx.pending_explanations[0].contains("empty patch"));
    }
}
