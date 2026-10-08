use std::path::PathBuf;
pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
}
pub fn executable_names(name: &str) -> Vec<String> {
    vec![name.into()]
}
