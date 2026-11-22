//! `model-relay` answers pi's model requests on a socket and carries them
//! to their one origin over a few long-lived HTTP/2 connections
//! (src/lib.rs says why).
//!
//! systemd hands it its listening socket (`LISTEN_FDS`); `--socket` binds
//! one itself, as the tests do. It takes the shape every command of ours
//! does (docs/commands.md): its help, usage that says what to try, and
//! failures with named exits.

use std::os::fd::FromRawFd;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, PermissionsExt};
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use model_relay::{CONNECT_WITHIN, Clocks, IDLE_EXIT, Origin, Relay, Roots};
use terminal_kit::{Exit, Failure, Help, flags};
use tokio::signal::unix::SignalKind;

/// The named flags it reads, every one taking a value.
const FLAGS: [&str; 6] = [
    "--origin",
    "--socket",
    "--connections",
    "--roots",
    "--connect-within",
    "--idle-exit",
];

/// How many connections it keeps to the origin unless told.
const CONNECTIONS: usize = 4;
/// The most it may be told to keep: past a handful, the flows share the
/// uplink as thinly as the separate sessions did.
const MOST_CONNECTIONS: usize = 16;

/// The first descriptor systemd hands a service it starts for a socket.
const LISTEN_FDS_START: i32 = 3;

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
        "how many connections to the origin it keeps, 4 if not told",
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
    .exit(
        Exit::Failed,
        "it couldn't listen or hear its stop, or its roots couldn't be read",
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

/// The socket systemd handed it, when it was started for one.
fn handed() -> Option<std::os::unix::net::UnixListener> {
    let ours = std::env::var("LISTEN_PID").ok()? == std::process::id().to_string();
    let one = std::env::var("LISTEN_FDS").ok()? == "1";
    // SAFETY: systemd passed this descriptor, open and listening, for this
    // process alone (LISTEN_PID), and nothing else here takes it.
    (ours && one)
        .then(|| unsafe { std::os::unix::net::UnixListener::from_raw_fd(LISTEN_FDS_START) })
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
    let clocks = Clocks {
        connect_within,
        idle_exit: (!idle_exit.is_zero()).then_some(idle_exit),
    };
    let relay = Relay::new(origin, connections, roots, clocks)
        .map_err(|why| Failure::new(Exit::Failed, "its TLS couldn't be set up").because(why))?;
    let listener = match (handed(), given[1].as_deref()) {
        (Some(listener), _) => listener,
        (None, Some(path)) => bound(Path::new(path))?,
        (None, None) => {
            return Err(Failure::new(
                Exit::Usage,
                "systemd handed it no socket, and it was given none",
            )
            .try_this("model-relay --origin URL --socket PATH"));
        }
    };
    listener
        .set_nonblocking(true)
        .map_err(|why| Failure::new(Exit::Failed, "it couldn't listen").because(why.to_string()))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|why| Failure::new(Exit::Failed, "it couldn't start").because(why.to_string()))?;
    runtime.block_on(async move {
        let listener = tokio::net::UnixListener::from_std(listener).map_err(|why| {
            Failure::new(Exit::Failed, "it couldn't listen").because(why.to_string())
        })?;
        // systemd stops it with SIGTERM, which it takes as its word to
        // leave once its answers end rather than at once.
        let mut terminate =
            tokio::signal::unix::signal(SignalKind::terminate()).map_err(|why| {
                Failure::new(Exit::Failed, "it couldn't hear its stop").because(why.to_string())
            })?;
        relay
            .serve_until(listener, async move {
                terminate.recv().await;
            })
            .await;
        Ok(())
    })
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        serve(&context.args)
    })
}
