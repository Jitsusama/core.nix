//! Commands the guard runs: producers' collections, podman's prunes and
//! predicates. None can hang it: a command runs without a shell or input,
//! in a process group of its own that a timeout kills whole, and its
//! output is read against the same deadline, capped, the rest drained so
//! it never blocks writing. Its priority is the guard's own, from its
//! unit, which children inherit.

use std::ffi::OsString;
use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub struct Job {
    pub argv: Vec<OsString>,
    pub cwd: Option<PathBuf>,
    /// Added to the guard's own environment.
    pub env: Vec<(OsString, OsString)>,
    pub timeout: Duration,
    /// How much of each stream is kept.
    pub limit: usize,
    /// Whether it runs at idle priority, for the processor and the disk.
    pub idle: bool,
}

#[derive(Debug, Default)]
pub struct Output {
    /// Its exit code, or `None` when a signal ended it.
    pub status: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// Whether either stream ran past the limit.
    pub truncated: bool,
}

#[derive(Debug)]
pub enum Ran {
    Finished(Output),
    TimedOut,
    /// Asked to stop before it finished, and ended or killed.
    Stopped,
}

/// How often a command is looked at while its output is quiet.
const TICK: Duration = Duration::from_millis(50);

/// How long output is waited for once the command itself has exited, in
/// case something it started holds the streams open.
const GRACE: Duration = Duration::from_millis(200);

/// Puts the calling thread at idle priority for the processor and the
/// disk. Best effort: a command the kernel won't lower still runs, at the
/// guard's own priority, and its timeout bounds it either way. Done in the
/// child alone, since an account can't raise a thread back from idle.
#[cfg(target_os = "linux")]
fn idle() {
    const IOPRIO_WHO_PROCESS: libc::c_int = 1;
    const IOPRIO_CLASS_IDLE: libc::c_int = 3;
    const IOPRIO_CLASS_SHIFT: libc::c_int = 13;
    let param = libc::sched_param { sched_priority: 0 };
    // SAFETY: both act on the calling thread alone, with valid arguments.
    unsafe {
        libc::sched_setscheduler(0, libc::SCHED_IDLE, &param);
        libc::syscall(
            libc::SYS_ioprio_set,
            IOPRIO_WHO_PROCESS,
            0,
            IOPRIO_CLASS_IDLE << IOPRIO_CLASS_SHIFT,
        );
    }
}

/// Puts the calling process in the background band, where macOS throttles
/// both its processor and its disk, best effort as on Linux.
#[cfg(target_os = "macos")]
fn idle() {
    // SAFETY: it acts on the calling process alone, with valid arguments.
    unsafe {
        libc::setpriority(libc::PRIO_DARWIN_PROCESS, 0, libc::PRIO_DARWIN_BG);
    }
}

/// Runs `job` as `run` does, until `stop` is set: then its group is asked
/// to end, and killed if it hasn't within `grace`.
pub fn stoppable(job: &Job, stop: &AtomicBool, grace: Duration) -> io::Result<Ran> {
    running(job, Some((stop, grace)))
}

pub fn run(job: &Job) -> io::Result<Ran> {
    running(job, None)
}

