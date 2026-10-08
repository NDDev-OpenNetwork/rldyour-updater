use crate::{config::Policy, model::ActionResult, process::CommandRunner};
use std::time::Duration;

pub fn run<R: CommandRunner>(policy: &Policy, runner: &R) -> ActionResult {
    let mut lines = Vec::new();
    for (name, argv) in [
        ("vllm", vec!["vllm".into(), "--version".into()]),
        ("ollama", vec!["ollama".into(), "--version".into()]),
        ("nvcc", vec!["nvcc".into(), "--version".into()]),
    ] {
        let output = runner.run(
            &argv,
            None,
            Duration::from_secs(policy.command_timeout_seconds.min(30)),
        );
        lines.push(format!(
            "{name}: {}",
            if output.ok {
                output.stdout.lines().next().unwrap_or("present")
            } else {
                "unavailable or needs review"
            }
        ));
    }
    ActionResult {
        action: "toolchain-observe".into(),
        ok: true,
        changed: false,
        observed: true,
        detail: lines.join("; "),
        stdout: String::new(),
        stderr: String::new(),
        exit_code: None,
    }
}
