use crate::{
    catalog::{self, UpdaterBinary},
    config::Policy,
    model::ActionResult,
    process::CommandRunner,
};
use std::{path::Path, time::Duration};
pub fn run<R: CommandRunner>(
    policy: &Policy,
    artifacts: &[UpdaterBinary],
    stage: &Path,
    runner: &R,
) -> ActionResult {
    match apply(policy, artifacts, stage, runner) {
        Ok(result) => result,
        Err(error) => ActionResult::failed("updater-binary", error),
    }
}
fn apply<R: CommandRunner>(
    _policy: &Policy,
    artifacts: &[UpdaterBinary],
    stage: &Path,
    runner: &R,
) -> Result<ActionResult, String> {
    let platform = if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "linux/x86_64"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "macos/arm64"
    } else {
        ""
    };
    let Some(artifact) = artifacts.iter().find(|a| a.platform == platform) else {
        let mut result =
            ActionResult::observed("updater-binary", "no approved binary for this platform");
        result.state = "not-applicable".into();
        return Ok(result);
    };
    let current = std::env::current_exe().map_err(|e| e.to_string())?;
    let expected = if crate::os::is_root() {
        std::path::PathBuf::from("/usr/local/bin/rldyour-updater")
    } else {
        crate::os::home_dir().join(".local/bin/rldyour-updater")
    };
    if current != expected {
        return Err("self update requires the managed installation path".into());
    }
    crate::storage::plain_path(&expected)?;
    let mut result =
        ActionResult::observed("updater-binary", "installed updater is current or newer");
    result.state = "verified".into();
    if catalog::version_tuple(&artifact.version)?
        <= catalog::version_tuple(env!("CARGO_PKG_VERSION"))?
    {
        return Ok(result);
    }
    let file = stage.join("rldyour-updater");
    crate::transport::download(runner, &artifact.url, &file, artifact.bytes, 120)?;
    let bytes = crate::storage::read_bounded(&file, 64 * 1024 * 1024)?;
    if bytes.len() as u64 != artifact.bytes || catalog::sha256(&bytes) != artifact.sha256 {
        return Err("updater binary differs from signed release".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    let probe = runner.run(
        &[file.to_string_lossy().into_owned(), "--version".into()],
        None,
        Duration::from_secs(15),
    );
    if !probe.ok
        || probe.truncated
        || probe.stdout.trim() != format!("rldyour-updater {}", artifact.version)
    {
        return Err("signed updater reports an unexpected version".into());
    }
    // Same-filesystem atomic publication. The current process keeps its old image;
    // no daemon/app/desktop process is restarted, and no backup file is made.
    let mut replacement =
        tempfile::NamedTempFile::new_in(expected.parent().ok_or("invalid binary destination")?)
            .map_err(|e| e.to_string())?;
    use std::io::Write;
    replacement.write_all(&bytes).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        replacement
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }
    replacement
        .as_file()
        .sync_all()
        .map_err(|e| e.to_string())?;
    replacement.persist(&expected).map_err(|e| e.to_string())?;
    result.changed = Some(true);
    result.state = "updated".into();
    result.detail = "signed updater installed atomically; next scheduled run uses it".into();
    Ok(result)
}
