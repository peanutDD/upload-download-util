use dotenvy::dotenv;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;

const PROMPT_PLACEHOLDER: &str = "{prompt}";
const PROMPT_FILE_PLACEHOLDER: &str = "{prompt_file}";

/// 通过本地 Codex CLI 执行模型任务。
///
/// 约定：
/// - 不使用 GPT/OpenAI API
/// - 命令必须由 `CODEX_AGENT_COMMAND` 显式配置，避免本工具递归调用自己
/// - 若命令参数包含 `{prompt}`，直接替换为完整 prompt
/// - 若命令参数包含 `{prompt_file}`，写入临时 prompt 文件并替换为路径
/// - 若没有占位符，则把完整 prompt 写入 stdin
pub struct CodexClient {
    command: Vec<String>,
    timeout: Duration,
    started_at: Instant,
    budget: Option<Duration>,
    budget_reserve: Duration,
}

impl CodexClient {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        dotenv().ok();
        let raw = env::var("CODEX_AGENT_COMMAND")
            .map_err(|_| "请设置 CODEX_AGENT_COMMAND，例如：codex exec --skip-git-repo-check -")?;
        let command = normalize_agent_command(parse_command(&raw)?);
        reject_recursive_command(&command)?;
        Ok(Self {
            command,
            timeout: agent_timeout(),
            started_at: Instant::now(),
            budget: auto_fix_budget(),
            budget_reserve: auto_fix_budget_reserve(),
        })
    }

    pub async fn call(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let prompt = build_prompt(system_prompt, user_prompt);
        let remaining_budget = self
            .budget
            .map(|budget| budget.saturating_sub(self.started_at.elapsed()));
        let timeout = effective_command_timeout_from(
            self.timeout,
            remaining_budget,
            self.budget_reserve,
            Duration::from_secs(30),
        )
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        run_local_codex_command(&self.command, &prompt, timeout).await
    }
}

fn parse_command(raw: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let parts = raw
        .split_whitespace()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return Err("CODEX_AGENT_COMMAND 不能为空".into());
    }
    Ok(parts)
}

fn build_prompt(system_prompt: &str, user_prompt: &str) -> String {
    format!(
        "System instructions:\n{}\n\nUser request:\n{}\n",
        system_prompt, user_prompt
    )
}

