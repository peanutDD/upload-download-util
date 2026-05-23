use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn pending_without_fix_blocks_instead_of_claiming_clean() {
    let output = plan(&[
        ("CURRENT_ROUND", "gemini-review-round-1"),
        ("FIXED", "false"),
        ("PUSH_BLOCKED", "false"),
        ("PENDING_COUNT", "2"),
        ("CODEX_AUTO_FIX_STRICT", "true"),
    ]);

    assert_eq!(output["action"], "needs_human");
    assert_eq!(output["request_review"], "false");
    assert_eq!(output["ready_to_merge"], "false");
    assert_eq!(output["human_block"], "true");
}

#[test]
fn relaxed_pending_without_fix_clears_review_state() {
    let output = plan(&[
        ("CURRENT_ROUND", "gemini-review-round-1"),
        ("FIXED", "false"),
        ("PUSH_BLOCKED", "false"),
        ("PENDING_COUNT", "2"),
        ("CODEX_AUTO_FIX_STRICT", "false"),
    ]);

    assert_eq!(output["action"], "relaxed_clear");
    assert_eq!(output["next_round"], "gemini-review-round-max");
    assert_eq!(output["request_review"], "false");
    assert_eq!(output["ready_to_merge"], "true");
    assert_eq!(output["human_block"], "false");
    assert_eq!(output["state_label"], "gemini-review-clean");
}

#[test]
fn relaxed_push_blocked_clears_review_state() {
    let output = plan(&[
        ("CURRENT_ROUND", "gemini-review-round-1"),
        ("FIXED", "false"),
        ("PUSH_BLOCKED", "true"),
        ("PENDING_COUNT", "3"),
        ("CODEX_AUTO_FIX_STRICT", "false"),
    ]);

    assert_eq!(output["action"], "relaxed_clear");
    assert_eq!(output["request_review"], "false");
    assert_eq!(output["ready_to_merge"], "true");
    assert_eq!(output["human_block"], "false");
    assert_eq!(output["state_label"], "gemini-review-clean");
}

#[test]
fn clean_first_round_requests_second_review() {
    let output = plan(&[
        ("CURRENT_ROUND", "gemini-review-round-1"),
        ("FIXED", "false"),
        ("PUSH_BLOCKED", "false"),
        ("PENDING_COUNT", "0"),
        ("CODEX_AUTO_FIX_STRICT", "true"),
    ]);

    assert_eq!(output["action"], "advance");
    assert_eq!(output["next_round"], "gemini-review-round-2");
    assert_eq!(output["request_review"], "true");
    assert_eq!(output["ready_to_merge"], "false");
}

#[test]
fn clean_second_round_marks_round_max_ready() {
    let output = plan(&[
        ("CURRENT_ROUND", "gemini-review-round-2"),
        ("FIXED", "false"),
        ("PUSH_BLOCKED", "false"),
        ("PENDING_COUNT", "0"),
        ("CODEX_AUTO_FIX_STRICT", "true"),
    ]);

    assert_eq!(output["action"], "complete");
    assert_eq!(output["next_round"], "gemini-review-round-max");
    assert_eq!(output["request_review"], "false");
    assert_eq!(output["ready_to_merge"], "true");
}

#[test]
fn pushed_partial_fix_continues_to_second_review_but_not_ready() {
    let output = plan(&[
        ("CURRENT_ROUND", "gemini-review-round-1"),
        ("FIXED", "true"),
        ("PUSH_BLOCKED", "false"),
        ("PENDING_COUNT", "1"),
        ("CODEX_AUTO_FIX_STRICT", "true"),
    ]);

    assert_eq!(output["action"], "advance_with_pending");
    assert_eq!(output["next_round"], "gemini-review-round-2");
    assert_eq!(output["request_review"], "true");
    assert_eq!(output["ready_to_merge"], "false");
}

