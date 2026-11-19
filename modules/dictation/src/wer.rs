//! How many of what was said a transcript got wrong: its word error rate,
//! the words substituted, deleted and inserted against a reference, over
//! the words the reference holds. call-transcripts' score (its main.rs)
//! measures its hearing so, on meetings whose every word was written down
//! by hand.
//!
//! Both sides are compared as plain words: lowercase, without punctuation,
//! a hyphenated word as its parts. Hesitations ("um", "uh", "mm-hmm") are
//! left out of both, since a reference writes every one and a speech model
//! writes few; so is a word said twice in a row, a stutter one keeps and
//! the other drops. Words a transcript leaves out on purpose, such as a
//! lone "yeah", aren't hesitations, so leaving them out counts.

use std::ops::AddAssign;

/// What's left out of both sides as a hesitation.
const HESITATIONS: [&str; 12] = [
    "ah", "eh", "er", "erm", "hmm", "mhm", "mm", "mm-hmm", "uh", "uh-huh", "um", "um-hmm",
];

/// `text`'s words as they're compared.
#[must_use]
pub fn words(text: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let lower = text.to_lowercase();
    let spoken = lower
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '\'' || c == '-'))
        .map(|word| word.trim_matches(['\'', '-']))
        .filter(|word| !word.is_empty() && !HESITATIONS.contains(word));
    for word in spoken.flat_map(|word| word.split('-')) {
        if !word.is_empty() && words.last().is_none_or(|last| last != word) {
            words.push(word.to_owned());
        }
    }
    words
}

/// The errors of a transcript against its reference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Errors {
    /// The words the reference holds.
    pub words: usize,
    pub substituted: usize,
    pub deleted: usize,
    pub inserted: usize,
}

impl Errors {
    /// The fewest errors that turn `said` into `heard`, and of those, the
    /// fewest substitutions, so a dropped word is counted as dropped.
    #[must_use]
    pub fn count(said: &[String], heard: &[String]) -> Self {
        // A row of the edit table at a time: each cell the errors that
        // turn the first i words said into the first j heard.
        let mut row: Vec<Self> = (0..=heard.len())
            .map(|j| Self {
                inserted: j,
                ..Self::default()
            })
            .collect();
        for (i, word) in said.iter().enumerate() {
            let mut next = vec![Self {
                words: i + 1,
                deleted: i + 1,
                ..Self::default()
            }];
            for (j, other) in heard.iter().enumerate() {
                let matched = Self {
                    words: row[j].words + 1,
                    substituted: row[j].substituted + usize::from(word != other),
                    ..row[j]
                };
                let deleted = Self {
                    words: row[j + 1].words + 1,
                    deleted: row[j + 1].deleted + 1,
                    ..row[j + 1]
                };
                let inserted = Self {
                    inserted: next[j].inserted + 1,
                    ..next[j]
                };
                let best = [matched, deleted, inserted]
                    .into_iter()
                    .min_by_key(|errors| (errors.total(), errors.substituted))
                    .unwrap_or(matched);
                next.push(best);
            }
            row = next;
        }
        row[heard.len()]
    }

    /// Every error, of each kind.
    #[must_use]
    pub fn total(&self) -> usize {
        self.substituted + self.deleted + self.inserted
    }

    /// The errors as a percentage of the words said; none said is none
    /// wrong.
    #[must_use]
    pub fn rate(&self) -> f64 {
        if self.words == 0 {
            return 0.0;
        }
        #[expect(clippy::cast_precision_loss, reason = "a set's words are thousands")]
        let rate = 100.0 * self.total() as f64 / self.words as f64;
        rate
    }
}

impl AddAssign for Errors {
    fn add_assign(&mut self, other: Self) {
        self.words += other.words;
        self.substituted += other.substituted;
        self.deleted += other.deleted;
        self.inserted += other.inserted;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn errors(said: &str, heard: &str) -> Errors {
        Errors::count(&words(said), &words(heard))
    }

    #[test]
    fn words_are_compared_plain() {
        assert_eq!(
            words("Okay, so the Remote-Control's DONE."),
            ["okay", "so", "the", "remote", "control's", "done"]
        );
    }

    #[test]
    fn hesitations_and_a_word_said_twice_are_left_out() {
        assert_eq!(
            words("Um so so I I think, uh, mm-hmm, we're done"),
            ["so", "i", "think", "we're", "done"]
        );
    }

    #[test]
    fn a_lone_yeah_is_a_word_that_counts() {
        assert_eq!(
            errors("Yeah. We merge it Thursday.", "We merge it Thursday."),
            Errors {
                words: 5,
                deleted: 1,
                ..Errors::default()
            }
        );
    }

    #[test]
    fn a_transcript_word_for_word_has_no_errors() {
        let same = errors("Can we move the review?", "can we move the review");
        assert_eq!(same.total(), 0);
        assert_eq!(same.words, 5);
    }

    #[test]
    fn each_kind_of_error_is_counted_apart() {
        assert_eq!(
            errors(
                "move the review to thursday",
                "move a review thursday please"
            ),
            Errors {
                words: 5,
                substituted: 1,
                deleted: 1,
                inserted: 1,
            }
        );
    }

    #[test]
    fn a_dropped_word_is_a_deletion_rather_than_substitutions() {
        assert_eq!(
            errors("we can move it", "we move it"),
            Errors {
                words: 4,
                deleted: 1,
                ..Errors::default()
            }
        );
    }

    #[test]
    fn nothing_heard_deletes_every_word_and_nothing_said_inserts() {
        assert_eq!(errors("move it", "").deleted, 2);
        assert_eq!(errors("", "move it").inserted, 2);
        assert!(errors("", "move it").rate().abs() < f64::EPSILON);
    }

    #[test]
    fn the_rate_is_the_errors_over_the_words_said_and_adds_up() {
        let mut all = errors(
            "move the review to thursday",
            "move a review thursday please",
        );
        assert!((all.rate() - 60.0).abs() < 1e-9);
        all += errors("thank you", "thank you");
        assert_eq!(all.words, 7);
        assert!((all.rate() - 300.0 / 7.0).abs() < 1e-9);
    }
}
