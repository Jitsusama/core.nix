//! The safe word that stops Joel's agents, read in context.
//!
//! Anyone who sees his agents going wrong may tag him with "pineapple
//! override", and what it names stops: one pull request, a quest, a
//! repository, a realm or everything. Only Joel starts them again. This
//! library is the judging half, with no reading or stopping of its own:
//!
//! - [`judge`] says whether a saying is the safe word, a resume, an ask to
//!   stop without it, or a near miss (no tag, a quote, talked about, a
//!   joke, a bot), and what its words name.
//! - [`reach`] turns what the words named, with what the place it was said
//!   in is about, into what the stop reaches, falling back to everything
//!   when nothing narrower is found.
//!
//! Readers hand it sayings from each platform, Slack first, and the
//! enforcer acts on what it decides. The cases in sayings.json are the one
//! table of what counts, so anything else judging a saying can be held to
//! the same answers.

mod judge;
mod scope;
mod words;

pub use judge::{Joel, Judgement, Platform, Saying, Verdict, Waved, judge};
pub use scope::{Context, Reach, Target, reach};
