//! The others' echo on Joel's side of a call, left out of its transcript.
//!
//! Joel's side is his microphone as it hears the room, before the call
//! app's echo cancelling, so on the laptop's speakers it hears the others
//! too. Where each word's time is known, their words his microphone heard
//! just after they said them are cut from his line, and his own words in
//! it stay. A line of his whose words repeat a line of theirs said at the
//! same time is their echo, and is left out whole, which also covers a
//! line read back from a transcript, whose words' times are lost. On a
//! headset there's none of either.

use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;

use crate::mel::RATE;
use crate::parakeet::Word;
use crate::transcript::{Line, Side};

/// How near in time a line of Joel's must be to one of theirs to be its
/// echo: Whisper's takes are cut at each side's own pauses, so the same
/// words start a little apart.
const SLACK: usize = 2 * RATE;

/// How many of a line's words must be in theirs for it to be their echo.
const SHARE: f32 = 0.6;

/// How many words a line of Joel's, most of whose words are in theirs,
/// must share with it to be their echo however long their line is. One
/// sharing fewer, such as "Okay." or "Okay, that's fine.", is
/// as likely Joel answering them in their own words, so it's their echo
/// only when it holds most of their line's words too: a long line of
/// theirs heard through his speakers comes back as long takes.
const WORDS: usize = 4;

/// How long after one of their words Joel's microphone may hear it, in
/// samples: the speakers' and the microphone's buffers, and a word timed
/// by the encoder frame it was written on, 80 ms each. Joel saying their
/// words back starts after they've finished saying them.
const LAG: usize = RATE;

/// How long before one of their words its echo may be timed, in samples,
/// since each side's word is timed by its own frame.
const LEAD: usize = RATE / 2;

/// How many of Joel's words in a row, each one of theirs heard just
/// after it, are their echo.
const RUN: usize = 1;

/// How many of their words a word of their echo may follow the last by:
/// either side may hear a word the other doesn't.
const GAP: usize = 2;

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
            && theirs.start <= line.end + SLACK
            && line.start <= theirs.end + SLACK
            && {
                let said = words(&theirs.text);
                let shared = mine.intersection(&said).count();
                share(shared, &mine) >= SHARE && (shared >= WORDS || share(shared, &said) >= SHARE)
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
        let from = theirs.partition_point(|said| said.at + LAG < word.at);
        let to = theirs.partition_point(|said| said.at <= word.at + LEAD);
        let before = i
            .checked_sub(1)
            .map_or(&[][..], |last| runs[last].as_slice());
        let heard = (from..to.max(from))
            .filter(|&j| !spelling.is_empty() && spelled(&theirs[j].text) == spelling)
            .map(|j| {
                let longest = before
                    .iter()
                    .filter(|&&(k, _)| k < j && j - k <= GAP)
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
        if run < RUN {
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
                .find(|&&(k, length)| length == run && k < j && j - k <= GAP)
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
pub fn cut_echoes<S: BuildHasher>(
    lines: &[Line],
    words: &HashMap<usize, Vec<Word>, S>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::{markdown, read};

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
        assert_eq!(
            cuts,
            Vec::<Cut>::new(),
            "words were cut from a line whose words are unknown"
        );
        assert!(
            kept(&cut).is_empty(),
            "the whole-line rule didn't leave the echo out"
        );
    }
}
