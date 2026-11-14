//! A command's help, declared once and set two ways: as text in the
//! levels, examples first, its groups under labels and every name in one
//! column; and as JSON, `help/1`, the same declaration for the
//! `commands-agree` check to read (docs/commands.md, Help). The shared
//! flags are added to every command's options, so none forgets one. A
//! subcommand declared with `subcommand` has a help of its own, which the kit
//! finds and answers (tree.rs).

use serde_json::{Value, json};

use crate::failure::Exit;
use crate::ink::Pen;
use crate::layout::{self, GAP, ITEMS, Room};

/// A name and what it's for: an example's command, a subcommand, a flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub words: String,
}

/// A command's help, as it declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Help {
    pub name: String,
    pub about: String,
    pub examples: Vec<Entry>,
    pub commands: Vec<Entry>,
    pub options: Vec<Entry>,
    /// The forms of its own `--format` takes beside the three.
    pub forms: Vec<String>,
    /// The codes it exits with past 0 and 64, in its own words.
    pub exits: Vec<(Exit, String)>,
    /// The words it takes before any subcommand of its own, as `<name>`,
    /// each optional; one ending `...` takes every word left.
    pub arguments: Vec<Entry>,
    /// Its subcommands that have a help of their own, in the order its
    /// help lists them.
    pub subs: Vec<Help>,
    /// The other names it answers to as a subcommand, an old spelling
    /// or the singular of a collection: the first is listed beside its
    /// name, the rest only answered.
    pub aliases: Vec<String>,
    /// Whether its parent's help lists it; one that isn't is kept only so
    /// an old caller's words still work.
    pub listed: bool,
}

/// The widest a name's column grows before a long name's words go on the
/// line under it, so one long flag doesn't push every other's words right.
const NAMES: usize = 30;

/// The inset of a group's entries under its label.
const INSET: usize = 2;

/// The fewest cells an entry's words are set in beside its name; fewer,
/// and they go under it.
const FEWEST_WORDS: usize = 24;

fn entry(name: &str, words: &str) -> Entry {
    Entry {
        name: name.to_owned(),
        words: words.to_owned(),
    }
}

impl Help {
    /// The help of a command by its name, and what it's for in its own
    /// words.
    pub fn new(name: &str, about: &str) -> Self {
        Self {
            name: name.to_owned(),
            about: about.to_owned(),
            examples: Vec::new(),
            commands: Vec::new(),
            options: Vec::new(),
            forms: Vec::new(),
            exits: Vec::new(),
            arguments: Vec::new(),
            subs: Vec::new(),
            aliases: Vec::new(),
            listed: true,
        }
    }

    /// The same, taking a word before its subcommands, as `<name>`, and
    /// what it is: the word after a collection is always a name, so a
    /// name never reads as a subcommand.
    #[must_use]
    pub fn argument(mut self, name: &str, words: &str) -> Self {
        self.arguments.push(entry(name, words));
        self
    }

    /// The same, with a subcommand that has a help of its own, named by
    /// its `Help`'s name.
    #[must_use]
    pub fn subcommand(mut self, help: Help) -> Self {
        self.subs.push(help);
        self
    }

    /// The same, answering to another name as a subcommand too.
    #[must_use]
    pub fn alias(mut self, name: &str) -> Self {
        self.aliases.push(name.to_owned());
        self
    }

    /// The same, left out of its parent's help: it still answers, so an
    /// old caller's words keep working.
    #[must_use]
    pub fn unlisted(mut self) -> Self {
        self.listed = false;
        self
    }

    /// Its subcommands as its help lists them: those declared with
    /// `command`, then each listed `subcommand`, its first alias beside
    /// it.
    pub fn every_command(&self) -> Vec<Entry> {
        let subs = self.subs.iter().filter(|sub| sub.listed).map(|sub| {
            let name = match sub.aliases.first() {
                Some(alias) => format!("{} (or {alias})", sub.name),
                None => sub.name.clone(),
            };
            entry(&name, &sub.about)
        });
        self.commands.iter().cloned().chain(subs).collect()
    }

