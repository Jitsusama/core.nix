//! The others' echo on Joel's side of a call, left out of its transcript.
//!
//! Joel's side is his microphone as it hears the room, before the call
//! app's echo cancelling, so on the laptop's speakers it hears the others
//! too. Where each word's time is known, their words his microphone heard
//! just after they said them, and quieter than they said them, are cut
//! from his line, as are words of his heard inside their speech far
//! quieter than it, however spelled, and, once his voice is known, far
//! quieter than his voice too. His own
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
/// echo. Until then, his words over theirs through a quiet microphone may
/// be cut too. A call still being written is cut again whole once his
/// voice is known, which brings them back, but lines read back after a
/// restart stay as they were written, and the journal says when each cut
/// was.
const ALONE: usize = 5;

/// How many of Joel's words heard alone, those nearest in time to a
/// word of his, his voice is taken from to judge it: his level moves
/// with his microphone, which he may change partway through a call. On
/// the clean meetings five or nine put his side past what one level for
/// the call reached, while eleven held it there and still follows a new
/// microphone once six of his words are heard alone on it. Nearest has
/// no limit in time: words alone long before or after count as soon as
/// too few are nearer.
const NEAR: usize = 11;

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

/// When each of Joel's `mine` heard while none of `theirs` is was heard,
/// and its level, in the order heard.
fn alone<'a>(mine: impl IntoIterator<Item = &'a Word>, theirs: &[Word]) -> Vec<(usize, f32)> {
    let mut alone: Vec<(usize, f32)> = mine
        .into_iter()
        .filter(|word| around(word, theirs).is_empty())
        .map(|word| (word.at, word.level))
        .collect();
    alone.sort_by_key(|&(at, _)| at);
    alone
}

/// The level of Joel's voice in his microphone at `at`: the middle level
/// of the `NEAR` of his words heard `alone` nearest it, if `ALONE` are,
/// the quieter of two middles, as a voice judged quieter cuts fewer of
/// his words.
fn voice(alone: &[(usize, f32)], at: usize) -> Option<f32> {
    if alone.len() < ALONE {
        return None;
    }
    let mut from = alone.partition_point(|&(heard, _)| heard < at);
    let mut to = from;
    while to - from < NEAR.min(alone.len()) {
        let before = from.checked_sub(1).map(|i| at - alone[i].0);
        let after = alone.get(to).map(|&(heard, _)| heard - at);
        match before {
            Some(before) if after.is_none_or(|after| before <= after) => from -= 1,
            _ => to += 1,
        }
    }
    let mut levels: Vec<f32> = alone[from..to].iter().map(|&(_, level)| level).collect();
    levels.sort_by(f32::total_cmp);
    Some(levels[(levels.len() - 1) / 2])
}

/// For each of Joel's `mine`, the loudest of `theirs` said within a
/// moment of it, if his word is its echo by how loud each is. Theirs must
/// be `DROWNED` louder than his word, and his `voice` around it, from his
/// words heard `alone`, `QUIETER` louder, once it's known. They must go on
/// speaking past his word, or his word just before it must have been cut
/// so, as the echo of their last words follows them. A word of his that
/// opens as their line ends, with none cut just before it, may be his
/// answer.
fn drowned(mine: &[Word], theirs: &[Word], alone: &[(usize, f32)]) -> Vec<Option<usize>> {
    let mut drowned: Vec<Option<usize>> = Vec::with_capacity(mine.len());
    for (i, word) in mine.iter().enumerate() {
        let near = around(word, theirs);
        let on = theirs[near.end..]
            .first()
            .is_some_and(|said| said.at <= word.at + LAG);
        let echoing = i > 0 && drowned[i - 1].is_some() && mine[i - 1].at + LAG >= word.at;
        let quiet = voice(alone, word.at).is_none_or(|his| word.level + QUIETER <= his);
        let loudest = near.max_by(|&a, &b| theirs[a].level.total_cmp(&theirs[b].level));
        let below = |j: &usize| theirs[*j].level >= word.level + DROWNED;
        drowned.push(loudest.filter(|j| (on || echoing) && quiet && below(j)));
    }
    drowned
}

/// Some of Joel's words cut from a line of his as the others' echo: the
/// line's index, when the first was heard, how long after its word of
/// theirs, in samples, what they say, and, if they were cut for their
/// level rather than their spelling, how far below the loudest of theirs
/// around it the first was, in dB.
#[derive(Debug, PartialEq)]
pub struct Cut {
    pub line: usize,
    pub at: usize,
    pub after: isize,
    pub text: String,
    pub below: Option<f32>,
}

