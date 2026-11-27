//! What a call being heard tells the journal of Joel's side left out as
//! the others' echo, and of what it later writes back: how many words and
//! when, never the words, since a call's words stay out of the journal.
//! What's wrongly left out is gone from the file, so this is all that says
//! where to listen for it.

use std::collections::{BTreeMap, HashMap, HashSet};

use dictation::echo::{self, Cut};
use dictation::parakeet::Word;
use dictation::transcript;

use super::Writing;
use crate::{said, split};

/// What the journal was told of a call's echo cuts and not since told
/// written back: each of Joel's words cut, by the sample it was heard at,
/// with the sample the run that told it starts at, so a run written back
/// in part, or grown at its front, tells only the words that changed;
/// and `left` the lines of his told as left out whole, by the sample each
/// starts at. Splitting their takes by voice moves his lines' indices on
/// but never their times, so what's told stays told across a rejoin.
#[derive(Default)]
pub(super) struct Told {
    words: HashMap<usize, usize>,
    left: HashSet<usize>,
}

/// A run of Joel's words cut as their echo, as the journal tells it: the
/// sample each word was heard at, in order, and what cut them
/// (`Cut::by`).
struct Run {
    words: Vec<usize>,
    by: String,
}

impl Told {
    /// A journal line for the words of each run of `cutting` newly cut,
    /// from the first of them, and one for each run told whose words the
    /// call written again keeps, as it may once more of Joel's words are
    /// heard alone, from the first it keeps; each word said once until
    /// it's cut again, and written back told before what's cut.
    #[must_use]
    fn runs(&mut self, cutting: &[Run]) -> Vec<String> {
        let cut: HashSet<usize> = cutting
            .iter()
            .flat_map(|run| run.words.iter().copied())
            .collect();
        // How many words of each run told are written back, and the first.
        let mut back: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
        self.words.retain(|&word, &mut run| {
            let still = cut.contains(&word);
            if !still {
                let (words, first) = back.entry(run).or_insert((0, word));
                *words += 1;
                *first = (*first).min(word);
            }
            still
        });
        let mut untold: Vec<String> = back
            .into_values()
            .map(|(words, first)| {
                format!(
                    "call-transcripts: {words} of Joel's words at {} written back",
                    transcript::clock(first),
                )
            })
            .collect();
        for run in cutting {
            let Some(&at) = run.words.first() else {
                continue;
            };
            let new: Vec<usize> = run
                .words
                .iter()
                .copied()
                .filter(|word| !self.words.contains_key(word))
                .collect();
            self.words.extend(new.iter().map(|&word| (word, at)));
            if let Some(&first) = new.first() {
                untold.push(format!(
                    "call-transcripts: {} of Joel's words at {} cut as their echo, {}",
                    new.len(),
                    transcript::clock(first),
                    run.by,
                ));
            }
        }
        untold
    }
}

impl Writing {
    /// What's newly left out of Joel's side as the others' echo, or
    /// written back, a journal line each, said once: for words cut from a
    /// line, how many, when and what they were cut by (`Cut::by`); for
    /// words told cut that the call written again keeps, how many and
    /// when; for a line left out whole, how many words and when.
    #[must_use]
    pub(super) fn untold_cuts(&mut self) -> Vec<String> {
        let (cut, cuts) = echo::cut_echoes(&self.lines, &self.words, &self.sound, said::spoken);
        let runs: Vec<Run> = cuts.iter().map(|cut| run_of(cut, &self.words)).collect();
        let mut untold = self.told.runs(&runs);
        let mut left: Vec<usize> = split::echoing(&cut, &self.pieces, self.read)
            .into_iter()
            .filter(|&index| self.told.left.insert(self.lines[index].start))
            .collect();
        left.sort_unstable();
        untold.extend(left.into_iter().map(|index| {
            format!(
                "call-transcripts: a line of Joel's, {} words at {}, left out whole as \
                 their echo",
                cut[index].text.split_whitespace().count(),
                transcript::clock(cut[index].start),
            )
        }));
        untold
    }
}

/// The run of Joel's words `cut` is, by the samples its words were heard
/// at in his line's `words`: as many as it spells, from its first.
fn run_of(cut: &Cut, words: &HashMap<usize, Vec<Word>>) -> Run {
    let spelled = cut.text.split_whitespace().count();
    let line = words.get(&cut.line).map_or(&[][..], Vec::as_slice);
    let from = line.iter().position(|word| word.at == cut.at);
    Run {
        words: from.map_or_else(Vec::new, |from| {
            line[from..]
                .iter()
                .take(spelled)
                .map(|word| word.at)
                .collect()
        }),
        by: cut.by(),
    }
}

#[cfg(test)]
#[path = "journal_tests.rs"]
mod tests;
