//! A call's transcript: what each side said, laid out in the order they
//! spoke, as Markdown.
//!
//! Joel's side is his microphone as it hears the room, before the call
//! app's echo cancelling, so on the laptop's speakers it hears the others
//! too. A line of his whose words repeat a line of theirs said at the same
//! time is their echo, and is left out; on a headset there's none. Where
//! each word's time is known, their words his microphone heard just after
//! they said them are cut from his line, and his own words in it stay.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use crate::mel::RATE;
use crate::parakeet::Word;

/// Who spoke a line: Joel, whose microphone it came from, or the others on
/// the call, whose sound it came from.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Side {
    Mine,
    Theirs,
}

/// A take heard: who said it, from when to when in the call, in samples,
/// and what; for their side, which of the voices told apart it is, from
/// 0, where it's one (voices.rs), and who that voice is, where who was on
/// the call says so for certain.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub side: Side,
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub voice: Option<usize>,
    pub name: Option<String>,
}

/// How near in time a line of Joel's must be to one of theirs to be its
/// echo: Whisper's takes are cut at each side's own pauses, so the same
/// words start a little apart.
const ECHO_SLACK: usize = 2 * RATE;

/// How many of a line's words must be in theirs for it to be their echo.
const ECHO_SHARE: f32 = 0.6;

/// How many words a line of Joel's, most of whose words are in theirs,
/// must share with it to be their echo however long their line is. One
/// sharing fewer, such as "Okay." or "Okay, that's fine.", is
/// as likely Joel answering them in their own words, so it's their echo
/// only when it holds most of their line's words too: a long line of
/// theirs heard through his speakers comes back as long takes.
const ECHO_WORDS: usize = 4;

/// How long after one of their words Joel's microphone may hear it, in
/// samples: the speakers' and the microphone's buffers, and a word timed
/// by the encoder frame it was written on, 80 ms each. Joel saying their
/// words back starts after they've finished saying them.
const ECHO_LAG: usize = RATE;

/// How long before one of their words its echo may be timed, in samples,
/// since each side's word is timed by its own frame.
const ECHO_LEAD: usize = RATE / 2;

/// How many of Joel's words in a row, each one of theirs heard just
/// after it, are their echo.
const ECHO_RUN: usize = 1;

/// How many of their words a word of their echo may follow the last by:
/// either side may hear a word the other doesn't.
const ECHO_GAP: usize = 2;

/// What the others on a call are called where nobody knows who they are.
pub const OTHERS: &str = "Others";

/// What a voice told apart is called, before its number from 1.
const VOICE: &str = "Voice ";

/// What the voice `voice`, from 0, is called in a transcript.
pub fn voice_called(voice: usize) -> String {
    format!("{VOICE}{}", voice + 1)
}

/// The voice, from 0, a speaker `markdown` wrote is, when it's one: Voice
/// and a number from 1, in digits alone, as it writes them, so a name such
/// as "Voice +1" or "Voice 01" isn't read as a voice.
fn voice_of(who: &str) -> Option<usize> {
    let number = who.strip_prefix(VOICE)?;
    if number.starts_with('0') || !number.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    number.parse::<usize>().ok()?.checked_sub(1)
}

/// Whether a speaker `markdown` wrote is their side unnamed, Others or a
/// voice told apart, so a reader never takes a voice for Joel.
pub fn unnamed(who: &str) -> bool {
    who == OTHERS || voice_of(who).is_some()
}

/// What the transcript says once it has voices, so a voice is never
/// taken for a person someone has named.
pub const VOICES_NOTE: &str = "Their side's voices are told apart by how they sound, and \
    numbered in the order they first spoke; who each one is isn't known. Lines no voice was \
    sure of, or heard after the service restarted during the call, stay under Others.";

/// What the transcript says once some of its voices are named, so a name
/// is read as only as sure as who was in the Meet.
pub const NAMED_NOTE: &str = "Their side's voices are told apart by how they sound. A voice \
    is called by name where only that person was in the Meet each time it spoke, and the rest \
    are numbered in the order they first spoke. Lines no voice was sure of, or heard after the \
    service restarted during the call, stay under Others. Where several people shared one \
    seat in the Meet, a name may be wrong.";