#[test]
fn fail_closed_security_blocks_loop() {
    let output = plan(&[
        ("CURRENT_ROUND", "gemini-review-round-1"),
        ("FIXED", "false"),
        ("PUSH_BLOCKED", "true"),
        ("PENDING_COUNT", "0"),
        ("CODEX_AUTO_FIX_STRICT", "true"),
    ]);

    assert_eq!(output["action"], "push_blocked");
    assert_eq!(output["request_review"], "false");
    assert_eq!(output["human_block"], "true");
}

#[test]
fn codex_auto_fix_concurrency_serializes_without_canceling_active_runs() {
    let workflow = fs::read_to_string(codex_auto_fix_workflow())
        .expect("codex auto-fix workflow should be readable");
    let jobs_index = workflow
        .find("jobs:")
        .expect("codex auto-fix workflow should define jobs");
    let top_level = &workflow[..jobs_index];
    let jobs = &workflow[jobs_index..];

    assert!(
        top_level.contains("\nconcurrency:\n"),
        "workflow-level concurrency is required to serialize same-PR review runs before they contend for the self-hosted runner"
    );
    assert!(
        top_level.contains("codex-auto-fix-pr-")
            && top_level.contains("github.event.issue.number")
            && top_level.contains("github.event.pull_request.number"),
        "workflow-level concurrency must use the PR number, not github.ref/actor, so unrelated PRs do not stale-block each other"
    );
    assert!(
        !top_level.contains("github.ref }}-${{ github.actor"),
        "github.ref/actor concurrency can serialize unrelated issue_comment review runs on the default branch"
    );
    assert!(
        top_level.contains("  cancel-in-progress: false"),
        "a newer Gemini review should wait for the active codex-fix run so it can finish state advancement and publish"
    );
    assert!(
        !jobs.contains("    concurrency:\n"),
        "job-level concurrency starts too late on self-hosted runners and can still leave newer runs waiting"
    );
}

#[test]
fn codex_auto_fix_serial_runner_has_timeouts() {
    let workflow = fs::read_to_string(codex_auto_fix_workflow())
        .expect("codex auto-fix workflow should be readable");

    assert!(
        workflow.contains("    timeout-minutes: 55"),
        "codex-fix job should have a hard timeout so one stale run cannot block the PR queue forever"
    );
    assert!(
        workflow.contains("        timeout-minutes: 45\n        if: steps.round.outputs.current_round != 'gemini-review-round-max'"),
        "the pr-auto-fix step should time out before the job timeout and release the concurrency group"
    );
    assert!(
        workflow.contains("CODEX_AGENT_TIMEOUT_SECONDS: 360"),
        "the local Codex child process should have a bounded timeout below the step timeout"
    );
    assert!(
        workflow.contains("CODEX_AGENT_MODEL: gpt-5.5"),
        "auto-fix should force the intended GPT-5.5 model instead of inheriting an interactive Codex profile"
    );
    assert!(
        workflow.contains("name: Auto-fix queue guard diagnostics")
            && workflow.contains("codex_queue_group=codex-auto-fix-${PR_NUMBER}"),
        "codex-fix should print queue guard diagnostics so stale-run waits are explainable"
    );
}

#[test]
fn codex_auto_fix_runs_doctor_before_long_auto_fix_step() {
    let workflow = fs::read_to_string(codex_auto_fix_workflow())
        .expect("codex auto-fix workflow should be readable");

    let doctor_index = workflow
        .find("doctor --json")
        .expect("workflow should run codex-auto-fix doctor before pr-auto-fix");
    let auto_fix_index = workflow
        .find("pr-auto-fix \\")
        .expect("workflow should run pr-auto-fix");

    assert!(
        doctor_index < auto_fix_index,
        "doctor should fail fast before the expensive auto-fix command starts"
    );
    assert!(
        workflow.contains("select(.name == \"agent.command\" and .status == \"warning\")"),
        "workflow should stop when CODEX_AGENT_COMMAND is missing or recursively points to codex-auto-fix"
    );
}

