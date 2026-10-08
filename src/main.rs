use clap::{Parser, Subcommand};
use rldyour_updater::{config::Policy, model::RunReport, paths, plan};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "rldyour-updater",
    version,
    about = "Signed-release and native package update coordinator"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[arg(long, global = true)]
    json: bool,
}
#[derive(Subcommand)]
enum Command {
    Plan,
    Apply,
    Status,
    Doctor,
    Configure {
        #[arg(long)]
        release_url: String,
        #[arg(long)]
        public_key: String,
        #[arg(long)]
        python: PathBuf,
        #[arg(long)]
        system_apps: bool,
        #[arg(long)]
        replace_policy: bool,
    },
    CatalogBuild {
        #[arg(long)]
        bootstrap: PathBuf,
        #[arg(long)]
        sequence: u64,
        #[arg(long, default_value_t = 30)]
        days: u64,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        debs: Option<PathBuf>,
        #[arg(long)]
        updater_binaries: Option<PathBuf>,
    },
    CatalogSign {
        #[arg(long)]
        payload: PathBuf,
        #[arg(long)]
        private_key: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    CatalogVerify {
        #[arg(long)]
        catalog: PathBuf,
        #[arg(long)]
        public_key: String,
    },
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let config = cli.config.unwrap_or(paths::config_path()?);
    match cli.command {
        Command::Configure {
            release_url,
            public_key,
            python,
            system_apps,
            replace_policy,
        } => {
            if config.exists() && !replace_policy {
                return Err("existing policy preserved; use --replace-policy after review".into());
            }
            let mut policy = Policy::defaults();
            policy.gds.enabled = !system_apps;
            policy.gds.applications = system_apps;
            policy.gds.python = python;
            policy.release.url = release_url;
            policy.release.public_key = public_key;
            policy.toolchains.observe_only = cfg!(target_os = "linux") && !system_apps;
            policy.native.homebrew = cfg!(target_os = "macos") && !system_apps;
            if system_apps {
                policy.state_dir = Some("/var/lib/rldyour-updater".into());
            }
            policy.validate()?;
            rldyour_updater::storage::replace(
                &config,
                &toml::to_string_pretty(&policy)
                    .map_err(|e| e.to_string())?
                    .into_bytes(),
            )?;
            println!("policy configured: {}", config.display());
        }
        Command::CatalogBuild {
            bootstrap,
            sequence,
            days,
            output,
            debs,
            updater_binaries,
        } => {
            let mut payload = rldyour_updater::release::build(&bootstrap, sequence, days)?;
            if let Some(path) = debs {
                payload.debs = serde_json::from_slice(&rldyour_updater::storage::read_bounded(
                    &path,
                    64 * 1024,
                )?)
                .map_err(|e| e.to_string())?;
                payload.validate(rldyour_updater::model::unix_seconds())?;
            }
            if let Some(path) = updater_binaries {
                payload.updater_binaries = serde_json::from_slice(
                    &rldyour_updater::storage::read_bounded(&path, 64 * 1024)?,
                )
                .map_err(|e| e.to_string())?;
                payload.validate(rldyour_updater::model::unix_seconds())?;
            }
            rldyour_updater::storage::replace(
                &output,
                &serde_json::to_vec_pretty(&payload).map_err(|e| e.to_string())?,
            )?;
            println!("catalogue payload built from {}", payload.bootstrap_commit);
        }
        Command::CatalogSign {
            payload,
            private_key,
            output,
        } => {
            let payload = serde_json::from_slice(&rldyour_updater::storage::read_bounded(
                &payload,
                rldyour_updater::catalog::LIMIT,
            )?)
            .map_err(|e| e.to_string())?;
            let (envelope, key) = rldyour_updater::catalog::sign(payload, &private_key)?;
            rldyour_updater::storage::replace(
                &output,
                &serde_json::to_vec_pretty(&envelope).map_err(|e| e.to_string())?,
            )?;
            println!("catalogue signed; public_key={key}");
        }
        Command::CatalogVerify {
            catalog,
            public_key,
        } => {
            let payload = rldyour_updater::catalog::verify(
                &rldyour_updater::storage::read_bounded(&catalog, rldyour_updater::catalog::LIMIT)?,
                &public_key,
                &Default::default(),
                rldyour_updater::model::unix_seconds(),
            )?;
            println!(
                "catalogue verified: sequence={}, source={}",
                payload.sequence, payload.bootstrap_commit
            );
        }
        Command::Plan => {
            let policy = Policy::from_path(&config)?;
            let value = plan(&policy);
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?
                );
            } else {
                println!("channel: {}\nactions: {:?}", value.channel, value.actions);
            }
        }
        Command::Apply => {
            let policy = Policy::from_path(&config)?;
            let state = rldyour_updater::release::report_path(&policy);
            let report = rldyour_updater::apply(&policy, &state)?;
            print_report(&report, cli.json)?;
            if report.results.iter().any(|result| !result.ok) {
                return Err("one or more update providers failed".into());
            }
        }
        Command::Status => {
            let state = if config.exists() {
                rldyour_updater::release::report_path(&Policy::from_path(&config)?)
            } else {
                paths::state_path()?
            };
            let report = rldyour_updater::status(&state)?;
            print_report(&report, cli.json)?;
        }
        Command::Doctor => {
            let policy = Policy::from_path(&config)?;
            policy.validate()?;
            println!(
                "policy: valid\nstate: {}\nScope: updater policy only; no general system diagnosis",
                policy.state_dir().display()
            );
        }
    }
    Ok(())
}
fn print_report(report: &RunReport, json: bool) -> Result<(), String> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(report).map_err(|e| e.to_string())?
        );
    } else {
        for result in &report.results {
            println!(
                "{}: {} — {}",
                result.action,
                if result.ok { "ok" } else { "failed" },
                result.detail
            );
        }
    }
    Ok(())
}
