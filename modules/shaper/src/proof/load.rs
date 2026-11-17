//! The agents' load a proof puts on the link, and how it ends: at the
//! proof's word, at a signal asking the proof to stop, or on its own once
//! the proof is gone, so a proof stopped any way leaves nothing uploading.

use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

/// Set once a signal asks the proof to stop.
pub static STOPPED: AtomicBool = AtomicBool::new(false);

extern "C" fn heard(_: libc::c_int) {}

/// Hears SIGTERM, SIGINT and SIGHUP as a stop, so the proof puts the
/// shaper back as it was and ends its load before it exits, where it
/// would otherwise die with the shaper off and the uploads going on.
pub fn hear_stops() {
    // SAFETY: the handler only stores to an atomic.
    unsafe {
        let handler = heard as *const () as libc::sighandler_t;
        for signal in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
            libc::signal(signal, handler);
        }
    }
}

/// One upload's loop, posting 400 MB to `server` again as each ends.
fn script(server: &str, _pid: u32) -> String {
    format!(
        "while :; do head -c 400000000 /dev/zero | curl -s -o /dev/null -X POST -T - \
         --max-time 120 '{server}/__up'; done"
    )
}

/// Starts the agents' uploads, from jobs.slice.
pub fn start(uploads: u32, server: &str) {
    let script = script(server, std::process::id());
    for n in 0..uploads {
        let _ = super::scoped("jobs.slice", &format!("load{n}"), &["sh", "-c", &script])
            .stdout(Stdio::null())
            .spawn();
    }
}

/// Stops every scope this proof started.
pub fn stop_scopes() {
    let _ = Command::new("systemctl")
        .args([
            "--user",
            "stop",
            &format!("shaper-proof-*-{}.scope", std::process::id()),
        ])
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::{Duration, Instant};

    /// Whether `script` ends by itself within `within`; it's killed if not.
    fn ends_within(script: &str, within: Duration) -> bool {
        let mut sh = Command::new("sh")
            .args(["-c", script])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("a shell");
        let began = Instant::now();
        while began.elapsed() < within {
            if sh.try_wait().ok().flatten().is_some() {
                return true;
            }
            thread::sleep(Duration::from_millis(50));
        }
        let _ = sh.kill();
        let _ = sh.wait();
        false
    }

    #[test]
    fn a_load_ends_once_its_proof_is_gone() {
        let mut gone = Command::new("true").spawn().expect("true");
        let pid = gone.id();
        gone.wait().expect("true to end");
        let nowhere = "http://127.0.0.1:1";
        assert!(
            ends_within(&script(nowhere, pid), Duration::from_secs(3)),
            "uploading on after its proof ended"
        );
        assert!(
            !ends_within(&script(nowhere, std::process::id()), Duration::from_secs(1)),
            "ended while its proof runs"
        );
    }

    #[test]
    fn a_stop_signal_is_heard_as_a_stop() {
        hear_stops();
        // SAFETY: raising a signal whose handler was just set.
        unsafe {
            libc::raise(libc::SIGHUP);
        }
        assert!(STOPPED.load(Ordering::Relaxed), "a SIGHUP went unheard");
    }
}
