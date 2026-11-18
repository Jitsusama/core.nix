//! How a proof stops and puts the shaper back: a signal heard as a stop
//! by every thread, progress said without dying when nothing reads it,
//! and the shaper's state kept on disk while a window owns it, so a
//! keeper beside a proof killed outright, or failing that the next proof,
//! puts it back as the killed one's window found it.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// How long a killed proof's keeper is waited for.
const KEEPER_TAKES: Duration = Duration::from_secs(3);

/// How long a killed proof's word stands when its keeper couldn't act,
/// from about when it died: a window freshens its word every
/// `FRESHENS`, and a keeper that fails freshens it once more. Past it,
/// Joel or a switch has likely set the shaper as they mean it since, and
/// putting it back would undo them.
const DEBT_STANDS: Duration = Duration::from_mins(30);

/// How often a window freshens its word, so its age says how long ago
/// the proof died, not when the window began.
const FRESHENS: Duration = Duration::from_mins(1);

/// How long a shaper starting or stopping is waited for before its state
/// is read: a crash's restart waits 2 s, and a switch's is quicker.
const SETTLES: Duration = Duration::from_secs(5);

/// Set once a signal asks the proof to stop.
pub static STOPPED: AtomicBool = AtomicBool::new(false);

extern "C" fn heard(signal: libc::c_int) {
    if STOPPED.swap(true, Ordering::Relaxed) {
        const AGAIN: &[u8] = b"already stopping: putting the shaper back, ending the load; \
            a SIGKILL leaves that to its keeper\n";
        // SAFETY: write(2) is async-signal-safe, and AGAIN outlives it.
        unsafe {
            libc::write(2, AGAIN.as_ptr().cast(), AGAIN.len());
        }
    }
    if signal == libc::SIGTSTP {
        // A Ctrl-Z stops the whole of the terminal's group, and so any
        // child of the proof's that shares it, which the proof would wait
        // on for good: each is woken to finish. The proof is woken too,
        // once it's already stopping, which is why this comes last.
        // SAFETY: kill(2) is async-signal-safe.
        unsafe {
            libc::kill(0, libc::SIGCONT);
        }
    }
}

/// A resume is a stop. Paused by SIGSTOP, which nothing hears, the proof
/// counts as gone, so its uploads end and its keeper may put the shaper
/// back; resumed, it would measure on with no load, or put the shaper
/// back again and again. Which of them acted can't be told for certain,
/// so a `fg` of a proof that was never paused stops it too, cleanly.
extern "C" fn resumed(_: libc::c_int) {
    STOPPED.store(true, Ordering::Relaxed);
}

/// The signals heard as a stop.
pub const STOPS: [libc::c_int; 4] = [libc::SIGTERM, libc::SIGINT, libc::SIGHUP, libc::SIGTSTP];

/// Hears SIGTERM, SIGINT, SIGHUP and SIGTSTP as a stop, so the proof puts
/// the shaper back as it was and ends its load before it exits, where it
/// would otherwise die with the shaper off and the uploads going on, or,
/// at a Ctrl-Z, sleep while they go on. A signal already ignored, as
/// nohup ignores SIGHUP, stays ignored. A SIGCONT is a stop too, as
/// `resumed` says.
pub fn hear_stops() {
    // SAFETY: the handlers only touch an atomic and call write(2) and
    // kill(2), and a disposition put back as it was found is no change.
    unsafe {
        let handler = heard as *const () as libc::sighandler_t;
        for signal in STOPS {
            if libc::signal(signal, handler) == libc::SIG_IGN {
                libc::signal(signal, libc::SIG_IGN);
            }
        }
        libc::signal(libc::SIGCONT, resumed as *const () as libc::sighandler_t);
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
/// read as off. It asks no password: from a group in the background, a
/// prompt reading the terminal would stop it for good, so an account
/// polkit doesn't let through fails at once, and the proof says so.
pub fn systemctl(verb: &str) -> bool {
    use std::os::unix::process::CommandExt;
    Command::new("systemctl")
        .args(["--no-ask-password", verb, "shaper.service"])
        .process_group(0)
        .stdin(Stdio::null())
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
        if shaper_on() == Some(on) {
            return true;
        }
        thread::sleep(Duration::from_secs(1));
    }
    false
}

/// Whether the shaper is on, as systemd has it once any start, stop or
/// restart under way is done, or None when that can't be told.
/// `is-active` reads a shaper restarting after a crash or at a switch as
/// off, and a window that took that for its state would stop it for good.
pub fn shaper_on() -> Option<bool> {
    settled(unit_state, SETTLES, Duration::from_millis(250))
}

/// What systemd says of the shaper: its `ActiveState`, and whether a job
/// on it is under way; None when systemd doesn't answer.
fn unit_state() -> Option<(String, bool)> {
    use std::os::unix::process::CommandExt;
    let out = Command::new("systemctl")
        .args([
            "show",
            "--property=ActiveState",
            "--property=Job",
            "shaper.service",
        ])
        .process_group(0)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|out| out.status.success())?;
    let said = String::from_utf8_lossy(&out.stdout).into_owned();
    let value = |key: &str| {
        said.lines()
            .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
            .map(str::trim)
    };
    let active = value("ActiveState")?.to_string();
    Some((active, value("Job").is_some_and(|job| !job.is_empty())))
}