#[test]
fn codex_auto_fix_passes_review_json_to_auto_fix() {
    let workflow = fs::read_to_string(codex_auto_fix_workflow())
        .expect("codex auto-fix workflow should be readable");

    let extract_index = workflow
        .find("name: 提取 Gemini Review 完整内容")
        .expect("workflow should extract the complete Gemini review first");
    let json_index = workflow
        .find("name: Convert review markdown to JSON primary input")
        .expect("workflow should convert extracted review markdown to primary JSON input");
    let auto_fix_index = workflow
        .find("name: 调用 codex-auto-fix pr-auto-fix")
        .expect("workflow should run codex-auto-fix after JSON validation");

    assert!(
        extract_index < json_index && json_index < auto_fix_index,
        "review extraction should feed JSON validation before auto-fix runs"
    );
    assert!(
        workflow.contains("printf '%s\\n' \"$REVIEW_TEXT\" > /tmp/review.md"),
        "workflow should persist the exact extracted review body for JSON conversion"
    );
    assert!(
        workflow.contains("review-to-json")
            && workflow.contains("--input /tmp/review.md")
            && workflow.contains("--output /tmp/review.json"),
        "workflow should call codex-auto-fix review-to-json on the extracted markdown"
    );
    assert!(
        workflow.contains("REVIEW_JSON_PATH=/tmp/review.json"),
        "workflow should expose the JSON path for downstream observability"
    );
    assert!(
        workflow.contains("--review-json \"$REVIEW_JSON_PATH\""),
        "workflow should drive pr-auto-fix from the validated JSON input"
    );
    assert!(
        workflow.contains("printf '%s\\n' \"$RESULT\" > /tmp/codex-result.json")
            && workflow.contains("CODEX_RESULT_PATH=/tmp/codex-result.json"),
        "workflow should persist pr-auto-fix JSON so the state machine can publish the exact Gemini issue/status table"
    );
}

#[test]
fn codex_auto_fix_has_coherent_runtime_budget() {
    let workflow = fs::read_to_string(codex_auto_fix_workflow())
        .expect("codex auto-fix workflow should be readable");

    assert!(
        workflow.contains("timeout-minutes: 55"),
        "codex-fix job should leave enough room for checkout, JSON conversion, auto-fix, and state advancement"
    );
    assert!(
        workflow.contains("timeout-minutes: 45"),
        "pr-auto-fix step should not be killed at the old 30 minute boundary"
    );
    assert!(
        workflow.contains("CODEX_AGENT_TIMEOUT_SECONDS: 360"),
        "each model call should be bounded so one slow audit cannot consume the whole step"
    );
    assert!(
        workflow.contains("CODEX_AUTO_FIX_BUDGET_SECONDS: 2400"),
        "the CLI should receive a budget lower than the step timeout so it can return JSON before Actions kills it"
    );
    assert!(
        workflow.contains("CODEX_AUTO_FIX_STRICT: false"),
        "review automation should run in relaxed mode so recoverable patch/audit failures do not interrupt GPT-5.5 repair"
    );
    assert!(
        workflow.contains("CODEX_AUTO_FIX_DIRECT_FULL_FILE: true"),
        "relaxed review automation should write complete files directly instead of blocking on patch context mismatch"
    );
    assert!(
        workflow.contains("CODEX_AGENT_MODEL: ${{ env.CODEX_AGENT_MODEL }}"),
        "codex-fix should pass the model override into codex-cli"
    );
    assert!(
        workflow.contains("steps.round.outputs.current_round == 'gemini-review-round-max' && env.CODEX_AUTO_FIX_STRICT == 'true'"),
        "relaxed review automation must not skip new Gemini review events just because an old round-max label is present"
    );
    assert!(
        workflow.contains("WAIT_FOR_GEMINI_REVIEW: false"),
        "relaxed review automation should clear state immediately instead of waiting for another Gemini review"
    );
    assert!(
        workflow.contains("codex_auto_fix_budget_seconds=${CODEX_AUTO_FIX_BUDGET_SECONDS}"),
        "queue diagnostics should print the effective CLI budget"
    );
}

