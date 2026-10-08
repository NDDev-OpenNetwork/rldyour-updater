//! Dedicated nonredirected state; no archive backups or user-data copies.
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

pub fn plain_path(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("absolute path required".into());
    }
    let mut cursor = PathBuf::new();
    for part in path.components() {
        if matches!(part, Component::ParentDir) {
            return Err("parent traversal refused".into());
        }
        cursor.push(part);
        if matches!(part, Component::Prefix(_)) {
            continue;
        }
        match cursor.symlink_metadata() {
            Ok(md) if md.file_type().is_symlink() => return Err("redirected path refused".into()),
            #[cfg(windows)]
            Ok(md)
                if {
                    use std::os::windows::fs::MetadataExt;
                    md.file_attributes() & 0x400 != 0
                } =>
            {
                return Err("reparse point refused".into());
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}
pub fn private_dir(path: &Path) -> Result<(), String> {
    plain_path(path)?;
    fs::create_dir_all(path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn state_file(path: &Path, limit: u64) -> Result<(), String> {
    plain_path(path)?;
    match path.symlink_metadata() {
        Ok(md) if !md.is_file() || md.len() > limit => {
            Err("state is not a regular bounded file".into())
        }
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
pub fn open_options(options: &mut OpenOptions) {
    #[cfg(not(unix))]
    let _ = options;
    #[cfg(unix)]
    options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
}
pub fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    state_file(path, limit)?;
    let mut options = OpenOptions::new();
    options.read(true);
    open_options(&mut options);
    let mut data = vec![];
    options
        .open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() as u64 > limit {
        return Err("file exceeds bound".into());
    }
    Ok(data)
}
pub fn replace(path: &Path, bytes: &[u8]) -> Result<(), String> {
    state_file(path, 4 * 1024 * 1024)?;
    let parent = path.parent().ok_or("destination has no parent")?;
    private_dir(parent)?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    tmp.write_all(bytes).map_err(|e| e.to_string())?;
    tmp.as_file().sync_all().map_err(|e| e.to_string())?;
    tmp.persist(path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    fs::File::open(parent)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(())
}
pub fn root_owned(path: &Path) -> Result<(), String> {
    plain_path(path)?;
    #[cfg(unix)]
    {
        let md = path.metadata().map_err(|e| e.to_string())?;
        if md.uid() != 0 || md.mode() & 0o022 != 0 {
            return Err("system policy must be root-owned and nonwritable by other users".into());
        }
    }
    Ok(())
}
