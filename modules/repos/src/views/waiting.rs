//! What waits on the person whose GitHub login the machine declares, as
//! the collection leads with it: each change asking their review, then
//! each of their own failing its checks or with changes asked. What needs
//! him leads, as docs/design.md's For You says, so a review asked of him
//! is found in a terminal rather than on GitHub.

use std::collections::BTreeSet;

use terminal_kit::Pen;
use terminal_kit::layout::{Align, ITEMS, Room};

use repos::model::{ChecksState, Record, Review};

use super::{named, table};

/// What waits on someone: the change, first, so it's never cut, why, and
/// its title, last, so a long one is cut, and where even that leaves no
/// room, it goes.
const FOR_YOU: [(&str, Align); 3] = [
    ("change", Align::Left),
    ("for you", Align::Left),
    ("title", Align::Left),
];
const FOR_YOU_GIVES_WAY: [usize; 1] = [2];

/// The table of what waits on `login`, or nothing where no login is
/// declared or nothing waits.
pub(super) fn waiting(record: &Record, login: Option<&str>, pen: &Pen, room: Room) -> Vec<String> {
    let rows = login.map_or_else(Vec::new, |login| for_you(record, login));
    if rows.is_empty() {
        return Vec::new();
    }
    table(&FOR_YOU, rows, &FOR_YOU_GIVES_WAY, pen, room)
}

/// The open changes waiting on the person GitHub knows as `login`: each
/// asking their review, then each of their own failing its checks or with
/// changes asked, each kind the one updated last first, across every
/// repository. Whose review a change asks is what the refresh kept of it,
/// worked out against the same declared login, since the logins aren't
/// kept (model.rs); whose it is, GitHub's author, matched as GitHub
/// matches logins, ASCII case aside. Nothing a change says of itself
/// counts. Two clones of one repository, or a remote typed in another
/// case, hold the same changes, so each is led with once, by GitHub's own
/// address for it, as the receptor counts it once.
fn for_you(record: &Record, login: &str) -> Vec<Vec<String>> {
    let is = |one: &str| one.eq_ignore_ascii_case(login);
    let mut led: BTreeSet<&str> = BTreeSet::new();
    let mut waiting: Vec<(bool, u64, Vec<String>)> = Vec::new();
    for repository in &record.repositories {
        let name = named(record, repository);
        for change in &repository.changes {
            if let Some(github) = &change.github
                && !led.insert(github.url.as_str())
            {
                continue;
            }
            let asked = change.asks_your_review;
            let why = if asked {
                "your review".to_owned()
            } else if change.author.as_deref().is_some_and(is) {
                let failing = change.checks.state == ChecksState::Failing;
                let changes_asked = change.review == Review::ChangesAsked;
                let why: Vec<&str> = [(failing, "failing"), (changes_asked, "changes asked")]
                    .into_iter()
                    .filter_map(|(holds, why)| holds.then_some(why))
                    .collect();
                if why.is_empty() {
                    continue;
                }
                why.join(ITEMS)
            } else {
                continue;
            };
            let change_named = match &change.github {
                Some(github) => format!("{name}#{}", github.number),
                None => format!("{name} {}", change.branch),
            };
            waiting.push((
                asked,
                change.updated_at,
                vec![change_named, why, change.title.clone()],
            ));
        }
    }
    waiting.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    waiting.into_iter().map(|(_, _, row)| row).collect()
}

#[cfg(test)]
mod tests;
