//! The theme's marks the views draw, looked up by meaning: the ones the
//! terminal kit's pen draws when a view starts, the theme's own or their
//! stand-ins on the console, else the defaults the theme module carries
//! (modules/theme/glyphs.nix), which the golden pages are set with.

use std::sync::OnceLock;

use terminal_kit::ink::Pen;

#[derive(Debug, Clone)]
pub struct Marks {
    /// A gap between two fields on one line.
    pub separator: String,
    /// Failed, an error: a node offline, eero unreachable.
    pub failed: String,
    /// A state worth seeing: an update waiting, a reading grown old.
    pub on: String,
}

impl Default for Marks {
    fn default() -> Self {
        Self {
            separator: "·".into(),
            failed: "✕".into(),
            on: "●".into(),
        }
    }
}

impl Marks {
    /// The marks as `pen` draws them, keeping a default for any it lacks.
    fn drawn_by(pen: &Pen) -> Self {
        let defaults = Self::default();
        let mark = |name: &str, fallback: String| {
            let drawn = pen.mark(name);
            if drawn.is_empty() {
                fallback
            } else {
                drawn.to_owned()
            }
        };
        Self {
            separator: mark("separator", defaults.separator),
            failed: mark("failed", defaults.failed),
            on: mark("on", defaults.on),
        }
    }
}

static MARKS: OnceLock<Marks> = OnceLock::new();

/// Takes the marks `pen` draws, for every page after.
pub fn take(pen: &Pen) {
    _ = MARKS.set(Marks::drawn_by(pen));
}

pub fn marks() -> &'static Marks {
    MARKS.get_or_init(Marks::default)
}

#[cfg(test)]
mod tests {
    use terminal_kit::ink::{Colours, Ink, Pen};

    use super::Marks;

    #[test]
    fn the_console_draws_each_marks_stand_in() {
        let console = Ink {
            colours: Colours::None,
            console: true,
        };
        let marks = Marks::drawn_by(&Pen::new(console, None));
        assert_eq!(
            [marks.separator, marks.failed, marks.on],
            [".", "x", "*"],
            "the console was given marks its font can't draw"
        );
    }

    #[test]
    fn a_terminal_draws_each_marks_glyph() {
        // Marks unlike the defaults, so a pen whose marks were ignored
        // would read differently.
        let mark = |glyph: &str| serde_json::json!({"glyph": glyph, "console": "?"});
        let theme = serde_json::json!({"marks": {
            "separator": mark("|"), "failed": mark("!"), "on": mark("+"),
        }});
        let marks = Marks::drawn_by(&Pen::new(Ink::plain(), Some(&theme)));
        assert_eq!(
            [marks.separator, marks.failed, marks.on],
            ["|", "!", "+"],
            "a terminal wasn't given the theme's own glyphs"
        );
    }

    #[test]
    fn a_mark_the_pen_lacks_keeps_its_default() {
        let marks = Marks::drawn_by(&Pen::plain());
        let defaults = Marks::default();
        assert_eq!(
            [marks.separator, marks.failed, marks.on],
            [defaults.separator, defaults.failed, defaults.on],
            "a plain pen's marks aren't the defaults"
        );
    }
}
