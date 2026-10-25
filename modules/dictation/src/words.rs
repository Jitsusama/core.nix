//! The words Joel uses that Whisper doesn't know, given to it before each
//! transcription as text it has already heard.
//!
//! Whisper takes the text of what came before as a prompt, after its
//! `<|startofprev|>` token, and leans toward the words and spellings in it:
//! a place it hears as "Sanita" it writes as Sarnia once Sarnia is in the
//! prompt. The words file holds one word or phrase a line, which become one
//! sentence, "Sarnia, Toronto, Forest.", given as Whisper's own tools give
//! a prompt: a line of text with a space before it.
//!
//! The prompt has to be in Whisper's tokens, so this holds GPT-2's byte
//! level BPE: the text is cut into pieces as GPT-2's pattern cuts it, and
//! each piece's bytes are joined pair by pair in merges.txt's order, the
//! lowest ranked pair first, until no pair in it has a rank.

use std::collections::HashMap;

use crate::vocab::{Vocab, byte_decoder};

/// `<|startofprev|>`, which comes before the text of what came before.
pub const PREVIOUS: usize = 50_361;
/// As many tokens of what came before as Whisper reads, the latest kept.
pub const MOST: usize = 223;

/// Turns text into Whisper's tokens.
pub struct Encoder {
    ranks: HashMap<(Vec<u8>, Vec<u8>), usize>,
    numbers: HashMap<Vec<u8>, usize>,
}

impl Encoder {
    /// An encoder from the vocabulary and merges.txt's text.
    ///
    /// # Errors
    ///
    /// When a merge line isn't two tokens, or holds a character no byte is.
    pub fn new(vocab: &Vocab, merges: &str) -> Result<Self, String> {
        let bytes = byte_decoder();
        let to_bytes = |text: &str| -> Result<Vec<u8>, String> {
            text.chars()
                .map(|c| {
                    bytes
                        .get(&c)
                        .copied()
                        .ok_or_else(|| format!("no byte is {c:?}"))
                })
                .collect()
        };
        let mut ranks = HashMap::new();
        for line in merges.lines().filter(|line| !line.starts_with("#version")) {
            if line.is_empty() {
                continue;
            }
            let (left, right) = line
                .split_once(' ')
                .ok_or_else(|| format!("a merge isn't two tokens: {line:?}"))?;
            let rank = ranks.len();
            ranks.insert((to_bytes(left)?, to_bytes(right)?), rank);
        }
        Ok(Self {
            ranks,
            numbers: vocab.numbers(),
        })
    }

    /// The text's tokens.
    ///
    /// # Errors
    ///
    /// When a piece of it joins into bytes that aren't a token, which a
    /// vocabulary holding every byte never does.
    pub fn encode(&self, text: &str) -> Result<Vec<usize>, String> {
        let mut tokens = Vec::new();
        for piece in pieces(text) {
            for part in self.join(piece.as_bytes()) {
                let token = self
                    .numbers
                    .get(&part)
                    .ok_or_else(|| format!("{:?} isn't a token", String::from_utf8_lossy(&part)))?;
                tokens.push(*token);
            }
        }
        Ok(tokens)
    }

    /// The bytes of a piece, joined pair by pair as merges.txt ranks them.
    fn join(&self, piece: &[u8]) -> Vec<Vec<u8>> {
        let mut parts: Vec<Vec<u8>> = piece.iter().map(|&b| vec![b]).collect();
        loop {
            let best = parts
                .windows(2)
                .filter_map(|pair| {
                    self.ranks
                        .get(&(pair[0].clone(), pair[1].clone()))
                        .map(|&rank| (rank, pair[0].clone(), pair[1].clone()))
                })
                .min();
            let Some((_, left, right)) = best else {
                return parts;
            };
            let mut joined = Vec::with_capacity(parts.len());
            let mut i = 0;
            while i < parts.len() {
                if i + 1 < parts.len() && parts[i] == left && parts[i + 1] == right {
                    joined.push([left.as_slice(), right.as_slice()].concat());
                    i += 2;
                } else {
                    joined.push(parts[i].clone());
                    i += 1;
                }
            }
            parts = joined;
        }
    }
}

/// The words file's lines as the sentence Whisper is given, a space first,
/// or nothing when it holds no words.
#[must_use]
pub fn sentence(words: &str) -> String {
    let words: Vec<&str> = words
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if words.is_empty() {
        String::new()
    } else {
        format!(" {}.", words.join(", "))
    }
}