/// The transcript: lines in the order they were said, echoes left out, a
/// speaker's lines in a row joined into one paragraph under the time it
/// began, Joel's under `me` and theirs under `them`, or Others when who
/// they are isn't known, or is called the same as Joel, whose lines
/// wouldn't be told from theirs then. Where `them` isn't known, a line of
/// theirs with a voice is under Voice and its number, and the voices'
/// note says what that means. A `note` saying where their name came from,
/// or who was on the call, comes first, under the heading.
pub fn markdown(lines: &[Line], me: &str, them: Option<&str>, note: Option<&str>) -> String {
    markdown_naming(lines, me, them, note, None)
}

/// `markdown`, where a line of theirs whose voice is named is under its
/// name, unless that's Joel's or reads as unnamed, and `guesses`, saying
/// who the voices left numbered may be, follows the voices' note.
pub fn markdown_naming(
    lines: &[Line],
    me: &str,
    them: Option<&str>,
    note: Option<&str>,
    guesses: Option<&str>,
) -> String {
    let heard: Vec<Line> = lines
        .iter()
        .filter(|line| !echoes(line, lines))
        .cloned()
        .collect();
    markdown_unechoed(&heard, me, them, note, guesses)
}

/// `markdown_naming` of `lines` whose echoes are already left out, as
/// call-transcripts leaves them out against each take of theirs whole
/// where it splits one by voice.
pub fn markdown_unechoed(
    lines: &[Line],
    me: &str,
    them: Option<&str>,
    note: Option<&str>,
    guesses: Option<&str>,
) -> String {
    let named = them.filter(|them| *them != me);
    let mut heard: Vec<&Line> = lines.iter().collect();
    heard.sort_by_key(|line| line.start);
    let name_of = |line: &Line| {
        line.name
            .clone()
            .filter(|name| name != me && !name.is_empty() && !unnamed(name))
    };
    let voiced = named.is_none()
        && heard
            .iter()
            .any(|line| line.voice.is_some() || name_of(line).is_some());
    let mut paragraphs = vec!["# Transcript".to_owned()];
    paragraphs.extend(note.map(str::to_owned));
    if voiced {
        let some_called = heard.iter().any(|line| name_of(line).is_some());
        paragraphs.push(if some_called { NAMED_NOTE } else { VOICES_NOTE }.to_owned());
        paragraphs.extend(guesses.map(str::to_owned));
    }
    // The notes are no line, so a run of lines never joins onto them. A
    // run is one side under one speaker, so a line of theirs never joins
    // Joel's, even where he's called what one of them is.
    let mut last: Option<(Side, String)> = None;
    for line in heard.into_iter().filter(|line| !line.text.is_empty()) {
        let who = match (line.side, named, line.voice.filter(|_| voiced)) {
            (Side::Mine, ..) => me.to_owned(),
            (Side::Theirs, Some(them), _) => them.to_owned(),
            (Side::Theirs, None, voice) => match (name_of(line), voice) {
                (Some(name), _) => name,
                (None, Some(voice)) => voice_called(voice),
                (None, None) => OTHERS.to_owned(),
            },
        };
        match paragraphs.last_mut() {
            Some(paragraph) if last.as_ref() == Some(&(line.side, who.clone())) => {
                paragraph.push(' ');
                paragraph.push_str(&line.text);
            }
            _ => {
                let mut paragraph = String::new();
                let _ = write!(paragraph, "**{who}**, {}: {}", clock(line.start), line.text);
                paragraphs.push(paragraph);
            }
        }
        last = Some((line.side, who));
    }
    paragraphs.join("\n\n") + "\n"
}

