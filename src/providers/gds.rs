use crate::{
    config::Policy, model::ActionResult, process::CommandRunner, release::VerifiedRelease,
};
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize)]
struct Receipt {
    schema: u8,
    components: Vec<Component>,
    vendor_configuration_mutated: bool,
}
#[derive(Deserialize)]
struct Component {
    id: String,
    changed: bool,
}

pub fn run<R: CommandRunner>(
    policy: &Policy,
    source: &VerifiedRelease,
    runner: &R,
) -> ActionResult {
    let root = source.root();
    let script = if policy.gds.applications {
        root.join("scripts/ubuntu/harness-apps.py")
    } else {
        root.join("scripts/managed_cli.py")
    };
    let mut argv = vec![
        policy.gds.python.to_string_lossy().into_owned(),
        "-I".into(),
        "-B".into(),
        script.to_string_lossy().into_owned(),
    ];
    let action = if policy.gds.applications {
        "verified-applications"
    } else {
        "gds-cli"
    };
    if policy.gds.applications {
        argv.extend(["update-verified".into(), "--json".into()]);
    } else {
        argv.extend([
            "install".into(),
            "--platform".into(),
            policy.gds.platform.clone(),
            "--json".into(),
        ]);
    }
    let output = runner.run(
        &argv,
        Some(root),
        Duration::from_secs(policy.command_timeout_seconds),
    );
    if !output.ok || output.truncated {
        let mut result = ActionResult::failed(
            action,
            "verified installer failed or output was incomplete; partial changes require review",
        );
        result.stdout = output.stdout;
        result.stderr = output.stderr;
        result.exit_code = output.exit_code;
        result.output_truncated = output.truncated;
        result.changed = None;
        return result;
    }
    let receipt: Receipt = match output
        .stdout
        .lines()
        .last()
        .and_then(|line| serde_json::from_str(line).ok())
    {
        Some(value) => value,
        None => {
            return ActionResult::failed(
                action,
                "installer did not produce a valid structured receipt",
            );
        }
    };
    let expected: std::collections::BTreeSet<_> = if policy.gds.applications {
        [
            "antigravity",
            "claude-code",
            "codex",
            "cursor",
            "grok-build",
            "opencode",
            "pi",
            "devin",
        ]
        .into_iter()
        .collect()
    } else {
        [
            "antigravity",
            "claude-code",
            "codex",
            "cursor",
            "grok-build",
            "opencode",
            "pi",
            "devin",
            "gddy",
        ]
        .into_iter()
        .collect()
    };
    let actual: std::collections::BTreeSet<_> =
        receipt.components.iter().map(|c| c.id.as_str()).collect();
    if receipt.schema != 1
        || receipt.vendor_configuration_mutated
        || actual != expected
        || receipt.components.len() != expected.len()
    {
        return ActionResult::failed(
            action,
            "installer receipt contradicts the signed component contract",
        );
    }
    let mut result = ActionResult::observed(
        action,
        "signed catalogue sources and component receipts verified",
    );
    result.changed = Some(receipt.components.iter().any(|c| c.changed));
    result.state = if result.changed == Some(true) {
        "updated".into()
    } else {
        "verified".into()
    };
    result.stdout = output.stdout;
    result.stderr = output.stderr;
    result.exit_code = output.exit_code;
    if !policy.gds.applications {
        let script = root.join("scripts/ai_launchers.py");
        let args = |verb: &str| {
            vec![
                policy.gds.python.to_string_lossy().into_owned(),
                "-I".into(),
                "-B".into(),
                script.to_string_lossy().into_owned(),
                verb.into(),
            ]
        };
        let verification = runner.run(
            &args("verify"),
            Some(root),
            Duration::from_secs(policy.command_timeout_seconds),
        );
        if !verification.ok {
            let install = runner.run(
                &args("install"),
                Some(root),
                Duration::from_secs(policy.command_timeout_seconds),
            );
            if !install.ok || install.truncated {
                result.ok = false;
                result.state = "failed".into();
                result.detail = "CLI verified, launcher refresh failed".into();
                result.stderr = install.stderr;
                result.exit_code = install.exit_code;
                result.changed = None;
            } else {
                result.changed = Some(true);
                result.state = "updated".into();
            }
        }
    }
    result
}
