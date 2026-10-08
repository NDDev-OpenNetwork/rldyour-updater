use crate::model::RunReport;
use std::{fs, io::Write, path::Path};
pub fn read(path: &Path) -> Result<RunReport, String> {
    serde_json::from_slice(&fs::read(path).map_err(|e| format!("read report: {e}"))?)
        .map_err(|e| format!("parse report: {e}"))
}
pub fn write_atomic(report: &RunReport, path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "report has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|e| format!("create report dir: {e}"))?;
    let temp = parent.join(format!(".last-run-{}.tmp", std::process::id()));
    let bytes = serde_json::to_vec_pretty(report).map_err(|e| format!("encode report: {e}"))?;
    let mut file = fs::File::create(&temp).map_err(|e| format!("create report temp: {e}"))?;
    file.write_all(&bytes)
        .map_err(|e| format!("write report: {e}"))?;
    file.sync_all().map_err(|e| format!("sync report: {e}"))?;
    fs::rename(temp, path).map_err(|e| format!("publish report: {e}"))
}
