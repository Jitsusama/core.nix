//! The sound daemon's own logic, apart from its socket and PipeWire's
//! loop so its tests reach it: what a client asks ([`interface`]), the
//! devices as the server says them and the records drawn from them
//! ([`devices`]), how loud a node is set, read from its Props and written
//! into an act's ([`levels`]), how each act is done ([`acts`]), and
//! Sound's rows, served ready for the launcher ([`rows`]), each device's
//! parts inside it ([`parts`]).
//!
//! A device record names every figure as PipeWire says it, never a
//! sentence; a row is worded, from the launcher's own vocabulary.

pub mod acts;
pub mod devices;
pub mod interface;
pub mod levels;
pub mod parts;
pub mod rows;
