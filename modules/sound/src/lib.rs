//! The sound daemon's own logic, apart from its socket and PipeWire's
//! loop so its tests reach it: what a client asks ([`interface`]), the
//! devices as the server says them and the records drawn from them
//! ([`devices`]), programs' streams and the links joining them to devices
//! ([`streams`]), how loud a node is set, read from its Props and written
//! into an act's ([`levels`]), how each act is done ([`acts`]), and
//! Sound's rows, served ready for the launcher ([`rows`]), each device's
//! parts inside it ([`parts`]), its programs among them ([`programs`]),
//! each named by its desktop entry ([`naming`]) and each program's drawer
//! served beside its row ([`drawer`]), and how each running
//! node keeps time, as PipeWire's profiler says it ([`timing`]).
//!
//! A device record names every figure as PipeWire says it, never a
//! sentence; a row is worded, from the launcher's own vocabulary.

pub mod acts;
pub mod devices;
pub mod drawer;
pub mod interface;
pub mod levels;
pub mod naming;
pub mod parts;
pub mod programs;
pub mod rows;
pub mod streams;
pub mod timing;
