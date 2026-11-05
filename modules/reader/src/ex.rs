//! The reader's command line, after `:`: the few of Neovim's commands a
//! page that writes nothing has, read as Neovim reads them, ranges and
//! abbreviations and all. Anything else is the error Neovim says of it.

/// The lines a command is given, as Neovim's ranges name them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Range {
    /// None given: the cursor's line.
    Here,
    /// From one line of the file as written to another, from 1; `$`, the
    /// last line, is `usize::MAX`, and `%` is every line.
    Lines(usize, usize),
    /// `'<,'>`: the lines the last selection took.
    Selected,
}

/// What a command line asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// A range alone: to its last line, as :12 and :$ go.
    Go(Range),
    /// :q, :quit, :qa and :qall, with or without !.
    Quit,
    /// :noh to :nohlsearch: the search's marks put away until n or N.
    NoHighlight,
    /// :d to :delete, which a page that writes nothing refuses.
    Change,
    /// :y to :yank: its lines copied as the file's markdown.
    Copy(Range),
    /// :e to :edit a file, or with none the file shown read again.
    Edit(Option<String>),
    /// :bn to :bnext, or :bp to :bprevious when not on, a count of files.
    File { on: bool, times: u32 },
    /// :h to :help: the key list.
    Help,
    /// :g/pat/ over a range: the pieces matching, listed as gO lists the
    /// headings; an empty pattern is the last search's.
    Global { pattern: String, range: Range },
    /// :marks, or only those of the letters given.
    Marks(String),
    /// :ju to :jumps.
    Jumps,
    /// Nothing typed, which does nothing, as in Neovim.
    Nothing,
    /// What Neovim says of a line it won't run, as it says it.
    Refused(String),
}

/// Whether `typed` is `name` or one of its abbreviations down to `least`
/// letters, as Neovim takes :noh for :nohlsearch.
fn abbreviates(typed: &str, name: &str, least: usize) -> bool {
    typed.len() >= least && name.starts_with(typed)
}

/// A line number in a range: digits, or `$` for the last.
fn line_number(typed: &str) -> Option<(usize, &str)> {
    if let Some(rest) = typed.strip_prefix('$') {
        return Some((usize::MAX, rest));
    }
    let digits = typed.len() - typed.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let line = typed[..digits].parse().ok()?;
    Some((line, &typed[digits..]))
}

/// The range a command line starts with, if any, and what follows it.
fn range(typed: &str) -> (Option<Range>, &str) {
    if let Some(rest) = typed.strip_prefix('%') {
        return (Some(Range::Lines(1, usize::MAX)), rest);
    }
    if let Some(rest) = typed.strip_prefix("'<,'>") {
        return (Some(Range::Selected), rest);
    }
    if let Some(rest) = typed.strip_prefix('.') {
        return (Some(Range::Here), rest);
    }
    let Some((first, rest)) = line_number(typed) else {
        return (None, typed);
    };
    match rest.strip_prefix(',').and_then(line_number) {
        // A backwards range is swapped, as Neovim offers to.
        Some((last, rest)) => (Some(Range::Lines(first.min(last), first.max(last))), rest),
        None => (Some(Range::Lines(first, first)), rest),
    }
}

/// Reads a command line as typed after its `:`. Colons and spaces before
/// it are passed over, as Neovim passes them.
pub fn parse(typed: &str) -> Command {
    let line = typed.trim_start_matches([':', ' ']).trim_end();
    let (given, rest) = range(line);
    let rest = rest.trim_start();
    let letters = rest.len()
        - rest
            .trim_start_matches(|c: char| c.is_ascii_alphabetic())
            .len();
    let (name, after) = rest.split_at(letters);
    let bang = after.starts_with('!');
    let argument = after.strip_prefix('!').unwrap_or(after).trim();
    if name.is_empty() {
        return match (given, after) {
            (None, "") => Command::Nothing,
            (Some(range), "") => Command::Go(range),
            _ => Command::Refused(unknown(line)),
        };
    }
    let command = if abbreviates(name, "quit", 1) || abbreviates(name, "qall", 2) {
        Some((Command::Quit, true, false))
    } else if abbreviates(name, "nohlsearch", 3) {
        Some((Command::NoHighlight, false, false))
    } else if abbreviates(name, "delete", 1) {
        Some((Command::Change, false, true))
    } else if abbreviates(name, "yank", 1) {
        Some((Command::Copy(given.unwrap_or(Range::Here)), false, true))
    } else if abbreviates(name, "edit", 1) {
        let file = (!argument.is_empty()).then(|| argument.to_string());
        return refusal(given, false, None).unwrap_or(Command::Edit(file));
    } else if abbreviates(name, "bnext", 2) || abbreviates(name, "bprevious", 2) {
        let on = name.starts_with("bn");
        let times = match argument {
            "" => 1,
            count => match count.parse() {
                Ok(times) => times,
                Err(_) => return Command::Refused(trailing(count)),
            },
        };
        return refusal(given, false, None).unwrap_or(Command::File { on, times });
    } else if abbreviates(name, "help", 1) {
        if !argument.is_empty() {
            return Command::Refused(format!("E149: Sorry, no help for {argument}"));
        }
        Some((Command::Help, true, false))
    } else if abbreviates(name, "global", 1) {
        if bang {
            return Command::Refused("E477: No ! allowed".into());
        }
        return global(
            given.unwrap_or(Range::Lines(1, usize::MAX)),
            after.trim_start(),
            line,
        );
    } else if abbreviates(name, "marks", 5) {
        let letters = argument.chars().filter(|c| !c.is_whitespace()).collect();
        return refusal(given, bang, None).unwrap_or(Command::Marks(letters));
    } else if abbreviates(name, "jumps", 2) {
        Some((Command::Jumps, false, false))
    } else {
        None
    };
    let Some((command, takes_bang, takes_range)) = command else {
        return Command::Refused(unknown(line));
    };
    // :y takes a register's name, and every register is the clipboard.
    let takes = matches!(command, Command::Copy(_)) && argument.chars().count() == 1;
    let argument = (!argument.is_empty() && !takes).then_some(argument);
    refusal(
        given.filter(|_| !takes_range),
        bang && !takes_bang,
        argument,
    )
    .unwrap_or(command)
}

