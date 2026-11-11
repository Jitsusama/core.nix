//! The windows daemon's own logic, apart from its socket and threads so
//! its tests reach it: what a client asks ([`interface`]), what niri's
//! events say of the desktop and the records drawn from it ([`desktop`]),
//! what's cast and shot of the screen ([`capture`]), which program each
//! window is ([`entries`]), and the launcher's Screens, served as rows
//! ready to show ([`screens`]).
//!
//! A record names every figure the shell draws a window's row from, as
//! niri says it, and none a row doesn't show: words are the shell's, and
//! what's said only so a row can be placed, its `order`, is when its
//! window was last looked at, in milliseconds.

pub mod capture;
pub mod desktop;
pub mod entries;
pub mod interface;
pub mod screens;
