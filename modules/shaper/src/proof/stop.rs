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

/// How long a killed proof's word stands when its keeper couldn't act.
/// Past it, Joel or a switch has likely set the shaper as they mean it
/// since, and putting it back would undo them.
const DEBT_STANDS: Duration = Duration::from_mins(30);

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
        if shaper_on() == on {
            return true;
        }
        thread::sleep(Duration::from_secs(1));
    }
    false
}

/// Whether the shaper is on, as systemd has it once any start or stop
/// under way is done. `is-active` reads a shaper restarting after a crash
/// or at a switch as off, and a window that took that for its state would
/// stop it for good.
pub fn shaper_on() -> bool {
    settled(active_state, SETTLES, Duration::from_millis(250))
}

/// The shaper's `ActiveState`, as systemd says it, or nothing.
fn active_state() -> String {
    use std::os::unix::process::CommandExt;
    Command::new("systemctl")
        .args([
            "show",
            "--property=ActiveState",
            "--value",
            "shaper.service",
        ])
        .process_group(0)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Reads `state` every `every` until it's neither starting nor stopping,
/// or `takes` runs out: then one still starting counts as on, since it's
/// on its way there, and one still stopping as off.
fn settled(mut state: impl FnMut() -> String, takes: Duration, every: Duration) -> bool {
    let asked = Instant::now();
    loop {
        match state().as_str() {
            "activating" | "deactivating" if asked.elapsed() < takes => thread::sleep(every),
            "active" | "reloading" | "refreshing" | "activating" => return true,
            _ => return false,
        }
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
/// while it runs, so a keeper puts it back if the proof is killed, and
/// given up once it ends, with the shaper put back or, failing that, left
/// to Joel by the proof's 69. Between windows it is nobody's, so a switch
/// or Joel's own change then stands.
pub fn owning<T>(marker: Option<&Path>, on: bool, window: impl FnOnce() -> T) -> T {
    if let Some(marker) = marker {
        claim(marker, on);
    }
    let got = window();
    if let Some(marker) = marker {
        forget(marker);
    }
    got
}

/// Writes `on` to `marker` as this proof's word, with its pid, so only
/// this proof's keeper acts on it.
pub fn claim(marker: &Path, on: bool) {
    let word = if on { "on" } else { "off" };
    if fs::write(marker, format!("{word} {}\n", std::process::id())).is_err() {
        tell(&format!("couldn't write {}", marker.display()));
    }
}

/// Why a proof can't run alone.
#[derive(Debug)]
pub enum NotAlone {
    /// Another proof holds the lock: its pid, as it wrote it.
    Another(String),
    /// The lock couldn't be opened, so it can't be told.
    Unopened(String),
}

/// Holds the lock beside `marker` for as long as the file it gives is
/// open, however the proof ends, writing this proof's pid in it: two at
/// once would load the link twice, and each would take the other's
/// shaper for its own.
pub fn alone(marker: &Path) -> Result<fs::File, NotAlone> {
    use std::io::{Seek, Write as _};
    use std::os::fd::AsRawFd;
    let at = marker.with_file_name("shaper-proof.lock");
    let mut lock = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&at)
        .map_err(|e| NotAlone::Unopened(format!("{}: {e}", at.display())))?;
    // SAFETY: flock(2) on a file this holds open.
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        let who = fs::read_to_string(&at).unwrap_or_default();
        return Err(NotAlone::Another(who.trim().to_string()));
    }
    let _ = lock.set_len(0);
    let _ = lock.rewind();
    let _ = writeln!(lock, "{}", std::process::id());
    Ok(lock)
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
/// it once that's done. A marker saying neither is nobody's word, and one
/// another proof wrote is that proof's, which this one's keeper leaves:
/// read again before it's forgotten, since the next proof may have
/// written its own meanwhile.
fn keeper(pid: u32, began: &str) -> String {
    let mine = format!("v=; who=; read -r v who 2>/dev/null <\"$1\"; [ \"$who\" = {pid} ]");
    super::load::once_gone_for(
        pid,
        began,
        &format!(
            "{mine} || exit 0; case $v in on) v=start;; off) v=stop;; *) exit 0;; esac; \
             \"$2\" --no-ask-password \"$v\" shaper.service && {{ {mine} && rm -f \"$1\"; }}"
        ),
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
            ("on", "--no-ask-password start shaper.service\n"),
            ("off", "--no-ask-password stop shaper.service\n"),
        ] {
            fs::write(&marker, format!("{was} {pid}\n")).expect("a marker");
            assert_eq!(kept(pid, &began), asks, "the shaper wasn't put back {was}");
            assert!(!marker.exists(), "the marker outlived putting it back");
        }
        // The next proof's marker, written once this proof was gone, is
        // that proof's to put back, with a keeper of its own.
        let next = pid + 1;
        fs::write(&marker, format!("off {next}\n")).expect("a marker");
        assert_eq!(kept(pid, &began), "", "acted on the next proof's marker");
        assert!(marker.exists(), "forgot the next proof's marker");
        // And one the next proof writes while this keeper's systemctl runs
        // is read again, and left.
        fs::write(&marker, format!("on {pid}\n")).expect("a marker");
        let racing = format!(
            "m=$1; next() {{ echo \"off {next}\" >\"$m\"; }}; {}",
            keeper(pid, &began)
        );
        Command::new("sh")
            .args(["-c", &racing, "sh", at, "next"])
            .status()
            .expect("a shell");
        assert_eq!(
            fs::read_to_string(&marker).ok(),
            Some(format!("off {next}\n")),
            "forgot the marker the next proof wrote meanwhile"
        );
        // A shaper that wouldn't go back keeps its marker, for the next.
        fs::write(&marker, format!("on {pid}\n")).expect("a marker");
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
    fn only_one_proof_runs_at_once() {
        let dir = std::env::temp_dir().join(format!("shaper-alone-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a folder");
        let marker = dir.join("was-on");
        let first = alone(&marker).expect("the first proof");
        match alone(&marker) {
            Err(NotAlone::Another(who)) => {
                assert_eq!(who, std::process::id().to_string(), "the holder unnamed");
            }
            other => panic!("a second proof ran beside the first: {other:?}"),
        }
        drop(first);
        assert!(alone(&marker).is_ok(), "a proof gone still held the next");
        assert!(
            matches!(alone(&dir.join("no/such")), Err(NotAlone::Unopened(_))),
            "a lock it couldn't open taken for another proof's"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_keeper_still_putting_the_shaper_back_is_waited_for() {
        let dir = std::env::temp_dir().join(format!("shaper-wait-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a folder");
        let marker = dir.join("was-on");
        fs::write(&marker, "on 1\n").expect("a marker");
        // The killed proof's keeper, putting the shaper back on and
        // forgetting it: the shaper's state then is its own again, and
        // read only then.
        let on = std::sync::Arc::new(AtomicBool::new(false));
        let forgotten = std::sync::Arc::new(AtomicBool::new(false));
        let (keeping, starting, done) = (marker.clone(), on.clone(), forgotten.clone());
        let keeper = thread::spawn(move || {
            thread::sleep(Duration::from_millis(300));
            starting.store(true, Ordering::Relaxed);
            // Said first, since the proof may read on the moment it's gone.
            done.store(true, Ordering::Relaxed);
            forget(&keeping);
        });
        assert_eq!(debt(&marker), None, "took a debt its keeper was paying");
        assert!(
            forgotten.load(Ordering::Relaxed),
            "read the marker before the keeper forgot it"
        );
        assert!(
            on.load(Ordering::Relaxed),
            "done before the keeper had the shaper on"
        );
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
        let long_ago = std::time::SystemTime::now() - DEBT_STANDS - Duration::from_secs(60);
        fs::File::options()
            .write(true)
            .open(&marker)
            .and_then(|f| f.set_modified(long_ago))
            .expect("an old marker");
        assert_eq!(owed(), None, "undid what was set since an old proof");
        assert!(!marker.exists(), "kept a word too old to act on");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_shaper_starting_or_stopping_is_read_once_settled() {
        let read = |states: &[&str], takes: Duration| {
            let mut states = states.iter().map(ToString::to_string);
            let mut last = String::new();
            settled(
                || {
                    last = states.next().unwrap_or_else(|| last.clone());
                    last.clone()
                },
                takes,
                Duration::ZERO,
            )
        };
        let long = Duration::from_secs(60);
        // A restart after a crash, or at a switch, is on its way back on.
        assert!(
            read(&["activating", "activating", "active"], long),
            "a restart read as off"
        );
        assert!(
            read(&["deactivating", "activating", "active"], long),
            "a switch read as off"
        );
        assert!(
            !read(&["deactivating", "inactive"], long),
            "a stop read as on"
        );
        assert!(!read(&["failed"], long), "a failed shaper read as on");
        assert!(!read(&[""], long), "no answer read as on");
        // Never settling, it is read as where it was heading.
        assert!(read(&["activating"], Duration::ZERO), "a start read as off");
        assert!(
            !read(&["deactivating"], Duration::ZERO),
            "a stop read as on"
        );
    }

    #[test]
    fn a_window_owns_the_shaper_only_while_it_runs() {
        let dir = std::env::temp_dir().join(format!("shaper-owning-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a folder");
        let marker = dir.join("was-on");
        let said = |m: &Path| fs::read_to_string(m).ok();
        owning(Some(&marker), true, || {
            assert_eq!(
                said(&marker),
                Some(format!("on {}\n", std::process::id())),
                "a window ran with no word for its keeper"
            );
        });
        assert_eq!(said(&marker), None, "held the shaper between windows");
        // The next window finds the shaper off, as a switch or Joel left
        // it, and claims that, not what the first found.
        owning(Some(&marker), false, || {
            assert_eq!(
                said(&marker),
                Some(format!("off {}\n", std::process::id())),
                "a window claimed what an earlier one found"
            );
        });
        // One that couldn't put the shaper back leaves it to Joel, told
        // by the proof's 69, so no later proof undoes what he does.
        assert_eq!(said(&marker), None, "left a word Joel was told to act on");
        let _ = fs::remove_dir_all(&dir);
    }
}