/// :g/pat/ over a range, `/` any delimiter but a letter, listing the
/// pieces it matches; with :d after it, the change refused, and any other
/// command, one the page hasn't.
fn global(range: Range, typed: &str, line: &str) -> Command {
    let Some(delimiter) = typed.chars().next() else {
        return Command::Refused("E148: Regular expression missing from :global".into());
    };
    if delimiter.is_alphanumeric() || matches!(delimiter, '\\' | '"' | '|') {
        return Command::Refused("E146: Regular expressions can't be delimited by letters".into());
    }
    let rest = &typed[delimiter.len_utf8()..];
    let (pattern, command) = rest.split_once(delimiter).unwrap_or((rest, ""));
    match command.trim() {
        "" | "p" | "pr" | "print" | "#" | "nu" | "number" | "l" | "list" => Command::Global {
            pattern: pattern.to_string(),
            range,
        },
        command if parse(command) == Command::Change => Command::Change,
        _ => Command::Refused(unknown(line)),
    }
}

/// What Neovim says of a range, a ! or words a command doesn't take.
fn refusal(range: Option<Range>, bang: bool, argument: Option<&str>) -> Option<Command> {
    if range.is_some() {
        Some(Command::Refused("E481: No range allowed".into()))
    } else if bang {
        Some(Command::Refused("E477: No ! allowed".into()))
    } else {
        argument.map(|words| Command::Refused(trailing(words)))
    }
}

/// What a change is told, as Neovim tells one in a buffer it can't modify.
pub const CANNOT_CHANGE: &str = "E21: Cannot make changes";

/// What :'<,'> is told before anything was selected.
pub const NOTHING_SELECTED: &str = "E20: Mark not set";

/// What a range past the file's end is told.
pub const PAST_THE_END: &str = "E16: Invalid range";

/// What the command line says of a command it doesn't have, as Neovim does.
fn unknown(command: &str) -> String {
    format!("E492: Not an editor command: {command}")
}

/// What Neovim says of words after a command that takes none.
fn trailing(words: &str) -> String {
    format!("E488: Trailing characters: {words}")
}

