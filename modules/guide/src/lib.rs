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
pub mod journal;
pub mod journey;
pub mod machine;
pub mod observe;
pub mod place;
pub mod screen;
pub mod sign;
pub mod step;
pub mod stick;
pub mod system;
pub mod tool;
pub mod vault;

pub use fault::Fault;
pub use journal::Journal;
pub use journey::{Journey, Next};
pub use machine::{Given, Machine, Program, Ran};
pub use place::Place;
pub use step::{Answer, Answers, Ask, Secret, Standing, Step};
pub use tool::Tool;
