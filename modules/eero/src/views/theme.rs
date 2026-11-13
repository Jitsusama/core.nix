//! The ink a terminal is given, from the account's theme: each role the
//! views draw in, as the one of the terminal's sixteen colours holding its
//! pigment or near it, the way the guide and the console read a theme
//! (modules/theme/nearest.nix). The terminal's colours carry no meaning of
//! their own, and a theme may put any pigment in any of them (Osaka Jade's
//! yellow is a green), so a role is found by its colour, never assumed to be
//! the slot of its name.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::text::{Ink, Palette};

/// Far enough to tell two hues apart, near enough to hold one the theme
/// nudged from a terminal colour, as nearest.nix holds it.
const NEAR: u32 = 48;

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

/// The SGR colour of the terminal slot nearest a role's pigment, or none
/// when the nearest is the terminal's own foreground, or none is near.
fn slot(theme: &Value, role: &str) -> Option<String> {
    let colour = channels(theme["colours"][role].as_str()?)?;
    let terminal = &theme["terminal"];
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

/// What a terminal draws each role in under a theme.
pub fn palette(theme: &Value) -> Palette {
    let with = |first: &str, rest: Option<String>| {
        Some(rest.map_or_else(|| first.to_owned(), |rest| format!("{first};{rest}")))
    };
    Palette {
        text: slot(theme, "text"),
        // Strong is bold as well as its colour, so it ranks in type when a
        // theme gives it the text's own.
        strong: with("1", slot(theme, "strong")),
        muted: slot(theme, "muted"),
        // The rule's colour, faint, as the design lays a graph's track at
        // half the edge colour's opacity.
        track: with("2", slot(theme, "rule")),
        alert: slot(theme, "alert"),
        warning: slot(theme, "warning"),
    }
}

/// The ink for a terminal with no theme of the account's to read: the
/// colours terminals agree on by name.
pub fn fallback() -> Palette {
    let sgr = |code: &str| Some(code.to_owned());
    Palette {
        text: None,
        strong: sgr("1"),
        muted: sgr("90"),
        track: sgr("2"),
        alert: sgr("31"),
        warning: sgr("33"),
    }
}

fn theme_file() -> Option<PathBuf> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".config")))?;
    Some(config.join("jitsusama/theme.json"))
}

/// The ink for standard output: the theme's when it's coloured, and plain
/// text otherwise. Whether it's coloured is the terminal kit's to decide,
/// from `--color`, `NO_COLOR`, `FORCE_COLOR` and whether it's a terminal,
/// as every command's is. The theme's marks are taken either way, since
/// they're the text's own.
pub fn ink(coloured: bool) -> Ink {
    let theme = theme_file()
        .and_then(|file| std::fs::read_to_string(file).ok())
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    if let Some(theme) = &theme {
        super::marks::take(theme);
    }
    if !coloured {
        return Ink::plain();
    }
    Ink {
        palette: Some(theme.as_ref().map_or_else(fallback, palette)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Osaka Jade's roles and terminal colours, as its theme.json holds
    /// them. They live in a file of their own, the one place in eero that
    /// names a colour, since they're what's tested on rather than a look.
    fn osaka_jade() -> Value {
        serde_json::from_str(include_str!("../../tests/osaka-jade.json")).unwrap()
    }

    #[test]
    fn each_role_takes_the_terminal_colour_holding_its_pigment() {
        let palette = palette(&osaka_jade());
        // Its warning is gold, in bright yellow; its yellow is a green.
        assert_eq!(palette.warning.as_deref(), Some("93"));
        // Its alert, ember, is vermilion nudged to read, and its red is
        // vermilion.
        assert_eq!(palette.alert.as_deref(), Some("31"));
        assert_eq!(palette.strong.as_deref(), Some("1;97"));
        assert_eq!(palette.muted.as_deref(), Some("37"));
    }

    #[test]
    fn the_text_is_the_terminals_own_foreground() {
        let palette = palette(&osaka_jade());
        assert_eq!(palette.text, None);
        // Its rule, a dark green, lies nearest the yellow slot, which holds
        // a green too, and the track draws it faint.
        assert_eq!(palette.track.as_deref(), Some("2;33"));
    }
}
