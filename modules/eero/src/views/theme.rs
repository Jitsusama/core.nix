//! The ink a terminal is given: the terminal kit's pen, which reads the
//! account's theme for each role's colour as every command's does, and
//! the theme's marks, which the views draw as text whatever the ink.

use std::path::{Path, PathBuf};

use serde_json::Value;
use terminal_kit::ink::Pen;

use super::text::Ink;

fn theme_file() -> Option<PathBuf> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".config")))?;
    Some(config.join("jitsusama/theme.json"))
}

/// The ink for standard output, drawn with `pen`: whether it's coloured
/// is the terminal kit's to decide, from `--color`, `NO_COLOR`,
/// `FORCE_COLOR` and whether it's a terminal. The theme's marks are taken
/// either way, since they're the text's own.
pub fn ink(pen: &Pen) -> Ink {
    let theme = theme_file()
        .and_then(|file| std::fs::read_to_string(file).ok())
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    if let Some(theme) = &theme {
        super::marks::take(theme);
    }
    Ink::new(pen.clone())
}