#[test]
fn codex_auto_fix_runs_frontend_pre_push_validation() {
    let workflow = fs::read_to_string(codex_auto_fix_workflow())
        .expect("codex auto-fix workflow should be readable");
    let script = fs::read_to_string(repo_path("scripts/codex-cli/tools/pre-push-verify.sh"))
        .expect("shared pre-push verify script should be readable");

    assert!(
        workflow.contains("CODEX_AUTO_FIX_VERIFY_COMMANDS:"),
        "codex-fix must define pre-push verification so generated patches are checked before publish"
    );
    assert!(
        workflow.contains("bash scripts/codex-cli/tools/pre-push-verify.sh --changed")
            && script.contains("git status --short -- frontend/")
            && script.contains("set -euo pipefail")
            && script.contains("npm ci --ignore-scripts")
            && script.contains("npm run lint")
            && script.contains("npx --no-install tsc -b --noEmit"),
        "frontend auto-fix changes must run fail-fast install, lint, and typecheck before commit/push without compiling native install scripts"
    );
}

#[test]
fn codex_auto_fix_runs_backend_pre_push_format_validation() {
    let workflow = fs::read_to_string(codex_auto_fix_workflow())
        .expect("codex auto-fix workflow should be readable");
    let script = fs::read_to_string(repo_path("scripts/codex-cli/tools/pre-push-verify.sh"))
        .expect("shared pre-push verify script should be readable");

    assert!(
        workflow.contains("bash scripts/codex-cli/tools/pre-push-verify.sh --changed")
            && script.contains("git status --short -- backend/")
            && script.contains("cd backend")
            && script.contains("cargo fmt --all -- --check")
            && script.contains("cargo clippy --all-targets --all-features -- -D warnings"),
        "backend auto-fix changes must run cargo fmt and clippy before commit/push because GitHub API publish does not trigger CI"
    );
}

#[test]
fn codex_auto_fix_uses_shared_pre_push_verify_script() {
    let workflow = fs::read_to_string(codex_auto_fix_workflow())
        .expect("codex auto-fix workflow should be readable");

    assert!(
        workflow.contains("bash scripts/codex-cli/tools/pre-push-verify.sh --changed"),
        "codex-fix should delegate publish verification to the shared script used by local git hooks"
    );
}

#[test]
fn codex_shared_pre_push_verify_script_covers_backend_and_frontend() {
    let script = fs::read_to_string(repo_path("scripts/codex-cli/tools/pre-push-verify.sh"))
        .expect("shared pre-push verify script should be readable");

    assert!(
        script.contains("git status --short -- frontend/")
            && script.contains("npm ci --ignore-scripts")
            && script.contains("npm run lint")
            && script.contains("npx --no-install tsc -b --noEmit"),
        "shared verify script must preserve the frontend auto-fix validation contract"
    );
    assert!(
        script.contains("git status --short -- backend/")
            && script.contains("cargo fmt --all -- --check")
            && script.contains("cargo clippy --all-targets --all-features -- -D warnings"),
        "shared verify script must preserve the backend auto-fix validation contract"
    );
}

#[test]
fn codex_git_hook_template_and_installer_delegate_to_shared_verify_script() {
    let hook = fs::read_to_string(repo_path("scripts/git-hooks/pre-push"))
        .expect("versioned pre-push hook template should be readable");
    let installer = fs::read_to_string(repo_path("scripts/git-hooks/install.sh"))
        .expect("git hook installer should be readable");

    assert!(
        hook.contains("SKIP_CODEX_VERIFY")
            && hook.contains("scripts/codex-cli/tools/pre-push-verify.sh --changed"),
        "local pre-push hook must support an explicit emergency bypass and otherwise delegate to the shared verifier"
    );
    assert!(
        installer.contains(".git/hooks/pre-push")
            && installer.contains("scripts/git-hooks/pre-push")
            && installer.contains("chmod +x"),
        "installer must copy the versioned hook template into the local untracked git hooks directory"
    );
}

