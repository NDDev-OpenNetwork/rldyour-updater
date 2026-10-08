use std::{env, path::PathBuf};
pub fn home() -> Result<PathBuf, String> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".into())
}
pub fn config_path() -> Result<PathBuf, String> {
    if let Some(value) = env::var_os("RLDYOUR_UPDATER_CONFIG") {
        return Ok(PathBuf::from(value));
    }
    Ok(crate::os::config_home().join("rldyour-updater/config.toml"))
}
pub fn state_path() -> Result<PathBuf, String> {
    if let Some(value) = env::var_os("RLDYOUR_UPDATER_STATE") {
        return Ok(PathBuf::from(value));
    }
    Ok(crate::os::state_home().join("rldyour-updater/last-run.json"))
}
