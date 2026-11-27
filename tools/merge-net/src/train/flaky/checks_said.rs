//! What changed-checks said in a gate's words of the checks it built: which
//! it built, which the riders added, and which main's quarantine excused.
//! It speaks in lines of its own, unprefixed, but so may a rider's Nix,
//! whose evaluation prints to the same log; so what excuses a check is
//! believed only where main's own list, read at the base, agrees.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use super::plainly_named;
use crate::world::QUARANTINE;

/// The checks the gate's `words` name as added or changed, which it built,
/// a row each of changed-checks' table: its kind, then its name.
pub(super) fn built(words: &str) -> Vec<String> {
    of_kinds(words, &["added", "changed"]).collect()
}

/// The checks the gate's `words` name as added: the riders made each, so
/// none of them is at the base to be built there.
pub(super) fn added(words: &str) -> BTreeSet<String> {
    of_kinds(words, &["added"]).collect()
}

/// The names of changed-checks' table rows of one of `kinds`.
fn of_kinds<'a>(words: &'a str, kinds: &'a [&str]) -> impl Iterator<Item = String> + 'a {
    words.lines().filter_map(|line| {
        let mut words = line.split_whitespace();
        let kind = words.next()?;
        let name = words.next()?;
        kinds.contains(&kind).then(|| name.to_owned())
    })
}

/// The checks whose failure the gate's `words` say main's quarantine
/// excused, of those it `held`: changed-checks built each alone, and its
/// red blocked nothing, so the failure its build left in the words isn't
/// the red's. Judging it again would build a check broken at every base
/// and blame the riders. A builder can't say it, since nix's `-L` puts the
/// builder's name before each line it prints, but a rider's evaluation
/// can, as a trace; a check the list doesn't hold is still judged.
pub(super) fn quarantined(words: &str, held: &BTreeSet<String>) -> BTreeSet<String> {
    words
        .lines()
        .filter_map(|line| {
            let said = line
                .strip_prefix("changed-checks: ")?
                .strip_suffix(", so it blocks nothing")?;
            let (name, _) = said.split_once(" failed, but ")?;
            (plainly_named(name) && held.contains(name)).then(|| name.to_owned())
        })
        .collect()
}

/// The checks main's quarantine list excuses whole at `base`, read in the
/// train's `checkout`, as changed-checks reads it from main; none where it
/// doesn't read, so nothing is then left out of the judging.
pub(super) fn held_at(checkout: &Path, base: &str) -> BTreeSet<String> {
    Command::new("git")
        .args(["show", &format!("{base}:{QUARANTINE}")])
        .current_dir(checkout)
        .output()
        .ok()
        .filter(|shown| shown.status.success())
        .and_then(|shown| serde_json::from_slice(&shown.stdout).ok())
        .map(|list| crate::quarantine::whole(&list).into_iter().collect())
        .unwrap_or_default()
}
