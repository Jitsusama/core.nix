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
fn id_of(one: &Session) -> Option<String> {
    one.presence()
        .and_then(|presence| presence.get("session"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

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
/// names.
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
    let terminal = |one: &Session| {
        if one.opened().is_empty() {
            kitties.get(one.address()).copied()
        } else {
            Some(SESSIONS_KITTY)
        }
    };
    let lit: HashSet<Option<&str>> = running
        .iter()
        .filter(|one| shown(one))
        .map(terminal)
        .collect();
    let mut seen = Windowless {
        heard: sessions.iter().filter_map(id_of).collect(),
        ..Windowless::default()
    };
    for one in running
        .iter()
        .filter(|one| !shown(one) && !untold.contains(one.address()))
    {
        let Some(id) = id_of(one) else {
            continue;
        };
        if lit.contains(&terminal(one)) {
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
    /// been for [`CLOSED_FOR`]; one unsure keeps its place as it is, a mark
    /// set before running its course, since its window went as its whole
    /// terminal did.
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
            // One in a window again, parked, ended or gone keeps its place
            // as it was kept, unmarked, so no mark outlives what set it and
            // comes due at once, again and again.
            let windowless = unwindowed.closed.contains(id) || unwindowed.unsure.contains(id);
            if now.contains_key(id) || !windowless {
                place.remove("closed");
                return true;
            }
            let closed = if unwindowed.closed.contains(id) {
                place.entry("closed").or_insert_with(|| json!(at)).as_u64()
            } else {
                place.get("closed").and_then(Value::as_u64)
            };
            closed.is_none_or(|closed| at.saturating_sub(closed) < closed_for())
        });
        for (id, place) in now {
            if place["unplaced"] == true && this.contains_key(&id) {
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
mod tests {
    use super::*;

    fn held(records: &Value) -> Held {
        let mut held = Held::default();
        held.took(&json!({ "records": records }));
        held
    }

    fn desk_parts() -> (Held, Held, HashMap<String, String>) {
        let windows = held(&json!([
            {"key": "window:7", "id": 7, "app_id": "kitty", "opened": "pi-session.7",
             "workspace": 1, "column": 2, "down": 1, "floating": false},
            {"key": "window:3", "id": 3, "app_id": "google-chrome", "title": "Charts - Chrome",
             "workspace": 2, "column": 1, "down": 1, "floating": false},
            {"key": "window:5", "id": 5, "app_id": "slack", "title": "Slack",
             "workspace": 2, "column": null, "down": null, "floating": true},
            {"key": "window:9", "id": 9, "app_id": "kitty", "opened": "pi-session.9",
             "workspace": 1, "column": 1, "down": 2, "floating": false}
        ]));
        let workspaces = held(&json!([
            {"key": "workspace:1", "id": 1, "idx": 3, "name": "LEAD", "output": "DP-4"},
            {"key": "workspace:2", "id": 2, "idx": 1, "output": "eDP-1"}
        ]));
        // Each as the windows daemon sends it: the row whole, under `row`.
        let screens = screens(&held(&json!([
            {"key": "row:displays:display:DP-4", "kind": "row", "row": {"held": {
             "output": "DP-4", "screen": "HP Inc. HP Z27k G3 CN41", "standing": "lit"}}},
            {"key": "row:displays:display:eDP-1", "kind": "row", "row": {"held": {
             "output": "eDP-1", "screen": "LG Display 0x07C6", "standing": "dark"}}},
            {"key": "row:displays:display:HDMI-1", "kind": "row", "row": {"held": {
             "output": "HDMI-1", "screen": "Dell U2720", "standing": "off"}}},
            {"key": "row:displays:answer:scale:DP-4", "kind": "row", "row": {"held": {
             "output": "DP-4", "part": "scale"}}},
            {"key": "row:displays:display:DP-9", "kind": "row", "row": {"held": {
             "output": "DP-9"}}},
            {"key": "row:displays:off", "kind": "row", "row": {"held": {}}}
        ])));
        (windows, workspaces, screens)
    }

    /// A pairing that couldn't tell `address`'s window from another's.
    fn untold(address: &str) -> Value {
        let mut one = json!({"address": address, "window": null});
        one[UNTOLD] = json!(true);
        one
    }

    /// A pairing in the kitty whose socket is `unix:@kitty-<pid>`.
    fn in_kitty(mut one: Value, pid: &str) -> Value {
        one["kitty"] = json!({"id": "1", "within": format!("unix:@kitty-{pid}")});
        one
    }

    fn session(address: &str, id: &str, standing: &str, under: &str) -> Value {
        json!({"key": format!("session:{address}"), "standing": standing, "here": true,
               "presence": {"addresses": [address], "session": id, "orchestratedBy": under}})
    }

    #[test]
    fn only_screens_laid_out_are_named_by_what_they_are() {
        let (_, _, screens) = desk_parts();
        assert_eq!(
            screens,
            HashMap::from([
                ("DP-4".into(), "HP Inc. HP Z27k G3 CN41".into()),
                ("eDP-1".into(), "LG Display 0x07C6".into()),
            ]),
            "a screen that's off or says not what it is, and rows that aren't screens, are left out"
        );
    }

    #[test]
    fn each_running_session_here_is_kept_where_its_window_sits() {
        let (windows, workspaces, screens) = desk_parts();
        let desk = Desk {
            windows: &windows,
            workspaces: &workspaces,
            screens: &screens,
        };
        let records = [
            session("LEAD", "s-lead", "running", ""),
            session("KELP", "s-kelp", "running", "lead"),
            session("GONE", "s-gone", "closed", ""),
            session("LOST", "s-lost", "running", ""),
        ];
        let paired = [
            json!({"address": "LEAD", "window": "window:9"}),
            json!({"address": "KELP", "window": "window:7"}),
            json!({"address": "GONE", "window": "window:3"}),
            json!({"address": "LOST", "window": null}),
        ];
        let kept = now(records.iter().map(Session), &paired, &desk);
        assert_eq!(
            Value::Object(kept),
            json!({
                "s-lead": {"screen": "HP Inc. HP Z27k G3 CN41", "output": "DP-4",
                           "workspace": 3, "workspaceName": "LEAD", "column": 1,
                           "tile": 2, "leads": true},
                "s-kelp": {"screen": "HP Inc. HP Z27k G3 CN41", "output": "DP-4",
                           "workspace": 3, "workspaceName": "LEAD", "column": 2,
                           "tile": 1, "leads": false, "under": "LEAD"}
            }),
            "one leading another says so, and one under another names it; each on a named \
             workspace keeps its name, since named ones come back in another order; a closed \
             one and one with no window aren't kept"
        );
    }

    #[test]
    fn other_tiled_windows_are_kept_by_program_and_title_oldest_first() {
        let (windows, workspaces, screens) = desk_parts();
        let desk = Desk {
            windows: &windows,
            workspaces: &workspaces,
            screens: &screens,
        };
        assert_eq!(
            others(&desk),
            [json!({"app": "google-chrome", "title": "Charts - Chrome",
                    "screen": "LG Display 0x07C6", "output": "eDP-1",
                    "workspace": 1, "column": 1, "tile": 1})],
            "kitty's windows and a floating one aren't kept"
        );
    }

    #[test]
    fn this_boots_places_join_those_kept_and_the_boot_before_stays() {
        let kept = Arrangement::of(&json!({"v": 1,
            "boots": {"this": {"s-ended": {"column": 4}}, "last": {"s-old": {"column": 1}},
                      "older": {"s-older": {"column": 2}}},
            "others": {"last": [{"app": "slack"}]}}));
        let now = Map::from_iter([("s-new".to_owned(), json!({"column": 3}))]);
        let after = kept.kept(
            "this",
            "last",
            now,
            vec![json!({"app": "chrome"})],
            &Windowless::default(),
            0,
        );
        assert_eq!(
            after.said(),
            json!({"v": 1,
                "boots": {"this": {"s-ended": {"column": 4}, "s-new": {"column": 3}},
                          "last": {"s-old": {"column": 1}}},
                "others": {"this": [{"app": "chrome"}], "last": [{"app": "slack"}]}}),
            "one ended keeps its place, the boot being restored stays, an older one goes"
        );
        assert_eq!(
            after.arranged("last"),
            HashSet::from(["s-old".to_owned()]),
            "the boot being restored kept a place of these"
        );
        assert_eq!(
            Arrangement::of(&json!({"v": 2, "boots": {}})),
            Arrangement::default(),
            "another version is kept as none"
        );
    }

    #[test]
    fn a_session_running_here_with_no_window_is_told_apart() {
        let (windows, workspaces, screens) = desk_parts();
        let desk = Desk {
            windows: &windows,
            workspaces: &workspaces,
            screens: &screens,
        };
        let mut away = session("AWAY", "s-away", "running", "");
        away["here"] = json!(false);
        let records = [
            session("SHOWN", "s-shown", "running", ""),
            session("SHUT", "s-shut", "running", ""),
            session("LAPSED", "s-lapsed", "running", ""),
            session("PARKED", "s-parked", "parked", ""),
            session("ENDED", "s-ended", "closed", ""),
            session("UNTOLD", "s-untold", "running", ""),
            away,
        ];
        let paired = [
            in_kitty(json!({"address": "SHOWN", "window": "window:7"}), "5"),
            in_kitty(json!({"address": "SHUT", "window": null}), "5"),
            in_kitty(json!({"address": "LAPSED", "window": "window:44"}), "5"),
            untold("UNTOLD"),
        ];
        let seen = unwindowed(records.iter().map(Session), &paired, &desk);
        assert_eq!(
            seen.closed,
            HashSet::from(["s-shut".to_owned(), "s-lapsed".to_owned()]),
            "one with no window paired, or one niri no longer lists; never one shown, parked, \
             ended or elsewhere, nor one whose window can't be told from another's"
        );
        assert_eq!(
            seen.heard,
            [
                "s-shown", "s-shut", "s-lapsed", "s-parked", "s-ended", "s-untold", "s-away"
            ]
            .into_iter()
            .map(str::to_owned)
            .collect::<HashSet<_>>(),
            "every session the node said is heard, however it stands"
        );
    }

    #[test]
    fn plain_pis_are_told_apart_by_the_kitty_drawing_them() {
        let (windows, workspaces, screens) = desk_parts();
        let desk = Desk {
            windows: &windows,
            workspaces: &workspaces,
            screens: &screens,
        };
        // NAMELESS is shown with no kitty said, which windows.rs never
        // pairs, so it lights no terminal for BARE, whose kitty is unknown.
        let records = [
            session("SHOWN", "s-shown", "running", ""),
            session("SHUT", "s-shut", "running", ""),
            session("OTHER", "s-other", "running", ""),
            session("NAMELESS", "s-nameless", "running", ""),
            session("BARE", "s-bare", "running", ""),
        ];
        let paired = [
            in_kitty(json!({"address": "SHOWN", "window": "window:7"}), "5"),
            in_kitty(json!({"address": "SHUT", "window": null}), "5"),
            in_kitty(json!({"address": "OTHER", "window": null}), "9"),
            json!({"address": "NAMELESS", "window": "window:9"}),
        ];
        let seen = unwindowed(records.iter().map(Session), &paired, &desk);
        assert_eq!(
            (seen.closed, seen.unsure),
            (
                HashSet::from(["s-shut".to_owned()]),
                HashSet::from(["s-other".to_owned(), "s-bare".to_owned()]),
            ),
            "a pi gone from a kitty still drawing another was closed by hand; one whose kitty \
             draws none, or that no kitty is known for, may have gone with it"
        );
    }

    #[test]
    fn with_no_session_window_left_none_is_taken_as_closed() {
        let (windows, workspaces, screens) = desk_parts();
        let desk = Desk {
            windows: &windows,
            workspaces: &workspaces,
            screens: &screens,
        };
        let records = [
            session("SHUT", "s-shut", "running", ""),
            session("LAPSED", "s-lapsed", "running", ""),
        ];
        let paired = [
            in_kitty(json!({"address": "SHUT", "window": null}), "5"),
            in_kitty(json!({"address": "LAPSED", "window": "window:44"}), "5"),
        ];
        assert_eq!(
            unwindowed(records.iter().map(Session), &paired, &desk).closed,
            HashSet::new(),
            "every session's window gone at once is their kitty or niri gone, as a crash \
             takes them, never windows closed by hand"
        );
    }

    #[test]
    fn a_terminal_drawing_none_of_its_sessions_is_gone_not_closed_by_hand() {
        let (windows, workspaces, screens) = desk_parts();
        let desk = Desk {
            windows: &windows,
            workspaces: &workspaces,
            screens: &screens,
        };
        let hosted = |address: &str, id: &str| {
            let mut one = session(address, id, "running", "");
            one["presence"]["hosted"] = json!(true);
            one
        };
        // Every viewer is a window of the sessions kitty; OWN is a pi in a
        // kitty of Joel's own, still shown as the sessions kitty crashes.
        let records = [
            hosted("REEF", "s-reef"),
            hosted("KELP", "s-kelp"),
            session("OWN", "s-own", "running", ""),
        ];
        let own = json!({"address": "OWN", "window": "window:3",
                         "kitty": {"id": "1", "within": "unix:@kitty-5"}});
        let crashed = unwindowed(records.iter().map(Session), &[own.clone()], &desk);
        assert_eq!(
            (crashed.closed, crashed.unsure),
            (
                HashSet::new(),
                HashSet::from(["s-reef".to_owned(), "s-kelp".to_owned()]),
            ),
            "every window one terminal drew gone at once is that terminal gone, though \
             another's still shows"
        );
        let kelp = json!({"address": "KELP", "window": "window:7"});
        let shut = unwindowed(records.iter().map(Session), &[own, kelp], &desk);
        assert_eq!(
            (shut.closed, shut.unsure),
            (HashSet::from(["s-reef".to_owned()]), HashSet::new()),
            "one viewer gone while another of that terminal's shows was closed by hand"
        );
    }

    #[test]
    fn a_window_that_cant_be_placed_is_kept_as_one_with_no_place() {
        let (windows, workspaces, screens) = desk_parts();
        let desk = Desk {
            windows: &windows,
            workspaces: &workspaces,
            screens: &screens,
        };
        let records = [
            session("LEAD", "s-lead", "running", ""),
            session("UNTOLD", "s-untold", "running", ""),
            session("FLOAT", "s-float", "running", ""),
            session("LOST", "s-lost", "running", ""),
            session("ENDED", "s-ended", "closed", ""),
        ];
        let paired = [
            json!({"address": "LEAD", "window": "window:9"}),
            untold("UNTOLD"),
            json!({"address": "FLOAT", "window": "window:5"}),
            json!({"address": "LOST", "window": null}),
            untold("ENDED"),
        ];
        let kept = now(records.iter().map(Session), &paired, &desk);
        let unplaced = json!({"unplaced": true});
        assert_eq!(
            (
                kept.get("s-untold"),
                kept.get("s-float"),
                kept.get("s-lost"),
                kept.get("s-ended"),
            ),
            (Some(&unplaced), Some(&unplaced), None, None),
            "one whose window can't be told apart, or floats, has a window all the same, so \
             Reopen Them All brings it back in one; one with none, or ended, has no place"
        );
        assert!(kept["s-lead"]["column"] == 1, "one tiled is placed");
    }

    #[test]
    fn one_with_no_place_now_keeps_the_place_it_had() {
        let kept = Arrangement::of(&json!({"v": 1,
            "boots": {"this": {"s-tiled": {"column": 4, "closed": 1_000}}}}));
        let now = Map::from_iter([
            ("s-tiled".to_owned(), json!({"unplaced": true})),
            ("s-new".to_owned(), json!({"unplaced": true})),
        ]);
        let after = kept.kept("this", "", now, Vec::new(), &Windowless::default(), 2_000);
        assert_eq!(
            after.boots["this"],
            json!({"s-tiled": {"column": 4}, "s-new": {"unplaced": true}}),
            "one seen in a window it can't be placed by keeps where it last sat, unmarked, \
             since its window is back; one never placed is kept as having a window"
        );
    }

    #[test]
    fn a_mark_stands_while_its_terminal_draws_none() {
        let kept = Arrangement::of(&json!({"v": 1,
            "boots": {"this": {"s-shut": {"column": 4, "closed": 1_000},
                               "s-last": {"column": 5}}}}));
        let unsure = Windowless {
            unsure: HashSet::from(["s-shut".to_owned(), "s-last".to_owned()]),
            ..Windowless::default()
        };
        let grace = u64::try_from(CLOSED_FOR.as_millis()).expect("minutes fit");
        let first = kept.kept("this", "", Map::new(), Vec::new(), &unsure, 2_000);
        assert_eq!(
            first.boots["this"], kept.boots["this"],
            "the last window closing leaves each place as it was: a mark already set \
             stands, and none is set"
        );
        let gone = first.kept("this", "", Map::new(), Vec::new(), &unsure, 1_000 + grace);
        assert_eq!(
            gone.boots["this"],
            json!({"s-last": {"column": 5}}),
            "one marked while others showed is let go when its time comes"
        );
    }

    #[test]
    fn a_window_shown_again_leaves_the_rest_its_terminal_lost_unsure() {
        let kept = Arrangement::of(&json!({"v": 1,
            "boots": {"this": {"s-reef": {"column": 1}, "s-kelp": {"column": 2}}}}));
        let both = HashSet::from(["s-reef".to_owned(), "s-kelp".to_owned()]);
        let crashed = Windowless {
            unsure: both.clone(),
            heard: both.clone(),
            ..Windowless::default()
        };
        let kelp_back = Windowless {
            closed: HashSet::from(["s-reef".to_owned()]),
            heard: both.clone(),
            ..Windowless::default()
        };
        let kelp = || Map::from_iter([("s-kelp".to_owned(), json!({"column": 3}))]);
        let grace = u64::try_from(CLOSED_FOR.as_millis()).expect("minutes fit");
        let first = kept.kept("this", "", Map::new(), Vec::new(), &crashed, 1_000);
        let second = first.kept("this", "", kelp(), Vec::new(), &kelp_back, 2_000);
        assert_eq!(
            second.boots["this"]["s-reef"].get("closed"),
            None,
            "one whose window went with its whole terminal isn't taken as closed by hand when \
             another of that terminal's is opened again"
        );
        let later = second.kept("this", "", kelp(), Vec::new(), &kelp_back, 2_000 + grace);
        assert!(
            later.boots["this"].get("s-reef").is_some(),
            "so its place stays"
        );
        let reef_shown = Map::from_iter([
            ("s-reef".to_owned(), json!({"column": 1})),
            ("s-kelp".to_owned(), json!({"column": 3})),
        ]);
        let heard = Windowless {
            heard: both,
            ..Windowless::default()
        };
        let shown = later.kept("this", "", reef_shown, Vec::new(), &heard, 3_000);
        let shut = shown.kept("this", "", kelp(), Vec::new(), &kelp_back, 4_000);
        assert_eq!(
            shut.boots["this"]["s-reef"]["closed"],
            json!(4_000),
            "once shown again, closing it by hand marks it"
        );
    }

    #[test]
    fn a_mark_stands_for_one_the_node_hasnt_said() {
        let kept = Arrangement::of(&json!({"v": 1,
            "boots": {"this": {"s-shut": {"column": 4, "closed": 1_000}}}}));
        let grace = u64::try_from(CLOSED_FOR.as_millis()).expect("minutes fit");
        let unheard = Windowless::default();
        assert_eq!(
            kept.kept("this", "", Map::new(), Vec::new(), &unheard, 2_000)
                .boots["this"],
            kept.boots["this"],
            "a keep before the node's sessions are heard leaves each mark as it was"
        );
        assert_eq!(
            kept.kept("this", "", Map::new(), Vec::new(), &unheard, 1_000 + grace)
                .boots["this"],
            json!({}),
            "and one comes due all the same"
        );
    }

    #[test]
    fn one_with_no_place_now_leaves_the_restored_boots_place_to_stand() {
        let kept = Arrangement::of(&json!({"v": 1,
            "boots": {"this": {}, "last": {"s-float": {"column": 2}}}}));
        let now = Map::from_iter([
            ("s-float".to_owned(), json!({"unplaced": true})),
            ("s-new".to_owned(), json!({"unplaced": true})),
        ]);
        let after = kept.kept("this", "last", now, Vec::new(), &Windowless::default(), 0);
        assert_eq!(
            after.boots["this"],
            json!({"s-new": {"unplaced": true}}),
            "one the boot being restored placed keeps going back there, rather than home"
        );
    }

    #[test]
    fn a_let_go_already_past_is_looked_at_no_sooner_than_still() {
        assert_eq!(
            again_in(10_000, 4_000),
            Duration::from_secs(6),
            "one still to come is looked at when it comes"
        );
        assert_eq!(
            again_in(4_000, 10_000),
            STILL,
            "one gone by waits as long as the windows' stillness, never waking the daemon at \
             once over and over"
        );
    }

    #[test]
    fn a_window_closed_while_the_boot_runs_on_lets_its_place_go() {
        let kept = Arrangement::of(&json!({"v": 1,
            "boots": {"this": {"s-shut": {"column": 4}, "s-ended": {"column": 5}}}}));
        let shut = Windowless {
            closed: HashSet::from(["s-shut".to_owned()]),
            ..Windowless::default()
        };
        let at = 1_000_000;
        let grace = u64::try_from(CLOSED_FOR.as_millis()).expect("minutes fit");
        let first = kept.kept("this", "", Map::new(), Vec::new(), &shut, at);
        assert_eq!(
            first.boots["this"],
            json!({"s-shut": {"column": 4, "closed": at}, "s-ended": {"column": 5}}),
            "one whose window closed while its session runs on is marked so; one that ended \
             keeps its place, as a boot's end ends them"
        );
        assert_eq!(
            first.let_go_at("this"),
            Some(at + grace),
            "looked at again once a boot's end would have taken it"
        );
        let later = first.kept("this", "", Map::new(), Vec::new(), &shut, at + grace - 1);
        assert_eq!(
            later, first,
            "within that, its first close is kept, not each look"
        );
        let gone = later.kept("this", "", Map::new(), Vec::new(), &shut, at + grace);
        assert_eq!(
            gone.boots["this"],
            json!({"s-ended": {"column": 5}}),
            "past it, its place is let go, so Reopen Them All brings it back with no window"
        );
        assert_eq!(gone.let_go_at("this"), None, "and nothing waits");
        let back = first.kept(
            "this",
            "",
            Map::from_iter([("s-shut".to_owned(), json!({"column": 2}))]),
            Vec::new(),
            &Windowless::default(),
            at + 1,
        );
        assert_eq!(
            back.boots["this"]["s-shut"],
            json!({"column": 2}),
            "a window opened again is kept afresh, unmarked"
        );
        let parked = first.kept(
            "this",
            "",
            Map::new(),
            Vec::new(),
            &Windowless {
                heard: HashSet::from(["s-shut".to_owned()]),
                ..Windowless::default()
            },
            at + 1,
        );
        assert_eq!(
            parked.boots["this"]["s-shut"],
            json!({"column": 4}),
            "one no longer running here without a window, parked or ended, is kept unmarked, \
             as before it was marked"
        );
        assert_eq!(parked.let_go_at("this"), None, "so nothing waits on it");
    }

    #[test]
    fn the_boot_before_is_the_one_restored_else_the_other_kept() {
        let kept = Arrangement::of(&json!({"v": 1, "boots": {"this": {}, "last": {}}}));
        assert_eq!(
            kept.before("this", ""),
            Some("last"),
            "none restoring: the other"
        );
        assert_eq!(
            kept.before("this", "this"),
            Some("last"),
            "this boot is never before"
        );
        assert_eq!(
            kept.before("this", "never"),
            Some("last"),
            "one not kept isn't it"
        );
        assert_eq!(
            Arrangement::default().before("this", ""),
            None,
            "none kept, none"
        );
    }

    #[test]
    fn places_are_kept_once_the_windows_are_still() {
        let start = Instant::now();
        let mut still = Still::default();
        assert_eq!(still.wait(start), None, "nothing moved, nothing waits");
        still.moved(start);
        let half = Duration::from_millis(500);
        assert_eq!(
            still.wait(start + half),
            Some(Duration::from_millis(1500)),
            "the rest of the wait"
        );
        assert!(
            !still.due(start + Duration::from_secs(1)),
            "not still long enough"
        );
        still.moved(start + Duration::from_secs(1));
        assert!(!still.due(start + STILL), "a move starts the wait again");
        assert!(
            still.due(start + Duration::from_secs(3)),
            "still long enough"
        );
        assert_eq!(
            still.wait(start + Duration::from_secs(4)),
            None,
            "kept, nothing waits"
        );
        still.again(start + Duration::from_secs(60));
        assert_eq!(
            still.wait(start + Duration::from_secs(10)),
            Some(Duration::from_secs(50)),
            "a place to let go later is waited for too"
        );
        still.moved(start + Duration::from_secs(10));
        assert!(
            still.due(start + Duration::from_secs(12)),
            "a move is kept first"
        );
        assert_eq!(
            still.wait(start + Duration::from_secs(12)),
            None,
            "and keeping it asks again when the place goes"
        );
        still.again(start + Duration::from_secs(60));
        assert!(
            still.due(start + Duration::from_secs(60)),
            "once it's time, its place is kept again"
        );
        still.moved(start + Duration::from_secs(70));
        still.again(start + Duration::from_secs(71));
        assert!(
            !still.due(start + Duration::from_secs(71)),
            "a place to let go while the windows move waits until they're still"
        );
        assert!(
            still.due(start + Duration::from_secs(72)),
            "and is kept then"
        );
    }

    #[test]
    fn what_the_shell_kept_is_read_from_its_newest_folder() {
        let shells =
            std::env::temp_dir().join(format!("agents-arrangement-{}", std::process::id()));
        let _ = fs::remove_dir_all(&shells);
        let older = shells.join("older");
        let newer = shells.join("newer");
        fs::create_dir_all(&older).expect("a shell's folder");
        fs::create_dir_all(&newer).expect("another's");
        fs::write(
            older.join("session-arrangement.json"),
            json!({"v": 1, "boots": {"a": {}}}).to_string(),
        )
        .expect("the older is written");
        std::thread::sleep(Duration::from_millis(20));
        fs::write(
            newer.join("session-arrangement.json"),
            json!({"v": 1, "boots": {"b": {}}}).to_string(),
        )
        .expect("the newer is written");
        let read = Arrangement::from_shell(&shells);
        assert_eq!(
            read.boots.keys().collect::<Vec<_>>(),
            ["b"],
            "the newest one's"
        );
        let kept = shells.join("agents").join("arrangement.json");
        read.write(&kept).expect("it's kept");
        assert_eq!(Arrangement::read(&kept), Some(read), "and read back whole");
        assert_eq!(
            Arrangement::from_shell(&shells.join("none")),
            Arrangement::default(),
            "no shell, nothing kept"
        );
        let _ = fs::remove_dir_all(&shells);
    }
}
