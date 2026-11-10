//! Ranks the pi window Joel is at, and those he was at a moment ago, as
//! his own programs rank.
//!
//! Every session's window is a scope in agents.slice, beside some eighty
//! agents' at the same weight. On 10 November at 14:0x Joel's typing in
//! pi felt sluggish while the processor and memory looked light:
//! agents.slice sat at its memory ceiling, 25.5 of 26.8 GB with 388,589
//! times over it, and the pi he was typing into had 726 MB in swap, paged
//! out like any agent's. The harness marks a pi as his only while a turn
//! he typed runs, which misses his reading and his editing, so focus is
//! the signal: niri says which window has it, as it changes.
//!
//! So the pi window with the focus, and those that had it within [`FOR`],
//! [`MOST`] in all, run at [`WEIGHT`], keep a floor of [`FLOOR`], which
//! holds inside agents.slice's own floor against its ceiling, have the
//! disk's 1 ms target, as Joel's apps do, and are the last systemd-oomd
//! ends. Each gives all of it back once it ages out. Focus comes before a
//! mark saying nobody is at a window, as `src/low.rs` says.
//!
//! A window is found by the app id the shell opened it with, which the
//! process the shell started in it is given as `MYCELIUM_APP_ID`
//! (modules/quickshell's Sessions.qml), or else, for a kitty of its own,
//! by the one scope that kitty has.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

/// A held window's weight for the processor, a session's apps' own.
pub const WEIGHT: u64 = 1000;

/// The memory a held window keeps under pressure: past the 726 MB one
/// had in swap, and three of them within agents.slice's floor of 15%,
/// 9.3 GiB on a 62 GiB machine, so each holds whole. An orchestrator's
/// window held gives up its own smaller floor while it is (`src/keep.rs`).
pub const FLOOR: u64 = 3 << 30;

/// How long a window Joel left stays held.
pub const FOR: Duration = Duration::from_secs(600);

/// The most windows held at once, the focused one among them.
pub const MOST: usize = 3;

/// The disk's latency target a held window has, in microseconds: the 1 ms
/// Joel's apps have (modules/slices).
pub const TARGET: u64 = 1000;

/// A window as niri says, as far as finding its scope goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    pub id: u64,
    pub app_id: String,
    pub pid: u32,
}

impl Window {
    /// Whether it holds a session: the shell classes each it opens for one
    /// as pi-session or pi-spawned (modules/quickshell's Niri.qml).
    pub fn session(&self) -> bool {
        self.app_id.starts_with("pi-session") || self.app_id.starts_with("pi-spawned")
    }
}

fn window(value: &Value) -> Option<(Window, bool)> {
    Some((
        Window {
            id: value.get("id")?.as_u64()?,
            app_id: value
                .get("app_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            pid: value
                .get("pid")
                .and_then(Value::as_u64)
                .and_then(|pid| u32::try_from(pid).ok())
                .unwrap_or(0),
        },
        value.get("is_focused").and_then(Value::as_bool) == Some(true),
    ))
}

/// niri's windows as its event stream says, and which has the focus.
#[derive(Debug, Default)]
pub struct Windows {
    all: HashMap<u64, Window>,
    focused: Option<u64>,
}

impl Windows {
    /// Takes one line of `niri msg --json event-stream`, and says whether
    /// the focus moved or a window closed.
    pub fn take(&mut self, line: &str) -> bool {
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            return false;
        };
        let before = (self.focused, self.all.len());
        if let Some(changed) = event.get("WindowsChanged") {
            let windows = changed.get("windows").and_then(Value::as_array);
            self.all.clear();
            self.focused = None;
            for (window, focused) in windows.into_iter().flatten().filter_map(window) {
                if focused {
                    self.focused = Some(window.id);
                }
                self.all.insert(window.id, window);
            }
        } else if let Some((window, focused)) = event
            .get("WindowOpenedOrChanged")
            .and_then(|changed| changed.get("window"))
            .and_then(window)
        {
            if focused {
                self.focused = Some(window.id);
            }
            self.all.insert(window.id, window);
        } else if let Some(id) = event
            .get("WindowClosed")
            .and_then(|closed| closed.get("id"))
            .and_then(Value::as_u64)
        {
            self.all.remove(&id);
            if self.focused == Some(id) {
                self.focused = None;
            }
        } else if let Some(changed) = event.get("WindowFocusChanged") {
            self.focused = changed.get("id").and_then(Value::as_u64);
        }
        (self.focused, self.all.len()) != before
    }

    /// The window with the focus, if niri knows one.
    pub fn focused(&self) -> Option<&Window> {
        self.all.get(&self.focused?)
    }

    /// Whether niri still has a window by `id`.
    pub fn has(&self, id: u64) -> bool {
        self.all.contains_key(&id)
    }
}

