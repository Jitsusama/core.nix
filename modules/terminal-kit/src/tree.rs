//! A command's tree of subcommands, walked from its words, so `eero devices
//! --help`, `eero devices -h` and `eero help devices` each answer the help
//! of `devices` alone, at any depth, and a word the tree doesn't hold is
//! refused with the nearest it does before the command starts
//! (docs/commands.md, Subcommands). Resources read as paths, as kubectl's
//! do: a level that takes a name takes the word after it as that name
//! before any subcommand, so a node called `devices` is still
//! `eero nodes devices`, and its devices are `eero nodes devices devices`.

use crate::failure::{Exit, Failure};
use crate::flags;
use crate::help::Help;

/// Where a command's words lead in its tree.
#[derive(Debug)]
pub struct Walked<'a> {
    /// Each help from the command's own down to the deepest the words
    /// name.
    pub chain: Vec<&'a Help>,
    /// The words, each alias put as the name it stands for, so the body
    /// reads one spelling of each.
    pub words: Vec<String>,
}

impl Walked<'_> {
    /// The deepest help, titled with its whole path and the names the way
    /// down needs, as `eero nodes <name> devices`, and taking its parent's
    /// exits where it names none of its own.
    pub fn shown(&self) -> Help {
        let deepest = self.chain.last().copied().cloned();
        let mut shown = deepest.unwrap_or_else(|| Help::new("", ""));
        shown.name = self.titled();
        if shown.exits.is_empty()
            && let Some(parent) = self.chain.iter().rev().find(|help| !help.exits.is_empty())
        {
            shown.exits.clone_from(&parent.exits);
        }
        shown
    }

    /// The path as a help's title says it: each level above the deepest
    /// with the arguments it takes before the next, since a subcommand
    /// under a name needs one. The deepest's own are its help's to list.
    fn titled(&self) -> String {
        let above = self.chain.len().saturating_sub(1);
        self.chain
            .iter()
            .enumerate()
            .flat_map(|(at, help)| {
                let needs = if at < above { &help.arguments[..] } else { &[] };
                std::iter::once(help.name.as_str())
                    .chain(needs.iter().map(|argument| argument.name.as_str()))
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The path the words name, as `eero nodes devices`: a command, as a
    /// try-line says it.
    pub fn path(&self) -> String {
        self.chain
            .iter()
            .map(|help| help.name.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Whether a flag the words hold takes the word after it as its value:
/// any help on the way declares it with a value, as `--since WHEN`.
fn takes_value(chain: &[&Help], word: &str) -> bool {
    chain.iter().flat_map(|help| &help.options).any(|option| {
        let variants: Vec<&str> = option.name.split(", ").collect();
        let valued = variants.last().is_some_and(|last| last.contains(' '));
        valued
            && variants
                .iter()
                .any(|variant| variant.split(' ').next() == Some(word))
    })
}

/// Whether a level takes another word as a name, having taken `filled`.
fn takes_name(help: &Help, filled: usize) -> bool {
    filled < help.arguments.len()
        || help
            .arguments
            .last()
            .is_some_and(|last| last.name.ends_with("..."))
}

/// Walks the words down the tree from `root`, which has subcommands of
/// its own. Flags are passed over, and everything from `--` on is the
/// command's.
///
/// # Errors
///
/// Asked wrongly, 64, for a word no subcommand at its level answers to,
/// naming the nearest that does, or for one too many after a level that
/// takes no more.
pub fn walk<'a>(root: &'a Help, args: &[String]) -> Result<Walked<'a>, Failure> {
    let mut walked = Walked {
        chain: vec![root],
        words: args.to_vec(),
    };
    let mut filled = 0;
    let mut value_next = false;
    // The words that led here, as a try-line types them again: each
    // subcommand by its name, and each name as given.
    let mut typed = vec![root.name.clone()];
    for (at, word) in args.iter().enumerate() {
        if value_next {
            value_next = false;
            continue;
        }
        if word == "--" {
            break;
        }
        if word.starts_with('-') && word.len() > 1 {
            value_next = !word.contains('=') && takes_value(&walked.chain, word);
            continue;
        }
        let here = walked.chain.last().copied().unwrap_or(root);
        if takes_name(here, filled) {
            filled += 1;
            typed.push(quoted(word));
            continue;
        }
        if let Some(sub) = here
            .subs
            .iter()
            .find(|sub| sub.name == *word || sub.aliases.contains(word))
        {
            walked.words[at].clone_from(&sub.name);
            typed.push(sub.name.clone());
            walked.chain.push(sub);
            filled = 0;
            continue;
        }
        // A level whose subcommands are only listed, not declared with
        // help of their own, reads the rest its own way.
        if here.subs.is_empty() && !here.commands.is_empty() {
            break;
        }
        let path = walked.path();
        let failure = if here.subs.is_empty() {
            Failure::new(Exit::Usage, format!("{path} takes no more words"))
                .because(format!("{word} is one too many"))
        } else {
            flags::unknown("command", word, &here.command_names())
        };
        typed.push("--help".to_owned());
        return Err(failure.try_this(typed.join(" ")));
    }
    Ok(walked)
}

/// A word as a shell reads it back whole: in double quotes where it holds
/// a space or is empty, as `"Main Floor"`.
fn quoted(word: &str) -> String {
    if word.is_empty() || word.contains(char::is_whitespace) {
        format!("\"{}\"", word.replace('"', "\\\""))
    } else {
        word.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eero() -> Help {
        Help::new("eero", "the home network")
            .subcommand(
                Help::new("nodes", "every node, or one")
                    .alias("node")
                    .argument("<name>", "a node's location")
                    .subcommand(Help::new("devices", "the devices on it"))
                    .subcommand(Help::new("radios", "its radios")),
            )
            .subcommand(
                Help::new("devices", "every device, or one")
                    .alias("device")
                    .argument("<name>", "a device's name")
                    .option("--since WHEN", "only those on since"),
            )
            .subcommand(
                Help::new("internet", "the way out").subcommand(
                    Help::new("sqm", "calls and games first")
                        .argument("on|off", "turn it on or off")
                        .exit(Exit::Refused, "not signed in"),
                ),
            )
            .subcommand(
                Help::new("sqm", "calls and games first")
                    .argument("on|off", "turn it on or off")
                    .unlisted(),
            )
            .exit(Exit::Unavailable, "the daemon isn't running")
    }

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    fn path(line: &str) -> String {
        let help = eero();
        walk(&help, &words(line)).map_or_else(
            |failure| format!("refused: {}", failure.what),
            |walked| walked.path(),
        )
    }

    #[test]
    fn the_words_lead_to_the_deepest_subcommand_they_name() {
        assert_eq!(path(""), "eero");
        assert_eq!(path("devices"), "eero devices");
        assert_eq!(path("devices Pixel"), "eero devices");
        assert_eq!(path("nodes Study devices"), "eero nodes devices");
        assert_eq!(path("internet sqm on"), "eero internet sqm");
    }

    #[test]
    fn the_word_after_a_collection_is_always_a_name() {
        assert_eq!(
            path("nodes devices"),
            "eero nodes",
            "a node called devices was read as the subcommand"
        );
        assert_eq!(path("nodes devices devices"), "eero nodes devices");
    }

    #[test]
    fn a_subcommands_help_is_titled_with_the_names_it_needs() {
        let help = eero();
        let walked = walk(&help, &words("nodes Study devices")).expect("a node's devices");
        assert_eq!(
            walked.shown().name,
            "eero nodes <name> devices",
            "the title left out the name a node's devices need"
        );
        assert_eq!(walked.path(), "eero nodes devices");
        let walked = walk(&help, &words("nodes Study")).expect("a node");
        assert_eq!(
            walked.shown().name,
            "eero nodes",
            "a level's own arguments are its help's, not its title's"
        );
    }

    #[test]
    fn an_alias_is_put_as_the_name_it_stands_for() {
        let help = eero();
        let walked = walk(&help, &words("device Pixel --json")).expect("device is devices");
        assert_eq!(walked.path(), "eero devices");
        assert_eq!(
            walked.words,
            words("devices Pixel --json"),
            "the alias wasn't put as its name"
        );
    }

    #[test]
    fn a_flags_value_is_never_taken_for_a_word() {
        assert_eq!(
            path("devices --since nodes Pixel"),
            "eero devices",
            "--since's value was walked"
        );
        assert_eq!(path("devices --since=nodes"), "eero devices");
    }

    #[test]
    fn a_word_no_subcommand_answers_to_names_the_nearest() {
        let help = eero();
        let failure = walk(&help, &words("devises")).expect_err("devises is no command");
        assert_eq!(failure.exit, Exit::Usage);
        assert_eq!(failure.why.as_deref(), Some("did you mean devices?"));
        assert_eq!(failure.then.as_deref(), Some("eero --help"));
        let failure = walk(&help, &words("internet sqn")).expect_err("sqn is no command");
        assert_eq!(failure.then.as_deref(), Some("eero internet --help"));
    }

    #[test]
    fn the_try_line_keeps_the_names_typed_on_the_way() {
        // Without the name, eero nodes devices --help would ask for a node
        // called devices.
        let help = eero();
        let typed: Vec<String> = ["node", "Main Floor", "devices", "extra"]
            .map(str::to_owned)
            .to_vec();
        let failure = walk(&help, &typed).expect_err("extra is one too many");
        assert_eq!(
            failure.then.as_deref(),
            Some("eero nodes \"Main Floor\" devices --help"),
            "the try-line lost the node's name, or didn't quote it"
        );
    }

    #[test]
    fn one_word_too_many_is_refused() {
        assert_eq!(
            path("devices Pixel Study"),
            "refused: eero devices takes no more words"
        );
    }

    #[test]
    fn an_unlisted_subcommand_answers_but_isnt_offered() {
        assert_eq!(path("sqm on"), "eero sqm");
        let names = eero().command_names().join(" ");
        assert_eq!(
            names, "nodes devices internet",
            "an unlisted one was offered"
        );
    }

    #[test]
    fn a_subcommand_shown_takes_its_parents_exits_where_it_names_none() {
        let help = eero();
        let shown = walk(&help, &words("devices")).expect("devices").shown();
        assert_eq!(shown.name, "eero devices");
        assert_eq!(shown.exits, help.exits, "devices didn't take eero's exits");
        let shown = walk(&help, &words("internet sqm")).expect("sqm").shown();
        assert_eq!(shown.exits, [(Exit::Refused, "not signed in".to_owned())]);
    }

    #[test]
    fn its_parent_lists_a_collection_once_with_its_singular() {
        let listed: Vec<String> = eero()
            .every_command()
            .into_iter()
            .map(|entry| entry.name)
            .collect();
        assert_eq!(
            listed,
            ["nodes (or node)", "devices (or device)", "internet"],
            "the commands weren't listed as their collections"
        );
    }
}
