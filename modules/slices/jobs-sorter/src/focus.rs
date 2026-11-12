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
//! holds inside agents.slice's own floor against its ceiling, and are the
//! last systemd-oomd ends. What of them is paged out stays compressed
//! in memory, never written to the disk: on 10 November the zswap pool
//! counted against agents.slice's ceiling, so the slice's reclaim wrote
//! a window's cold pages to disk long before the pool filled, and 97% of
//! the held windows' swap-ins read the disk, a 4 KiB page a fault on pi's
//! own thread, where a window Joel came back to stalled for up to 2 s.
//! They have no disk latency target: on 10 November a target anywhere
//! under the account held Joel's own reads in the kernel's throttle, at
//! up to 141 ms against 9.8 ms with none. Each
//! gives all of it back once it ages out. A window he only passed over,
//! for less than [`STAY`], wasn't one he was at, so it's held once he
//! stays or if he was at it before: on 11 November windows opening took
//! the focus in turn, 364 holds in four hours, most given back within the
//! second. Focus comes before a mark saying nobody is at a window, as
//! `src/low.rs` says.
//!
//! Past those, every session window Joel was at within [`KEEP`] keeps
//! what of it is paged out compressed in memory too, with no floor and
//! no weight of its own. On 11 November, with a held window's swap read
//! back at once as the focus returns (`src/swap_in.rs`), 44.8% of the
//! machine's swap-ins still read the disk, against 27.1% the evening
//! before, and a window's read-back of a median 180 MiB took a second:
//! a window he comes back to after ten minutes had gone to disk. A
//! sorter started again, as each switch does, takes up the windows
//! niri says he was at within [`KEEP`] from when each last had the focus.
//! A file at [`KEEP_OFF`] under the account's config folder turns this
//! off, as an experiment's switch.
//!
//! A window is found by the app id the shell opened it with, which the
//! process the shell started in it is given as `MYCELIUM_APP_ID`
//! (modules/quickshell's Sessions.qml), or else, for a kitty of its own,
//! by the one scope that kitty has, or else by what the kitty drawing it
//! says runs there (`src/kitty.rs`).

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

/// A held window's weight for the processor, a session's apps' own.
pub const WEIGHT: u64 = 1000;

/// The memory a held window keeps under pressure: past the 726 MB one
/// had in swap, and three of them within agents.slice's floor of 10%,
/// 6.2 GiB on a 62 GiB machine, so each holds whole. An orchestrator's
/// window held gives up its own smaller floor while it is (`src/keep.rs`).
pub const FLOOR: u64 = 2 << 30;

/// How long a window Joel left stays held.
pub const FOR: Duration = Duration::from_secs(600);

/// How long a window Joel left keeps its paged-out memory compressed in
/// memory rather than on the disk.
pub const KEEP: Duration = Duration::from_hours(2);

/// The file, under the account's config folder, whose being there turns
/// [`KEEP`] off.
pub const KEEP_OFF: &str = "memory/no-arm-c";

/// The most windows held at once, the focused one among them.
pub const MOST: usize = 3;

/// How long a window keeps the focus before leaving it counts as having
/// been at it. On 10 November at 17:26 Joel moved the focus over ten
/// windows in two seconds, and each he passed over took a place from
/// those he had been at.
pub const STAY: Duration = Duration::from_secs(2);

/// A window as niri says, as far as finding its scope goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    pub id: u64,
    pub app_id: String,
    pub pid: u32,
    /// What its title bar says, which its kitty says too (`src/kitty.rs`).
    pub title: String,
}

impl Window {
    /// Whether it holds a session: the shell classes each it opens for one
    /// as pi-session or pi-spawned (modules/quickshell's Niri.qml).
    pub fn session(&self) -> bool {
        self.app_id.starts_with("pi-session") || self.app_id.starts_with("pi-spawned")
    }
}

/// A window from niri's JSON, and whether it has the focus.
pub fn window(value: &Value) -> Option<(Window, bool)> {
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
            title: value
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
        },
        value.get("is_focused").and_then(Value::as_bool) == Some(true),
    ))
}

