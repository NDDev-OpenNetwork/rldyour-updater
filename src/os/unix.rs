use std::{
    io::{self, Read},
    os::fd::AsRawFd,
    os::unix::process::CommandExt,
    process::{Child, Command},
};
pub trait Pipe: Read + AsRawFd {}
impl<T: Read + AsRawFd> Pipe for T {}
pub fn prepare_pipe(source: &impl Pipe) -> io::Result<()> {
    let fd = source.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
pub fn read_pipe(source: &mut impl Pipe, buffer: &mut [u8]) -> io::Result<Option<usize>> {
    match source.read(buffer) {
        Ok(size) => Ok(Some(size)),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
            ) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}
pub struct ProcessTree {
    group: libc::pid_t,
}
impl ProcessTree {
    pub fn spawn(command: &mut Command) -> io::Result<(Child, Self)> {
        command.process_group(0);
        let child = command.spawn()?;
        let group = child
            .id()
            .try_into()
            .map_err(|_| io::Error::other("invalid child PID"))?;
        Ok((child, Self { group }))
    }
    pub fn terminate(&mut self) {
        if self.group > 0 {
            unsafe { libc::kill(-self.group, libc::SIGKILL) };
            self.group = 0;
        }
    }
}
impl Drop for ProcessTree {
    fn drop(&mut self) {
        self.terminate();
    }
}
