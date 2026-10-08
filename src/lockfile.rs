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
impl Drop for RunLock {
    fn drop(&mut self) {
        // Explicit release also covers a descriptor briefly inherited by a
        // concurrently spawned child before its close-on-exec boundary.
        let _ = fs2::FileExt::unlock(&self._file);
    }
}
impl RunLock {
    pub fn acquire(dir: &Path) -> Result<Self, String> {
        crate::storage::private_dir(dir)?;
        #[cfg(unix)]
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("protect lock dir: {e}"))?;
        let path = dir.join("run.lock");
        crate::storage::state_file(&path, 1024)?;
        let mut options = OpenOptions::new();
        options.create(true).truncate(false).read(true).write(true);
        #[cfg(unix)]
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        let file = options
            .open(&path)
            .map_err(|e| format!("open {}: {e}", path.display()))?;
        #[cfg(unix)]
        file.set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("protect lock file: {e}"))?;
        file.try_lock_exclusive()
            .map_err(|e| format!("lock {}: {e}", path.display()))?;
        Ok(Self { _file: file })
    }
}
