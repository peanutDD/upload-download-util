use serde_json::Value;
use std::process::Command;

#[test]
fn doctor_json_reports_installation_freshness_and_runtime_checks() {
    let output = Command::new(env!("CARGO_BIN_EXE_codex-auto-fix"))
        .args(["doctor", "--json"])
        .env("CODEX_AGENT_COMMAND", "codex exec --skip-git-repo-check -")
        .output()
        .expect("doctor command should run");

    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("doctor stdout should be JSON");
    assert_eq!(json["package"], "codex-cli");
    assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
    assert!(json["current_exe"].as_str().is_some_and(|s| !s.is_empty()));
    assert!(json["manifest_dir"].as_str().is_some_and(|s| !s.is_empty()));

    let checks = json["checks"]
        .as_array()
        .expect("doctor should expose check list");
    let check_names = checks
        .iter()
        .filter_map(|check| check["name"].as_str())
        .collect::<Vec<_>>();
    assert!(check_names.contains(&"path.codex-auto-fix"));
    assert!(check_names.contains(&"source.freshness"));
    assert!(check_names.contains(&"agent.command"));
}

#[test]
fn doctor_warns_when_agent_command_recurses_into_auto_fix_binary() {
    let output = Command::new(env!("CARGO_BIN_EXE_codex-auto-fix"))
        .args(["doctor", "--json"])
        .env("CODEX_AGENT_COMMAND", "codex-auto-fix pr-auto-fix --yes")
        .output()
        .expect("doctor command should run");

    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("doctor stdout should be JSON");
    let agent_check = json["checks"]
        .as_array()
        .expect("doctor should expose checks")
        .iter()
        .find(|check| check["name"] == "agent.command")
        .expect("doctor should expose agent.command check");

    assert_eq!(agent_check["status"], "warning");
    assert!(
        agent_check["message"]
            .as_str()
            .is_some_and(|message| message.contains("recursively points to codex-auto-fix")),
        "message={agent_check:?}"
    );
}