/// When a window last had the focus, from niri's `focus_timestamp`, on
/// the clock since boot.
fn stamp(value: &Value) -> Option<Duration> {
    let stamp = value.get("focus_timestamp")?;
    Some(Duration::new(
        stamp.get("secs")?.as_u64()?,
        u32::try_from(stamp.get("nanos")?.as_u64()?).ok()?,
    ))
}

/// niri's windows as its event stream says, and which has the focus.
#[derive(Debug, Default)]
pub struct Windows {
    all: HashMap<u64, Window>,
    focused: Option<u64>,
    /// When each window last had the focus, on the clock since boot,
    /// as niri's list of every window says.
    stamps: HashMap<u64, Duration>,
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
            self.stamps = windows
                .into_iter()
                .flatten()
                .filter_map(|value| Some((value.get("id")?.as_u64()?, stamp(value)?)))
                .collect();
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

    /// Each window without the focus, with when it last had it, as niri's
    /// last list of every window said.
    pub fn stamped(&self) -> Vec<(Window, Duration)> {
        self.all
            .values()
            .filter(|window| Some(window.id) != self.focused)
            .filter_map(|window| Some((window.clone(), *self.stamps.get(&window.id)?)))
            .collect()
    }
}

/// The session window Joel is at, since when, and those he has been at,
/// with when he left each.
#[derive(Debug, Default)]
pub struct Recent {
    at: Option<(Window, Instant)>,
    left: Vec<(Window, Instant)>,
    /// Whether the windows from before this sorter are taken up.
    seeded: bool,
    /// Whether the next focus heard is the one taken up with them, a
    /// window Joel was already at rather than one passed over.
    taken_up: bool,
}

impl Recent {
    /// Notes the focus on `window`, or on none, `now`. One that holds no
    /// session is nobody's to hold, but leaving one for it still counts.
    /// Leaving one within [`STAY`] leaves where it was among those left.
    pub fn focus(&mut self, window: Option<&Window>, now: Instant) {
        let window = window.filter(|window| window.session());
        if self.at.as_ref().map(|(at, _)| at) == window {
            return;
        }
        if let Some((was, since)) = self.at.take()
            && now.saturating_duration_since(since) >= STAY
        {
            self.left.retain(|(left, _)| left.id != was.id);
            self.left.insert(0, (was, now));
        }
        let since = if std::mem::take(&mut self.taken_up) {
            now.checked_sub(STAY).unwrap_or(now)
        } else {
            now
        };
        self.at = window.map(|window| (window.clone(), since));
    }

    /// Forgets the windows `has` no longer knows, which have closed.
    pub fn keep_only(&mut self, has: impl Fn(u64) -> bool) {
        self.left.retain(|(window, _)| has(window.id));
        if self.at.as_ref().is_some_and(|(window, _)| !has(window.id)) {
            self.at = None;
        }
    }

    /// The session window with the focus, if one has it.
    pub fn at(&self) -> Option<&Window> {
        self.at.as_ref().map(|(window, _)| window)
    }

    /// Takes up, once, the session windows Joel was at within [`KEEP`]
    /// before this sorter started, from when each last had the focus on
    /// the clock since boot and that clock's time `since_boot`, as `now`.
    /// A window's focus came before he left it, so each counts as left
    /// a little early, never late.
    pub fn seed(&mut self, stamped: &[(Window, Duration)], since_boot: Duration, now: Instant) {
        if std::mem::replace(&mut self.seeded, true) {
            return;
        }
        self.taken_up = true;
        let mut left: Vec<(Window, Instant)> = stamped
            .iter()
            .filter(|(window, _)| window.session())
            .filter_map(|(window, stamp)| {
                let ago = since_boot.checked_sub(*stamp)?;
                (ago < KEEP).then_some(())?;
                Some((window.clone(), now.checked_sub(ago)?))
            })
            .collect();
        left.retain(|(window, _)| self.left.iter().all(|(known, _)| known.id != window.id));
        self.left.extend(left);
        self.left.sort_by_key(|(_, when)| std::cmp::Reverse(*when));
    }

    /// The windows to keep in memory `now`: every session window left
    /// within [`KEEP`], latest first, those [`Recent::held`] among them.
    pub fn kept(&self, now: Instant) -> Vec<Window> {
        self.left
            .iter()
            .filter(|(_, left)| now.saturating_duration_since(*left) < KEEP)
            .map(|(window, _)| window.clone())
            .collect()
    }

