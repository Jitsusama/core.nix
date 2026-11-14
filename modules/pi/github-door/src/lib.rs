//! The door a fenced pi command reaches GitHub through.
//!
//! The agent harness runs an agent's shell commands in a fence that keeps
//! out the keyring, where gh keeps the person's token, so a fenced `gh`
//! has no token: `gh api`, project boards and sub-issues fail. Letting the
//! token in would let a command do anything the person can on GitHub. The
//! door stands between instead, outside the fence, holding the token in
//! memory only:
//!
//! - **Sessions:** the harness asks the control socket, which the fence
//!   never lets in, to open a session (src/control.rs). The door makes
//!   the session a folder holding its own socket and a bundle of the
//!   system's certificates plus the session's own CA (src/authority.rs),
//!   whose key never leaves the door, and the fence binds that folder in.
//! - **The relay:** inside the fence, gh wants a proxy over TCP, so the
//!   relay (src/relay.rs) listens on localhost for the fence's own
//!   processes alone, sends each tunnel to GitHub's four hosts to the
//!   session's socket, and takes every other host straight out itself.
//! - **The door:** for api.github.com, github.com, uploads.github.com and
//!   codeload.github.com, it answers the tunnel as GitHub with the
//!   session's CA, reads each request (src/http.rs), judges it
//!   (src/judge.rs, src/graphql.rs), and sends what it allows on with the
//!   person's token in place of the placeholder the fence holds
//!   (src/door.rs, src/token.rs). Any other host reaching it is tunnelled
//!   untouched and unseen.
//! - **The journal:** each call passed or refused is a line of fixed
//!   fields (src/said.rs), never a body or a header, and a session past
//!   600 calls an hour is warned of, and past 2,000 refused until the
//!   hour resets or Joel lifts it (src/cap.rs).

pub mod authority;
pub mod cap;
pub mod control;
pub mod door;
pub mod graphql;
pub mod http;
pub mod judge;
pub mod peer;
pub mod relay;
pub mod said;
pub mod token;
