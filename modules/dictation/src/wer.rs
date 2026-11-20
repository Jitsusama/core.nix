//! How many of what was said a transcript got wrong: its word error rate,
//! the words substituted, deleted and inserted against a reference, over
//! the words the reference holds. call-transcripts' score (its main.rs)
//! measures its hearing so, on meetings whose every word was written down
//! by hand.
//!
//! Both sides are compared as plain words: lowercase, without punctuation,
//! a hyphenated word as its parts, and a word spelled out letter by letter
//! as AMI writes one (`T_V_`) as one word. Hesitations ("um", "uh",
//! "mm-hmm") are left out of both, since a reference writes every one and a
//! speech model writes few. A word said twice in a row is said once in the
//! reference (`said`), since a reference keeps every stutter and a speech
//! model drops them; a word heard twice still counts twice, so a model
//! caught in a loop is seen. Words a transcript leaves out on purpose, such
//! as a lone "yeah", aren't hesitations, so leaving them out counts.

use std::ops::AddAssign;

/// What's left out of both sides as a hesitation.
const HESITATIONS: [&str; 13] = [
    "ah", "eh", "er", "erm", "hm", "hmm", "mhm", "mm", "mm-hmm", "uh", "uh-huh", "um", "um-hmm",
];

/// `text`'s words as they're compared, heard.
#[must_use]
pub fn words(text: &str) -> Vec<String> {
    // A curly apostrophe is an apostrophe, as tidy.rs reads one.
    let lower = text.to_lowercase().replace('’', "'");
    lower
        .split(|c: char| !(c.is_alphanumeric() || matches!(c, '\'' | '-' | '_')))
        .map(|word| word.replace('_', ""))
        .filter(|word| !HESITATIONS.contains(&word.trim_matches(['\'', '-'])))
        .flat_map(|word| {
            word.split('-')
                .map(|part| part.trim_matches('\'').to_owned())
                .collect::<Vec<_>>()
        })
        // A hesitation spelled with a hyphen the list doesn't hold, such
        // as "mm-hm", is still one in its parts.
        .filter(|word| !word.is_empty() && !HESITATIONS.contains(&word.as_str()))
        .collect()
}

/// A reference's words as they're compared: its words, a word said twice
/// in a row said once.
#[must_use]
pub fn said(text: &str) -> Vec<String> {
    let mut said = words(text);
    said.dedup();
    said
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

    /// The errors as a percentage of the words said, in tenths of a
    /// percent, a half rounded up: counted in whole numbers, so what's
    /// said and what's held to a ceiling are the same; none said is none
    /// wrong.
    #[must_use]
    pub fn tenths(&self) -> usize {
        if self.words == 0 {
            return 0;
        }
        (1000 * self.total() + self.words / 2) / self.words
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

    fn errors(reference: &str, heard: &str) -> Errors {
        Errors::count(&said(reference), &words(heard))
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
            said("Um so so I I think, uh, mm-hmm, we're done"),
            ["so", "i", "think", "we're", "done"]
        );
    }

    #[test]
    fn a_word_heard_over_and_over_counts_each_time() {
        assert_eq!(errors("the the plan", "the the the plan").inserted, 2);
    }

    #[test]
    fn a_word_spelled_out_is_one_word() {
        assert_eq!(
            words("a T_V_ and a T_ Rex"),
            ["a", "tv", "and", "a", "t", "rex"]
        );
    }

    #[test]
    fn a_hesitation_with_a_hyphen_is_left_out_in_its_parts() {
        assert_eq!(words("Mm-hm, uh-oh, mm-kay, uh-huh"), ["oh", "kay"]);
    }

    #[test]
    fn a_curly_apostrophe_keeps_its_word_whole() {
        assert_eq!(words("don’t"), ["don't"]);
    }

    #[test]
    fn a_word_with_an_accent_is_one_word() {
        assert_eq!(words("a naïve café"), ["a", "naïve", "café"]);
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
    fn of_as_few_errors_the_fewest_are_substitutions() {
        // Two substitutions, or a word dropped and one added: as many
        // errors either way, and the word was dropped where it was said.
        assert_eq!(
            errors("move it", "it move"),
            Errors {
                words: 2,
                deleted: 1,
                inserted: 1,
                ..Errors::default()
            }
        );
    }

    #[test]
    fn nothing_heard_deletes_every_word_and_nothing_said_inserts() {
        assert_eq!(errors("move it", "").deleted, 2);
        assert_eq!(errors("", "move it").inserted, 2);
        assert_eq!(
            errors("", "move it").tenths(),
            0,
            "nothing said has a rate of 0"
        );
    }

    #[test]
    fn the_rate_is_the_errors_over_the_words_said_and_adds_up() {
        let mut all = errors(
            "move the review to thursday",
            "move a review thursday please",
        );
        assert_eq!(all.tenths(), 600, "3 errors in 5 words are 60.0%");
        all += errors("thank you", "thank you");
        assert_eq!(all.words, 7);
        assert_eq!(all.tenths(), 429, "3 errors in 7 words are 42.9%");
    }

    #[test]
    fn a_rate_halfway_between_tenths_rounds_up() {
        let errors = Errors {
            words: 4000,
            substituted: 798,
            ..Errors::default()
        };
        assert_eq!(errors.tenths(), 200, "798 errors in 4000 words are 20.0%");
    }
}
