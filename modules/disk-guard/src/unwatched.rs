//! Watching who opens a build's artifacts, where nothing can: macOS hears
//! opens only through Endpoint Security, which takes an entitlement from
//! Apple. No watch is ever made, so no older build is dated by its opens,
//! and one nothing dates is never offered.

use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// What a watch saw.
#[derive(Clone, Debug, PartialEq)]
pub enum Seen {
    Opened(PathBuf),
    /// Events were lost, and anything watched may have been opened.
    Lost,
}

/// A watch there can't be.
pub struct Watch {
    never: std::convert::Infallible,
}

impl Watch {
    pub fn new() -> io::Result<Watch> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "nothing here hears who opens a file",
        ))
    }

    pub fn watch(&self, _dir: &Path) -> io::Result<()> {
        match self.never {}
    }

    pub fn wait(&self, _within: Duration) -> io::Result<Vec<Seen>> {
        match self.never {}
    }
}
