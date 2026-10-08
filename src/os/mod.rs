use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;
#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(target_os = "macos")]
pub use macos::*;
#[cfg(unix)]
pub use unix::{Pipe, ProcessTree, prepare_pipe, read_pipe};
#[cfg(windows)]
pub use windows::*;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
compile_error!("Supported platforms: Linux, macOS and Windows");

pub fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home_dir().join(".config"))
}
pub fn state_home() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home_dir().join(".local/state"))
}
pub fn validate_absolute(path: &Path) -> Result<(), String> {
    if path.is_absolute() {
        Ok(())
    } else {
        Err(format!("path is not absolute: {}", path.display()))
    }
}
pub fn is_root() -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}
pub fn curl_binary() -> &'static str {
    #[cfg(unix)]
    {
        "/usr/bin/curl"
    }
    #[cfg(windows)]
    {
        "C:\\Windows\\System32\\curl.exe"
    }
}