    /// The windows to hold `now`: the one with the focus, once Joel has
    /// stayed at it for [`STAY`] or at once if he was at it before, then
    /// those left within [`FOR`], latest first, [`MOST`] in all.
    pub fn held(&self, now: Instant) -> Vec<Window> {
        let at = self
            .at
            .as_ref()
            .filter(|(window, since)| {
                now.saturating_duration_since(*since) >= STAY
                    || self.left.iter().any(|(left, _)| left.id == window.id)
            })
            .map(|(window, _)| window);
        at.into_iter()
            .chain(
                self.left
                    .iter()
                    .filter(|(window, left)| {
                        Some(window.id) != at.map(|at| at.id)
                            && now.saturating_duration_since(*left) < FOR
                    })
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

    /// The session window with the focus, if one has it.
    pub fn at(&self) -> Option<Window> {
        let heard = self.heard.lock().unwrap_or_else(PoisonError::into_inner);
        heard.1.at().cloned()
    }

    /// The windows to keep in memory `now` beside those held.
    pub fn kept(&self, now: Instant) -> Vec<Window> {
        let heard = self.heard.lock().unwrap_or_else(PoisonError::into_inner);
        heard.1.kept(now)
    }
}

/// The time on the clock niri stamps a window's focus with, which counts
/// from boot. `/proc/uptime` counts a suspend too, where niri's clock
/// doesn't, so after one a window reads as left earlier than it was and
/// is kept for less, never more.
fn since_boot() -> Option<Duration> {
    let uptime = std::fs::read_to_string("/proc/uptime").ok()?;
    Duration::try_from_secs_f64(uptime.split_whitespace().next()?.parse().ok()?).ok()
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
                            if let Some(since_boot) = since_boot() {
                                recent.seed(&windows.stamped(), since_boot, Instant::now());
                            }
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
            title: String::new(),
        }
    }

