//! What niri says of the desktop, heard event by event from its event
//! stream, and the records the shell draws from it: a `window` record of
//! each window, a `workspace` record of each workspace, and one `desktop`
//! record of what holds the keys.
//!
//! A record holds only what a row draws, or places it by, so a change
//! niri tells that no row shows changes no record: a window resizing, or
//! the view scrolling, moves its tile but not its column; and a focus
//! moving changes only the two windows it moved between, and its `order`
//! only the one it moved to.

use std::collections::{BTreeMap, HashMap};

use serde_json::{Value, json};

use crate::entries::{Entries, tidy};

/// What hearing one event did.
#[derive(Debug, PartialEq, Eq)]
pub enum Heard {
    /// It changed what niri holds, so the records may have changed.
    Changed,
    /// It changed nothing records say, or wasn't an event this hears.
    Nothing,
}

/// What niri holds of the desktop, as its events describe it.
#[derive(Debug, Default)]
pub struct Desktop {
    /// Every window by id, as niri last described it.
    windows: BTreeMap<u64, Value>,
    /// Every workspace by id.
    workspaces: BTreeMap<u64, Value>,
    /// The window niri says has the keys, which no window has while a
    /// layer such as the launcher does.
    focused: Option<u64>,
    overview: bool,
    settings_failed: bool,
}

fn id_of(value: &Value) -> Option<u64> {
    value.as_u64()
}