#[test]
fn codex_auto_fix_supports_markdown_rollback_switch() {
    let workflow = fs::read_to_string(codex_auto_fix_workflow())
        .expect("codex auto-fix workflow should be readable");

    assert!(
        workflow.contains("USE_REVIEW_JSON: true"),
        "workflow should default to JSON review input"
    );
    assert!(
        workflow.contains("env.USE_REVIEW_JSON == 'true'"),
        "JSON conversion should be skipped when USE_REVIEW_JSON=false"
    );
    assert!(
        workflow.contains("if [[ \"${USE_REVIEW_JSON}\" == \"true\" ]]"),
        "auto-fix step should branch on the rollback switch"
    );
    assert!(
        workflow.contains("--review-json \"$REVIEW_JSON_PATH\""),
        "JSON branch should pass the validated review JSON"
    );
    assert!(
        workflow.contains("--gemini-review \"$REVIEW_BODY\""),
        "Markdown rollback branch should preserve the previous input path"
    );
    assert!(
        workflow.contains("gemini-review-needs-human"),
        "JSON conversion failures should route to the human fallback label/comment"
    );
}

#[test]
fn codex_auto_fix_bootstraps_pr_head_without_git_https_checkout() {
    let workflow = fs::read_to_string(codex_auto_fix_workflow())
        .expect("codex auto-fix workflow should be readable");

    assert!(
        !workflow.contains("uses: actions/checkout@v4"),
        "self-hosted auto-fix should not depend on actions/checkout Git HTTPS fetch as the first step"
    );
    assert!(
        workflow.contains("CODEX_LOCAL_REPO_SEED"),
        "workflow should seed the workspace from a local repository before contacting GitHub"
    );
    assert!(
        workflow.contains("CODEX_LOCAL_BARE_MIRROR"),
        "workflow should prefer a maintained local bare mirror before falling back to network tarballs"
    );
    assert!(
        workflow.contains("sync_local_bare_mirror"),
        "workflow should maintain the local bare mirror instead of assuming another process refreshed it"
    );
    assert!(
        workflow.contains("http.lowSpeedTime=30"),
        "local mirror refresh should bound stalled Git transfers before falling back"
    );
    assert!(
        workflow.contains("hydrate_from_local_sources"),
        "workflow should try to hydrate missing PR head commits from local sources before GitHub tarballs"
    );
    let clean_index = workflow
        .find("prepare_reused_workspace")
        .expect("workflow should define reused workspace cleanup before checkout");
    let hydrate_index = workflow
        .find("hydrate_from_local_sources")
        .expect("workflow should hydrate from local sources");
    assert!(
        clean_index < hydrate_index
            && workflow.contains("git reset --hard")
            && workflow.contains("git clean -ffdx"),
        "self-hosted runner workspaces must be cleaned before checkout so stale local edits cannot block PR head hydration"
    );
    assert!(
        workflow.contains("workspace path matches CODEX_LOCAL_REPO_SEED"),
        "workflow should refuse to clean the local seed repository if workspace/seed are accidentally the same path"
    );
    assert!(
        workflow.contains("headRefOid") && workflow.contains("headRefName"),
        "workflow should resolve the exact PR head branch and SHA through the GitHub API"
    );
    assert!(
        workflow.contains("tarball/${HEAD_SHA}"),
        "workflow should use the GitHub API tarball path instead of Git smart HTTP fetch when needed"
    );
    assert!(
        workflow.contains("download_pr_head_archive")
            && workflow.contains("CODEX_PR_TARBALL_BUDGET_SECONDS")
            && workflow.contains("tarball_deadline"),
        "workflow should retry transient PR tarball stream failures inside a hard total budget before failing closed"
    );
    assert!(
        workflow.contains("curl")
            && workflow.contains("--http1.1")
            && workflow.contains("--retry-all-errors"),
        "workflow should fall back to HTTP/1.1 curl retries when gh api streaming is cancelled"
    );
    assert!(
        workflow.contains("--max-time 45"),
        "each tarball network attempt should be bounded so bootstrap cannot burn the review budget"
    );
    assert!(
        workflow.contains("bootstrap_status=blocked"),
        "workflow should expose checkout/bootstrap blocked status instead of making failures look like review failures"
    );
    assert!(
        workflow.contains("Codex checkout/bootstrap blocked"),
        "workflow should post a clear PR comment when exact PR head bootstrap is blocked"
    );
    assert!(
        workflow.contains("Cannot verify exact PR head"),
        "workflow should fail closed instead of auto-fixing a stale local seed"
    );
    assert!(
        workflow.contains("CODEX_PUBLISH_VIA_GH_API=true"),
        "synthetic local checkout should force codex-cli to publish via GitHub API rather than git push"
    );
}

