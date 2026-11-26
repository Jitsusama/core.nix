//! The others' echo on Joel's side of a call, left out of its transcript.
//!
//! Joel's side is his microphone as it hears the room, before the call
//! app's echo cancelling, so on the laptop's speakers it hears the others
//! too. Where each word's time is known, their words his microphone heard
//! just after they said them, and quieter than they said them, are cut
//! from his line, as are words of his heard inside their speech far
//! quieter than it and than his own voice, however spelled. His own
//! words in it stay: their microphones pick up his voice too, far
//! quieter than their own, and that's his. A line of his whose words
//! repeat a line of theirs said at the same time is their echo, and is
//! left out whole, which also covers a line read back from a transcript,
//! whose words' times are lost. On a headset there's none of either.

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
/// samples. On AMI's meetings with their side mixed into his, echo landed
/// within 400 ms of their word; a live call's two recorders may also sit
/// up to half a second apart, since a side is only filled with silence
/// once it falls further behind than that (follow.rs). The cost is that
/// Joel saying two of their words back within a second of each, no louder
/// than they said them, is cut as their echo.
const LAG: usize = RATE;

/// How long before one of their words its echo may be timed, in samples,
/// since each side's word is timed by its own frame: on AMI's meetings
/// with their side mixed into his, echo landed from 300 ms before their
/// word to 400 ms after, and his own words said just before theirs
/// were cut when this was half a second.
const LEAD: usize = RATE * 3 / 10;

/// How many of Joel's words in a row, each one of theirs heard just
/// after it, are their echo: a lone word of theirs, such as "Okay", is
/// as likely his own.
const RUN: usize = 2;

/// How many words on either side a word of their echo may follow the
/// last by: either side may hear a word the other doesn't.
const GAP: usize = 2;

/// How far below the loudest of their words said around it, in dB, a
/// word of Joel's said inside their speech is their echo however it's
/// spelled: each side's model hears the echo its own way, so on the
/// echoed meetings most of the echo left on his side spelled none of
/// their words. Echo comes back 15 to 35 dB down, while his own voice
/// in his microphone is near theirs in theirs.
const DROWNED: f32 = 15.0;

/// How far below his own voice, in dB, a word of Joel's must also be to
/// be cut for how far below theirs it is: each side's level is its own
/// recorder's, so a quiet microphone or a loud call app can put all his
/// voice `DROWNED` below theirs, while echo is quieter in his microphone
/// than his voice is. On the echoed meetings echo is mixed 20 dB down.
const QUIETER: f32 = 12.0;

/// How many of Joel's words, heard while none of theirs is, it takes to
/// know his voice's level: with fewer, a word is cut for its level
/// against theirs alone, as where he says little his side is mostly their
/// echo.
const ALONE: usize = 5;

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

/// Whether Joel's `heard` may be the echo of their `said` by how loud
/// each is: an echo is quieter in his microphone than they said it, while
/// their microphones pick up his voice quieter than his does. On AMI's
/// meetings, echo mixed in 20 or 30 dB down was 15 to 35 dB louder in
/// theirs, and his voice in theirs 5 to 50 dB quieter, so even is the
/// line. Judged against each side's usual level instead, echo was kept
/// where he says little, since his side's usual level is then the echo's.
fn may_echo(heard: &Word, said: &Word) -> bool {
    said.level >= heard.level
}