/// The lines of a transcript `markdown` wrote, a paragraph each, so a
/// call taken up again after the daemon restarts goes on from them, a
/// line Joel's only when it's under `me`, one of theirs under a voice
/// read back with it, and one under a name with that name, so a voice
/// named before a restart stays named. Only when each began is kept,
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
            let side = if who == me { Side::Mine } else { Side::Theirs };
            let voice = voice_of(who).filter(|_| side == Side::Theirs);
            let name = (side == Side::Theirs && !unnamed(who)).then(|| who.to_owned());
            Some(Line {
                side,
                start: seconds * RATE,
                end: seconds * RATE,
                text: text.trim_end().to_owned(),
                voice,
                name,
            })
        })
        .collect()
}

/// Whether `line` is Joel's microphone hearing one of the others' lines,
/// and so left out of the transcript; the meetings check says which of
/// Joel's lines it leaves out (score.rs).
pub fn echoes(line: &Line, lines: &[Line]) -> bool {
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
                share(shared, &mine) >= ECHO_SHARE
                    && (shared >= ECHO_WORDS || share(shared, &said) >= ECHO_SHARE)
            }
    })
}

/// For each of Joel's `mine`, the one of `theirs`, in the order they were
/// said, it's the echo of, if it's one: a run of his words, each spelled
/// as one of theirs, in their order, that his microphone heard within a
/// moment of them saying it.
pub fn echoes_of(mine: &[Word], theirs: &[Word]) -> Vec<Option<usize>> {
    // Each of his words' runs: which of theirs it may echo, and how long
    // the run ending on it is.
    let mut runs: Vec<Vec<(usize, usize)>> = Vec::with_capacity(mine.len());
    for (i, word) in mine.iter().enumerate() {
        let spelling = spelled(&word.text);
        let from = theirs.partition_point(|said| said.at + ECHO_LAG < word.at);
        let to = theirs.partition_point(|said| said.at <= word.at + ECHO_LEAD);
        let before = i
            .checked_sub(1)
            .map_or(&[][..], |last| runs[last].as_slice());
        let heard = (from..to.max(from))
            .filter(|&j| !spelling.is_empty() && spelled(&theirs[j].text) == spelling)
            .map(|j| {
                let longest = before
                    .iter()
                    .filter(|&&(k, _)| k < j && j - k <= ECHO_GAP)
                    .map(|&(_, run)| run)
                    .max()
                    .unwrap_or(0);
                (j, longest + 1)
            })
            .collect();
        runs.push(heard);
    }
    // Walked back from each run's end, so a run long enough is all marked.
    let mut echoes = vec![None; mine.len()];
    for end in (0..mine.len()).rev() {
        if echoes[end].is_some() {
            continue;
        }
        let Some(&(mut j, mut run)) = runs[end].iter().max_by_key(|&&(_, run)| run) else {
            continue;
        };
        if run < ECHO_RUN {
            continue;
        }
        echoes[end] = Some(j);
        for i in (0..end).rev() {
            if run == 1 {
                break;
            }
            run -= 1;
            let Some(&(k, _)) = runs[i]
                .iter()
                .find(|&&(k, length)| length == run && k < j && j - k <= ECHO_GAP)
            else {
                break;
            };
            echoes[i] = Some(k);
            j = k;
        }
    }
    echoes
}

/// Some of Joel's words cut from a line of his as the others' echo: the
/// line's index, when the first was heard, how long after its word of
/// theirs, in samples, and what they say.
#[derive(Debug, PartialEq)]
pub struct Cut {
    pub line: usize,
    pub at: usize,
    pub after: isize,
    pub text: String,
}

