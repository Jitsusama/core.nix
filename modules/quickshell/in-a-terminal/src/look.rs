//! How the launcher's words are inked in a terminal: the theme's roles, as
//! the file its install gives (`LAUNCHER_LOOK`, written by ../home.nix)
//! spells each, a 24-bit escape where the terminal says it draws them and
//! otherwise the one of its sixteen colours that holds the role's pigment
//! or near it (../../theme/nearest.nix), and no ink at all into a pipe or
//! with `NO_COLOR`. On the console (`TERM=linux`) each mark is its stand-in
//! (../../theme/glyphs.nix's `console`), which its font can draw.

use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, IsTerminal};

use serde_json::Value;

/// How a terminal is inked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colours {
    /// Plain words, into a pipe or where `NO_COLOR` asks for none.
    None,
    /// The terminal's sixteen colours.
    Sixteen,
    /// 24-bit colour, where `COLORTERM` says the terminal draws it.
    Truecolour,
}

/// How what's printed is inked, and whether its marks are the console's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ink {
    pub colours: Colours,
    pub console: bool,
}

impl Ink {
    /// How stdout is inked, by the terminal it reaches.
    pub fn for_stdout() -> Self {
        Self::of(io::stdout().is_terminal())
    }

    /// How stderr is inked, by the terminal it reaches.
    pub fn for_stderr() -> Self {
        Self::of(io::stderr().is_terminal())
    }

    fn of(terminal: bool) -> Self {
        Self::by(
            terminal,
            env::var_os("NO_COLOR").is_some_and(|set| !set.is_empty()),
            env::var("COLORTERM").unwrap_or_default().as_str(),
            env::var("TERM").unwrap_or_default().as_str(),
        )
    }

    /// How a terminal is inked, by what it is and what its environment
    /// says: no colour into a pipe or with `NO_COLOR`, 24-bit where
    /// `COLORTERM` says truecolor or 24bit, else the sixteen.
    pub fn by(terminal: bool, no_colour: bool, colorterm: &str, term: &str) -> Self {
        let colours = if !terminal || no_colour {
            Colours::None
        } else if matches!(colorterm, "truecolor" | "24bit") {
            Colours::Truecolour
        } else {
            Colours::Sixteen
        };
        Self {
            colours,
            console: term == "linux",
        }
    }
}

/// One role's escapes, without their `\e[` and `m`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Role {
    truecolour: String,
    sixteen: String,
}

/// The theme's roles and marks, and how this terminal is inked.
#[derive(Debug, Clone)]
pub struct Look {
    ink: Ink,
    roles: HashMap<String, Role>,
    /// Each mark's console stand-in, by the mark.
    stand_ins: HashMap<String, String>,
    failed: String,
}

impl Look {
    /// The look its install gives, or plain words where it gives none.
    pub fn read(path: Option<&OsStr>, ink: Ink) -> Self {
        let value = path
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or(Value::Null);
        Self::from_value(&value, ink)
    }

