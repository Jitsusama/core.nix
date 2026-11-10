//! The Google account, for the desktop, as a library the program
//! (main.rs) and its tests share: the account as the harness keeps it
//! (account.rs), the token to ask with (token.rs), asking Google
//! (api.rs), the meetings and the link each is joined by (calendar.rs,
//! link.rs), the mail that wants Joel (mail.rs) and where the files it
//! carries go (files.rs), the documents he looked at lately and Drive's
//! own places (drive.rs),
//! what the shell is told (state.rs) and how, in the daemon interface
//! (daemon.rs), how a thing stopped wanting Joel (done.rs), what the
//! shell asks (ask.rs), what Joel saves into Drive (saves.rs), the Meet
//! calls Joel is in (meet.rs), the account as it stands (watch.rs), and
//! everyone its meetings and mail name, for People (people.rs).

pub mod account;
pub mod api;
pub mod ask;
pub mod calendar;
pub mod daemon;
pub mod done;
pub mod drive;
pub mod files;
pub mod link;
pub mod mail;
pub mod meet;
pub mod people;
pub mod rows;
pub mod saves;
pub mod state;
pub mod token;
pub mod watch;