/// Reads `state` every `every` until the shaper is neither starting nor
/// stopping and no job on it is under way, or `takes` runs out. A
/// restart's job reads `inactive` between its stop and its start, so a
/// state read while one is under way says nothing settled. Run out, one
/// still starting counts as on, since it's on its way there, as one an
/// automatic restart waits on is; one still stopping with no job after
/// it counts as off; and one whose job is still under way, or that
/// systemd doesn't say, is None, since either guess could undo it.
fn settled(
    mut state: impl FnMut() -> Option<(String, bool)>,
    takes: Duration,
    every: Duration,
) -> Option<bool> {
    let asked = Instant::now();
    loop {
        let (active, job) = state()?;
        let moving = job || matches!(active.as_str(), "activating" | "deactivating");
        if moving && asked.elapsed() < takes {
            thread::sleep(every);
            continue;
        }
        return match (active.as_str(), job) {
            ("activating", _) | ("active" | "reloading" | "refreshing", false) => Some(true),
            ("inactive" | "failed" | "deactivating", false) => Some(false),
            _ => None,
        };
    }
}

/// Where the shaper's own state waits while a proof may have it off:
/// the runtime folder, since a boot starts the shaper afresh. With none,
/// there's nowhere private to keep it, and it isn't kept.
pub fn marker() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR").map(|d| PathBuf::from(d).join("shaper-proof-was-on"))
}

/// What an earlier proof that didn't put the shaper back left in `marker`:
/// how the shaper was before it, once any keeper of its is done, or None
/// when nothing is owed.
/// A word older than `DEBT_STANDS` is dropped, the shaper left as it is.
pub fn debt(marker: &Path) -> Option<bool> {
    debt_within(marker, KEEPER_TAKES, DEBT_STANDS)
}

/// `debt`, waiting `takes` for a keeper, and taking a word `stands` old.
fn debt_within(marker: &Path, takes: Duration, stands: Duration) -> Option<bool> {
    // A killed proof's keeper puts the shaper back within about a second,
    // and forgets the marker: waited for, so its word isn't taken for the
    // shaper's own once it's back.
    let asked = Instant::now();
    while marker.exists() && asked.elapsed() < takes {
        thread::sleep(Duration::from_millis(100));
    }
    let earlier = fs::read_to_string(marker)
        .ok()
        .and_then(|s| match s.split_whitespace().next() {
            Some("on") => Some(true),
            Some("off") => Some(false),
            _ => None,
        });
    let age = fs::metadata(marker)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|at| at.elapsed().ok())
        .unwrap_or_default();
    if earlier.is_some() && age > stands {
        tell(&format!(
            "an earlier proof didn't put the shaper back {} min ago: left as it is now, \
             since Joel or a switch has likely set it since",
            age.as_secs() / 60
        ));
        forget(marker);
        return None;
    }
    if let Some(on) = earlier {
        tell(&format!(
            "an earlier proof didn't put the shaper back: putting it back {} now",
            if on { "on" } else { "off" }
        ));
    }
    earlier
}

/// Runs one window owning the shaper, found `on`: claimed in `marker`
/// while it runs, freshened every `FRESHENS`, so a keeper puts it back
/// if the proof is killed, and given up once the window says it put the
/// shaper back. One that couldn't keeps its claim, so the keeper tries
/// once more as the proof exits 69, and the next proof, failing that,
/// within `DEBT_STANDS`: either puts it as the 69 tells Joel to. Between
/// windows it is nobody's, so a switch or Joel's own change then stands.
pub fn owning<T>(marker: Option<&Path>, on: bool, window: impl FnOnce() -> (T, bool)) -> (T, bool) {
    let Some(marker) = marker else {
        return window();
    };
    claim(marker, on);
    let ended = AtomicBool::new(false);
    let (got, back) = thread::scope(|scope| {
        scope.spawn(|| freshen(marker, &ended, FRESHENS));
        let got = window();
        ended.store(true, Ordering::Relaxed);
        got
    });
    if back {
        forget(marker);
    }
    (got, back)
}

