//! Where each session's window sits, kept so Reopen Them All puts every
//! one back on its workspace in its order after a cold boot (Joel,
//! 2026-10-29, after a restore: "every session is all over the place on
//! one workspace, out of order"), served as `arrangement:agents` for the
//! shell to put them back by. Kept here rather than in the shell, which
//! worked it out from niri at every still moment (Joel, 2026-11-10: "the
//! daemons need to be the data layer, and they need to serve the
//! launcher").
//!
//! Each place is kept by the session's own id, which pi keeps as a session
//! is reopened: the screen, by what it is (the windows daemon's Screens
//! rows, as Places.js's described says it) and by
//! its connector, the workspace as niri numbers it there, then its column
//! and its tile in that column, and whether it `leads`, orchestrating one
//! of the others, so it begins first. One's kept after it ends, since a
//! boot's end ends them one by one, and only this boot's and the one
//! before it are kept. Every other tiled window's place is kept beside
//! them, by its program and title (`others`), so Reopen Them All can put
//! back those that reopen themselves. It's kept once the windows have been
//! still a moment ([`Still`]), never for each change.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use daemon_serve::watch::Held;
use serde_json::{Map, Value, json};

use crate::file;
use crate::served::PLACE;
use crate::session::{Lives, Session};
use crate::windows::UNTOLD;

/// The kind it's served as.
pub const KIND: &str = "arrangement";

/// How long the windows stay still before their places are kept, as the
/// shell waited.
pub const STILL: Duration = Duration::from_secs(2);

/// How long a session runs on here with no window before its place is let
/// go, so Reopen Them All brings it back with none, as it had. A boot's end
/// closes each window before its session ends, user@ given 5 s to stop
/// (modules/systemd/nixos.nix), and a hosted session's viewer stays open
/// through its host's stop of up to 150 s (modules/pi/host.nix), so a
/// window gone for longer was closed while the boot ran on: after the
/// 10:05 restart on 2026-11-21, six sessions whose windows had been
/// closed hours before came back in windows.
pub const CLOSED_FOR: Duration = Duration::from_secs(180);

/// Each lit screen's connector and what the screen is, from the windows
/// daemon's Screens rows, each sent whole under `row` and holding what
/// its screen is and how it stands (modules/windows, screens.rs); rows
/// inside a screen's hold neither. One that's off, as the laptop's is
/// while its lid is shut on the dock, has no place in the layout, so
/// it's as gone as one unplugged. A screen's row that says neither is
/// warned of once, since every place kept went on naming no screen
/// unseen when the windows daemon stopped serving it (#4892).
pub fn screens(rows: &Held) -> HashMap<String, String> {
    static WARNED: AtomicBool = AtomicBool::new(false);
    rows.records()
        .filter_map(|(key, row)| {
            let held = &row["row"]["held"];
            let output = held["output"].as_str()?;
            let (Some(screen), Some(standing)) =
                (held["screen"].as_str(), held["standing"].as_str())
            else {
                if key.starts_with(SCREEN_ROW) && !WARNED.swap(true, Ordering::Relaxed) {
                    eprintln!(
                        "<4>the windows daemon's {key} says not what its screen is or how it \
                         stands, so no session's place names it"
                    );
                }
                return None;
            };
            (standing != "off").then(|| (output.to_owned(), screen.to_owned()))
        })
        .collect()
}

/// A screen's own row's key, as the windows daemon serves it, beside the
/// rows inside its hold.
const SCREEN_ROW: &str = "row:displays:display:";

/// What niri says of the desk: its windows, workspaces and lit screens.
pub struct Desk<'a> {
    pub windows: &'a Held,
    pub workspaces: &'a Held,
    pub screens: &'a HashMap<String, String>,
}

impl Desk<'_> {
    /// Where a tiled window sits, with `fields` first: its screen, the
    /// workspace there, and its column and tile. A floating one, or one on
    /// no workspace, sits nowhere to keep.
    fn at(&self, window: &Value, fields: Map<String, Value>) -> Option<Map<String, Value>> {
        let (Some(column), Some(tile)) = (window["column"].as_u64(), window["down"].as_u64())
        else {
            return None;
        };
        if window["floating"] == true {
            return None;
        }
        let workspace = self
            .workspaces
            .get(&format!("workspace:{}", window["workspace"].as_u64()?))?;
        let output = workspace["output"].as_str().unwrap_or("");
        let mut place = fields;
        place.insert(
            "screen".into(),
            json!(self.screens.get(output).cloned().unwrap_or_default()),
        );
        place.insert("output".into(), json!(output));
        place.insert("workspace".into(), workspace["idx"].clone());
        // Named workspaces, an orchestrator's, come back from a boot in
        // another order, so the name says where it goes better than the
        // number (Reopening.js's namedBack).
        if let Some(name) = workspace["name"].as_str().filter(|name| !name.is_empty()) {
            place.insert("workspaceName".into(), json!(name));
        }
        place.insert("column".into(), json!(column));
        place.insert("tile".into(), json!(tile));
        Some(place)
    }
}