#[test]
fn gemini_kickoff_only_skips_actual_auto_fix_commit_subjects() {
    let workflow = fs::read_to_string(gemini_kickoff_workflow())
        .expect("gemini kickoff workflow should be readable");

    assert!(
        !workflow.contains("*codex auto-fix*"),
        "Gemini kickoff must not skip any human commit that merely mentions codex auto-fix"
    );
    assert!(
        workflow.contains("\"$LAST_COMMIT_MESSAGE\" == \"🤖 codex auto-fix:\"*"),
        "Gemini kickoff should only skip the exact auto-fix bot commit subject prefix"
    );
    assert!(
        workflow.contains("GEMINI_REVIEW_REQUIRED: false"),
        "Gemini kickoff should request review without making missing Gemini responses a blocking check in relaxed mode"
    );
}

#[test]
fn state_script_names_medium_and_medium_plus_as_pending_scope() {
    let script =
        fs::read_to_string(workflow_script()).expect("workflow state script should be readable");

    assert!(
        script.contains("pending Medium/Medium+/High/Critical review items"),
        "pending label description should not imply Medium findings are ignored"
    );
    assert!(
        script.contains("no pending Medium/Medium+/High/Critical findings"),
        "clean label description should cover both Medium and Medium+"
    );
    assert!(
        script.contains("Gemini Review 的 Medium/Medium+/High/Critical 问题"),
        "needs-human comment should tell reviewers Medium findings are actionable"
    );
    assert!(
        script.contains("Medium/Medium+/High/Critical 未自动修复说明"),
        "pending comments should cover both Medium and Medium+ findings"
    );
    assert!(
        script.contains("问题清单见上方 Codex 分析评论"),
        "state comments should always tell reviewers where the automatic issue list is"
    );
    assert!(
        script.contains("Medium/Medium+/High/Critical 对应状态"),
        "state comments should point humans to the automatic one-to-one status table"
    );
    assert!(
        !script.contains("medium+ review items"),
        "state labels must not narrow the loop to Medium+ only"
    );
}

#[test]
fn relaxed_clear_posts_review_status_table() {
    let script =
        fs::read_to_string(workflow_script()).expect("workflow state script should be readable");

    assert!(
        script.contains("relaxed_review_status_comment"),
        "relaxed clear should build a fresh review status comment instead of only pointing to older comments"
    );
    assert!(
        script.contains("CODEX_RESULT_PATH") && script.contains(".issue_statuses // []"),
        "relaxed clear should read pr-auto-fix issue_statuses from the persisted result JSON"
    );
    assert!(
        script.contains("| # | 严重级别 | 位置 | Gemini 问题 | Codex 状态 | 解决方案/说明 |"),
        "relaxed clear comments should include a visible Gemini issue/solution table"
    );
    assert!(
        script.contains("宽松模式只表示这些问题不再阻塞自动闭环，不代表每一项都已代码修复"),
        "relaxed clear must explain that green automation does not mean every Gemini finding was fixed"
    );
}

fn plan(envs: &[(&str, &str)]) -> HashMap<String, String> {
    let script = workflow_script();
    let output = Command::new("bash")
        .arg(script)
        .arg("plan")
        .env("MAX_ROUNDS", "2")
        .envs(envs.iter().copied())
        .output()
        .expect("workflow state script should run");

    assert!(
        output.status.success(),
        "status={:?}\nstdout={}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

fn workflow_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".github/scripts/codex-auto-fix-state.sh")
}

fn repo_path(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn codex_auto_fix_workflow() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".github/workflows/codex-auto-fix.yml")
}

fn gemini_kickoff_workflow() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".github/workflows/gemini-review-kickoff.yml")
}