    /// The same, with an example to copy and what it shows.
    #[must_use]
    pub fn example(mut self, command: &str, words: &str) -> Self {
        self.examples.push(entry(command, words));
        self
    }

    /// The same, with a subcommand.
    #[must_use]
    pub fn command(mut self, name: &str, words: &str) -> Self {
        self.commands.push(entry(name, words));
        self
    }

    /// The same, with a flag of its own.
    #[must_use]
    pub fn option(mut self, name: &str, words: &str) -> Self {
        self.options.push(entry(name, words));
        self
    }

    /// The same, with a form of its own `--format` takes.
    #[must_use]
    pub fn form(mut self, name: &str) -> Self {
        self.forms.push(name.to_owned());
        self
    }

    /// The same, with a code it exits with and what that means for it.
    #[must_use]
    pub fn exit(mut self, exit: Exit, words: &str) -> Self {
        self.exits.push((exit, words.to_owned()));
        self
    }

    /// Its subcommands' names, to suggest the nearest of: an entry's
    /// first word, and each listed sub's name.
    pub fn command_names(&self) -> Vec<&str> {
        self.commands
            .iter()
            .filter_map(|entry| entry.name.split_whitespace().next())
            .chain(
                self.subs
                    .iter()
                    .filter(|sub| sub.listed)
                    .map(|sub| sub.name.as_str()),
            )
            .collect()
    }

    /// Its options and the shared flags every command takes.
    pub fn every_option(&self) -> Vec<Entry> {
        let forms: Vec<&str> = ["text", "json", "jsonl"]
            .into_iter()
            .chain(self.forms.iter().map(String::as_str))
            .collect();
        let mut options = self.options.clone();
        options.extend([
            entry(
                &format!("--format {}", forms.join("|")),
                "the answer's form; --json is json",
            ),
            entry(
                "--color never|always|auto",
                "whether to ink; NO_COLOR and FORCE_COLOR too",
            ),
            entry("-h, --help", "this"),
            entry("--version", "its version"),
        ]);
        options
    }

    /// Every code it exits with: 0 and 64, which every command has, and
    /// its own.
    pub fn every_exit(&self) -> Vec<(Exit, String)> {
        let mut exits = self.exits.clone();
        for exit in [Exit::Done, Exit::Usage] {
            if !exits.iter().any(|(had, _)| *had == exit) {
                exits.push((exit, exit.means().to_owned()));
            }
        }
        exits.sort_by_key(|(exit, _)| exit.code());
        exits
    }

    /// Its first line: the name as the title, what it's for in muted.
    fn head(&self, pen: &Pen, measure: usize) -> String {
        let lead = layout::width(&self.name) + GAP;
        let about = layout::hung(&self.about, lead, Room::Cells(measure));
        let about: Vec<String> = about
            .split('\n')
            .map(|line| {
                let words = line.trim_start();
                let pad = &line[..line.len() - words.len()];
                format!("{pad}{}", pen.role("muted", words))
            })
            .collect();
        format!("{}  {}", pen.title(&self.name), about.join("\n"))
    }

    /// As a person reads it: set at the reading measure, whatever the
    /// room.
    pub fn text(&self, pen: &Pen, room: Room) -> String {
        let measure = room.measure();
        let options = self.every_option();
        let commands = self.every_command();
        let groups: [(&str, &[Entry]); 4] = [
            ("Examples", &self.examples),
            ("Arguments", &self.arguments),
            ("Commands", &commands),
            ("Options", &options),
        ];
        let column = groups
            .iter()
            .flat_map(|(_, entries)| entries.iter())
            .map(|entry| layout::width(&entry.name))
            .filter(|width| *width <= NAMES)
            .max()
            .unwrap_or(0)
            + GAP;
        let mut blocks = vec![vec![self.head(pen, measure)]];
        for (label, entries) in groups {
            if entries.is_empty() {
                continue;
            }
            let mut block = vec![pen.label(label)];
            block.extend(
                entries
                    .iter()
                    .flat_map(|entry| entered(entry, column, measure, pen)),
            );
            blocks.push(block);
        }
        let exits: Vec<String> = self
            .every_exit()
            .iter()
            .map(|(exit, words)| format!("{} {words}", exit.code()))
            .collect();
        let mut block = vec![pen.label("Exit")];
        block.extend(
            layout::wrapped(&exits.join(ITEMS), measure.saturating_sub(INSET))
                .iter()
                .map(|line| format!("{}{}", " ".repeat(INSET), pen.role("muted", line))),
        );
        blocks.push(block);
        layout::joined(blocks)
    }

