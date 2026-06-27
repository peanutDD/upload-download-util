use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn watchdog_plan_marks_found_review_clean() {
    let output = plan(&[("GEMINI_REVIEW_FOUND", "true")]);

    assert_eq!(output["action"], "found");
    assert_eq!(output["needs_human"], "false");
}

#[test]
fn watchdog_plan_marks_missing_review_as_nonblocking_by_default() {
    let output = plan(&[
        ("GEMINI_REVIEW_FOUND", "false"),
        ("GEMINI_REVIEW_TIMEOUT_SECONDS", "120"),
    ]);

    assert_eq!(output["action"], "timeout");
    assert_eq!(output["needs_human"], "false");
    assert_eq!(output["timeout_seconds"], "120");
}

#[test]
fn watchdog_plan_can_require_missing_review_as_human_block() {
    let output = plan(&[
        ("GEMINI_REVIEW_FOUND", "false"),
        ("GEMINI_REVIEW_REQUIRED", "true"),
    ]);

    assert_eq!(output["action"], "timeout");
    assert_eq!(output["needs_human"], "true");
}

#[test]
fn watchdog_timeout_comment_explains_no_new_issue_list() {
    let script =
        std::fs::read_to_string(workflow_script()).expect("watchdog script should be readable");

    assert!(
        script.contains("无法生成新的 Medium/Medium+/High/Critical 问题清单"),
        "quota or timeout comments must explain why no new issue list was generated"
    );
    assert!(
        script.contains("Gemini review timeout is non-blocking in relaxed mode"),
        "relaxed mode should document that Gemini timeout does not fail the kickoff check"
    );
}

fn plan(envs: &[(&str, &str)]) -> HashMap<String, String> {
    let script = workflow_script();
    let output = Command::new("bash")
        .arg(script)
        .arg("plan")
        .envs(envs.iter().copied())
        .output()
        .expect("watchdog script should run");

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
        .join(".github/scripts/gemini-review-watchdog.sh")
}