/// The session windows Joel has been at, and when he left each.
#[derive(Debug, Default)]
pub struct Recent {
    at: Option<Window>,
    left: Vec<(Window, Instant)>,
}

impl Recent {
    /// Notes the focus on `window`, or on none, `now`. One that holds no
    /// session is nobody's to hold, but leaving one for it still counts.
    pub fn focus(&mut self, window: Option<&Window>, now: Instant) {
        let window = window.filter(|window| window.session());
        if self.at.as_ref() == window {
            return;
        }
        if let Some(was) = self.at.take() {
            self.left.retain(|(left, _)| left.id != was.id);
            self.left.insert(0, (was, now));
        }
        if let Some(window) = window {
            self.left.retain(|(left, _)| left.id != window.id);
            self.at = Some(window.clone());
        }
    }

    /// Forgets the windows `has` no longer knows, which have closed.
    pub fn keep_only(&mut self, has: impl Fn(u64) -> bool) {
        self.left.retain(|(window, _)| has(window.id));
        if self.at.as_ref().is_some_and(|window| !has(window.id)) {
            self.at = None;
        }
    }

    /// The windows to hold `now`: the one with the focus, then those left
    /// within [`FOR`], latest first, [`MOST`] in all.
    pub fn held(&self, now: Instant) -> Vec<Window> {
        self.at
            .iter()
            .chain(
                self.left
                    .iter()
                    .filter(|(_, left)| now.saturating_duration_since(*left) < FOR)
                    .map(|(window, _)| window),
            )
            .take(MOST)
            .cloned()
            .collect()
    }
}

/// The scope a window's session runs in, from each scope's unit and the
/// app ids its processes were started with: the one started with the
/// window's own, or else the one scope of a kitty that draws it alone.
/// Two started with the same, as a shell started again could leave, are
/// neither.
pub fn scope_of<'a>(window: &Window, scopes: &'a [(String, Vec<String>)]) -> Option<&'a str> {
    let one = |found: Vec<&'a str>| (found.len() == 1).then(|| found[0]);
    let started: Vec<&str> = scopes
        .iter()
        .filter(|(_, apps)| apps.contains(&window.app_id))
        .map(|(unit, _)| unit.as_str())
        .collect();
    if !started.is_empty() {
        return one(started);
    }
    let own = format!("kitty-{}-", window.pid);
    one(scopes
        .iter()
        .map(|(unit, _)| unit.as_str())
        .filter(|unit| unit.starts_with(&own))
        .collect())
}

/// The app id a process was started with, from its environment's bytes.
pub fn app_id(environ: &[u8]) -> Option<String> {
    environ
        .split(|byte| *byte == 0)
        .find_map(|entry| entry.strip_prefix(b"MYCELIUM_APP_ID="))
        .map(|id| String::from_utf8_lossy(id).into_owned())
}

/// What a held window's `io.latency` is given, on the disk its slice's
/// own names (`259:0 target=1000`), or to give it back: a target of 0,
/// since systemd leaves a kernel's target in place when its is emptied.
pub fn latency(slice: &str, hold: bool) -> Option<String> {
    let device = slice.split_whitespace().next()?;
    let target = if hold { TARGET } else { 0 };
    Some(format!("{device} target={target}\n"))
}

