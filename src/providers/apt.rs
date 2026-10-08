use crate::{config::Policy, model::ActionResult, process::CommandRunner};
use std::time::Duration;
pub fn run<R: CommandRunner>(policy: &Policy, runner: &R) -> ActionResult {
    if !cfg!(target_os = "linux") {
        let mut result = ActionResult::observed("apt", "not applicable");
        result.state = "not-applicable".into();
        return result;
    }
    let output = runner.run(
        &[
            "/usr/bin/apt-get".into(),
            "-s".into(),
            "--with-new-pkgs".into(),
            "upgrade".into(),
        ],
        None,
        Duration::from_secs(policy.command_timeout_seconds.min(60)),
    );
    let mut result = if output.ok {
        ActionResult::observed(
            "apt",
            "read-only package plan; unattended-upgrades owns mutation",
        )
    } else {
        ActionResult::failed("apt", "native package simulation failed")
    };
    result.stdout = output.stdout;
    result.stderr = output.stderr;
    result.exit_code = output.exit_code;
    result.output_truncated = output.truncated;
    result
}
