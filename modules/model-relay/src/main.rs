//! `model-relay` answers pi's model requests on a socket and carries them
//! to their one origin over a few long-lived HTTP/2 connections
//! (src/lib.rs says why).
//!
//! systemd hands it its listening socket (`LISTEN_FDS`), and beside it
//! the lend's, named `lend`, and the one it's asked to hand over on,
//! named `control`, when there are those; `--socket`, `--lend-socket` and
//! `--control-socket` bind them itself, as the tests do. It takes the
//! shape every command of ours does (docs/commands.md): its help, usage
//! that says what to try, and failures with named exits.
//!
//! A switch reloads it rather than restarting it, and the reload runs the
//! new build as `model-relay --hand-over CONTROL` with the flags it's to
//! serve with. That asks the running relay, on its control socket, to
//! start this build in its place: it starts it with its own sockets left
//! open across the exec and named in `--taken-over`, waits for it to say
//! it listens, tells systemd the new one is the service's main process
//! now, and leaves as src/lib.rs says. Should the new build not say so
//! within `READY_WITHIN`, it's stopped, the running relay carries on, and
//! the reload fails saying why, so a build that can't serve never takes
//! pi's socket from one that can.
//!
//! Long because telling handed sockets apart, the hand-over's two sides
//! and their tests each need the others close: the sockets a hand-over
//! leaves open are those systemd handed, named the same way.

use std::io::{BufRead, Read, Write};
use std::os::fd::{FromRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, PermissionsExt};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::process::{ExitCode, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use model_relay::{
    CONNECT_WITHIN, Clocks, DRAINS_WITHIN, HANDED_OVER_ANSWERS_WITHIN, IDLE_EXIT, Origin, Relay,
    Roots,
};
use terminal_kit::{Exit, Failure, Help, flags};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::signal::unix::SignalKind;

/// The named flags it reads, every one taking a value.
const FLAGS: [&str; 11] = [
    "--origin",
    "--socket",
    "--connections",
    "--roots",
    "--connect-within",
    "--idle-exit",
    "--drain-within",
    "--lend-socket",
    "--control-socket",
    "--hand-over",
    "--taken-over",
];

/// Where `--hand-over` sits in `FLAGS`.
const HAND_OVER: usize = 9;
/// Where `--taken-over` sits in `FLAGS`.
const TAKEN_OVER: usize = 10;

/// The name systemd gives the lend's socket (`FileDescriptorName`).
const LEND: &str = "lend";
/// The name systemd gives the socket it's asked to hand over on.
const CONTROL: &str = "control";
/// The name pi's socket goes by in `--taken-over`.
const AGENTS: &str = "agents";

/// How long a build started to take over has to say it listens. A relay
/// starts in well under a second; one taking longer is wedged.
const READY_WITHIN: Duration = Duration::from_secs(30);
/// What a build started to take over says on its stdout once it listens.
const READY: &str = "listening";
/// What a relay says on its control socket once it has a hand-over's ask.
const ASKED: &str = "asked";
/// How long a hand-over's ask may take to arrive whole.
const ASKED_WITHIN: Duration = Duration::from_secs(5);
/// The most a hand-over's ask, or its answer, may hold: a path and a few
/// flags.
const ASK_AT_MOST: u64 = 64 * 1024;

/// How many connections it keeps to the origin unless told.
const CONNECTIONS: usize = 4;
/// The most it may be told to keep: still a handful beside the separate
/// sessions' own connections it replaces. Each slot retires its connection
/// for a reset at most once a minute, so a run of resets retires no more
/// than this many a minute, though each retired one stays open while its
/// answers stream. More than the default carries more at once over a lossy
/// path: on 26 November bodies sharing four connections took 7 to 9 s, and
/// ten took 3 to 4 s. Nothing past ten was measured.
const MOST_CONNECTIONS: usize = 16;

/// The first descriptor systemd hands a service it starts for a socket.
const LISTEN_FDS_START: i32 = 3;

/// How long it waits, its counts said, on a name lookup of the origin
/// still running before it leaves anyway. Nothing can cut a lookup, and
/// one waiting on a resolver gone quiet can take tens of seconds, which a
/// relay leaving within its stop timeout hasn't got.
const LOOKUPS_GIVEN: Duration = Duration::from_millis(500);

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "model-relay",
        "carries pi's model requests to their origin over a few shared connections",
    )
    .example(
        "model-relay --origin https://models.example --socket relay.sock",
        "answer relay.sock, forwarding to models.example",
    )
    .option("--origin URL", "the one https origin it forwards to")
    .option(
        "--socket PATH",
        "the socket to answer, if systemd hands it none",
    )
    .option(
        "--connections N",
        &format!(
            "how many connections to the origin it keeps, {CONNECTIONS} if not told, at most \
             {MOST_CONNECTIONS}"
        ),
    )
    .option(
        "--lend-socket PATH",
        "a socket for a client that can't go direct, if systemd hands it none",
    )
    .option(
        "--control-socket PATH",
        "a socket to be asked to hand over on, if systemd hands it none",
    )
    .option(
        "--hand-over CONTROL",
        "ask the relay answering CONTROL to hand over to this build, with these flags",
    )
    .option(
        "--taken-over agents=FD,...",
        "serve the sockets a relay handing over left open at these descriptors",
    )
    .option(
        "--roots FILE",
        "trust only the roots in this PEM file, not the public ones",
    )
    .option(
        "--connect-within SECONDS",
        "how long a connection to the origin may take to open, 10 if not told",
    )
    .option(
        "--idle-exit SECONDS",
        "leave once asked nothing this long, 120 if not told, never at 0",
    )
    .option(
        "--drain-within SECONDS",
        "once stopped, wait on the answers it holds this long, then cut them, 80 if not told",
    )
    .exit(
        Exit::Failed,
        "it couldn't listen or hear its stop, its roots couldn't be read, or a hand-over \
         was refused",
    )
}

