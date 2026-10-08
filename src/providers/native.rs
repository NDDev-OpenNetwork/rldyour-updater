use crate::{config::Policy, model::ActionResult, process::CommandRunner};
use std::time::Duration;
const MANAGED: &[&str] = &[
    "antigravity",
    "claude",
    "claude-code",
    "codex",
    "cursor",
    "cursor-agent",
    "grok",
    "grok-build",
    "opencode",
    "pi",
    "devin",
    "herdr",
    "rldyour-cleaner",
    "rldyour-clipboard",
    "rldyour-sysinfo",
    "rldyour-updater",
];
pub fn run<R: CommandRunner>(policy: &Policy, runner: &R) -> Vec<ActionResult> {
    let mut results = vec![];
    if policy.native.homebrew {
        let root = if std::path::Path::new("/opt/homebrew/bin/brew").is_file() {
            "/opt/homebrew/bin/brew"
        } else {
            "/usr/local/bin/brew"
        };
        let timeout = Duration::from_secs(policy.command_timeout_seconds);
        let output = runner.run(&[root.into(), "update-if-needed".into()], None, timeout);
        if !output.ok {
            results.push(ActionResult::failed("homebrew-index", output.stderr));
            return results;
        }
        let outdated = runner.run(
            &[root.into(), "outdated".into(), "--json=v2".into()],
            None,
            timeout,
        );
        let value: serde_json::Value = match serde_json::from_str(&outdated.stdout) {
            Ok(value) if outdated.ok && !outdated.truncated => value,
            _ => {
                results.push(ActionResult::failed(
                    "homebrew",
                    "outdated inventory is incomplete or malformed",
                ));
                return results;
            }
        };
        for (field, kind) in [("formulae", "--formula"), ("casks", "--cask")] {
            let Some(entries) = value.get(field).and_then(|v| v.as_array()) else {
                results.push(ActionResult::failed(
                    "homebrew",
                    "native inventory schema is unknown",
                ));
                return results;
            };
            let mut names = vec![];
            for entry in entries {
                let Some(name) = entry.get("name").and_then(|n| n.as_str()) else {
                    results.push(ActionResult::failed(
                        "homebrew",
                        "package identity is unknown",
                    ));
                    return results;
                };
                if name.is_empty()
                    || name.starts_with('-')
                    || !name
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"+-_.@/".contains(&b))
                {
                    results.push(ActionResult::failed(
                        "homebrew",
                        "invalid native package name",
                    ));
                    return results;
                }
                if !MANAGED.contains(&name)
                    && entry.get("pinned").and_then(|v| v.as_bool()) != Some(true)
                {
                    names.push(name.to_string());
                }
            }
            let id = if field == "formulae" {
                "homebrew-formula"
            } else {
                "homebrew-apps"
            };
            if names.is_empty() {
                let mut result = ActionResult::observed(
                    id,
                    "no eligible updates; managed tools and pins preserved",
                );
                result.state = "verified".into();
                results.push(result);
                continue;
            }
            let mut args = vec![root.into(), "upgrade".into(), kind.into()];
            if kind == "--cask" {
                args.extend(["--no-quit".into(), "--require-sha".into()]);
            }
            args.extend(names);
            let output = runner.run(&args, None, timeout);
            let mut result = if output.ok {
                ActionResult::observed(id, "native update completed; no forced application quit")
            } else {
                ActionResult::failed(id, "native Homebrew update failed")
            };
            result.changed = None;
            result.state = if output.ok {
                "native-owner".into()
            } else {
                "failed".into()
            };
            result.stdout = output.stdout;
            result.stderr = output.stderr;
            result.exit_code = output.exit_code;
            result.output_truncated = output.truncated;
            results.push(result);
        }
    }
    if policy.native.flatpak {
        let output = runner.run(
            &[
                "/usr/bin/flatpak".into(),
                "update".into(),
                "--user".into(),
                "--noninteractive".into(),
            ],
            None,
            Duration::from_secs(policy.command_timeout_seconds),
        );
        let mut result = if output.ok {
            ActionResult::observed("flatpak", "native per-user Flatpak update completed")
        } else {
            ActionResult::failed("flatpak", "native Flatpak update failed")
        };
        result.changed = None;
        result.stdout = output.stdout;
        result.stderr = output.stderr;
        result.exit_code = output.exit_code;
        results.push(result);
    }
    results
}
