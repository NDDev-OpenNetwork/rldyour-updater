use crate::{config::Policy, model::ActionResult, process::CommandRunner};
use std::{path::Path, time::Duration};

pub fn run<R: CommandRunner>(policy: &Policy, runner: &R) -> ActionResult {
    let argv = &policy.gds.argv;
    let valid_shape = argv.iter().any(|arg| arg.ends_with("managed_cli.py"))
        && argv.iter().any(|arg| arg == "install")
        && argv.iter().any(|arg| arg == "--platform");
    if !valid_shape {
        return ActionResult {
            action: "gds-install".into(),
            ok: false,
            changed: false,
            observed: false,
            detail: "refused: policy is not a managed_cli.py install invocation".into(),
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
        };
    }
    let cwd = argv.iter().find_map(|arg| {
        Path::new(arg)
            .parent()
            .filter(|_| arg.ends_with("managed_cli.py"))
    });
    let output = runner.run(
        argv,
        cwd,
        Duration::from_secs(policy.command_timeout_seconds),
    );
    if !output.ok {
        return ActionResult {
            action: "gds-install".into(),
            ok: false,
            changed: false,
            observed: true,
            detail: "signed GDS managed CLI install failed".into(),
            stdout: output.stdout,
            stderr: output.stderr,
            exit_code: None,
        };
    }
    let script_index = argv.iter().position(|arg| arg.ends_with("managed_cli.py"));
    let companion = script_index.and_then(|index| {
        let script = Path::new(&argv[index]);
        let companion = script.parent()?.join("ai_launchers.py");
        companion.is_file().then(|| {
            let python = argv.first().cloned().unwrap_or_else(|| "python3".into());
            vec![python, companion.to_string_lossy().into_owned(), "install".into()]
        })
    });
    let (stdout, stderr, companion_ok) = if let Some(companion_argv) = companion {
        let companion_output = runner.run(
            &companion_argv,
            companion_argv
                .get(1)
                .and_then(|value| Path::new(value).parent()),
            Duration::from_secs(policy.command_timeout_seconds),
        );
        (
            format!("{}\n{}", output.stdout, companion_output.stdout),
            format!("{}\n{}", output.stderr, companion_output.stderr),
            companion_output.ok,
        )
    } else {
        (output.stdout, output.stderr, true)
    };
    ActionResult {
        action: "gds-install".into(),
        ok: companion_ok,
        changed: companion_ok,
        observed: true,
        detail: if companion_ok {
            "signed GDS managed CLI install and launcher refresh completed".into()
        } else {
            "GDS install completed but launcher refresh failed".into()
        },
        stdout,
        stderr,
        exit_code: None,
    }
}
