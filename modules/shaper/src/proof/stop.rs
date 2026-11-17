//! How a proof stops and puts the shaper back: a signal heard as a stop
//! by every thread, progress said without dying when nothing reads it,
//! and the shaper's own state kept on disk until it's back, so a keeper
//! beside a proof killed outright, or failing that the next proof, puts
//! it back as the killed one found it.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// How long a killed proof's keeper is waited for.
const KEEPER_TAKES: Duration = Duration::from_secs(3);

/// Set once a signal asks the proof to stop.
pub static STOPPED: AtomicBool = AtomicBool::new(false);

extern "C" fn heard(signal: libc::c_int) {
    if signal == libc::SIGTSTP {
        // A Ctrl-Z stops the whole of the terminal's group, and so any
        // child of the proof's that shares it, which the proof would wait
        // on for good: each is woken to finish.
        // SAFETY: kill(2) is async-signal-safe.
        unsafe {
            libc::kill(0, libc::SIGCONT);
        }
    }
    if STOPPED.swap(true, Ordering::Relaxed) && signal != libc::SIGCONT {
        const AGAIN: &[u8] = b"already stopping: putting the shaper back, ending the load\n";
        // SAFETY: write(2) is async-signal-safe, and AGAIN outlives it.
        unsafe {
            libc::write(2, AGAIN.as_ptr().cast(), AGAIN.len());
        }
    }
}

/// The signals heard as a stop.
pub const STOPS: [libc::c_int; 5] = [
    libc::SIGTERM,
    libc::SIGINT,
    libc::SIGHUP,
    libc::SIGTSTP,
    libc::SIGCONT,
];

