use fs2::FileExt;
use std::fs::{File, OpenOptions};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
pub struct RunLock {
    _file: File,
}
impl RunLock {
    pub fn acquire(dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("create lock dir: {e}"))?;
        #[cfg(unix)]
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("protect lock dir: {e}"))?;
        let path = dir.join("run.lock");
        let mut options = OpenOptions::new();
        options.create(true).truncate(false).read(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);
        let file = options
            .open(&path)
            .map_err(|e| format!("open {}: {e}", path.display()))?;
        file.try_lock_exclusive()
            .map_err(|e| format!("lock {}: {e}", path.display()))?;
        Ok(Self { _file: file })
    }
}