    #[test]
    fn the_focus_is_followed_through_niris_events() {
        let mut windows = Windows::default();
        assert!(
            windows.take(
                r#"{"WindowsChanged":{"windows":[
                {"id":23,"app_id":"pi-session.22","pid":2684,"is_focused":false},
                {"id":70,"app_id":"slack","pid":311171,"is_focused":true}]}}"#
            ),
            "every window, at first"
        );
        assert_eq!(windows.focused().map(|window| window.id), Some(70));
        assert!(
            windows.take(r#"{"WindowFocusChanged":{"id":23}}"#),
            "the focus moved"
        );
        assert_eq!(
            windows.focused(),
            Some(&Window {
                id: 23,
                app_id: "pi-session.22".into(),
                pid: 2684,
                title: String::new(),
            })
        );
        assert!(
            !windows.take(r#"{"WorkspacesChanged":{"workspaces":[]}}"#),
            "nothing moved"
        );
        assert!(
            windows.take(
                r#"{"WindowOpenedOrChanged":{"window":
                {"id":9,"app_id":"pi-spawned.4","pid":2684,"is_focused":false}}}"#
            ),
            "a window opened"
        );
        assert_eq!(windows.focused().map(|window| window.id), Some(23));
        assert!(
            windows.take(r#"{"WindowClosed":{"id":23}}"#),
            "the focused window closed"
        );
        assert_eq!(windows.focused(), None);
        assert!(!windows.has(23), "a closed window is gone");
        assert!(
            !windows.take(r#"{"WindowFocusChanged":{"id":null}}"#),
            "the focus was on nothing already"
        );
    }

    #[test]
    fn the_focused_window_and_those_left_within_ten_minutes_are_held_three_at_most() {
        let start = Instant::now();
        let at = |seconds| start + Duration::from_secs(seconds);
        let mut recent = Recent::default();
        recent.focus(Some(&pi(1)), at(0));
        assert_eq!(recent.held(at(2)), [pi(1)]);
        recent.focus(Some(&pi(2)), at(10));
        recent.focus(Some(&pi(3)), at(20));
        recent.focus(Some(&pi(4)), at(30));
        assert_eq!(
            recent.held(at(32)),
            [pi(4), pi(3), pi(2)],
            "the focused first, then the latest left"
        );
        // Going back to one moves it up at once rather than twice.
        recent.focus(Some(&pi(2)), at(40));
        assert_eq!(recent.held(at(40)), [pi(2), pi(4), pi(3)]);
        // Leaving for an app keeps the pi he left held, until it ages out.
        let slack = Window {
            id: 70,
            app_id: "slack".into(),
            pid: 311_171,
            title: String::new(),
        };
        recent.focus(Some(&slack), at(50));
        assert_eq!(recent.held(at(50)), [pi(2), pi(4), pi(3)]);
        assert_eq!(
            recent.held(at(50 + 599)),
            [pi(2)],
            "4 and 3 were left earlier"
        );
        assert_eq!(recent.held(at(50 + 600)), [] as [Window; 0]);
    }

    #[test]
    fn a_window_passed_over_on_the_way_to_another_is_not_one_he_was_at() {
        let start = Instant::now();
        let at = |millis| start + Duration::from_millis(millis);
        let mut recent = Recent::default();
        recent.focus(Some(&pi(1)), at(0));
        recent.focus(Some(&pi(2)), at(10_000));
        recent.focus(Some(&pi(3)), at(20_000));
        recent.focus(Some(&pi(4)), at(30_000));
        recent.focus(Some(&pi(5)), at(30_500));
        assert_eq!(
            recent.held(at(32_500)),
            [pi(5), pi(3), pi(2)],
            "4 had the focus for half a second, on the way to 5"
        );
        recent.focus(Some(&pi(2)), at(40_000));
        recent.focus(Some(&pi(6)), at(40_500));
        assert_eq!(
            recent.held(at(42_500)),
            [pi(6), pi(5), pi(3)],
            "passing over 2 again neither brings it up nor forgets it"
        );
        recent.keep_only(|id| id != 5 && id != 3);
        assert_eq!(
            recent.held(at(42_500)),
            [pi(6), pi(2), pi(1)],
            "2 and 1 are still those he was at before"
        );
    }

    #[test]
    fn a_window_that_takes_the_focus_as_it_opens_is_held_only_once_he_stays() {
        let start = Instant::now();
        let at = |millis| start + Duration::from_millis(millis);
        let mut recent = Recent::default();
        recent.focus(Some(&pi(1)), at(0));
        // Three windows open in turn, each taking the focus for a moment,
        // and the focus comes back to the one he was at.
        for (n, id) in [7, 8, 9].into_iter().enumerate() {
            let when = 10_000 + 200 * u64::try_from(n).unwrap();
            recent.focus(Some(&pi(id)), at(when));
            assert_eq!(
                recent.held(at(when + 100)),
                [pi(1)],
                "{id} only passed by, so only 1 is held"
            );
        }
        recent.focus(Some(&pi(1)), at(10_600));
        assert_eq!(
            recent.held(at(10_600)),
            [pi(1)],
            "he was at 1 before, so it's held the moment he's back"
        );
        recent.focus(Some(&pi(9)), at(20_000));
        assert_eq!(recent.held(at(21_999)), [pi(1)], "not yet a stay");
        assert_eq!(
            recent.held(at(22_000)),
            [pi(9), pi(1)],
            "staying makes it one he's at"
        );
    }

    #[test]
    fn a_closed_window_is_no_longer_held() {
        let now = Instant::now();
        let mut recent = Recent::default();
        recent.focus(Some(&pi(1)), now);
        recent.focus(Some(&pi(2)), now + STAY);
        let now = now + STAY + STAY;
        assert_eq!(recent.held(now), [pi(2), pi(1)], "1 was left after a stay");
        recent.keep_only(|id| id != 1);
        assert_eq!(recent.held(now), [pi(2)]);
        recent.keep_only(|_| false);
        assert_eq!(recent.held(now), [] as [Window; 0]);
    }

    #[test]
    fn a_window_is_found_by_its_app_id_or_its_kittys_one_scope() {
        let scopes = [
            (
                "kitty-2684-22.scope".to_owned(),
                vec!["pi-session.22".to_owned()],
            ),
            (
                "kitty-2684-46.scope".to_owned(),
                vec!["pi-session.46".to_owned()],
            ),
            (
                "kitty-2684-60.scope".to_owned(),
                vec!["pi-spawned.3".to_owned()],
            ),
            (
                "kitty-2684-61.scope".to_owned(),
                vec!["pi-spawned.3".to_owned()],
            ),
            ("kitty-3295902-0.scope".to_owned(), vec![]),
        ];
        let find = |app_id: &str, pid| {
            let window = Window {
                id: 1,
                app_id: app_id.into(),
                pid,
                title: String::new(),
            };
            scope_of(&window, &scopes).map(str::to_owned)
        };
        assert_eq!(
            find("pi-session.46", 2684).as_deref(),
            Some("kitty-2684-46.scope")
        );
        assert_eq!(
            find("pi-spawned.96260002", 3_295_902).as_deref(),
            Some("kitty-3295902-0.scope"),
            "a kitty of its own"
        );
        assert_eq!(
            find("pi-spawned.3", 2684),
            None,
            "two started with one app id"
        );
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
        assert!(pi(1).session(), "a pi-session window");
        let spawned = Window {
            app_id: "pi-spawned.96260002".into(),
            ..pi(1)
        };
        assert!(spawned.session(), "a pi-spawned window");
        let kitty = Window {
            app_id: "kitty".into(),
            ..pi(1)
        };
        assert!(!kitty.session(), "a plain kitty holds no session");
    }

    #[test]
    fn every_window_left_within_two_hours_is_kept_in_memory() {
        let start = Instant::now();
        let at = |seconds| start + Duration::from_secs(seconds);
        let mut recent = Recent::default();
        for id in 1..=6 {
            recent.focus(Some(&pi(id)), at(id * 10));
        }
        assert_eq!(
            recent.kept(at(60)),
            [pi(5), pi(4), pi(3), pi(2), pi(1)],
            "past the three held, all he left, latest first"
        );
        assert_eq!(
            recent.kept(at(20 + 7199)),
            [pi(5), pi(4), pi(3), pi(2), pi(1)]
        );
        assert_eq!(
            recent.kept(at(20 + 7200)),
            [pi(5), pi(4), pi(3), pi(2)],
            "the first left goes to disk two hours after"
        );
    }

    #[test]
    fn a_sorter_started_again_keeps_the_windows_niri_says_he_was_at_within_two_hours() {
        let mut windows = Windows::default();
        assert!(
            windows.take(
                r#"{"WindowsChanged":{"windows":[
                {"id":1,"app_id":"pi-session.1","pid":5,"is_focused":false,
                 "focus_timestamp":{"secs":9000,"nanos":0}},
                {"id":2,"app_id":"pi-session.2","pid":5,"is_focused":false,
                 "focus_timestamp":{"secs":2000,"nanos":0}},
                {"id":3,"app_id":"slack","pid":6,"is_focused":false,
                 "focus_timestamp":{"secs":9500,"nanos":0}},
                {"id":4,"app_id":"pi-session.4","pid":5,"is_focused":true,
                 "focus_timestamp":{"secs":9900,"nanos":0}},
                {"id":5,"app_id":"pi-session.5","pid":5,"is_focused":false,
                 "focus_timestamp":{"secs":9800,"nanos":5}},
                {"id":6,"app_id":"pi-session.6","pid":5,"is_focused":false}]}}"#
            ),
            "a list of windows is a change"
        );
        let now = Instant::now();
        let mut recent = Recent::default();
        recent.seed(&windows.stamped(), Duration::from_secs(10_000), now);
        recent.focus(windows.focused(), now);
        let ids = |found: Vec<Window>| found.iter().map(|window| window.id).collect::<Vec<_>>();
        assert_eq!(
            ids(recent.kept(now)),
            [5, 1],
            "sessions left within two hours, not one left long ago, an app or one unstamped"
        );
        assert_eq!(
            ids(recent.held(now)),
            [4, 5],
            "one left 200 s ago is held too"
        );
        recent.seed(&windows.stamped(), Duration::from_secs(10_100), now);
        assert_eq!(ids(recent.kept(now)), [5, 1], "taken up once");
    }

    #[test]
    fn three_floors_fit_inside_the_agents_slices_own() {
        // agents.slice's floor is 10% of a 62 GiB machine (modules/slices).
        const {
            assert!(
                FLOOR * MOST as u64 <= (62 << 30) * 10 / 100,
                "three floors past agents.slice's own"
            );
        };
    }
}
