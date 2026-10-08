use std::path::PathBuf;
use std::{
    io::{self, Read},
    os::windows::io::AsRawHandle,
    process::{Child, Command},
};
pub trait Pipe: Read + AsRawHandle {}
impl<T: Read + AsRawHandle> Pipe for T {}
pub fn prepare_pipe(_source: &impl Pipe) -> io::Result<()> {
    Ok(())
}
pub fn read_pipe(source: &mut impl Pipe, buffer: &mut [u8]) -> io::Result<Option<usize>> {
    source.read(buffer).map(Some)
}
pub struct ProcessTree;
impl ProcessTree {
    pub fn spawn(command: &mut Command) -> io::Result<(Child, Self)> {
        Ok((command.spawn()?, Self))
    }
    pub fn terminate(&mut self) {}
}
pub fn home_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_default()
}
pub fn executable_names(name: &str) -> Vec<String> {
    vec![format!("{name}.exe")]
}
