//! Whisper's words: vocab.json read into each token's bytes, and a run of
//! tokens turned back into text.
//!
//! vocab.json is one flat object, each token's text to its number. The
//! text is GPT-2's: every byte stands as a printable character, so a token
//! is turned back into bytes one character at a time.

use std::collections::HashMap;

/// Each token's bytes, by its number.
pub struct Vocab {
    tokens: Vec<Vec<u8>>,
}

impl Vocab {
    /// Reads vocab.json.
    ///
    /// # Errors
    ///
    /// When the text isn't a flat object of strings to numbers, or a token
    /// holds a character no byte stands for.
    pub fn parse(json: &str) -> Result<Self, String> {
        let bytes = byte_decoder();
        let mut tokens: Vec<Vec<u8>> = Vec::new();
        let mut reader = Reader {
            chars: json.chars().peekable(),
        };
        reader.expect('{')?;
        loop {
            reader.skip_space();
            if reader.take('}') {
                break;
            }
            let text = reader.string()?;
            reader.skip_space();
            reader.expect(':')?;
            reader.skip_space();
            let id = reader.number()?;
            let token = text
                .chars()
                .map(|c| {
                    bytes
                        .get(&c)
                        .copied()
                        .ok_or_else(|| format!("no byte is {c:?}"))
                })
                .collect::<Result<Vec<u8>, String>>()?;
            if tokens.len() <= id {
                tokens.resize(id + 1, Vec::new());
            }
            tokens[id] = token;
            reader.skip_space();
            if !reader.take(',') {
                reader.skip_space();
                reader.expect('}')?;
                break;
            }
        }
        Ok(Self { tokens })
    }

    /// The text of the tokens below `first_special`, the rest left out,
    /// with the space Whisper starts its first word with trimmed.
    #[must_use]
    pub fn text(&self, ids: &[usize], first_special: usize) -> String {
        let bytes: Vec<u8> = ids
            .iter()
            .filter(|&&id| id < first_special)
            .filter_map(|&id| self.tokens.get(id))
            .flatten()
            .copied()
            .collect();
        String::from_utf8_lossy(&bytes).trim().to_owned()
    }

    /// How many numbers have a token.
    #[must_use]
    pub fn count(&self) -> usize {
        self.tokens.len()
    }

    /// Each token's number, by its bytes.
    #[must_use]
    pub fn numbers(&self) -> HashMap<Vec<u8>, usize> {
        self.tokens
            .iter()
            .enumerate()
            .filter(|(_, bytes)| !bytes.is_empty())
            .map(|(id, bytes)| (bytes.clone(), id))
            .collect()
    }
}

/// GPT-2's table from the character each byte is written as to the byte:
/// printable Latin-1 stands for itself, and the other 68 bytes are written
/// as the characters from U+0100 up, in order.
pub fn byte_decoder() -> HashMap<char, u8> {
    let printable = |b: u8| matches!(b, b'!'..=b'~' | 0xa1..=0xac | 0xae..=0xff);
    let mut table = HashMap::new();
    let mut shifted = 0x100_u32;
    for b in 0..=255_u8 {
        let c = if printable(b) {
            char::from(b)
        } else {
            let c = char::from_u32(shifted).expect("U+0100 to U+0143 are characters");
            shifted += 1;
            c
        };
        table.insert(c, b);
    }
    table
}

struct Reader<I: Iterator<Item = char>> {
    chars: std::iter::Peekable<I>,
}

impl<I: Iterator<Item = char>> Reader<I> {
    fn skip_space(&mut self) {
        while self.chars.next_if(|c| c.is_whitespace()).is_some() {}
    }

    fn take(&mut self, want: char) -> bool {
        self.chars.next_if_eq(&want).is_some()
    }

    fn expect(&mut self, want: char) -> Result<(), String> {
        self.skip_space();
        if self.take(want) {
            Ok(())
        } else {
            Err(format!("wanted {want:?}, found {:?}", self.chars.peek()))
        }
    }

    fn number(&mut self) -> Result<usize, String> {
        let mut digits = String::new();
        while let Some(d) = self.chars.next_if(char::is_ascii_digit) {
            digits.push(d);
        }
        digits
            .parse()
            .map_err(|_| format!("wanted a number, found {digits:?}"))
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let digits: String = (0..4).filter_map(|_| self.chars.next()).collect();
        u32::from_str_radix(&digits, 16).map_err(|_| format!("bad escape \\u{digits}"))
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            match self.chars.next().ok_or("the text ended inside a string")? {
                '"' => return Ok(out),
                '\\' => out.push(self.escape()?),
                c => out.push(c),
            }
        }
    }

    fn escape(&mut self) -> Result<char, String> {
        Ok(
            match self.chars.next().ok_or("the text ended inside an escape")? {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                'b' => '\u{8}',
                'f' => '\u{c}',
                'u' => {
                    let high = self.hex4()?;
                    let code = if (0xd800..0xdc00).contains(&high) {
                        if !(self.take('\\') && self.take('u')) {
                            return Err("a lone high surrogate".to_owned());
                        }
                        let low = self.hex4()?;
                        0x10000 + ((high - 0xd800) << 10) + (low - 0xdc00)
                    } else {
                        high
                    };
                    char::from_u32(code).ok_or_else(|| format!("no character U+{code:X}"))?
                }
                c => c,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_byte_has_one_character() {
        let table = byte_decoder();
        assert_eq!(table.len(), 256);
        assert_eq!(table[&'\u{120}'], b' ');
        assert_eq!(table[&'\u{10a}'], b'\n');
        assert_eq!(table[&'a'], b'a');
    }

    #[test]
    fn tokens_read_back_into_text() {
        let vocab =
            Vocab::parse(r#"{"And": 0, "\u0120so": 1, "\u0120caf\u00c3\u00a9": 2, "!": 3}"#)
                .expect("the vocab reads");
        assert_eq!(vocab.count(), 4);
        assert_eq!(vocab.text(&[0, 1, 2, 3], 4), "And so café!");
    }

    #[test]
    fn specials_are_left_out() {
        let vocab = Vocab::parse(r#"{"hi": 0, "<|endoftext|>": 1}"#).expect("the vocab reads");
        assert_eq!(vocab.text(&[0, 1], 1), "hi");
    }

    #[test]
    fn escapes_read_as_json_writes_them() {
        let vocab = Vocab::parse(r#"{"\"\\\/": 0}"#).expect("the vocab reads");
        assert_eq!(vocab.text(&[0], 1), r#""\/"#);
    }

    #[test]
    fn a_character_no_byte_is_refused() {
        assert!(Vocab::parse(r#"{"\u4e00": 0}"#).is_err());
    }

    #[test]
    fn a_broken_object_is_refused() {
        assert!(Vocab::parse(r#"{"a": 0 "b": 1}"#).is_err());
        assert!(Vocab::parse(r#"{"a": }"#).is_err());
    }
}
