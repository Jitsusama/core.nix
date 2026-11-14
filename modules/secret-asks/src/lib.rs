//! The release line's first experiment: an ask for a secret, put up as a
//! file, drawn by the launcher as a row in Asks, and released on Joel's
//! yes into one fixed checker (daemon.rs says what it holds and what it
//! doesn't yet).
//!
//! - `ask`: an ask as its file says it, checked.
//! - `tree`: the tree an ask runs in, as git says it, asked safely.
//! - `row`: an ask as the launcher draws it, and the digest of it.
//! - `mask`: a command's output with the value masked.
//! - `words`: Asks' words, from the launcher's vocabulary.
//! - `reading`: how each ask ended, told to the journal as fields alone.
//! - `engine`: the folder watched, the asks held as rows, and the yes,
//!   whatever a daemon may release.
//! - `daemon`: the experiment's engine, served over its own socket.

pub mod ask;
pub mod daemon;
pub mod engine;
pub mod mask;
pub mod reading;
pub mod row;
pub mod tree;
pub mod words;
