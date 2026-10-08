use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::path::Path;
pub struct RunLock {
    _file: File,
}
impl RunLock {
    pub fn acquire(dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("create lock dir: {e}"))?;
        let path = dir.join("run.lock");
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| format!("open {}: {e}", path.display()))?;
        file.try_lock_exclusive()
            .map_err(|e| format!("lock {}: {e}", path.display()))?;
        Ok(Self { _file: file })
    }
}
