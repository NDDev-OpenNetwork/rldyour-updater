use crate::model::RunReport;
use std::path::Path;
pub fn read(path: &Path) -> Result<RunReport, String> {
    serde_json::from_slice(&crate::storage::read_bounded(path, 4 * 1024 * 1024)?)
        .map_err(|e| e.to_string())
}
pub fn write_atomic(report: &RunReport, path: &Path) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(report).map_err(|e| e.to_string())?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("report exceeds bound".into());
    }
    crate::storage::replace(path, &bytes)
}
