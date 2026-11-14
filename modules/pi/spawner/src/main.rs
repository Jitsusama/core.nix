//! Starts pi's shell commands for it, from a process small enough to
//! start them without stalling pi.
//!
//! pi grows past a gigabyte in a long session, and each command its bash
//! tool starts forks the whole process first: 36 ms at the median from a
//! 1.1 GB node against 4 ms from a fresh one (10 November 2026), on the
//! thread that also reads the person's keys. The agent harness starts
//! this once for each pi and hands it every command instead; pi then
//! only writes a request and reads what comes back.
//!
//! Requests arrive on stdin and answers leave on stdout, each framed with
//! its length (see `read_request` and `send`). Each command runs in a
//! process group of its own, as pi's detached commands do, so pi stops a
//! command and all it started by signalling the group itself. When stdin
//! ends pi has gone, and every group still running is killed, as pi kills
//! its detached commands when it exits.
//!
//! Its help and its refusal of words it doesn't take are the terminal
//! kit's (docs/commands.md); stdout is the protocol's alone, written
//! straight, never through the kit.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use terminal_kit::{Exit, Failure, Help};

/// A command's process id, sent once it has started: four bytes.
const STARTED: u8 = b'p';
/// Output: one byte naming the stream (1 stdout, 2 stderr), then bytes.
const OUTPUT: u8 = b'o';
/// A stream reached its end: one byte naming it.
const CLOSED: u8 = b'c';
/// The command ended: its exit code, or -1 when a signal ended it, then
/// the signal, or 0; four bytes each.
const EXITED: u8 = b'x';
/// The command could not start: why, as text.
const FAILED: u8 = b'e';

/// How much of a command's output one frame carries at most.
const CHUNK: usize = 64 * 1024;

/// A command still running, as stdin's end and a drop need it.
struct Running {
    pid: i32,
    /// Set once pi stops listening, so its output goes unsent.
    dropped: Arc<AtomicBool>,
}

/// What every thread shares: the one way out, and what still runs.
struct Shared {
    out: Mutex<io::Stdout>,
    running: Mutex<HashMap<u32, Running>>,
}

/// What pi asks: start a command, or stop sending one's output.
enum Request {
    Run {
        id: u32,
        program: Vec<u8>,
        cwd: Vec<u8>,
        args: Vec<Vec<u8>>,
        env: Vec<(Vec<u8>, Vec<u8>)>,
    },
    Drop {
        id: u32,
    },
}

fn help() -> Help {
    Help::new("pi-spawner", "starts pi's shell commands for it")
        .example("pi-spawner", "takes framed requests on stdin until pi goes")
}

fn main() -> std::process::ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        if let Some(word) = context.args.first() {
            return Err(Failure::new(Exit::Usage, format!("it takes no {word}"))
                .try_this("pi-spawner --help"));
        }
        serve();
        Ok(())
    })
}

/// Starts what pi asks until stdin ends, then ends what still runs.
fn serve() {
    // Whoever writes to stdin starts commands outside pi's own checks, so
    // only pi may reach it. A process that can't be dumped keeps its
    // /proc entries from other processes of the same user: without this,
    // a command in a sandbox that shares the process namespace could
    // open /proc/<spawner>/fd/0 and send requests of its own. What the
    // spawner starts is dumpable again, as execve resets the flag. macOS
    // has no /proc to reach it through.
    // SAFETY: prctl with PR_SET_DUMPABLE takes plain integers and only
    // changes a flag of this process.
    #[cfg(target_os = "linux")]
    unsafe {
        libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
    }
    let shared = Arc::new(Shared {
        out: Mutex::new(io::stdout()),
        running: Mutex::new(HashMap::new()),
    });
    let mut input = io::stdin().lock();
    while let Some(request) = read_request(&mut input) {
        match request {
            Request::Run {
                id,
                program,
                cwd,
                args,
                env,
            } => run(&shared, id, &program, &cwd, &args, &env),
            Request::Drop { id } => {
                if let Some(one) = lock(&shared.running).get(&id) {
                    one.dropped.store(true, Ordering::SeqCst);
                }
            }
        }
    }
    // pi has gone, so nothing it started outlives it.
    for one in lock(&shared.running).values() {
        // SAFETY: kill only sends a signal; a group already gone answers
        // an error, which leaves nothing to do.
        unsafe {
            libc::kill(-one.pid, libc::SIGKILL);
        }
    }
}