/// Hears SIGTERM, SIGINT, SIGHUP and SIGTSTP as a stop, so the proof puts
/// the shaper back as it was and ends its load before it exits, where it
/// would otherwise die with the shaper off and the uploads going on, or,
/// at a Ctrl-Z, sleep while they go on. SIGCONT is one too: a proof
/// paused by SIGSTOP counts as gone, so its load has ended and its keeper
/// has put the shaper back, and resumed it would measure nothing real. A
/// signal already ignored, as nohup ignores SIGHUP, stays ignored.
pub fn hear_stops() {
    // SAFETY: the handler only touches an atomic and calls write(2), and
    // a disposition put back as it was found is no change.
    unsafe {
        let handler = heard as *const () as libc::sighandler_t;
        for signal in STOPS {
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

/// Asks systemd to `verb` the shaper, in a process group of its own, so
/// a Ctrl-C at the terminal can't cut it short and have an `is-active`
/// read as off.
pub fn systemctl(verb: &str) -> bool {
    use std::os::unix::process::CommandExt;
    Command::new("systemctl")
        .args([verb, "shaper.service"])
        .process_group(0)
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
    // A killed proof's keeper puts the shaper back within about a second,
    // and forgets the marker: waited for, so its word isn't taken for the
    // shaper's own once it's back.
    let asked = Instant::now();
    while marker.exists() && asked.elapsed() < KEEPER_TAKES {
        thread::sleep(Duration::from_millis(100));
    }
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

/// Holds the lock beside `marker` for as long as the file it gives is
/// open, however the proof ends, or None while another proof holds it:
/// two at once would load the link twice, and each would take the
/// other's shaper for its own.
pub fn alone(marker: &Path) -> Option<fs::File> {
    use std::os::fd::AsRawFd;
    let lock = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(marker.with_file_name("shaper-proof.lock"))
        .ok()?;
    // SAFETY: flock(2) on a file this holds open.
    let held = unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0;
    held.then_some(lock)
}

pub fn forget(marker: &Path) {
    let _ = fs::remove_file(marker);
}

/// Starts a keeper beside this proof, in a scope of its own named apart
/// from the proof's, which the proof's stops leave alone. Once the proof
/// is gone, however it went, the keeper puts the shaper back as `marker`
/// says and forgets it, so one killed outright leaves the shaper as it
/// found it within about a second. A proof that put it back has
/// forgotten the marker already, and its keeper does nothing.
pub fn keep(marker: &Path) {
    let Some(at) = marker.to_str() else {
        return;
    };
    let me = std::process::id();
    let script = keeper(me, &super::load::began());
    let unit = format!("shaper-keeper-{me}");
    let command = ["sh", "-c", &script, "sh", at, "systemctl"];
    let unkept = "a proof killed outright would leave the shaper off";
    match super::scoped_as("app.slice", &unit, &command)
        .stdout(Stdio::null())
        .spawn()
    {
        Err(_) => tell(&format!("couldn't start the keeper: {unkept}")),
        // It ends only once the proof is gone, so one ended already is
        // systemd-run refusing the scope.
        Ok(mut keeper) => {
            thread::sleep(Duration::from_millis(300));
            if keeper.try_wait().is_ok_and(|ended| ended.is_some()) {
                tell(&format!("the keeper's scope was refused: {unkept}"));
            }
        }
    }
}

/// The keeper's shell for the proof `pid` that began at `began`: given
/// the marker and systemctl as its arguments, it waits for the proof to
/// go, then starts or stops the shaper as the marker says, and forgets
/// it once that's done. A marker saying neither is nobody's word.
fn keeper(pid: u32, began: &str) -> String {
    super::load::once_gone_for(
        pid,
        began,
        "v=; read -r v 2>/dev/null <\"$1\"; case $v in on) v=start;; off) v=stop;; \
         *) exit 0;; esac; \"$2\" \"$v\" shaper.service && rm -f \"$1\"",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, PoisonError};

    /// Held by each test that sets or sends a signal the process hears,
    /// since handlers and STOPPED are the whole test binary's.
    static SIGNALS: Mutex<()> = Mutex::new(());

    fn signals() -> MutexGuard<'static, ()> {
        SIGNALS.lock().unwrap_or_else(PoisonError::into_inner)
    }

    #[test]
    fn a_stop_signal_is_heard_as_a_stop() {
        let _signals = signals();
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
        for signal in STOPS {
            // Asked first, since a Ctrl-Z unheard would stop the tests.
            // SAFETY: sigaction(2) with no new action only reads one.
            let set = unsafe {
                let mut was: libc::sigaction = std::mem::zeroed();
                libc::sigaction(signal, std::ptr::null(), &raw mut was);
                was.sa_sigaction
            };
            assert_eq!(
                set, heard as *const () as libc::sighandler_t,
                "signal {signal} isn't heard as a stop"
            );
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
            for signal in STOPS {
                libc::signal(signal, libc::SIG_DFL);
            }
        }
        STOPPED.store(false, Ordering::Relaxed);
    }

    #[test]
    fn the_keeper_puts_the_shaper_back_once_its_proof_is_gone() {
        let dir = std::env::temp_dir().join(format!("shaper-keeper-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a folder");
        let marker = dir.join("was-on");
        let at = marker.to_str().expect("a path");
        // What it asks, with echo standing in for systemctl.
        let kept = |pid: u32, began: &str| {
            let out = Command::new("sh")
                .args(["-c", &keeper(pid, began), "sh", at, "echo"])
                .output()
                .expect("a shell");
            String::from_utf8_lossy(&out.stdout).into_owned()
        };
        let mut gone = Command::new("true").spawn().expect("true");
        let pid = gone.id();
        let began = super::super::load::began_of(&pid.to_string());
        gone.wait().expect("true to end");
        for (was, asks) in [
            ("on", "start shaper.service\n"),
            ("off", "stop shaper.service\n"),
        ] {
            fs::write(&marker, was).expect("a marker");
            assert_eq!(kept(pid, &began), asks, "the shaper wasn't put back {was}");
            assert!(!marker.exists(), "the marker outlived putting it back");
        }
        // A shaper that wouldn't go back keeps its marker, for the next.
        fs::write(&marker, "on").expect("a marker");
        let failing = Command::new("sh")
            .args(["-c", &keeper(pid, &began), "sh", at, "false"])
            .status()
            .expect("a shell");
        assert!(!failing.success(), "a keeper that failed said it hadn't");
        assert!(marker.exists(), "forgot a shaper it couldn't put back");
        forget(&marker);
        // A proof that put it back left nothing to do.
        assert_eq!(kept(pid, &began), "", "acted on no marker");
        fs::write(&marker, "").expect("an empty marker");
        assert_eq!(kept(pid, &began), "", "took an empty marker for a state");
        let _ = fs::remove_dir_all(&dir);
    }

    /// The state `pid` is in, as /proc tells it.
    fn state_of(pid: u32) -> String {
        fs::read_to_string(format!("/proc/{pid}/stat"))
            .ok()
            .and_then(|s| Some(s.rsplit_once(") ")?.1.split(' ').next()?.to_string()))
            .unwrap_or_default()
    }

    #[test]
    fn a_ctrl_z_wakes_what_it_stopped_beside_the_proof() {
        // A child in the proof's own group, as systemctl or pw-dump is,
        // stopped by the terminal's Ctrl-Z, would hold the proof for good.
        let _signals = signals();
        let mut child = Command::new("sleep").arg("5").spawn().expect("sleep");
        let pid = child.id();
        let id = libc::pid_t::try_from(pid).expect("a pid");
        // SAFETY: stopping this test's own child.
        unsafe {
            libc::kill(id, libc::SIGSTOP);
        }
        let stopped = std::time::Instant::now();
        while state_of(pid) != "T" && stopped.elapsed() < Duration::from_secs(2) {
            thread::sleep(Duration::from_millis(10));
        }
        let was = STOPPED.load(Ordering::Relaxed);
        heard(libc::SIGTSTP);
        STOPPED.store(was, Ordering::Relaxed);
        let woke = std::time::Instant::now();
        while state_of(pid) == "T" && woke.elapsed() < Duration::from_secs(1) {
            thread::sleep(Duration::from_millis(10));
        }
        let state = state_of(pid);
        let _ = child.kill();
        let _ = child.wait();
        assert_ne!(state, "T", "a Ctrl-Z left the proof's child stopped");
    }

    #[test]
    fn a_proof_resumed_from_a_pause_stops() {
        // Paused by SIGSTOP, which nothing hears, the proof counts as gone:
        // its load ends and its keeper puts the shaper back. Resumed, it
        // would measure on with no load.
        let _signals = signals();
        hear_stops();
        // SAFETY: sigaction(2) with no new action only reads one.
        let set = unsafe {
            let mut was: libc::sigaction = std::mem::zeroed();
            libc::sigaction(libc::SIGCONT, std::ptr::null(), &raw mut was);
            was.sa_sigaction
        };
        // SAFETY: back to the default, as the other tests expect.
        unsafe {
            for signal in STOPS {
                libc::signal(signal, libc::SIG_DFL);
            }
        }
        assert_eq!(
            set, heard as *const () as libc::sighandler_t,
            "a resumed proof measures on"
        );
    }

    #[test]
    fn only_one_proof_runs_at_once() {
        let dir = std::env::temp_dir().join(format!("shaper-alone-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a folder");
        let marker = dir.join("was-on");
        let first = alone(&marker).expect("the first proof");
        assert!(
            alone(&marker).is_none(),
            "a second proof ran beside the first"
        );
        drop(first);
        assert!(alone(&marker).is_some(), "a proof gone still held the next");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_keeper_still_putting_the_shaper_back_is_waited_for() {
        let dir = std::env::temp_dir().join(format!("shaper-wait-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a folder");
        let marker = dir.join("was-on");
        fs::write(&marker, "off").expect("a marker");
        // The killed proof's keeper, putting the shaper back off and
        // forgetting it: the shaper's state then is its own again.
        let keeping = marker.clone();
        let keeper = thread::spawn(move || {
            thread::sleep(Duration::from_millis(300));
            forget(&keeping);
        });
        assert!(
            own_state(&marker, true),
            "took a marker its keeper was forgetting"
        );
        keeper.join().expect("the keeper");
        let _ = fs::remove_dir_all(&dir);
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
