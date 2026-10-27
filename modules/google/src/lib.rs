//! The Google account, for the desktop, as a library the program
//! (main.rs) and its tests share: the account as the harness keeps it
//! (account.rs), the token to ask with (token.rs), asking Google
//! (api.rs), the meetings and the link each is joined by (calendar.rs,
//! link.rs), the mail that wants Joel (mail.rs), what the shell is told (state.rs) and how (socket.rs), what
//! the shell asks (ask.rs), and the account as it stands (watch.rs).

pub mod account;
pub mod api;
pub mod ask;
pub mod calendar;
pub mod link;
pub mod mail;
pub mod socket;
pub mod state;
pub mod token;
pub mod watch;