fn running(job: &Job, stop: Option<(&AtomicBool, Duration)>) -> io::Result<Ran> {
    let Some((program, args)) = job.argv.split_first() else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "no program"));
    };
    let mut command = Command::new(program);
    command
        .args(args)
        .envs(job.env.iter().map(|(name, value)| (name, value)))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    if let Some(cwd) = &job.cwd {
        command.current_dir(cwd);
    }
    if job.idle {
        // SAFETY: between fork and exec, only two system calls, both safe
        // in a forked child, touching nothing of the parent's.
        unsafe {
            command.pre_exec(|| {
                idle();
                Ok(())
            });
        }
    }
    let mut child = command.spawn()?;
    let deadline = Instant::now() + job.timeout;
    let mut streams = [
        Stream::new(
            child
                .stdout
                .take()
                .map(|out| Box::new(out) as Box<dyn ReadFd>),
        )?,
        Stream::new(
            child
                .stderr
                .take()
                .map(|err| Box::new(err) as Box<dyn ReadFd>),
        )?,
    ];
    let mut exited = None;
    let mut truncated = false;
    // When it was asked to end, and so must have by.
    let mut ending: Option<Instant> = None;
    loop {
        let now = Instant::now();
        if exited.is_none()
            && let Some(status) = child.try_wait()?
        {
            exited = Some((status, now));
        }
        if let Some((stop, grace)) = stop {
            match (ending, exited) {
                (None, None) if stop.load(Ordering::Relaxed) => {
                    signal(&child, libc::SIGTERM);
                    ending = Some(now + grace);
                }
                (Some(by), None) if now >= by => {
                    kill(child)?;
                    return Ok(Ran::Stopped);
                }
                // Whatever it said on the way out, it didn't finish.
                (Some(_), Some(_)) => return Ok(Ran::Stopped),
                _ => {}
            }
        }
        let open = streams.iter().any(|stream| stream.reader.is_some());
        match exited {
            Some((status, _)) if !open => {
                return Ok(Ran::Finished(finish(status, streams, truncated)));
            }
            Some((status, at)) if now >= at + GRACE => {
                return Ok(Ran::Finished(finish(status, streams, truncated)));
            }
            _ if now >= deadline => return kill(child),
            _ => {}
        }
        let wait = TICK.min(deadline.saturating_duration_since(now));
        wait_readable(&streams, wait)?;
        for stream in &mut streams {
            truncated |= stream.drain(job.limit)?;
        }
    }
}

fn finish(status: std::process::ExitStatus, streams: [Stream; 2], truncated: bool) -> Output {
    let [stdout, stderr] = streams;
    Output {
        status: status.code(),
        stdout: stdout.kept,
        stderr: stderr.kept,
        truncated,
    }
}

fn kill(mut child: Child) -> io::Result<Ran> {
    signal(&child, libc::SIGKILL);
    child.wait()?;
    Ok(Ran::TimedOut)
}

/// Signals the child's whole group, which is its pid; whatever it started
/// is in it unless it left on purpose.
fn signal(child: &Child, signal: libc::c_int) {
    // SAFETY: a plain system call; the group is the child's own.
    unsafe { libc::kill(-(child.id() as libc::pid_t), signal) };
}

trait ReadFd: Read + AsRawFd {}
impl<T: Read + AsRawFd> ReadFd for T {}

struct Stream {
    reader: Option<Box<dyn ReadFd>>,
    kept: Vec<u8>,
}

impl Stream {
    fn new(reader: Option<Box<dyn ReadFd>>) -> io::Result<Stream> {
        if let Some(reader) = &reader {
            let fd = reader.as_raw_fd();
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
            {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(Stream {
            reader,
            kept: Vec::new(),
        })
    }

    /// Reads whatever is waiting, keeping up to `limit`: true if some had
    /// to be dropped.
    fn drain(&mut self, limit: usize) -> io::Result<bool> {
        let Some(reader) = &mut self.reader else {
            return Ok(false);
        };
        let mut dropped = false;
        let mut buffer = [0; 64 << 10];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    self.reader = None;
                    return Ok(dropped);
                }
                Ok(read) => {
                    let room = limit.saturating_sub(self.kept.len()).min(read);
                    self.kept.extend_from_slice(&buffer[..room]);
                    dropped |= room < read;
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(dropped),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
    }
}

fn wait_readable(streams: &[Stream], wait: Duration) -> io::Result<()> {
    let mut fds: Vec<libc::pollfd> = streams
        .iter()
        .filter_map(|stream| stream.reader.as_ref())
        .map(|reader| libc::pollfd {
            fd: reader.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        })
        .collect();
    if fds.is_empty() {
        std::thread::sleep(wait);
        return Ok(());
    }
    let millis = wait.as_millis().min(i32::MAX as u128) as i32;
    if unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, millis) } < 0 {
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
    Ok(())
}
