use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    #[serde(default = "schema")]
    pub schema: u8,
    #[serde(default = "channel")]
    pub channel: String,
    #[serde(default = "timeout")]
    pub command_timeout_seconds: u64,
    #[serde(default)]
    pub gds: GdsPolicy,
    #[serde(default)]
    pub apt: AptPolicy,
    #[serde(default)]
    pub toolchains: ToolchainPolicy,
    #[serde(default)]
    pub state_dir: Option<PathBuf>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GdsPolicy {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "platform")]
    pub platform: String,
    /// Absolute, reviewed argv. Usually python3, managed_cli.py, install, --platform and the platform name.
    #[serde(default)]
    pub argv: Vec<String>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AptPolicy {
    #[serde(default = "yes")]
    pub observe: bool,
    /// Only a root-owned system unit may enable this. User schedules fail closed.
    #[serde(default)]
    pub apply: bool,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolchainPolicy {
    #[serde(default = "yes")]
    pub observe_only: bool,
}
impl Default for GdsPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            platform: platform(),
            argv: Vec::new(),
        }
    }
}
impl Default for AptPolicy {
    fn default() -> Self {
        Self {
            observe: true,
            apply: false,
        }
    }
}
impl Default for ToolchainPolicy {
    fn default() -> Self {
        Self { observe_only: true }
    }
}
impl Policy {
    pub fn from_path(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
        let policy: Self = toml::from_slice(&bytes)
            .map_err(|error| format!("parse {}: {error}", path.display()))?;
        policy.validate()?;
        Ok(policy)
    }
    pub fn defaults() -> Self {
        Self {
            schema: 1,
            channel: channel(),
            command_timeout_seconds: timeout(),
            gds: GdsPolicy::default(),
            apt: AptPolicy::default(),
            toolchains: ToolchainPolicy::default(),
            state_dir: None,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1 {
            return Err(format!("unsupported policy schema {}", self.schema));
        }
        if self.channel != "signed-gds" {
            return Err("channel must be signed-gds".into());
        }
        if !(1..=3600).contains(&self.command_timeout_seconds) {
            return Err("command_timeout_seconds must be 1..=3600".into());
        }
        if self.gds.enabled {
            if self.gds.argv.is_empty() {
                return Err("gds.argv is required when gds.enabled=true".into());
            }
            if !self
                .gds
                .argv
                .iter()
                .any(|arg| arg.ends_with("managed_cli.py"))
            {
                return Err("gds.argv must invoke managed_cli.py".into());
            }
            if !self.gds.argv.iter().any(|arg| arg == "install")
                || !self.gds.argv.iter().any(|arg| arg == "--platform")
            {
                return Err("gds.argv must be an install invocation with --platform".into());
            }
            if self
                .gds
                .argv
                .iter()
                .any(|arg| arg.is_empty() || arg.len() > 4096)
            {
                return Err("gds.argv contains an invalid argument".into());
            }
        }
        if self.apt.apply && !cfg!(target_os = "linux") {
            return Err("apt.apply is supported only on Linux".into());
        }
        Ok(())
    }
    pub fn state_dir(&self) -> PathBuf {
        self.state_dir
            .clone()
            .unwrap_or_else(|| crate::os::state_home().join("rldyour-updater"))
    }
}
fn schema() -> u8 {
    1
}
fn channel() -> String {
    "signed-gds".into()
}
fn timeout() -> u64 {
    600
}
fn yes() -> bool {
    true
}
fn platform() -> String {
    if cfg!(target_os = "macos") {
        "macos".into()
    } else if cfg!(target_os = "windows") {
        "windows".into()
    } else {
        "ubuntu".into()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_fail_closed_until_gds_is_bound() {
        assert!(
            Policy::defaults()
                .validate()
                .unwrap_err()
                .contains("gds.argv")
        );
    }
    #[test]
    fn unknown_fields_fail() {
        assert!(toml::from_str::<Policy>("unknown=true\n").is_err());
    }
}
