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
//! is reopened: the screen, by what it is (Places.js's described) and by
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
use std::time::{Duration, Instant};

use daemon_serve::watch::Held;
use serde_json::{Map, Value, json};

use crate::file;
use crate::served::PLACE;
use crate::session::{Lives, Session};

/// The kind it's served as.
pub const KIND: &str = "arrangement";

/// How long the windows stay still before their places are kept, as the
/// shell waited.
pub const STILL: Duration = Duration::from_secs(2);

/// Each lit screen's connector and what the screen is, from the windows
/// daemon's Screens rows, each sent whole under `row` and holding niri's
/// record of its output. One
/// that's off, as the laptop's is while its lid is shut on the dock, has
/// no place in the layout, so it's as gone as one unplugged.
pub fn screens(rows: &Held) -> HashMap<String, String> {
    rows.records()
        .filter_map(|(_, row)| {
            let held = &row["row"]["held"];
            let niri = &held["niri"];
            let output = held["output"].as_str()?;
            (niri.is_object() && !niri["logical"].is_null())
                .then(|| (output.to_owned(), described(niri)))
        })
        .collect()
}

/// A screen as niri describes it, as its config names one: "HP Inc. HP
/// Z27k G3 CN41282GC1".
fn described(output: &Value) -> String {
    ["make", "model", "serial"]
        .iter()
        .filter_map(|part| output[part].as_str().filter(|said| !said.is_empty()))
        .collect::<Vec<_>>()
        .join(" ")
}

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
        place.insert("column".into(), json!(column));
        place.insert("tile".into(), json!(tile));
        Some(place)
    }
}

/// Where each running session here sits now, by pi's id for it: its window
/// as `paired` names it by the session's address (the `agent-window`
/// records), and whether it leads one of the sessions running or parked.
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
    sessions
        .filter(|one| one.here() && one.lives() == Lives::Running)
        .filter_map(|one| {
            let id = one.presence()?.get("session")?.as_str()?;
            let window = desk.windows.get(windows.get(one.address())?)?;
            let addresses = one.presence()?.get("addresses").and_then(Value::as_array);
            let leads = addresses.map_or_else(
                || leaders.contains(&one.address().to_uppercase()),
                |all| {
                    all.iter()
                        .filter_map(Value::as_str)
                        .any(|address| leaders.contains(&address.to_uppercase()))
                },
            );
            let mut place = desk.at(window, Map::new())?;
            place.insert("leads".into(), json!(leads));
            Some((id.to_owned(), Value::Object(place)))
        })
        .collect()
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

    /// The places kept once `now` and `others` are, this boot's added to
    /// what it kept already, so one that's ended keeps its place.
    #[must_use]
    pub fn kept(
        &self,
        boot: &str,
        restoring: &str,
        now: Map<String, Value>,
        others: Vec<Value>,
    ) -> Self {
        let mut this = self
            .boots
            .get(boot)
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        this.extend(now);
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

/// When the windows were last seen to move, so their places are kept only
/// once they've been still [`STILL`].
#[derive(Debug, Default)]
pub struct Still {
    moved: Option<Instant>,
}

impl Still {
    /// The windows moved at `at`.
    pub fn moved(&mut self, at: Instant) {
        self.moved = Some(at);
    }

    /// How long from `at` until they've been still long enough, or none
    /// while nothing waits to be kept.
    pub fn wait(&self, at: Instant) -> Option<Duration> {
        self.moved
            .map(|moved| (moved + STILL).saturating_duration_since(at))
    }

    /// Whether they've been still long enough at `at`, after which nothing
    /// waits until they move again.
    pub fn due(&mut self, at: Instant) -> bool {
        let due = self.wait(at) == Some(Duration::ZERO);
        if due {
            self.moved = None;
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
            {"key": "workspace:1", "id": 1, "idx": 3, "output": "DP-4"},
            {"key": "workspace:2", "id": 2, "idx": 1, "output": "eDP-1"}
        ]));
        // Each as the windows daemon sends it: the row whole, under `row`.
        let screens = screens(&held(&json!([
            {"key": "row:displays:display:DP-4", "kind": "row", "row": {"held": {
             "output": "DP-4",
             "niri": {"make": "HP Inc.", "model": "HP Z27k G3", "serial": "CN41", "logical": {}}}}},
            {"key": "row:displays:display:eDP-1", "kind": "row", "row": {"held": {
             "output": "eDP-1",
             "niri": {"make": "LG Display", "model": "0x07C6", "serial": null, "logical": {}}}}},
            {"key": "row:displays:display:HDMI-1", "kind": "row", "row": {"held": {
             "output": "HDMI-1", "niri": {"make": "Dell", "model": "U2720", "logical": null}}}},
            {"key": "row:displays:off", "kind": "row", "row": {"held": {}}}
        ])));
        (windows, workspaces, screens)
    }

    fn session(address: &str, id: &str, standing: &str, under: &str) -> Value {
        json!({"key": format!("session:{address}"), "standing": standing, "here": true,
               "presence": {"addresses": [address], "session": id, "orchestratedBy": under}})
    }

    #[test]
    fn only_lit_screens_are_named_by_what_they_are() {
        let (_, _, screens) = desk_parts();
        assert_eq!(
            screens,
            HashMap::from([
                ("DP-4".into(), "HP Inc. HP Z27k G3 CN41".into()),
                ("eDP-1".into(), "LG Display 0x07C6".into()),
            ]),
            "a screen that's off, and the row that isn't a screen, are left out"
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
                           "workspace": 3, "column": 1, "tile": 2, "leads": true},
                "s-kelp": {"screen": "HP Inc. HP Z27k G3 CN41", "output": "DP-4",
                           "workspace": 3, "column": 2, "tile": 1, "leads": false}
            }),
            "one leading another says so; a closed one and one with no window aren't kept"
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
        let after = kept.kept("this", "last", now, vec![json!({"app": "chrome"})]);
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
