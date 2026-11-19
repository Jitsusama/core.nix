//! A call's transcript: what each side said, laid out in the order they
//! spoke, as Markdown.
//!
//! Joel's side is his microphone as it hears the room, before the call
//! app's echo cancelling, so on the laptop's speakers it hears the others
//! too. A line of his whose words repeat a line of theirs said at the same
//! time is their echo, and is left out; on a headset there's none.

use std::collections::HashSet;
use std::fmt::Write as _;

use crate::mel::RATE;

/// Who spoke a line: Joel, whose microphone it came from, or the others on
/// the call, whose sound it came from.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Side {
    Mine,
    Theirs,
}

/// A take heard: who said it, from when to when in the call, in samples,
/// and what.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub side: Side,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// How near in time a line of Joel's must be to one of theirs to be its
/// echo: Whisper's takes are cut at each side's own pauses, so the same
/// words start a little apart.
const ECHO_SLACK: usize = 2 * RATE;

/// How many of a line's words must be in theirs for it to be their echo.
const ECHO_SHARE: f32 = 0.6;

/// What the others on a call are called where nobody knows who they are.
pub const OTHERS: &str = "Others";

/// The transcript: lines in the order they were said, echoes left out, a
/// speaker's lines in a row joined into one paragraph under the time it
/// began, Joel's under `me` and theirs under `them`, or Others when who
/// they are isn't known, or is called the same as Joel, whose lines
/// wouldn't be told from theirs then. A `note` saying where their name
/// came from, or who was on the call, comes first, under the heading.
pub fn markdown(lines: &[Line], me: &str, them: Option<&str>, note: Option<&str>) -> String {
    let them = them.filter(|them| *them != me).unwrap_or(OTHERS);
    let mut heard: Vec<&Line> = lines.iter().filter(|line| !echoes(line, lines)).collect();
    heard.sort_by_key(|line| line.start);
    let mut paragraphs = vec!["# Transcript".to_owned()];
    paragraphs.extend(note.map(str::to_owned));
    // The note is no line, so a run of lines never joins onto it.
    let mut last = None;
    for line in heard {
        match paragraphs.last_mut() {
            Some(paragraph) if last == Some(line.side) => {
                paragraph.push(' ');
                paragraph.push_str(&line.text);
            }
            _ => {
                let who = match line.side {
                    Side::Mine => me,
                    Side::Theirs => them,
                };
                let mut paragraph = String::new();
                let _ = write!(paragraph, "**{who}**, {}: {}", clock(line.start), line.text);
                paragraphs.push(paragraph);
            }
        }
        last = Some(line.side);
    }
    paragraphs.join("\n\n") + "\n"
}

/// The lines of a transcript `markdown` wrote, a paragraph each, so a
/// call taken up again after the daemon restarts goes on from them, a
/// line Joel's only when it's under `me`. Only when each began is kept,
/// which is all the layout needs.
pub fn read(transcript: &str, me: &str) -> Vec<Line> {
    transcript
        .split("\n\n")
        .filter_map(|paragraph| {
            let (who, rest) = paragraph.strip_prefix("**")?.split_once("**, ")?;
            let (time, text) = rest.split_once(": ")?;
            let seconds = time
                .split(':')
                .try_fold(0, |sum, part| part.parse::<usize>().map(|n| sum * 60 + n))
                .ok()?;
            Some(Line {
                side: if who == me { Side::Mine } else { Side::Theirs },
                start: seconds * RATE,
                end: seconds * RATE,
                text: text.trim_end().to_owned(),
            })
        })
        .collect()
}

/// Whether `line` is Joel's microphone hearing one of the others' lines.
fn echoes(line: &Line, lines: &[Line]) -> bool {
    if line.side != Side::Mine {
        return false;
    }
    let mine = words(&line.text);
    if mine.is_empty() {
        return false;
    }
    lines.iter().any(|theirs| {
        theirs.side == Side::Theirs
            && theirs.start <= line.end + ECHO_SLACK
            && line.start <= theirs.end + ECHO_SLACK
            && {
                let said = words(&theirs.text);
                let shared = mine.intersection(&said).count();
                #[expect(clippy::cast_precision_loss, reason = "a line's words are few")]
                let share = shared as f32 / mine.len() as f32;
                share >= ECHO_SHARE
            }
    })
}

