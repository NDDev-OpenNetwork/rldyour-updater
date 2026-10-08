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
    AptObserve,
    ToolchainObserve,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionResult {
    pub action: String,
    pub ok: bool,
    pub changed: bool,
    pub observed: bool,
    pub detail: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunReport {
    pub schema: u8,
    pub started_at: u64,
    pub finished_at: u64,
    pub channel: String,
    pub results: Vec<ActionResult>,
}
pub fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| value.as_secs())
}
