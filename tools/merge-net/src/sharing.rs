//! What `ready` says an earlier gate holding a change back shares with
//! it (`turn.rs`). A gate holds a change for a file both change, and,
//! since 15 November, for a file of the change in a crate beside one the
//! gate changes, which the gate doesn't change at all; saying the second
//! as the first sent an author looking for a merge base that was right.

/// What a gate changing `theirs` shares with the change asking, `shared`
/// among that change's files, in words: those it changes too, and those
/// held only for a crate of the gate's they're in, read or are read by,
/// which it doesn't change. Most often the crate is the one they're in, so
/// that comes first.
pub fn said(theirs: &[String], shared: &[String]) -> String {
    let (both, near): (Vec<&str>, Vec<&str>) = shared
        .iter()
        .map(String::as_str)
        .partition(|file| theirs.iter().any(|theirs| theirs == file));
    let near = (!near.is_empty()).then(|| {
        let reads = if near.len() == 1 {
            "is in, reads or is"
        } else {
            "are in, read or are"
        };
        format!("a crate this one's {} {reads} read by", near.join(", "))
    });
    match (both.is_empty(), near) {
        (false, None) => format!("changes {} too", both.join(", ")),
        (false, Some(near)) => format!("changes {} too, and {near}", both.join(", ")),
        (true, Some(near)) => format!("changes {near}"),
        (true, None) => "changes nothing this one does".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn a_gate_is_said_to_change_a_file_only_when_it_does() {
        // On 15 November a gate changing terminal-kit was said to change
        // code-health's files, which read it, so its owner went looking
        // for a wrong merge base.
        let theirs = owned(&[
            "modules/terminal-kit/src/lib.rs",
            "modules/cadence/src/day.rs",
        ]);
        for (shared, words) in [
            (
                owned(&["modules/cadence/src/day.rs"]),
                "changes modules/cadence/src/day.rs too",
            ),
            (
                owned(&["modules/code-health/src/hour.rs"]),
                "changes a crate this one's modules/code-health/src/hour.rs is in, reads or is \
                 read by",
            ),
            (
                owned(&[
                    "modules/cadence/src/day.rs",
                    "modules/code-health/src/hour.rs",
                ]),
                "changes modules/cadence/src/day.rs too, and a crate this one's \
                 modules/code-health/src/hour.rs is in, reads or is read by",
            ),
            // Held behind a gate changing the same crate's views, also on
            // 15 November, and said as changing them.
            (
                owned(&["modules/net/src/sources/hp.rs", "modules/net/src/model.rs"]),
                "changes a crate this one's modules/net/src/sources/hp.rs, \
                 modules/net/src/model.rs are in, read or are read by",
            ),
        ] {
            assert_eq!(
                said(&theirs, &shared),
                words,
                "what a gate shares with {shared:?} was said wrongly"
            );
        }
    }
}
