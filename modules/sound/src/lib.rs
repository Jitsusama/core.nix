//! The sound daemon's own logic, apart from its socket and PipeWire's
//! loop so its tests reach it: what a client asks ([`interface`]), the
//! devices as the server says them and the records drawn from them
//! ([`devices`]), how loud a node is set, read from its Props and written
//! into an act's ([`levels`]), and how each act is done ([`acts`]).
//!
//! A record names every figure the shell draws a device's row from, as
//! PipeWire says it: words are the shell's.

pub mod acts;
pub mod devices;
pub mod interface;
pub mod levels;
