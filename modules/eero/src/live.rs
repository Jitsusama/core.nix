//! Reading the record the daemon keeps, for a view: it's there, or the
//! daemon isn't running, or it's not a record this eero reads.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use terminal_kit::{Exit, Failure, Room};

use crate::model::HomeNetwork;
use crate::views::{self, Clock, Ink, View};

/// Where the daemon keeps the record.
pub fn file() -> Result<PathBuf, Failure> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(|run| PathBuf::from(run).join("eero/home-network.json"))
        .ok_or_else(|| {
            Failure::new(Exit::Config, "there's nowhere to find the record")
                .because("XDG_RUNTIME_DIR isn't set")
        })
}

/// The record, or why there's none to show: the daemon isn't keeping one,
/// which starting it mends, or what's there isn't a record this eero reads.
pub fn read(file: &Path) -> Result<HomeNetwork, Failure> {
    let text = fs::read_to_string(file).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => Failure::new(Exit::Unavailable, "nothing read yet")
            .because(format!(
                "{} isn't there, so eero-daemon isn't running",
                file.display()
            ))
            .try_this("systemctl --user start eero-daemon"),
        _ => Failure::new(Exit::Failed, format!("can't read {}", file.display()))
            .because(error.to_string()),
    })?;
    serde_json::from_str(&text).map_err(|error| {
        Failure::new(
            Exit::BadInput,
            format!("{} isn't a record this eero reads", file.display()),
        )
        .because(error.to_string())
    })
}

/// A view of the record, as text or as JSON. A view of one thing fails as
/// not found when the record holds no such thing, which is the only way a
/// view of a record that reads can fail, and says what it does hold. The
/// text is laid out in `room`: whole into a pipe, fitted to a terminal.
pub fn show(view: &View, json: bool, ink: &Ink, room: Room) -> Result<String, Failure> {
    let model = read(&file()?)?;
    let page = if json {
        views::json(view, &model)
    } else {
        views::text(view, &model, &Clock::system(), ink, room)
    };
    page.map_err(|why| Failure::new(Exit::NotFound, why))
}
