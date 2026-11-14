//! What zsh's completion offers after the words typed so far, walked down
//! eero's tree (commands.rs), so completion follows the tree as its help
//! does: a collection's names after it, its subcommands after a name,
//! an argument's own words (`on`, `off`) where it has them, and each
//! listed subcommand elsewhere. An old spelling still leads where it
//! always did, since an alias is followed as its name.

use terminal_kit::Help;

/// What comes next.
#[derive(Debug, PartialEq, Eq)]
pub enum Next {
    /// The names the record holds of a kind, as `eero names nodes` says
    /// them.
    Names(&'static str),
    /// Words and what each is.
    Words(Vec<(String, String)>),
}

/// The kind of name a collection takes, by its name.
fn kind(help: &Help) -> Option<&'static str> {
    match help.name.as_str() {
        "nodes" => Some("nodes"),
        "devices" => Some("devices"),
        "profiles" => Some("profiles"),
        _ => None,
    }
}

/// Whether a level takes another word as a name, having taken `filled`:
/// one argument a word, past which its subcommands come.
fn takes_name(help: &Help, filled: usize) -> bool {
    filled < help.arguments.len()
}

/// The words an argument is one of, as `on|off`; none for one that
/// stands for what's typed, as `<email or phone>`.
fn choices(help: &Help, at: usize) -> Vec<(String, String)> {
    let Some(argument) = help.arguments.get(at) else {
        return Vec::new();
    };
    if argument.name.starts_with(['<', '[']) {
        return Vec::new();
    }
    argument
        .name
        .split('|')
        .map(|word| (word.to_owned(), argument.words.clone()))
        .collect()
}

/// What may follow the words typed, flags left out, from `root` down.
pub fn next(root: &Help, typed: &[&str]) -> Next {
    let mut here = root;
    let mut filled = 0;
    for word in typed.iter().filter(|word| !word.starts_with('-')) {
        if takes_name(here, filled) {
            filled += 1;
            continue;
        }
        let found = here
            .subs
            .iter()
            .find(|sub| sub.name == *word || sub.aliases.iter().any(|alias| alias == word));
        match found {
            // The old `eero watch <view>` takes a view's words, so they
            // complete as from eero itself.
            Some(sub) if sub.name == "watch" => {
                here = root;
                filled = 0;
            }
            Some(sub) => {
                here = sub;
                filled = 0;
            }
            None => return Next::Words(Vec::new()),
        }
    }
    if takes_name(here, filled) {
        return match kind(here) {
            Some(kind) if filled == 0 => Next::Names(kind),
            _ => Next::Words(choices(here, filled)),
        };
    }
    Next::Words(
        here.subs
            .iter()
            .filter(|sub| sub.listed)
            .map(|sub| (sub.name.clone(), sub.about.clone()))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands;

    fn words(line: &str) -> Vec<String> {
        let typed: Vec<&str> = line.split_whitespace().collect();
        match next(&commands::help(), &typed) {
            Next::Names(kind) => vec![format!("names of {kind}")],
            Next::Words(words) => words.into_iter().map(|(word, _)| word).collect(),
        }
    }

    #[test]
    fn bare_it_offers_each_listed_command_and_no_old_spelling() {
        let offered = words("");
        for listed in [
            "status", "nodes", "devices", "profiles", "radios", "internet",
        ] {
            assert!(
                offered.iter().any(|word| word == listed),
                "{listed} isn't offered: {offered:?}"
            );
        }
        for old in ["node", "device", "sqm", "speedtest", "watch"] {
            assert!(
                !offered.iter().any(|word| word == old),
                "the old {old} is offered: {offered:?}"
            );
        }
    }

    #[test]
    fn after_a_collection_its_names_are_offered_and_after_a_name_what_it_holds() {
        assert_eq!(words("nodes"), ["names of nodes"]);
        assert_eq!(
            words("node"),
            ["names of nodes"],
            "an alias leads as its name"
        );
        assert_eq!(words("devices"), ["names of devices"]);
        assert_eq!(words("profiles"), ["names of profiles"]);
        assert_eq!(words("nodes Study"), ["devices", "radios"]);
        assert_eq!(words("profiles Kids --json"), ["devices"]);
    }

    #[test]
    fn an_act_offers_its_own_words() {
        assert_eq!(words("internet"), ["sqm"]);
        assert_eq!(words("internet sqm"), ["on", "off"]);
        assert_eq!(words("sqm"), ["on", "off"], "the old sqm still completes");
        assert_eq!(words("speeds"), ["run"]);
    }

    #[test]
    fn the_old_watch_completes_a_view_as_eero_does() {
        assert_eq!(words("watch"), words(""));
        assert_eq!(words("watch node"), ["names of nodes"]);
    }

    #[test]
    fn nothing_is_offered_past_the_end_or_after_a_word_the_tree_doesnt_hold() {
        for line in [
            "devises",
            "internet sqm on",
            "nodes Study devices",
            "sign-in",
        ] {
            assert!(
                words(line).is_empty(),
                "eero {line} offered {:?}",
                words(line)
            );
        }
    }
}
