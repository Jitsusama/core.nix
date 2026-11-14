//! The clipboard a fenced pi command reaches.
//!
//! The agent harness runs an agent's shell commands in a fence that
//! keeps out the compositor's socket, since any client on it can record
//! the screen, type and move the pointer into any window, and read every
//! copy as it's made. So wl-copy and wl-paste fail inside. This stands
//! between: a door outside the fence (src/main.rs) holds the desktop's
//! clipboard through the compositor's data-control protocol
//! (src/desk.rs), which sets and reads it without a window, the keys or
//! the focus, and the fence binds in only the door's socket. A fenced
//! command's wl-copy and wl-paste are one shim (src/shim.rs) speaking a
//! line apiece to the door (src/wire.rs).
//!
//! The door answers a client only when the kernel says it runs in a
//! fence of this account (src/peer.rs), and tells the session it runs
//! for from what pi set outside the fence, which nothing inside can
//! change. It never reads a copy a password manager marked secret
//! (src/kinds.rs), and it says each copy, paste, listing, clearing and
//! refusal in the journal in one line of a fixed shape, with the
//! session, never what was copied (src/said.rs).

pub mod desk;
pub mod door;
pub mod kinds;
pub mod peer;
pub mod said;
pub mod shim;
pub mod wire;