    /// The look a file of the install's shape spells.
    pub fn from_value(value: &Value, ink: Ink) -> Self {
        let text = |value: &Value, key: &str| {
            value
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let roles = value
            .get("roles")
            .and_then(Value::as_object)
            .map(|roles| {
                roles
                    .iter()
                    .map(|(name, role)| {
                        let role = Role {
                            truecolour: text(role, "truecolour"),
                            sixteen: text(role, "sixteen"),
                        };
                        (name.clone(), role)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut stand_ins: HashMap<String, String> = value
            .get("marks")
            .and_then(Value::as_array)
            .map(|marks| {
                marks
                    .iter()
                    .map(|mark| (text(mark, "glyph"), text(mark, "console")))
                    .collect()
            })
            .unwrap_or_default();
        // The failure mark carries its own stand-in, since it's said
        // on stderr whether or not a row's marks hold it.
        let failed = value
            .get("failed")
            .map(|mark| text(mark, "glyph"))
            .unwrap_or_default();
        if let Some(console) = value.get("failed").map(|mark| text(mark, "console")) {
            stand_ins.insert(failed.clone(), console);
        }
        Self {
            ink,
            roles,
            stand_ins,
            failed,
        }
    }

    /// Words in a role's colour, or plain where nothing is inked.
    pub fn ink(&self, role: &str, words: &str) -> String {
        self.inked(role, words, false)
    }

    /// A title: bold, in strong, the only words set bold.
    pub fn title(&self, words: &str) -> String {
        self.inked("strong", words, true)
    }

    fn inked(&self, role: &str, words: &str, bold: bool) -> String {
        if words.is_empty() || self.ink.colours == Colours::None {
            return words.to_string();
        }
        let colour = self
            .roles
            .get(role)
            .map_or("", |role| match self.ink.colours {
                Colours::Truecolour => role.truecolour.as_str(),
                _ => role.sixteen.as_str(),
            });
        let escape = match (bold, colour.is_empty()) {
            (true, true) => "1".to_string(),
            (true, false) => format!("1;{colour}"),
            (false, true) => return words.to_string(),
            (false, false) => colour.to_string(),
        };
        format!("\u{1b}[{escape}m{words}\u{1b}[0m")
    }

    /// A mark as this terminal draws it: its stand-in on the console.
    pub fn mark(&self, glyph: &str) -> String {
        if self.ink.console {
            self.stand_ins.get(glyph).cloned().unwrap_or_default()
        } else {
            glyph.to_string()
        }
    }

    /// The theme's mark for a failure, as this terminal draws it.
    pub fn failure(&self) -> String {
        self.mark(&self.failed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look(ink: Ink) -> Look {
        let value: Value = serde_json::from_str(
            r#"{"roles":{"strong":{"truecolour":"38;2;247;232;178","sixteen":"97"},
                "muted":{"truecolour":"38;2;183;182;157","sixteen":"39"}},
               "marks":[{"glyph":"\u25c6","console":"*"}],
               "failed":{"glyph":"\u2715","console":"x"}}"#,
        )
        .expect("the look parses");
        Look::from_value(&value, ink)
    }

    const TRUE: Ink = Ink {
        colours: Colours::Truecolour,
        console: false,
    };
    const SIXTEEN: Ink = Ink {
        colours: Colours::Sixteen,
        console: false,
    };
    const PLAIN: Ink = Ink {
        colours: Colours::None,
        console: false,
    };

    #[test]
    fn a_pipe_and_no_colour_are_plain() {
        assert_eq!(
            Ink::by(false, false, "truecolor", "xterm-kitty").colours,
            Colours::None
        );
        assert_eq!(
            Ink::by(true, true, "truecolor", "xterm-kitty").colours,
            Colours::None
        );
    }

    #[test]
    fn truecolour_only_where_colorterm_says_so() {
        assert_eq!(
            Ink::by(true, false, "truecolor", "xterm-kitty").colours,
            Colours::Truecolour
        );
        assert_eq!(
            Ink::by(true, false, "24bit", "xterm").colours,
            Colours::Truecolour
        );
        assert_eq!(
            Ink::by(true, false, "", "xterm-kitty").colours,
            Colours::Sixteen
        );
    }

    #[test]
    fn the_console_draws_its_stand_ins() {
        let console = Ink {
            colours: Colours::Sixteen,
            console: true,
        };
        assert!(Ink::by(true, false, "", "linux").console);
        assert_eq!(look(console).mark("\u{25c6}"), "*");
        assert_eq!(look(console).failure(), "x");
        assert_eq!(look(SIXTEEN).mark("\u{25c6}"), "\u{25c6}");
    }

    #[test]
    fn a_role_is_inked_as_the_terminal_draws() {
        assert_eq!(
            look(TRUE).ink("strong", "Disk"),
            "\u{1b}[38;2;247;232;178mDisk\u{1b}[0m"
        );
        assert_eq!(
            look(SIXTEEN).ink("strong", "Disk"),
            "\u{1b}[97mDisk\u{1b}[0m"
        );
        assert_eq!(look(PLAIN).ink("strong", "Disk"), "Disk");
    }

    #[test]
    fn only_a_title_is_bold() {
        assert_eq!(look(SIXTEEN).title("Disk"), "\u{1b}[1;97mDisk\u{1b}[0m");
        assert_eq!(look(PLAIN).title("Disk"), "Disk");
    }

    #[test]
    fn a_role_the_look_lacks_is_plain() {
        assert_eq!(look(SIXTEEN).ink("action", "Go"), "Go");
    }
}
