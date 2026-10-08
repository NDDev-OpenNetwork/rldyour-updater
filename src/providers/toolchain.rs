use crate::{config::Policy, model::ActionResult, process::CommandRunner};
use std::{path::PathBuf, time::Duration};

pub fn run<R: CommandRunner>(policy: &Policy, runner: &R) -> ActionResult {
    let mut lines = Vec::new();
    let mut failed = false;
    for (name, program) in [
        ("vllm", user_binary("vllm")),
        ("ollama", ollama_binary()),
        ("nvcc", nvcc_binary()),
    ] {
        if !program.exists() {
            lines.push(format!("{name}: not-installed"));
            continue;
        }
        let argv = vec![program.to_string_lossy().into_owned(), "--version".into()];
        let output = runner.run(
            &argv,
            None,
            Duration::from_secs(policy.command_timeout_seconds.min(30)),
        );
        let detail = if output.ok {
            output.stdout.lines().next().unwrap_or("present").to_owned()
        } else {
            failed = true;
            let reason = output.stderr.lines().next().unwrap_or("command failed");
            format!("unavailable or needs review ({reason})")
        };
        lines.push(format!("{name}: {detail}"));
    }
    if failed {
        ActionResult::failed("toolchain-observe", lines.join("; "))
    } else {
        ActionResult::observed("toolchain-observe", lines.join("; "))
    }
}

fn user_binary(name: &str) -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(".local/bin")
        .join(name)
}

fn ollama_binary() -> PathBuf {
    if cfg!(target_os = "macos") {
        PathBuf::from("/opt/homebrew/bin/ollama")
    } else {
        PathBuf::from("/usr/bin/ollama")
    }
}

fn nvcc_binary() -> PathBuf {
    if cfg!(target_os = "macos") {
        PathBuf::from("/opt/homebrew/bin/nvcc")
    } else {
        PathBuf::from("/usr/local/cuda/bin/nvcc")
    }
}
