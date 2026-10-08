use crate::{
    catalog::{self, Acceptance, Payload},
    config::Policy,
    process::CommandRunner,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
#[derive(Debug)]
pub struct VerifiedRelease {
    pub payload: Payload,
    directory: tempfile::TempDir,
}
impl VerifiedRelease {
    pub fn root(&self) -> &Path {
        self.directory.path()
    }
}

pub fn fetch<R: CommandRunner>(policy: &Policy, runner: &R) -> Result<VerifiedRelease, String> {
    let state = policy.state_dir();
    crate::storage::private_dir(&state)?;
    let accepted = state.join("accepted-catalog.json");
    let acceptance: Acceptance = match accepted.try_exists().map_err(|e| e.to_string())? {
        true => serde_json::from_slice(&crate::storage::read_bounded(&accepted, 16 * 1024)?)
            .map_err(|e| format!("invalid acceptance state: {e}"))?,
        false => Acceptance::default(),
    };
    let stage = tempfile::tempdir_in(&state).map_err(|e| e.to_string())?;
    let feed = stage.path().join("catalog.json");
    crate::transport::download(
        runner,
        &policy.release.url,
        &feed,
        catalog::LIMIT,
        policy.command_timeout_seconds.min(60),
    )?;
    let now = crate::model::unix_seconds();
    let payload = catalog::verify(
        &crate::storage::read_bounded(&feed, catalog::LIMIT)?,
        &policy.release.public_key,
        &acceptance,
        now,
    )?;
    // Accept before execution: a crash or failed install cannot authorize replay
    // of an older catalogue. Equal sequence+digest remains retryable.
    let current = Acceptance {
        sequence: payload.sequence,
        digest: payload.digest()?,
        observed_at: now,
    };
    crate::storage::replace(
        &accepted,
        &serde_json::to_vec(&current).map_err(|e| e.to_string())?,
    )?;
    for entry in &payload.files {
        let path = stage.path().join(&entry.path);
        fs::create_dir_all(path.parent().ok_or("invalid source path")?)
            .map_err(|e| e.to_string())?;
        let url = format!(
            "https://raw.githubusercontent.com/NDDev-OpenNetwork/macos-ubuntu-bootstrap/{}/{}",
            payload.bootstrap_commit, entry.path
        );
        crate::transport::download(
            runner,
            &url,
            &path,
            entry.bytes,
            policy.command_timeout_seconds.min(60),
        )?;
        let bytes = crate::storage::read_bounded(&path, entry.bytes)?;
        if bytes.len() as u64 != entry.bytes || catalog::sha256(&bytes) != entry.sha256 {
            return Err(format!("source integrity mismatch: {}", entry.path));
        }
    }
    Ok(VerifiedRelease {
        payload,
        directory: stage,
    })
}

pub fn build(bootstrap: &Path, sequence: u64, days: u64) -> Result<Payload, String> {
    if !(1..=31).contains(&days) {
        return Err("validity must be 1..=31 days".into());
    }
    let run = |args: &[&str]| -> Result<Vec<u8>, String> {
        let output = std::process::Command::new("git")
            .args(["-C", bootstrap.to_str().ok_or("invalid source path")?])
            .args(args)
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err("Git source query failed".into());
        }
        Ok(output.stdout)
    };
    if !run(&["status", "--porcelain", "--untracked-files=no"])?.is_empty() {
        return Err("catalogue build requires a clean tracked source".into());
    }
    let commit = String::from_utf8(run(&["rev-parse", "HEAD"])?)
        .map_err(|e| e.to_string())?
        .trim()
        .to_owned();
    let mut files = vec![];
    for path in catalog::FILES {
        let spec = format!("{commit}:{path}");
        let bytes = run(&["show", &spec])?;
        files.push(catalog::SourceFile {
            path: (*path).into(),
            bytes: bytes.len() as u64,
            sha256: catalog::sha256(&bytes),
        });
    }
    let now = crate::model::unix_seconds();
    let payload = Payload {
        schema: 1,
        sequence,
        issued_at: now,
        expires_at: now + days * 86400,
        bootstrap_commit: commit,
        files,
        debs: vec![],
        updater_binaries: vec![],
    };
    payload.validate(now)?;
    Ok(payload)
}
pub fn report_path(policy: &Policy) -> PathBuf {
    policy.state_dir().join("last-run.json")
}
