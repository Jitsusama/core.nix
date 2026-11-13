//! The flags every command shares, taken out of its arguments before it
//! reads the rest its own way: `--format`, `--json`, `--color`, `-h`,
//! `--help` and `--version` (docs/commands.md, The Convention). Nothing
//! after `--` is taken, since that's the command's to pass on whole.

use crate::failure::{Exit, Failure};

/// The form an answer takes on stdout.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Format {
    /// Laid out for a person, in a terminal or a pipe alike.
    #[default]
    Text,
    /// One JSON object, the whole answer.
    Json,
    /// One JSON object a line, ending with an `end` line.
    Jsonl,
    /// A form of the command's own beside the three, such as spend-watch's
    /// `markdown`, by its name.
    Own(String),
}

impl Format {
    /// The name `--format` takes for it.
    pub fn name(&self) -> &str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::Jsonl => "jsonl",
            Self::Own(name) => name,
        }
    }

    /// Whether stdout and errors are JSON under it.
    pub fn is_json(&self) -> bool {
        matches!(self, Self::Json | Self::Jsonl)
    }
}

/// What `--color` asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Colour {
    /// As the environment and the stream say.
    #[default]
    Auto,
    /// Ink, even into a pipe.
    Always,
    /// No ink, even into a terminal.
    Never,
}

/// The shared flags as a command line gave them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Shared {
    pub format: Format,
    pub colour: Colour,
    /// `-h` or `--help` was given, which wins over everything else.
    pub help: bool,
    /// `--version` was given.
    pub version: bool,
}

/// The shared flags taken from `args`, and what's left for the command,
/// in its order. `forms` names the forms of its own a command offers
/// beside the three.
pub fn take(args: Vec<String>, forms: &[&str]) -> Result<(Shared, Vec<String>), Failure> {
    let mut shared = Shared::default();
    let mut rest = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--" {
            rest.push(arg);
            rest.extend(args);
            break;
        }
        let (flag, attached) = match arg.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => {
                (flag.to_owned(), Some(value.to_owned()))
            }
            _ => (arg.clone(), None),
        };
        match flag.as_str() {
            "-h" | "--help" => shared.help = true,
            "--version" => shared.version = true,
            "--json" => shared.format = Format::Json,
            "--format" => {
                let value = attached.or_else(|| args.next());
                shared.format = format(value.as_deref(), forms)?;
            }
            "--color" => {
                let value = attached.or_else(|| args.next());
                shared.colour = colour(value.as_deref())?;
            }
            _ => rest.push(arg),
        }
    }
    Ok((shared, rest))
}

fn format(value: Option<&str>, forms: &[&str]) -> Result<Format, Failure> {
    match value {
        Some("text") => Ok(Format::Text),
        Some("json") => Ok(Format::Json),
        Some("jsonl") => Ok(Format::Jsonl),
        Some(own) if forms.contains(&own) => Ok(Format::Own(own.to_owned())),
        _ => {
            let names: Vec<&str> = ["text", "json", "jsonl"]
                .into_iter()
                .chain(forms.iter().copied())
                .collect();
            Err(asked_wrongly("--format", value, &names))
        }
    }
}

fn colour(value: Option<&str>) -> Result<Colour, Failure> {
    match value {
        Some("auto") => Ok(Colour::Auto),
        Some("always") => Ok(Colour::Always),
        Some("never") => Ok(Colour::Never),
        _ => Err(asked_wrongly(
            "--color",
            value,
            &["never", "always", "auto"],
        )),
    }
}

fn asked_wrongly(flag: &str, value: Option<&str>, names: &[&str]) -> Failure {
    let what = match value {
        Some(value) => format!("{flag} can't be {value}"),
        None => format!("{flag} needs a value"),
    };
    Failure::new(Exit::Usage, what).because(format!("it takes {}", or(names)))
}

/// Names as a person offers a choice of them: "a, b or c".
pub fn or(names: &[&str]) -> String {
    listed(names, "or")
}

/// Names as a person lists them all: "a, b and c".
pub fn and(names: &[&str]) -> String {
    listed(names, "and")
}

fn listed(names: &[&str], last_by: &str) -> String {
    match names.split_last() {
        None => String::new(),
        Some((last, [])) => (*last).to_owned(),
        Some((last, before)) => format!("{} {last_by} {last}", before.join(", ")),
    }
}