/// A window's app id as the shell knows it, and the one it opened with.
/// A kitty window opened for a session is classed apart, with a number of
/// its own (Sessions.qml's inSessions): pi-spawned for one another
/// session started, so niri's rule opens it without taking the focus
/// (modules/niri/config.kdl), and pi-session for the rest. Each is
/// kitty's all the same to every list, and says it holds a session.
pub fn app_of(opened: &str) -> (&str, bool) {
    let session = ["pi-spawned", "pi-session"].iter().any(|class| {
        opened.strip_prefix(class).is_some_and(|rest| {
            rest.is_empty()
                || rest
                    .strip_prefix('.')
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
    });
    if session {
        ("kitty", true)
    } else {
        (opened, false)
    }
}

/// When a window was last looked at, in milliseconds, or 0 for never.
fn focused_at_ms(window: &Value) -> u64 {
    let stamp = &window["focus_timestamp"];
    let secs = stamp["secs"].as_u64().unwrap_or(0);
    let nanos = stamp["nanos"].as_u64().unwrap_or(0);
    secs * 1000 + nanos / 1_000_000
}

impl Desktop {
    /// Hears one event from niri's stream.
    pub fn hear(&mut self, event: &Value) -> Heard {
        let Some((name, said)) = event.as_object().and_then(|event| event.iter().next()) else {
            return Heard::Nothing;
        };
        match name.as_str() {
            "OverviewOpenedOrClosed" => self.overview = said["is_open"].as_bool() == Some(true),
            "WindowsChanged" => {
                self.focused = None;
                self.windows = said["windows"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|window| Some((id_of(&window["id"])?, window.clone())))
                    .collect();
                self.focused = self
                    .windows
                    .iter()
                    .find(|(_, window)| window["is_focused"] == true)
                    .map(|(id, _)| *id);
            }
            "WindowOpenedOrChanged" => {
                let window = &said["window"];
                let Some(id) = id_of(&window["id"]) else {
                    return Heard::Nothing;
                };
                // One that's focused takes focus from every other, as
                // niri's IPC documents, without an event for each.
                if window["is_focused"] == true {
                    self.focus(Some(id));
                }
                self.windows.insert(id, window.clone());
            }
            "WindowClosed" => {
                let id = id_of(&said["id"]);
                if let Some(id) = id {
                    self.windows.remove(&id);
                }
                if self.focused == id {
                    self.focused = None;
                }
            }
            "WindowFocusChanged" => self.focus(id_of(&said["id"])),
            "WindowFocusTimestampChanged" => {
                if let Some(window) = id_of(&said["id"]).and_then(|id| self.windows.get_mut(&id)) {
                    window["focus_timestamp"] = said["focus_timestamp"].clone();
                }
            }
            "WindowUrgencyChanged" => {
                if let Some(window) = id_of(&said["id"]).and_then(|id| self.windows.get_mut(&id)) {
                    window["is_urgent"] = said["urgent"].clone();
                }
            }
            "WindowLayoutsChanged" => {
                for change in said["changes"].as_array().into_iter().flatten() {
                    if let Some(window) = id_of(&change[0]).and_then(|id| self.windows.get_mut(&id))
                    {
                        window["layout"] = change[1].clone();
                    }
                }
            }
            "WorkspacesChanged" => {
                self.workspaces = said["workspaces"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|workspace| Some((id_of(&workspace["id"])?, workspace.clone())))
                    .collect();
            }
            "WorkspaceActivated" => {
                let Some(id) = id_of(&said["id"]) else {
                    return Heard::Nothing;
                };
                let output = self.workspaces.get(&id).map(|w| w["output"].clone());
                let focused = said["focused"] == true;
                for (each, workspace) in &mut self.workspaces {
                    if Some(&workspace["output"]) == output.as_ref() {
                        workspace["is_active"] = json!(*each == id);
                    }
                    if focused {
                        workspace["is_focused"] = json!(*each == id);
                    }
                }
            }
            "WorkspaceActiveWindowChanged" => {
                if let Some(workspace) =
                    id_of(&said["workspace_id"]).and_then(|id| self.workspaces.get_mut(&id))
                {
                    workspace["active_window_id"] = said["active_window_id"].clone();
                }
            }
            "ConfigLoaded" => self.settings_failed = said["failed"] == true,
            _ => return Heard::Nothing,
        }
        Heard::Changed
    }

    fn focus(&mut self, id: Option<u64>) {
        self.focused = id;
        for (each, window) in &mut self.windows {
            window["is_focused"] = json!(Some(*each) == id);
        }
    }

    /// The focused workspace, the one in front of Joel.
    fn here(&self) -> Option<&Value> {
        self.workspaces.values().find(|w| w["is_focused"] == true)
    }

    /// The window the keys were in: the one niri says has them, else the
    /// focused workspace's active window, as while the launcher has them;
    /// none on an empty workspace.
    pub fn current(&self) -> Option<u64> {
        self.focused
            .or_else(|| self.here().and_then(|w| id_of(&w["active_window_id"])))
    }

    /// Every record, keyed, as it stands now. Each window's program is
    /// looked up once by its app id (`names`, kept until the entries
    /// change).
    pub fn records(&self, entries: &Entries, names: &mut Names) -> Vec<Value> {
        let here = self.here().and_then(|w| id_of(&w["id"]));
        let current = self.current();
        let seen: Vec<(&Value, &str, bool, Named)> = self
            .windows
            .values()
            .map(|window| {
                let opened = window["app_id"].as_str().unwrap_or("");
                let (app, session) = app_of(opened);
                (window, app, session, names.of(entries, app).clone())
            })
            .collect();
        // How many windows each title is, so those alike say their
        // column.
        let titled: Vec<(String, bool)> = seen
            .iter()
            .map(|(window, _, _, named)| {
                let title = window["title"].as_str().unwrap_or("");
                let own = tidy(title, &named.program);
                let shown = if own.is_empty() {
                    named.program.clone()
                } else {
                    own.to_owned()
                };
                (shown, own.is_empty() || own == named.program)
            })
            .collect();
        let mut alike: HashMap<&str, u64> = HashMap::new();
        for (title, _) in &titled {
            *alike.entry(title.as_str()).or_default() += 1;
        }
        let mut records: Vec<Value> = seen
            .iter()
            .zip(&titled)
            .map(|((window, app, session, named), (title, untitled))| {
                let id = id_of(&window["id"]).unwrap_or(0);
                let workspace =
                    id_of(&window["workspace_id"]).and_then(|w| self.workspaces.get(&w));
                let tile = &window["layout"]["pos_in_scrolling_layout"];
                json!({
                    "key": format!("window:{id}"),
                    "order": focused_at_ms(window),
                    "id": id,
                    "app_id": app,
                    "opened": window["app_id"],
                    "session": session,
                    "title": window["title"],
                    "pid": window["pid"],
                    "entry": named.entry,
                    "program": named.program,
                    "owner": named.owner,
                    "named": title,
                    "untitled": untitled,
                    "alike": alike[title.as_str()],
                    "workspace": workspace.map(|w| w["id"].clone()),
                    "workspace_idx": workspace.map(|w| w["idx"].clone()),
                    "elsewhere": workspace.is_some_and(|w| id_of(&w["id"]) != here),
                    "column": tile[0],
                    "down": tile[1],
                    "floating": window["is_floating"] == true,
                    "urgent": window["is_urgent"] == true,
                    "focused": Some(id) == current,
                })
            })
            .collect();
        // The programs on each workspace, by their windows' ids, and each
        // screen's workspaces in niri's order, so a row offering one says
        // what's on it, and the shell offers them in order, without
        // walking every window or sorting.
        let mut inside: HashMap<u64, Vec<&str>> = HashMap::new();
        for (window, _, _, named) in &seen {
            if let Some(workspace) = id_of(&window["workspace_id"]) {
                inside
                    .entry(workspace)
                    .or_default()
                    .push(named.program.as_str());
            }
        }
        let mut spaces: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
        for w in self.workspaces.values() {
            spaces
                .entry(w["output"].as_str().unwrap_or(""))
                .or_default()
                .push(w);
        }
        for on in spaces.values_mut() {
            on.sort_by_key(|w| w["idx"].as_u64().unwrap_or(0));
        }
        records.extend(self.workspaces.values().map(|w| {
            let id = id_of(&w["id"]).unwrap_or(0);
            let programs = inside.get(&id).cloned().unwrap_or_default();
            // niri keeps an empty workspace last on each screen, which is
            // how a new one is made.
            let last = spaces
                .get(w["output"].as_str().unwrap_or(""))
                .and_then(|on| on.last())
                .is_some_and(|l| id_of(&l["id"]) == Some(id));
            json!({
                "key": format!("workspace:{}", w["id"]),
                "id": w["id"],
                "idx": w["idx"],
                "name": w["name"],
                "output": w["output"],
                "active": w["is_active"] == true,
                "focused": w["is_focused"] == true,
                "urgent": w["is_urgent"] == true,
                "active_window": w["active_window_id"],
                "new": last && programs.is_empty(),
                "inside": programs,
            })
        }));
        let spaces: BTreeMap<&str, Vec<Value>> = spaces
            .into_iter()
            .map(|(output, on)| (output, on.iter().map(|w| w["id"].clone()).collect()))
            .collect();
        let front = self.here();
        // The window the keys were in, as what's done to it says it, so
        // Capture watches this record alone rather than every window.
        let window = current.and_then(|id| {
            let at = seen
                .iter()
                .position(|(w, ..)| id_of(&w["id"]) == Some(id))?;
            let (window, app_id, _, program) = &seen[at];
            Some(json!({
                "id": id,
                "title": window["title"],
                "app_id": app_id,
                "entry": program.entry,
                "program": program.program,
            }))
        });
        records.push(json!({
            "key": "desktop",
            "current": current,
            "window": window,
            "workspace": front.map(|w| w["id"].clone()),
            "output": front.map(|w| w["output"].clone()),
            "spaces": spaces,
            "overview": self.overview,
            "settings_failed": self.settings_failed,
        }));
        records
    }
}

/// What an app id names: the entry the lookup finds, the program's name
/// a row says (the entry's, else the app id itself), and the program the
/// window counts for, which a browser's window of its own adds to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Named {
    pub entry: Option<String>,
    pub program: String,
    pub owner: Option<String>,
}

/// Each app id's names, looked up once until the entries change.
#[derive(Debug, Default)]
pub struct Names {
    by_app: HashMap<String, Named>,
}

impl Names {
    pub fn forget(&mut self) {
        self.by_app.clear();
    }

    pub fn of(&mut self, entries: &Entries, app: &str) -> &Named {
        self.by_app.entry(app.to_owned()).or_insert_with(|| {
            let entry = (!app.is_empty()).then(|| entries.lookup(app)).flatten();
            let owner = (!app.is_empty()).then(|| entries.owner(app)).flatten();
            Named {
                entry: entry.map(|e| e.id.clone()),
                program: entry.map_or_else(|| app.to_owned(), |e| e.name.clone()),
                owner: owner.map(|e| e.id.clone()),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// niri's events as its stream says them on a desktop of two
    /// workspaces: a kitty for a session, two Chrome windows titled
    /// alike, and a foot.
    const BEGINNING: &str = r#"{"WorkspacesChanged":{"workspaces":[
        {"id":1,"idx":1,"name":null,"output":"eDP-1","is_urgent":false,"is_active":true,"is_focused":true,"active_window_id":10},
        {"id":2,"idx":2,"name":null,"output":"eDP-1","is_urgent":false,"is_active":false,"is_focused":false,"active_window_id":12}]}}
{"WindowsChanged":{"windows":[
        {"id":10,"title":"pi","app_id":"pi-session.3","pid":100,"workspace_id":1,"is_focused":true,"is_floating":false,"is_urgent":false,
         "layout":{"pos_in_scrolling_layout":[1,1],"tile_size":[640,800]},"focus_timestamp":{"secs":1000,"nanos":500000000}},
        {"id":11,"title":"Inbox - Google Chrome","app_id":"google-chrome","pid":101,"workspace_id":1,"is_focused":false,"is_floating":false,"is_urgent":false,
         "layout":{"pos_in_scrolling_layout":[2,1],"tile_size":[640,800]},"focus_timestamp":{"secs":900,"nanos":0}},
        {"id":12,"title":"Inbox - Google Chrome","app_id":"google-chrome","pid":101,"workspace_id":2,"is_focused":false,"is_floating":false,"is_urgent":false,
         "layout":{"pos_in_scrolling_layout":[1,2],"tile_size":[640,400]},"focus_timestamp":{"secs":800,"nanos":0}},
        {"id":13,"title":"~","app_id":"foot","pid":102,"workspace_id":2,"is_focused":false,"is_floating":true,"is_urgent":false,
         "layout":{"pos_in_scrolling_layout":null,"tile_size":[300,200]},"focus_timestamp":null}]}}"#;

    fn entries() -> Entries {
        Entries::from_texts([
            ("kitty", "[Desktop Entry]\nType=Application\nName=kitty\n"),
            (
                "google-chrome",
                "[Desktop Entry]\nType=Application\nName=Google Chrome\n",
            ),
        ])
    }

    /// The desktop once it's heard these lines, each an event.
    fn heard(lines: &str) -> Desktop {
        let mut desktop = Desktop::default();
        hear(&mut desktop, lines);
        desktop
    }

    fn hear(desktop: &mut Desktop, lines: &str) {
        // An event may run over lines here, for reading; each begins a
        // line with `{"` and a capital.
        let mut events = Vec::new();
        for line in lines.lines() {
            if line.starts_with("{\"") {
                events.push(String::new());
            }
            if let Some(event) = events.last_mut() {
                event.push_str(line);
            }
        }
        for event in events {
            desktop.hear(&serde_json::from_str(&event).expect("an event is JSON"));
        }
    }

    fn records(desktop: &Desktop) -> BTreeMap<String, Value> {
        desktop
            .records(&entries(), &mut Names::default())
            .into_iter()
            .map(|record| (record["key"].as_str().unwrap().to_owned(), record))
            .collect()
    }

    /// The keys whose records differ between two readings, and those gone.
    fn changed(before: &BTreeMap<String, Value>, after: &BTreeMap<String, Value>) -> Vec<String> {
        let mut keys: Vec<String> = after
            .iter()
            .filter(|(key, record)| before.get(*key) != Some(record))
            .map(|(key, _)| key.clone())
            .collect();
        keys.extend(
            before
                .keys()
                .filter(|key| !after.contains_key(*key))
                .cloned(),
        );
        keys
    }

    #[test]
    fn a_window_is_said_with_its_program_place_and_order() {
        let said = records(&heard(BEGINNING));
        let pi = &said["window:10"];
        assert_eq!(pi["app_id"], "kitty");
        assert_eq!(pi["opened"], "pi-session.3");
        assert_eq!(pi["session"], true);
        assert_eq!(pi["entry"], "kitty");
        assert_eq!(pi["order"], 1_000_500);
        assert_eq!(pi["focused"], true);
        assert_eq!(pi["elsewhere"], false);
        let inbox = &said["window:12"];
        assert_eq!(inbox["program"], "Google Chrome");
        assert_eq!(inbox["named"], "Inbox");
        assert_eq!(inbox["untitled"], false);
        assert_eq!(inbox["alike"], 2);
        assert_eq!(inbox["elsewhere"], true);
        assert_eq!(inbox["workspace_idx"], 2);
        assert_eq!(
            (inbox["column"].clone(), inbox["down"].clone()),
            (json!(1), json!(2))
        );
        let foot = &said["window:13"];
        assert_eq!(foot["entry"], Value::Null);
        assert_eq!(foot["program"], "foot");
        assert_eq!(foot["floating"], true);
        assert_eq!(foot["order"], 0);
        assert_eq!(foot["column"], Value::Null);
        assert_eq!(said["desktop"]["current"], 10);
        assert_eq!(
            said["desktop"]["window"],
            json!({"id": 10, "title": "pi", "app_id": "kitty", "entry": "kitty", "program": "kitty"})
        );
        assert_eq!(said["desktop"]["output"], "eDP-1");
    }

    #[test]
    fn a_retitle_changes_its_window_and_those_it_reads_alike_with() {
        let mut desktop = heard(BEGINNING);
        let before = records(&desktop);
        hear(
            &mut desktop,
            r#"{"WindowOpenedOrChanged":{"window":{"id":11,"title":"Calendar - Google Chrome","app_id":"google-chrome","pid":101,"workspace_id":1,"is_focused":false,"is_floating":false,"is_urgent":false,"layout":{"pos_in_scrolling_layout":[2,1],"tile_size":[640,800]},"focus_timestamp":{"secs":900,"nanos":0}}}}"#,
        );
        // The other Inbox no longer reads alike, so it says so too.
        assert_eq!(
            changed(&before, &records(&desktop)),
            ["window:11", "window:12"]
        );
        let before = records(&desktop);
        hear(
            &mut desktop,
            r#"{"WindowOpenedOrChanged":{"window":{"id":13,"title":"~/src","app_id":"foot","pid":102,"workspace_id":2,"is_focused":false,"is_floating":true,"is_urgent":false,"layout":{"pos_in_scrolling_layout":null,"tile_size":[300,200]},"focus_timestamp":null}}}"#,
        );
        assert_eq!(changed(&before, &records(&desktop)), ["window:13"]);
    }

    #[test]
    fn a_focus_moving_changes_only_the_two_windows_it_moved_between() {
        let mut desktop = heard(BEGINNING);
        let before = records(&desktop);
        hear(
            &mut desktop,
            concat!(
                r#"{"WindowFocusChanged":{"id":11}}"#,
                "\n",
                r#"{"WindowFocusTimestampChanged":{"id":11,"focus_timestamp":{"secs":1100,"nanos":0}}}"#,
                "\n",
                r#"{"WorkspaceActiveWindowChanged":{"workspace_id":1,"active_window_id":11}}"#
            ),
        );
        let after = records(&desktop);
        assert_eq!(
            changed(&before, &after),
            ["desktop", "window:10", "window:11", "workspace:1"]
        );
        assert_eq!(after["window:11"]["order"], 1_100_000);
        assert_eq!(after["window:10"]["order"], before["window:10"]["order"]);
    }

    #[test]
    fn the_launcher_taking_the_keys_moves_no_window() {
        let mut desktop = heard(BEGINNING);
        let before = records(&desktop);
        // niri says no window has focus while a layer has the keys; the
        // window the keys were in is still the one in front.
        hear(&mut desktop, r#"{"WindowFocusChanged":{"id":null}}"#);
        assert_eq!(changed(&before, &records(&desktop)), Vec::<String>::new());
    }

    #[test]
    fn a_layout_change_that_moves_no_column_changes_nothing() {
        let mut desktop = heard(BEGINNING);
        let before = records(&desktop);
        hear(
            &mut desktop,
            r#"{"WindowLayoutsChanged":{"changes":[[10,{"pos_in_scrolling_layout":[1,1],"tile_size":[900,800],"tile_pos_in_workspace_view":[-200,0]}]]}}"#,
        );
        assert_eq!(changed(&before, &records(&desktop)), Vec::<String>::new());
        hear(
            &mut desktop,
            r#"{"WindowLayoutsChanged":{"changes":[[11,{"pos_in_scrolling_layout":[3,1],"tile_size":[640,800]}]]}}"#,
        );
        assert_eq!(changed(&before, &records(&desktop)), ["window:11"]);
    }

    #[test]
    fn a_workspace_switch_changes_where_its_windows_say_they_are() {
        let mut desktop = heard(BEGINNING);
        let before = records(&desktop);
        hear(
            &mut desktop,
            r#"{"WorkspaceActivated":{"id":2,"focused":true}}"#,
        );
        let after = records(&desktop);
        assert_eq!(
            changed(&before, &after),
            [
                "desktop",
                "window:10",
                "window:11",
                "window:12",
                "window:13",
                "workspace:1",
                "workspace:2"
            ]
        );
        assert_eq!(after["window:10"]["elsewhere"], true);
        assert_eq!(after["window:12"]["elsewhere"], false);
        // The keys were in the pi window, which stays the one focused
        // until niri says another is.
        assert_eq!(after["desktop"]["workspace"], 2);
    }

    #[test]
    fn a_window_closed_is_gone_and_the_whole_list_replaces_what_was() {
        let mut desktop = heard(BEGINNING);
        let before = records(&desktop);
        hear(&mut desktop, r#"{"WindowClosed":{"id":13}}"#);
        // A window closed elsewhere changes no other window's record, only
        // what its workspace says is on it.
        assert_eq!(
            changed(&before, &records(&desktop)),
            ["workspace:2", "window:13"]
        );
        hear(
            &mut desktop,
            r#"{"WindowsChanged":{"windows":[{"id":10,"title":"pi","app_id":"pi-session.3","pid":100,"workspace_id":1,"is_focused":true,"is_floating":false,"is_urgent":false,"layout":{"pos_in_scrolling_layout":[1,1],"tile_size":[640,800]},"focus_timestamp":{"secs":1000,"nanos":500000000}}]}}"#,
        );
        let left: Vec<String> = records(&desktop)
            .into_keys()
            .filter(|key| key.starts_with("window:"))
            .collect();
        assert_eq!(left, ["window:10"]);
    }

    #[test]
    fn a_workspace_says_whats_on_it_and_each_screen_its_workspaces_in_order() {
        let mut desktop = heard(BEGINNING);
        // niri's empty last workspace on each screen, and a second
        // screen whose workspaces it lists out of order.
        hear(
            &mut desktop,
            r#"{"WorkspacesChanged":{"workspaces":[
        {"id":1,"idx":1,"name":null,"output":"eDP-1","is_urgent":false,"is_active":true,"is_focused":true,"active_window_id":10},
        {"id":2,"idx":2,"name":"mail","output":"eDP-1","is_urgent":false,"is_active":false,"is_focused":false,"active_window_id":12},
        {"id":5,"idx":2,"name":null,"output":"HDMI-A-1","is_urgent":false,"is_active":false,"is_focused":false,"active_window_id":null},
        {"id":3,"idx":3,"name":null,"output":"eDP-1","is_urgent":false,"is_active":false,"is_focused":false,"active_window_id":null},
        {"id":4,"idx":1,"name":null,"output":"HDMI-A-1","is_urgent":false,"is_active":true,"is_focused":false,"active_window_id":null}]}}"#,
        );
        let said = records(&desktop);
        assert_eq!(said["workspace:1"]["inside"], json!(["kitty", "Google Chrome"]));
        assert_eq!(said["workspace:2"]["inside"], json!(["Google Chrome", "foot"]));
        assert_eq!(said["workspace:2"]["name"], "mail");
        assert_eq!(said["workspace:3"]["inside"], json!([]));
        assert_eq!(
            [1, 2, 3, 4, 5].map(|id| said[&format!("workspace:{id}")]["new"].clone()),
            [false, false, true, false, true].map(Value::Bool)
        );
        assert_eq!(
            said["desktop"]["spaces"],
            json!({"HDMI-A-1": [4, 5], "eDP-1": [1, 2, 3]})
        );
        // A window opened on the empty last one makes it no longer new,
        // and changes no other workspace.
        let before = said;
        hear(
            &mut desktop,
            r#"{"WindowOpenedOrChanged":{"window":{"id":14,"title":"~","app_id":"foot","pid":103,"workspace_id":3,"is_focused":false,"is_floating":false,"is_urgent":false,"layout":{"pos_in_scrolling_layout":[1,1],"tile_size":[640,800]},"focus_timestamp":null}}}"#,
        );
        let after = records(&desktop);
        assert_eq!(changed(&before, &after), ["window:14", "workspace:3"]);
        assert_eq!(after["workspace:3"]["new"], false);
    }

    #[test]
    fn an_urgent_window_says_so() {
        let mut desktop = heard(BEGINNING);
        hear(
            &mut desktop,
            r#"{"WindowUrgencyChanged":{"id":12,"urgent":true}}"#,
        );
        assert_eq!(records(&desktop)["window:12"]["urgent"], true);
    }

    #[test]
    fn a_session_window_is_kittys_by_its_numbered_class_alone() {
        assert_eq!(app_of("pi-spawned"), ("kitty", true));
        assert_eq!(app_of("pi-session.12"), ("kitty", true));
        assert_eq!(app_of("pi-session."), ("pi-session.", false));
        assert_eq!(app_of("pi-sessions"), ("pi-sessions", false));
        assert_eq!(app_of("kitty"), ("kitty", false));
    }

    #[test]
    fn an_event_it_doesnt_follow_changes_nothing() {
        let mut desktop = heard(BEGINNING);
        assert_eq!(
            desktop.hear(&json!({"ScreenshotCaptured": {"path": "/tmp/a.png"}})),
            Heard::Nothing
        );
        assert_eq!(desktop.hear(&json!({"Ok": "Handled"})), Heard::Nothing);
    }
}
