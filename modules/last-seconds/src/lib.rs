//! last-seconds: what the machine was doing in the seconds before each boot
//! ended, kept where a cut can't take it.
//!
//! When the XPS 14 resets or loses power, the journal's last minutes are
//! lost with it: they wait in memory to be written, and nothing of them is
//! on the disk. So `last-seconds-keep` (src/bin/last-seconds-keep.rs),
//! the recorder, which its unit runs from boot to power off, reads once a
//! second what the processor and the platform draw, the package's heat,
//! the fans, the load, how long tasks waited on the processor, memory and
//! the disk, the memory left, the battery and the charger, and how long the
//! processors rested in each state, and writes it to a ring of ten minutes
//! on the disk (src/ring.rs, which says what it costs the disk).
//! `last-seconds` (src/main.rs) reads the rings back as the account that
//! asks: how each boot's record ended, stopped, asleep or cut short, and
//! its last seconds.
//!
//! What it records is a second's average, the same as the shell's own
//! readings; the energy counters are kept from other accounts because a
//! reading milliseconds apart could tell what a processor was computing,
//! which a second's average can't.

pub mod record;
pub mod ring;
pub mod sample;
/// The tests' folders, the crate's and the recorder's run whole alike.
pub mod scratch;

/// Where the rings are kept, one folder a boot.
pub const STATE: &str = "/var/lib/last-seconds";
/// The variable that names another folder for them, for the tests.
pub const STATE_VARIABLE: &str = "LAST_SECONDS_STATE";
/// What a boot's ring is called in its folder.
pub const RING: &str = "ring";