/// What's kept of a session whose window can't be placed: one floating, on
/// a workspace niri hasn't listed, or one its kitty draws beside another
/// (windows.rs's [`UNTOLD`]). It has a window all the same, so Reopen Them
/// All brings it back in one rather than taking it for one with none
/// (Reopening.js's broughtBack).
fn unplaced() -> Value {
    json!({"unplaced": true})
}

/// Where each running session here sits now, by pi's id for it: its window
/// as `paired` names it by the session's address (the `agent-window`
/// records), and whether it leads one of the sessions running or parked.
/// One with a window that can't be placed is kept as [`unplaced`].
pub fn now<'a>(
    sessions: impl Iterator<Item = Session<'a>> + Clone,
    paired: &[Value],
    desk: &Desk<'_>,
) -> Map<String, Value> {
    let leaders: HashSet<String> = sessions
        .clone()
        .filter(|one| matches!(one.lives(), Lives::Running | Lives::Parked))
        .filter_map(|one| one.presence()?.get("orchestratedBy")?.as_str())
        .filter(|address| !address.is_empty())
        .map(str::to_uppercase)
        .collect();
    let windows: HashMap<&str, &str> = paired
        .iter()
        .filter_map(|one| Some((one["address"].as_str()?, one["window"].as_str()?)))
        .collect();
    let untold = untold_of(paired);
    sessions
        .filter(|one| one.here() && one.lives() == Lives::Running)
        .filter_map(|one| {
            let id = one.presence()?.get("session")?.as_str()?;
            let window = windows
                .get(one.address())
                .and_then(|window| desk.windows.get(window));
            let Some(window) = window else {
                return untold
                    .contains(one.address())
                    .then(|| (id.to_owned(), unplaced()));
            };
            let addresses = one.presence()?.get("addresses").and_then(Value::as_array);
            let leads = addresses.map_or_else(
                || leaders.contains(&one.address().to_uppercase()),
                |all| {
                    all.iter()
                        .filter_map(Value::as_str)
                        .any(|address| leaders.contains(&address.to_uppercase()))
                },
            );
            let Some(mut place) = desk.at(window, Map::new()) else {
                return Some((id.to_owned(), unplaced()));
            };
            place.insert("leads".into(), json!(leads));
            // The orchestrator it sits under, so Reopen Them All puts it
            // back on that one's workspace whatever number the workspace
            // has after a boot (Sessions.qml's opensOn, Places.js's homeOf).
            let under = one
                .presence()?
                .get("orchestratedBy")
                .and_then(Value::as_str);
            if let Some(under) = under.filter(|under| !under.is_empty()) {
                place.insert("under".into(), json!(under.to_uppercase()));
            }
            Some((id.to_owned(), Value::Object(place)))
        })
        .collect()
}

/// The sessions running here with no window, by pi's id, as a keep sees
/// them.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Windowless {
    /// Those whose window was closed by hand, so their places go.
    pub closed: HashSet<String>,
    /// Those whose window can't be said to be closed by hand, so their
    /// places stay as they are.
    pub unsure: HashSet<String>,
    /// Every session the node has said, wherever and however it stands,
    /// so one it hasn't, as before its sessions are heard, keeps its mark.
    pub heard: HashSet<String>,
}