/// Marks `marker` as written now every `every`, until `ended`, so a
/// killed proof's word is aged from about when it died. One forgotten
/// meanwhile isn't written again.
fn freshen(marker: &Path, ended: &AtomicBool, every: Duration) {
    let mut since = Instant::now();
    while !ended.load(Ordering::Relaxed) {
        if since.elapsed() >= every {
            let _ = fs::File::options()
                .write(true)
                .open(marker)
                .and_then(|f| f.set_modified(std::time::SystemTime::now()));
            since = Instant::now();
        }
        // The window's end is heard within a tenth of a second.
        thread::sleep(every.min(Duration::from_millis(100)));
    }
}

/// Writes `on` to `marker` as this proof's word, with its pid, so only
/// this proof's keeper acts on it.
pub fn claim(marker: &Path, on: bool) {
    let word = if on { "on" } else { "off" };
    if fs::write(marker, format!("{word} {}\n", std::process::id())).is_err() {
        tell(&format!("couldn't write {}", marker.display()));
    }
}

/// Forgets the word in `marker`, the shaper nobody's again.
pub fn forget(marker: &Path) {
    let _ = fs::remove_file(marker);
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, PoisonError};

    /// Held by each test that sets or sends a signal the process hears,
    /// since handlers and STOPPED are the whole test binary's.
    static SIGNALS: Mutex<()> = Mutex::new(());

    fn signals() -> MutexGuard<'static, ()> {
        SIGNALS.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Puts every signal `hear_stops` set back to its default, so the
    /// rest of the tests hear Ctrl-C as ever.
    fn unheard() {
        // SAFETY: setting the default disposition.
        unsafe {
            for signal in STOPS {
                libc::signal(signal, libc::SIG_DFL);
            }
            libc::signal(libc::SIGCONT, libc::SIG_DFL);
        }
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
        }
        unheard();
        STOPPED.store(false, Ordering::Relaxed);
    }

    /// Marks `marker` as written `ago`.
    pub fn age(marker: &Path, ago: Duration) {
        fs::File::options()
            .write(true)
            .open(marker)
            .and_then(|f| f.set_modified(std::time::SystemTime::now() - ago))
            .expect("an aged marker");
    }

    /// How long ago `marker` was written.
    pub fn aged(marker: &Path) -> Duration {
        fs::metadata(marker)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|at| at.elapsed().ok())
            .unwrap_or_default()
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
        // The SIGCONT reaches this process too, maybe on another thread
        // a moment on: waited out while the lock is held.
        thread::sleep(Duration::from_millis(100));
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
    fn a_resumed_proof_stops() {
        // Paused by SIGSTOP, the proof counts as gone: its load ends and
        // its keeper may put the shaper back. Resumed, it stops for good,
        // and every wait and window asks the same flag.
        let _signals = signals();
        let was = STOPPED.swap(false, Ordering::Relaxed);
        hear_stops();
        // SAFETY: raising a signal whose handler was just set.
        unsafe {
            libc::raise(libc::SIGCONT);
        }
        let resumed_stops = STOPPED.load(Ordering::Relaxed);
        let halts = halted(&AtomicBool::new(false));
        unheard();
        STOPPED.store(was, Ordering::Relaxed);
        assert!(resumed_stops, "a resumed proof measured on");
        assert!(halts, "a resume didn't halt the window");
    }

    #[test]
    fn a_keeper_still_putting_the_shaper_back_is_waited_for() {
        let dir = std::env::temp_dir().join(format!("shaper-wait-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a folder");
        let marker = dir.join("was-on");
        fs::write(&marker, "on 1\n").expect("a marker");
        // The killed proof's keeper, putting the shaper back on and
        // forgetting it: the shaper's state then is its own again, and
        // read only then. Read before, the marker would be a debt.
        let keeping = marker.clone();
        let keeper = thread::spawn(move || {
            thread::sleep(Duration::from_millis(300));
            forget(&keeping);
        });
        assert_eq!(debt(&marker), None, "took a debt its keeper was paying");
        keeper.join().expect("the keeper");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_killed_proof_leaves_the_next_what_it_found() {
        let dir = std::env::temp_dir().join(format!("shaper-own-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a folder");
        let marker = dir.join("was-on");
        // With no keeper to wait for, as when one's scope was refused.
        let owed = || debt_within(&marker, Duration::ZERO, DEBT_STANDS);
        assert_eq!(owed(), None, "a debt with no proof before");
        // A proof killed in an on arm, the shaper left off: the next
        // finds it was on.
        claim(&marker, true);
        assert_eq!(owed(), Some(true), "lost the killed proof's word");
        forget(&marker);
        assert_eq!(owed(), None, "remembered a proof that put it back");
        // A marker that says neither, as a write cut short leaves, is
        // nobody's word, and nothing is owed.
        fs::write(&marker, "").expect("an empty marker");
        assert_eq!(owed(), None, "took an empty marker for a debt");
        forget(&marker);
        // A word left long ago is dropped: the shaper stands as Joel or a
        // switch has set it since.
        claim(&marker, true);
        age(&marker, DEBT_STANDS + Duration::from_mins(1));
        assert_eq!(owed(), None, "undid what was set since an old proof");
        assert!(!marker.exists(), "kept a word too old to act on");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_shaper_starting_or_stopping_is_read_once_settled() {
        // Each state as systemd says it, a job under way marked `+`, and
        // `?` for no answer; the last stands once they run out.
        let read = |states: &[&str], takes: Duration| {
            let mut states = states.iter().copied();
            let mut last = "?";
            settled(
                || {
                    last = states.next().unwrap_or(last);
                    let (active, job) = last.strip_suffix('+').map_or((last, false), |s| (s, true));
                    (active != "?").then(|| (active.to_string(), job))
                },
                takes,
                Duration::ZERO,
            )
        };
        let long = Duration::from_secs(60);
        // A restart after a crash, or at a switch, is on its way back on.
        assert_eq!(
            read(&["activating", "activating", "active"], long),
            Some(true),
            "a crash's restart"
        );
        assert_eq!(
            read(
                &["deactivating+", "inactive+", "activating+", "active"],
                long
            ),
            Some(true),
            "a restart's job read as off between its stop and start"
        );
        assert_eq!(
            read(&["deactivating+", "inactive"], long),
            Some(false),
            "a stop"
        );
        assert_eq!(read(&["failed"], long), Some(false), "a failed shaper");
        // What can't be read is neither, since a window taking it for off
        // would stop a shaper that was on.
        assert_eq!(read(&["?"], long), None, "no answer");
        assert_eq!(
            read(&["maintenance"], long),
            None,
            "a state it doesn't know"
        );
        assert_eq!(
            read(&["activating", "?"], long),
            None,
            "no answer once starting"
        );
        // Never settling, it is read as where it was heading, unless a job
        // may yet turn it round.
        assert_eq!(read(&["activating"], Duration::ZERO), Some(true), "a start");
        assert_eq!(
            read(&["deactivating"], Duration::ZERO),
            Some(false),
            "a stop"
        );
        assert_eq!(
            read(&["inactive+"], Duration::ZERO),
            None,
            "a restart under way"
        );
    }

    #[test]
    fn a_window_owns_the_shaper_only_while_it_runs() {
        let dir = std::env::temp_dir().join(format!("shaper-owning-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a folder");
        let marker = dir.join("was-on");
        let said = |m: &Path| fs::read_to_string(m).ok();
        let me = std::process::id();
        let ((), back) = owning(Some(&marker), true, || {
            assert_eq!(
                said(&marker),
                Some(format!("on {me}\n")),
                "a window ran with no word for its keeper"
            );
            ((), true)
        });
        assert!(back, "a window's put-back went unsaid");
        assert_eq!(said(&marker), None, "held the shaper between windows");
        // The next window finds the shaper off, as a switch or Joel left
        // it, and claims that, not what the first found; it can't put the
        // shaper back, so its word stays for the keeper's last try.
        let ((), back) = owning(Some(&marker), false, || {
            assert_eq!(
                said(&marker),
                Some(format!("off {me}\n")),
                "a window claimed what an earlier one found"
            );
            ((), false)
        });
        assert!(!back, "a failed put-back went unsaid");
        assert_eq!(
            said(&marker),
            Some(format!("off {me}\n")),
            "forgot a shaper it couldn't put back"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_window_keeps_its_word_fresh_while_it_runs() {
        let dir = std::env::temp_dir().join(format!("shaper-fresh-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a folder");
        let marker = dir.join("was-on");
        claim(&marker, true);
        age(&marker, Duration::from_mins(20));
        let ended = AtomicBool::new(false);
        thread::scope(|scope| {
            scope.spawn(|| freshen(&marker, &ended, Duration::from_millis(20)));
            let began = Instant::now();
            while aged(&marker) > Duration::from_mins(1) && began.elapsed() < Duration::from_secs(2)
            {
                thread::sleep(Duration::from_millis(10));
            }
            ended.store(true, Ordering::Relaxed);
        });
        assert!(
            aged(&marker) < Duration::from_mins(1),
            "a long window's word aged from its start"
        );
        // One forgotten isn't written again.
        forget(&marker);
        let ended = AtomicBool::new(false);
        thread::scope(|scope| {
            scope.spawn(|| freshen(&marker, &ended, Duration::from_millis(5)));
            thread::sleep(Duration::from_millis(50));
            ended.store(true, Ordering::Relaxed);
        });
        assert!(!marker.exists(), "wrote a forgotten word again");
        let _ = fs::remove_dir_all(&dir);
    }
}