/// The value each named flag was given, in `FLAGS`' order, or a usage
/// failure for a flag it doesn't read or one left without a value.
fn named(args: &[String]) -> Result<Vec<Option<String>>, Failure> {
    let mut given = vec![None; FLAGS.len()];
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let Some(at) = FLAGS.iter().position(|name| name == arg) else {
            return Err(flags::unknown("flag", arg, &FLAGS));
        };
        let value = args.next().ok_or_else(|| {
            Failure::new(Exit::Usage, format!("{arg} needs a value")).try_this("model-relay --help")
        })?;
        given[at] = Some(value.clone());
    }
    Ok(given)
}

/// The sockets it answers: pi's, the lend's when there's one, and the one
/// it's asked to hand over on when there's that.
struct Handed {
    agents: std::os::unix::net::UnixListener,
    lend: Option<std::os::unix::net::UnixListener>,
    control: Option<std::os::unix::net::UnixListener>,
}

/// Which of `Handed`'s sockets each descriptor is, by its name, pi's going
/// unnamed, or why the names don't say one pi's and at most one of each
/// other.
fn sorted<'a>(named: impl Iterator<Item = (&'a str, RawFd)>) -> Result<[Option<RawFd>; 3], String> {
    let mut sorted = [None; 3];
    for (name, fd) in named {
        let (at, whose) = match name {
            LEND => (1, "the lend's"),
            CONTROL => (2, "the one it's asked to hand over on"),
            _ => (0, "pi's"),
        };
        if sorted[at].replace(fd).is_some() {
            return Err(format!("two of them are {whose}"));
        }
    }
    if sorted[0].is_none() {
        return Err("none of them is pi's, which goes unnamed".to_owned());
    }
    Ok(sorted)
}

/// Tells the `count` descriptors systemd handed apart by their `names`
/// (`LISTEN_FDNAMES`), or says why it can't.
fn places(count: &str, names: &str) -> Result<[Option<RawFd>; 3], String> {
    let count: i32 = count
        .parse()
        .ok()
        .filter(|count| (1..=3).contains(count))
        .ok_or_else(|| {
            format!("systemd handed it {count} sockets, where it answers one to three")
        })?;
    let mut given = names.split(':');
    sorted(
        (LISTEN_FDS_START..LISTEN_FDS_START + count)
            .map(|fd| (given.next().unwrap_or_default(), fd)),
    )
    .map_err(|why| format!("of the {count} sockets systemd handed it, named {names:?}, {why}"))
}

