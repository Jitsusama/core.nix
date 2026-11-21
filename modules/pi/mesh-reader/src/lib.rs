//! pi-mesh-reader: one reader of the mesh's folders for every pi process.
//!
//! Since 20 November every pi conversation runs in a process of its own,
//! and each kept its own mirror of the mesh's presence folder: on 21
//! November seventy idle conversations listed its 163 files every second,
//! and the directory and lock waits that followed were 82% of Joel's
//! stalls of two seconds or more. Reading only what each watch names cut
//! the listings by 84%, but every process still reads every change for
//! itself.
//!
//! This reads each folder once for all of them. A pi process connects to
//! its socket and names a folder and the suffix of the files it wants
//! ([`wire`]). The reader answers with every such file, a line each, then
//! `ready`, then a line for each file put or gone, as long as the client
//! stays. However many ask, a folder has one watch and one read per
//! change ([`reader`]), and serving them touches no disk. It lists the
//! folder again only as it starts, when its watch says it missed changes
//! or lost the folder, and every half minute in case the watch missed one
//! ([`watch`]).
//!
//! It only reads: what a pi process publishes it still writes to the
//! folder itself, and the reader hears it there. A client too slow to
//! keep up is let go rather than held for, and starts again with a fresh
//! listing. A folder is served only under a root the service was given.
//! When the reader isn't there, a pi process reads the folder for itself,
//! slowly, as before.

pub mod reader;
pub mod watch;
pub mod wire;