async fn run_local_codex_command(
    command: &[String],
    prompt: &str,
    command_timeout: Duration,
) -> Result<String, Box<dyn std::error::Error>> {
    let (program, arg_template) = command
        .split_first()
        .ok_or("CODEX_AGENT_COMMAND 不能为空")?;

    let mut prompt_file: Option<PathBuf> = None;
    let mut uses_prompt_arg = false;
    let mut args = Vec::with_capacity(arg_template.len());
    for arg in arg_template {
        if arg == PROMPT_PLACEHOLDER {
            uses_prompt_arg = true;
            args.push(prompt.to_string());
        } else if arg == PROMPT_FILE_PLACEHOLDER {
            let path = write_prompt_file(prompt).await?;
            args.push(path.to_string_lossy().to_string());
            prompt_file = Some(path);
        } else {
            args.push(arg.to_string());
        }
    }

    let mut process = Command::new(program);
    process
        .args(&args)
        .stdin(if uses_prompt_arg || prompt_file.is_some() {
            Stdio::null()
        } else {
            Stdio::piped()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    configure_child_process_group(&mut process);

    let child_result = process.spawn();
    let mut child = match child_result {
        Ok(child) => child,
        Err(e) => {
            if let Some(path) = prompt_file {
                let _ = tokio::fs::remove_file(path).await;
            }
            return Err(format!("启动本地 Codex 命令失败: {} ({})", program, e).into());
        }
    };

    if !uses_prompt_arg && prompt_file.is_none() {
        let Some(mut stdin) = child.stdin.take() else {
            return Err("无法写入本地 Codex 命令 stdin".into());
        };
        stdin.write_all(prompt.as_bytes()).await?;
        drop(stdin);
    }

    let child_pid = child.id();
    let output_result = timeout(command_timeout, child.wait_with_output()).await;
    if let Some(path) = prompt_file {
        let _ = tokio::fs::remove_file(path).await;
    }

    let output = match output_result {
        Ok(output) => output?,
        Err(_) => {
            terminate_process_group(child_pid).await;
            return Err(format!("本地 Codex 命令超时（{} 秒）", command_timeout.as_secs()).into());
        }
    };

    if !output.status.success() {
        return Err(format!(
            "本地 Codex 命令失败，status={}，stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn normalize_agent_command(command: Vec<String>) -> Vec<String> {
    let model = env::var("CODEX_AGENT_MODEL").unwrap_or_else(|_| "gpt-5.5".to_string());
    normalize_codex_exec_command(command, Some(model.as_str()))
}

fn normalize_codex_exec_command(mut command: Vec<String>, model: Option<&str>) -> Vec<String> {
    let Some(program) = command.first() else {
        return command;
    };
    if Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .is_none_or(|name| name != "codex")
    {
        return command;
    }
    let Some(exec_index) = command.iter().position(|arg| arg == "exec") else {
        return command;
    };

    let mut additions = Vec::new();
    for flag in ["--ignore-user-config", "--ignore-rules", "--ephemeral"] {
        if !command.iter().any(|arg| arg == flag) {
            additions.push(flag.to_string());
        }
    }
    if !command
        .iter()
        .any(|arg| arg == "--model" || arg == "-m" || arg.starts_with("--model="))
        && let Some(model) = model.map(str::trim).filter(|model| !model.is_empty())
    {
        additions.push("--model".to_string());
        additions.push(model.to_string());
    }

    let insert_at = exec_index + 1;
    for (offset, arg) in additions.into_iter().enumerate() {
        command.insert(insert_at + offset, arg);
    }
    command
}

fn configure_child_process_group(command: &mut Command) {
    #[cfg(unix)]
    {
        // Put the local Codex executor in its own process group so timeouts can
        // terminate plugin fetches, MCP servers, and other grandchildren too.
        unsafe {
            command.pre_exec(|| {
                if libc::setpgid(0, 0) == 0 {
                    Ok(())
                } else {
                    Err(std::io::Error::last_os_error())
                }
            });
        }
    }
}

async fn terminate_process_group(pid: Option<u32>) {
    #[cfg(unix)]
    {
        let Some(pid) = pid.and_then(|pid| i32::try_from(pid).ok()) else {
            return;
        };
        let process_group = -pid;
        unsafe {
            libc::kill(process_group, libc::SIGTERM);
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
        unsafe {
            libc::kill(process_group, libc::SIGKILL);
        }
    }

    #[cfg(not(unix))]
    {
        let _ = pid;
    }
}

async fn write_prompt_file(prompt: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = env::temp_dir().join(format!("codex-agent-prompt-{}.md", now));
    tokio::fs::write(&path, prompt).await?;
    Ok(path)
}

fn agent_timeout() -> Duration {
    let raw = env::var("CODEX_AGENT_TIMEOUT_SECONDS").ok();
    agent_timeout_from(raw.as_deref())
}

fn agent_timeout_from(raw: Option<&str>) -> Duration {
    let seconds = raw
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(900);
    Duration::from_secs(seconds)
}

fn auto_fix_budget() -> Option<Duration> {
    let raw = env::var("CODEX_AUTO_FIX_BUDGET_SECONDS").ok();
    duration_from_env(raw.as_deref())
}

fn auto_fix_budget_reserve() -> Duration {
    let raw = env::var("CODEX_AUTO_FIX_RESERVE_SECONDS").ok();
    duration_from_env(raw.as_deref()).unwrap_or_else(|| Duration::from_secs(120))
}

fn duration_from_env(raw: Option<&str>) -> Option<Duration> {
    raw.and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .map(Duration::from_secs)
}

fn effective_command_timeout_from(
    agent_timeout: Duration,
    remaining_budget: Option<Duration>,
    reserve: Duration,
    minimum: Duration,
) -> Result<Duration, String> {
    let Some(remaining_budget) = remaining_budget else {
        return Ok(agent_timeout);
    };
    let available = remaining_budget
        .checked_sub(reserve)
        .unwrap_or_else(|| Duration::from_secs(0));
    if available < minimum {
        return Err(format!(
            "自动修复剩余时间不足（remaining={}s, reserve={}s, minimum={}s），跳过新的 Codex 调用以便返回 JSON",
            remaining_budget.as_secs(),
            reserve.as_secs(),
            minimum.as_secs()
        ));
    }
    Ok(agent_timeout.min(available))
}

fn reject_recursive_command(command: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let Some(program) = command.first() else {
        return Err("CODEX_AGENT_COMMAND 不能为空".into());
    };

    let current = env::current_exe()
        .ok()
        .and_then(|p| std::fs::canonicalize(p).ok());
    let target = resolve_program_path(program).and_then(|p| std::fs::canonicalize(p).ok());

    if current.is_some() && current == target {
        return Err(format!(
            "CODEX_AGENT_COMMAND 指向了当前 codex-cli 二进制，会导致递归调用: {}",
            program
        )
        .into());
    }

    Ok(())
}

fn resolve_program_path(program: &str) -> Option<PathBuf> {
    let path = Path::new(program);
    if path.components().count() > 1 || path.is_absolute() {
        return Some(path.to_path_buf());
    }

    let paths = env::var_os("PATH")?;
    env::split_paths(&paths)
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_command_rejects_empty_input() {
        assert!(parse_command("   ").is_err());
    }

    #[test]
    fn parse_command_splits_program_and_args() {
        let parsed = parse_command("codex exec --skip-git-repo-check -").unwrap();
        assert_eq!(parsed[0], "codex");
        assert_eq!(parsed[1], "exec");
        assert_eq!(parsed.last().map(String::as_str), Some("-"));
    }

    #[test]
    fn codex_exec_command_gets_noninteractive_safe_defaults() {
        let normalized = normalize_codex_exec_command(
            vec![
                "codex".to_string(),
                "exec".to_string(),
                "--skip-git-repo-check".to_string(),
                "-".to_string(),
            ],
            Some("gpt-5.5"),
        );

        assert_eq!(normalized[0], "codex");
        assert_eq!(normalized[1], "exec");
        assert!(normalized.contains(&"--ignore-user-config".to_string()));
        assert!(normalized.contains(&"--ignore-rules".to_string()));
        assert!(normalized.contains(&"--ephemeral".to_string()));
        assert!(
            normalized
                .windows(2)
                .any(|pair| pair == ["--model", "gpt-5.5"])
        );
        assert_eq!(normalized.last().map(String::as_str), Some("-"));
    }

    #[test]
    fn codex_exec_command_does_not_duplicate_existing_safe_defaults() {
        let normalized = normalize_codex_exec_command(
            vec![
                "codex".to_string(),
                "exec".to_string(),
                "--ignore-user-config".to_string(),
                "--ignore-rules".to_string(),
                "--ephemeral".to_string(),
                "--model".to_string(),
                "gpt-5.4".to_string(),
                "-".to_string(),
            ],
            Some("gpt-5.5"),
        );

        assert_eq!(
            normalized
                .iter()
                .filter(|arg| arg.as_str() == "--ignore-user-config")
                .count(),
            1
        );
        assert!(
            normalized
                .windows(2)
                .any(|pair| pair == ["--model", "gpt-5.4"])
        );
        assert!(
            !normalized
                .windows(2)
                .any(|pair| pair == ["--model", "gpt-5.5"])
        );
    }

    #[test]
    fn build_prompt_keeps_system_and_user_text() {
        let prompt = build_prompt("rules", "task");
        assert!(prompt.contains("rules"));
        assert!(prompt.contains("task"));
    }

    #[test]
    fn agent_timeout_defaults_to_fifteen_minutes() {
        assert_eq!(agent_timeout_from(None), Duration::from_secs(900));
    }

    #[test]
    fn agent_timeout_reads_positive_seconds() {
        assert_eq!(agent_timeout_from(Some("42")), Duration::from_secs(42));
    }

    #[test]
    fn agent_timeout_ignores_zero_and_invalid_values() {
        assert_eq!(agent_timeout_from(Some("0")), Duration::from_secs(900));
        assert_eq!(agent_timeout_from(Some("nope")), Duration::from_secs(900));
    }

    #[test]
    fn effective_timeout_leaves_budget_reserve() {
        assert_eq!(
            effective_command_timeout_from(
                Duration::from_secs(600),
                Some(Duration::from_secs(500)),
                Duration::from_secs(120),
                Duration::from_secs(30),
            )
            .unwrap(),
            Duration::from_secs(380)
        );
    }

    #[test]
    fn effective_timeout_fails_when_budget_is_too_low() {
        let err = effective_command_timeout_from(
            Duration::from_secs(600),
            Some(Duration::from_secs(130)),
            Duration::from_secs(120),
            Duration::from_secs(30),
        )
        .unwrap_err();
        assert!(err.contains("自动修复剩余时间不足"));
    }

    #[tokio::test]
    async fn command_timeout_kills_process_group_children() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = env::temp_dir().join(format!("codex-cli-process-group-timeout-{now}"));
        std::fs::create_dir_all(&dir).unwrap();
        let child_pid_path = dir.join("child.pid");
        let script_path = dir.join("spawn-child.sh");
        std::fs::write(
            &script_path,
            format!(
                "#!/bin/sh\nsleep 30 &\nprintf '%s' \"$!\" > '{}'\nwait\n",
                child_pid_path.display()
            ),
        )
        .unwrap();

        let result = run_local_codex_command(
            &["sh".to_string(), script_path.to_string_lossy().to_string()],
            "prompt",
            Duration::from_secs(1),
        )
        .await;

        assert!(result.is_err());
        let child_pid = std::fs::read_to_string(&child_pid_path)
            .unwrap()
            .parse::<i32>()
            .unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_ne!(
            unsafe { libc::kill(child_pid, 0) },
            0,
            "timeout must kill grandchildren spawned by the local Codex command"
        );

        let _ = std::fs::remove_dir_all(dir);
    }
}