/// The listeners at `fds`, as `sorted` found them.
///
/// # Safety
///
/// Each descriptor must be open and listening, this process's alone, and
/// taken by nothing else here.
unsafe fn listeners(fds: [Option<RawFd>; 3]) -> Option<Handed> {
    let at = |fd: Option<RawFd>| {
        fd.map(|fd| {
            // SAFETY: the caller's promise.
            unsafe { std::os::unix::net::UnixListener::from_raw_fd(fd) }
        })
    };
    let [agents, lend, control] = fds;
    Some(Handed {
        agents: at(agents)?,
        lend: at(lend),
        control: at(control),
    })
}

/// The sockets systemd handed it, when it was started for one to three,
/// or why those it handed can't be told apart.
fn handed() -> Result<Option<Handed>, Failure> {
    let ours = std::env::var("LISTEN_PID").is_ok_and(|pid| pid == std::process::id().to_string());
    let Some(count) = std::env::var("LISTEN_FDS").ok().filter(|_| ours) else {
        return Ok(None);
    };
    let names = std::env::var("LISTEN_FDNAMES").unwrap_or_default();
    let fds = places(&count, &names).map_err(|why| {
        Failure::new(
            Exit::Failed,
            "it couldn't tell the sockets systemd handed it apart",
        )
        .because(why)
        .try_this(format!(
            "name the lend's socket {LEND} and the hand-over's {CONTROL} with \
             FileDescriptorName, and leave pi's unnamed"
        ))
    })?;
    // SAFETY: systemd passed these descriptors, open and listening, for
    // this process alone (LISTEN_PID), and nothing else here takes them.
    Ok(unsafe { listeners(fds) })
}

/// The sockets a relay handing over left open across its exec, as
/// `--taken-over` names them, such as `agents=3,lend=4,control=5`, set to
/// close on this build's own execs again.
fn taken(over: &str) -> Result<Handed, Failure> {
    let unread = || {
        Failure::new(Exit::Usage, "--taken-over names no sockets it can take")
            .try_this("--taken-over agents=3,lend=4,control=5")
    };
    let mut named = Vec::new();
    for pair in over.split(',') {
        let (name, fd) = pair.split_once('=').ok_or_else(unread)?;
        let fd: RawFd = fd.parse().map_err(|_| unread())?;
        // Below 3 are its stdin, stdout and stderr, never a socket.
        if fd < 3 || ![AGENTS, LEND, CONTROL].contains(&name) {
            return Err(unread());
        }
        named.push((name, fd));
    }
    let fds = sorted(named.into_iter()).map_err(|_| unread())?;
    // SAFETY: the relay handing over left these open and listening across
    // its exec for this process alone, named as `--taken-over` says, and
    // nothing else here takes them.
    unsafe { listeners(fds) }.ok_or_else(unread)
}

/// Binds `path` itself, answering only the account that runs it. The
/// socket is made inside a folder of its own that only the account can
/// enter, kept to the account there, and only then linked at `path`, so
/// there's no moment it's open to anyone a loose umask would let in.
///
/// Something at `path` that isn't a socket is left be, and so is a socket
/// a relay answers. One nothing answers, left by a relay that's gone, is
/// taken. A link, unlike a move, fails on finding something there, so two
/// relays starting together can't both take it.
fn bound(path: &Path) -> Result<std::os::unix::net::UnixListener, Failure> {
    let cannot_listen = |why: std::io::Error| {
        Failure::new(Exit::Failed, "it couldn't listen")
            .because(why.to_string())
            .try_this(format!(
                "check that {}'s folder is there and writable",
                path.display()
            ))
    };
    let taken = |why: String| Failure::new(Exit::Failed, "it couldn't listen").because(why);
    match std::fs::symlink_metadata(path) {
        Ok(there) if !there.file_type().is_socket() => {
            return Err(taken(format!(
                "{} is there already, and isn't a socket",
                path.display()
            )));
        }
        Ok(_) if std::os::unix::net::UnixStream::connect(path).is_ok() => {
            return Err(taken(format!(
                "another relay already answers {}",
                path.display()
            )));
        }
        Ok(_) => std::fs::remove_file(path).map_err(cannot_listen)?,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
        Err(why) => return Err(cannot_listen(why)),
    }
    let folder = path.parent().unwrap_or_else(|| Path::new("."));
    let unguessable = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let private = folder.join(format!(".model-relay-{}-{unguessable}", std::process::id()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&private)
        .map_err(cannot_listen)?;
    let made = private.join("socket");
    let listened = std::os::unix::net::UnixListener::bind(&made)
        .and_then(|listener| {
            std::fs::set_permissions(&made, std::fs::Permissions::from_mode(0o600))?;
            std::fs::hard_link(&made, path)?;
            Ok(listener)
        })
        .map_err(|why| {
            if why.kind() == std::io::ErrorKind::AlreadyExists {
                taken(format!("another relay took {} just now", path.display()))
            } else {
                cannot_listen(why)
            }
        });
    // The folder held the socket only on its way to `path`, which links it
    // now if the link was made; neither is anything to keep, and one that
    // can't be removed is the account's own to remove.
    let _ = std::fs::remove_file(&made);
    let _ = std::fs::remove_dir(&private);
    listened
}

/// What a socket that can't be listened on fails as.
fn unlistened(why: &std::io::Error) -> Failure {
    Failure::new(Exit::Failed, "it couldn't listen").because(why.to_string())
}

/// Where its sockets come from, as its flags say.
struct Sockets<'a> {
    socket: Option<&'a str>,
    lend: Option<&'a str>,
    control: Option<&'a str>,
    taken_over: Option<&'a str>,
}

