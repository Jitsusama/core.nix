//! How a command's words are inked: whether a stream takes ink at all, by
//! `--color`, `NO_COLOR`, `FORCE_COLOR` and whether it's a terminal, in
//! that order; and the theme's roles and marks, read from the account's
//! theme.json at each run, so a theme switch reaches every command without
//! a rebuild (docs/commands.md, Colour and Marks). A role is drawn in
//! 24-bit colour where `COLORTERM` says the terminal draws it, and
//! otherwise as the one of the terminal's sixteen colours holding its
//! pigment or near it, found by its colour as modules/theme/nearest.nix
//! finds it, since a theme may put any pigment in any slot.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use crate::environment::Environment;
use crate::flags::Colour;

/// How a stream is inked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colours {
    /// Plain words.
    None,
    /// The terminal's sixteen colours.
    Sixteen,
    /// 24-bit colour.
    Truecolour,
}

/// How a stream is inked, and whether its marks are the console's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ink {
    pub colours: Colours,
    /// On the console (`TERM=linux`), whose font draws only the stand-ins.
    pub console: bool,
}

impl Ink {
    /// No ink, and the theme's own marks.
    pub const fn plain() -> Self {
        Self {
            colours: Colours::None,
            console: false,
        }
    }

    /// The sixteen colours and the theme's own marks, for a test.
    pub const fn sixteen() -> Self {
        Self {
            colours: Colours::Sixteen,
            console: false,
        }
    }

    /// How a stream is inked, first match winning: `--color`, then
    /// `NO_COLOR`, then `FORCE_COLOR`, then whether it's a terminal that
    /// isn't `dumb`.
    pub fn decide(terminal: bool, asked: Colour, environment: &Environment) -> Self {
        let term = environment.var("TERM").unwrap_or_default();
        let inked = match asked {
            Colour::Never => false,
            Colour::Always => true,
            Colour::Auto => {
                if environment.var("NO_COLOR").is_some() {
                    false
                } else if environment.var("FORCE_COLOR").is_some() {
                    true
                } else {
                    terminal && term != "dumb"
                }
            }
        };
        let colours = if !inked {
            Colours::None
        } else if matches!(environment.var("COLORTERM"), Some("truecolor" | "24bit")) {
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

/// Far enough to tell two hues apart, near enough to hold one the theme
/// nudged from a terminal colour, as nearest.nix holds it.
const NEAR: u32 = 48;

/// The terminal's sixteen colours, as theme.json's `terminal` names them.
const SIXTEEN: [&str; 16] = [
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "magenta",
    "cyan",
    "white",
    "bright_black",
    "bright_red",
    "bright_green",
    "bright_yellow",
    "bright_blue",
    "bright_magenta",
    "bright_cyan",
    "bright_white",
];

/// The roles a command may name: the theme's words for what a colour is
/// for, never a colour.
pub const ROLES: [&str; 13] = [
    "text", "strong", "muted", "alert", "warning", "success", "accent", "rule", "action",
    "program", "machine", "person", "voice",
];

/// The colours terminals agree on by name, for a terminal with no theme
/// of the account's to read.
const AGREED: [(&str, &str); 6] = [
    ("muted", "90"),
    ("rule", "90"),
    ("alert", "31"),
    ("success", "32"),
    ("warning", "33"),
    ("accent", "36"),
];

/// The marks a command's states wear, and each one's stand-in, for a
/// terminal with no theme to read them from: modules/theme/glyphs.nix's,
/// but for caution, whose own mark only the theme's font holds, so a
/// terminal without the theme draws its stand-in.
const MARKS: [(&str, &str, &str); 12] = [
    ("done", "\u{2713}", "v"),
    ("failed", "\u{2715}", "x"),
    ("running", "\u{2192}", ">"),
    ("active", "\u{25c8}", "@"),
    ("pending", "\u{25c7}", "o"),
    ("queued", "\u{25e6}", "o"),
    ("stopped", "\u{25a0}", "#"),
    ("cancelled", "\u{2212}", "-"),
    ("caution", "!", "!"),
    ("more", "\u{2026}", "~"),
    ("separator", "\u{b7}", "."),
    ("cursor", "\u{25b8}", ">"),
];

fn channels(hex: &str) -> Option<[u32; 3]> {
    let hex = hex.strip_prefix('#')?;
    let channel = |at: usize| u32::from_str_radix(hex.get(at..at + 2)?, 16).ok();
    Some([channel(0)?, channel(2)?, channel(4)?])
}

fn apart(one: [u32; 3], other: [u32; 3]) -> u32 {
    one.iter()
        .zip(other)
        .map(|(a, b)| a.abs_diff(b).pow(2))
        .sum()
}

/// The SGR colour of the terminal slot nearest a pigment, or none where
/// the nearest is the terminal's own foreground, or none is near.
fn slot(terminal: &Value, colour: [u32; 3]) -> Option<String> {
    let candidates = SIXTEEN
        .iter()
        .map(|name| (Some(*name), terminal[*name].as_str()))
        .chain([(None, terminal["foreground"].as_str())]);
    let (name, distance) = candidates
        .filter_map(|(name, hex)| Some((name, apart(colour, channels(hex?)?))))
        .min_by_key(|(_, distance)| *distance)?;
    let at = SIXTEEN.iter().position(|slot| Some(*slot) == name)?;
    (distance <= NEAR * NEAR).then(|| (if at < 8 { 30 + at } else { 82 + at }).to_string())
}

/// One role's escapes, without their `\e[` and `m`; empty where the role
/// is drawn as the terminal's own text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Escapes {
    truecolour: String,
    sixteen: String,
}

/// A mark and its stand-in.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Mark {
    glyph: String,
    console: String,
}

/// Where the account's theme is: `$XDG_CONFIG_HOME/jitsusama/theme.json`,
/// else under `~/.config`.
pub fn theme_file(environment: &Environment) -> Option<PathBuf> {
    let config = environment
        .var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            environment
                .var("HOME")
                .map(|home| PathBuf::from(home).join(".config"))
        })?;
    Some(config.join("jitsusama/theme.json"))
}

