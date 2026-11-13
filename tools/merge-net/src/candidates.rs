//! Which of a day's passed runs could be an earlier commit of the change
//! `merge-net ready` is asked about, read cheaply before `ready` asks git
//! for each one's patch id.

use std::collections::{HashMap, HashSet};

/// Of `records`, those that could be the change: the ones carrying one of
/// its own `subjects`, as `said` gives each record's subject when git
/// could say them, else the ones whose newest commit touched one of its
/// `files`, as `touched` says, else every one, as slow as before but no
/// less right. The subjects leave a few of a day's thousand records, so
/// the files, the dearer read, are needed only when the subjects aren't,
/// and judging by them too lost the pass of a change whose newest commit
/// touched none of its files, as a commit taking a note back does.
pub fn could_be(
    records: &[String],
    said: Option<&str>,
    subjects: &[String],
    touched: impl FnOnce(&[String]) -> Option<String>,
    files: &[String],
) -> Vec<String> {
    if let Some(said) = said {
        return titled(records, said, subjects);
    }
    touched(records).map_or(records.to_vec(), |touched| likely(records, &touched, files))
}

/// Of `candidates`, those whose commit carries the subject of one of the
/// change's own commits, `subjects`, read from `said`, lines of a commit
/// and its subject split by `\x01`: a rebase keeps a commit's message, so
/// an earlier run of the same change is among them. Read
/// from the commits alone, it costs a fraction of a second, where the
/// files each touched, read from their trees, took 52 s of a `ready` past
/// five minutes under the disk's load on 10 November.
fn titled(candidates: &[String], said: &str, subjects: &[String]) -> Vec<String> {
    let kept: HashSet<&str> = said
        .lines()
        .filter_map(|line| line.split_once('\x01'))
        .filter(|(_, subject)| subjects.iter().any(|own| own == subject))
        .map(|(id, _)| id)
        .collect();
    candidates
        .iter()
        .filter(|candidate| kept.contains(candidate.as_str()))
        .cloned()
        .collect()
}

/// The candidates, in their order, whose own newest commit changed one of
/// `files`, the files the head's change makes, as `touched` says: `git log
/// --name-only` over every candidate at once, each commit's files below a
/// line holding `\x01` and its id. A rebase keeps a change's newest commit
/// and the files it touches, so a candidate whose newest commit touched
/// none of them is another change, told apart without a git of its own.
/// On 10 November a day held 1,034 passed records, and three gits for
/// each, a merge base, a diff and a patch id, made `ready` take two
/// minutes, and past five once, so the review tool's merge gave up on
/// #2820. Read only when the subjects can't be, it loses a change whose
/// newest commit touches only what the change as a whole leaves alone, or
/// is a merge, which runs its checks again: a run, never a merge unchecked.
fn likely(candidates: &[String], touched: &str, files: &[String]) -> Vec<String> {
    let mut changed: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut current = None;
    for line in touched.lines() {
        if let Some(id) = line.strip_prefix('\x01') {
            current = Some(id.trim());
            changed.entry(id.trim()).or_default();
        } else if let Some(id) = current
            && !line.is_empty()
        {
            changed.entry(id).or_default().push(line);
        }
    }
    candidates
        .iter()
        .filter(|candidate| {
            changed
                .get(candidate.as_str())
                .is_some_and(|its| its.iter().any(|file| files.iter().any(|f| f == file)))
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn only_runs_carrying_one_of_the_changes_subjects_are_read() {
        let candidates = owned(&["a", "b", "c", "d"]);
        let said = "a\x01fix(x): mend it\nb\x01feat(y): other work\nc\x01Merge e into f\n";
        assert_eq!(
            titled(&candidates, said, &["fix(x): mend it".to_owned()]),
            vec!["a".to_owned()],
            "only its own commit; d was never said"
        );
    }

    #[test]
    fn only_runs_whose_newest_commit_touched_the_change_are_read() {
        let touched = "\x01rebased\n\na.rs\nc.md\n\x01other\n\nd.rs\n\x01merge\n\x01old\n\nb.md\n";
        let candidates = owned(&["rebased", "other", "merge", "gone", "old"]);
        let files = owned(&["a.rs", "b.md"]);
        assert_eq!(
            likely(&candidates, touched, &files),
            owned(&["rebased", "old"]),
            "it kept a run of another change, or lost one of this change, or their order"
        );
    }

    // #4015's newest commit took back a note it had added to a file of
    // another crate, so its own files were none of the change's, and on
    // 13 November its pass was lost and a whole gate ran again.
    #[test]
    fn a_run_carrying_the_changes_subject_is_read_whatever_its_newest_commit_touched() {
        let records = owned(&["rebased", "other"]);
        let said = "rebased\x01fix(x): take the note back\nother\x01feat(y): other work\n";
        let touched = |_: &[String]| Some("\x01rebased\n\nelsewhere.rs\n".to_owned());
        assert_eq!(
            could_be(
                &records,
                Some(said),
                &owned(&["fix(x): take the note back"]),
                touched,
                &owned(&["a.rs"]),
            ),
            owned(&["rebased"]),
            "the change's own earlier run was lost for what its newest commit touched"
        );
    }

    #[test]
    fn without_subjects_only_runs_touching_the_change_are_read() {
        let records = owned(&["rebased", "other"]);
        let touched = |_: &[String]| Some("\x01rebased\n\na.rs\n\x01other\n\nd.rs\n".to_owned());
        assert_eq!(
            could_be(&records, None, &[], touched, &owned(&["a.rs"])),
            owned(&["rebased"]),
            "with no subjects to read, another change's run was kept"
        );
    }
}
