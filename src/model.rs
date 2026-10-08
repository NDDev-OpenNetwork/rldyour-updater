use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Plan {
    pub schema: u8,
    pub channel: String,
    pub actions: Vec<Action>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Action {
    GdsInstall { platform: String },
    VerifiedApplications,
    AptObserve,
    ToolchainObserve,
    Homebrew,
    Flatpak,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionResult {
    pub action: String,
    pub ok: bool,
    pub changed: Option<bool>,
    pub observed: bool,
    pub detail: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub output_truncated: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunReport {
    pub schema: u8,
    pub started_at: u64,
    pub finished_at: u64,
    pub channel: String,
    pub results: Vec<ActionResult>,
    #[serde(default)]
    pub catalog_sequence: Option<u64>,
    #[serde(default)]
    pub bootstrap_commit: Option<String>,
}
impl ActionResult {
    pub fn observed(action: &str, detail: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            ok: true,
            changed: Some(false),
            observed: true,
            detail: detail.into(),
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            state: "observed".into(),
            output_truncated: false,
        }
    }
    pub fn failed(action: &str, detail: impl Into<String>) -> Self {
        Self {
            ok: false,
            state: "failed".into(),
            changed: None,
            ..Self::observed(action, detail)
        }
    }
}
pub fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| value.as_secs())
}
