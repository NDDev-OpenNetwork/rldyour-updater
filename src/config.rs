use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema: u8,
    pub channel: String,
    #[serde(default = "timeout")]
    pub command_timeout_seconds: u64,
    #[serde(default)]
    pub release: ReleasePolicy,
    #[serde(default)]
    pub gds: GdsPolicy,
    #[serde(default)]
    pub apt: AptPolicy,
    #[serde(default)]
    pub toolchains: ToolchainPolicy,
    #[serde(default)]
    pub native: NativePolicy,
    #[serde(default)]
    pub state_dir: Option<PathBuf>,
}
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleasePolicy {
    pub url: String,
    /// Ed25519 public key bytes, hex. Trust is provisioned locally, never by the feed.
    pub public_key: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GdsPolicy {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "platform")]
    pub platform: String,
    #[serde(default)]
    pub python: PathBuf,
    /// Parsed for migration only. Never executed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub argv: Vec<String>,
    #[serde(default)]
    pub applications: bool,
}
impl Default for GdsPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            platform: platform(),
            python: PathBuf::new(),
            argv: vec![],
            applications: false,
        }
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AptPolicy {
    #[serde(default = "yes")]
    pub observe: bool,
    #[serde(default)]
    pub apply: bool,
}
impl Default for AptPolicy {
    fn default() -> Self {
        Self {
            observe: true,
            apply: false,
        }
    }
}
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NativePolicy {
    #[serde(default)]
    pub homebrew: bool,
    #[serde(default)]
    pub flatpak: bool,
    #[serde(default = "batch")]
    pub max_updates_per_run: usize,
}
fn batch() -> usize {
    10
}
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolchainPolicy {
    #[serde(default)]
    pub observe_only: bool,
}
impl Policy {
    pub fn from_path(path: &Path) -> Result<Self, String> {
        let bytes = crate::storage::read_bounded(path, 64 * 1024)?;
        let policy: Self = toml::from_slice(&bytes).map_err(|e| format!("invalid policy: {e}"))?;
        policy.validate()?;
        if crate::os::is_root() {
            crate::storage::root_owned(path)?;
        }
        Ok(policy)
    }
    pub fn defaults() -> Self {
        Self {
            schema: 2,
            channel: "signed-gds".into(),
            command_timeout_seconds: timeout(),
            release: ReleasePolicy::default(),
            gds: GdsPolicy::default(),
            apt: AptPolicy::default(),
            native: NativePolicy::default(),
            toolchains: ToolchainPolicy::default(),
            state_dir: None,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 2 {
            return Err("policy schema 2 required; migrate before enabling the scheduler".into());
        }
        if self.channel != "signed-gds" {
            return Err("only signed-gds is supported".into());
        }
        if !(1..=3600).contains(&self.command_timeout_seconds) {
            return Err("timeout must be 1..=3600 seconds".into());
        }
        if self.gds.enabled || self.gds.applications {
            crate::transport::validate_https(&self.release.url)?;
            crate::catalog::decode_hex::<32>(&self.release.public_key)?;
            crate::os::validate_absolute(&self.gds.python)?;
            if !self.gds.python.is_file() {
                return Err("Python interpreter is unavailable".into());
            }
            if !matches!(self.gds.platform.as_str(), "ubuntu" | "macos") {
                return Err("bootstrap supports ubuntu and macos only".into());
            }
        }
        if self.apt.apply {
            return Err(
                "APT mutation belongs to unattended-upgrades; apt.apply must be false".into(),
            );
        }
        if self.gds.enabled && crate::os::is_root() {
            return Err("user CLI updates must never run as root".into());
        }
        if self.gds.applications && (!cfg!(target_os = "linux") || !crate::os::is_root()) {
            return Err("application packages require a separate root-owned Linux policy".into());
        }
        if self.gds.applications {
            crate::storage::root_owned(
                &self.gds.python.canonicalize().map_err(|e| e.to_string())?,
            )?;
        }
        if self.native.homebrew && !cfg!(target_os = "macos") {
            return Err("Homebrew provider is macOS only".into());
        }
        if self.native.homebrew && !(1..=50).contains(&self.native.max_updates_per_run) {
            return Err("native update batch must be 1..=50".into());
        }
        if let Some(dir) = &self.state_dir {
            crate::storage::plain_path(dir)?;
        }
        Ok(())
    }
    pub fn state_dir(&self) -> PathBuf {
        self.state_dir
            .clone()
            .unwrap_or_else(|| crate::os::state_home().join("rldyour-updater"))
    }
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
    } else {
        "ubuntu".into()
    }
}
