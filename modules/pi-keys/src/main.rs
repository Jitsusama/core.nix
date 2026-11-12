//! Samples, a few times a day, what a key typed into the pi window Joel is
//! at waits on.
//!
//! Typing in pi felt sluggish while the machine sat mostly idle, and the
//! cause was found only by measuring once, by hand, for hours. pi-keys
//! keeps the measure going cheaply: run by a timer, it waits for a pi
//! window to hold the focus, then for ten minutes at pi windows reads the
//! three threads a key passes every 2 ms, without privilege: niri's main
//! thread, the kitty drawing the window and pi's main thread. From their
//! bursts it models what a key waited (src/model.rs), and appends one line
//! to $XDG_STATE_HOME/pi-keys/<UTC day>.jsonl, for cadence's daily read.
//!
//! Usage: pi-keys sample [--minutes N] [--wait N] [--call-check PROGRAM]

mod burst;
mod model;
mod path;

fn main() {}