/// `lines` with the others' echo cut from each of Joel's whose words
/// `words` holds, by the line's index, what's left tidied by `tidy`, and
/// what was cut. A line left with nothing has no text, and isn't
/// written, so the lines keep their indices.
pub fn cut_echoes(
    lines: &[Line],
    words: &HashMap<usize, Vec<Word>>,
    tidy: impl Fn(&str) -> Option<String>,
) -> (Vec<Line>, Vec<Cut>) {
    let mut theirs: Vec<Word> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.side == Side::Theirs)
        .filter_map(|(index, _)| words.get(&index))
        .flatten()
        .cloned()
        .collect();
    theirs.sort_by_key(|word| word.at);
    let mut cuts = Vec::new();
    let mut cut = lines.to_vec();
    for (index, line) in cut.iter_mut().enumerate() {
        let Some(mine) = words.get(&index).filter(|_| line.side == Side::Mine) else {
            continue;
        };
        let echoes = echoes_of(mine, &theirs);
        if echoes.iter().all(Option::is_none) {
            continue;
        }
        let mut kept = Vec::new();
        let mut run: Option<Cut> = None;
        for (word, echo) in mine.iter().zip(&echoes) {
            let Some(j) = *echo else {
                kept.push(word.text.as_str());
                cuts.extend(run.take());
                continue;
            };
            if let Some(cutting) = run.as_mut() {
                cutting.text.push(' ');
                cutting.text.push_str(&word.text);
            } else {
                run = Some(Cut {
                    line: index,
                    at: word.at,
                    after: later(word.at, theirs[j].at),
                    text: word.text.clone(),
                });
            }
        }
        cuts.extend(run);
        line.text = tidy(&kept.join(" ")).unwrap_or_default();
    }
    (cut, cuts)
}

/// How many samples `heard` comes after `said`, before it if negative.
fn later(heard: usize, said: usize) -> isize {
    let by = |from: usize, to: usize| isize::try_from(to - from).unwrap_or(isize::MAX);
    if heard >= said {
        by(said, heard)
    } else {
        -by(heard, said)
    }
}

/// The share of `of`'s words that `shared` of them are.
#[expect(clippy::cast_precision_loss, reason = "a line's words are few")]
fn share(shared: usize, of: &HashSet<String>) -> f32 {
    shared as f32 / of.len() as f32
}

/// A line's words, lowercased, without their punctuation.
fn words(text: &str) -> HashSet<String> {
    text.split_whitespace()
        .map(spelled)
        .filter(|word| !word.is_empty())
        .collect()
}