/// A session's id, as pi's presence says it.
fn id_of(one: Session) -> Option<String> {
    one.presence()
        .and_then(|presence| presence.get("session"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// What a place is marked while its window is gone with every other its
/// terminal drew, as a crash takes them, until it shows again: one of
/// those opened again doesn't make the rest closed by hand.
const ORPHANED: &str = "orphaned";

/// The addresses whose window windows.rs couldn't tell from another's.
fn untold_of(paired: &[Value]) -> HashSet<&str> {
    paired
        .iter()
        .filter(|one| one[UNTOLD] == true)
        .filter_map(|one| one["address"].as_str())
        .collect()
}

/// The terminal a session opened with an app id is drawn in: the sessions
/// kitty, as Sessions.qml's inSessions opens each, every viewer too. No
/// kitty's socket is named so.
const SESSIONS_KITTY: &str = "the sessions kitty";

/// The sessions running here now with no window, by pi's id: none paired
/// with it, or one niri no longer lists. One whose window windows.rs
/// couldn't tell from another's isn't among them. One is closed by hand
/// only while another session its terminal draws still shows: every
/// window one terminal drew gone at once is that terminal or niri gone,
/// as a crash takes them, so those are unsure. The terminal is the
/// sessions kitty for one opened with an app id, else the kitty its pair
/// names, else, for a held one, the sessions kitty.
pub fn unwindowed<'a>(
    sessions: impl Iterator<Item = Session<'a>>,
    paired: &[Value],
    desk: &Desk<'_>,
) -> Windowless {
    let windows: HashMap<&str, &str> = paired
        .iter()
        .filter_map(|one| Some((one["address"].as_str()?, one["window"].as_str()?)))
        .collect();
    let kitties: HashMap<&str, &str> = paired
        .iter()
        .filter_map(|one| Some((one["address"].as_str()?, one["kitty"]["within"].as_str()?)))
        .collect();
    let untold = untold_of(paired);
    let sessions: Vec<Session> = sessions.collect();
    let running: Vec<Session> = sessions
        .iter()
        .copied()
        .filter(|one| one.here() && one.lives() == Lives::Running)
        .collect();
    let shown = |one: &Session| {
        windows
            .get(one.address())
            .is_some_and(|window| desk.windows.get(window).is_some())
    };
    // A held pi draws in the sessions kitty too, its app id going with its
    // window, as the node says no window once it's closed.
    let terminal = |one: &Session| {
        if one.opened().is_empty() {
            kitties
                .get(one.address())
                .copied()
                .or_else(|| (!one.0["held"].is_null()).then_some(SESSIONS_KITTY))
        } else {
            Some(SESSIONS_KITTY)
        }
    };
    // A terminal no kitty is known for lights nothing: windows.rs pairs a
    // plain pi's window only through its kitty, so it can't be told whose.
    let lit: HashSet<&str> = running
        .iter()
        .filter(|one| shown(one))
        .filter_map(terminal)
        .collect();
    let mut seen = Windowless {
        heard: sessions.iter().copied().filter_map(id_of).collect(),
        ..Windowless::default()
    };
    for one in running
        .iter()
        .filter(|one| !shown(one) && !untold.contains(one.address()))
    {
        let Some(id) = id_of(*one) else {
            continue;
        };
        if terminal(one).is_some_and(|its| lit.contains(its)) {
            seen.closed.insert(id);
        } else {
            seen.unsure.insert(id);
        }
    }
    seen
}

/// Every tiled window but kitty's, whose windows are sessions or a
/// terminal of Joel's own, oldest first: Chrome's, Slack's and the like.
pub fn others(desk: &Desk<'_>) -> Vec<Value> {
    let mut windows: Vec<&Value> = desk
        .windows
        .records()
        .map(|(_, window)| window)
        .filter(|window| window["app_id"] != "kitty")
        .collect();
    windows.sort_by_key(|window| window["id"].as_u64().unwrap_or(0));
    windows
        .into_iter()
        .filter_map(|window| {
            let mut named = Map::new();
            named.insert("app".into(), json!(window["app_id"].as_str().unwrap_or("")));
            named.insert(
                "title".into(),
                json!(window["title"].as_str().unwrap_or("")),
            );
            desk.at(window, named).map(Value::Object)
        })
        .collect()
}

/// The places kept, this boot's and the one before it, each boot's by
/// pi's id for each session, and the other windows' beside them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Arrangement {
    pub boots: Map<String, Value>,
    pub others: Map<String, Value>,
}

impl Arrangement {
    /// What a kept arrangement says, or none kept where it isn't one.
    pub fn of(said: &Value) -> Self {
        let (Some(boots), true) = (said["boots"].as_object(), said["v"] == 1) else {
            return Self::default();
        };
        let others = said["others"].as_object().cloned().unwrap_or_default();
        Self {
            boots: boots.clone(),
            others,
        }
    }

    /// What's kept at `path`, or none there.
    pub fn read(path: &Path) -> Option<Self> {
        let bytes = fs::read(path).ok()?;
        Some(Self::of(&serde_json::from_slice(&bytes).ok()?))
    }

    /// What the shell kept before the daemon did, in the newest of its own
    /// folders under `shells`, read once while the daemon has kept none.
    pub fn from_shell(shells: &Path) -> Self {
        file::newest(shells, "session-arrangement.json")
            .and_then(|file| Self::read(&file))
            .unwrap_or_default()
    }

    /// It as kept, whole.
    pub fn said(&self) -> Value {
        json!({"v": 1, "boots": self.boots, "others": self.others})
    }

    /// The boot kept beside `boot`: the one being restored when it's kept,
    /// else the other kept. Not only while restore.json names it, since
    /// that moves on once its sessions are back: on 2026-11-02 it named
    /// another boot at 18:41, and the places Reopen Them All had just used
    /// were dropped at 18:43, so nobody could see which it misplaced.
    pub fn before(&self, boot: &str, restoring: &str) -> Option<&str> {
        if restoring != boot
            && let Some((kept, _)) = self.boots.get_key_value(restoring)
        {
            return Some(kept);
        }
        self.boots
            .keys()
            .map(String::as_str)
            .find(|kept| *kept != boot)
    }

    /// The places kept once `now` and `others` are at `at` (ms since the
    /// epoch), this boot's added to what it kept already, so one that's
    /// ended keeps its place, and one now [`unplaced`] keeps the place it
    /// had. One `unwindowed` closed, running on with its window closed by
    /// hand, is marked `closed` when first seen so, and let go once it has
    /// been for [`CLOSED_FOR`]; one unsure keeps its place, a mark set
    /// before running its course, since its window went as its whole
    /// terminal did, and one with none is marked [`ORPHANED`], never then
    /// taken as closed until it shows again. A marker for a window with no
    /// place never stands over a place this boot or the one restored has.
    #[must_use]
    pub fn kept(
        &self,
        boot: &str,
        restoring: &str,
        now: Map<String, Value>,
        others: Vec<Value>,
        unwindowed: &Windowless,
        at: u64,
    ) -> Self {
        let mut this = self
            .boots
            .get(boot)
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        this.retain(|id, place| {
            let Some(place) = place.as_object_mut() else {
                return !unwindowed.closed.contains(id);
            };
            closed_from(place, at);
            // One in a window again keeps its place as it was kept, unmarked.
            // One heard parked, ended or elsewhere keeps a closed mark and
            // lets its place go once that's due, since the host parks an
            // idle one minutes after Joel closes its window, or sooner while
            // short of memory. Its orphaned mark stays as well, since the
            // host parks idle ones soonest while short of memory, as a crash
            // taking every viewer most often is, and one woken windowless
            // later would read as closed by hand. One not heard, as before
            // the node's sessions are, keeps its marks.
            let windowless = unwindowed.closed.contains(id) || unwindowed.unsure.contains(id);
            if now.contains_key(id) {
                place.remove("closed");
                place.remove(ORPHANED);
                return true;
            }
            if !windowless && unwindowed.heard.contains(id) {
                return place
                    .get("closed")
                    .and_then(Value::as_u64)
                    .is_none_or(|closed| held(closed, at));
            }
            if unwindowed.unsure.contains(id) && !place.contains_key("closed") {
                place.insert(ORPHANED.to_owned(), json!(true));
            }
            let closed = if unwindowed.closed.contains(id) && !place.contains_key(ORPHANED) {
                place.entry("closed").or_insert_with(|| json!(at)).as_u64()
            } else {
                place.get("closed").and_then(Value::as_u64)
            };
            closed.is_none_or(|closed| held(closed, at))
        });
        let restored = self
            .before(boot, restoring)
            .and_then(|before| self.boots.get(before))
            .and_then(Value::as_object);
        for (id, place) in now {
            let placed_before = restored
                .and_then(|those| those.get(&id))
                .filter(|there| there.get("unplaced").is_none());
            if place["unplaced"] == true && this.contains_key(&id) {
                continue;
            }
            // One with no place now that the boot being restored placed
            // keeps that place here too, unmarked, so it goes back there all
            // this boot and the boot after this one still has a place for it.
            if let (true, Some(there)) = (place["unplaced"] == true, placed_before) {
                let mut there = there.clone();
                if let Some(marks) = there.as_object_mut() {
                    marks.remove("closed");
                    marks.remove(ORPHANED);
                }
                this.insert(id, there);
                continue;
            }
            this.insert(id, place);
        }
        let mut kept = Self::default();
        kept.boots.insert(boot.to_owned(), Value::Object(this));
        kept.others.insert(boot.to_owned(), Value::Array(others));
        if let Some(before) = self.before(boot, restoring) {
            kept.boots
                .insert(before.to_owned(), self.boots[before].clone());
            if let Some(Value::Array(those)) = self.others.get(before) {
                kept.others
                    .insert(before.to_owned(), Value::Array(those.clone()));
            }
        }
        kept
    }

    /// It with `boot`'s places closed by hand for [`CLOSED_FOR`] by `at`
    /// let go, and nothing else changed, for while no window is listed to
    /// work the rest out from: a place closed before the last window went
    /// still goes when due, never reopened at the next boot. A mark
    /// stamped ahead of `at` counts from `at`, as [`Self::kept`] has it.
    #[must_use]
    pub fn let_go(&self, boot: &str, at: u64) -> Self {
        let mut kept = self.clone();
        if let Some(Value::Object(places)) = kept.boots.get_mut(boot) {
            // The same rules as kept()'s, so a place goes on one schedule
            // whether or not a window is listed.
            places.retain(|_, place| {
                place
                    .as_object_mut()
                    .and_then(|place| closed_from(place, at))
                    .is_none_or(|closed| held(closed, at))
            });
        }
        kept
    }

    /// When, in ms since the epoch, the first of `boot`'s places marked
    /// closed is to be let go, or none while none is.
    #[must_use]
    pub fn let_go_at(&self, boot: &str) -> Option<u64> {
        self.boots
            .get(boot)?
            .as_object()?
            .values()
            .filter_map(|place| place["closed"].as_u64())
            .min()
            .map(|closed| closed.saturating_add(closed_for()))
    }

    /// The sessions the boot being restored kept a place of, by pi's id.
    pub fn arranged(&self, restoring: &str) -> HashSet<String> {
        self.boots
            .get(restoring)
            .and_then(Value::as_object)
            .map(|places| places.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Writes it to `path` whole, renamed into place, so a daemon started
    /// again reads it as it was.
    pub fn write(&self, path: &Path) -> io::Result<()> {
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)?;
        }
        let writing = path.with_extension("json.new");
        fs::write(&writing, self.said().to_string())?;
        fs::rename(&writing, path)
    }

    /// Its record, as the shell reads it to put the sessions back.
    pub fn record(&self) -> Value {
        json!({"key": format!("{KIND}:{PLACE}"), "kind": KIND, "arrangement": self.said()})
    }
}

/// [`CLOSED_FOR`] in ms, as places are stamped.
fn closed_for() -> u64 {
    u64::try_from(CLOSED_FOR.as_millis()).unwrap_or(u64::MAX)
}

/// When `place`'s closed mark counts from at `at`, if it has one. A mark
/// stamped ahead of `at`, as after the wall clock stepped back, is
/// stamped `at`, so its place goes a boot's end from here, never held
/// for as long as the clock stepped. So is one that isn't a time in ms,
/// as a hand edit may leave, which would otherwise hold it for good.
fn closed_from(place: &mut Map<String, Value>, at: u64) -> Option<u64> {
    let closed = place.get("closed")?.as_u64().unwrap_or(u64::MAX);
    if closed > at {
        place.insert("closed".to_owned(), json!(at));
        return Some(at);
    }
    Some(closed)
}

/// Whether a place closed by hand at `closed` is still held at `at`.
fn held(closed: u64, at: u64) -> bool {
    at.saturating_sub(closed) < closed_for()
}

/// How long from `now` until the places are kept again for the first
/// closed to be let go at `let_go`, both in ms since the epoch: never
/// sooner than [`STILL`], so one already past can't wake the daemon at
/// once over and over.
#[must_use]
pub fn again_in(let_go: u64, now: u64) -> Duration {
    Duration::from_millis(let_go.saturating_sub(now)).max(STILL)
}

/// When the windows were last seen to move, so their places are kept only
/// once they've been still [`STILL`], and when a place closed is to be let
/// go, so it's kept again then though nothing moves.
#[derive(Debug, Default)]
pub struct Still {
    moved: Option<Instant>,
    again: Option<Instant>,
}

impl Still {
    /// The windows moved at `at`.
    pub fn moved(&mut self, at: Instant) {
        self.moved = Some(at);
    }

    /// How long from `at` until they've been still long enough, or none
    /// while nothing waits to be kept. A place to let go while they move
    /// waits until they're still, when keeping them asks again.
    pub fn wait(&self, at: Instant) -> Option<Duration> {
        self.moved
            .map(|moved| moved + STILL)
            .or(self.again)
            .map(|due| due.saturating_duration_since(at))
    }

    /// Its places are to be kept again at `at`, as a place closed is let
    /// go then.
    pub fn again(&mut self, at: Instant) {
        self.again = Some(at);
    }

    /// Whether they've been still long enough at `at`, after which nothing
    /// waits until they move again.
    pub fn due(&mut self, at: Instant) -> bool {
        let due = self.wait(at) == Some(Duration::ZERO);
        if due {
            self.moved = None;
            self.again = None;
        }
        due
    }
}

#[cfg(test)]
mod tests;
