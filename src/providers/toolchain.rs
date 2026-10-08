use crate::{config::Policy, model::ActionResult, process::CommandRunner};
use std::{path::PathBuf, time::Duration};

pub fn run<R: CommandRunner>(policy: &Policy, runner: &R) -> ActionResult {
    let mut lines = Vec::new();
    for (name, program) in [
        ("vllm", user_binary("vllm")),
        ("ollama", PathBuf::from("/usr/bin/ollama")),
        ("nvcc", PathBuf::from("/usr/local/cuda/bin/nvcc")),
    ] {
        let argv = vec![program.to_string_lossy().into_owned(), "--version".into()];
        let output = runner.run(
            &argv,
            None,
            Duration::from_secs(policy.command_timeout_seconds.min(30)),
        );
        let detail = if output.ok {
            output.stdout.lines().next().unwrap_or("present").to_owned()
        } else {
            let reason = output.stderr.lines().next().unwrap_or("command failed");
            format!("unavailable or needs review ({reason})")
        };
        lines.push(format!("{name}: {detail}"));
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

fn user_binary(name: &str) -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(".local/bin")
        .join(name)
}