/// The account's theme, where there's one to read.
pub fn theme(environment: &Environment) -> Option<Value> {
    let text = fs::read_to_string(theme_file(environment)?).ok()?;
    serde_json::from_str(&text).ok()
}

/// What a stream is drawn with: its ink, and the theme's roles and marks.
#[derive(Debug, Clone)]
pub struct Pen {
    ink: Ink,
    roles: HashMap<String, Escapes>,
    marks: HashMap<String, Mark>,
}

impl Pen {
    /// A pen for a stream inked as `ink`, from the theme, or from what
    /// terminals agree on where there's none.
    pub fn new(ink: Ink, theme: Option<&Value>) -> Self {
        let mut marks: HashMap<String, Mark> = MARKS
            .iter()
            .map(|(name, glyph, console)| {
                let mark = Mark {
                    glyph: (*glyph).to_owned(),
                    console: (*console).to_owned(),
                };
                ((*name).to_owned(), mark)
            })
            .collect();
        let Some(theme) = theme else {
            let roles = AGREED
                .iter()
                .map(|(role, code)| {
                    let escapes = Escapes {
                        truecolour: (*code).to_owned(),
                        sixteen: (*code).to_owned(),
                    };
                    ((*role).to_owned(), escapes)
                })
                .collect();
            return Self { ink, roles, marks };
        };
        let roles = ROLES
            .iter()
            .filter_map(|role| {
                let colour = channels(theme["colours"][*role].as_str()?)?;
                let [red, green, blue] = colour;
                let sixteen = slot(&theme["terminal"], colour).unwrap_or_default();
                let escapes = Escapes {
                    // The text is the terminal's own foreground, drawn as
                    // it is wherever the sixteen hold it.
                    truecolour: if *role == "text" && sixteen.is_empty() {
                        String::new()
                    } else {
                        format!("38;2;{red};{green};{blue}")
                    },
                    sixteen,
                };
                Some(((*role).to_owned(), escapes))
            })
            .collect();
        let text = |mark: &Value, key: &str| mark[key].as_str().map(str::to_owned);
        for (name, mark) in theme["marks"].as_object().into_iter().flatten() {
            if let (Some(glyph), Some(console)) = (text(mark, "glyph"), text(mark, "console")) {
                marks.insert(name.clone(), Mark { glyph, console });
            }
        }
        Self { ink, roles, marks }
    }

