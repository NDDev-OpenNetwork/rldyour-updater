//! Non-destructive path boundaries. Cached data is changed only by its owning
//! tool's GC, never by our recursive delete or a process-liveness heuristic.
use std::fs;
use std::path::{Component, Path, PathBuf};

pub use crate::os::is_redirect;

/// Reject redirected components, relative paths and parent traversals. A
/// missing path is allowed for creation; an existing one must resolve wholly.
pub fn plain_path(path: &Path) -> std::io::Result<()> {
    if !path.is_absolute() {
        return Err(std::io::Error::other("path must be absolute"));
    }
    let mut current = PathBuf::new();
    for part in path.components() {
        if matches!(part, Component::ParentDir) {
            return Err(std::io::Error::other("parent traversal refused"));
        }
        current.push(part);
        // A Windows drive/UNC prefix alone is not an inspectable filesystem
        // path; wait for its root component before asking for metadata.
        if matches!(part, Component::Prefix(_)) {
            continue;
        }
        match fs::symlink_metadata(&current) {
            Ok(md) if is_redirect(&md) => {
                return Err(std::io::Error::other("redirected path refused"));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

pub fn private_dir(path: &Path) -> std::io::Result<()> {
    plain_path(path)?;
    fs::create_dir_all(path)?;
    crate::os::private_permissions(path, true)?;
    Ok(())
}

/// Validate a bounded state destination before mutation; directories/FIFOs
/// must never turn a later report operation into a hang or surprise failure.
pub fn state_file(path: &Path, limit: u64) -> std::io::Result<()> {
    plain_path(path)?;
    match fs::symlink_metadata(path) {
        Ok(md) if !md.is_file() || md.len() > limit => Err(std::io::Error::other(
            "state must be a regular bounded file",
        )),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

