//! A call's one file, transcript.md in its folder: its title, then a
//! Summary heading over its summary, then a Transcript heading over the
//! whole transcript, so the call is read, searched and shared as one.
//! The transcript writes it as it's heard (hear.rs), keeping the summary
//! there; the summarizer writes it again with the summary once it's
//! made (summarizer.rs). A folder from before keeps its summary.md
//! beside it, and is read as it was written (book.rs).

/// What the Summary heading holds before a summary is written, or where
/// none ever is, on a laptop with no summarizer or once summing up failed.
pub const PENDING: &str = "No summary has been written yet.";

const SUMMARY: &str = "## Summary";
const TRANSCRIPT: &str = "## Transcript";

/// The call's file: `title` as its heading, when it has one, `summary`
/// under the Summary heading, or that it's still to be written, and
/// `transcript`, as transcript.rs writes it or as [`transcript`] reads
/// it back, whole under the Transcript heading.
pub fn whole(title: &str, summary: Option<&str>, transcript: &str) -> String {
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let summary = summary.map_or(PENDING, |summary| untitled(summary).trim());
    let said = transcript.trim_start();
    let said = said
        .strip_prefix(TRANSCRIPT)
        .or_else(|| said.strip_prefix("# Transcript"))
        .unwrap_or(said)
        .trim_start();
    let heading = if title.is_empty() {
        String::new()
    } else {
        format!("# {title}\n\n")
    };
    let mut file = format!("{heading}{SUMMARY}\n\n{summary}\n\n{TRANSCRIPT}\n\n{said}");
    if !file.ends_with('\n') {
        file.push('\n');
    }
    file
}

/// The call's file, `file`, with `summary` written into it, as long as
/// its transcript is still `summed`, the one the summary was made from;
/// none once the call went on, rejoined, since a summary of part of a
/// call reads as the whole.
pub fn summed(file: &str, title: &str, summary: &str, summed: &str) -> Option<String> {
    let said = transcript(file);
    (said == summed).then(|| whole(title, Some(summary), said))
}

/// Whether `file` is a call's one file, rather than a transcript from
/// before, which has its summary.md beside it.
pub fn is_one(file: &str) -> bool {
    headings(file).is_some()
}

/// A summary without the title summary.rs writes it under, since the
/// file's own heading says it.
pub fn untitled(summary: &str) -> &str {
    let summary = summary.trim_start();
    match summary.split_once('\n') {
        Some((first, rest)) if first.starts_with("# ") => rest,
        None if summary.starts_with("# ") => "",
        _ => summary,
    }
}

/// Where the Summary and Transcript headings open, as `whole` wrote them:
/// the Transcript heading looked for only after the Summary's, so a
/// title can't be taken for it, and a summary is lines of quotes, which
/// never open with a heading.
fn headings(file: &str) -> Option<(usize, usize)> {
    let summary = at_line(file, SUMMARY, 0)?;
    let transcript = at_line(file, TRANSCRIPT, summary + SUMMARY.len())?;
    Some((summary, transcript))
}

/// Where `heading` first stands on a line of its own in `file`, at or
/// after `from`.
fn at_line(file: &str, heading: &str, from: usize) -> Option<usize> {
    let mut at = from;
    for line in file.get(from..)?.split_inclusive('\n') {
        if line.trim_end() == heading && (at == 0 || file[..at].ends_with('\n')) {
            return Some(at);
        }
        at += line.len();
    }
    None
}

/// The summary a call's file holds under its Summary heading, or none
/// while it's still to be written, or in a file with no such heading.
pub fn summary(file: &str) -> Option<String> {
    let (summary, transcript) = headings(file)?;
    let held = file[summary + SUMMARY.len()..transcript].trim();
    (held != PENDING && !held.is_empty()).then(|| held.to_owned())
}

/// The transcript a call's file holds under its Transcript heading, as
/// transcript.rs wrote it; or the whole file, for one with no such
/// heading, as a folder from before holds.
pub fn transcript(file: &str) -> &str {
    headings(file).map_or(file, |(_, transcript)| &file[transcript..])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAID: &str = "# Transcript\n\nInvited: Ada and Bo.\n\n\
        **Joel**, 0:00: Can we move it?\n\n**Others**, 0:04: ## Thursday works.\n";
    const SUMMED: &str = "# Planning\n\nThese are the lines a language model picked.\n\n\
        - Others, to Joel: \"Thursday works\" [0:04]";

    #[test]
    fn a_summed_up_call_is_one_file_its_summary_over_its_whole_transcript() {
        let file = whole("Planning", Some(SUMMED), SAID);
        let want = "# Planning\n\n## Summary\n\n\
            These are the lines a language model picked.\n\n\
            - Others, to Joel: \"Thursday works\" [0:04]\n\n\
            ## Transcript\n\nInvited: Ada and Bo.\n\n\
            **Joel**, 0:00: Can we move it?\n\n**Others**, 0:04: ## Thursday works.\n";
        assert_eq!(file, want);
        assert_eq!(
            transcript(&file),
            SAID.replacen("# Transcript", TRANSCRIPT, 1),
            "the Transcript section is the whole transcript, byte for byte"
        );
        assert_eq!(
            summary(&file).as_deref(),
            Some(
                "These are the lines a language model picked.\n\n\
                 - Others, to Joel: \"Thursday works\" [0:04]"
            ),
            "the summary comes back without the title it was written under"
        );
    }

    #[test]
    fn a_summary_is_kept_only_over_the_transcript_it_was_made_from() {
        let first = whole("Planning", None, SAID);
        let read = transcript(&first).to_owned();
        let kept = summed(&first, "Planning", SUMMED, &read);
        assert_eq!(kept, Some(whole("Planning", Some(SUMMED), SAID)));
        let newer = "- Joel, to Others: \"Can we move it?\" [0:00]";
        let replaced = summed(kept.as_deref().unwrap(), "Planning", newer, &read).unwrap();
        assert_eq!(
            summary(&replaced).as_deref(),
            Some(newer),
            "a summary there before gives way to the new one"
        );
        let rejoined = whole(
            "Planning",
            None,
            &format!("{SAID}\n**Joel**, 9:00: Back.\n"),
        );
        assert_eq!(
            summed(&rejoined, "Planning", SUMMED, &read),
            None,
            "the call went on, so a summary of its first part is no summary of it"
        );
        assert!(is_one(&first), "{first}");
        assert!(
            !is_one(SAID),
            "a transcript from before has no Summary heading"
        );
    }

    #[test]
    fn a_call_not_yet_summed_up_says_it_has_no_summary_yet() {
        let file = whole("", None, SAID);
        assert!(
            file.starts_with(&format!("{SUMMARY}\n\n{PENDING}\n\n{TRANSCRIPT}\n\n")),
            "{file}"
        );
        assert_eq!(summary(&file), None);
        assert_eq!(
            transcript(&file),
            SAID.replacen("# Transcript", TRANSCRIPT, 1)
        );
    }

    #[test]
    fn a_call_summed_up_again_keeps_its_transcript_and_takes_the_new_summary() {
        let first = whole("Planning", None, SAID);
        let again = whole("Planning", Some(SUMMED), transcript(&first));
        assert_eq!(again, whole("Planning", Some(SUMMED), SAID), "{again}");
    }

    #[test]
    fn a_transcript_from_before_is_read_whole() {
        assert_eq!(transcript(SAID), SAID);
        assert_eq!(summary(SAID), None);
    }
}
