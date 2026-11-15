//! How the launcher's words are inked in a terminal: by the kit's pen for
//! stdout, which reads the account's theme and decides the ink as every
//! command does. The shell names a row's mark by its glyph rather than by
//! what it means, so on the console (`TERM=linux`) the glyph is swapped for
//! the stand-in the same theme gives it (../../theme/glyphs.nix's
//! `console`), which the console's font can draw.

use std::collections::HashMap;

use serde_json::Value;
use terminal_kit::ink::Pen;

/// The pen the launcher draws with, and each mark's console stand-in, by
/// its glyph.
#[derive(Debug, Clone)]
pub struct Look {
    pen: Pen,
    stand_ins: HashMap<String, String>,
}

impl Look {
    /// A look drawn with `pen`, its stand-ins from `theme`'s marks.
    pub fn new(pen: Pen, theme: Option<&Value>) -> Self {
        let text = |mark: &Value, key: &str| mark[key].as_str().map(str::to_owned);
        let stand_ins = theme
            .and_then(|theme| theme["marks"].as_object())
            .into_iter()
            .flatten()
            .filter_map(|(_, mark)| Some((text(mark, "glyph")?, text(mark, "console")?)))
            .collect();
        Self { pen, stand_ins }
    }

    /// Plain words and the marks as they're given, for a pipe or a test.
    #[cfg(test)]
    pub fn plain() -> Self {
        Self::new(Pen::plain(), None)
    }

    /// Words in a role's colour, or plain where nothing is inked.
    pub fn ink(&self, role: &str, words: &str) -> String {
        self.pen.role(role, words)
    }

    /// A title: bold, in strong, the only words set bold.
    pub fn title(&self, words: &str) -> String {
        self.pen.title(words)
    }

    /// A mark as this terminal draws it: its stand-in on the console, and
    /// nothing there for a glyph the theme gives none.
    pub fn mark(&self, glyph: &str) -> String {
        if self.pen.ink().console {
            self.stand_ins.get(glyph).cloned().unwrap_or_default()
        } else {
            glyph.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terminal_kit::ink::{Colours, Ink};

    /// The one theme the launcher's tests name colours in, kept in a file
    /// of its own since its colours are what's tested, not a look.
    fn theme() -> Value {
        serde_json::from_str(include_str!("../tests/theme.json")).expect("the theme parses")
    }

    fn look(ink: Ink) -> Look {
        Look::new(Pen::new(ink, Some(&theme())), Some(&theme()))
    }

    #[test]
    fn the_console_draws_its_stand_ins() {
        let console = Ink {
            colours: Colours::Sixteen,
            console: true,
        };
        assert_eq!(
            look(console).mark("\u{25c6}"),
            "*",
            "the console's stand-in"
        );
        assert_eq!(
            look(console).mark("\u{2192}"),
            "",
            "a glyph with no stand-in reached the console"
        );
        assert_eq!(
            look(Ink::sixteen()).mark("\u{25c6}"),
            "\u{25c6}",
            "a terminal's mark isn't its glyph"
        );
    }

    #[test]
    fn a_role_and_a_title_are_drawn_by_the_kit() {
        assert_eq!(
            look(Ink::sixteen()).ink("strong", "Disk"),
            "\u{1b}[97mDisk\u{1b}[0m",
            "strong isn't the slot holding its pigment"
        );
        assert_eq!(
            look(Ink::sixteen()).title("Disk"),
            "\u{1b}[1;97mDisk\u{1b}[0m",
            "a title isn't bold strong"
        );
        assert_eq!(
            look(Ink::plain()).ink("strong", "Disk"),
            "Disk",
            "a pipe is inked"
        );
        assert_eq!(
            look(Ink::plain()).title("Disk"),
            "Disk",
            "a pipe's title is inked"
        );
    }
}