/// The sockets a relay handing over left it, or else those systemd handed
/// it, or else those it binds as `sockets` say, ready to be answered
/// without blocking.
fn listening(sockets: &Sockets<'_>) -> Result<Handed, Failure> {
    let bind = |path: Option<&str>| path.map(|path| bound(Path::new(path))).transpose();
    let handed = match (sockets.taken_over, handed()?, sockets.socket) {
        (Some(over), _, _) => taken(over)?,
        (None, Some(handed), _) => handed,
        (None, None, Some(path)) => Handed {
            agents: bound(Path::new(path))?,
            lend: bind(sockets.lend)?,
            control: bind(sockets.control)?,
        },
        (None, None, None) => {
            return Err(Failure::new(
                Exit::Usage,
                "systemd handed it no socket, and it was given none",
            )
            .try_this("model-relay --origin URL --socket PATH"));
        }
    };
    for listener in [
        Some(&handed.agents),
        handed.lend.as_ref(),
        handed.control.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        // Those systemd hands and those a hand-over leaves come open across
        // execs, so a build it hands over to would hold each a second time,
        // unnamed, beside the copies `--taken-over` names.
        let socket = socket2::SockRef::from(listener);
        socket
            .set_nonblocking(true)
            .and_then(|()| socket.set_cloexec(true))
            .map_err(|why| unlistened(&why))?;
    }
    Ok(handed)
}

/// Asks the relay answering `control` to hand over to this build, to serve
/// with `args`, its own flags but `--hand-over`, and says what it answered.
fn hand_over(control: &str, args: &[String]) -> Result<(), Failure> {
    let refused =
        |why: String| Failure::new(Exit::Failed, "the relay didn't hand over").because(why);
    let exe = std::env::current_exe()
        .map_err(|why| refused(format!("this build's own path couldn't be read: {why}")))?;
    let mut ask = exe.as_os_str().as_bytes().to_vec();
    let mut flags = args.iter();
    while let Some(arg) = flags.next() {
        if arg == FLAGS[HAND_OVER] {
            flags.next();
            continue;
        }
        ask.push(0);
        ask.extend_from_slice(arg.as_bytes());
    }
    let stream = std::os::unix::net::UnixStream::connect(control).map_err(|why| {
        refused(format!("nothing answers {control}: {why}")).try_this(
            "start the relay, which then runs this build: systemctl --user start \
                       model-relay-agents",
        )
    })?;
    // A relay that can hand over says it has the ask at once; systemd holds
    // the control socket even for one started before it could, which never
    // takes the ask, so only that word tells the two apart in good time.
    let mut heard = std::io::BufReader::new(&stream);
    let mut first = String::new();
    stream
        .set_read_timeout(Some(ASKED_WITHIN))
        .and_then(|()| (&stream).write_all(&ask))
        .and_then(|()| stream.shutdown(std::net::Shutdown::Write))
        .and_then(|()| (&mut heard).take(ASK_AT_MOST).read_line(&mut first))
        .map_err(|why| {
            refused(format!(
                "nothing took the ask on {control} within {} s: {why}",
                ASKED_WITHIN.as_secs()
            ))
            .try_this(
                "a relay started before it could hand over takes none, so restart it once: \
                 systemctl --user restart model-relay-agents",
            )
        })?;
    let answer = if first.trim() == ASKED {
        let mut answer = String::new();
        stream
            .set_read_timeout(Some(READY_WITHIN + ASKED_WITHIN))
            .and_then(|()| (&mut heard).take(ASK_AT_MOST).read_to_string(&mut answer))
            .map_err(|why| refused(format!("the relay at {control} didn't answer: {why}")))?;
        answer
    } else {
        first
    };
    match answer.trim().strip_prefix("taken by ") {
        Some(pid) => {
            eprintln!("model-relay: handed over to this build, running as {pid}");
            Ok(())
        }
        None => Err(refused(answer.trim().to_owned())),
    }
}

