use serde::Serialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub package: &'static str,
    pub version: &'static str,
    pub status: CheckStatus,
    pub current_exe: String,
    pub manifest_dir: String,
    pub reinstall_hint: String,
    pub checks: Vec<DoctorCheck>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Ok,
    Warning,
}

#[derive(Debug, Serialize)]
pub struct DoctorCheck {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

pub fn build_report() -> DoctorReport {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let current_exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("unknown"));
    let mut checks = vec![
        path_check("codex-auto-fix", &current_exe),
        path_check("codex", &current_exe),
        freshness_check(&manifest_dir, &current_exe),
        agent_command_check(),
    ];

    for dep in ["git", "gh", "jq", "cargo"] {
        checks.push(dependency_check(dep));
    }

    let status = if checks
        .iter()
        .any(|check| check.status == CheckStatus::Warning)
    {
        CheckStatus::Warning
    } else {
        CheckStatus::Ok
    };

    DoctorReport {
        package: "codex-cli",
        version: env!("CARGO_PKG_VERSION"),
        status,
        current_exe: current_exe.display().to_string(),
        manifest_dir: manifest_dir.display().to_string(),
        reinstall_hint: format!("cargo install --path {} --force", manifest_dir.display()),
        checks,
    }
}

pub fn render_human(report: &DoctorReport) -> String {
    let mut lines = vec![
        format!("codex-cli doctor: {:?}", report.status).to_lowercase(),
        format!("version: {}", report.version),
        format!("current_exe: {}", report.current_exe),
        format!("manifest_dir: {}", report.manifest_dir),
        String::new(),
        "checks:".to_string(),
    ];

    for check in &report.checks {
        let marker = match check.status {
            CheckStatus::Ok => "ok",
            CheckStatus::Warning => "warn",
        };
        if let Some(path) = &check.path {
            lines.push(format!(
                "- [{}] {}: {} ({})",
                marker, check.name, check.message, path
            ));
        } else {
            lines.push(format!("- [{}] {}: {}", marker, check.name, check.message));
        }
    }

    if report.status == CheckStatus::Warning {
        lines.push(String::new());
        lines.push(format!("update hint: {}", report.reinstall_hint));
    }

    lines.join("\n")
}

fn path_check(command: &str, current_exe: &Path) -> DoctorCheck {
    match find_on_path(command) {
        Some(path) => {
            let should_match_current = current_exe
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == command);
            let same_binary = !should_match_current || paths_equivalent(&path, current_exe);
            DoctorCheck {
                name: format!("path.{command}"),
                status: if same_binary {
                    CheckStatus::Ok
                } else {
                    CheckStatus::Warning
                },
                message: if !should_match_current {
                    "command found on PATH".to_string()
                } else if same_binary {
                    "PATH resolves to the running binary".to_string()
                } else {
                    "PATH resolves to a different binary than the one currently running".to_string()
                },
                path: Some(path.display().to_string()),
            }
        }
        None => DoctorCheck {
            name: format!("path.{command}"),
            status: CheckStatus::Warning,
            message: "command was not found on PATH".to_string(),
            path: None,
        },
    }
}

fn freshness_check(manifest_dir: &Path, current_exe: &Path) -> DoctorCheck {
    let binary_mtime = fs::metadata(current_exe)
        .and_then(|metadata| metadata.modified())
        .ok();
    let source_mtime = newest_source_mtime(manifest_dir);

    match (binary_mtime, source_mtime) {
        (Some(binary), Some(source)) if binary >= source => DoctorCheck {
            name: "source.freshness".to_string(),
            status: CheckStatus::Ok,
            message: "running binary is newer than tracked codex-cli source files".to_string(),
            path: Some(current_exe.display().to_string()),
        },
        (Some(binary), Some(source)) => DoctorCheck {
            name: "source.freshness".to_string(),
            status: CheckStatus::Warning,
            message: format!(
                "running binary appears older than source files (binary={}, source={})",
                unix_seconds(binary),
                unix_seconds(source)
            ),
            path: Some(current_exe.display().to_string()),
        },
        _ => DoctorCheck {
            name: "source.freshness".to_string(),
            status: CheckStatus::Warning,
            message: "could not compare binary and source modification times".to_string(),
            path: Some(current_exe.display().to_string()),
        },
    }
}

