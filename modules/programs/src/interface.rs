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
    /// with the words typed, best first (search.rs).
    Search(Search),
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
    /// New Terminal Here on a repository's row (`code_rows.rs`): a kitty
    /// started in its checkout, by the folder's absolute path.
    Terminal(String),
    /// Pull on a repository's row (`pull.rs`): its checkout brought up to
    /// the branch it follows by fast-forward alone, by the checkout's
    /// absolute path.
    Pull(String),
    /// Forget Gone Trees on a repository's row (`tend.rs`): git's record
    /// of the trees cut from its checkout whose folders are gone, taken,
    /// by the checkout's absolute path.
    Forget(String),
    /// Gather Its Store on a repository's row (`tend.rs`): git's own
    /// upkeep of its checkout's store, by the checkout's absolute path.
    Gather(String),
    /// Edit Here on a repository's row (`code_rows.rs`): Neovim started
    /// on its checkout, by the folder's absolute path.
    Edit(String),
    /// Switch to It on a branch's row (`switch.rs`): its checkout, by its
    /// absolute path, put on the branch.
    Switch { folder: String, branch: String },
    /// What the rows are made of, held until the next (docs/daemons.md,
    /// Settings).
    Settings(Settings),
}

/// A search of `place` for the `words` typed: at most `most` rows where
/// it says, and only those inside the row keyed `within` where it names
/// one, as what's typed in a step finds among the rows it steps into.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Search {
    pub place: String,
    pub words: String,
    pub most: Option<usize>,
    pub within: Option<String>,
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
        "search" => Some(Own::Search(Search {
            place: text_of(said, "place"),
            words: text_of(said, "words"),
            most: number("most").and_then(|most| usize::try_from(most).ok()),
            within: said
                .get("within")
                .and_then(Value::as_str)
                .map(str::to_owned),
        })),
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
        "code:terminal" => {
            let folder = text_of(said, "path");
            folder.starts_with('/').then_some(Own::Terminal(folder))
        }
        "code:pull" => {
            let folder = text_of(said, "path");
            folder.starts_with('/').then_some(Own::Pull(folder))
        }
        "code:forget" => {
            let folder = text_of(said, "path");
            folder.starts_with('/').then_some(Own::Forget(folder))
        }
        "code:gather" => {
            let folder = text_of(said, "path");
            folder.starts_with('/').then_some(Own::Gather(folder))
        }
        "code:edit" => {
            let folder = text_of(said, "path");
            folder.starts_with('/').then_some(Own::Edit(folder))
        }
        "code:switch" => {
            let (folder, branch) = (text_of(said, "path"), text_of(said, "branch"));
            (folder.starts_with('/') && !branch.is_empty())
                .then_some(Own::Switch { folder, branch })
        }
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
    fn a_repositorys_pull_names_the_checkout_it_brings_forward() {
        assert_eq!(
            own_of(&json!({"ask": "code:pull", "path": "/home/j/src/core.nix"})),
            Some(Own::Pull("/home/j/src/core.nix".into()))
        );
        assert_eq!(
            own_of(&json!({"ask": "code:pull", "path": "src/core.nix"})),
            None,
            "a pull of a folder relative to nothing was taken"
        );
    }

    #[test]
    fn a_repositorys_tending_names_the_checkout_it_tends() {
        assert_eq!(
            own_of(&json!({"ask": "code:forget", "path": "/home/j/src/core.nix"})),
            Some(Own::Forget("/home/j/src/core.nix".into()))
        );
        assert_eq!(
            own_of(&json!({"ask": "code:gather", "path": "/home/j/src/core.nix"})),
            Some(Own::Gather("/home/j/src/core.nix".into()))
        );
        for ask in ["code:forget", "code:gather"] {
            assert_eq!(
                own_of(&json!({"ask": ask, "path": "src/core.nix"})),
                None,
                "{ask} of a folder relative to nothing was taken"
            );
        }
    }

    #[test]
    fn a_branchs_switch_names_its_checkout_and_the_branch() {
        assert_eq!(
            own_of(&json!({"ask": "code:switch", "path": "/home/j/src/core.nix", "branch": "fix"})),
            Some(Own::Switch {
                folder: "/home/j/src/core.nix".into(),
                branch: "fix".into(),
            })
        );
        assert_eq!(
            own_of(&json!({"ask": "code:switch", "path": "src/core.nix", "branch": "fix"})),
            None,
            "a switch of a folder relative to nothing was taken"
        );
        assert_eq!(
            own_of(&json!({"ask": "code:switch", "path": "/home/j/src/core.nix"})),
            None,
            "a switch naming no branch was taken"
        );
    }

    #[test]
    fn a_repositorys_terminal_and_editor_name_the_folder_they_open_in() {
        assert_eq!(
            own_of(&json!({"ask": "code:terminal", "path": "/home/j/src/core.nix"})),
            Some(Own::Terminal("/home/j/src/core.nix".into()))
        );
        assert_eq!(
            own_of(&json!({"ask": "code:terminal"})),
            None,
            "a terminal naming no folder was taken"
        );
        assert_eq!(
            own_of(&json!({"ask": "code:terminal", "path": "src/core.nix"})),
            None,
            "a terminal in a folder relative to nothing was taken"
        );
        assert_eq!(
            own_of(&json!({"ask": "code:edit", "path": "/home/j/src/core.nix"})),
            Some(Own::Edit("/home/j/src/core.nix".into()))
        );
        assert_eq!(
            own_of(&json!({"ask": "code:edit"})),
            None,
            "an editor naming no folder was taken"
        );
        assert_eq!(
            own_of(&json!({"ask": "code:edit", "path": "src/core.nix"})),
            None,
            "an editor in a folder relative to nothing was taken"
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
            Some(Own::Search(Search {
                place: "programs".into(),
                words: "chr".into(),
                most: None,
                within: None,
            }))
        );
        assert_eq!(
            own_of(&json!({"ask": "search", "place": "taken", "words": "zz", "most": 50})),
            Some(Own::Search(Search {
                place: "taken".into(),
                words: "zz".into(),
                most: Some(50),
                within: None,
            }))
        );
        assert_eq!(
            own_of(&json!({"ask": "search", "place": "code", "words": "gu",
                "within": "row:code:github:a/b:files"})),
            Some(Own::Search(Search {
                place: "code".into(),
                words: "gu".into(),
                most: None,
                within: Some("row:code:github:a/b:files".into()),
            })),
            "a search typed in a step didn't say which"
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
