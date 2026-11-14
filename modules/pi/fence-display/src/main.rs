//! The display a fenced pi command reaches: a Wayland socket the
//! compositor treats as a sandbox's.
//!
//! The agent harness's fence keeps out the compositor's own socket, since
//! any client on it can record the screen, type and move the pointer into
//! any window, and read every copy as it's made. Yet a fenced command may
//! need to open a window of its own, as core's shell-preview does with a
//! nested niri. So this makes a second socket through
//! wp-security-context-v1 and hands its listening end to the compositor,
//! which accepts every client on it as a sandboxed one. niri then hides
//! the privileged protocols from those clients: data-control, screencopy,
//! the virtual keyboard and pointer, foreign toplevels, output and
//! workspace management, layer shell, the session lock, input methods,
//! gamma control and security-context itself. The fence binds in only
//! this socket's folder and points `WAYLAND_DISPLAY` at it.
//!
//! It never falls back to anything: a compositor without the protocol
//! makes it fail, and no socket is made. The socket lasts while this
//! runs, since the compositor stops accepting on it once the pipe this
//! holds closes, as the protocol has it; systemd starts it again when it
//! ends, binding the socket afresh.
//!
//! Its help and refusals are the terminal kit's (docs/commands.md); what
//! it says as it runs goes to the journal, through stderr.

use std::fs;
use std::io;
use std::os::fd::AsFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

use terminal_kit::{Exit, Failure, Help};

use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_registry;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::wp::security_context::v1::client::{
    wp_security_context_manager_v1::WpSecurityContextManagerV1,
    wp_security_context_v1::WpSecurityContextV1,
};

/// The sandbox engine the compositor is told its clients run under.
const ENGINE: &str = "pi-fence";

/// The application its clients are said to be.
const APP_ID: &str = "pi.fence";

/// Only the account reads or connects to the socket, as its folder has it
/// too.
const SOCKET_MODE: u32 = 0o600;

fn help() -> Help {
    Help::new(
        "pi-fence-display",
        "makes a Wayland socket the compositor treats as a sandbox's, for pi's fenced commands",
    )
    .option("--socket PATH", "where the socket is made, afresh")
    .example(
        "pi-fence-display --socket wayland",
        "makes the socket and holds it until the compositor goes",
    )
    .exit(Exit::Usage, "no --socket, or a word it doesn't take")
    .exit(
        Exit::Unavailable,
        "the compositor can't be reached, has no wp-security-context-v1, or went",
    )
}

fn main() -> std::process::ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let socket = socket(&context.args)
            .map_err(|why| Failure::new(Exit::Usage, why).try_this("pi-fence-display --help"))?;
        run(&socket).map_err(|why| Failure::new(Exit::Unavailable, why))
    })
}

/// The socket the command line names.
fn socket(args: &[String]) -> Result<PathBuf, String> {
    let mut socket = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--socket" => socket = Some(PathBuf::from(args.next().ok_or("--socket takes a path")?)),
            _ => return Err(format!("it takes no {arg}")),
        }
    }
    socket.ok_or_else(|| "--socket is needed".to_owned())
}

/// Make the sandboxed socket at `path` and hold it open until the
/// compositor goes.
fn run(path: &Path) -> Result<(), String> {
    let connection = Connection::connect_to_env()
        .map_err(|error| format!("the compositor can't be reached: {error}"))?;
    let (globals, mut queue) = registry_queue_init::<State>(&connection)
        .map_err(|error| format!("the compositor's globals can't be read: {error}"))?;
    let handle = queue.handle();
    let manager: WpSecurityContextManagerV1 = globals
        .bind(&handle, 1..=1, ())
        .map_err(|error| format!("the compositor offers no wp-security-context-v1: {error}"))?;
    let listener =
        bound(path).map_err(|error| format!("{} can't be bound: {error}", path.display()))?;
    let (close_read, close_write) =
        io::pipe().map_err(|error| format!("no pipe to hold the socket open with: {error}"))?;
    let context = manager.create_listener(listener.as_fd(), close_read.as_fd(), &handle, ());
    context.set_sandbox_engine(ENGINE.to_owned());
    context.set_app_id(APP_ID.to_owned());
    context.commit();
    let mut state = State;
    queue
        .roundtrip(&mut state)
        .map_err(|error| format!("the compositor refused the socket: {error}"))?;
    // The compositor holds its own copies of both now; only the pipe's
    // other end, which says the socket is still wanted, stays here.
    drop(listener);
    drop(close_read);
    context.destroy();
    manager.destroy();
    eprintln!(
        "pi-fence-display: sandboxed clients connect at {}",
        path.display()
    );
    loop {
        if let Err(error) = queue.blocking_dispatch(&mut state) {
            drop(close_write);
            return Err(format!("the compositor went: {error}"));
        }
    }
}

/// A listening socket at `path`, made afresh, only the account's.
fn bound(path: &Path) -> io::Result<UnixListener> {
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let listener = UnixListener::bind(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(SOCKET_MODE))?;
    Ok(listener)
}

/// Nothing the compositor says here needs keeping.
struct State;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WpSecurityContextManagerV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &WpSecurityContextManagerV1,
        _: <WpSecurityContextManagerV1 as Proxy>::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WpSecurityContextV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &WpSecurityContextV1,
        _: <WpSecurityContextV1 as Proxy>::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::process;

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn the_socket_is_named_or_refused() {
        assert_eq!(
            socket(&words("--socket /run/x")),
            Ok(PathBuf::from("/run/x"))
        );
        for line in ["", "--socket", "--raw /run/x"] {
            assert!(socket(&words(line)).is_err(), "{line:?} was taken");
        }
    }

    #[test]
    fn a_socket_is_bound_afresh_and_only_the_accounts() {
        let path = env::temp_dir().join(format!("pi-fence-display-{}", process::id()));
        fs::write(&path, b"stale").unwrap();
        let _listener = bound(&path).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, SOCKET_MODE);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn with_no_compositor_no_socket_is_made() {
        let path = env::temp_dir().join(format!("pi-fence-display-none-{}", process::id()));
        // SAFETY: the one test here touching the environment.
        unsafe {
            env::set_var("WAYLAND_DISPLAY", "/nonexistent/wayland-none");
            env::remove_var("WAYLAND_SOCKET");
        }
        let said = run(&path).unwrap_err();
        assert!(said.contains("can't be reached"), "{said}");
        assert!(!path.exists(), "a socket was made with no compositor");
    }
}
