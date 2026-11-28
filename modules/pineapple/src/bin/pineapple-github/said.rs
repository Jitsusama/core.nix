//! What was said on one pull request or issue, as a round reads it, and
//! who may stop Joel there: kept apart from the round, which only lists
//! what to read and judges what this gives back.

use std::collections::HashMap;

use pineapple::Context;
use pineapple::github::{self, Kind, Said, Thread};

use super::{GitHub, PAGE, Unread};

/// What a round hears: each saying, with where and whether by an outsider.
pub(super) type Sayings = Vec<(Said, Thread, Context, bool)>;

/// What was said on a pull request or issue since `since`, each with it,
/// the place it's about and whether an outsider said it: where anyone may
/// comment (`open`), a writer not the repository's own, or anyone on a
/// repository Joel doesn't work in. Reviews can't be asked for by time,
/// so all are read and the round keeps those since. Whether he works in
/// it is asked only when something there is one the round `keeps`, which
/// most threads hold nothing of, and once a round however many threads it
/// has, so his rate limit isn't spent on the rest.
pub(super) fn said_on(
    github: &GitHub<'_>,
    thread: &Thread,
    since: &str,
    who: (bool, &str),
    keeps: &dyn Fn(&Said) -> bool,
    learned: &mut HashMap<String, bool>,
) -> Result<Sayings, Unread> {
    let (open, login) = who;
    let base = format!("/repos/{}", thread.repo);
    let number = thread.number;
    let opened = github.get(&format!("{base}/issues/{number}"), &[])?;
    // Opening it, as a pull request from his fork, he chose to work there.
    let joel_opened = opened["user"]["login"]
        .as_str()
        .is_some_and(|by| by.eq_ignore_ascii_case(login));
    let mut found = github::said(&opened, thread, Kind::Opening);
    // Its writer's standing alone, before anything said after it joins it.
    let opened_by_own = found.first().is_some_and(|it| it.own);
    let recent = [("since", since), ("per_page", PAGE)];
    let comments = github.all(&format!("{base}/issues/{number}/comments"), &recent)?;
    found.extend(github::said(&comments, thread, Kind::Comment));
    if thread.pull {
        let on_lines = github.all(&format!("{base}/pulls/{number}/comments"), &recent)?;
        found.extend(github::said(&on_lines, thread, Kind::ReviewComment));
        let reviews = github.all(
            &format!("{base}/pulls/{number}/reviews"),
            &[("per_page", PAGE)],
        )?;
        found.extend(github::said(&reviews, thread, Kind::Review));
    }
    let works_in =
        !open || joel_opened || (found.iter().any(keeps) && joel_works_in(github, &base, learned)?);
    let trusted = !open || joel_opened || (works_in && opened_by_own);
    let opening = if trusted {
        format!(
            "{}\n\n{}",
            opened["title"].as_str().unwrap_or_default(),
            opened["body"].as_str().unwrap_or_default()
        )
    } else {
        String::new()
    };
    let context = thread.context(&opening);
    Ok(found
        .into_iter()
        .map(|said| {
            let outsider = open && !(works_in && said.own);
            (said, thread.clone(), context.clone(), outsider)
        })
        .collect())
}

/// Whether Joel works in the repository at `base`, asked of GitHub only
/// the first time a round wants to know, and asked again the next round,
/// so a change to what he may do there is heard within a minute.
fn joel_works_in(
    github: &GitHub<'_>,
    base: &str,
    learned: &mut HashMap<String, bool>,
) -> Result<bool, Unread> {
    if let Some(&known) = learned.get(base) {
        return Ok(known);
    }
    let known = match github.get(base, &[]) {
        Ok(repo) => github::works_in(&repo),
        // Unreadable, it's one he doesn't work in; his own words still count.
        Err(Unread::Gone(why)) => {
            eprintln!(
                "<4>{why}, so a stop there by anyone but Joel counts this round \
                 only on what he opened"
            );
            false
        }
        Err(failed) => return Err(failed),
    };
    learned.insert(base.to_owned(), known);
    Ok(known)
}
