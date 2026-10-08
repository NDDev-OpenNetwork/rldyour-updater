#![deny(unsafe_op_in_unsafe_fn)]

pub mod config;
pub mod lockfile;
pub mod model;
pub mod os;
pub mod paths;
pub mod process;
pub mod providers;
pub mod report;

use config::Policy;
use lockfile::RunLock;
use model::{Action, Plan, RunReport};
use process::{CommandRunner, RealCommandRunner};
use std::path::Path;

pub fn plan(policy: &Policy) -> Plan {
    let mut actions = Vec::new();
    if policy.gds.enabled {
        actions.push(Action::GdsInstall {
            platform: policy.gds.platform.clone(),
        });
    }
    if policy.apt.observe {
        actions.push(Action::AptObserve);
    }
    if policy.toolchains.observe_only {
        actions.push(Action::ToolchainObserve);
    }
    Plan {
        schema: 1,
        channel: policy.channel.clone(),
        actions,
    }
}

pub fn apply(policy: &Policy, state_path: &Path) -> Result<RunReport, String> {
    let _lock = RunLock::acquire(&policy.state_dir())?;
    apply_with_runner(policy, state_path, &RealCommandRunner)
}
pub fn apply_with_runner<R: CommandRunner>(
    policy: &Policy,
    state_path: &Path,
    runner: &R,
) -> Result<RunReport, String> {
    let started_at = model::unix_seconds();
    let mut results = Vec::new();
    if policy.gds.enabled {
        results.push(providers::gds::run(policy, runner));
    }
    if policy.apt.observe {
        results.push(providers::apt::run(policy, runner));
    }
    if policy.toolchains.observe_only {
        results.push(providers::toolchain::run(policy, runner));
    }
    let report = RunReport {
        schema: 1,
        started_at,
        finished_at: model::unix_seconds(),
        channel: policy.channel.clone(),
        results,
    };
    report::write_atomic(&report, state_path)?;
    Ok(report)
}

pub fn status(path: &Path) -> Result<RunReport, String> {
    report::read(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::Path, time::Duration};
    struct Fake;
    impl CommandRunner for Fake {
        fn run(
            &self,
            _argv: &[String],
            _cwd: Option<&Path>,
            _timeout: Duration,
        ) -> process::CommandOutput {
            process::CommandOutput {
                ok: true,
                stdout: "ok".into(),
                stderr: String::new(),
                truncated: false,
            }
        }
    }
    fn policy() -> Policy {
        toml::from_str(
            r#"schema=1
channel="signed-gds"
[gds]
enabled=true
platform="ubuntu"
argv=["/usr/bin/python3","/srv/estate/scripts/managed_cli.py","install","--platform","ubuntu"]
[apt]
observe=true
apply=false
[toolchains]
observe_only=false
"#,
        )
        .unwrap()
    }
    #[test]
    fn apply_writes_one_report_and_runs_only_declared_provider() {
        let dir = std::env::temp_dir().join(format!("rldyour-updater-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let report = apply_with_runner(&policy(), &dir.join("last-run.json"), &Fake).unwrap();
        assert_eq!(report.results.len(), 2);
        assert!(dir.join("last-run.json").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
