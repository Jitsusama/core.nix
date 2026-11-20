//! What waits on the person whose GitHub login the machine declares, as
//! the collection leads with it: each change asking their review, then
//! each of their own failing its checks or with changes asked. What needs
//! him leads, as docs/design.md's For You says, so a review asked of him
//! is found in a terminal rather than on GitHub.

use terminal_kit::Pen;
use terminal_kit::layout::{Align, ITEMS, Room};

use repos::model::{ChecksState, Record, Review};

use super::{named, table};

/// What waits on someone: why, the change, and its title, last, so a long
/// one is cut, and where even that leaves no room, it goes.
const FOR_YOU: [(&str, Align); 3] = [
    ("for you", Align::Left),
    ("change", Align::Left),
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
/// changes asked, each kind the one updated last first. The login is the
/// one the machine declares, matched as GitHub matches one, ASCII case
/// aside, against GitHub's own fields for who asked and who wrote, never
/// anything a change says of itself.
fn for_you(record: &Record, login: &str) -> Vec<Vec<String>> {
    let is = |one: &str| one.eq_ignore_ascii_case(login);
    let mut waiting: Vec<(bool, u64, Vec<String>)> = Vec::new();
    for repository in &record.repositories {
        let name = named(record, repository);
        for change in &repository.changes {
            let asked = change.review_asked_of.iter().any(|one| is(one));
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
                vec![why, change_named, change.title.clone()],
            ));
        }
    }
    waiting.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    waiting.into_iter().map(|(_, _, row)| row).collect()
}

#[cfg(test)]
mod tests;
