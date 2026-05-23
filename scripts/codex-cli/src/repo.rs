use crate::types::{ChangelogEntryInput, SkillPackResolvedSkill, SkillPackSkillMeta};
use serde_json::json;
use std::fs;
use std::path::{Component, Path};
use std::process::Command as StdCommand;
use std::process::Output;
use std::time::{SystemTime, UNIX_EPOCH};

pub use crate::patch::PatchApplyResult;
pub use crate::patch::unified_diff::{
    apply_patch_safely, apply_patch_safely_in, apply_patch_with_details_in, classify_apply_failure,
    validate_unified_diff_for_file,
};

/// 从大模型输出中提取第一个代码块内容。
///
/// 为什么需要：
/// - `Review/Refactor/Doc` 等子命令要求模型返回代码块
/// - 模型有时会用 ```rust 或 ``` 包裹，这里做一次“尽量提取”
pub fn extract_code_block(text: &str) -> Option<String> {
    let start_tag = "```rust";
    let end_tag = "```";

    if let Some(start) = text.find(start_tag) {
        let content_start = start + start_tag.len();
        if let Some(end) = text[content_start..].find(end_tag) {
            return Some(text[content_start..content_start + end].trim().to_string());
        }
    }

    let start_tag_generic = "```";
    if let Some(start) = text.find(start_tag_generic) {
        let content_start = start + start_tag_generic.len();
        if let Some(end) = text[content_start..].find(end_tag) {
            return Some(text[content_start..content_start + end].trim().to_string());
        }
    }

    None
}

/// 获取当前 git 仓库根目录。
///
/// 作为多数 repo 相关操作的前置条件（例如读取 `AGENTS.md`、写入 `docs/CHANGELOG.md`）。
pub fn git_repo_root() -> Result<String, Box<dyn std::error::Error>> {
    git_repo_root_from(None)
}

