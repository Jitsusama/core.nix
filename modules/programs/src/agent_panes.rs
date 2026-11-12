//! Which of kitty's panes and niri's windows are agents', as the agents
//! daemon names them in its `agent-window` records, one a session with a
//! window: its kitty pane, by the socket of the kitty it runs in and the
//! pane's window id, and its window, by the windows daemon's key. kitty's
//! page lists what open-now printed without those panes, and Programs
//! counts those windows apart from Joel's own, so the dozens of agents'
//! terminals don't bury the few he went to (VKCF3Z, 2026-11-11).

use std::collections::BTreeSet;

use serde_json::Value;

/// A kitty pane: the kitty's process and the pane's window id.
pub type Pane = (u64, u64);

/// The pane a record names, from its kitty's socket, `unix:@kitty-<pid>`,
/// and its window id, given as text or a number.
#[must_use]
pub fn pane(record: &Value) -> Option<Pane> {
    let kitty = &record["kitty"];
    let pid = kitty["within"]
        .as_str()?
        .strip_prefix("unix:@kitty-")?
        .parse()
        .ok()?;
    let id = match &kitty["id"] {
        Value::String(id) => id.parse().ok()?,
        id => id.as_u64()?,
    };
    Some((pid, id))
}

/// The windows daemon's key for a record's window, where it has one.
#[must_use]
pub fn window(record: &Value) -> Option<&str> {
    record["window"].as_str()
}

/// What open-now printed of kitty, a line a kitty, `<pid>`, a tab and its
/// OS windows as kitty lists them, without the panes named, and how many
/// it left out. A tab left with no pane goes, and an OS window left with
/// no tab; a line that isn't kitty's list is kept as it was.
#[must_use]
pub fn without(printed: &str, panes: &BTreeSet<Pane>) -> (String, usize) {
    let mut lines = Vec::new();
    let mut taken = 0;
    for line in printed.lines().filter(|line| !line.is_empty()) {
        let parsed = line.split_once('\t').and_then(|(pid, list)| {
            Some((
                pid.parse::<u64>().ok()?,
                serde_json::from_str::<Value>(list).ok()?,
            ))
        });
        let Some((pid, mut windows)) = parsed else {
            lines.push(line.to_owned());
            continue;
        };
        let left = left_out(pid, &mut windows, panes);
        taken += left;
        lines.push(if left == 0 {
            line.to_owned()
        } else {
            format!("{pid}\t{windows}")
        });
    }
    (lines.join("\n"), taken)
}

/// Takes the panes named out of one kitty's OS windows, saying how many.
fn left_out(pid: u64, windows: &mut Value, panes: &BTreeSet<Pane>) -> usize {
    let named = |pane: &Value| {
        pane["id"]
            .as_u64()
            .is_some_and(|id| panes.contains(&(pid, id)))
    };
    let mut taken = 0;
    let Some(windows) = windows.as_array_mut() else {
        return 0;
    };
    for window in windows.iter_mut() {
        let Some(tabs) = window["tabs"].as_array_mut() else {
            continue;
        };
        for tab in tabs.iter_mut() {
            if let Some(held) = tab["windows"].as_array_mut() {
                let before = held.len();
                held.retain(|pane| !named(pane));
                taken += before - held.len();
            }
        }
        tabs.retain(|tab| {
            tab["windows"]
                .as_array()
                .is_none_or(|held| !held.is_empty())
        });
    }
    windows.retain(|window| {
        window["tabs"]
            .as_array()
            .is_none_or(|tabs| !tabs.is_empty())
    });
    taken
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn record(within: &str, id: &Value, window: Option<&str>) -> Value {
        json!({
            "key": "agent-window:agents:ABC", "kind": "agent-window", "address": "ABC",
            "kitty": {"id": id, "within": within}, "window": window,
        })
    }

    #[test]
    fn a_records_pane_is_its_kittys_process_and_its_window_id() {
        assert_eq!(
            pane(&record("unix:@kitty-4242", &json!("7"), None)),
            Some((4242, 7)),
            "an id given as text"
        );
        assert_eq!(
            pane(&record("unix:@kitty-4242", &json!(7), Some("window:9"))),
            Some((4242, 7)),
            "an id given as a number"
        );
        assert_eq!(
            pane(&record("unix:/tmp/kitty", &json!("7"), None)),
            None,
            "a socket that isn't a kitty's by its process names no pane"
        );
        assert_eq!(
            window(&record("unix:@kitty-4242", &json!("7"), Some("window:9"))),
            Some("window:9"),
            "its window by the windows daemon's key"
        );
    }

    #[test]
    fn the_agents_panes_leave_kittys_list_with_any_tab_and_window_they_empty() {
        let mine = json!([{"id": 1, "tabs": [
            {"id": 1, "windows": [{"id": 1}, {"id": 3}]},
            {"id": 2, "windows": [{"id": 2}]},
        ]}, {"id": 2, "tabs": [{"id": 3, "windows": [{"id": 4}]}]}]);
        let theirs = json!([{"id": 1, "tabs": [{"id": 1, "windows": [{"id": 2}]}]}]);
        let printed = format!("4242\t{mine}\n5151\t{theirs}\n");
        let panes = BTreeSet::from([(4242, 2), (4242, 3), (4242, 4), (6000, 1)]);
        let (left, taken) = without(&printed, &panes);
        assert_eq!(taken, 3, "the three panes of 4242's named");
        let lines: Vec<&str> = left.lines().collect();
        assert_eq!(
            serde_json::from_str::<Value>(lines[0].split_once('\t').unwrap().1).unwrap(),
            json!([{"id": 1, "tabs": [{"id": 1, "windows": [{"id": 1}]}]}]),
            "a pane named goes, and the tab and window it emptied go too"
        );
        assert_eq!(
            lines[1],
            format!("5151\t{theirs}"),
            "another kitty's pane of the same id stays, its line as printed"
        );
    }

    #[test]
    fn a_line_that_isnt_kittys_list_is_kept_as_it_was() {
        let (left, taken) = without("not a list\n4242\tnot json", &BTreeSet::from([(4242, 1)]));
        assert_eq!(
            (left.as_str(), taken),
            ("not a list\n4242\tnot json", 0),
            "lines it can't read are kept, none taken"
        );
    }
}