/// A lock, taken even when a thread holding it panicked: what it guards
/// stays usable, and a spawner that stopped answering would strand pi.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The next request, or nothing once stdin has ended or makes no sense.
///
/// A request is a four-byte little-endian length, then fields each ended
/// by a zero byte: `run`, the id, the program, the folder, the count of
/// arguments and each argument, then the count of variables and each
/// variable as `NAME=value`; or `drop` and the id. A zero byte can end a
/// field since none can sit in an argument or a variable.
fn read_request(input: &mut impl Read) -> Option<Request> {
    let mut length = [0u8; 4];
    input.read_exact(&mut length).ok()?;
    let mut body = vec![0u8; usize::try_from(u32::from_le_bytes(length)).ok()?];
    input.read_exact(&mut body).ok()?;
    let mut fields = body.split(|byte| *byte == 0);
    let op = fields.next()?;
    let id = number(fields.next()?)?;
    match op {
        b"drop" => Some(Request::Drop { id }),
        b"run" => {
            let program = fields.next()?.to_vec();
            let cwd = fields.next()?.to_vec();
            let arguments = number(fields.next()?)?;
            let args = (0..arguments)
                .map(|_| fields.next().map(<[u8]>::to_vec))
                .collect::<Option<Vec<_>>>()?;
            let variables = number(fields.next()?)?;
            let env = (0..variables)
                .map(|_| {
                    let pair = fields.next()?;
                    let at = pair.iter().position(|byte| *byte == b'=')?;
                    Some((pair[..at].to_vec(), pair[at + 1..].to_vec()))
                })
                .collect::<Option<Vec<_>>>()?;
            Some(Request::Run {
                id,
                program,
                cwd,
                args,
                env,
            })
        }
        _ => None,
    }
}

/// A field holding a decimal number.
fn number(field: &[u8]) -> Option<u32> {
    std::str::from_utf8(field).ok()?.parse().ok()
}

/// Start one command and pass on all it says, from threads of its own,
/// so a long command never holds up the next request.
fn run(
    shared: &Arc<Shared>,
    id: u32,
    program: &[u8],
    cwd: &[u8],
    args: &[Vec<u8>],
    env: &[(Vec<u8>, Vec<u8>)],
) {
    let mut command = Command::new(OsStr::from_bytes(program));
    command
        .args(args.iter().map(|arg| OsStr::from_bytes(arg)))
        .current_dir(OsStr::from_bytes(cwd))
        .env_clear()
        .envs(
            env.iter()
                .map(|(name, value)| (OsStr::from_bytes(name), OsStr::from_bytes(value))),
        )
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            send(shared, FAILED, id, error.to_string().as_bytes());
            return;
        }
    };
    let pid = child.id();
    let dropped = Arc::new(AtomicBool::new(false));
    if let Ok(group) = i32::try_from(pid) {
        lock(&shared.running).insert(
            id,
            Running {
                pid: group,
                dropped: Arc::clone(&dropped),
            },
        );
    }
    send(shared, STARTED, id, &pid.to_le_bytes());
    if let Some(stdout) = child.stdout.take() {
        forward(shared, id, 1, stdout, &dropped);
    }
    if let Some(stderr) = child.stderr.take() {
        forward(shared, id, 2, stderr, &dropped);
    }
    let shared = Arc::clone(shared);
    thread::spawn(move || {
        let (code, signal) = match child.wait() {
            Ok(status) => (status.code().unwrap_or(-1), status.signal().unwrap_or(0)),
            Err(_) => (-1, 0),
        };
        lock(&shared.running).remove(&id);
        let mut ended = code.to_le_bytes().to_vec();
        ended.extend_from_slice(&signal.to_le_bytes());
        send(&shared, EXITED, id, &ended);
    });
}

/// Pass on one stream of a command, until its end, unless pi dropped it.
fn forward(
    shared: &Arc<Shared>,
    id: u32,
    stream: u8,
    mut from: impl Read + Send + 'static,
    dropped: &Arc<AtomicBool>,
) {
    let shared = Arc::clone(shared);
    let dropped = Arc::clone(dropped);
    thread::spawn(move || {
        let mut buffer = vec![0u8; CHUNK + 1];
        buffer[0] = stream;
        loop {
            match from.read(&mut buffer[1..]) {
                Ok(0) | Err(_) => break,
                Ok(read) => {
                    if !dropped.load(Ordering::SeqCst) {
                        send(&shared, OUTPUT, id, &buffer[..=read]);
                    }
                }
            }
        }
        if !dropped.load(Ordering::SeqCst) {
            send(&shared, CLOSED, id, &[stream]);
        }
    });
}

/// Send one frame: its kind, the command's id and the payload's length,
/// four little-endian bytes each but the kind, then the payload.
fn send(shared: &Shared, kind: u8, id: u32, payload: &[u8]) {
    let Ok(length) = u32::try_from(payload.len()) else {
        return;
    };
    let mut frame = Vec::with_capacity(payload.len() + 9);
    frame.push(kind);
    frame.extend_from_slice(&id.to_le_bytes());
    frame.extend_from_slice(&length.to_le_bytes());
    frame.extend_from_slice(payload);
    let mut out = lock(&shared.out);
    // A pi that stopped reading has gone; stdin's end then kills the rest.
    let _ = out.write_all(&frame).and_then(|()| out.flush());
}
