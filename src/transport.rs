use crate::process::CommandRunner;
use std::{path::Path, time::Duration};

pub fn validate_https(url: &str) -> Result<(), String> {
    let rest = url.strip_prefix("https://").ok_or("HTTPS required")?;
    let host = rest.split('/').next().unwrap_or_default();
    if host.is_empty()
        || host.contains('@')
        || host.contains(':')
        || url.contains(['\n', '\r', '\0', '#'])
    {
        return Err("credential-free HTTPS URL required".into());
    }
    Ok(())
}
pub fn download<R: CommandRunner>(
    runner: &R,
    url: &str,
    file: &Path,
    max: u64,
    timeout: u64,
) -> Result<(), String> {
    validate_https(url)?;
    crate::storage::plain_path(file)?;
    let out = runner.run(
        &[
            crate::os::curl_binary().into(),
            "--disable".into(),
            "--fail".into(),
            "--location".into(),
            "--silent".into(),
            "--show-error".into(),
            "--proto".into(),
            "=https".into(),
            "--proto-redir".into(),
            "=https".into(),
            "--connect-timeout".into(),
            "15".into(),
            "--max-time".into(),
            timeout.to_string(),
            "--max-filesize".into(),
            max.to_string(),
            "--output".into(),
            file.to_string_lossy().into_owned(),
            "--url".into(),
            url.into(),
        ],
        None,
        Duration::from_secs(timeout + 5),
    );
    if !out.ok {
        return Err(format!("download failed: {}", out.stderr));
    }
    let md = file.symlink_metadata().map_err(|e| e.to_string())?;
    if !md.is_file() || md.len() > max {
        return Err("download exceeds declared bound".into());
    }
    Ok(())
}
