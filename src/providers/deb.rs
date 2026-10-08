use crate::{catalog::DebArtifact, config::Policy, model::ActionResult, process::CommandRunner};
use std::{path::Path, time::Duration};
pub fn run<R: CommandRunner>(
    policy: &Policy,
    artifacts: &[DebArtifact],
    stage: &Path,
    runner: &R,
) -> Vec<ActionResult> {
    artifacts
        .iter()
        .map(|artifact| match update(policy, artifact, stage, runner) {
            Ok(result) => result,
            Err(error) => ActionResult::failed(&format!("package:{}", artifact.package), error),
        })
        .collect()
}
fn update<R: CommandRunner>(
    policy: &Policy,
    artifact: &DebArtifact,
    stage: &Path,
    runner: &R,
) -> Result<ActionResult, String> {
    if !crate::os::is_root() {
        return Err("verified Debian packages require a root-owned policy".into());
    }
    let timeout = Duration::from_secs(policy.command_timeout_seconds);
    let query = runner.run(
        &[
            "/usr/bin/dpkg-query".into(),
            "-W".into(),
            "-f=${Status}\\n${Version}".into(),
            artifact.package.clone(),
        ],
        None,
        Duration::from_secs(10),
    );
    let mut result = ActionResult::observed(
        &format!("package:{}", artifact.package),
        "package is absent; no new installation requested",
    );
    result.state = "not-installed".into();
    if query.exit_code == Some(1) {
        return Ok(result);
    }
    if !query.ok || query.truncated {
        return Err("native installed-package query failed or was truncated".into());
    }
    if !query.stdout.starts_with("install ok installed\n") {
        result.state = "deferred".into();
        result.detail = "package is not fully configured; native recovery required".into();
        return Ok(result);
    }
    let current = query
        .stdout
        .lines()
        .nth(1)
        .ok_or("missing native package version")?;
    let comparison = runner.run(
        &[
            "/usr/bin/dpkg".into(),
            "--compare-versions".into(),
            artifact.version.clone(),
            "gt".into(),
            current.into(),
        ],
        None,
        Duration::from_secs(10),
    );
    if comparison.exit_code == Some(1) {
        result.state = "verified".into();
        result.detail = "installed package is current or newer; no downgrade".into();
        return Ok(result);
    }
    if !comparison.ok {
        return Err("native version comparison failed".into());
    }
    let file = stage.join(format!("{}.deb", artifact.package));
    crate::transport::download(
        runner,
        &artifact.url,
        &file,
        artifact.bytes,
        policy.command_timeout_seconds,
    )?;
    // Stream the large payload hash instead of retaining package bytes in memory.
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut hash = Sha256::new();
    let mut input = std::fs::File::open(&file).map_err(|e| e.to_string())?;
    let mut buffer = [0u8; 65536];
    loop {
        let n = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    if file.metadata().map_err(|e| e.to_string())?.len() != artifact.bytes
        || crate::catalog::encode_hex(&hash.finalize()) != artifact.sha256
    {
        return Err("package artifact integrity mismatch".into());
    }
    let metadata = runner.run(
        &[
            "/usr/bin/dpkg-deb".into(),
            "--field".into(),
            file.to_string_lossy().into_owned(),
            "Package".into(),
            "Version".into(),
            "Architecture".into(),
        ],
        None,
        Duration::from_secs(20),
    );
    let fields: std::collections::BTreeMap<_, _> = metadata
        .stdout
        .lines()
        .filter_map(|line| line.split_once(": "))
        .collect();
    if !metadata.ok
        || metadata.truncated
        || fields.get("Package") != Some(&artifact.package.as_str())
        || fields.get("Version") != Some(&artifact.version.as_str())
        || !matches!(fields.get("Architecture"), Some(&"amd64") | Some(&"all"))
    {
        return Err("package metadata contradicts signed identity".into());
    }
    let output = runner.run(
        &[
            "/usr/bin/env".into(),
            "DEBIAN_FRONTEND=noninteractive".into(),
            "/usr/bin/apt-get".into(),
            "-o".into(),
            "DPkg::Lock::Timeout=30".into(),
            "--no-remove".into(),
            "install".into(),
            "-y".into(),
            file.to_string_lossy().into_owned(),
        ],
        None,
        timeout,
    );
    if !output.ok {
        return Err(format!(
            "package transaction failed; review native apt journal: {}",
            output.stderr
        ));
    }
    let verification = runner.run(
        &[
            "/usr/bin/dpkg-query".into(),
            "-W".into(),
            "-f=${Version}".into(),
            artifact.package.clone(),
        ],
        None,
        Duration::from_secs(10),
    );
    if !verification.ok || verification.stdout.trim() != artifact.version {
        return Err("installed package differs from approved version".into());
    }
    result.state = "updated".into();
    result.changed = Some(true);
    result.detail = "verified package update completed".into();
    result.exit_code = output.exit_code;
    Ok(result)
}
