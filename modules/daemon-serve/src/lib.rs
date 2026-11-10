//! The serving side of the launcher's daemon interface, which every
//! daemon serving the shell holds: the socket systemd holds for it, a
//! thread a connection, watches by kind, by key or by place, and a
//! `change` that tells each watch only the records it watches that moved
//! and the keys that went. Lifted from the to-do daemon, so the ready rows
//! every daemon serves (Y2YBTS's ready-rows.md) sit on one copy of it.
//!
//! - `asks`: what a client's line asks, for a domain's own kinds.
//! - `serve`: the records held, the socket, and what each ask is answered.

pub mod asks;
pub mod serve;
