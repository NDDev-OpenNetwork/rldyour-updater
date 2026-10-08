use crate::{config::Policy, model::ActionResult, process::CommandRunner};
pub fn run<R: CommandRunner>(policy: &Policy, runner: &R) -> ActionResult {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = policy;
        let _ = runner;
        ActionResult {
            action: "apt".into(),
            ok: true,
            changed: false,
            observed: true,
            detail: "APT is not applicable on this platform".into(),
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
        }
    }
    #[cfg(target_os = "linux")]
    {
        use std::time::Duration;
        let timeout = Duration::from_secs(policy.command_timeout_seconds);
        if policy.apt.apply {
            if !is_root() {
                return ActionResult {
                    action: "apt".into(),
                    ok: false,
                    changed: false,
                    observed: true,
                    detail: "apt.apply refused: updater is not running as root".into(),
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: None,
                };
            }
            let update = runner.run(&["apt-get".into(), "update".into()], None, timeout);
            if !update.ok {
                return result("apt", false, "apt-get update failed", update);
            }
            let upgrade = runner.run(
                &[
                    "apt-get".into(),
                    "-y".into(),
                    "--with-new-pkgs".into(),
                    "upgrade".into(),
                ],
                None,
                timeout,
            );
            return result(
                "apt",
                upgrade.ok,
                if upgrade.ok {
                    "APT upgrade completed; reboot state remains observed-only"
                } else {
                    "APT upgrade failed"
                },
                upgrade,
            );
        }
        let simulation = runner.run(
            &[
                "apt-get".into(),
                "-s".into(),
                "--with-new-pkgs".into(),
                "upgrade".into(),
            ],
            None,
            timeout,
        );
        let detail = if simulation.ok {
            "APT upgrade plan observed; apt-daily/unattended-upgrades remains the mutation owner"
        } else {
            "APT simulation failed"
        };
        result("apt", simulation.ok, detail, simulation)
    }
}

#[cfg(target_os = "linux")]
fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}
#[cfg(target_os = "linux")]
fn result(
    action: &str,
    ok: bool,
    detail: &str,
    output: crate::process::CommandOutput,
) -> ActionResult {
    ActionResult {
        action: action.into(),
        ok,
        changed: false,
        observed: true,
        detail: detail.into(),
        stdout: output.stdout,
        stderr: output.stderr,
        exit_code: None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn policy_default_never_mutates_apt() {
        assert!(!crate::config::Policy::defaults().apt.apply);
    }
}