/// A word lowercased, without its punctuation, as its echo would be.
fn spelled(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_alphanumeric() || *c == '\'')
        .collect::<String>()
        .to_lowercase()
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
            voice: None,
            name: None,
        }
    }

    fn voiced(second: usize, voice: usize, text: &str) -> Line {
        Line {
            voice: Some(voice),
            ..line(Side::Theirs, second, text)
        }
    }

    #[test]
    fn voices_told_apart_are_numbered_said_to_be_no_names_and_read_back() {
        let lines = [
            voiced(0, 0, "Shall we start?"),
            voiced(3, 1, "Yes, go ahead."),
            voiced(6, 1, "I have the numbers."),
            line(Side::Theirs, 9, "Mm."),
            line(Side::Mine, 12, "Thanks."),
        ];
        let want = [
            "# Transcript",
            "",
            "Invited: Ada, Bo and Cy.",
            "",
            VOICES_NOTE,
            "",
            "**Voice 1**, 0:00: Shall we start?",
            "",
            "**Voice 2**, 0:03: Yes, go ahead. I have the numbers.",
            "",
            "**Others**, 0:09: Mm.",
            "",
            "**Joel**, 0:12: Thanks.",
            "",
        ];
        let written = markdown(&lines, "Joel", None, Some("Invited: Ada, Bo and Cy."));
        assert_eq!(
            written,
            want.join("\n"),
            "the voices weren't laid out as told apart"
        );
        let back: Vec<Option<usize>> = read(&written, "Joel").iter().map(|l| l.voice).collect();
        assert_eq!(
            back,
            [Some(0), Some(1), None, None],
            "the voices didn't read back as they were written"
        );
        assert_eq!(
            markdown(
                &read(&written, "Joel"),
                "Joel",
                None,
                Some("Invited: Ada, Bo and Cy.")
            ),
            written,
            "a transcript with voices didn't write again as it was"
        );
    }

    fn named(second: usize, voice: usize, name: &str, text: &str) -> Line {
        Line {
            name: Some(name.to_owned()),
            ..voiced(second, voice, text)
        }
    }

    #[test]
    fn a_voice_named_for_certain_is_under_its_name_and_reads_back_named() {
        let lines = [
            named(0, 0, "Ada", "Shall we start?"),
            voiced(3, 1, "Yes, go ahead."),
            named(6, 0, "Ada", "Good."),
            line(Side::Mine, 9, "Thanks."),
        ];
        let guesses = "Voice 2 may be Bo or Cy.";
        let want = [
            "# Transcript",
            "",
            "In the Meet: Ada, Bo and Cy.",
            "",
            NAMED_NOTE,
            "",
            guesses,
            "",
            "**Ada**, 0:00: Shall we start?",
            "",
            "**Voice 2**, 0:03: Yes, go ahead.",
            "",
            "**Ada**, 0:06: Good.",
            "",
            "**Joel**, 0:09: Thanks.",
            "",
        ];
        let note = Some("In the Meet: Ada, Bo and Cy.");
        let written = markdown_naming(&lines, "Joel", None, note, Some(guesses));
        assert_eq!(
            written,
            want.join("\n"),
            "a named voice wasn't under its name"
        );
        let back = read(&written, "Joel");
        let names: Vec<Option<&str>> = back.iter().map(|l| l.name.as_deref()).collect();
        assert_eq!(
            names,
            [Some("Ada"), None, Some("Ada"), None],
            "a named voice didn't read back with its name"
        );
        assert_eq!(
            markdown_naming(&back, "Joel", None, note, Some(guesses)),
            written,
            "a transcript with names didn't write again as it was"
        );
    }

    #[test]
    fn a_transcript_whose_every_voice_is_named_writes_again_with_its_notes() {
        let lines = [
            named(0, 0, "Ada", "Shall we start?"),
            named(3, 1, "Bo", "Yes, go ahead."),
        ];
        let guesses = Some("Voice 3 may be Cy.");
        let written = markdown_naming(&lines, "Joel", None, None, guesses);
        assert!(
            written.contains(NAMED_NOTE) && written.contains("Voice 3 may be Cy."),
            "a transcript naming every voice lost the note or the guesses: {written}"
        );
        let back = read(&written, "Joel");
        assert_eq!(
            markdown_naming(&back, "Joel", None, None, guesses),
            written,
            "read back, names but no voice numbers lost the note on writing again"
        );
    }

    #[test]
    fn a_voice_named_what_joel_is_or_what_reads_as_unnamed_keeps_its_number() {
        for name in ["Joel", "Others", "Voice 3", ""] {
            let written = markdown(&[named(0, 0, name, "Hello.")], "Joel", None, None);
            assert!(
                written.contains("**Voice 1**, 0:00") && written.contains(VOICES_NOTE),
                "a voice named {name:?} wasn't kept under its number: {written}"
            );
        }
    }

    #[test]
    fn a_side_named_for_certain_is_named_whatever_its_voices() {
        let lines = [voiced(0, 0, "Hello."), voiced(3, 1, "Hi.")];
        let written = markdown(&lines, "Joel", Some("Ada"), None);
        assert!(
            written.contains("**Ada**, 0:00: Hello. Hi.") && !written.contains(VOICES_NOTE),
            "a side named for certain was split into voices: {written}"
        );
    }

    #[test]
    fn joel_saying_voice_is_still_joel() {
        let written = markdown(&[line(Side::Mine, 0, "Hello.")], "Voice 1", None, None);
        assert_eq!(
            read(&written, "Voice 1")[0].voice,
            None,
            "Joel's own line was read back as a voice of theirs"
        );
    }

    #[test]
    fn a_line_of_theirs_never_joins_joels_whatever_they_are_called() {
        for (me, theirs) in [
            ("Voice 1", voiced(5, 0, "Yes.")),
            ("Others", line(Side::Theirs, 5, "Yes.")),
        ] {
            let written = markdown(&[line(Side::Mine, 0, "Hello."), theirs], me, None, None);
            let paragraphs = written.lines().filter(|l| l.starts_with("**")).count();
            assert_eq!(
                paragraphs, 2,
                "with Joel called {me}, their line joined his paragraph: {written}"
            );
        }
    }

    #[test]
    fn only_a_voice_as_markdown_writes_one_is_read_as_one() {
        for name in ["Voice +1", "Voice 01", "Voice 0", "Voice ", "Voice 1a"] {
            assert!(!unnamed(name), "{name:?} was taken for a voice told apart");
        }
        assert!(
            unnamed("Voice 12"),
            "Voice 12 wasn't taken for a voice told apart"
        );
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

    /// Joel's lines that `markdown` keeps, among `lines`.
    fn kept(lines: &[Line]) -> Vec<String> {
        let written = markdown(lines, "Joel", None, None);
        read(&written, "Joel")
            .into_iter()
            .filter(|line| line.side == Side::Mine)
            .map(|line| line.text)
            .collect()
    }

    #[test]
    fn lines_whose_echoes_are_left_out_are_written_as_they_are() {
        let lines = [
            line(Side::Theirs, 10, "Not yet."),
            line(Side::Mine, 11, "Not yet."),
        ];
        assert!(
            kept(&lines).is_empty(),
            "this case's line of Joel's isn't an echo"
        );
        let written = markdown_unechoed(&lines, "Joel", None, None, None);
        let joels: Vec<String> = read(&written, "Joel")
            .into_iter()
            .filter(|line| line.side == Side::Mine)
            .map(|line| line.text)
            .collect();
        assert_eq!(
            joels,
            ["Not yet."],
            "a line already judged was judged again"
        );
    }

    #[test]
    fn joels_short_answer_to_a_longer_line_of_theirs_is_his_own() {
        // On the meetings, which hold no echo, each line of Joel's the
        // filter left out was a short answer like these, its few words
        // all in the longer line of theirs it answered.
        let theirs = "Okay, that's fine, yes, but let's check the budget.";
        for answer in ["Okay.", "Yes. Yes.", "Okay. That's fine."] {
            let lines = [line(Side::Theirs, 4, theirs), line(Side::Mine, 5, answer)];
            assert_eq!(
                kept(&lines),
                [answer],
                "Joel's answer {answer:?} was taken for their echo"
            );
        }
    }

    #[test]
    fn joels_long_line_taking_up_their_words_is_his_own() {
        // Four of their five words are his, but under half of his.
        let mine = "Thursday morning works for me, but not for my team at all.";
        let lines = [
            line(Side::Theirs, 4, "Thursday morning works for everyone?"),
            line(Side::Mine, 5, mine),
        ];
        assert_eq!(
            kept(&lines),
            [mine],
            "Joel's long line was taken for their echo"
        );
    }

    #[test]
    fn an_echo_of_a_short_line_of_theirs_is_still_left_out() {
        let lines = [
            line(Side::Theirs, 4, "Sounds good."),
            line(Side::Mine, 4, "Sounds good"),
        ];
        assert!(
            kept(&lines).is_empty(),
            "the echo of their short line was kept as Joel's"
        );
    }

    #[test]
    fn four_words_of_a_longer_line_of_theirs_are_its_echo() {
        // Joel's microphone hears their long line as two takes where it
        // cuts at its own pauses, and each is the echo of part of it.
        let theirs = "Thursday morning works for everyone, then lunch at noon.";
        let lines = [
            line(Side::Theirs, 4, theirs),
            line(Side::Mine, 4, "Thursday morning works for"),
            line(Side::Mine, 6, "everyone then lunch at noon"),
        ];
        assert!(
            kept(&lines).is_empty(),
            "a part of their long line was kept as Joel's: {:?}",
            kept(&lines)
        );
    }

    /// `text`'s words, the first heard `from` milliseconds into the call
    /// and each after 300 ms on.
    fn timed(text: &str, from: usize) -> Vec<Word> {
        text.split_whitespace()
            .enumerate()
            .map(|(i, word)| Word {
                text: word.to_owned(),
                at: (from + 300 * i) * RATE / 1000,
            })
            .collect()
    }

    /// A line of `side`'s holding `words`, from its first word to 300 ms
    /// after its last.
    fn worded(side: Side, words: &[Word]) -> Line {
        let joined: Vec<&str> = words.iter().map(|word| word.text.as_str()).collect();
        Line {
            side,
            start: words[0].at,
            end: words[words.len() - 1].at + RATE * 3 / 10,
            text: joined.join(" "),
            voice: None,
            name: None,
        }
    }

    /// Joel's lines as written once their echo is cut from them, each
    /// side's words given as they were heard.
    fn cut_from(theirs: &[Vec<Word>], mine: &[Vec<Word>]) -> Vec<String> {
        let mut lines = Vec::new();
        let mut words = HashMap::new();
        for (side, said) in [(Side::Theirs, theirs), (Side::Mine, mine)] {
            for heard in said {
                words.insert(lines.len(), heard.clone());
                lines.push(worded(side, heard));
            }
        }
        let (cut, _) = cut_echoes(&lines, &words, |left| Some(left.to_owned()));
        kept(&cut)
    }

    #[test]
    fn their_words_heard_just_after_them_are_cut_from_joels_line() {
        // As Joel's microphone heard a meeting, 50 ms after the far end
        // said it, with his own words before it in one take.
        let theirs = timed(
            "Please please leave me a space at the bottom, I'm little.",
            3_000,
        );
        let mut mine = timed("Uh yeah, okay. Sorry.", 1_800);
        mine.extend(timed(
            "Please please leave me a space at the bottom.",
            3_050,
        ));
        assert_eq!(
            cut_from(&[theirs], &[mine]),
            ["Uh yeah, okay. Sorry."],
            "their echo wasn't cut from Joel's line, or his own words went with it"
        );
    }

    #[test]
    fn joels_short_answer_heard_with_their_echo_is_his_own() {
        // His answer is in their line too, but said before they said it.
        let theirs = timed("Yes, we could use a seagull, yes.", 2_000);
        let mut mine = timed("Yes yes.", 1_000);
        mine.extend(timed("we could use a seagull, yes.", 2_350));
        assert_eq!(
            cut_from(&[theirs], &[mine]),
            ["Yes yes."],
            "Joel's answer went with their echo"
        );
    }

    #[test]
    fn joel_saying_their_words_back_after_them_is_his_own() {
        // They say "a seagull" from 2 s; Joel says it back from 3.5 s.
        let theirs = timed("Could it be a seagull?", 1_100);
        let mine = timed("A seagull, right.", 3_500);
        assert_eq!(
            cut_from(&[theirs], &[mine]),
            ["A seagull, right."],
            "Joel's own words were cut as their echo"
        );
    }

    #[test]
    fn a_line_of_joels_all_their_echo_is_left_out() {
        let theirs = timed("Flying around everywhere.", 8_000);
        let mine = timed("Flying around everywhere.", 8_080);
        assert!(
            cut_from(&[theirs], &[mine]).is_empty(),
            "a line of Joel's that's all their echo was written"
        );
    }

    #[test]
    fn what_was_cut_says_where_it_was_and_how_late() {
        let theirs = timed("Lunch at noon", 5_000);
        let mine = timed("Fine. Lunch at noon", 4_700);
        let lines = [worded(Side::Theirs, &theirs), worded(Side::Mine, &mine)];
        let words = HashMap::from([(0, theirs), (1, mine)]);
        let (cut, cuts) = cut_echoes(&lines, &words, |left| Some(left.to_owned()));
        assert_eq!(cut[1].text, "Fine.");
        assert_eq!(
            cuts,
            [Cut {
                line: 1,
                at: 5_000 * RATE / 1000,
                after: 0,
                text: "Lunch at noon".to_owned(),
            }]
        );
    }

    #[test]
    fn a_line_whose_words_are_unknown_is_cut_from_by_its_text_alone() {
        // Lines read back after a restart have no words; the whole-line
        // rule still leaves their echo out where the transcript's written.
        let lines = [
            line(Side::Theirs, 4, "Sounds good."),
            line(Side::Mine, 4, "Sounds good"),
        ];
        let (cut, cuts) = cut_echoes(&lines, &HashMap::new(), |left| Some(left.to_owned()));
        assert_eq!(cut, lines);
        assert!(cuts.is_empty());
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
