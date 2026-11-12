//! The programs daemon's own logic, apart from its socket and threads so
//! its tests reach it: what a client asks ([`interface`]), where the
//! desktop entries are ([`xdg`]), what one says ([`entry`]), the file
//! kinds it opens ([`mime`]), the package its command comes from
//! ([`package`]), each program's lists of what it had open ([`source`]),
//! the commands that read them ([`command`]), a folder asked for by its
//! path ([`folder`]), the git repositories under one and their files
//! ([`repositories`]), what a file shows of itself ([`preview`]), when
//! zoxide saw each folder ([`visited`]), the machines ssh knows and how
//! often it reached them ([`machines`]), the socket it answers on
//! ([`listen`]), and inotify, which says when any of them changes ([`notify`]).
//!
//! A record names every figure its source has, as the source says it,
//! with its unit in its key (`_s` seconds since 1970, `_ms`
//! milliseconds), and none it doesn't have: what a source doesn't say is
//! null, never a guess. Words are the shell's.

pub mod command;
pub mod entry;
pub mod folder;
pub mod interface;
pub mod listen;
pub mod machines;
pub mod mime;
pub mod notify;
pub mod package;
pub mod preview;
pub mod program_rows;
pub mod repositories;
pub mod source;
pub mod tab_rows;
pub mod taken_rows;
pub mod visited;
pub mod xdg;

/// Milliseconds since the epoch.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}
