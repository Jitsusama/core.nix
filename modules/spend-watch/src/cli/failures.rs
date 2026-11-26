//! How the command line tells what went wrong: which exit each failure
//! takes, and what it says to try. A passing state, another watch or a
//! busy history, is a try-again, never a failure.

use crate::config::Config;
use terminal_kit::{Exit, Failure};

/// The history couldn't be read: for now, while another connection
/// writes it, or for good.
pub(super) fn unread(why: String) -> Failure {
    if crate::daemon::busy(&why) {
        return Failure::new(Exit::TryAgain, "the history is busy")
            .because(why)
            .try_this("the same again in a moment");
    }
    Failure::new(Exit::Failed, "the history couldn't be read").because(why)
}

/// Something the daemon does couldn't be done.
pub(super) fn broke(why: String) -> Failure {
    Failure::new(Exit::Failed, "spend-watch couldn't carry on").because(why)
}

/// The watch stopped. A lock still held when its wait ran out, or a
/// history still busy then, is for a restart to try again, as it was
/// before the watch waited and as a reader's busy history is.
pub(super) fn stopped(why: String) -> Failure {
    if why.starts_with("another spend-watch holds") {
        return Failure::new(Exit::TryAgain, "another spend-watch holds the history")
            .because(why)
            .try_this("systemctl --user status spend-watch");
    }
    if crate::daemon::busy(&why) {
        return Failure::new(Exit::TryAgain, "the history stayed busy")
            .because(why)
            .try_this("systemctl --user status spend-watch");
    }
    broke(why)
}

/// The answer couldn't be written: its reader went, or the disk is full.
pub(super) fn wrote(written: std::io::Result<()>) -> Result<(), Failure> {
    written.map_err(|error| {
        Failure::new(Exit::Failed, "the answer couldn't be written").because(error.to_string())
    })
}

/// Nothing while no other watch answers on the socket. One that does is
/// the running watch, and a second is told so at once; a reader holding
/// the lock as it moves the history answers nothing there, so the watch
/// waits for it instead, as `daemon::run` does.
pub(super) fn running(config: &Config) -> Result<(), Failure> {
    if std::os::unix::net::UnixStream::connect(config.socket()).is_ok() {
        return Err(
            Failure::new(Exit::TryAgain, "another spend-watch is running")
                .because(format!("{} answers", config.socket().display()))
                .try_this("systemctl --user status spend-watch"),
        );
    }
    Ok(())
}

/// The history's lock, taken while no other spend-watch holds it.
pub(super) fn held(config: &Config) -> Result<std::fs::File, Failure> {
    crate::daemon::lock(&config.state).map_err(|why| {
        Failure::new(Exit::TryAgain, "another spend-watch holds the history")
            .because(why)
            .try_this("systemctl --user status spend-watch")
    })
}
