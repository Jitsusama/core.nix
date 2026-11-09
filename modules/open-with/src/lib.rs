//! What open-with's programs share: the desktop's list of files opened
//! lately (recent.rs), which the portal (main.rs) and recent-files
//! (bin/recent-files.rs) both add to, what a program has open now
//! (open.rs), which open-now (bin/open-now.rs) says, and asking a running
//! Neovim what it holds over its own server (neovim.rs).

pub mod neovim;
pub mod open;
pub mod recent;