fn agent_command_check() -> DoctorCheck {
    match env::var("CODEX_AGENT_COMMAND") {
        Ok(value) if !value.trim().is_empty() => {
            if agent_command_points_to_auto_fix(&value) {
                DoctorCheck {
                    name: "agent.command".to_string(),
                    status: CheckStatus::Warning,
                    message:
                        "CODEX_AGENT_COMMAND recursively points to codex-auto-fix; use the real Codex CLI executor"
                            .to_string(),
                    path: None,
                }
            } else {
                DoctorCheck {
                    name: "agent.command".to_string(),
                    status: CheckStatus::Ok,
                    message: "CODEX_AGENT_COMMAND is configured".to_string(),
                    path: None,
                }
            }
        }
        _ => DoctorCheck {
            name: "agent.command".to_string(),
            status: CheckStatus::Warning,
            message: "CODEX_AGENT_COMMAND is not configured; auto-fix model calls will fail"
                .to_string(),
            path: None,
        },
    }
}

fn agent_command_points_to_auto_fix(command: &str) -> bool {
    command
        .split_whitespace()
        .map(|part| part.trim_matches(|ch| ch == '"' || ch == '\''))
        .any(|part| {
            part == "codex-auto-fix"
                || Path::new(part)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name == "codex-auto-fix")
        })
        || command.contains("--bin codex-auto-fix")
}

fn dependency_check(command: &str) -> DoctorCheck {
    match find_on_path(command) {
        Some(path) => DoctorCheck {
            name: format!("dependency.{command}"),
            status: CheckStatus::Ok,
            message: "dependency found on PATH".to_string(),
            path: Some(path.display().to_string()),
        },
        None => DoctorCheck {
            name: format!("dependency.{command}"),
            status: CheckStatus::Warning,
            message: "dependency was not found on PATH".to_string(),
            path: None,
        },
    }
}

fn newest_source_mtime(manifest_dir: &Path) -> Option<SystemTime> {
    let mut newest = fs::metadata(manifest_dir.join("Cargo.toml"))
        .and_then(|metadata| metadata.modified())
        .ok();
    if let Ok(time) = fs::metadata(manifest_dir.join("Cargo.lock")).and_then(|m| m.modified()) {
        newest = Some(newest.map_or(time, |current| current.max(time)));
    }
    newest_dir_mtime(&manifest_dir.join("src"), &mut newest);
    newest
}

fn newest_dir_mtime(path: &Path, newest: &mut Option<SystemTime>) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            newest_dir_mtime(&path, newest);
        } else if let Ok(time) = metadata.modified() {
            *newest = Some(newest.map_or(time, |current| current.max(time)));
        }
    }
}

fn find_on_path(command: &str) -> Option<PathBuf> {
    if command.contains(std::path::MAIN_SEPARATOR) {
        let path = PathBuf::from(command);
        return path.is_file().then_some(path);
    }

    let paths = env::var_os("PATH")?;
    for dir in env::split_paths(&paths) {
        let candidate = dir.join(command);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

fn paths_equivalent(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(left), Ok(right)) => left == right,
        _ => a == b,
    }
}

fn unix_seconds(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_render_includes_update_hint_when_any_check_warns() {
        let report = DoctorReport {
            package: "codex-cli",
            version: "0.1.0",
            status: CheckStatus::Warning,
            current_exe: "/tmp/codex-auto-fix".to_string(),
            manifest_dir: "/tmp/codex-cli".to_string(),
            reinstall_hint: "cargo install --path /tmp/codex-cli --force".to_string(),
            checks: vec![DoctorCheck {
                name: "agent.command".to_string(),
                status: CheckStatus::Warning,
                message: "CODEX_AGENT_COMMAND is not configured".to_string(),
                path: None,
            }],
        };

        let rendered = render_human(&report);

        assert!(rendered.contains("codex-cli doctor: warning"));
        assert!(rendered.contains("update hint: cargo install --path /tmp/codex-cli --force"));
    }

    #[test]
    fn path_check_treats_other_found_tools_as_ok() {
        let current_exe = env::current_exe().expect("test binary path should be available");

        let check = path_check("cargo", &current_exe);

        assert_eq!(check.status, CheckStatus::Ok);
    }
}