    /// What a bare command shows where it has no state to: the first
    /// line, the examples, and where the rest is.
    pub fn brief(&self, pen: &Pen, room: Room) -> String {
        let column = self
            .examples
            .iter()
            .map(|entry| layout::width(&entry.name))
            .max()
            .unwrap_or(0)
            + GAP;
        let mut blocks = vec![vec![self.head(pen, room.measure())]];
        if !self.examples.is_empty() {
            let mut block = vec![pen.label("Examples")];
            block.extend(
                self.examples
                    .iter()
                    .flat_map(|entry| entered(entry, column, room.measure(), pen)),
            );
            blocks.push(block);
        }
        blocks.push(vec![
            pen.voice(&format!("{} --help says everything", self.name)),
        ]);
        layout::joined(blocks)
    }

    /// As the check reads it.
    pub fn json(&self) -> Value {
        let entries = |entries: &[Entry]| -> Value {
            entries
                .iter()
                .map(|entry| json!({"name": entry.name, "words": entry.words}))
                .collect()
        };
        let exits: Value = self
            .every_exit()
            .iter()
            .map(|(exit, words)| json!({"code": exit.code(), "kind": exit.kind(), "words": words}))
            .collect();
        let formats: Vec<String> = ["text", "json", "jsonl"]
            .into_iter()
            .map(str::to_owned)
            .chain(self.forms.iter().cloned())
            .collect();
        json!({
            "schema": "help/1",
            "name": self.name,
            "about": self.about,
            "examples": entries(&self.examples),
            "arguments": entries(&self.arguments),
            "commands": entries(&self.every_command()),
            "aliases": self.aliases,
            "options": entries(&self.every_option()),
            "formats": formats,
            "exits": exits,
        })
    }
}

