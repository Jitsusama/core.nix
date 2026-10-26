//! Where the guide runs, which decides what it can see and do. A journey
//! moves through the places in this order, and never back.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Place {
    /// The machine a stick is made on, which may be the one it's for.
    Writer,
    /// The stick, booted on the machine it installs or repairs.
    Stick,
    /// The machine as installed, from its first boot on.
    Installed,
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Place::Writer => "the machine making the stick",
            Place::Stick => "the stick",
            Place::Installed => "the installed machine",
        })
    }
}
