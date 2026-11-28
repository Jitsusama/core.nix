//! One thing said on GitHub, judged on the pull request or issue it was
//! said on, and what follows from its decision: a line in decisions.jsonl,
//! a stop's record, and Joel's own resume, which ends only stops written
//! in shadow.

use std::path::Path;

use pineapple::gate::Gate;
use pineapple::github::{Said, Thread};
use pineapple::record::{self, Source};
use pineapple::resume;
use pineapple::{Context, Heard, Joel, Platform, Reply, Target, Verdict, decide};

use super::millis;

/// Microseconds in a second.
const MICROS: u128 = 1_000_000;

/// Whether Joel's resume said on GitHub may end a live stop. Not while his
/// agents post as his GitHub account: a resume one forged would defeat
/// the safe word (WCJQSG, 2026-11-14, as for Slack's).
const GITHUB_RESUMES_LIVE: bool = false;

/// The rule a reply is held by on GitHub, where none is said in public.
const NO_REPLIES: &str = "no-replies-here";

/// One thing said, judged on the pull request or issue it was said on,
/// and written down.
pub(super) fn judge(
    state: &Path,
    joel: &Joel,
    said: &Said,
    place: (&Thread, Context, bool),
    shadow: bool,
) {
    let (thread, context, outsider) = place;
    let decision = decide(
        &Heard {
            platform: Platform::Markdown,
            author: said.author.clone(),
            by_bot: said.by_bot,
            outsider,
            text: said.text.clone(),
            context,
        },
        joel,
    );
    let thread_key = thread.key();
    // As Slack writes its times, so a resume compares them alike.
    let said_at = format!("{}.000000", said.at);
    let source = Source {
        key: &said.key,
        thread: &thread_key,
        link: &said.link,
        author: &said.author,
        text: &said.text,
        said_at: &said_at,
    };
    let now = millis();
    let gate = if decision.reply == Reply::Nothing {
        Gate::Unasked
    } else {
        Gate::Held(NO_REPLIES)
    };
    let line = record::line(&source, &decision, gate, now, shadow);
    if let Err(error) = record::append(&state.join("decisions.jsonl"), &line) {
        eprintln!("<3>couldn't write a decision down: {error}");
    }
    if decision.stops() {
        // Never kept as an earlier resume of Joel's left it, however late
        // it's heard: his account here is his agents' too, and a comment's
        // time isn't always when its words were shown (src/resume.rs).
        let stop = record::stop(&source, &decision, now, shadow);
        match record::keep(&state.join("stops"), &said.key, &stop) {
            Ok(true) if shadow => eprintln!("<5>a stop, in shadow: {}", said.link),
            Ok(true) => eprintln!("<5>a stop: {}", said.link),
            Ok(false) => {}
            Err(error) => eprintln!("<3>couldn't keep a stop's record: {error}"),
        }
    }
    if decision.verdict == Verdict::Resume && decision.by_joel {
        let at = (now, u128::from(said.at) * MICROS);
        resume_as_joel(&state.join("stops"), &decision.reaches, at, &said.link);
    }
}

/// Ends the stops Joel's own "pineapple resume" names (src/resume.rs),
/// only those written in shadow until GitHub resumes are let go live.
/// `at` is when it's heard, in milliseconds, and when it was said, in
/// microseconds.
fn resume_as_joel(stops: &Path, reaches: &[Target], at: (u64, u128), link: &str) {
    let (now, said) = at;
    let shadow = !GITHUB_RESUMES_LIVE;
    for reach in reaches {
        let scope = match reach {
            Target::Everything => "all".to_owned(),
            named => named.to_string(),
        };
        match resume::as_joel(stops, &scope, now, said, shadow) {
            Ok(ids) if ids.is_empty() => {}
            Ok(ids) if shadow => eprintln!("<5>a resume, in shadow, of {}: {link}", ids.join(", ")),
            Ok(ids) => eprintln!("<5>Joel resumed {}: {link}", ids.join(", ")),
            Err(error) => eprintln!("<3>couldn't resume what Joel named: {error}"),
        }
    }
}