/// What :y says once it has copied, as Neovim says it past 'report''s two.
pub fn copied(lines: usize) -> Option<String> {
    (lines > 2).then(|| format!("{lines} lines yanked"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refused(typed: &str) -> String {
        match parse(typed) {
            Command::Refused(said) => said,
            other => panic!("{typed} is {other:?}, not refused"),
        }
    }

    #[test]
    fn a_range_alone_goes_to_its_last_line() {
        assert_eq!(parse("12"), Command::Go(Range::Lines(12, 12)));
        assert_eq!(parse(" :12 "), Command::Go(Range::Lines(12, 12)));
        assert_eq!(
            parse("$"),
            Command::Go(Range::Lines(usize::MAX, usize::MAX))
        );
        assert_eq!(parse("%"), Command::Go(Range::Lines(1, usize::MAX)));
        assert_eq!(parse("'<,'>"), Command::Go(Range::Selected));
        assert_eq!(parse("9,3"), Command::Go(Range::Lines(3, 9)), "swapped");
    }

    #[test]
    fn quit_and_noh_are_read_as_neovim_abbreviates_them() {
        for typed in ["q", "qu", "quit", "q!", "qa", "qall!"] {
            assert_eq!(parse(typed), Command::Quit, "{typed}");
        }
        for typed in ["noh", "nohl", "nohlsearch"] {
            assert_eq!(parse(typed), Command::NoHighlight, "{typed}");
        }
        assert_eq!(refused("no"), "E492: Not an editor command: no");
    }

    #[test]
    fn delete_is_read_to_be_refused() {
        for typed in ["d", "de", "delete", "%d", "'<,'>d"] {
            assert_eq!(parse(typed), Command::Change, "{typed}");
        }
        assert_eq!(refused("dx"), "E492: Not an editor command: dx");
    }

    #[test]
    fn yank_takes_a_range_and_a_register() {
        assert_eq!(parse("y"), Command::Copy(Range::Here));
        assert_eq!(parse("%y"), Command::Copy(Range::Lines(1, usize::MAX)));
        assert_eq!(parse("'<,'>yank"), Command::Copy(Range::Selected));
        assert_eq!(parse("3,5y"), Command::Copy(Range::Lines(3, 5)));
        assert_eq!(parse("% y +"), Command::Copy(Range::Lines(1, usize::MAX)));
        assert_eq!(refused("y two"), "E488: Trailing characters: two");
        assert_eq!(copied(2), None);
        assert_eq!(copied(3).as_deref(), Some("3 lines yanked"));
    }

    #[test]
    fn edit_names_a_file_or_reads_this_one_again() {
        assert_eq!(parse("e notes.md"), Command::Edit(Some("notes.md".into())));
        assert_eq!(
            parse("edit! ~/a b.md"),
            Command::Edit(Some("~/a b.md".into()))
        );
        assert_eq!(parse("e"), Command::Edit(None));
        assert_eq!(refused("%e"), "E481: No range allowed");
    }

    #[test]
    fn bnext_and_bprevious_go_on_and_back_by_a_count() {
        assert_eq!(parse("bn"), Command::File { on: true, times: 1 });
        assert_eq!(parse("bnext 3"), Command::File { on: true, times: 3 });
        assert_eq!(
            parse("bp"),
            Command::File {
                on: false,
                times: 1
            }
        );
        assert_eq!(
            parse("bprevious!"),
            Command::File {
                on: false,
                times: 1
            }
        );
        assert_eq!(refused("b"), "E492: Not an editor command: b");
        assert_eq!(refused("bn x"), "E488: Trailing characters: x");
    }

    #[test]
    fn help_lists_the_keys_and_knows_no_subject() {
        for typed in ["h", "he", "help", "help!"] {
            assert_eq!(parse(typed), Command::Help, "{typed}");
        }
        assert_eq!(refused("h gO"), "E149: Sorry, no help for gO");
    }

    #[test]
    fn global_lists_what_its_pattern_matches_over_its_range() {
        let every = Range::Lines(1, usize::MAX);
        let global = |pattern: &str, range| Command::Global {
            pattern: pattern.into(),
            range,
        };
        assert_eq!(parse("g/Part/"), global("Part", every));
        assert_eq!(parse("global/Part"), global("Part", every));
        assert_eq!(parse("g#a/b#p"), global("a/b", every), "any delimiter");
        assert_eq!(parse("'<,'>g/x/"), global("x", Range::Selected));
        assert_eq!(parse("g//"), global("", every), "the last search's");
        assert_eq!(parse("g/x/d"), Command::Change);
        assert_eq!(
            refused("g/x/s/a/b/"),
            "E492: Not an editor command: g/x/s/a/b/"
        );
        assert_eq!(
            refused("g"),
            "E148: Regular expression missing from :global"
        );
        assert_eq!(
            refused("g x"),
            "E146: Regular expressions can't be delimited by letters"
        );
        assert_eq!(refused("g!/x/"), "E477: No ! allowed");
    }

    #[test]
    fn marks_and_jumps_are_listed() {
        assert_eq!(parse("marks"), Command::Marks(String::new()));
        assert_eq!(parse("marks aB"), Command::Marks("aB".into()));
        assert_eq!(refused("mar"), "E492: Not an editor command: mar");
        assert_eq!(parse("ju"), Command::Jumps);
        assert_eq!(parse("jumps"), Command::Jumps);
        assert_eq!(refused("jumps 3"), "E488: Trailing characters: 3");
        assert_eq!(refused("%marks"), "E481: No range allowed");
    }

    #[test]
    fn what_a_command_does_not_take_is_refused_as_neovim_refuses_it() {
        assert_eq!(refused("%q"), "E481: No range allowed");
        assert_eq!(refused("noh!"), "E477: No ! allowed");
        assert_eq!(refused("noh x"), "E488: Trailing characters: x");
    }

    #[test]
    fn nothing_typed_does_nothing_and_anything_else_is_unknown() {
        assert_eq!(parse(""), Command::Nothing);
        assert_eq!(refused("w"), "E492: Not an editor command: w");
        assert_eq!(refused("'a"), "E492: Not an editor command: 'a");
        assert_eq!(refused("%foo"), "E492: Not an editor command: %foo");
        assert_eq!(unknown("w"), "E492: Not an editor command: w");
    }
}
