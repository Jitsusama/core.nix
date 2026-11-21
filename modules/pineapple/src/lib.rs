//! The safe word that stops Joel's agents, read in context.
//!
//! Anyone who sees his agents going wrong may tag him with "pineapple
//! override", and what it names stops: one pull request, a quest, a
//! repository, a realm or everything. Only Joel starts them again. This
//! library is the judging half, with what a reader needs beside it:
//!
//! - [`judge`] says whether a saying is the safe word, a resume, an ask to
//!   stop without it, or a near miss (no tag, a quote, talked about, a
//!   joke, a bot), and what its words name.
//! - [`reach`] turns what the words named, with what the place it was said
//!   in is about, into what the stop reaches, falling back to everything
//!   when nothing narrower is found.
//! - [`decide`] says what follows from a saying heard in a place: what it
//!   stops or resumes, how loudly Joel is told, and what its writer is
//!   answered.
//! - [`gate`](mod@gate) says whether that answer goes out in public, by
//!   Joel's rules against spam, and names the rule either way, and
//!   [`reply`] says what one that goes says, from Joel's own account,
//!   only in the thread that asked, in words he said yes to.
//! - [`slack`] and [`github`] turn each platform's answers into sayings
//!   to judge, [`gateway`] reads Slack's as a tool gateway words them, and [`record`] writes down what was decided, a stop's
//!   record included.
//! - [`resume`] lists the stops standing and marks those Joel resumes,
//!   for his own command (src/bin/pineapple.rs), which refuses under pi.
//! - [`tell`] makes the rows Joel is shown in the launcher's Needs You,
//!   with the presses each offers, and [`acts`] takes those presses
//!   back; [`serve`] serves them on a socket of its own, taking a press
//!   only from his shell ([`peer`]), and knocks as a row first appears.
//! - [`sign_in`] runs the program a reader is given for Joel's sign-in,
//!   never waiting on it for good (../slack/sign-in-program), and says so when
//!   it ran over.
//! - [`phone`] says the same alert in Joel's own note-to-self IM in
//!   Slack, the one place it can send to, so it reaches his phone.
//! - [`deaf`] tells Joel where Slack isn't needed when the Slack reader
//!   can't hear Slack, since that alert goes through it.
//! - [`judged`] keeps how far the Slack reader has read, so a restart
//!   neither judges a message twice nor misses one.
//! - [`pause`] is the stop on everything Joel says himself, as before a
//!   demo: beside its record, it refuses agents' starts, stops their
//!   timers and freezes their work, here and on each machine named, and
//!   undoes exactly that once he resumes it.
//!
//! Readers hand it sayings from each platform, Slack first
//! (src/bin/pineapple-slack.rs), then GitHub
//! (src/bin/pineapple-github.rs), and the enforcer acts on what it
//! decides. The cases in sayings.json are the one table of what counts,
//! so anything else judging a saying can be held to the same answers.

pub mod acts;
pub mod deaf;
mod decide;
pub mod gate;
pub mod gateway;
pub mod github;
mod judge;
pub mod judged;
pub mod pause;
pub mod peer;
pub mod phone;
pub mod progress;
pub mod record;
pub mod reply;
pub mod resume;
mod scope;
pub mod serve;
pub mod sign_in;
pub mod slack;
pub mod tell;
pub mod vocabulary;
mod words;

pub use decide::{Decision, Heard, Reply, Tell, decide};
pub use judge::{Joel, Judgement, Platform, Saying, Verdict, Waved, judge};
pub use scope::{Context, Reach, Target, reach};
