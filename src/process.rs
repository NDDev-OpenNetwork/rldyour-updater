//! One deadline includes child execution and pipe draining. Platform-owned
//! process trees prevent inherited streams from stranding a reader thread.
use crate::os::{self, Pipe, ProcessTree};
use std::{
    io,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
const OUTPUT_LIMIT: usize = 16 * 1024;
pub struct Output {
    pub stdout: String,
    pub stderr: String,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub exit_code: Option<i32>,
}
fn drain(
    mut source: impl Pipe,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> io::Result<(Vec<u8>, bool)> {
    let mut output = Vec::with_capacity(1024);
    let mut truncated = false;
    let mut chunk = [0u8; 4096];
    os::prepare_pipe(&source)?;
    loop {
        if cancelled.load(Ordering::Relaxed) || Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "output drain cancelled",
            ));
        }
        match os::read_pipe(&mut source, &mut chunk)? {
            Some(0) => return Ok((output, truncated)),
            Some(count) => {
                let keep = count.min(OUTPUT_LIMIT.saturating_sub(output.len()));
                truncated |= keep < count;
                output.extend_from_slice(&chunk[..keep]);
            }
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }
}
fn contained_run(command: &mut Command, timeout: Duration) -> Result<Output, String> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or("invalid command deadline")?;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let (mut child, mut tree) = ProcessTree::spawn(command)
        .map_err(|e| format!("cannot launch contained native tool: {e}"))?;
    let out = child.stdout.take().expect("stdout pipe");
    let err = child.stderr.take().expect("stderr pipe");
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancel_out = Arc::clone(&cancelled);
    let stdout = match std::thread::Builder::new()
        .name("update-stdout".into())
        .stack_size(128 * 1024)
        .spawn(move || drain(out, deadline, &cancel_out))
    {
        Ok(thread) => thread,
        Err(e) => {
            tree.terminate();
            let _ = child.kill();
            let _ = child.wait();
            return Err(e.to_string());
        }
    };
    let cancel_err = Arc::clone(&cancelled);
    let stderr = match std::thread::Builder::new()
        .name("update-stderr".into())
        .stack_size(128 * 1024)
        .spawn(move || drain(err, deadline, &cancel_err))
    {
        Ok(thread) => thread,
        Err(e) => {
            cancelled.store(true, Ordering::Relaxed);
            tree.terminate();
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout.join();
            return Err(e.to_string());
        }
    };
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            Ok(None) => {
                break Err("native command timed out; owned process tree terminated".to_string());
            }
            Err(e) => break Err(e.to_string()),
        }
    };
    tree.terminate(); // also terminates descendants after a parent exits early
    if status.is_err() {
        cancelled.store(true, Ordering::Relaxed);
        let _ = child.kill();
    }
    let _ = child.wait();
    let out = stdout
        .join()
        .map_err(|_| "stdout reader failed")?
        .map_err(|e| e.to_string());
    let err = stderr
        .join()
        .map_err(|_| "stderr reader failed")?
        .map_err(|e| e.to_string());
    let status = status?;
    let (stdout, stdout_truncated) = out?;
    let (stderr, stderr_truncated) = err?;
    let output = Output {
        stdout: String::from_utf8_lossy(&stdout).trim().into(),
        stderr: String::from_utf8_lossy(&stderr).trim().into(),
        stdout_truncated,
        stderr_truncated,
        exit_code: status.code(),
    };
    Ok(output)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
    pub truncated: bool,
    pub exit_code: Option<i32>,
}
pub trait CommandRunner {
    fn run(
        &self,
        argv: &[String],
        cwd: Option<&std::path::Path>,
        timeout: Duration,
    ) -> CommandOutput;
}
pub struct RealCommandRunner;
impl CommandRunner for RealCommandRunner {
    fn run(
        &self,
        argv: &[String],
        cwd: Option<&std::path::Path>,
        timeout: Duration,
    ) -> CommandOutput {
        let Some(program) = argv.first() else {
            return CommandOutput {
                ok: false,
                stdout: String::new(),
                stderr: "empty argv".into(),
                truncated: false,
                exit_code: None,
            };
        };
        let mut command = Command::new(program);
        command.args(&argv[1..]);
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        match contained_run(&mut command, timeout) {
            Ok(output) => CommandOutput {
                ok: output.exit_code == Some(0),
                stdout: output.stdout,
                stderr: output.stderr,
                truncated: output.stdout_truncated || output.stderr_truncated,
                exit_code: output.exit_code,
            },
            Err(error) => CommandOutput {
                ok: false,
                stdout: String::new(),
                stderr: error,
                truncated: false,
                exit_code: None,
            },
        }
    }
}
