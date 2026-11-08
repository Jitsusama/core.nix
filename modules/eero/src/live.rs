//! Reading the record the daemon keeps, for a view: it's there, or the
//! daemon isn't running, or it's not a record this eero reads.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::model::HomeNetwork;
use crate::views::{self, Clock, Ink, View};

/// Where the daemon keeps the record.
pub fn file() -> Result<PathBuf, String> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(|run| PathBuf::from(run).join("eero/home-network.json"))
        .ok_or_else(|| "XDG_RUNTIME_DIR isn't set".into())
}

/// The record, or why there's none to show.
pub fn read(file: &Path) -> Result<HomeNetwork, String> {
    let text = fs::read_to_string(file).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => format!(
            "nothing read yet: {} isn't there, so eero-daemon isn't running; \
             start it with systemctl --user start eero-daemon",
            file.display()
        ),
        _ => format!("can't read {}: {error}", file.display()),
    })?;
    serde_json::from_str(&text)
        .map_err(|error| format!("{} isn't a record this eero reads: {error}", file.display()))
}

/// A view of the record, as text or as JSON.
pub fn show(view: &View, json: bool, ink: &Ink) -> Result<String, String> {
    let model = read(&file()?)?;
    if json {
        views::json(view, &model)
    } else {
        views::text(view, &model, &Clock::system(), ink)
    }
}
