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
//!   Joel's rules against spam, and names the rule either way.
//! - [`slack`] turns Slack's answers into messages to judge, and
//!   [`record`] writes down what was decided, a stop's record included.
//! - [`resume`] lists the stops standing and marks those Joel resumes,
//!   for his own command (src/bin/pineapple.rs), which refuses under pi.
//! - [`tell`] makes the rows Joel is shown in the launcher's Needs You,
//!   with the presses each offers, and [`acts`] takes those presses
//!   back; [`serve`] serves them on a socket of its own, taking a press
//!   only from his shell ([`peer`]), and knocks as a row first appears.
//!
//! Readers hand it sayings from each platform, Slack first
//! (src/bin/pineapple-slack.rs), and the enforcer acts on what it
//! decides. The cases in sayings.json are the one table of what counts,
//! so anything else judging a saying can be held to the same answers.

pub mod acts;
mod decide;
pub mod gate;
mod judge;
pub mod peer;
pub mod record;
pub mod resume;
mod scope;
pub mod serve;
pub mod slack;
pub mod tell;
pub mod vocabulary;
mod words;

pub use decide::{Decision, Heard, Reply, Tell, decide};
pub use judge::{Joel, Judgement, Platform, Saying, Verdict, Waved, judge};
pub use scope::{Context, Reach, Target, reach};
