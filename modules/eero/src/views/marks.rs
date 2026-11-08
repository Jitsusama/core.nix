//! The theme's marks the views draw, looked up by meaning: the account's
//! own from its theme.json when a view starts, else the defaults the theme
//! module carries (modules/theme/glyphs.nix), which the golden pages are
//! set with.

use std::sync::OnceLock;

use serde_json::Value;

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

static MARKS: OnceLock<Marks> = OnceLock::new();

/// Takes the theme's marks, where it names them, for every page after.
pub fn take(theme: &Value) {
    let defaults = Marks::default();
    let mark = |name: &str, fallback: String| {
        theme["marks"][name]["glyph"]
            .as_str()
            .map_or(fallback, str::to_owned)
    };
    _ = MARKS.set(Marks {
        separator: mark("separator", defaults.separator),
        failed: mark("failed", defaults.failed),
        on: mark("on", defaults.on),
    });
}

pub fn marks() -> &'static Marks {
    MARKS.get_or_init(Marks::default)
}
