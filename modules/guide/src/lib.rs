//! The guide walks a person through a machine's journey one step at a
//! time: from a blank laptop to a sealed, enrolled machine, and through its
//! repairs. Every step observes the machine, says what it needs before it
//! starts, acts, and is only ever done because observing again says so, so
//! the guide picks up where it left off after any reboot.

/// What a tool printed on the Dell XPS 14 (`fixtures/`).
#[cfg(test)]
macro_rules! xps {
    ($file:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/dell-xps-14-da14260/",
            $file
        ))
    };
}

/// What a tool printed on the Surface Laptop 4 (`fixtures/`).
#[cfg(test)]
macro_rules! surface {
    ($file:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/microsoft-surface-laptop-4-1958/",
            $file
        ))
    };
}

/// What a tool printed in a NixOS test's QEMU with canokey keys
/// (`fixtures/`).
#[cfg(test)]
macro_rules! qemu {
    ($file:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/qemu-canokey/",
            $file
        ))
    };
}

/// A unit test's scratch directory, made by the test and gone when this
/// drops, so a failing test leaves nothing behind either.
#[cfg(test)]
pub(crate) struct Scratch(std::path::PathBuf);

#[cfg(test)]
impl Scratch {
    pub(crate) fn new(name: &str) -> Scratch {
        let directory = std::env::temp_dir().join(format!("guide-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        Scratch(directory)
    }
}

#[cfg(test)]
impl std::ops::Deref for Scratch {
    type Target = std::path::Path;

    fn deref(&self) -> &std::path::Path {
        &self.0
    }
}

#[cfg(test)]
impl AsRef<std::path::Path> for Scratch {
    fn as_ref(&self) -> &std::path::Path {
        &self.0
    }
}

#[cfg(test)]
impl From<&Scratch> for std::path::PathBuf {
    fn from(scratch: &Scratch) -> std::path::PathBuf {
        scratch.0.clone()
    }
}

#[cfg(test)]
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The fixtures' directories, for a test that reads files by name.
#[cfg(test)]
mod fixtures {
    pub const XPS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/dell-xps-14-da14260");
    pub const SURFACE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/microsoft-surface-laptop-4-1958"
    );
}

pub mod fake;
pub mod fault;
pub mod host;
pub mod install;
pub mod installed;
pub mod journal;
pub mod journey;
pub mod machine;
pub mod observe;
pub mod place;
pub mod progress;
pub mod recover;
pub mod rescue;
pub mod screen;
pub mod sign;
pub mod step;
pub mod stick;
pub mod system;
pub mod tool;
pub mod vault;
pub mod words;

pub use fault::Fault;
pub use journal::Journal;
pub use journey::{Journey, Next};
pub use machine::{Given, Machine, Program, Ran};
pub use place::Place;
pub use step::{Answer, Answers, Ask, Secret, Standing, Step};
pub use tool::Tool;
