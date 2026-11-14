//! What this daemon is to the shell's daemon interface (docs/daemons.md),
//! which daemon-serve speaks for it: its domain and kinds, and the asks
//! of its own beside hello, get, list, watch and health.

use std::collections::BTreeMap;

use daemon_serve::asks::Domain;
use serde_json::Value;

/// The kinds of record this daemon keeps, Tabs' head among them
/// (`tab_rows.rs`), and the tab in front of a window (front.rs).
pub const KINDS: [&str; 14] = [
    "program",
    "recent",
    "opened",
    "listed",
    "folder",
    "repositories",
    "files",
    "preview",
    "visited",
    "machines",
    "reached",
    "row",
    "head",
    "front",
];

/// The programs daemon as its clients know it.
pub const DOMAIN: Domain = Domain {
    name: "programs",
    kinds: &KINDS,
    sole: &[],
    // A window's tab in front is made only as a get names it, so an ask
    // naming no kind has none to say.
    named_only: &["front"],
};

/// The kind a key names: `program` for `program:kitty`.
pub fn kind_of(key: &str) -> Option<&str> {
    DOMAIN.kind_of(key)
}

/// One of this daemon's own asks.
#[derive(Clone, Debug, PartialEq)]
pub enum Own {
    /// The rows of a place whose names, or what else they're called, start
    /// with the words typed, best first (search.rs), at most `most` of
    /// them where the ask says.
    Search {
        place: String,
        words: String,
        most: Option<usize>,
    },
    /// What a program says of these keys' lists: they changed, so each
    /// is read again, at once where a watch names it; `from`, the process
    /// saying so, tells of its own changes from now on.
    Changed {
        keys: Vec<String>,
        from: Option<u64>,
    },
    /// What a kitty tab's row asks done to it, its OS window's niri id
    /// with Go to the Tab (`kitty_tabs.rs`).
    Tab(TabAsk),
    /// What the rows are made of, held until the next (docs/daemons.md,
    /// Settings).
    Settings(Settings),
}

/// What the rows are made of, as the shell says before it watches them;
/// a field an ask leaves out keeps what the one before said.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    /// The hidden programs Programs lists all the same, as those another
    /// provider gives acts (`program_rows.rs`).
    pub shown: Option<Vec<String>>,
    /// The folders Capture's rows are taken from (`taken_rows.rs`).
    pub folders: Option<Vec<String>>,
    /// What each program's key binds do, by its id, which its search
    /// finds it by too (search.rs).
    pub also: Option<BTreeMap<String, String>>,
}

/// One of kitty's tabs, by its kitty's process and its own id, and what's
/// asked done to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabAsk {
    pub act: TabAct,
    pub pid: u64,
    pub tab: u64,
    pub window: Option<u64>,
}

/// What a kitty tab's row asks: go to it, or close it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabAct {
    Go,
    Close,
}

/// The strings a field of an ask lists, where it lists any.
fn strings(said: &Value, field: &str) -> Option<Vec<String>> {
    let listed = said.get(field)?.as_array()?;
    Some(
        listed
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
    )
}

/// A field of an ask that's a text, or an empty one.
fn text_of(said: &Value, field: &str) -> String {
    said.get(field)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

/// Which of this daemon's own asks a line is, or none for one it
/// doesn't know, which daemon-serve refuses.
pub fn own_of(said: &Value) -> Option<Own> {
    let number = |field: &str| said.get(field).and_then(Value::as_u64);
    match said.get("ask").and_then(Value::as_str)? {
        "search" => Some(Own::Search {
            place: text_of(said, "place"),
            words: text_of(said, "words"),
            most: number("most").and_then(|most| usize::try_from(most).ok()),
        }),
        "changed" => Some(Own::Changed {
            keys: strings(said, "keys")
                .unwrap_or_default()
                .into_iter()
                .filter(|key| kind_of(key).is_some())
                .collect(),
            from: number("from"),
        }),
        "settings" => Some(Own::Settings(Settings {
            shown: strings(said, "shown"),
            folders: strings(said, "folders"),
            also: said.get("also").and_then(Value::as_object).map(|also| {
                also.iter()
                    .filter_map(|(id, said)| Some((id.clone(), said.as_str()?.to_owned())))
                    .collect()
            }),
        })),
        asked @ ("tab:go" | "tab:close") => {
            let act = if asked == "tab:go" {
                TabAct::Go
            } else {
                TabAct::Close
            };
            Some(Own::Tab(TabAsk {
                act,
                pid: number("pid")?,
                tab: number("tab")?,
                window: number("window"),
            }))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_kitty_tabs_go_and_close_name_its_kitty_tab_and_window() {
        assert_eq!(
            own_of(&json!({"ask": "tab:go", "pid": 40, "tab": 5, "window": 7, "token": "t"})),
            Some(Own::Tab(TabAsk {
                act: TabAct::Go,
                pid: 40,
                tab: 5,
                window: Some(7),
            }))
        );
        assert_eq!(
            own_of(&json!({"ask": "tab:close", "pid": 40, "tab": 5})),
            Some(Own::Tab(TabAsk {
                act: TabAct::Close,
                pid: 40,
                tab: 5,
                window: None,
            }))
        );
        assert_eq!(
            own_of(&json!({"ask": "tab:close", "tab": 5})),
            None,
            "a tab's ask naming no kitty was taken"
        );
    }

    #[test]
    fn settings_say_what_rows_are_made_of_and_leave_the_rest_be() {
        let asked = json!({
            "ask": "settings", "folders": ["/a", 1], "also": {"kitty": "Open", "x": 1},
        });
        assert_eq!(
            own_of(&asked),
            Some(Own::Settings(Settings {
                shown: None,
                folders: Some(vec!["/a".into()]),
                also: Some(BTreeMap::from([("kitty".into(), "Open".into())])),
            })),
            "only words are kept, and what isn't named is left out"
        );
    }

    #[test]
    fn a_search_says_its_words() {
        assert_eq!(
            own_of(&json!({"ask": "search", "place": "programs", "words": "chr"})),
            Some(Own::Search {
                place: "programs".into(),
                words: "chr".into(),
                most: None,
            })
        );
        assert_eq!(
            own_of(&json!({"ask": "search", "place": "taken", "words": "zz", "most": 50})),
            Some(Own::Search {
                place: "taken".into(),
                words: "zz".into(),
                most: Some(50),
            })
        );
    }

    #[test]
    fn a_program_says_which_of_its_lists_changed() {
        assert_eq!(
            own_of(&json!({"ask": "changed", "keys": ["listed:kitty:open", "disk:nvme0n1"]})),
            Some(Own::Changed {
                keys: vec!["listed:kitty:open".into()],
                from: None,
            }),
            "a key of another kind is left out, and no process said"
        );
        assert_eq!(
            own_of(&json!({"ask": "changed", "keys": ["listed:kitty:open"], "from": 2684})),
            Some(Own::Changed {
                keys: vec!["listed:kitty:open".into()],
                from: Some(2684),
            }),
            "the process telling is said by its id"
        );
    }

    #[test]
    fn an_ask_it_doesnt_know_is_left_to_be_refused() {
        assert_eq!(own_of(&json!({"ask": "dance"})), None);
    }
}
