//! What `ready` says an earlier gate holding a change back shares with
//! it (`turn.rs`). A gate holds a change for a file both change, and,
//! since 15 November, for a file of the change in a crate beside one the
//! gate changes, which the gate doesn't change at all; saying the second
//! as the first sent an author looking for a merge base that was right.
//! Since 27 November a file held for a fate the gate touches
//! (`rules/fates.rs`) is said as the fate, not as a crate.

/// What a gate changing `theirs` shares with the change asking, `shared`
/// among that change's files, in words: those it changes too, those held
/// for a fate of the gate's they lie in, as `fated` names each, and those
/// held only for a crate of the gate's they're in, read or are read by,
/// which it doesn't change. Most often the crate is the one they're in, so
/// that comes first.
pub fn said(theirs: &[String], shared: &[String], fated: &[(String, String)]) -> String {
    let fate_of = |file: &str| {
        fated
            .iter()
            .find(|(fated, _)| fated == file)
            .map(|(_, fate)| fate.as_str())
    };
    let mut both: Vec<&str> = Vec::new();
    let mut near: Vec<&str> = Vec::new();
    let mut by_fate: Vec<(&str, Vec<&str>)> = Vec::new();
    for file in shared.iter().map(String::as_str) {
        if theirs.iter().any(|theirs| theirs == file) {
            both.push(file);
        } else if let Some(fate) = fate_of(file) {
            match by_fate.iter_mut().find(|(named, _)| *named == fate) {
                Some((_, files)) => files.push(file),
                None => by_fate.push((fate, vec![file])),
            }
        } else {
            near.push(file);
        }
    }
    let mut clauses: Vec<String> = Vec::new();
    if !both.is_empty() {
        clauses.push(format!("{} too", both.join(", ")));
    }
    if !near.is_empty() {
        let reads = if near.len() == 1 {
            "is in, reads or is"
        } else {
            "are in, read or are"
        };
        clauses.push(format!(
            "a crate this one's {} {reads} read by",
            near.join(", ")
        ));
    }
    for (fate, files) in by_fate {
        let lie = if files.len() == 1 { "lies" } else { "lie" };
        clauses.push(format!(
            "a folder of the fate {fate:?}, which one check measures and this one's {} {lie} in",
            files.join(", ")
        ));
    }
    if clauses.is_empty() {
        return "changes nothing this one does".to_owned();
    }
    format!("changes {}", clauses.join(", and "))
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
                said(&theirs, &shared, &[]),
                words,
                "what a gate shares with {shared:?} was said wrongly"
            );
        }
    }

    #[test]
    fn a_file_held_for_a_fate_is_said_as_the_fate() {
        // A file held for the map's fate was said to be in a crate of the
        // gate's, which no crate holds.
        let theirs = owned(&[
            "modules/mycelium/site/view.js",
            "modules/cadence/src/day.rs",
        ]);
        let shared = owned(&[
            "modules/cadence/src/day.rs",
            "modules/local-sites/kit/head.css",
            "modules/local-sites/kit/feed.ts",
        ]);
        let fated = [
            (
                "modules/local-sites/kit/head.css".to_owned(),
                "the map".to_owned(),
            ),
            (
                "modules/local-sites/kit/feed.ts".to_owned(),
                "the map".to_owned(),
            ),
        ];
        assert_eq!(
            said(&theirs, &shared, &fated),
            "changes modules/cadence/src/day.rs too, and a folder of the fate \"the map\", \
             which one check measures and this one's modules/local-sites/kit/head.css, \
             modules/local-sites/kit/feed.ts lie in",
            "a file held for a fate was said as something else"
        );
    }
}