/// What the decoder starts from: what came before, when there is any, its
/// latest `MOST` tokens after `<|startofprev|>`, then the task's own.
#[must_use]
pub fn start(before: &[usize], task: &[usize]) -> Vec<usize> {
    let mut ids = Vec::new();
    if !before.is_empty() {
        ids.push(PREVIOUS);
        ids.extend_from_slice(&before[before.len().saturating_sub(MOST)..]);
    }
    ids.extend_from_slice(task);
    ids
}

/// The text cut as GPT-2's pattern cuts it: the contractions 's, 't, 're,
/// 've, 'm, 'll and 'd; a run of letters, of digits, or of anything else
/// but space, each with the one space before it; and the spaces between.
fn pieces(text: &str) -> Vec<String> {
    #[derive(PartialEq, Clone, Copy)]
    enum Kind {
        Letter,
        Digit,
        Other,
    }
    let kind = |c: char| {
        if c.is_alphabetic() {
            Kind::Letter
        } else if c.is_numeric() {
            Kind::Digit
        } else {
            Kind::Other
        }
    };
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let rest: String = chars[i..].iter().take(3).collect();
        if let Some(contraction) = ["'ll", "'re", "'ve", "'s", "'t", "'m", "'d"]
            .iter()
            .find(|c| rest.starts_with(*c))
        {
            out.push((*contraction).to_owned());
            i += contraction.len();
            continue;
        }
        let start = i;
        if chars[i] == ' ' && chars.get(i + 1).is_some_and(|c| !c.is_whitespace()) {
            i += 1;
        }
        if chars[i].is_whitespace() {
            while i < chars.len()
                && chars[i].is_whitespace()
                && !(chars[i] == ' ' && chars.get(i + 1).is_some_and(|c| !c.is_whitespace()))
            {
                i += 1;
            }
            if i == start {
                i += 1;
            }
        } else {
            let first = kind(chars[i]);
            while i < chars.len() && !chars[i].is_whitespace() && kind(chars[i]) == first {
                i += 1;
            }
        }
        out.push(chars[start..i].iter().collect());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoder() -> Encoder {
        // Bytes as GPT-2 writes them: \u0120 is a space.
        let vocab = Vocab::parse(
            r#"{"S": 0, "a": 1, "r": 2, "n": 3, "i": 4, "\u0120": 5, ",": 6, ".": 7,
                "ar": 8, "\u0120S": 9, "\u0120Sar": 10, "ni": 11, "nia": 12, "\u0120Sarnia": 13}"#,
        )
        .expect("the vocab reads");
        Encoder::new(
            &vocab,
            "#version: 0.2\na r\n\u{120} S\n\u{120}S ar\nn i\nni a\n\u{120}Sar nia\n",
        )
        .expect("the merges read")
    }

    #[test]
    fn a_word_joins_into_its_token() {
        assert_eq!(encoder().encode(" Sarnia").expect("it encodes"), [13]);
    }

    #[test]
    fn pairs_join_lowest_rank_first() {
        // "ar" ranks before "\u0120S", so " Sar" is " S" + "ar", not " Sa" + "r".
        assert_eq!(encoder().encode(" Sar").expect("it encodes"), [10]);
        assert_eq!(encoder().encode("rain").expect("it encodes"), [2, 1, 4, 3]);
    }

    #[test]
    fn punctuation_is_its_own_piece() {
        assert_eq!(
            encoder().encode(" Sarnia, Sarnia.").expect("it encodes"),
            [13, 6, 13, 7]
        );
    }

    #[test]
    fn text_is_cut_as_gpt2_cuts_it() {
        assert_eq!(
            pieces(" Sarnia, it's 290 km.  Then"),
            [
                " Sarnia", ",", " it", "'s", " 290", " km", ".", " ", " Then"
            ]
        );
    }

    #[test]
    fn the_words_file_reads_as_a_sentence() {
        assert_eq!(
            sentence("Sarnia\n  Green P \n\nForest\n"),
            " Sarnia, Green P, Forest."
        );
        assert_eq!(sentence("\n \n"), "");
    }

    #[test]
    fn what_came_before_comes_first_and_only_when_there_is_any() {
        assert_eq!(start(&[], &[1, 2]), [1, 2]);
        assert_eq!(start(&[7, 8], &[1, 2]), [PREVIOUS, 7, 8, 1, 2]);
    }

    #[test]
    fn only_the_latest_of_a_long_prompt_is_kept() {
        let long: Vec<usize> = (0..MOST + 5).collect();
        let ids = start(&long, &[1]);
        assert_eq!(ids.len(), 1 + MOST + 1);
        assert_eq!(ids[1], 5);
    }
}