/// Tells systemd `state` as `sd_notify` does, when it was started to listen
/// for that, or says why it couldn't.
fn notify(state: &str) -> Result<(), String> {
    let Some(path) = std::env::var_os("NOTIFY_SOCKET") else {
        // Started by systemd with no way to tell it its new main process,
        // it would take the new build down with it as it left.
        return match std::env::var_os("INVOCATION_ID") {
            Some(_) => Err(
                "systemd can't be told its new main process: NOTIFY_SOCKET is unset, \
                            so the unit needs NotifyAccess"
                    .to_owned(),
            ),
            None => Ok(()),
        };
    };
    let socket = std::os::unix::net::UnixDatagram::unbound()
        .map_err(|why| format!("systemd's notify socket couldn't be reached: {why}"))?;
    socket
        .send_to(state.as_bytes(), &path)
        .map(|_| ())
        .map_err(|why| {
            format!(
                "systemd couldn't be told {state} at {}: {why}",
                Path::new(&path).display()
            )
        })
}

/// The sockets a hand-over leaves open across the exec, by the names
/// `--taken-over` gives them.
struct Shared {
    named: Vec<(&'static str, std::os::unix::net::UnixListener)>,
}

impl Shared {
    /// Copies of `handed`'s sockets, kept to give a build taking over.
    fn of(handed: &Handed) -> std::io::Result<Shared> {
        let mut named = vec![(AGENTS, handed.agents.try_clone()?)];
        if let Some(lend) = &handed.lend {
            named.push((LEND, lend.try_clone()?));
        }
        if let Some(control) = &handed.control {
            named.push((CONTROL, control.try_clone()?));
        }
        Ok(Shared { named })
    }

    /// Whether each is left open across an exec, as `open` says.
    fn across_exec(&self, open: bool) -> std::io::Result<()> {
        for (_, listener) in &self.named {
            socket2::SockRef::from(listener).set_cloexec(!open)?;
        }
        Ok(())
    }

