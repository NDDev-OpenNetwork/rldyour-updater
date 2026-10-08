#![deny(unsafe_op_in_unsafe_fn)]
pub mod catalog;
pub mod config;
pub mod lockfile;
pub mod model;
pub mod os;
pub mod paths;
pub mod process;
pub mod providers;
pub mod release;
pub mod report;
pub mod storage;
pub mod transport;

use config::Policy;
use model::{Action, ActionResult, Plan, RunReport};
use process::{CommandRunner, RealCommandRunner};
use std::path::Path;

pub fn plan(policy: &Policy) -> Plan {
    let mut actions = vec![];
    if policy.gds.enabled {
        actions.push(Action::GdsInstall {
            platform: policy.gds.platform.clone(),
        });
    }
    if policy.gds.applications {
        actions.push(Action::VerifiedApplications);
    }
    if policy.apt.observe {
        actions.push(Action::AptObserve);
    }
    if policy.native.homebrew {
        actions.push(Action::Homebrew);
    }
    if policy.native.flatpak {
        actions.push(Action::Flatpak);
    }
    if policy.toolchains.observe_only {
        actions.push(Action::ToolchainObserve);
    }
    Plan {
        schema: 2,
        channel: policy.channel.clone(),
        actions,
    }
}
pub fn apply(policy: &Policy, state: &Path) -> Result<RunReport, String> {
    policy.validate()?;
    storage::state_file(state, 4 * 1024 * 1024)?;
    let _lock = lockfile::RunLock::acquire(&policy.state_dir())?;
    apply_with_runner(policy, state, &RealCommandRunner)
}
pub fn apply_with_runner<R: CommandRunner>(
    policy: &Policy,
    state: &Path,
    runner: &R,
) -> Result<RunReport, String> {
    let started_at = model::unix_seconds();
    let mut report = RunReport {
        schema: 2,
        started_at,
        finished_at: started_at,
        channel: policy.channel.clone(),
        results: vec![],
        catalog_sequence: None,
        bootstrap_commit: None,
    };
    if policy.gds.enabled || policy.gds.applications {
        match release::fetch(policy, runner) {
            Ok(source) => {
                report.catalog_sequence = Some(source.payload.sequence);
                report.bootstrap_commit = Some(source.payload.bootstrap_commit.clone());
                report
                    .results
                    .push(providers::gds::run(policy, &source, runner));
                if policy.gds.applications {
                    report.results.extend(providers::deb::run(
                        policy,
                        &source.payload.debs,
                        source.root(),
                        runner,
                    ));
                }
                report.results.push(providers::self_update::run(
                    policy,
                    &source.payload.updater_binaries,
                    source.root(),
                    runner,
                ));
            }
            Err(error) => report
                .results
                .push(ActionResult::failed("signed-catalog", error)),
        }
    }
    // Independent native owners remain useful when the signed feed is unavailable.
    if policy.apt.observe {
        report.results.push(providers::apt::run(policy, runner));
    }
    report
        .results
        .extend(providers::native::run(policy, runner));
    if policy.toolchains.observe_only {
        report
            .results
            .push(providers::toolchain::run(policy, runner));
    }
    report.finished_at = model::unix_seconds();
    report::write_atomic(&report, state)?;
    Ok(report)
}
pub fn status(path: &Path) -> Result<RunReport, String> {
    report::read(path)
}