pub fn git_repo_root_from(cwd: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    let output = StdCommand::new("git")
        .args(cwd.into_iter().flat_map(|p| ["-C", p]))
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    if !output.status.success() {
        return Err("无法定位 git 仓库根目录".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// 读取仓库根目录下的 `AGENTS.md` 规则文本（用于注入到模型 system prompt）。
///
/// 读取失败时使用一个最小兜底规则，避免 CLI 直接崩溃影响可用性。
pub fn read_agents_rules() -> String {
    let root = git_repo_root().ok();
    read_rules(root.as_deref(), None)
}

pub fn read_rules(repo_root: Option<&str>, rules_file: Option<&str>) -> String {
    if let Some(root) = repo_root {
        if let Some(p) = rules_file
            && let Ok(path) = resolve_existing_repo_path(root, p)
            && let Ok(content) = fs::read_to_string(path)
        {
            return content;
        }

        if let Ok(path) = resolve_existing_repo_path(root, "AGENTS.md")
            && let Ok(content) = fs::read_to_string(path)
        {
            return content;
        }
    }

    "严格遵循项目架构铁律和 TDD 铁律。".to_string()
}

/// 构建写入 CHANGELOG 的条目文本（稳定格式，便于后续检索/复盘）。
pub fn build_changelog_entry(input: &ChangelogEntryInput) -> String {
    let mut files = input.files.clone();
    files.sort();
    files.dedup();

    let security_info = if input.security_passed {
        "通过"
    } else {
        "发现潜在风险"
    };

    let header = format!(
        "#### 🤖 Codex Auto-Fix (PR #{}, round {}) — ts={}\n\n",
        input.pr_number, input.round, input.unix_ts
    );

    let mut body = String::new();
    body.push_str(&format!("- 安全扫描：{}\n", security_info));
    body.push_str(&format!("- 质量评分：{} / 100\n", input.quality_score));
    body.push_str("- 变更文件：\n");
    for f in files {
        body.push_str(&format!("  - `{}`\n", f));
    }
    body.push('\n');

    header + &body
}

/// 将条目插入到 `docs/CHANGELOG.md` 中 `### 🤖 AI 自动修复` 区块下。
///
/// 插入策略：
/// - 若已存在该小节：插入到小节标题之后（最靠前，倒序）
/// - 若不存在但存在 `[未发布]`：在其下创建小节并插入
/// - 否则：追加到文末（兜底）
pub fn update_changelog(
    changelog_path: &str,
    entry: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let original = fs::read_to_string(changelog_path)?;

    let marker = "### 🤖 AI 自动修复";
    let updated = if original.contains(marker) {
        original.replacen(marker, &format!("{}\n\n{}", marker, entry.trim_end()), 1)
    } else if let Some(pos) = original.find("## [未发布]") {
        let insert_at = original[pos..]
            .find('\n')
            .map(|i| pos + i + 1)
            .unwrap_or(original.len());
        let mut out = String::new();
        out.push_str(&original[..insert_at]);
        out.push_str("\n### 🤖 AI 自动修复\n\n");
        out.push_str(entry.trim_end());
        out.push('\n');
        out.push_str(&original[insert_at..]);
        out
    } else {
        format!("{}\n\n{}", original.trim_end(), entry.trim_end())
    };

    fs::write(changelog_path, updated)?;
    Ok(())
}

/// 在 repo 根目录下的 `docs/CHANGELOG.md` 追加一次 “AI 自动修复” 记录。
///
/// 该函数会把 `docs/CHANGELOG.md` 加入 `fixed_files`，以确保后续提交/推送包含变更记录。
pub fn append_ai_changelog(
    fixed_files: &mut Vec<String>,
    input: &ChangelogEntryInput,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = git_repo_root().map_err(|e| {
        format!(
            "DocumentationSkill: 获取 git 仓库根目录失败（需要在 git 仓库内运行）。原始错误: {}",
            e
        )
    })?;

    let entry = build_changelog_entry(input);
    let changelog_path = format!("{}/docs/CHANGELOG.md", root);
    update_changelog(&changelog_path, &entry).map_err(|e| {
        format!(
            "DocumentationSkill: 写入 docs/CHANGELOG.md 失败（path={}）。原始错误: {}",
            changelog_path, e
        )
    })?;

    if !fixed_files.iter().any(|f| f == "docs/CHANGELOG.md") {
        fixed_files.push("docs/CHANGELOG.md".to_string());
    }

    Ok(())
}

pub fn resolve_changelog_path(repo_root: &str, changelog_path: Option<&str>) -> Option<String> {
    if let Some(p) = changelog_path {
        return resolve_repo_path(repo_root, p)
            .ok()
            .map(|path| path.to_string_lossy().to_string());
    }

    let default_path = resolve_repo_path(repo_root, "docs/CHANGELOG.md").ok()?;
    if default_path.exists() {
        Some(default_path.to_string_lossy().to_string())
    } else {
        None
    }
}

pub fn append_ai_changelog_in(
    repo_root: &str,
    fixed_files: &mut Vec<String>,
    input: &ChangelogEntryInput,
    changelog_path: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(changelog_path) = resolve_changelog_path(repo_root, changelog_path) else {
        return Ok(());
    };

    let entry = build_changelog_entry(input);
    update_changelog(&changelog_path, &entry)?;

    let rel = if Path::new(&changelog_path).is_absolute() {
        Path::new(&changelog_path)
            .strip_prefix(repo_root)
            .ok()
            .and_then(|p| p.to_str())
            .map(|s| s.trim_start_matches('/').to_string())
    } else {
        Some(changelog_path.clone())
    };

    if let Some(rel) = rel
        && !fixed_files.iter().any(|f| f == &rel)
    {
        fixed_files.push(rel);
    }

    Ok(())
}

/// 获取当前 Unix 时间戳（秒）。
pub fn now_unix_ts() -> Result<u64, Box<dyn std::error::Error>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

fn canonical_repo_root(repo_root: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    Ok(Path::new(repo_root).canonicalize()?)
}

fn resolve_repo_path(
    repo_root: &str,
    path: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let root = canonical_repo_root(repo_root)?;
    let candidate = if Path::new(path).is_absolute() {
        std::path::PathBuf::from(path)
    } else {
        root.join(path)
    };
    let file_name = candidate
        .file_name()
        .ok_or_else(|| format!("无效文件路径: {}", path))?;

    let parent = candidate.parent().unwrap_or(&root);
    let mut existing_parent = parent;
    let mut missing_components = Vec::new();
    while !existing_parent.exists() {
        let Some(name) = existing_parent.file_name() else {
            return Err(format!("无效文件路径: {}", path).into());
        };
        missing_components.push(name.to_os_string());
        existing_parent = existing_parent
            .parent()
            .ok_or_else(|| format!("无效文件路径: {}", path))?;
    }

    let mut canonical_parent = existing_parent.canonicalize()?;
    if !canonical_parent.starts_with(&root) {
        return Err(format!("拒绝访问仓库外路径: {}", path).into());
    }
    for component in missing_components.iter().rev() {
        canonical_parent.push(component);
    }

    Ok(canonical_parent.join(file_name))
}

fn resolve_existing_repo_path(
    repo_root: &str,
    path: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let root = canonical_repo_root(repo_root)?;
    let resolved = resolve_repo_path(repo_root, path)?.canonicalize()?;
    if !resolved.starts_with(&root) {
        return Err(format!("拒绝访问仓库外路径: {}", path).into());
    }
    Ok(resolved)
}

fn resolve_repo_path_for_create(
    repo_root: &str,
    path: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let root = canonical_repo_root(repo_root)?;
    let raw_path = Path::new(path);
    let relative = if raw_path.is_absolute() {
        raw_path
            .strip_prefix(&root)
            .map_err(|_| format!("拒绝访问仓库外路径: {}", path))?
    } else {
        raw_path
    };

    let mut components = relative.components().peekable();
    let mut current = root.clone();
    while let Some(component) = components.next() {
        let is_final = components.peek().is_none();
        match component {
            Component::Normal(part) if is_final => {
                return Ok(current.join(part));
            }
            Component::Normal(part) => {
                current.push(part);
                if !current.exists()
                    && let Err(e) = fs::create_dir(&current)
                    && e.kind() != std::io::ErrorKind::AlreadyExists
                {
                    return Err(e.into());
                }
                if current.exists() {
                    let metadata = fs::symlink_metadata(&current)?;
                    if metadata.file_type().is_symlink() {
                        return Err(format!("拒绝通过符号链接目录写入: {}", path).into());
                    }
                    if !metadata.is_dir() {
                        return Err(format!("写入路径父级不是目录: {}", path).into());
                    }
                }
                let canonical = current.canonicalize()?;
                if !canonical.starts_with(&root) {
                    return Err(format!("拒绝访问仓库外路径: {}", path).into());
                }
                current = canonical;
            }
            _ => return Err(format!("无效文件路径: {}", path).into()),
        }
    }

    Err(format!("无效文件路径: {}", path).into())
}

pub fn read_repo_file(repo_root: &str, path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let abs = resolve_existing_repo_path(repo_root, path)?;
    Ok(fs::read_to_string(abs)?)
}

pub fn write_repo_file(
    repo_root: &str,
    path: &str,
    content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let abs = resolve_repo_path(repo_root, path)?;
    let parent = abs
        .parent()
        .ok_or_else(|| format!("无效文件路径: {}", path))?;
    fs::create_dir_all(parent)?;
    let tmp = parent.join(format!(
        ".codex-cli-write-{}-{}.tmp",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    let _tmp_cleanup = TempFileCleanup { path: tmp.clone() };
    {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        std::io::Write::write_all(&mut file, content.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&tmp, &abs)?;
    Ok(())
}

pub(crate) fn write_repo_file_creating_parent(
    repo_root: &str,
    path: &str,
    content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let abs = resolve_repo_path_for_create(repo_root, path)?;
    let parent = abs
        .parent()
        .ok_or_else(|| format!("无效文件路径: {}", path))?;
    let tmp = parent.join(format!(
        ".codex-cli-write-{}-{}.tmp",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    let _tmp_cleanup = TempFileCleanup { path: tmp.clone() };
    {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        std::io::Write::write_all(&mut file, content.as_bytes())?;
        file.sync_all()?;
    }
    fs::rename(&tmp, &abs)?;
    Ok(())
}

/// 通过 GitHub CLI 以 raw 形式读取仓库文件内容。
///
/// 选择 `gh api ... Accept: raw` 的原因：
/// - 避免 base64/JSON 包装层，拿到直接文本用于 diff 生成
pub fn gh_get_file_raw(repo: &str, path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let output = checked_output(StdCommand::new("gh").args([
        "api",
        &format!("repos/{}/contents/{}", repo, path),
        "-H",
        "Accept: application/vnd.github.v3.raw",
    ]))
    .map_err(|e| format!("无法读取文件 {}: {}", path, e))?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// 提交并推送本轮修复。
///
/// 注意：自动修复提交必须触发 CI；Gemini kickoff 通过识别该提交消息避免重复请求 review。
pub fn commit_and_push(fixed_files: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    commit_and_push_in(".", fixed_files, true)
}

pub fn commit_and_push_in(
    repo_root: &str,
    fixed_files: &[String],
    push: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let msg = auto_fix_commit_message(fixed_files.len());

    let safe_fixed_files = fixed_files
        .iter()
        .map(|file| validate_git_pathspec_file(file))
        .collect::<Result<Vec<_>, _>>()?;
    run_auto_fix_verify_commands(repo_root)?;
    checked_output(
        StdCommand::new("git")
            .args(["-C", repo_root])
            .args(["add", "--"])
            .args(safe_fixed_files.iter().map(String::as_str)),
    )?;
    checked_output(
        StdCommand::new("git")
            .args(["-C", repo_root])
            .args(["commit", "-m", &msg]),
    )?;
    if push {
        if publish_via_github_api_only() {
            eprintln!(
                "⚠️ CODEX_PUBLISH_VIA_GH_API=true，跳过 git push 并改用 GitHub API 发布当前提交"
            );
            push_head_via_github_api(repo_root)?;
            return Ok(());
        }

        let push_result = checked_output_with_retry(
            || {
                let mut cmd = StdCommand::new("git");
                cmd.args(["-C", repo_root]).args(["push", "origin", "HEAD"]);
                cmd
            },
            "git push",
        );
        if let Err(error) = push_result {
            let message = error.to_string();
            if classify_git_network_error(&message).is_some() {
                eprintln!("⚠️ git push 多次网络失败，改用 GitHub API 发布当前提交");
                push_head_via_github_api(repo_root)?;
            } else {
                return Err(error);
            }
        }
    }

    Ok(())
}

fn run_auto_fix_verify_commands(repo_root: &str) -> Result<(), Box<dyn std::error::Error>> {
    let Ok(commands) = std::env::var("CODEX_AUTO_FIX_VERIFY_COMMANDS") else {
        return Ok(());
    };
    let commands = commands.trim();
    if commands.is_empty() {
        return Ok(());
    }

    eprintln!("🧪 [AutoFix] 正在执行提交前验证: CODEX_AUTO_FIX_VERIFY_COMMANDS");
    let output = StdCommand::new("sh")
        .arg("-c")
        .arg(commands)
        .current_dir(repo_root)
        .output()?;
    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "CODEX_AUTO_FIX_VERIFY_COMMANDS 验证失败，已阻止自动修复提交/推送。\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .into())
}

fn validate_git_pathspec_file(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    if path.is_empty()
        || path.starts_with(':')
        || Path::new(path).is_absolute()
        || Path::new(path)
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(format!("拒绝不安全 git pathspec: {}", path).into());
    }
    Ok(format!(":(literal){}", path))
}

fn publish_via_github_api_only() -> bool {
    env_flag_enabled(std::env::var("CODEX_PUBLISH_VIA_GH_API").ok().as_deref())
}

fn auto_fix_commit_message(file_count: usize) -> String {
    let prefix = if workflow_owns_next_gemini_review() {
        "🤖 codex auto-fix"
    } else {
        "🤖 codex local auto-fix"
    };
    format!("{prefix}: 修复 {file_count} 个文件 (基于 Gemini Review)")
}

fn workflow_owns_next_gemini_review() -> bool {
    env_flag_enabled(
        std::env::var("CODEX_AUTO_FIX_WORKFLOW_OWNS_REVIEW")
            .ok()
            .as_deref(),
    )
}

fn env_flag_enabled(value: Option<&str>) -> bool {
    value
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

fn push_head_via_github_api(repo_root: &str) -> Result<(), Box<dyn std::error::Error>> {
    let repo = github_repo_full_name()?;
    let branch = current_branch(repo_root)?;
    let remote_head = gh_api_get_jq(
        &format!("repos/{repo}/git/ref/heads/{branch}"),
        ".object.sha",
    )?;
    ensure_api_fallback_fast_forward_safe(repo_root, &remote_head)?;
    let base_tree = gh_api_get_jq(
        &format!("repos/{repo}/git/commits/{remote_head}"),
        ".tree.sha",
    )?;
    let tree_entries = api_tree_entries_for_head(repo_root)?;
    if tree_entries.is_empty() {
        return Ok(());
    }

    let tree = gh_api_json_jq(
        "POST",
        &format!("repos/{repo}/git/trees"),
        json!({
            "base_tree": base_tree,
            "tree": tree_entries,
        }),
        ".sha",
    )?;
    let message = git_stdout(repo_root, &["log", "-1", "--pretty=%B"])?;
    let commit = gh_api_json_jq(
        "POST",
        &format!("repos/{repo}/git/commits"),
        json!({
            "message": message.trim_end(),
            "tree": tree,
            "parents": [remote_head],
        }),
        ".sha",
    )?;
    gh_api_json(
        "PATCH",
        &format!("repos/{repo}/git/refs/heads/{branch}"),
        json!({
            "sha": commit,
            "force": false,
        }),
    )?;
    eprintln!("✅ GitHub API fallback 已更新远端分支 {branch}");
    Ok(())
}

fn github_repo_full_name() -> Result<String, Box<dyn std::error::Error>> {
    if let Ok(repo) = std::env::var("GITHUB_REPOSITORY")
        && !repo.trim().is_empty()
    {
        return Ok(repo);
    }

    let output = checked_output_with_retry(
        || {
            let mut cmd = StdCommand::new("gh");
            cmd.args([
                "repo",
                "view",
                "--json",
                "nameWithOwner",
                "-q",
                ".nameWithOwner",
            ]);
            cmd
        },
        "gh repo view",
    )?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn current_branch(repo_root: &str) -> Result<String, Box<dyn std::error::Error>> {
    let branch = git_stdout(repo_root, &["branch", "--show-current"])?;
    let branch = branch.trim();
    if !branch.is_empty() {
        return Ok(branch.to_string());
    }
    if let Ok(branch) = std::env::var("GITHUB_HEAD_REF")
        && !branch.trim().is_empty()
    {
        return Ok(branch);
    }
    Err("无法确定当前分支，无法使用 GitHub API fallback 推送".into())
}

fn ensure_api_fallback_fast_forward_safe(
    repo_root: &str,
    remote_head: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let local_head = git_stdout(repo_root, &["rev-parse", "HEAD"])?;
    let local_parent = git_stdout(repo_root, &["rev-parse", "HEAD^"])?;
    if remote_head.trim() != local_parent.trim() {
        if local_parent_is_synthetic_bootstrap_for(repo_root, remote_head.trim())? {
            return Ok(());
        }
        return Err(format!(
            "拒绝 GitHub API fallback 推送：远端 HEAD 与本地提交父提交不一致（local_head={}, remote_head={}）",
            local_head.trim(),
            remote_head.trim()
        )
        .into());
    }

    let output = StdCommand::new("git")
        .args(["-C", repo_root])
        .args([
            "merge-base",
            "--is-ancestor",
            remote_head.trim(),
            local_head.trim(),
        ])
        .output()?;
    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "拒绝 GitHub API fallback 推送：远端 HEAD 不是本地 HEAD 的祖先（local_head={}, remote_head={}）",
        local_head.trim(),
        remote_head.trim()
    )
    .into())
}

fn local_parent_is_synthetic_bootstrap_for(
    repo_root: &str,
    remote_head: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let subject = git_stdout(repo_root, &["log", "-1", "--pretty=%s", "HEAD^"])?;
    Ok(subject.trim() == format!("bootstrap PR head {remote_head}"))
}

fn api_tree_entries_for_head(
    repo_root: &str,
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let changed = git_stdout(
        repo_root,
        &["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"],
    )?;
    let mut entries = Vec::new();
    for path in changed
        .lines()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    {
        match git_stdout(repo_root, &["show", &format!("HEAD:{path}")]) {
            Ok(content) => entries.push(json!({
                "path": path,
                "mode": git_file_mode(repo_root, path),
                "type": "blob",
                "content": content,
            })),
            Err(_) => entries.push(json!({
                "path": path,
                "mode": "100644",
                "type": "blob",
                "sha": serde_json::Value::Null,
            })),
        }
    }
    Ok(entries)
}

fn git_file_mode(repo_root: &str, path: &str) -> String {
    git_stdout(repo_root, &["ls-tree", "HEAD", "--", path])
        .ok()
        .and_then(|line| line.split_whitespace().next().map(ToString::to_string))
        .filter(|mode| mode == "100755")
        .unwrap_or_else(|| "100644".to_string())
}

fn git_stdout(repo_root: &str, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = checked_output(StdCommand::new("git").arg("-C").arg(repo_root).args(args))?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn gh_api_get_jq(endpoint: &str, jq: &str) -> Result<String, Box<dyn std::error::Error>> {
    let endpoint = endpoint.to_string();
    let jq = jq.to_string();
    let output = checked_output_with_retry(
        || {
            let mut cmd = StdCommand::new("gh");
            cmd.args(["api", &endpoint, "--jq", &jq]);
            cmd
        },
        "gh api",
    )?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn gh_api_json_jq(
    method: &str,
    endpoint: &str,
    payload: serde_json::Value,
    jq: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let output = gh_api_json_output(method, endpoint, payload, Some(jq))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn gh_api_json(
    method: &str,
    endpoint: &str,
    payload: serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    gh_api_json_output(method, endpoint, payload, None)?;
    Ok(())
}

struct TempFileCleanup {
    path: std::path::PathBuf,
}

impl Drop for TempFileCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn gh_api_json_output(
    method: &str,
    endpoint: &str,
    payload: serde_json::Value,
    jq: Option<&str>,
) -> Result<Output, Box<dyn std::error::Error>> {
    let tmp = std::env::temp_dir().join(format!(
        "codex-cli-gh-api-{}-{}.json",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    let _tmp_cleanup = TempFileCleanup { path: tmp.clone() };
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = file.metadata()?.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(&tmp, permissions)?;
    }
    std::io::Write::write_all(&mut file, &serde_json::to_vec(&payload)?)?;
    file.sync_all()?;
    drop(file);
    let method = method.to_string();
    let endpoint = endpoint.to_string();
    let jq = jq.map(ToString::to_string);
    let input = tmp.to_string_lossy().to_string();
    checked_output_with_retry(
        || {
            let mut cmd = StdCommand::new("gh");
            cmd.args(["api", "-X", &method, &endpoint, "--input", &input]);
            if let Some(jq) = &jq {
                cmd.args(["--jq", jq]);
            }
            cmd
        },
        "gh api",
    )
}

fn checked_output_with_retry<F>(
    build_command: F,
    label: &str,
) -> Result<Output, Box<dyn std::error::Error>>
where
    F: FnMut() -> StdCommand,
{
    checked_output_with_retry_inner(build_command, label, true)
}

fn checked_output_with_retry_inner<F>(
    mut build_command: F,
    label: &str,
    sleep_between_attempts: bool,
) -> Result<Output, Box<dyn std::error::Error>>
where
    F: FnMut() -> StdCommand,
{
    let max_attempts = 3;
    let mut last_error: Option<String> = None;

    for attempt in 1..=max_attempts {
        let output = build_command().output()?;
        if output.status.success() {
            return Ok(output);
        }

        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let message = format!(
            "{} failed attempt {}/{} status={} stderr={}",
            label,
            attempt,
            max_attempts,
            output.status,
            stderr.trim()
        );
        last_error = Some(message.clone());

        let Some(network_reason) = classify_git_network_error(&stderr) else {
            return Err(message.into());
        };
        if attempt == max_attempts {
            return Err(message.into());
        }

        eprintln!(
            "⚠️ {} 网络抖动（{}），已重试 {}/{}",
            label, network_reason, attempt, max_attempts
        );
        if sleep_between_attempts {
            let sleep_secs = std::env::var("CODEX_REMOTE_RETRY_SLEEP_SECONDS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or((attempt * 10) as u64);
            if sleep_secs > 0 {
                std::thread::sleep(std::time::Duration::from_secs(sleep_secs));
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| format!("{} failed without output", label))
        .into())
}

#[cfg(test)]
fn is_transient_remote_error(stderr: &str) -> bool {
    classify_git_network_error(stderr).is_some()
}

fn classify_git_network_error(stderr: &str) -> Option<&'static str> {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("empty reply from server") {
        Some("empty_reply_from_server")
    } else if lower.contains("failed to connect") {
        Some("failed_to_connect")
    } else if lower.contains("connection reset") {
        Some("connection_reset")
    } else if lower.contains("connection timed out") || lower.contains("operation timed out") {
        Some("timeout")
    } else if lower.contains("unexpected disconnect")
        || lower.contains("the remote end hung up unexpectedly")
    {
        Some("unexpected_disconnect")
    } else if lower.contains("http/2 stream") {
        Some("http2_stream")
    } else if lower.contains("tls") {
        Some("tls")
    } else {
        None
    }
}

/// 在指定 PR 下发布评论（用于 Dry-Run 提示与修复结果回传）。
pub fn post_comment(pr_number: u32, body: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pr_number = pr_number.to_string();
    checked_output_with_retry(
        || {
            let mut cmd = StdCommand::new("gh");
            cmd.args(["pr", "comment", &pr_number, "--body", body]);
            cmd
        },
        "gh pr comment",
    )?;
    Ok(())
}

fn checked_output(cmd: &mut StdCommand) -> Result<Output, Box<dyn std::error::Error>> {
    let output = cmd.output()?;
    if output.status.success() {
        return Ok(output);
    }

    Err(format!("命令执行失败 status={}", output.status).into())
}

pub fn discover_skill_pack_skills(
    plugin_root: &str,
) -> Result<Vec<SkillPackSkillMeta>, Box<dyn std::error::Error>> {
    let skills_dir = Path::new(plugin_root).join("skills");
    if !skills_dir.exists() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(skills_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        let skill_md = path.join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }
        let text = fs::read_to_string(&skill_md)?;
        let (name, description, version, _body) = parse_skill_md(&text);
        out.push(SkillPackSkillMeta {
            id,
            name,
            description,
            version,
            skill_md_path: skill_md.to_string_lossy().to_string(),
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

pub fn resolve_skill_pack_skill(
    plugin_root: &str,
    skill: &str,
) -> Result<SkillPackResolvedSkill, Box<dyn std::error::Error>> {
    let candidates = discover_skill_pack_skills(plugin_root)?;
    let Some(meta) = candidates.into_iter().find(|m| {
        if m.id == skill {
            return true;
        }
        if let Some(name) = &m.name {
            return name == skill;
        }
        false
    }) else {
        return Err(format!("未找到 skill: {}", skill).into());
    };

    let skill_md = Path::new(&meta.skill_md_path);
    let text = fs::read_to_string(skill_md)?;
    let (_name, _description, _version, body) = parse_skill_md(&text);
    Ok(SkillPackResolvedSkill { meta, body })
}

pub fn read_skill_pack_agents_rules(plugin_root: &str) -> String {
    read_rules(Some(plugin_root), None)
}

pub fn find_skill_pack_root_from(start: &str) -> Option<String> {
    let mut cur = Path::new(start);
    loop {
        let marker = cur.join(".claude-plugin").join("plugin.json");
        if marker.exists() {
            return Some(cur.to_string_lossy().to_string());
        }
        cur = cur.parent()?;
    }
}

fn parse_skill_md(text: &str) -> (Option<String>, Option<String>, Option<String>, String) {
    let trimmed = text.trim_start();
    if !trimmed.starts_with("---\n") && trimmed != "---" && !trimmed.starts_with("---\r\n") {
        return (None, None, None, text.to_string());
    }

    let normalized = text.replace("\r\n", "\n");
    let mut lines = normalized.lines();
    let first = lines.next();
    if first != Some("---") {
        return (None, None, None, text.to_string());
    }

    let mut frontmatter = Vec::new();
    for line in &mut lines {
        if line == "---" {
            break;
        }
        frontmatter.push(line.to_string());
    }

    let mut name: Option<String> = None;
    let mut version: Option<String> = None;
    let mut description: Option<String> = None;

    let mut i = 0usize;
    while i < frontmatter.len() {
        let line = frontmatter[i].as_str();
        if let Some(rest) = line.strip_prefix("name:") {
            name = Some(rest.trim().trim_matches('"').to_string());
            i += 1;
            continue;
        }
        if let Some(rest) = line.strip_prefix("version:") {
            version = Some(rest.trim().trim_matches('"').to_string());
            i += 1;
            continue;
        }
        if let Some(rest) = line.strip_prefix("description:") {
            let rest = rest.trim();
            if rest == "|" {
                let mut buf = Vec::new();
                i += 1;
                while i < frontmatter.len() {
                    let l = frontmatter[i].as_str();
                    if l.starts_with(' ') || l.starts_with('\t') {
                        buf.push(l.trim().to_string());
                        i += 1;
                        continue;
                    }
                    break;
                }
                let joined = buf.join("\n").trim().to_string();
                if !joined.is_empty() {
                    description = Some(joined);
                }
                continue;
            }
            let single = rest.trim_matches('"').to_string();
            if !single.is_empty() {
                description = Some(single);
            }
            i += 1;
            continue;
        }
        i += 1;
    }

    let body = lines
        .collect::<Vec<_>>()
        .join("\n")
        .trim_start()
        .to_string();
    (name, description, version, body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn codex_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn lock_codex_env() -> std::sync::MutexGuard<'static, ()> {
        codex_env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn publish_via_github_api_flag_accepts_only_truthy_values() {
        assert!(env_flag_enabled(Some("true")));
        assert!(env_flag_enabled(Some("TRUE")));
        assert!(env_flag_enabled(Some("1")));
        assert!(env_flag_enabled(Some("yes")));
        assert!(!env_flag_enabled(Some("false")));
        assert!(!env_flag_enabled(Some("0")));
        assert!(!env_flag_enabled(Some("")));
        assert!(!env_flag_enabled(None));
    }

    #[test]
    fn parse_skill_md_supports_frontmatter_and_body() {
        let text = r#"---
name: demo-skill
description: |
  line1
  line2
version: "1.2.3"
---

BODY
"#;
        let (name, description, version, body) = parse_skill_md(text);
        assert_eq!(name.as_deref(), Some("demo-skill"));
        assert_eq!(description.as_deref(), Some("line1\nline2"));
        assert_eq!(version.as_deref(), Some("1.2.3"));
        assert!(body.starts_with("BODY"));
    }

    #[test]
    fn discover_skill_pack_skills_finds_skills_dir() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("codex-cli-skill-pack-{}", now));
        let skills = dir.join("skills");
        fs::create_dir_all(skills.join("a-skill")).unwrap();
        fs::create_dir_all(skills.join("b-skill")).unwrap();
        fs::write(
            skills.join("a-skill").join("SKILL.md"),
            "---\nname: a-skill\n---\n\nA\n",
        )
        .unwrap();
        fs::write(
            skills.join("b-skill").join("SKILL.md"),
            "---\nname: b\n---\n\nB\n",
        )
        .unwrap();

        let found = discover_skill_pack_skills(dir.to_str().unwrap()).unwrap();
        let _ = fs::remove_dir_all(&dir);

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].id, "a-skill");
        assert_eq!(found[1].id, "b-skill");
    }

    #[test]
    fn find_skill_pack_root_from_walks_up_to_plugin_json() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("codex-cli-find-pack-{}", now));
        let plugin_root = dir.join("pack");
        let nested = plugin_root.join("a").join("b");
        fs::create_dir_all(nested.join("c")).unwrap();
        fs::create_dir_all(plugin_root.join(".claude-plugin")).unwrap();
        fs::write(plugin_root.join(".claude-plugin").join("plugin.json"), "{}").unwrap();

        let found = find_skill_pack_root_from(nested.to_str().unwrap()).unwrap();
        let _ = fs::remove_dir_all(&dir);

        assert!(found.ends_with("/pack"));
    }

    #[test]
    fn build_changelog_entry_includes_files_and_scores() {
        let input = ChangelogEntryInput {
            pr_number: 123,
            round: 1,
            unix_ts: 1_700_000_000,
            files: vec![
                "src/a.rs".to_string(),
                "src/b.rs".to_string(),
                "src/a.rs".to_string(),
            ],
            security_passed: true,
            quality_score: 95,
        };

        let entry = build_changelog_entry(&input);
        assert!(entry.contains("PR #123"));
        assert!(entry.contains("round 1"));
        assert!(entry.contains("安全扫描：通过"));
        assert!(entry.contains("质量评分：95 / 100"));
        assert!(entry.contains("`src/a.rs`"));
        assert!(entry.contains("`src/b.rs`"));
    }

    #[test]
    fn update_changelog_inserts_under_unreleased() {
        let base = "# CHANGELOG\n\n## [未发布] — 2026 年（当前会话）\n\n### 🧱 架构调整\n";
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("codex-cli-changelog-{}.md", now));
        fs::write(&path, base).unwrap();
        update_changelog(path.to_str().unwrap(), "#### entry\n").unwrap();
        let out = fs::read_to_string(&path).unwrap();
        let _ = fs::remove_file(&path);
        assert!(out.contains("### 🤖 AI 自动修复"));
        assert!(out.contains("#### entry"));
    }

    #[test]
    fn resolve_changelog_path_returns_none_when_default_missing() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("codex-cli-no-changelog-{}", now));
        fs::create_dir_all(&dir).unwrap();

        let resolved = resolve_changelog_path(dir.to_str().unwrap(), None);
        let _ = fs::remove_dir_all(&dir);

        assert!(resolved.is_none());
    }

    #[test]
    fn resolve_changelog_path_returns_default_when_present() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("codex-cli-with-changelog-{}", now));
        let docs = dir.join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("CHANGELOG.md"), "# CHANGELOG\n").unwrap();

        let resolved = resolve_changelog_path(dir.to_str().unwrap(), None).unwrap();
        let _ = fs::remove_dir_all(&dir);

        assert!(resolved.ends_with("/docs/CHANGELOG.md"));
    }

    #[test]
    fn resolve_changelog_path_joins_relative_override() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("codex-cli-rel-changelog-{}", now));
        fs::create_dir_all(&dir).unwrap();

        let resolved = resolve_changelog_path(dir.to_str().unwrap(), Some("CHANGELOG.md")).unwrap();
        let _ = fs::remove_dir_all(&dir);

        assert!(resolved.ends_with("/CHANGELOG.md"));
    }

    fn create_patch_test_repo(name: &str) -> std::path::PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("codex-cli-{}-{}", name, now));
        fs::create_dir_all(dir.join("src")).unwrap();
        StdCommand::new("git")
            .arg("init")
            .arg(&dir)
            .output()
            .unwrap();
        fs::write(
            dir.join("src/lib.rs"),
            "pub fn value() -> i32 {\n    1\n}\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn apply_patch_preflight_rejects_patch_fragment_without_git_apply() {
        let repo = create_patch_test_repo("patch-fragment");
        let patch = "@@ -1,3 +1,3 @@\n pub fn value() -> i32 {\n-    1\n+    2\n }\n";

        let result =
            apply_patch_with_details_in(repo.to_str().unwrap(), "src/lib.rs", patch).unwrap();
        let _ = fs::remove_dir_all(&repo);

        assert!(!result.applied);
        assert_eq!(result.fail_reason.as_deref(), Some("malformed_diff"));
        assert!(result.stderr.contains("preflight"));
        assert!(result.stderr.contains("diff --git"));
    }

    #[test]
    fn apply_patch_preflight_rejects_malformed_hunk_header() {
        let repo = create_patch_test_repo("bad-hunk");
        let patch = "diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,3 +1,3\n pub fn value() -> i32 {\n-    1\n+    2\n }\n";

        let result =
            apply_patch_with_details_in(repo.to_str().unwrap(), "src/lib.rs", patch).unwrap();
        let _ = fs::remove_dir_all(&repo);

        assert!(!result.applied);
        assert_eq!(result.fail_reason.as_deref(), Some("malformed_diff"));
        assert!(result.stderr.contains("preflight"));
        assert!(result.stderr.contains("malformed hunk header"));
    }

    #[test]
    fn apply_patch_preflight_rejects_hunk_body_count_mismatch() {
        let repo = create_patch_test_repo("bad-hunk-count");
        let patch = "diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,4 +1,4 @@\n pub fn value() -> i32 {\n-    1\n+    2\n }\n";

        let result =
            apply_patch_with_details_in(repo.to_str().unwrap(), "src/lib.rs", patch).unwrap();
        let _ = fs::remove_dir_all(&repo);

        assert!(!result.applied);
        assert_eq!(result.fail_reason.as_deref(), Some("malformed_diff"));
        assert!(result.stderr.contains("preflight"));
        assert!(result.stderr.contains("hunk body count mismatch"));
    }

    #[test]
    fn transient_remote_errors_are_retryable() {
        assert!(is_transient_remote_error(
            "fatal: unable to access 'https://github.com/owner/repo/': Empty reply from server"
        ));
        assert_eq!(
            classify_git_network_error(
                "fatal: unable to access 'https://github.com/owner/repo/': Empty reply from server"
            ),
            Some("empty_reply_from_server")
        );
        assert!(is_transient_remote_error(
            "fatal: unable to access 'https://github.com/owner/repo/': Failed to connect to github.com port 443"
        ));
        assert!(is_transient_remote_error(
            "send-pack: unexpected disconnect while reading sideband packet"
        ));
        assert!(!is_transient_remote_error(
            "error: failed to push some refs to 'github.com:owner/repo.git'"
        ));
    }

    #[test]
    fn checked_output_with_retry_retries_empty_reply_from_server() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("codex-cli-retry-{}", now));
        fs::create_dir_all(&dir).unwrap();
        let attempts = dir.join("attempts");
        let script = dir.join("flaky.sh");
        fs::write(
            &script,
            format!(
                "#!/bin/sh\nset -eu\ncount=0\nif [ -f '{attempts}' ]; then count=$(cat '{attempts}'); fi\ncount=$((count + 1))\nprintf '%s' \"$count\" > '{attempts}'\nif [ \"$count\" -lt 3 ]; then\n  printf '%s\\n' 'fatal: unable to access https://github.com/owner/repo/: Empty reply from server' >&2\n  exit 1\nfi\nprintf '%s\\n' ok\n",
                attempts = attempts.display()
            ),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&script).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&script, permissions).unwrap();
        }

        let output = checked_output_with_retry_inner(
            || StdCommand::new(&script),
            "test remote command",
            false,
        )
        .unwrap();
        let attempt_count = fs::read_to_string(&attempts).unwrap();
        let _ = fs::remove_dir_all(&dir);

        assert_eq!(attempt_count, "3");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "ok");
    }

    #[test]
    fn commit_and_push_runs_configured_verify_commands_before_commit() {
        let _env_guard = lock_codex_env();
        let repo = create_patch_test_repo("pre-push-verify");
        StdCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        StdCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();
        StdCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["add", "src/lib.rs"])
            .output()
            .unwrap();
        StdCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["commit", "-m", "initial"])
            .output()
            .unwrap();
        fs::write(
            repo.join("src/lib.rs"),
            "pub fn value() -> i32 {\n    2\n}\n",
        )
        .unwrap();

        unsafe {
            std::env::set_var("CODEX_AUTO_FIX_VERIFY_COMMANDS", "exit 12");
        }
        let result = commit_and_push_in(repo.to_str().unwrap(), &["src/lib.rs".to_string()], false);
        unsafe {
            std::env::remove_var("CODEX_AUTO_FIX_VERIFY_COMMANDS");
        }

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("CODEX_AUTO_FIX_VERIFY_COMMANDS")
        );
        let head = checked_output(StdCommand::new("git").arg("-C").arg(&repo).args([
            "log",
            "--oneline",
            "-1",
        ]))
        .unwrap();
        assert!(
            String::from_utf8_lossy(&head.stdout).contains("initial"),
            "failed verification must prevent the auto-fix commit"
        );
        let _ = fs::remove_dir_all(&repo);
    }

    #[test]
    fn commit_and_push_uses_local_prefix_by_default_so_gemini_kickoff_runs() {
        let _env_guard = lock_codex_env();
        let repo = create_patch_test_repo("local-auto-fix-prefix");
        StdCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        StdCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();
        StdCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["add", "src/lib.rs"])
            .output()
            .unwrap();
        StdCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["commit", "-m", "initial"])
            .output()
            .unwrap();
        fs::write(
            repo.join("src/lib.rs"),
            "pub fn value() -> i32 {\n    2\n}\n",
        )
        .unwrap();

        unsafe {
            std::env::remove_var("CODEX_AUTO_FIX_WORKFLOW_OWNS_REVIEW");
        }
        let result = commit_and_push_in(repo.to_str().unwrap(), &["src/lib.rs".to_string()], false);

        let head = checked_output(StdCommand::new("git").arg("-C").arg(&repo).args([
            "log",
            "--format=%s",
            "-1",
        ]))
        .unwrap();
        let subject = String::from_utf8_lossy(&head.stdout);
        let _ = fs::remove_dir_all(&repo);

        assert!(result.is_ok(), "result={result:?}");
        assert!(
            subject.starts_with("🤖 codex local auto-fix:"),
            "local publish must not use the workflow-owned skip prefix: {subject}"
        );
        assert!(
            !subject.starts_with("🤖 codex auto-fix:"),
            "local publish would make gemini-review-kickoff skip incorrectly"
        );
    }

    #[test]
    fn commit_and_push_keeps_workflow_prefix_when_workflow_owns_review() {
        let _env_guard = lock_codex_env();
        let repo = create_patch_test_repo("workflow-auto-fix-prefix");
        StdCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        StdCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();
        StdCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["add", "src/lib.rs"])
            .output()
            .unwrap();
        StdCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["commit", "-m", "initial"])
            .output()
            .unwrap();
        fs::write(
            repo.join("src/lib.rs"),
            "pub fn value() -> i32 {\n    2\n}\n",
        )
        .unwrap();

        unsafe {
            std::env::set_var("CODEX_AUTO_FIX_WORKFLOW_OWNS_REVIEW", "true");
        }
        let result = commit_and_push_in(repo.to_str().unwrap(), &["src/lib.rs".to_string()], false);
        unsafe {
            std::env::remove_var("CODEX_AUTO_FIX_WORKFLOW_OWNS_REVIEW");
        }

        let head = checked_output(StdCommand::new("git").arg("-C").arg(&repo).args([
            "log",
            "--format=%s",
            "-1",
        ]))
        .unwrap();
        let subject = String::from_utf8_lossy(&head.stdout);
        let _ = fs::remove_dir_all(&repo);

        assert!(result.is_ok(), "result={result:?}");
        assert!(
            subject.starts_with("🤖 codex auto-fix:"),
            "workflow auto-fix must keep the exact skip prefix: {subject}"
        );
    }

    #[test]
    fn commit_and_push_falls_back_to_github_api_when_git_push_network_fails() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("codex-cli-api-push-fallback-{}", now));
        let repo = dir.join("repo");
        let bin = dir.join("bin");
        fs::create_dir_all(repo.join("src")).unwrap();
        fs::create_dir_all(&bin).unwrap();

        let real_git = StdCommand::new("sh")
            .arg("-c")
            .arg("command -v git")
            .output()
            .unwrap();
        let real_git = String::from_utf8_lossy(&real_git.stdout).trim().to_string();

        StdCommand::new(&real_git)
            .arg("init")
            .arg(&repo)
            .output()
            .unwrap();
        StdCommand::new(&real_git)
            .arg("-C")
            .arg(&repo)
            .args(["checkout", "-b", "codex/test"])
            .output()
            .unwrap();
        StdCommand::new(&real_git)
            .arg("-C")
            .arg(&repo)
            .args(["config", "user.name", "Codex Test"])
            .output()
            .unwrap();
        StdCommand::new(&real_git)
            .arg("-C")
            .arg(&repo)
            .args(["config", "user.email", "codex-test@example.invalid"])
            .output()
            .unwrap();
        fs::write(
            repo.join("src/lib.rs"),
            "pub fn value() -> i32 {\n    1\n}\n",
        )
        .unwrap();
        StdCommand::new(&real_git)
            .arg("-C")
            .arg(&repo)
            .args(["add", "."])
            .output()
            .unwrap();
        StdCommand::new(&real_git)
            .arg("-C")
            .arg(&repo)
            .args(["commit", "-m", "initial"])
            .output()
            .unwrap();
        let parent = StdCommand::new(&real_git)
            .arg("-C")
            .arg(&repo)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        let parent = String::from_utf8_lossy(&parent.stdout).trim().to_string();
        let tree = StdCommand::new(&real_git)
            .arg("-C")
            .arg(&repo)
            .args(["rev-parse", "HEAD^{tree}"])
            .output()
            .unwrap();
        let tree = String::from_utf8_lossy(&tree.stdout).trim().to_string();
        fs::write(
            repo.join("src/lib.rs"),
            "pub fn value() -> i32 {\n    2\n}\n",
        )
        .unwrap();

        let push_attempts = dir.join("push-attempts");
        let git_script = bin.join("git");
        fs::write(
            &git_script,
            format!(
                "#!/bin/sh\nset -eu\nrepo_arg=\"\"\nif [ \"${{1:-}}\" = \"-C\" ]; then repo_arg=\"-C $2\"; shift 2; fi\nif [ \"${{1:-}}\" = \"push\" ]; then\n  count=0\n  if [ -f '{push_attempts}' ]; then count=$(cat '{push_attempts}'); fi\n  count=$((count + 1))\n  printf '%s' \"$count\" > '{push_attempts}'\n  printf '%s\\n' 'fatal: unable to access https://github.com/owner/repo/: Empty reply from server' >&2\n  exit 128\nfi\nexec '{real_git}' $repo_arg \"$@\"\n",
                push_attempts = push_attempts.display(),
                real_git = real_git
            ),
        )
        .unwrap();

        let gh_invocations = dir.join("gh-invocations");
        let gh_script = bin.join("gh");
        fs::write(
            &gh_script,
            format!(
                "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{gh_invocations}'\nif [ \"$1\" = \"repo\" ]; then printf '%s\\n' 'owner/repo'; exit 0; fi\nif [ \"$1\" = \"api\" ]; then\n  args=\"$*\"\n  case \"$args\" in\n    *'-X PATCH'*'git/refs/heads/codex/test'*) printf '%s\\n' '{{}}'; exit 0 ;;\n    *'git/ref/heads/codex/test'*) printf '%s\\n' '{parent}'; exit 0 ;;\n    *'git/commits/{parent}'*) printf '%s\\n' '{tree}'; exit 0 ;;\n    *'-X POST'*'git/trees'*) printf '%s\\n' 'api-tree-sha'; exit 0 ;;\n    *'-X POST'*'git/commits'*) printf '%s\\n' 'api-commit-sha'; exit 0 ;;\n  esac\nfi\nprintf '%s\\n' \"unexpected gh $*\" >&2\nexit 1\n",
                gh_invocations = gh_invocations.display(),
                parent = parent,
                tree = tree
            ),
        )
        .unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for script in [&git_script, &gh_script] {
                let mut permissions = fs::metadata(script).unwrap().permissions();
                permissions.set_mode(0o755);
                fs::set_permissions(script, permissions).unwrap();
            }
        }

        let original_path = std::env::var("PATH").unwrap_or_default();
        let original_sleep = std::env::var("CODEX_REMOTE_RETRY_SLEEP_SECONDS").ok();
        let path = format!("{}:{}", bin.display(), original_path);
        unsafe {
            std::env::set_var("PATH", &path);
            std::env::set_var("CODEX_REMOTE_RETRY_SLEEP_SECONDS", "0");
        }
        let result = commit_and_push_in(repo.to_str().unwrap(), &["src/lib.rs".to_string()], true);
        unsafe {
            std::env::set_var("PATH", original_path);
            if let Some(original_sleep) = original_sleep {
                std::env::set_var("CODEX_REMOTE_RETRY_SLEEP_SECONDS", original_sleep);
            } else {
                std::env::remove_var("CODEX_REMOTE_RETRY_SLEEP_SECONDS");
            }
        }
        let push_attempt_count = fs::read_to_string(&push_attempts).unwrap();
        let gh_calls = fs::read_to_string(&gh_invocations).unwrap_or_default();
        let _ = fs::remove_dir_all(&dir);

        assert!(result.is_ok(), "result={result:?}");
        assert_eq!(push_attempt_count, "3");
        assert!(gh_calls.contains("-X PATCH"));
        assert!(gh_calls.contains("git/refs/heads/codex/test"));
    }

    #[test]
    fn api_fallback_accepts_synthetic_tarball_bootstrap_parent() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("codex-cli-bootstrap-parent-{}", now));
        let repo = dir.join("repo");
        fs::create_dir_all(repo.join("src")).unwrap();

        StdCommand::new("git")
            .arg("init")
            .arg(&repo)
            .output()
            .unwrap();
        StdCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["config", "user.name", "Codex Test"])
            .output()
            .unwrap();
        StdCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["config", "user.email", "codex-test@example.invalid"])
            .output()
            .unwrap();
        fs::write(
            repo.join("src/lib.rs"),
            "pub fn value() -> i32 {\n    1\n}\n",
        )
        .unwrap();
        StdCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["add", "."])
            .output()
            .unwrap();
        StdCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["commit", "-m", "remote head"])
            .output()
            .unwrap();
        let remote_head = git_stdout(repo.to_str().unwrap(), &["rev-parse", "HEAD"]).unwrap();
        StdCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["commit", "--allow-empty", "-m"])
            .arg(format!("bootstrap PR head {}", remote_head.trim()))
            .output()
            .unwrap();
        fs::write(
            repo.join("src/lib.rs"),
            "pub fn value() -> i32 {\n    2\n}\n",
        )
        .unwrap();
        StdCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["add", "."])
            .output()
            .unwrap();
        StdCommand::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["commit", "-m", "auto fix"])
            .output()
            .unwrap();

        let result = ensure_api_fallback_fast_forward_safe(repo.to_str().unwrap(), &remote_head);
        let _ = fs::remove_dir_all(&dir);

        assert!(result.is_ok(), "result={result:?}");
    }
}