    /// A pen with no ink and the built-in marks, for a pipe or a test.
    pub fn plain() -> Self {
        Self::new(Ink::plain(), None)
    }

    /// The ink this pen draws with.
    pub fn ink(&self) -> Ink {
        self.ink
    }

    /// Whether anything it draws is inked.
    pub fn inks(&self) -> bool {
        self.ink.colours != Colours::None
    }

    fn escaped(&self, codes: &[&str], words: &str) -> String {
        let codes: Vec<&str> = codes
            .iter()
            .copied()
            .filter(|code| !code.is_empty())
            .collect();
        if words.is_empty() || !self.inks() || codes.is_empty() {
            return words.to_owned();
        }
        format!("\u{1b}[{}m{words}\u{1b}[0m", codes.join(";"))
    }

    fn colour(&self, role: &str) -> &str {
        self.roles
            .get(role)
            .map_or("", |escapes| match self.ink.colours {
                Colours::Truecolour => escapes.truecolour.as_str(),
                _ => escapes.sixteen.as_str(),
            })
    }

    /// Words in a role's colour: text, strong or muted for words, the
    /// rest for marks.
    pub fn role(&self, role: &str, words: &str) -> String {
        self.escaped(&[self.colour(role)], words)
    }

    /// A title: bold, in strong, the only words a command sets bold.
    pub fn title(&self, words: &str) -> String {
        self.escaped(&["1", self.colour("strong")], words)
    }

    /// A label over a group or a column: muted capitals.
    pub fn label(&self, words: &str) -> String {
        self.role("muted", &words.to_uppercase())
    }

    /// What the command says of itself: italic, muted.
    pub fn voice(&self, words: &str) -> String {
        self.escaped(&["3", self.colour("muted")], words)
    }

    /// Words drawn faint in a role, as a meter's track is.
    pub fn faint(&self, role: &str, words: &str) -> String {
        self.escaped(&["2", self.colour(role)], words)
    }

    /// A mark by what it means, as this terminal draws it: its stand-in on
    /// the console, and nothing for a mark the theme doesn't hold.
    pub fn mark(&self, name: &str) -> &str {
        self.marks.get(name).map_or("", |mark| {
            if self.ink.console {
                mark.console.as_str()
            } else {
                mark.glyph.as_str()
            }
        })
    }

    /// A mark in a role's colour, and the space after it, which kitty
    /// needs to draw a wide mark whole and which starts the text
    /// after it on one column (docs/commands.md, A mark and its space).
    pub fn marked(&self, name: &str, role: &str) -> String {
        let mark = self.mark(name);
        if mark.is_empty() {
            return " ".repeat(crate::layout::MARK);
        }
        format!("{} ", self.role(role, mark))
    }

