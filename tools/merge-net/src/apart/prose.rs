//! Whether two changes to one of docs/'s notes touch different paragraphs,
//! so the gate can count them as two files. A note is read by no check but
//! formatting and rumdl, which fixes nothing and reads a paragraph, a list
//! or a heading's place on its own, so changes a paragraph apart meet as
//! two files' would. A heading or a reference definition is what one part
//! of a note reads of another (an anchor, a link's target), so a change to
//! either stays shared.

use std::collections::BTreeSet;

use super::{Edits, edits_in_place, meet_apart, merged};

/// docs/'s notes a check reads as data: the glossary's words are read by
/// the launcher's words checks, the glossary's own and the names check.
const READ: [&str; 1] = ["docs/glossary.md"];

/// Whether `file` is a note in docs/ no check reads as data.
pub fn is_prose(file: &str) -> bool {
    file.starts_with("docs/")
        && std::path::Path::new(file)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        && !READ.contains(&file)
}

/// Whether a note both sides changed was changed apart: each side's
/// changes sit in paragraphs the other's don't touch, neither touches a
/// heading or a reference definition, and the rebased `head` is `base`
/// with both sides' changes. On 12 November docs/design.md sent #3769
/// back to gate though it and main had edited different parts of it, and
/// #3736 paid one of its five gates for the same.
#[must_use]
pub fn prose_apart(base: &str, branch: &str, main: &str, head: &str) -> bool {
    let lines: Vec<&str> = base.lines().collect();
    let head: Vec<&str> = head.lines().collect();
    let (Some(ours), Some(theirs)) = (edits_in_place(&lines, branch), edits_in_place(&lines, main))
    else {
        return false;
    };
    meet(&lines, &ours, &theirs)
        && [(&ours, &theirs), (&theirs, &ours)]
            .iter()
            .any(|(first, then)| merged(&lines, first, then) == head)
}

/// Whether two changes from `base`, `one` and `other`, would meet as
/// `prose_apart` asks once either is rebased onto the other.
#[must_use]
pub fn would_prose_be_apart(base: &str, one: &str, other: &str) -> bool {
    let lines: Vec<&str> = base.lines().collect();
    match (edits_in_place(&lines, one), edits_in_place(&lines, other)) {
        (Some(one), Some(other)) => meet(&lines, &one, &other),
        _ => false,
    }
}

/// Whether two sides' `Edits` of `base` meet apart, as `prose_apart` says.
fn meet(base: &[&str], one: &Edits, other: &Edits) -> bool {
    meet_apart(one, other)
        && !structural(base, one)
        && !structural(base, other)
        && paragraphs(base, one).is_disjoint(&paragraphs(base, other))
}

/// Whether a side takes away or adds a heading or a reference definition.
fn structural(base: &[&str], side: &Edits) -> bool {
    let taken = side.removed.iter().map(|at| base[*at]);
    let given = side.added.values().flatten().copied();
    taken.chain(given).any(|line| {
        let line = line.trim_start();
        line.starts_with('#') || reference(line)
    })
}

/// Whether `line`, its indent gone, defines a reference: `[label]: target`.
fn reference(line: &str) -> bool {
    line.strip_prefix('[')
        .and_then(|rest| rest.split_once("]:"))
        .is_some_and(|(label, _)| !label.is_empty())
}

/// The paragraphs of `base` a side touches, each named by how many blank
/// lines come before it. A line taken away is in its paragraph; a blank
/// one taken away joins the paragraphs either side, so it touches both.
/// Lines added before `base`'s line `at` touch the paragraphs of the lines
/// either side of where they go.
fn paragraphs(base: &[&str], side: &Edits) -> BTreeSet<usize> {
    let blank = |at: usize| base.get(at).is_some_and(|line| line.trim().is_empty());
    let mut before = Vec::with_capacity(base.len() + 1);
    let mut count = 0;
    for at in 0..=base.len() {
        before.push(count);
        if blank(at) {
            count += 1;
        }
    }
    // The paragraphs a line of `base` is in: its own, or for a blank line
    // both it divides.
    let of = |at: usize| -> Vec<usize> {
        if blank(at) {
            vec![before[at], before[at] + 1]
        } else {
            vec![before[at]]
        }
    };
    let mut touched = BTreeSet::new();
    for at in &side.removed {
        touched.extend(of(*at));
    }
    for at in side.added.keys() {
        touched.extend(at.checked_sub(1).map_or_else(Vec::new, of));
        if *at < base.len() {
            touched.extend(of(*at));
        }
    }
    touched
}

#[cfg(test)]
mod tests;
