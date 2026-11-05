//! The reader's command line, after `:`: the few of Neovim's commands a
//! page that writes nothing has, read as Neovim reads them, abbreviations
//! and all. Anything else is Neovim's E492, said as Neovim says it.

/// What a command line asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// :{n}, a line of the file as written; :$, its last.
    Line(usize),
    Last,
    /// :q, :quit, :qa and :qall, with or without !.
    Quit,
    /// :noh to :nohlsearch: the search's marks put away until n or N.
    NoHighlight,
    /// :d to :delete, which a page that writes nothing refuses.
    Change,
    /// Nothing typed, which does nothing, as in Neovim.
    Nothing,
    /// Anything else, as typed, for its message.
    Unknown(String),
}

/// Whether `typed` is `name` or one of its abbreviations down to `least`
/// letters, as Neovim takes :noh for :nohlsearch.
fn abbreviates(typed: &str, name: &str, least: usize) -> bool {
    typed.len() >= least && name.starts_with(typed)
}

/// Reads a command line as typed after its `:`. Colons and spaces before
/// it are passed over, as Neovim passes them.
pub fn parse(typed: &str) -> Command {
    let command = typed.trim_start_matches([':', ' ']).trim_end();
    if command.is_empty() {
        return Command::Nothing;
    }
    if command == "$" {
        return Command::Last;
    }
    if let Ok(line) = command.parse::<usize>() {
        return Command::Line(line);
    }
    let name = command.strip_suffix('!').unwrap_or(command);
    if abbreviates(name, "quit", 1) || abbreviates(name, "qall", 2) {
        return Command::Quit;
    }
    if abbreviates(name, "nohlsearch", 3) {
        return Command::NoHighlight;
    }
    if abbreviates(name, "delete", 1) {
        return Command::Change;
    }
    Command::Unknown(command.to_string())
}

/// What a change is told, as Neovim tells one in a buffer it can't modify.
pub const CANNOT_CHANGE: &str = "E21: Cannot make changes";

/// What the command line says of a command it doesn't have, as Neovim does.
pub fn unknown(command: &str) -> String {
    format!("E492: Not an editor command: {command}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_number_is_a_line_and_a_dollar_the_last() {
        assert_eq!(parse("12"), Command::Line(12));
        assert_eq!(parse(" :12 "), Command::Line(12));
        assert_eq!(parse("$"), Command::Last);
    }

    #[test]
    fn quit_and_noh_are_read_as_neovim_abbreviates_them() {
        for typed in ["q", "qu", "quit", "q!", "qa", "qall!"] {
            assert_eq!(parse(typed), Command::Quit, "{typed}");
        }
        for typed in ["noh", "nohl", "nohlsearch"] {
            assert_eq!(parse(typed), Command::NoHighlight, "{typed}");
        }
        assert_eq!(parse("no"), Command::Unknown("no".into()));
    }

    #[test]
    fn delete_is_read_to_be_refused() {
        for typed in ["d", "de", "delete"] {
            assert_eq!(parse(typed), Command::Change, "{typed}");
        }
        assert_eq!(parse("dx"), Command::Unknown("dx".into()));
    }

    #[test]
    fn nothing_typed_does_nothing_and_anything_else_is_unknown() {
        assert_eq!(parse(""), Command::Nothing);
        assert_eq!(parse("w"), Command::Unknown("w".into()));
        assert_eq!(unknown("w"), "E492: Not an editor command: w");
    }
}
