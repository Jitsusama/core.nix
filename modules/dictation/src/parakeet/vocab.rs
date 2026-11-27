//! Parakeet's vocabulary: the pieces its tokens write, which of them may
//! be written, and the text and words a run of tokens spells.

use super::Word;
use crate::openvino::Result;

/// The tokenizer's mark on a piece that starts a word. Parakeet's
/// exported vocabulary writes a plain space in its place, so `words`
/// takes either.
const SPACE: char = '\u{2581}';

/// The model's pieces, by token, and which may be written.
pub(super) struct Vocab {
    pieces: Vec<String>,
    allowed: Vec<bool>,
}

impl Vocab {
    /// The vocabulary from the export's JSON, an object of pieces by
    /// their tokens' numbers.
    pub(super) fn parse(json: &str) -> Result<Self> {
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(|e| format!("reading the vocabulary: {e}"))?;
        let object = value.as_object().ok_or("the vocabulary isn't an object")?;
        let mut pieces = vec![String::new(); object.len()];
        for (id, piece) in object {
            let slot = id
                .parse::<usize>()
                .ok()
                .and_then(|id| pieces.get_mut(id))
                .ok_or_else(|| format!("the vocabulary's {id} isn't one of its tokens"))?;
            piece
                .as_str()
                .ok_or_else(|| format!("the vocabulary's {id} isn't text"))?
                .clone_into(slot);
        }
        let allowed = pieces.iter().map(|piece| english(piece)).collect();
        Ok(Self { pieces, allowed })
    }

    /// Blank, the token after the vocabulary's last.
    pub(super) fn blank(&self) -> usize {
        self.pieces.len()
    }

    pub(super) fn allowed(&self, token: usize) -> bool {
        self.allowed.get(token).copied().unwrap_or(false)
    }

    /// The text `tokens` spell, each word's piece starting it with a space.
    pub(super) fn text(&self, tokens: &[usize]) -> String {
        let joined: String = tokens
            .iter()
            .filter_map(|&token| self.pieces.get(token))
            .map(String::as_str)
            .collect();
        joined.replace(SPACE, " ").trim().to_owned()
    }

    /// The words `tokens` spell, each at the sample its first piece was
    /// written at and as loud as `level` says the sound is there; a piece
    /// that starts a word starts the next.
    pub(super) fn words(
        &self,
        tokens: &[(usize, usize)],
        level: impl Fn(usize) -> f32,
    ) -> Vec<Word> {
        let mut words: Vec<Word> = Vec::new();
        for &(token, at) in tokens {
            let Some(piece) = self.pieces.get(token) else {
                continue;
            };
            let text = piece.trim_start_matches([SPACE, ' ']);
            match words.last_mut() {
                Some(word) if text.len() == piece.len() || word.text.is_empty() => {
                    word.text.push_str(text);
                }
                _ => words.push(Word {
                    text: text.to_owned(),
                    at,
                    level: level(at),
                }),
            }
        }
        words.retain(|word| !word.text.is_empty());
        words
    }
}