impl Cut {
    /// What it was cut by, as the journal and the meetings check say it:
    /// how far below theirs, if for its level, else how late after theirs.
    #[must_use]
    pub fn by(&self) -> String {
        if let Some(below) = self.below {
            format!("{below:.0} dB below theirs")
        } else {
            let rate = isize::try_from(RATE).unwrap_or(isize::MAX);
            format!("{} ms after theirs", self.after * 1000 / rate)
        }
    }
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
    cut_echoes_heard(lines, words, tidy, |_| None)
}

/// Lab only: `cut_echoes`, also cutting each word of his that `loud`
/// says their sound stands far enough above, by how far.
pub fn cut_echoes_heard<S: BuildHasher>(
    lines: &[Line],
    words: &HashMap<usize, Vec<Word>, S>,
    tidy: impl Fn(&str) -> Option<String>,
    loud: impl Fn(&Word) -> Option<f32>,
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
    let his = alone(
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
        let echoes: Vec<Option<(usize, Option<f32>)>> = echoes_of(mine, &theirs)
            .into_iter()
            .zip(drowned(mine, &theirs, &his))
            .zip(mine)
            .map(|((by_spelling, by_level), word)| {
                let below = |j: usize| (j, Some(theirs[j].level - word.level));
                let heard = || {
                    let last = theirs.len().checked_sub(1)?;
                    let j = theirs.partition_point(|their| their.at < word.at).min(last);
                    loud(word).map(|gap| (j, Some(gap)))
                };
                by_spelling
                    .map(|j| (j, None))
                    .or(by_level.map(below))
                    .or_else(heard)
            })
            .collect();
        if echoes.iter().all(Option::is_none) {
            continue;
        }
        let mut kept = Vec::new();
        let mut first = None;
        let mut run: Option<Cut> = None;
        for (word, echo) in mine.iter().zip(&echoes) {
            let Some((j, below)) = *echo else {
                kept.push(word.text.as_str());
                first = first.or(Some(word.at));
                cuts.extend(run.take());
                continue;
            };
            if run
                .as_ref()
                .is_some_and(|cutting| cutting.below.is_some() != below.is_some())
            {
                cuts.extend(run.take());
            }
            if let Some(cutting) = run.as_mut() {
                cutting.text.push(' ');
                cutting.text.push_str(&word.text);
            } else {
                run = Some(Cut {
                    line: index,
                    at: word.at,
                    after: later(word.at, theirs[j].at),
                    text: word.text.clone(),
                    below,
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

/// Lab only: each of Joel's words in `lines` left uncut, with when it
/// was heard, why the level rule kept it, how far below the loudest of
/// theirs around it it was, and how far below his voice there, in dB.
pub fn kept<S: BuildHasher>(
    lines: &[Line],
    words: &HashMap<usize, Vec<Word>, S>,
) -> Vec<(usize, &'static str, f32, Option<f32>)> {
    let mut theirs: Vec<Word> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.side == Side::Theirs)
        .filter_map(|(index, _)| words.get(&index))
        .flatten()
        .cloned()
        .collect();
    theirs.sort_by_key(|word| word.at);
    let his = alone(
        lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.side == Side::Mine)
            .filter_map(|(index, _)| words.get(&index))
            .flatten(),
        &theirs,
    );
    let mut kept = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some(mine) = words.get(&index).filter(|_| line.side == Side::Mine) else {
            continue;
        };
        let by_spelling = echoes_of(mine, &theirs);
        let by_level = drowned(mine, &theirs, &his);
        for (i, word) in mine.iter().enumerate() {
            if by_spelling[i].is_some() || by_level[i].is_some() {
                continue;
            }
            let near = around(word, &theirs);
            let Some(loudest) = near
                .clone()
                .max_by(|&a, &b| theirs[a].level.total_cmp(&theirs[b].level))
            else {
                kept.push((word.at, "none of theirs around", 0.0, None));
                continue;
            };
            let below = theirs[loudest].level - word.level;
            let voice = voice(&his, word.at).map(|his| his - word.level);
            let on = theirs[near.end..]
                .first()
                .is_some_and(|said| said.at <= word.at + LAG);
            let echoing = i > 0 && by_level[i - 1].is_some() && mine[i - 1].at + LAG >= word.at;
            let why = if below < DROWNED {
                "near their level"
            } else if voice.is_some_and(|under| under < QUIETER) {
                "near his voice"
            } else if !(on || echoing) {
                "their line ended"
            } else {
                "unknown"
            };
            kept.push((word.at, why, below, voice));
        }
    }
    kept
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
#[path = "echo_tests.rs"]
mod tests;
