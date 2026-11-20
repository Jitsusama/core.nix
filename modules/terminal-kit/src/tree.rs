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
    /// Whether each level above the deepest took names before the next,
    /// which an old word typed before them, as `location set`, didn't.
    named: Vec<bool>,
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
    /// under a name needs one, unless the next was reached before them.
    /// The deepest's own are its help's to list.
    fn titled(&self) -> String {
        self.chain
            .iter()
            .enumerate()
            .flat_map(|(at, help)| {
                let named = self.named.get(at).copied().unwrap_or(false);
                let needs = if named { &help.arguments[..] } else { &[] };
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
/// any help in the tree declares it with a value, as `--since WHEN`. The
/// whole tree, not just the way down so far, since a caller may type a
/// subcommand's flag before the subcommand.
fn takes_value(root: &Help, word: &str) -> bool {
    let mut helps = vec![root];
    let mut at = 0;
    while let Some(help) = helps.get(at) {
        helps.extend(&help.subs);
        at += 1;
    }
    helps.iter().flat_map(|help| &help.options).any(|option| {
        let variants: Vec<&str> = option.name.split(", ").collect();
        let valued = variants.last().is_some_and(|last| last.contains(' '));
        valued
            && variants
                .iter()
                .any(|variant| variant.split(' ').next() == Some(word))
    })
}

/// Whether any level of the tree, or the flags every command shares,
/// takes `word` as a flag.
fn knows_flag(root: &Help, word: &str) -> bool {
    let mut helps = vec![root];
    let mut at = 0;
    while let Some(help) = helps.get(at) {
        helps.extend(&help.subs);
        at += 1;
    }
    helps
        .iter()
        .flat_map(|help| help.every_option())
        .any(|entry| flag_names(&entry.name).any(|name| name == word))
}

/// The flags an option's name gives, as `-h` and `--help` from
/// `-h, --help`, or `--since` from `--since WHEN`.
fn flag_names(name: &str) -> impl Iterator<Item = &str> {
    name.split(", ")
        .filter_map(|variant| variant.split(' ').next())
}

/// Whether a level's only argument is any number of words that may be
/// left out, which its subcommands' names answer before. Taken as names
/// first, those words would leave every subcommand beside them unreached.
fn searches(help: &Help) -> bool {
    matches!(
        help.arguments.as_slice(),
        [only] if only.name.starts_with('[') && only.name.trim_end_matches(']').ends_with("...")
    ) && !help.subs.is_empty()
}

/// Whether a level takes another word as a name, having taken `filled`.
/// A last argument takes any number when its name ends in `...`, inside
/// its optional brackets or outside them (`[why...]` or `[why]...`).
fn takes_name(help: &Help, filled: usize) -> bool {
    filled < help.arguments.len()
        || help
            .arguments
            .last()
            .is_some_and(|last| last.name.trim_end_matches(']').ends_with("..."))
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
        named: Vec::new(),
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
            value_next = !word.contains('=') && takes_value(root, word);
            continue;
        }
        let here = walked.chain.last().copied().unwrap_or(root);
        let answers = |sub: &&Help| sub.name == *word || sub.aliases.contains(word);
        // An old word typed where its collection now takes a name, as
        // `findings expect KEY` before `findings KEY expect`, still
        // answers: an unlisted subcommand is the only kind matched
        // before the name, so a name never meets a listed word.
        // Words that may be left out beside subcommands, as health's
        // `[words]...`, are what's looked for only where no subcommand
        // answers, so a listed one is matched first there too.
        let first = if searches(here) {
            here.subs.iter().find(answers)
        } else {
            here.subs.iter().find(|sub| !sub.listed && answers(sub))
        };
        let old = (filled == 0).then_some(first).flatten();
        if old.is_none() && takes_name(here, filled) {
            // A level whose words are unsaid is typed again by what its
            // argument is called, never by what was given for it.
            let name = here.arguments.get(filled).or(here.arguments.last());
            typed.push(match name {
                Some(name) if !here.says_words => name.name.clone(),
                _ => crate::failure::shell_word(word).into_owned(),
            });
            filled += 1;
            continue;
        }
        if let Some(sub) = old.or_else(|| here.subs.iter().find(answers)) {
            walked.words[at].clone_from(&sub.name);
            typed.push(sub.name.clone());
            walked.named.push(filled > 0 || old.is_none());
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
        // A word left over after a flag no level knows is most likely that
        // flag's value, so the flag is what was mistyped.
        let before = at.checked_sub(1).and_then(|before| args.get(before));
        let unknown = before.filter(|flag| {
            flag.len() > 1
                && flag.starts_with('-')
                && !flag.contains('=')
                && !knows_flag(root, flag)
        });
        let failure = if let Some(flag) = unknown {
            let known = here.every_option();
            let names: Vec<&str> = known
                .iter()
                .flat_map(|entry| flag_names(&entry.name))
                .collect();
            if here.says_words {
                flags::unknown("option", flag, &names)
            } else {
                flags::unknown_unsaid("option", flag, &names)
            }
        } else if here.subs.is_empty() && !here.says_words {
            Failure::new(Exit::Usage, format!("{path} takes no more words")).because(
                "a word was left over, not said here, since it may be a reason or a secret's name",
            )
        } else if here.subs.is_empty() {
            Failure::new(Exit::Usage, format!("{path} takes no more words"))
                .because(format!("{word} is one too many"))
        } else if !here.says_words {
            flags::unknown_unsaid("command", word, &here.command_names())
        } else {
            flags::unknown("command", word, &here.command_names())
        };
        typed.push("--help".to_owned());
        return Err(failure.try_this(typed.join(" ")));
    }
    Ok(walked)
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
    fn a_subcommands_flag_typed_before_it_still_takes_its_value() {
        // A caller may put a subcommand's flag first, as it could before
        // the flag was its subcommand's alone.
        assert_eq!(
            path("--since nodes devices Pixel"),
            "eero devices",
            "--since's value was walked, typed before devices"
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
    fn a_level_whose_words_are_unsaid_never_says_one_back() {
        let help = Help::new("asks", "asks for secrets")
            .subcommand(
                Help::new("ask", "puts one up")
                    .option("--why WORDS", "the reason")
                    .words_unsaid(),
            )
            .subcommand(
                Help::new("show", "shows one")
                    .argument("<item>", "the item")
                    .words_unsaid(),
            )
            .words_unsaid();
        for line in [
            "ask --why To lighthouse",
            "show Hidden lighthouse",
            "ask --why To --lighthouse Hidden",
            "lighthouse",
        ] {
            let failure = walk(&help, &words(line)).expect_err("a word was left over");
            assert_eq!(
                failure.exit,
                Exit::Usage,
                "{line} wasn't refused as asked wrongly"
            );
            let said = format!("{failure:?}");
            assert!(
                !said.contains("lighthouse") && !said.contains("Hidden"),
                "{line}'s refusal said a word it was given: {said}"
            );
        }
        let failure = walk(&help, &words("show Hidden lighthouse")).expect_err("one too many");
        assert_eq!(
            failure.then.as_deref(),
            Some("asks show <item> --help"),
            "the try line didn't type the item by what it's called"
        );
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
            Some("eero nodes 'Main Floor' devices --help"),
            "the try-line lost the node's name, or didn't quote it"
        );
    }

    #[test]
    fn a_mistyped_flag_before_its_value_is_named_rather_than_the_value() {
        // Without it, the value reads as one word too many, and the flag
        // that was mistyped goes unsaid.
        let help = eero();
        let failure = walk(&help, &words("devices Pixel --sinse today"));
        let failure = failure.expect_err("--sinse is no option");
        assert_eq!(
            failure.what, "no option called --sinse",
            "the flag went unnamed"
        );
        assert_eq!(
            failure.why.as_deref(),
            Some("did you mean --since?"),
            "no nearest"
        );
        let failure = walk(&help, &words("devices Pixel --version today"));
        let failure = failure.expect_err("--version takes no value");
        assert_eq!(
            failure.what, "eero devices takes no more words",
            "a flag it knows was called unknown"
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
    fn an_argument_that_may_take_many_takes_them_however_its_bracketed() {
        // hibernate-unlock's rescue is told a word and a name, as
        // `rescue device virtio-pci`, under an argument written `[why...]`.
        for name in ["[why...]", "[why]...", "why..."] {
            let help = Help::new("unlock", "what it does").subcommand(
                Help::new("rescue", "tells of a rollback").argument(name, "why it rolled back"),
            );
            let walked = walk(&help, &words("rescue device virtio-pci"));
            assert_eq!(
                walked
                    .map(|walked| walked.path())
                    .map_err(|failure| failure.what),
                Ok("unlock rescue".to_owned()),
                "{name} took only one word"
            );
        }
    }

    #[test]
    fn words_that_may_be_left_out_beside_subcommands_are_asked_for_by_what_they_say() {
        // health's own views answer to their names; any other words are
        // looked for in the names of the things that day.
        let help = Help::new("health", "what the journals said")
            .argument("[words]...", "what to look for in the things' names")
            .subcommand(Help::new("boot", "each boot's stages"))
            .subcommand(Help::new("memory", "what each copy holds"));
        let path = |line: &str| {
            walk(&help, &words(line)).map_or_else(
                |failure| format!("refused: {}", failure.what),
                |walked| walked.path(),
            )
        };
        assert_eq!(path("boot"), "health boot", "a view was looked for");
        assert_eq!(
            walk(&help, &words("boot"))
                .map(|walked| walked.shown().name)
                .ok(),
            Some("health boot".to_owned()),
            "a view's help was titled as if it came after the words"
        );
        assert_eq!(path("wifid"), "health", "words were refused");
        assert_eq!(path("wifid calls"), "health", "a second word was refused");
        assert_eq!(
            path("boot again"),
            "refused: health boot takes no more words",
            "a view took words it doesn't read"
        );
        let one = Help::new("axes", "each axis")
            .argument("[axis]", "the one to step into")
            .subcommand(Help::new("speed", "how fast"));
        assert_eq!(
            walk(&one, &words("speed")).map(|walked| walked.path()).ok(),
            Some("axes".to_owned()),
            "a single name that may be left out was taken for a subcommand"
        );
    }

    #[test]
    fn an_old_word_before_a_collections_name_still_answers() {
        let help = Help::new("spend", "what it costs").subcommand(
            Help::new("findings", "each finding, or one")
                .argument("<key>", "a finding's key")
                .subcommand(Help::new("expect", "mark it expected"))
                .subcommand(
                    Help::new("expect", "mark one expected")
                        .argument("<key>", "a finding's key")
                        .unlisted(),
                ),
        );
        let path = |line: &str| {
            walk(&help, &words(line)).map_or_else(
                |failure| format!("refused: {}", failure.what),
                |walked| walked.path(),
            )
        };
        assert_eq!(
            path("findings burn:s1 expect"),
            "spend findings expect",
            "the action after its name"
        );
        assert_eq!(
            path("findings expect burn:s1"),
            "spend findings expect",
            "the old word before the name was read as a name"
        );
        assert_eq!(
            path("findings burn:s1"),
            "spend findings",
            "a name alone is still a name"
        );
    }

    /// A daemon started with its names first, as `location SOCKET ROOT`,
    /// keeps its commands as unlisted subcommands, so `location set`
    /// reaches set's help, titled as typed, with no name before it.
    #[test]
    fn an_old_word_before_a_name_is_titled_without_the_name() {
        let help = Help::new("location", "where the laptop is")
            .argument("SOCKET", "where it answers")
            .argument("ROOT", "whose zone it says")
            .subcommand(
                Help::new("set", "pins here")
                    .argument("PIN", "the pin's file")
                    .argument("WORDS...", "the town")
                    .unlisted(),
            );
        let walked = walk(&help, &words("set pin.json Halifax")).expect("set");
        assert_eq!(
            walked.shown().name,
            "location set",
            "set's title asked for the names it was typed before"
        );
        let walked = walk(&help, &words("location.sock /")).expect("the daemon");
        assert_eq!(
            walked.path(),
            "location",
            "the daemon's names read as a command"
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
