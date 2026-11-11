//! The sound daemon's own logic, apart from its socket and PipeWire's
//! loop so its tests reach it: what a client asks ([`interface`]), the
//! devices as the server says them and the records drawn from them
//! ([`devices`]), and how loud a node is set, read from its Props
//! ([`levels`]).
//!
//! A record names every figure the shell draws a device's row from, as
//! PipeWire says it: words are the shell's.

pub mod devices;
pub mod interface;
pub mod levels;