/// The one of `known` nearest `word`, where one is near enough to be what
/// was meant: a start of only one, or within a third of its letters by
/// edits, as a mistake is suggested and never run (docs/commands.md,
/// Help).
pub fn nearest<'a>(word: &str, known: &[&'a str]) -> Option<&'a str> {
    let starting: Vec<&&str> = known.iter().filter(|name| name.starts_with(word)).collect();
    if let [only] = starting.as_slice()
        && !word.is_empty()
    {
        return Some(only);
    }
    known
        .iter()
        .map(|name| (*name, edits(word, name)))
        .filter(|(name, apart)| *apart > 0 && *apart <= name.chars().count().div_ceil(3))
        .min_by_key(|(_, apart)| *apart)
        .map(|(name, _)| name)
}

/// How many letters must change, go or come to make one word the other.
fn edits(one: &str, other: &str) -> usize {
    let other: Vec<char> = other.chars().collect();
    let mut above: Vec<usize> = (0..=other.len()).collect();
    for (row, letter) in one.chars().enumerate() {
        let mut here = vec![row + 1; other.len() + 1];
        for (column, against) in other.iter().enumerate() {
            let change = above[column] + usize::from(letter != *against);
            here[column + 1] = change.min(above[column + 1] + 1).min(here[column] + 1);
        }
        above = here;
    }
    above[other.len()]
}

/// The failure for a word a command doesn't know, naming the nearest it
/// does where one is near.
pub fn unknown(what: &str, word: &str, known: &[&str]) -> Failure {
    let failure = Failure::new(Exit::Usage, format!("no {what} called {word}"));
    match nearest(word, known) {
        Some(near) => failure.because(format!("did you mean {near}?")),
        None => failure.because(format!("the {what}s are {}", and(known))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    fn taken(line: &str) -> (Shared, Vec<String>) {
        take(args(line), &["markdown"]).expect("the line is taken")
    }

    #[test]
    fn the_shared_flags_come_out_and_the_rest_keeps_its_order() {
        let (shared, rest) = taken("show --json machine --color never disk -v");
        assert_eq!(shared.format, Format::Json);
        assert_eq!(shared.colour, Colour::Never);
        assert_eq!(rest, args("show machine disk -v"));
    }

    #[test]
    fn a_value_comes_attached_or_after() {
        assert_eq!(taken("--format=jsonl").0.format, Format::Jsonl);
        assert_eq!(taken("--format jsonl").0.format, Format::Jsonl);
        assert_eq!(taken("--color=always").0.colour, Colour::Always);
    }

    #[test]
    fn a_form_of_the_commands_own_is_taken_by_its_name() {
        assert_eq!(
            taken("--format markdown").0.format,
            Format::Own("markdown".into())
        );
    }

    #[test]
    fn nothing_after_a_double_dash_is_taken() {
        let (shared, rest) = taken("run -- --help --json");
        assert!(!shared.help, "--help after -- is the command's");
        assert_eq!(shared.format, Format::Text);
        assert_eq!(rest, args("run -- --help --json"));
    }

    #[test]
    fn help_is_seen_wherever_it_is() {
        assert!(taken("show machine -h").0.help, "-h at the end is help");
        assert!(taken("--help show").0.help, "--help first is help");
        assert!(taken("--version").0.version, "--version is the version");
    }

    #[test]
    fn a_value_it_cant_take_is_asked_wrongly_and_says_what_it_takes() {
        let failure = take(args("--format yaml"), &[]).expect_err("yaml isn't a form");
        assert_eq!(failure.exit, Exit::Usage);
        assert_eq!(failure.what, "--format can't be yaml");
        assert_eq!(failure.why.as_deref(), Some("it takes text, json or jsonl"));
        let failure = take(args("--color"), &[]).expect_err("--color needs a value");
        assert_eq!(failure.what, "--color needs a value");
    }

    #[test]
    fn a_mistake_names_the_nearest_word() {
        let known = ["show", "do", "completion"];
        assert_eq!(nearest("shwo", &known), Some("show"));
        assert_eq!(nearest("comp", &known), Some("completion"));
        assert_eq!(nearest("zebra", &known), None);
        let failure = unknown("subcommand", "shwo", &known);
        assert_eq!(failure.what, "no subcommand called shwo");
        assert_eq!(failure.why.as_deref(), Some("did you mean show?"));
        assert_eq!(
            unknown("subcommand", "zebra", &known).why.as_deref(),
            Some("the subcommands are show, do and completion")
        );
    }
}
