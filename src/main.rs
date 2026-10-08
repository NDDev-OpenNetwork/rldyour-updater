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
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let config = cli.config.unwrap_or(paths::config_path()?);
    let state = paths::state_path()?;
    match cli.command {
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
            let report = rldyour_updater::apply(&policy, &state)?;
            print_report(&report, cli.json)?;
            if report.results.iter().any(|result| !result.ok) {
                return Err("one or more update providers failed".into());
            }
        }
        Command::Status => {
            let report = rldyour_updater::status(&state)?;
            print_report(&report, cli.json)?;
        }
        Command::Doctor => {
            let policy = Policy::from_path(&config)?;
            policy.validate()?;
            println!("policy: valid\nstate: {}", state.display());
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