/// Whether a piece is English text: no special token such as `<unk>` or
/// `<|nospeech|>`, and nothing outside ASCII but the mark of a word's start.
fn english(piece: &str) -> bool {
    let special = piece.len() > 2 && piece.starts_with('<') && piece.ends_with('>');
    !special && piece.chars().all(|c| c == SPACE || c.is_ascii())
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::parakeet::FRAME;
    use std::path::Path;

    // Spaced as Parakeet's exported vocabulary is: a piece that starts a
    // word leads with a plain space, never the tokenizer's own mark.
    const VOCAB: &str = r#"{"0": "<unk>", "1": "<|nospeech|>", "2": " hello", "3": " wor",
        "4": "ld", "5": " é", "6": "<", "7": " ne"}"#;

    /// A vocabulary of eight pieces, for the decoder's tests too.
    pub(in crate::parakeet) fn vocab() -> Vocab {
        Vocab::parse(VOCAB).unwrap()
    }

    #[test]
    fn words_are_spelled_from_their_pieces() {
        assert_eq!(vocab().text(&[2, 3, 4]), "hello world");
    }

    #[test]
    fn blank_follows_the_vocabulary() {
        assert_eq!(vocab().blank(), 8);
    }

    #[test]
    fn only_english_text_may_be_written() {
        let vocab = vocab();
        let allowed: Vec<usize> = (0..vocab.blank()).filter(|&t| vocab.allowed(t)).collect();
        assert_eq!(allowed, [2, 3, 4, 6, 7]);
        assert!(english("\u{2581}hello"), "the tokenizer's mark refused");
    }

    #[test]
    fn a_piece_that_would_break_a_line_is_never_written() {
        // A transcript's lines are its paragraphs, so a piece writing a
        // newline, a tab or another control would let what someone said
        // open a paragraph the transcript reads as its own note.
        for piece in ["\n", " a\n\nb", "\t", "\r", "\u{7f}"] {
            assert!(!english(piece), "{piece:?} may be written");
        }
    }

    #[test]
    fn a_vocabulary_with_a_gap_is_refused() {
        assert!(
            Vocab::parse(r#"{"0": "a", "2": "b"}"#).is_err(),
            "a vocabulary missing token 1 was taken"
        );
    }

    #[test]
    fn words_start_where_their_first_piece_was_written_and_spell_the_text() {
        let vocab = vocab();
        let tokens = [(2, 0), (3, 2 * FRAME), (4, 3 * FRAME)];
        let words = vocab.words(&tokens, |at| if at == 0 { -20.0 } else { -30.0 });
        assert_eq!(
            words,
            [
                Word {
                    text: "hello".to_owned(),
                    at: 0,
                    level: -20.0
                },
                Word {
                    text: "world".to_owned(),
                    at: 2 * FRAME,
                    level: -30.0
                }
            ],
            "a word didn't start at its first piece, or wasn't as loud as it was there"
        );
        let spelled: Vec<&str> = words.iter().map(|word| word.text.as_str()).collect();
        assert_eq!(
            spelled.join(" "),
            vocab.text(&[2, 3, 4]),
            "the words spell other than the text"
        );
    }

    #[test]
    fn a_piece_that_doesnt_start_a_word_opens_one_when_nothing_came_before() {
        let words = vocab().words(&[(4, FRAME), (7, 2 * FRAME)], |_| 0.0);
        let spelled: Vec<(&str, usize)> = words.iter().map(|w| (w.text.as_str(), w.at)).collect();
        assert_eq!(
            spelled,
            [("ld", FRAME), ("ne", 2 * FRAME)],
            "a take opening mid-word lost its first piece"
        );
    }

    #[test]
    fn a_bare_word_start_opens_the_word_its_next_piece_spells() {
        // Parakeet's export has one bare piece, a plain space; the
        // tokenizer's own vocabularies write its mark, which `words` takes too.
        for mark in [" ", "\u{2581}"] {
            let pieces = format!(r#"{{"0": "{mark}", "1": "ld", "2": "{mark}hello"}}"#);
            let vocab = Vocab::parse(&pieces).unwrap();
            let tokens = [(0, 0), (1, FRAME), (2, 2 * FRAME)];
            let words = vocab.words(&tokens, |_| 0.0);
            let spelled: Vec<(&str, usize)> =
                words.iter().map(|w| (w.text.as_str(), w.at)).collect();
            assert_eq!(
                spelled,
                [("ld", 0), ("hello", 2 * FRAME)],
                "a bare {mark:?} didn't open the word after it"
            );
            let joined: Vec<&str> = spelled.iter().map(|(text, _)| *text).collect();
            assert_eq!(
                joined.join(" "),
                vocab.text(&[0, 1, 2]),
                "{mark:?} spelled otherwise"
            );
        }
    }

    #[test]
    fn the_shipped_vocabulary_gives_a_word_for_each_its_text_has() {
        // The build names the vocabulary Parakeet ships with (package.nix),
        // so one spaced another way fails here rather than leaving every
        // call's takes whole; `nix develop` on the package names it too.
        // A plain `cargo test`, or one in the repository's shell, has none
        // to read. A shell sets NIX_BUILD_TOP as a build does, but sets
        // IN_NIX_SHELL beside it.
        let Some(path) = std::env::var_os("PARAKEET_VOCAB") else {
            let building = std::env::var_os("NIX_BUILD_TOP").is_some()
                && std::env::var_os("IN_NIX_SHELL").is_none();
            assert!(
                !building,
                "the build named no vocabulary in PARAKEET_VOCAB, as package.nix does"
            );
            eprintln!("the shipped vocabulary wasn't read: PARAKEET_VOCAB names none");
            return;
        };
        let path = Path::new(&path);
        let read = std::fs::read_to_string(path)
            .unwrap_or_else(|why| panic!("PARAKEET_VOCAB's {} didn't read: {why}", path.display()));
        let vocab = Vocab::parse(&read)
            .unwrap_or_else(|why| panic!("PARAKEET_VOCAB's {} isn't one: {why}", path.display()));
        let tokens: Vec<usize> = (0..vocab.blank()).filter(|&t| vocab.allowed(t)).collect();
        let timed: Vec<(usize, usize)> = tokens.iter().map(|&t| (t, t * FRAME)).collect();
        let words = vocab.words(&timed, |_| 0.0);
        let spelled: Vec<&str> = words.iter().map(|word| word.text.as_str()).collect();
        let text = vocab.text(&tokens);
        let said: Vec<&str> = text.split_whitespace().collect();
        assert!(
            said.len() > 1000,
            "the vocabulary spelled only {} words",
            said.len()
        );
        assert_eq!(spelled, said, "its words aren't the ones its text has");
    }
}
