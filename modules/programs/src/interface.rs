//! What a client asks, in version 1 of the shell's daemon interface
//! (RSCH-20261029-0E8050, "Daemons: One Interface"): JSON lines, each ask
//! with an `id` echoed in its answers.

use std::collections::BTreeMap;

use serde_json::{Value, json};

/// The interface's version this speaks.
pub const VERSION: u64 = 1;

/// The kinds of record this daemon keeps.
pub const KINDS: [&str; 12] = [
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
];

/// The kind a key names: `program` for `program:kitty`.
pub fn kind_of(key: &str) -> Option<&str> {
    let (kind, rest) = key.split_once(':')?;
    (KINDS.contains(&kind) && !rest.is_empty()).then_some(kind)
}

/// What a client asked, one line of it.
#[derive(Clone, Debug, PartialEq)]
pub enum Ask {
    Hello {
        id: Value,
    },
    /// Records by key, as they stand now.
    Get {
        id: Value,
        keys: Vec<String>,
    },
    /// Every record of these kinds.
    List {
        id: Value,
        kinds: Vec<String>,
    },
    /// Records of these kinds as they change, all of them or these keys.
    /// `shown` names the hidden programs the watcher lists all the same,
    /// as Programs' rows do those another provider gives acts; `place`
    /// the one place whose rows it hears (ready-rows.md); `folders`
    /// those Capture's rows are taken from; and `also`, what each
    /// program's key binds do, by its id, which its search finds it by too.
    Watch {
        id: Value,
        kinds: Vec<String>,
        keys: Option<Vec<String>>,
        shown: Vec<String>,
        place: Option<String>,
        folders: Vec<String>,
        also: BTreeMap<String, String>,
    },
    /// The rows of a place whose names, or what else they're called, start
    /// with the words typed, best first (search.rs), at most `most` of
    /// them where the ask says.
    Search {
        id: Value,
        place: String,
        words: String,
        most: Option<usize>,
    },
    Unwatch {
        id: Value,
    },
    /// What a program says of these keys' lists: they changed, so each
    /// is read again, at once where a watch names it; `from`, the process
    /// saying so, tells of its own changes from now on.
    Changed {
        id: Value,
        keys: Vec<String>,
        from: Option<u64>,
    },
    Health {
        id: Value,
    },
    /// What a kitty tab's row asks done to it, its OS window's niri id
    /// with Go to the Tab (`kitty_tabs.rs`).
    Tab {
        id: Value,
        tab: TabAsk,
    },
    /// A kind this daemon doesn't keep.
    UnknownKind {
        id: Value,
        kind: String,
    },
    /// An ask this daemon doesn't know, or a line that isn't one.
    Unknown {
        id: Value,
        ask: String,
    },
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

/// The kinds an ask names, as one or a list, or every kind when it names
/// none; or the first it doesn't keep.
fn kinds(said: &Value) -> Result<Vec<String>, String> {
    let named: Vec<String> = match said.get("kind") {
        None | Some(Value::Null) => return Ok(KINDS.iter().map(|k| (*k).to_owned()).collect()),
        Some(Value::String(kind)) => vec![kind.clone()],
        Some(Value::Array(kinds)) => kinds
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        Some(other) => return Err(other.to_string()),
    };
    match named.iter().find(|kind| !KINDS.contains(&kind.as_str())) {
        Some(unknown) => Err(unknown.clone()),
        None => Ok(named),
    }
}

/// The strings a field of an ask lists, or none.
fn strings(said: &Value, field: &str) -> Vec<String> {
    said.get(field)
        .and_then(Value::as_array)
        .map(|listed| {
            listed
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// What a line from a client asks.
pub fn ask(line: &str) -> Ask {
    let Ok(said) = serde_json::from_str::<Value>(line) else {
        return Ask::Unknown {
            id: Value::Null,
            ask: String::new(),
        };
    };
    let id = said.get("id").cloned().unwrap_or(Value::Null);
    let asked = said
        .get("ask")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let keys = said.get("keys").and_then(Value::as_array).map(|keys| {
        keys.iter()
            .filter_map(Value::as_str)
            .filter(|key| kind_of(key).is_some())
            .map(str::to_owned)
            .collect::<Vec<_>>()
    });
    let named = || kinds(&said);
    match asked.as_str() {
        "hello" => Ask::Hello { id },
        "get" => Ask::Get {
            id,
            keys: keys.unwrap_or_default(),
        },
        "list" => match named() {
            Ok(kinds) => Ask::List { id, kinds },
            Err(kind) => Ask::UnknownKind { id, kind },
        },
        "watch" => match named() {
            Ok(kinds) => Ask::Watch {
                id,
                kinds,
                keys,
                shown: strings(&said, "shown"),
                place: said.get("place").and_then(Value::as_str).map(str::to_owned),
                folders: strings(&said, "folders"),
                also: said
                    .get("also")
                    .and_then(Value::as_object)
                    .map(|also| {
                        also.iter()
                            .filter_map(|(id, said)| Some((id.clone(), said.as_str()?.to_owned())))
                            .collect()
                    })
                    .unwrap_or_default(),
            },
            Err(kind) => Ask::UnknownKind { id, kind },
        },
        "search" => Ask::Search {
            id,
            place: text_of(&said, "place"),
            words: text_of(&said, "words"),
            most: said
                .get("most")
                .and_then(Value::as_u64)
                .and_then(|most| usize::try_from(most).ok()),
        },
        "unwatch" => Ask::Unwatch { id },
        "changed" => Ask::Changed {
            id,
            keys: keys.unwrap_or_default(),
            from: said.get("from").and_then(Value::as_u64),
        },
        "health" => Ask::Health { id },
        "tab:go" | "tab:close" => {
            let number = |field: &str| said.get(field).and_then(Value::as_u64);
            let act = if asked == "tab:go" {
                TabAct::Go
            } else {
                TabAct::Close
            };
            match (number("pid"), number("tab")) {
                (Some(pid), Some(tab)) => Ask::Tab {
                    id,
                    tab: TabAsk {
                        act,
                        pid,
                        tab,
                        window: number("window"),
                    },
                },
                _ => Ask::Unknown { id, ask: asked },
            }
        }
        _ => Ask::Unknown { id, ask: asked },
    }
}

/// A field of an ask that's a text, or an empty one.
fn text_of(said: &Value, field: &str) -> String {
    said.get(field)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

/// An error answer, by the interface's codes.
pub fn error(id: &Value, code: &str, message: &str) -> Value {
    json!({"id": id, "error": {"code": code, "message": message}})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kitty_tabs_go_and_close_name_its_kitty_tab_and_window() {
        assert_eq!(
            ask(r#"{"id":3,"ask":"tab:go","pid":40,"tab":5,"window":7,"token":"t"}"#),
            Ask::Tab {
                id: json!(3),
                tab: TabAsk {
                    act: TabAct::Go,
                    pid: 40,
                    tab: 5,
                    window: Some(7),
                },
            }
        );
        assert_eq!(
            ask(r#"{"id":4,"ask":"tab:close","pid":40,"tab":5}"#),
            Ask::Tab {
                id: json!(4),
                tab: TabAsk {
                    act: TabAct::Close,
                    pid: 40,
                    tab: 5,
                    window: None,
                },
            }
        );
        assert_eq!(
            ask(r#"{"id":5,"ask":"tab:close","tab":5}"#),
            Ask::Unknown {
                id: json!(5),
                ask: "tab:close".to_owned(),
            },
            "a tab's ask naming no kitty was taken"
        );
    }

    #[test]
    fn a_watch_names_keys_of_its_kinds() {
        assert_eq!(
            ask(r#"{"id":1,"ask":"watch","keys":["program:kitty","recent:shared"]}"#),
            Ask::Watch {
                id: json!(1),
                kinds: KINDS.iter().map(|k| (*k).to_owned()).collect(),
                keys: Some(vec!["program:kitty".into(), "recent:shared".into()]),
                shown: Vec::new(),
                place: None,
                folders: Vec::new(),
                also: BTreeMap::new(),
            }
        );
    }

    #[test]
    fn a_watch_of_rows_names_its_place_and_what_it_takes_them_from() {
        assert_eq!(
            ask(r#"{"id":5,"ask":"watch","kind":"row","place":"taken","folders":["/a",1]}"#),
            Ask::Watch {
                id: json!(5),
                kinds: vec!["row".into()],
                keys: None,
                shown: Vec::new(),
                place: Some("taken".into()),
                folders: vec!["/a".into()],
                also: BTreeMap::new(),
            }
        );
    }

    #[test]
    fn a_watch_says_what_programs_binds_do_and_a_search_its_words() {
        let watched = ask(r#"{"id":6,"ask":"watch","kind":"row","also":{"kitty":"Open","x":1}}"#);
        let Ask::Watch { also, .. } = watched else {
            panic!("a watch is read as one: {watched:?}");
        };
        assert_eq!(also.len(), 1, "only words are kept: {also:?}");
        assert_eq!(also["kitty"], "Open");
        assert_eq!(
            ask(r#"{"id":7,"ask":"search","place":"programs","words":"chr"}"#),
            Ask::Search {
                id: json!(7),
                place: "programs".into(),
                words: "chr".into(),
                most: None,
            }
        );
        assert_eq!(
            ask(r#"{"id":8,"ask":"search","place":"taken","words":"zz","most":50}"#),
            Ask::Search {
                id: json!(8),
                place: "taken".into(),
                words: "zz".into(),
                most: Some(50),
            }
        );
    }

    #[test]
    fn a_kind_it_doesnt_keep_is_refused() {
        assert_eq!(
            ask(r#"{"id":2,"ask":"list","kind":"disk"}"#),
            Ask::UnknownKind {
                id: json!(2),
                kind: "disk".into()
            }
        );
    }

    #[test]
    fn a_program_says_which_of_its_lists_changed() {
        assert_eq!(
            ask(r#"{"id":4,"ask":"changed","keys":["listed:kitty:open","disk:nvme0n1"]}"#),
            Ask::Changed {
                id: json!(4),
                keys: vec!["listed:kitty:open".into()],
                from: None,
            },
            "a key of another kind is left out, and no process said"
        );
        assert_eq!(
            ask(r#"{"id":5,"ask":"changed","keys":["listed:kitty:open"],"from":2684}"#),
            Ask::Changed {
                id: json!(5),
                keys: vec!["listed:kitty:open".into()],
                from: Some(2684),
            },
            "the process telling is said by its id"
        );
    }

    #[test]
    fn a_key_of_another_kind_is_left_out() {
        assert_eq!(
            ask(r#"{"id":3,"ask":"get","keys":["opened:mpv","disk:nvme0n1","program:"]}"#),
            Ask::Get {
                id: json!(3),
                keys: vec!["opened:mpv".into()]
            }
        );
    }
}