/// One entry's lines: its name, then its words in muted at the column,
/// wrapped under themselves; a name past the column puts its words on
/// the line under it. Where the column would leave the words fewer than
/// `FEWEST_WORDS` cells, as in a narrow terminal, every entry's words go
/// under its name, two cells in, rather than down a sliver beside it.
fn entered(entry: &Entry, column: usize, measure: usize, pen: &Pen) -> Vec<String> {
    let inset = " ".repeat(INSET);
    let column = if measure.saturating_sub(INSET + column) < FEWEST_WORDS {
        GAP
    } else {
        column
    };
    let at = INSET + column;
    let words = layout::wrapped(&entry.words, measure.saturating_sub(at).max(1));
    let name_width = layout::width(&entry.name);
    let under = " ".repeat(at);
    let mut lines = Vec::new();
    let mut words = words.iter().filter(|line| !line.is_empty());
    if name_width + GAP <= column {
        let name = pen.role("text", &entry.name);
        let pad = " ".repeat(column - name_width);
        match words.next() {
            Some(first) => lines.push(format!("{inset}{name}{pad}{}", pen.role("muted", first))),
            None => lines.push(format!("{inset}{name}")),
        }
    } else {
        // A name too long for a line of its own wraps at its words, each
        // further line under its first, as an example's command does.
        let named = layout::wrapped(&entry.name, measure.saturating_sub(INSET).max(1));
        lines.extend(named.iter().enumerate().map(|(at, line)| {
            let lead = if at == 0 { INSET } else { INSET + GAP };
            format!("{}{}", " ".repeat(lead), pen.role("text", line))
        }));
    }
    lines.extend(words.map(|line| format!("{under}{}", pen.role("muted", line))));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launcher() -> Help {
        Help::new(
            "launcher",
            "any place, row or drawer the launcher holds, in a terminal",
        )
        .example(
            "launcher show machine disk",
            "the disk's row and what it holds",
        )
        .example("launcher show --drawers", "home, with every row's drawer")
        .command("show", "a place or a row, as the launcher shows it")
        .command("do", "what a row does on Return")
        .option("--yes", "do an act that asks first, unasked")
        .exit(Exit::NotFound, "no such row")
        .exit(Exit::Unavailable, "the shell isn't answering")
    }

    #[test]
    fn help_reads_examples_first_every_name_in_one_column() {
        assert_eq!(
            launcher().text(&Pen::plain(), Room::Cells(100)),
            "launcher  any place, row or drawer the launcher holds, in a terminal\n\
             \n\
             EXAMPLES\n  \
             launcher show machine disk  the disk's row and what it holds\n  \
             launcher show --drawers     home, with every row's drawer\n\
             \n\
             COMMANDS\n  \
             show                        a place or a row, as the launcher shows it\n  \
             do                          what a row does on Return\n\
             \n\
             OPTIONS\n  \
             --yes                       do an act that asks first, unasked\n  \
             --format text|json|jsonl    the answer's form; --json is json\n  \
             --color never|always|auto   whether to ink; NO_COLOR and FORCE_COLOR too\n  \
             -h, --help                  this\n  \
             --version                   its version\n\
             \n\
             EXIT\n  \
             0 done \u{b7} 64 asked wrongly \u{b7} 66 no such row \u{b7} 69 the shell isn't answering\n"
        );
    }

    #[test]
    fn words_too_long_for_the_measure_wrap_under_themselves() {
        let help = Help::new("x", "y").command("go", "one two three four five six");
        let text = help.text(&Pen::plain(), Room::Cells(54));
        assert!(
            text.contains("  go                         one two three four\n"),
            "{text}"
        );
        assert!(
            text.contains("\n                             five six\n"),
            "{text}"
        );
    }

    #[test]
    fn a_narrow_terminal_puts_words_under_their_names() {
        let help = Help::new("launcher", "any place, row or drawer the launcher holds")
            .example(
                "launcher show machine disk --drawers",
                "the disk and its drawers",
            )
            .command("go", "one two three four five six");
        let text = help.text(&Pen::plain(), Room::Cells(30));
        assert!(
            text.starts_with(
                "launcher  any place, row or\n          drawer the\n          launcher holds\n"
            ),
            "{text}"
        );
        assert!(
            text.contains(
                "\n  launcher show machine\n    disk --drawers\n    the disk and its drawers\n"
            ),
            "{text}"
        );
        assert!(
            text.contains("\n  go\n    one two three four\n    five six\n"),
            "{text}"
        );
        assert!(text.lines().all(|line| layout::width(line) <= 30), "{text}");
    }

    #[test]
    fn a_bare_command_shows_its_examples_and_where_the_rest_is() {
        assert_eq!(
            launcher().brief(&Pen::plain(), Room::Cells(100)),
            "launcher  any place, row or drawer the launcher holds, in a terminal\n\
             \n\
             EXAMPLES\n  \
             launcher show machine disk  the disk's row and what it holds\n  \
             launcher show --drawers     home, with every row's drawer\n\
             \n\
             launcher --help says everything\n"
        );
    }

    #[test]
    fn as_json_it_holds_the_shared_flags_and_every_exit() {
        let value = launcher().form("markdown").json();
        assert_eq!(value["schema"], "help/1");
        assert_eq!(
            value["formats"],
            json!(["text", "json", "jsonl", "markdown"])
        );
        let options: Vec<&str> = value["options"]
            .as_array()
            .expect("options are a list")
            .iter()
            .filter_map(|option| option["name"].as_str())
            .collect();
        assert!(
            options.contains(&"--format text|json|jsonl|markdown"),
            "{options:?}"
        );
        assert!(options.contains(&"-h, --help"), "{options:?}");
        let codes: Vec<u64> = value["exits"]
            .as_array()
            .expect("exits are a list")
            .iter()
            .filter_map(|exit| exit["code"].as_u64())
            .collect();
        assert_eq!(codes, [0, 64, 66, 69]);
        assert_eq!(value["exits"][2]["kind"], "not-found");
    }
}