/// For each of Joel's `mine`, the one of `theirs`, in the order they were
/// said, it's the echo of, if it's one: a run of his words, each spelled
/// as one of theirs, in their order, that his microphone heard within a
/// moment of them saying it, and quieter. A run may pass over a word
/// either side heard and the other didn't, and one his side heard alone
/// is kept as his.
fn echoes_of(mine: &[Word], theirs: &[Word]) -> Vec<Option<usize>> {
    // Each of his words' runs: which of theirs it may echo, and how long
    // the run ending on it is.
    let mut runs: Vec<Vec<(usize, usize)>> = Vec::with_capacity(mine.len());
    for (i, word) in mine.iter().enumerate() {
        let spelling = spelled(&word.text);
        let from = theirs.partition_point(|said| said.at + LAG < word.at);
        let to = theirs.partition_point(|said| said.at <= word.at + LEAD);
        let before = &runs[i.saturating_sub(GAP)..i];
        let heard = (from..to.max(from))
            .filter(|&j| {
                !spelling.is_empty()
                    && spelled(&theirs[j].text) == spelling
                    && may_echo(word, &theirs[j])
            })
            .map(|j| {
                let longest = before
                    .iter()
                    .flatten()
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
        let mut i = end;
        while run > 1 {
            run -= 1;
            let Some((h, k)) = (i.saturating_sub(GAP)..i).rev().find_map(|h| {
                runs[h]
                    .iter()
                    .find(|&&(k, length)| length == run && k < j && j - k <= GAP)
                    .map(|&(k, _)| (h, k))
            }) else {
                break;
            };
            echoes[h] = Some(k);
            (i, j) = (h, k);
        }
    }
    echoes
}

/// The indices of `theirs` said within a moment of `word`, as its echo
/// would be.
fn around(word: &Word, theirs: &[Word]) -> std::ops::Range<usize> {
    let from = theirs.partition_point(|said| said.at + LAG < word.at);
    let to = theirs.partition_point(|said| said.at <= word.at + LEAD);
    from..to.max(from)
}

/// The level of Joel's voice in his microphone: the middle level of his
/// `mine` heard while none of `theirs` is, if `ALONE` of them are.
fn voice<'a>(mine: impl IntoIterator<Item = &'a Word>, theirs: &[Word]) -> Option<f32> {
    let mut levels: Vec<f32> = mine
        .into_iter()
        .filter(|word| around(word, theirs).is_empty())
        .map(|word| word.level)
        .collect();
    if levels.len() < ALONE {
        return None;
    }
    levels.sort_by(f32::total_cmp);
    Some(levels[levels.len() / 2])
}

/// For each of Joel's `mine`, the loudest of `theirs` said within a
/// moment of it, if that's `DROWNED` louder, his `voice`, once known, is
/// `QUIETER` louder, and they go on speaking past it, or his word just before it
/// was cut so: his words heard inside their speech far below it are its
/// echo, and so is the echo of their last words, which follows them,
/// while a word of his that opens as their line ends may be his answer.
fn drowned(mine: &[Word], theirs: &[Word], voice: Option<f32>) -> Vec<Option<usize>> {
    let mut drowned: Vec<Option<usize>> = Vec::with_capacity(mine.len());
    for (i, word) in mine.iter().enumerate() {
        let near = around(word, theirs);
        let on = theirs[near.end..]
            .first()
            .is_some_and(|said| said.at <= word.at + LAG);
        let echoing = i > 0 && drowned[i - 1].is_some() && mine[i - 1].at + LAG >= word.at;
        let quiet = voice.is_none_or(|voice| word.level + QUIETER <= voice);
        let loudest = near.max_by(|&a, &b| theirs[a].level.total_cmp(&theirs[b].level));
        let below = |j: &usize| theirs[*j].level >= word.level + DROWNED;
        drowned.push(loudest.filter(|j| (on || echoing) && quiet && below(j)));
    }
    drowned
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
    pub below: Option<f32>,
}

/// `lines` with the others' echo cut from each of Joel's whose words
/// `words` holds, by the line's index, what's left tidied by `tidy`, and
/// what was cut. A line whose opening words are cut starts at the first
/// word kept, so his answer to what they said is written after it. A
/// line left with nothing has no text, and isn't written, so the lines
/// keep their indices.
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
    let his = voice(
        lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.side == Side::Mine)
            .filter_map(|(index, _)| words.get(&index))
            .flatten(),
        &theirs,
    );
    let mut cuts = Vec::new();
    let mut cut = lines.to_vec();
    for (index, line) in cut.iter_mut().enumerate() {
        let Some(mine) = words.get(&index).filter(|_| line.side == Side::Mine) else {
            continue;
        };
        let echoes: Vec<Option<usize>> = echoes_of(mine, &theirs)
            .into_iter()
            .zip(drowned(mine, &theirs, his))
            .map(|(by_spelling, by_level)| by_spelling.or(by_level))
            .collect();
        if echoes.iter().all(Option::is_none) {
            continue;
        }
        let mut kept = Vec::new();
        let mut first = None;
        let mut run: Option<Cut> = None;
        for (word, echo) in mine.iter().zip(&echoes) {
            let Some(j) = *echo else {
                kept.push(word.text.as_str());
                first = first.or(Some(word.at));
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
                    below: None,
                });
            }
        }
        cuts.extend(run);
        if echoes[0].is_some()
            && let Some(at) = first
        {
            line.start = at;
        }
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
    use crate::transcript::tests::{kept, line};

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
                level: 0.0,
            })
            .collect()
    }

    /// `words` as heard at `level` dB.
    fn at_level(mut words: Vec<Word>, level: f32) -> Vec<Word> {
        for word in &mut words {
            word.level = level;
        }
        words
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
    fn joels_words_the_far_ends_microphone_picked_up_are_his_own() {
        // On AMI's meetings the others' headsets pick up Joel's voice as he
        // says it, far quieter than their own: this is his, not their echo.
        let theirs = [
            at_level(timed("So what should the remote do first then?", 0), -20.0),
            at_level(timed("control all your household", 5_040), -45.0),
        ];
        let mine = "I think it should control all your household devices.";
        assert_eq!(
            cut_from(&theirs, &[at_level(timed(mine, 3_800), -20.0)]),
            [mine],
            "Joel's words their microphone picked up were cut as their echo"
        );
    }

    #[test]
    fn their_echo_quieter_in_joels_microphone_than_they_said_it_is_cut() {
        let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
        let own = "Fine by me, I'll book the room for us.";
        let mut mine = at_level(timed(own, 1_000), -20.0);
        mine.extend(at_level(
            timed("Lunch at noon on Thursday then.", 5_050),
            -40.0,
        ));
        assert_eq!(
            cut_from(&[theirs], &[mine]),
            [own],
            "their echo, quieter in his microphone than in theirs, wasn't cut"
        );
    }

    #[test]
    fn their_quieter_words_echoed_are_cut_however_little_joel_says() {
        // His side mostly their echo, as TS3003a's is, so his usual level
        // is the echo's: their quieter words, echoed with his microphone's
        // hiss, are no quieter against it than they said them against theirs.
        let theirs = [
            at_level(timed("We could meet next week if that suits", 0), -20.0),
            at_level(timed("Lunch at noon on Thursday then.", 5_000), -30.0),
        ];
        let own = "Sure, I'll put it in the calendar for us all.";
        let mut mine = at_level(timed(own, 2_000), -40.0);
        mine.extend(at_level(
            timed("Lunch at noon on Thursday then.", 5_050),
            -45.0,
        ));
        let echo = at_level(timed("We could meet next week if that suits", 50), -40.0);
        assert_eq!(
            cut_from(&theirs, &[echo, mine]),
            [own],
            "their echo, quieter in his microphone than they said it, wasn't cut"
        );
    }

    #[test]
    fn their_echo_misheard_far_below_what_they_said_is_cut() {
        // Each side's model hears the echo its own way, so his words for
        // it may spell none of theirs; far quieter than they said them,
        // as echo comes back 15 to 35 dB down, they're still theirs.
        let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
        let own = "Fine by me, I'll book the room for us.";
        let mut mine = at_level(timed(own, 1_000), -20.0);
        mine.extend(at_level(timed("Bunch had moon in Tuesday", 5_050), -42.0));
        assert_eq!(
            cut_from(&[theirs], &[mine]),
            [own],
            "their echo, misheard and far quieter than they said it, wasn't cut"
        );
    }

    #[test]
    fn their_echo_misheard_far_below_them_is_cut_however_little_joel_says_alone() {
        // Where Joel says little, too few of his words are heard alone to
        // know his voice, while his side is mostly their echo: that's then
        // told by how far below theirs it is alone.
        let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
        let mut mine = at_level(timed("Fine.", 1_000), -20.0);
        mine.extend(at_level(timed("Bunch had moon in Tuesday", 5_050), -42.0));
        assert_eq!(
            cut_from(&[theirs], &[mine]),
            ["Fine."],
            "their echo, misheard far below them where Joel says little, wasn't cut"
        );
    }

    #[test]
    fn joel_talking_over_them_near_their_level_is_his_own() {
        // His own voice in his microphone is near theirs in theirs, so
        // words of his said over them stay his, however they're spelled.
        let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
        let own = "Hang on, Friday suits me better.";
        assert_eq!(
            cut_from(&[theirs], &[at_level(timed(own, 5_100), -26.0)]),
            [own],
            "Joel talking over them was cut as their echo"
        );
    }

    #[test]
    fn joel_talking_over_them_through_a_quiet_microphone_is_his_own() {
        // Each side's level is its own recorder's, so a quiet microphone
        // or a loud call app puts all of his voice far below theirs: his
        // words over them are as loud as his words alone, and so his. His
        // two lines have none of theirs between, so they're written as one.
        let theirs = at_level(timed("We could meet next week if that suits", 0), -20.0);
        let over = "Hang on, wait.";
        let alone = "Fine by me, I'll book the room for us.";
        let mine = [
            at_level(timed(over, 600), -40.0),
            at_level(timed(alone, 10_000), -40.0),
        ];
        assert_eq!(
            cut_from(&[theirs], &mine),
            [format!("{over} {alone}")],
            "Joel talking over them through a quiet microphone was cut as their echo"
        );
    }

    #[test]
    fn joels_answer_in_the_take_holding_their_echo_is_his_own() {
        // His take may run on from the echo of their last words into his
        // answer: what follows the echo at his own voice's level is his.
        let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
        let own = "Fine by me, I'll book the room for us.";
        let answer = "Sure, that works.";
        let mut mine = at_level(timed(own, 1_000), -35.0);
        mine.extend(at_level(timed("Bunch had moon in Tuesday", 5_050), -52.0));
        mine.extend(at_level(timed(answer, 6_600), -35.0));
        assert_eq!(
            cut_from(&[theirs], &[mine]),
            [format!("{own} {answer}")],
            "Joel's answer after their echo in one take was cut as their echo"
        );
    }

    /// Joel's line saying `own` alone at `voice` dB, then `heard` within
    /// their "Lunch at noon on Thursday then." at -20 dB, as written once
    /// their echo is cut.
    fn heard_under_their_lunch(voice: f32, heard: &[Vec<Word>]) -> Vec<String> {
        let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
        let mut mine = at_level(
            timed("Fine by me, I'll book the room for us.", 1_000),
            voice,
        );
        mine.extend(heard.iter().flatten().cloned());
        cut_from(&[theirs], &[mine])
    }

    #[test]
    fn a_word_is_drowned_from_fifteen_db_below_theirs() {
        let own = "Fine by me, I'll book the room for us.";
        for (level, written) in [
            (-35.0, own.to_owned()),
            (-34.5, format!("{own} Bunch had moon")),
        ] {
            let heard = at_level(timed("Bunch had moon", 5_050), level);
            assert_eq!(
                heard_under_their_lunch(-20.0, &[heard]),
                [written],
                "a word {level} dB to their -20 was judged wrongly"
            );
        }
    }

    #[test]
    fn a_word_is_drowned_only_from_twelve_db_below_his_voice() {
        let own = "Fine by me, I'll book the room for us.";
        for (level, written) in [
            (-42.0, own.to_owned()),
            (-41.0, format!("{own} Bunch had moon")),
        ] {
            let heard = at_level(timed("Bunch had moon", 5_050), level);
            assert_eq!(
                heard_under_their_lunch(-30.0, &[heard]),
                [written],
                "a word {level} dB to his voice's -30 was judged wrongly"
            );
        }
    }

    #[test]
    fn the_echo_of_their_last_words_reaches_under_a_second_past_his_last_cut() {
        // Their last word is within a second of his "Right.", but the echo
        // cut before it ended over a second earlier, so it's his answer.
        let echo = at_level(timed("Bunch had moon in Tuesday", 5_050), -42.0);
        let right = at_level(timed("Right.", 7_300), -42.0);
        assert_eq!(
            heard_under_their_lunch(-20.0, &[echo, right]),
            ["Fine by me, I'll book the room for us. Right."],
            "a word over a second past the echo's end was cut as its echo"
        );
    }

    #[test]
    fn a_word_only_joels_microphone_heard_doesnt_break_their_echo() {
        // Echo comes in quiet over his room, so his side may hear a word
        // in it theirs didn't: their "noon" must still be cut after it.
        let theirs = timed("Lunch at noon", 5_000);
        let mut mine = timed("Fine by me.", 1_000);
        mine.extend(timed("Lunch at the noon", 5_050));
        assert_eq!(
            cut_from(&[theirs], &[mine]),
            ["Fine by me. the"],
            "a word of theirs after one only Joel's side heard was written as his"
        );
    }

    #[test]
    fn one_word_of_joels_heard_just_after_theirs_is_his_own() {
        // A lone "Okay" is as likely his own as their echo.
        let theirs = timed("Okay so the next item", 2_000);
        let mut mine = timed("Right, I see.", 1_100);
        mine.extend(timed("Okay", 2_050));
        assert_eq!(
            cut_from(&[theirs], &[mine]),
            ["Right, I see. Okay"],
            "Joel's lone word was cut as their echo"
        );
    }

    #[test]
    fn joels_words_said_well_before_theirs_are_his_own() {
        // Echo is heard from 300 ms before their word, by each side's frame,
        // to 400 ms after; Joel's "so it goes" comes 400 ms before theirs.
        let theirs = timed("So it goes.", 5_400);
        let mine = "Yeah, so it goes with these things every time.";
        assert_eq!(
            cut_from(&[theirs], &[timed(mine, 4_700)]),
            [mine],
            "Joel's words said before theirs were cut as their echo"
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
                below: None,
            }]
        );
    }

    #[test]
    fn what_was_cut_for_its_level_says_how_far_below_theirs_it_was() {
        // A word cut for its level echoes none of theirs in particular,
        // so how late it came says nothing; how far below them it was
        // does, where the margin is judged on a live call. Their words
        // spelled alike are cut for that, as before.
        let theirs = at_level(timed("Lunch at noon on Thursday then.", 5_000), -20.0);
        let own = at_level(
            timed("Fine by me, I'll book the room for us.", 1_000),
            -20.0,
        );
        for (heard, below) in [
            ("Bunch had moon in Tuesday", Some(22.0)),
            ("Lunch at noon on Thursday then.", None),
        ] {
            let mut mine = own.clone();
            mine.extend(at_level(timed(heard, 5_050), -42.0));
            let lines = [worded(Side::Theirs, &theirs), worded(Side::Mine, &mine)];
            let words = HashMap::from([(0, theirs.clone()), (1, mine)]);
            let (_, cuts) = cut_echoes(&lines, &words, |left| Some(left.to_owned()));
            let told: Vec<(&str, Option<f32>)> = cuts
                .iter()
                .map(|cut| (cut.text.as_str(), cut.below))
                .collect();
            assert_eq!(
                told,
                [(heard, below)],
                "the cut of {heard:?} didn't say how far below theirs it was by level alone"
            );
        }
    }

    #[test]
    fn their_echo_timed_just_before_their_word_is_cut_and_says_how_early() {
        // Each side's words are timed by its own encoder frames, so echo
        // can be timed before the word it echoes, as on AMI's meetings.
        let theirs = timed("Lunch at noon on Thursday then.", 5_000);
        let mut mine = timed("Fine by me.", 1_000);
        mine.extend(timed("Lunch at noon on Thursday then.", 4_800));
        let lines = [worded(Side::Theirs, &theirs), worded(Side::Mine, &mine)];
        let words = HashMap::from([(0, theirs), (1, mine)]);
        let (cut, cuts) = cut_echoes(&lines, &words, |left| Some(left.to_owned()));
        assert_eq!(
            cut[1].text, "Fine by me.",
            "their echo timed 200 ms before their words wasn't cut"
        );
        let early = isize::try_from(200 * RATE / 1000).unwrap();
        assert_eq!(
            cuts.iter().map(|cut| cut.after).collect::<Vec<_>>(),
            [-early],
            "the cut didn't say it was heard 200 ms before their word"
        );
    }

    #[test]
    fn a_line_whose_opening_echo_is_cut_starts_at_joels_first_word() {
        // His microphone hears their "Lunch at noon" before he answers in
        // the same take, so what's left of it starts when he does.
        let theirs = timed("Lunch at noon", 5_000);
        let mut mine = timed("Lunch at noon", 5_050);
        mine.extend(timed("Fine by me.", 6_500));
        let lines = [worded(Side::Theirs, &theirs), worded(Side::Mine, &mine)];
        let words = HashMap::from([(0, theirs), (1, mine)]);
        let (cut, _) = cut_echoes(&lines, &words, |left| Some(left.to_owned()));
        assert_eq!(cut[1].text, "Fine by me.", "their echo wasn't cut");
        assert_eq!(
            cut[1].start,
            6_500 * RATE / 1000,
            "Joel's line starts where their echo did, so it's written before what he answers"
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
