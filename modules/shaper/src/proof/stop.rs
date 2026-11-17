//! How a proof stops and puts the shaper back: a signal heard as a stop
//! by every thread, progress said without dying when nothing reads it,
//! and the shaper's own state kept on disk until it's back, so a proof
//! killed outright leaves the next one what it found.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

/// Set once a signal asks the proof to stop.
pub static STOPPED: AtomicBool = AtomicBool::new(false);

extern "C" fn heard(_: libc::c_int) {
    if STOPPED.swap(true, Ordering::Relaxed) {
        const AGAIN: &[u8] = b"already stopping: putting the shaper back, ending the load\n";
        // SAFETY: write(2) is async-signal-safe, and AGAIN outlives it.
        unsafe {
            libc::write(2, AGAIN.as_ptr().cast(), AGAIN.len());
        }
    }
}

/// Hears SIGTERM, SIGINT and SIGHUP as a stop, so the proof puts the
/// shaper back as it was and ends its load before it exits, where it
/// would otherwise die with the shaper off and the uploads going on. A
/// signal already ignored, as nohup ignores SIGHUP, stays ignored.
pub fn hear_stops() {
    // SAFETY: the handler only touches an atomic and calls write(2), and
    // a disposition put back as it was found is no change.
    unsafe {
        let handler = heard as *const () as libc::sighandler_t;
        for signal in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
            if libc::signal(signal, handler) == libc::SIG_IGN {
                libc::signal(signal, libc::SIG_IGN);
            }
        }
    }
}

/// Whether the window's `stop` or a signal says to stop. The main thread
/// asks both, since the watch that passes a signal on may be stuck asking
/// the gate.
pub fn halted(stop: &AtomicBool) -> bool {
    stop.load(Ordering::Relaxed) || STOPPED.load(Ordering::Relaxed)
}

/// Says `line` on stderr, if anything still reads it: after a SIGHUP from
/// a closed terminal, `eprintln!` would panic and leave the shaper off.
pub fn tell(line: &str) {
    let _ = writeln!(std::io::stderr(), "{line}");
}

pub fn systemctl(verb: &str) -> bool {
    Command::new("systemctl")
        .args([verb, "shaper.service"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Puts the shaper back `on` or off, and says whether it is, asking
/// three times a second apart, since a start may still be under way.
pub fn put_back(on: bool) -> bool {
    for _ in 0..3 {
        systemctl(if on { "start" } else { "stop" });
        if systemctl("is-active") == on {
            return true;
        }
        thread::sleep(Duration::from_secs(1));
    }
    false
}

/// Where the shaper's own state waits while a proof may have it off:
/// the runtime folder, since a boot starts the shaper afresh. With none,
/// there's nowhere private to keep it, and it isn't kept.
pub fn marker() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR").map(|d| PathBuf::from(d).join("shaper-proof-was-on"))
}

/// The shaper's own state: what `marker` holds, when a proof killed
/// outright left it there and maybe left the shaper off, else `active`.
/// It stays in `marker` until `forget`, once the shaper is back.
pub fn own_state(marker: &Path, active: bool) -> bool {
    let earlier = fs::read_to_string(marker)
        .ok()
        .and_then(|s| match s.trim() {
            "on" => Some(true),
            "off" => Some(false),
            _ => None,
        });
    if let Some(on) = earlier {
        tell(&format!(
            "an earlier proof was killed outright: the shaper goes back {} after this one",
            if on { "on" } else { "off" }
        ));
    }
    let on = earlier.unwrap_or(active);
    if fs::write(marker, if on { "on" } else { "off" }).is_err() {
        tell(&format!("couldn't write {}", marker.display()));
    }
    on
}

pub fn forget(marker: &Path) {
    let _ = fs::remove_file(marker);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stop_signal_is_heard_as_a_stop() {
        // Under nohup a SIGHUP is ignored, and stays so.
        // SAFETY: setting a disposition, then raising what it ignores.
        unsafe {
            libc::signal(libc::SIGHUP, libc::SIG_IGN);
            hear_stops();
            libc::raise(libc::SIGHUP);
        }
        assert!(
            !STOPPED.load(Ordering::Relaxed),
            "a SIGHUP nohup ignores stopped it"
        );
        // SAFETY: back to the default, so the stops below are heard.
        unsafe {
            libc::signal(libc::SIGHUP, libc::SIG_DFL);
        }
        hear_stops();
        for signal in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
            STOPPED.store(false, Ordering::Relaxed);
            // SAFETY: raising a signal whose handler was just set.
            unsafe {
                libc::raise(signal);
            }
            assert!(
                STOPPED.load(Ordering::Relaxed),
                "signal {signal} went unheard"
            );
            assert!(halted(&AtomicBool::new(false)), "a signal didn't halt");
        }
        // SAFETY: a second signal only writes; then each goes back to
        // its default, so the rest of the tests hear Ctrl-C as ever.
        unsafe {
            libc::raise(libc::SIGTERM);
            for signal in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
                libc::signal(signal, libc::SIG_DFL);
            }
        }
        STOPPED.store(false, Ordering::Relaxed);
    }

    #[test]
    fn a_killed_proof_leaves_the_next_what_it_found() {
        let dir = std::env::temp_dir().join(format!("shaper-own-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a folder");
        let marker = dir.join("was-on");
        assert!(
            own_state(&marker, true),
            "the shaper's own state, with no proof before"
        );
        // A proof killed in an off arm: the next finds the shaper off.
        assert!(
            own_state(&marker, false),
            "took the killed proof's off as its own"
        );
        forget(&marker);
        assert!(
            !own_state(&marker, false),
            "remembered a proof that put it back"
        );
        forget(&marker);
        // A marker that says neither, as a write cut short leaves, is
        // nobody's word: the shaper's own state stands.
        fs::write(&marker, "").expect("an empty marker");
        assert!(own_state(&marker, true), "took an empty marker for off");
        forget(&marker);
        let _ = fs::remove_dir_all(&dir);
    }
}