/// A line's words, lowercased, without their punctuation.
fn words(text: &str) -> HashSet<String> {
    text.split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|c| c.is_alphanumeric() || *c == '\'')
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect()
}

/// Where in the call a sample falls, as minutes and seconds, with hours
/// once there are any.
pub fn clock(sample: usize) -> String {
    let seconds = sample / RATE;
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(side: Side, second: usize, text: &str) -> Line {
        Line {
            side,
            start: second * RATE,
            end: (second + 2) * RATE,
            text: text.to_owned(),
        }
    }

    #[test]
    fn the_sides_are_laid_out_in_the_order_they_spoke_and_a_run_is_one_paragraph() {
        let lines = [
            line(Side::Theirs, 4, "Thursday works."),
            line(Side::Mine, 0, "Can we move it?"),
            line(Side::Mine, 9, "Great."),
            line(Side::Mine, 11, "Thanks."),
        ];
        let want = [
            "# Transcript",
            "",
            "**Joel**, 0:00: Can we move it?",
            "",
            "**Others**, 0:04: Thursday works.",
            "",
            "**Joel**, 0:09: Great. Thanks.",
            "",
        ];
        assert_eq!(markdown(&lines, "Joel", None, None), want.join("\n"));
    }

    #[test]
    fn their_side_is_written_under_their_name_once_its_known_and_read_back_as_theirs() {
        let lines = [
            line(Side::Mine, 0, "Can we move it?"),
            line(Side::Theirs, 4, "Thursday works."),
        ];
        let note = "Their side is Ada Lovelace, the one other person invited.";
        let want = [
            "# Transcript",
            "",
            note,
            "",
            "**Joel**, 0:00: Can we move it?",
            "",
            "**Ada Lovelace**, 0:04: Thursday works.",
            "",
        ];
        let written = markdown(&lines, "Joel", Some("Ada Lovelace"), Some(note));
        assert_eq!(written, want.join("\n"));
        let back = read(&written, "Joel");
        let sides: Vec<Side> = back.iter().map(|line| line.side).collect();
        assert_eq!(sides, [Side::Mine, Side::Theirs], "the note is no line");
    }

    #[test]
    fn someone_called_what_joel_is_called_is_written_as_others() {
        let lines = [line(Side::Theirs, 4, "Thursday works.")];
        let written = markdown(&lines, "Joel", Some("Joel"), None);
        assert!(written.contains("**Others**, 0:04"), "{written}");
        assert_eq!(read(&written, "Joel")[0].side, Side::Theirs);
    }

    #[test]
    fn joels_microphone_hearing_the_others_is_left_out_but_his_own_words_stay() {
        let lines = [
            line(Side::Theirs, 4, "Thursday morning works for everyone."),
            line(Side::Mine, 5, "Thursday morning works for everyone"),
            line(Side::Mine, 30, "Thursday morning works for everyone."),
            line(Side::Mine, 6, "Great, thank you."),
        ];
        let want = [
            "# Transcript",
            "",
            "**Others**, 0:04: Thursday morning works for everyone.",
            "",
            "**Joel**, 0:06: Great, thank you. Thursday morning works for everyone.",
            "",
        ];
        assert_eq!(markdown(&lines, "Joel", None, None), want.join("\n"));
    }

    #[test]
    fn a_transcript_reads_back_as_the_lines_it_was_written_from() {
        let lines = [
            line(Side::Mine, 0, "Can we move it?"),
            line(Side::Theirs, 3_725, "Thursday works: mornings."),
        ];
        let written = markdown(&lines, "Joel", None, None);
        let back = read(&written, "Joel");
        assert_eq!(back.len(), 2);
        assert_eq!(back[1].side, Side::Theirs);
        assert_eq!(back[1].start, 3_725 * RATE);
        assert_eq!(back[1].text, "Thursday works: mornings.");
        assert_eq!(markdown(&back, "Joel", None, None), written);
    }

    #[test]
    fn the_clock_gains_hours_only_once_there_are_some() {
        assert_eq!(clock(65 * RATE), "1:05");
        assert_eq!(clock(3_725 * RATE), "1:02:05");
    }
}