    /// Words that open a URL or a path on a click: a kitty hyperlink
    /// where there's ink, and the words alone where there's none.
    pub fn link(&self, target: &str, words: &str) -> String {
        if !self.inks() {
            return words.to_owned();
        }
        format!("\u{1b}]8;;{target}\u{1b}\\{words}\u{1b}]8;;\u{1b}\\")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn osaka_jade() -> Value {
        serde_json::from_str(include_str!("../tests/osaka-jade.json")).expect("the theme parses")
    }

    fn decided(terminal: bool, asked: Colour, vars: &[(&str, &str)]) -> Colours {
        Ink::decide(terminal, asked, &Environment::of([false; 3], vars)).colours
    }

    #[test]
    fn a_terminal_is_inked_and_a_pipe_isnt() {
        assert_eq!(decided(true, Colour::Auto, &[]), Colours::Sixteen);
        assert_eq!(decided(false, Colour::Auto, &[]), Colours::None);
        assert_eq!(
            decided(true, Colour::Auto, &[("TERM", "dumb")]),
            Colours::None
        );
    }

    #[test]
    fn the_flag_wins_then_no_color_then_force_color() {
        let both = [("NO_COLOR", "1"), ("FORCE_COLOR", "1")];
        assert_eq!(decided(false, Colour::Always, &both), Colours::Sixteen);
        assert_eq!(decided(true, Colour::Never, &[]), Colours::None);
        assert_eq!(decided(true, Colour::Auto, &both), Colours::None);
        assert_eq!(
            decided(false, Colour::Auto, &[("FORCE_COLOR", "1")]),
            Colours::Sixteen
        );
        assert_eq!(
            decided(false, Colour::Auto, &[("FORCE_COLOR", "")]),
            Colours::None
        );
    }

    #[test]
    fn truecolour_only_where_colorterm_says_so() {
        let true_colour = [("COLORTERM", "truecolor")];
        assert_eq!(
            decided(true, Colour::Auto, &true_colour),
            Colours::Truecolour
        );
        assert_eq!(
            decided(true, Colour::Auto, &[("COLORTERM", "24bit")]),
            Colours::Truecolour
        );
    }

    #[test]
    fn each_role_takes_the_terminal_colour_holding_its_pigment() {
        let pen = Pen::new(Ink::sixteen(), Some(&osaka_jade()));
        // Osaka Jade's warning is gold, in bright yellow, and its yellow is
        // a green; its alert is vermilion nudged to read, near its red.
        assert_eq!(pen.role("warning", "w"), "\u{1b}[93mw\u{1b}[0m");
        assert_eq!(pen.role("alert", "a"), "\u{1b}[31ma\u{1b}[0m");
        assert_eq!(pen.title("T"), "\u{1b}[1;97mT\u{1b}[0m");
        // Its text is the terminal's own foreground, so it's left as it is.
        assert_eq!(pen.role("text", "t"), "t");
    }

    #[test]
    fn truecolour_draws_the_pigment_itself() {
        let ink = Ink {
            colours: Colours::Truecolour,
            console: false,
        };
        let pen = Pen::new(ink, Some(&osaka_jade()));
        assert_eq!(
            pen.role("strong", "s"),
            "\u{1b}[38;2;247;232;178ms\u{1b}[0m"
        );
        assert_eq!(pen.role("text", "t"), "t");
    }

    #[test]
    fn a_pen_with_no_ink_draws_plain_words_and_its_marks() {
        let pen = Pen::new(Ink::plain(), Some(&osaka_jade()));
        assert_eq!(pen.title("Disk"), "Disk");
        assert_eq!(pen.label("Examples"), "EXAMPLES");
        assert_eq!(pen.marked("done", "success"), "\u{2713} ");
        assert_eq!(pen.link("https://example.com", "page"), "page");
    }

    #[test]
    fn the_console_draws_each_marks_stand_in() {
        let console = Ink {
            colours: Colours::Sixteen,
            console: true,
        };
        assert_eq!(Pen::new(console, Some(&osaka_jade())).mark("failed"), "x");
        assert_eq!(Pen::new(console, None).mark("caution"), "!");
        assert!(
            Ink::decide(
                true,
                Colour::Auto,
                &Environment::of([false; 3], &[("TERM", "linux")])
            )
            .console,
            "the Linux console draws the stand-ins"
        );
    }

    #[test]
    fn a_mark_the_theme_lacks_keeps_its_cells() {
        assert_eq!(Pen::plain().marked("nothing-by-this-name", "muted"), "  ");
    }

    #[test]
    fn with_no_theme_the_agreed_colours_ink_the_states() {
        let pen = Pen::new(Ink::sixteen(), None);
        assert_eq!(pen.role("alert", "a"), "\u{1b}[31ma\u{1b}[0m");
        assert_eq!(pen.title("T"), "\u{1b}[1mT\u{1b}[0m");
        assert_eq!(pen.voice("v"), "\u{1b}[3;90mv\u{1b}[0m");
    }

    #[test]
    fn the_theme_is_found_under_the_config_home() {
        let environment = Environment::of([false; 3], &[("HOME", "/home/someone")]);
        assert_eq!(
            theme_file(&environment),
            Some(PathBuf::from("/home/someone/.config/jitsusama/theme.json"))
        );
        let environment = Environment::of([false; 3], &[("XDG_CONFIG_HOME", "/c"), ("HOME", "/h")]);
        assert_eq!(
            theme_file(&environment),
            Some(PathBuf::from("/c/jitsusama/theme.json"))
        );
    }
}