    /// What `--taken-over` names them as.
    fn taken_over(&self) -> String {
        let named: Vec<String> = self
            .named
            .iter()
            .map(|(name, listener)| format!("{name}={}", listener.as_raw_fd()))
            .collect();
        named.join(",")
    }
}

/// Starts the build `ask` names to take over `shared`, and once it says
/// it listens, makes it systemd's main process, giving back its pid; or
/// stops it and says why.
async fn take_over(ask: &[u8], shared: &Shared, stopping: &AtomicBool) -> Result<u32, String> {
    let mut parts = ask.split(|&byte| byte == 0);
    let exe = std::ffi::OsStr::from_bytes(parts.next().unwrap_or_default());
    if !Path::new(exe).is_absolute() {
        return Err("a hand-over names its build by its whole path".to_owned());
    }
    let args: Vec<&std::ffi::OsStr> = parts.map(std::ffi::OsStr::from_bytes).collect();
    shared
        .across_exec(true)
        .map_err(|why| format!("its sockets couldn't be left open for the new build: {why}"))?;
    let started = std::process::Command::new(exe)
        .args(&args)
        .arg(FLAGS[TAKEN_OVER])
        .arg(shared.taken_over())
        .env_remove("LISTEN_PID")
        .env_remove("LISTEN_FDS")
        .env_remove("LISTEN_FDNAMES")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn();
    // Open across this build's later execs, they'd reach whatever else it
    // started; it starts nothing else, but holds them closed all the same.
    let closed = shared.across_exec(false);
    let mut child =
        started.map_err(|why| format!("{} couldn't start: {why}", Path::new(exe).display()))?;
    closed.map_err(|why| format!("its sockets couldn't be closed across execs again: {why}"))?;
    let pid = child.id();
    let stdout = child.stdout.take();
    let heard = tokio::task::spawn_blocking(move || {
        let mut line = String::new();
        if let Some(stdout) = stdout {
            let _ = std::io::BufReader::new(stdout).read_line(&mut line);
        }
        line
    });
    let ready = match tokio::time::timeout(READY_WITHIN, heard).await {
        Ok(Ok(line)) if line.trim() == READY => Ok(()),
        Ok(Ok(line)) if line.is_empty() => Err(format!(
            "the new build left before it listened: {}",
            child
                .wait()
                .map_or_else(|why| why.to_string(), |status| status.to_string())
        )),
        Ok(Ok(line)) => Err(format!(
            "the new build said {:?}, not that it listens",
            line.trim()
        )),
        Ok(Err(why)) => Err(format!("the new build couldn't be heard: {why}")),
        Err(_) => Err(format!(
            "the new build didn't say it listens within {} s",
            READY_WITHIN.as_secs()
        )),
    };
    // Stopped while the new build started, it may start nothing that
    // outlasts it, as `answer_hand_overs` says.
    let made_main = ready
        .and_then(|()| {
            if stopping.load(Ordering::SeqCst) {
                Err("this relay was stopped as the new build started".to_owned())
            } else {
                Ok(())
            }
        })
        .and_then(|()| notify(&format!("MAINPID={pid}")));
    if let Err(why) = made_main {
        // Killed, its read ends, and nothing is left of it to wait on.
        let _ = child.kill();
        let _ = child.wait();
        return Err(why);
    }
    Ok(pid)
}

/// Answers asks to hand over on `control` until one is taken, then says so
/// on `handed_over`. One that comes once it's `stopping` is refused, as a
/// relay that's stopped may not start another.
async fn answer_hand_overs(
    control: tokio::net::UnixListener,
    shared: Shared,
    stopping: Arc<AtomicBool>,
    handed_over: tokio::sync::oneshot::Sender<()>,
) {
    loop {
        let mut stream = match control.accept().await {
            Ok((stream, _)) => stream,
            Err(why) => {
                eprintln!("model-relay: couldn't take a hand-over's ask: {why}");
                tokio::time::sleep(ASKED_WITHIN).await;
                continue;
            }
        };
        let mut ask = Vec::new();
        let read = tokio::time::timeout(
            ASKED_WITHIN,
            (&mut stream).take(ASK_AT_MOST).read_to_end(&mut ask),
        )
        .await;
        let taken = match read {
            // Asked nothing, as by a look at whether it listens.
            Ok(Ok(0)) => continue,
            Ok(Ok(_)) if stopping.load(Ordering::SeqCst) => {
                Err("this relay is stopping, so it hands over to nothing".to_owned())
            }
            Ok(Ok(_)) => {
                // Its asker waits only `ASKED_WITHIN` for this, and one
                // that has hung up is refused nothing it can still hear.
                let _ = stream.write_all(format!("{ASKED}\n").as_bytes()).await;
                take_over(&ask, &shared, &stopping).await
            }
            Ok(Err(why)) => Err(format!("its ask couldn't be read: {why}")),
            Err(_) => Err(format!(
                "its ask didn't arrive within {} s",
                ASKED_WITHIN.as_secs()
            )),
        };
        let answer = match &taken {
            Ok(pid) => format!("taken by pid {pid}\n"),
            Err(why) => format!("{why}\n"),
        };
        // One that has hung up is told nothing, and its build was started
        // or refused all the same.
        let _ = stream.write_all(answer.as_bytes()).await;
        match taken {
            Ok(pid) => {
                eprintln!("model-relay: a new build took its sockets over, as pid {pid}");
                let _ = handed_over.send(());
                return;
            }
            Err(why) => eprintln!("model-relay: a hand-over was refused: {why}"),
        }
    }
}

/// A number of seconds a flag was given, or `default` when it wasn't.
fn seconds(given: Option<&str>, flag: &str, default: Duration) -> Result<Duration, Failure> {
    given.map_or(Ok(default), |text| {
        text.parse::<u64>()
            .map(Duration::from_secs)
            .map_err(|_| Failure::new(Exit::Usage, format!("{flag} takes whole seconds")))
    })
}

/// Listens and forwards until it's stopped.
fn serve(args: &[String]) -> Result<(), Failure> {
    let given = named(args)?;
    let origin = given[0].as_deref().ok_or_else(|| {
        Failure::new(Exit::Usage, "it needs --origin").try_this("model-relay --help")
    })?;
    let origin = Origin::parse(origin).map_err(|why| {
        Failure::new(Exit::Usage, "its --origin can't be forwarded to")
            .because(why)
            .try_this("give an https origin alone, such as https://models.example")
    })?;
    let connections = match given[2].as_deref() {
        None => CONNECTIONS,
        Some(text) => text
            .parse()
            .ok()
            .filter(|count| (1..=MOST_CONNECTIONS).contains(count))
            .ok_or_else(|| {
                Failure::new(
                    Exit::Usage,
                    format!("--connections takes 1 to {MOST_CONNECTIONS}"),
                )
            })?,
    };
    let roots = match given[3].as_deref() {
        None => Roots::public(),
        Some(file) => Roots::from_pem(Path::new(file))
            .map_err(|why| Failure::new(Exit::Failed, "its roots couldn't be read").because(why))?,
    };
    let connect_within = seconds(given[4].as_deref(), "--connect-within", CONNECT_WITHIN)?;
    if connect_within.is_zero() {
        return Err(Failure::new(
            Exit::Usage,
            "--connect-within takes at least a second",
        ));
    }
    let idle_exit = seconds(given[5].as_deref(), "--idle-exit", IDLE_EXIT)?;
    let drains_within = seconds(given[6].as_deref(), "--drain-within", DRAINS_WITHIN)?;
    // None would cut every answer in hand at a stop, which is what the
    // drain is for.
    if drains_within.is_zero() {
        return Err(Failure::new(
            Exit::Usage,
            "--drain-within takes at least a second",
        ));
    }
    // A build that took over waits out the longest the one it took over
    // from may still answer before it counts itself idle: it's systemd's
    // main process, so its leaving would take the unit down, and with it
    // the answers the other still holds.
    let idle_exit = if given[TAKEN_OVER].is_some() {
        idle_exit.max(HANDED_OVER_ANSWERS_WITHIN)
    } else {
        idle_exit
    };
    let clocks = Clocks {
        connect_within,
        idle_exit: (!idle_exit.is_zero()).then_some(idle_exit),
        drains_within,
    };
    let relay = Relay::new(origin, connections, roots, clocks)
        .map_err(|why| Failure::new(Exit::Failed, "its TLS couldn't be set up").because(why))?;
    answer(relay, &given)
}

/// Answers the sockets `given` names, or those systemd or a relay handing
/// over left it, until `relay` leaves.
fn answer(relay: Relay, given: &[Option<String>]) -> Result<(), Failure> {
    let handed = listening(&Sockets {
        socket: given[1].as_deref(),
        lend: given[7].as_deref(),
        control: given[8].as_deref(),
        taken_over: given[TAKEN_OVER].as_deref(),
    })?;
    // Kept only by a relay that can be asked to hand over.
    let shared = handed
        .control
        .is_some()
        .then(|| Shared::of(&handed))
        .transpose()
        .map_err(|why| unlistened(&why))?;
    let taking_over = given[TAKEN_OVER].is_some();
    let Handed {
        agents: listener,
        lend,
        control,
    } = handed;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|why| Failure::new(Exit::Failed, "it couldn't start").because(why.to_string()))?;
    let served = runtime.block_on(async move {
        let listener =
            tokio::net::UnixListener::from_std(listener).map_err(|why| unlistened(&why))?;
        let lend = lend
            .map(tokio::net::UnixListener::from_std)
            .transpose()
            .map_err(|why| unlistened(&why))?;
        let control = control
            .map(tokio::net::UnixListener::from_std)
            .transpose()
            .map_err(|why| unlistened(&why))?;
        let stopping = Arc::new(AtomicBool::new(false));
        let (handed_over, taken) = tokio::sync::oneshot::channel();
        if let (Some(control), Some(shared)) = (control, shared) {
            tokio::spawn(answer_hand_overs(
                control,
                shared,
                Arc::clone(&stopping),
                handed_over,
            ));
        }
        // systemd stops it with SIGTERM, which it takes as its word to
        // leave once its answers end rather than at once, and a second as
        // its word to leave now. Each is heard for as long as it runs,
        // since taking SIGTERM ends its default of leaving at once.
        let mut terminate =
            tokio::signal::unix::signal(SignalKind::terminate()).map_err(|why| {
                Failure::new(Exit::Failed, "it couldn't hear its stop").because(why.to_string())
            })?;
        // Room for a second stop close behind a first not yet read, since
        // the second says something the first doesn't; a third says nothing
        // new. Stops that come between two reads of `terminate` are heard
        // as one, as tokio's signals are.
        let (stop, stops) = tokio::sync::mpsc::channel(2);
        tokio::spawn(async move {
            while terminate.recv().await.is_some() {
                stopping.store(true, Ordering::SeqCst);
                let _ = stop.try_send(());
            }
        });
        // The relay handing over waits on this to make it the main process,
        // so it's said once every socket is answered and its stop heard.
        if taking_over {
            let mut stdout = std::io::stdout();
            writeln!(stdout, "{READY}")
                .and_then(|()| stdout.flush())
                .map_err(|why| {
                    Failure::new(Exit::Failed, "it couldn't say it listens")
                        .because(why.to_string())
                })?;
        }
        Ok(relay.serve_until(listener, lend, stops, taken).await)
    });
    // Dropped, the runtime would wait on a lookup for as long as it took.
    runtime.shutdown_timeout(LOOKUPS_GIVEN);
    // The answers it cut count only as the runtime drops them, just now.
    served?.say();
    Ok(())
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        match named(&context.args)?[HAND_OVER].as_deref() {
            Some(control) => hand_over(control, &context.args),
            None => serve(&context.args),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::places;

    #[test]
    fn one_socket_unnamed_or_named_by_its_unit_is_pis() {
        for names in ["", "model-relay-agents.socket"] {
            assert_eq!(
                places("1", names),
                Ok([Some(3), None, None]),
                "one socket named {names:?} wasn't taken as pi's"
            );
        }
    }

    #[test]
    fn one_socket_named_the_lends_is_refused_as_leaving_pi_none() {
        let refused = places("1", "lend");
        assert!(
            refused
                .as_ref()
                .is_err_and(|why| why.contains("none of them is pi's")),
            "one socket named the lend's wasn't refused as leaving pi none: {refused:?}"
        );
    }

    #[test]
    fn sockets_are_told_apart_by_their_names_in_any_order() {
        for (count, names, sorted) in [
            (
                "2",
                "model-relay-agents.socket:lend",
                [Some(3), Some(4), None],
            ),
            (
                "2",
                "lend:model-relay-agents.socket",
                [Some(4), Some(3), None],
            ),
            (
                "2",
                "control:model-relay-agents.socket",
                [Some(4), None, Some(3)],
            ),
            (
                "3",
                "control:lend:model-relay-agents.socket",
                [Some(5), Some(4), Some(3)],
            ),
        ] {
            assert_eq!(
                places(count, names),
                Ok(sorted),
                "{count} sockets named {names:?} weren't told apart as pi's, the lend's \
                 and the hand-over's"
            );
        }
    }

    #[test]
    fn sockets_not_one_pis_and_one_of_each_other_are_refused_saying_their_names() {
        for (count, names) in [
            ("2", "a.socket:b.socket"),
            ("2", "lend:lend"),
            ("2", ""),
            ("2", "a.socket:b.socket:lend"),
            ("3", "lend:control:control"),
        ] {
            let refused = places(count, names);
            assert!(
                refused
                    .as_ref()
                    .is_err_and(|why| why.contains(&format!("{names:?}"))),
                "{count} sockets named {names:?} weren't refused saying their names: \
                 {refused:?}"
            );
        }
    }

    #[test]
    fn any_other_count_of_sockets_is_refused_saying_how_many() {
        for count in ["0", "4", "two"] {
            let refused = places(count, "lend");
            assert!(
                refused
                    .as_ref()
                    .is_err_and(|why| why.contains(&format!("handed it {count} sockets"))),
                "{count} sockets weren't refused saying how many: {refused:?}"
            );
        }
    }
}