/// What busctl is given to hold a window's unit with [`FLOOR`] and
/// systemd-oomd's avoid preference, or to give both back, as runtime
/// properties gone at the next boot.
pub fn hold_call(unit: &str, hold: bool) -> Vec<String> {
    let (floor, preference) = if hold {
        (FLOOR.to_string(), "avoid")
    } else {
        ("0".to_owned(), "none")
    };
    [
        "--user",
        "--quiet",
        "call",
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
        "SetUnitProperties",
        "sba(sv)",
        unit,
        "true",
        "2",
        "MemoryLow",
        "t",
        &floor,
        "ManagedOOMPreference",
        "s",
        preference,
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect()
}

/// The scopes held, each by its path, so each is set once and given back
/// once it ages out.
#[derive(Debug, Default)]
pub struct Holding {
    held: HashSet<PathBuf>,
    /// Whether a failure to hold a window has been said.
    pub failed: bool,
}

impl Holding {
    /// The scopes to change, each with whether to hold it, from those to
    /// hold now.
    pub fn changes(&self, wanted: &[PathBuf]) -> Vec<(PathBuf, bool)> {
        let hold = wanted
            .iter()
            .filter(|scope| !self.held.contains(*scope))
            .map(|scope| (scope.clone(), true));
        let give_back = self
            .held
            .iter()
            .filter(|scope| !wanted.contains(scope))
            .map(|scope| (scope.clone(), false));
        hold.chain(give_back).collect()
    }

    /// Notes a scope as held or given back.
    pub fn done(&mut self, scope: &Path, hold: bool) {
        if hold {
            self.held.insert(scope.to_path_buf());
        } else {
            self.held.remove(scope);
        }
    }

    /// Whether a scope's unit is held.
    pub fn holds(&self, unit: &str) -> bool {
        self.held
            .iter()
            .any(|scope| scope.file_name().is_some_and(|name| name == unit))
    }

    /// Forgets the scopes that have gone.
    pub fn keep_only(&mut self, there: impl Fn(&Path) -> bool) {
        self.held.retain(|scope| there(scope));
    }
}

/// What niri says, as its stream comes in on a thread of its own, so the
/// sorter's loop never waits on niri.
pub struct Following {
    heard: Arc<Mutex<(Windows, Recent)>>,
}

/// How long a stream that ended waits before it's started again.
const AGAIN: Duration = Duration::from_secs(10);

impl Following {
    /// Starts following niri's event stream with `niri`, writing a byte to
    /// `wake` each time the focus moves, so the sorter holds at once.
    pub fn start(niri: &Path, wake: Option<UnixStream>) -> Following {
        let heard = Arc::new(Mutex::new((Windows::default(), Recent::default())));
        let shared = Arc::clone(&heard);
        let niri = niri.to_path_buf();
        let followed = thread::Builder::new()
            .name("focus".into())
            .spawn(move || follow(&niri, &shared, wake.as_ref()));
        if let Err(error) = followed {
            eprintln!("jobs-sorter: niri's focus can't be followed: {error}");
        }
        Following { heard }
    }

    /// The windows to hold `now`.
    pub fn held(&self, now: Instant) -> Vec<Window> {
        let heard = self.heard.lock().unwrap_or_else(PoisonError::into_inner);
        heard.1.held(now)
    }
}

/// niri's socket, from what the user manager hands the units it starts
/// now: the sorter may start before niri, and outlive one niri for the
/// next, so its own environment can name none or an old one.
fn socket() -> Option<String> {
    let output = Command::new("systemctl")
        .args(["--user", "show-environment"])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    socket_in(&String::from_utf8_lossy(&output.stdout))
}

fn socket_in(environment: &str) -> Option<String> {
    environment
        .lines()
        .find_map(|line| line.strip_prefix("NIRI_SOCKET="))
        .map(str::to_owned)
}

fn follow(niri: &Path, heard: &Mutex<(Windows, Recent)>, wake: Option<&UnixStream>) {
    loop {
        let mut command = Command::new(niri);
        if let Some(socket) = socket() {
            command.env("NIRI_SOCKET", socket);
        }
        match command
            .args(["msg", "--json", "event-stream"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(mut stream) => {
                if let Some(out) = stream.stdout.take() {
                    for line in BufReader::new(out).lines().map_while(Result::ok) {
                        let mut guard = heard.lock().unwrap_or_else(PoisonError::into_inner);
                        let (windows, recent) = &mut *guard;
                        if windows.take(&line) {
                            recent.keep_only(|id| windows.has(id));
                            recent.focus(windows.focused(), Instant::now());
                            drop(guard);
                            if let Some(mut wake) = wake {
                                let _ = std::io::Write::write(&mut wake, &[1]);
                            }
                        }
                    }
                }
                let _ = stream.wait();
            }
            Err(error) => eprintln!("jobs-sorter: niri wouldn't start: {error}"),
        }
        thread::sleep(AGAIN);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pi(id: u64) -> Window {
        Window {
            id,
            app_id: format!("pi-session.{id}"),
            pid: 2684,
        }
    }

    #[test]
    fn the_focus_is_followed_through_niris_events() {
        let mut windows = Windows::default();
        assert!(windows.take(
            r#"{"WindowsChanged":{"windows":[
                {"id":23,"app_id":"pi-session.22","pid":2684,"is_focused":false},
                {"id":70,"app_id":"slack","pid":311171,"is_focused":true}]}}"#
        ));
        assert_eq!(windows.focused().map(|window| window.id), Some(70));
        assert!(windows.take(r#"{"WindowFocusChanged":{"id":23}}"#));
        assert_eq!(
            windows.focused(),
            Some(&Window {
                id: 23,
                app_id: "pi-session.22".into(),
                pid: 2684
            })
        );
        assert!(
            !windows.take(r#"{"WorkspacesChanged":{"workspaces":[]}}"#),
            "nothing moved"
        );
        assert!(windows.take(
            r#"{"WindowOpenedOrChanged":{"window":
                {"id":9,"app_id":"pi-spawned.4","pid":2684,"is_focused":false}}}"#
        ));
        assert_eq!(windows.focused().map(|window| window.id), Some(23));
        assert!(windows.take(r#"{"WindowClosed":{"id":23}}"#));
        assert_eq!(windows.focused(), None);
        assert!(!windows.has(23));
        assert!(windows.take(r#"{"WindowFocusChanged":{"id":null}}"#) || windows.focused().is_none());
    }

    #[test]
    fn the_focused_window_and_those_left_within_ten_minutes_are_held_three_at_most() {
        let start = Instant::now();
        let at = |seconds| start + Duration::from_secs(seconds);
        let mut recent = Recent::default();
        recent.focus(Some(&pi(1)), at(0));
        assert_eq!(recent.held(at(0)), [pi(1)]);
        recent.focus(Some(&pi(2)), at(10));
        recent.focus(Some(&pi(3)), at(20));
        recent.focus(Some(&pi(4)), at(30));
        assert_eq!(
            recent.held(at(30)),
            [pi(4), pi(3), pi(2)],
            "the focused first, then the latest left"
        );
        // Going back to one moves it up rather than holding it twice.
        recent.focus(Some(&pi(2)), at(40));
        assert_eq!(recent.held(at(40)), [pi(2), pi(4), pi(3)]);
        // Leaving for an app keeps the pi he left held, until it ages out.
        let slack = Window {
            id: 70,
            app_id: "slack".into(),
            pid: 311_171,
        };
        recent.focus(Some(&slack), at(50));
        assert_eq!(recent.held(at(50)), [pi(2), pi(4), pi(3)]);
        assert_eq!(recent.held(at(50 + 599)), [pi(2)], "4 and 3 were left earlier");
        assert_eq!(recent.held(at(50 + 600)), [] as [Window; 0]);
    }

    #[test]
    fn a_closed_window_is_no_longer_held() {
        let now = Instant::now();
        let mut recent = Recent::default();
        recent.focus(Some(&pi(1)), now);
        recent.focus(Some(&pi(2)), now);
        recent.keep_only(|id| id != 1);
        assert_eq!(recent.held(now), [pi(2)]);
        recent.keep_only(|_| false);
        assert_eq!(recent.held(now), [] as [Window; 0]);
    }

    #[test]
    fn a_window_is_found_by_its_app_id_or_its_kittys_one_scope() {
        let scopes = [
            ("kitty-2684-22.scope".to_owned(), vec!["pi-session.22".to_owned()]),
            ("kitty-2684-46.scope".to_owned(), vec!["pi-session.46".to_owned()]),
            ("kitty-2684-60.scope".to_owned(), vec!["pi-spawned.3".to_owned()]),
            ("kitty-2684-61.scope".to_owned(), vec!["pi-spawned.3".to_owned()]),
            ("kitty-3295902-0.scope".to_owned(), vec![]),
        ];
        let find = |app_id: &str, pid| {
            let window = Window {
                id: 1,
                app_id: app_id.into(),
                pid,
            };
            scope_of(&window, &scopes).map(str::to_owned)
        };
        assert_eq!(find("pi-session.46", 2684).as_deref(), Some("kitty-2684-46.scope"));
        assert_eq!(
            find("pi-spawned.96260002", 3_295_902).as_deref(),
            Some("kitty-3295902-0.scope"),
            "a kitty of its own"
        );
        assert_eq!(find("pi-spawned.3", 2684), None, "two started with one app id");
        assert_eq!(find("pi-session.99", 2684), None, "one kitty, many scopes");
    }

    #[test]
    fn niris_socket_is_read_from_the_user_managers_environment() {
        assert_eq!(
            socket_in("HOME=/home/j\nNIRI_SOCKET=/run/user/1000/niri.sock\n").as_deref(),
            Some("/run/user/1000/niri.sock")
        );
        assert_eq!(socket_in("HOME=/home/j\n"), None);
    }

    #[test]
    fn an_app_id_is_read_from_a_processs_environment() {
        assert_eq!(
            app_id(b"HOME=/home/j\0MYCELIUM_APP_ID=pi-session.22\0TERM=kitty\0").as_deref(),
            Some("pi-session.22")
        );
        assert_eq!(app_id(b"HOME=/home/j\0"), None);
    }

    #[test]
    fn a_session_window_is_one_the_shell_classed_for_a_session() {
        assert!(pi(1).session());
        let spawned = Window {
            app_id: "pi-spawned.96260002".into(),
            ..pi(1)
        };
        assert!(spawned.session());
        let kitty = Window {
            app_id: "kitty".into(),
            ..pi(1)
        };
        assert!(!kitty.session());
    }

    #[test]
    fn a_held_window_has_the_disks_target_on_its_slices_disk_and_gives_it_back_as_nought() {
        assert_eq!(
            latency("259:0 target=1000\n", true).as_deref(),
            Some("259:0 target=1000\n")
        );
        assert_eq!(
            latency("259:0 target=1000\n", false).as_deref(),
            Some("259:0 target=0\n")
        );
        assert_eq!(latency("", true), None, "a slice with no target names no disk");
    }

    #[test]
    fn a_window_is_held_with_a_floor_and_avoided_or_given_both_back() {
        let held = hold_call("kitty-1-9.scope", true);
        assert_eq!(
            held[8..],
            [
                "kitty-1-9.scope",
                "true",
                "2",
                "MemoryLow",
                "t",
                "3221225472",
                "ManagedOOMPreference",
                "s",
                "avoid"
            ]
        );
        let back = hold_call("kitty-1-9.scope", false);
        assert_eq!(back[13], "0");
        assert_eq!(back[16], "none");
    }

    #[test]
    fn a_scope_is_held_once_and_given_back_once_it_ages_out() {
        let a = PathBuf::from("/a/kitty-1-1.scope");
        let b = PathBuf::from("/a/kitty-1-2.scope");
        let mut holding = Holding::default();
        assert_eq!(holding.changes(&[a.clone()]), [(a.clone(), true)]);
        holding.done(&a, true);
        assert!(holding.holds("kitty-1-1.scope"));
        assert_eq!(holding.changes(&[a.clone()]), []);
        assert_eq!(
            holding.changes(&[b.clone()]),
            [(b.clone(), true), (a.clone(), false)]
        );
        holding.done(&a, false);
        assert!(!holding.holds("kitty-1-1.scope"));
        holding.done(&b, true);
        holding.keep_only(|_| false);
        assert_eq!(holding.changes(&[]), [], "a scope gone is nobody's to give back");
    }

    #[test]
    fn three_floors_fit_inside_the_agents_slices_own() {
        // agents.slice's floor is 15% of a 62 GiB machine (modules/slices).
        const { assert!(FLOOR * MOST as u64 <= (62 << 30) * 15 / 100) };
    }
}
