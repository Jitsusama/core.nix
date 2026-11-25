//! Whether a line of Rust turns a lint off, read as Rust reads its
//! attributes: across lines to each one's close, past comments, strings and
//! characters, and into a `cfg_attr`'s attributes, for the suppressions
//! measure. It's apart from measure.rs because it changes only with how
//! Rust writes an attribute.

/// Whether a line of Rust opens with an attribute turning a lint off:
/// `allow` or `expect`, inner or outer, or a `cfg_attr` holding one among
/// its attributes, since a lint turned off under a condition is off. `rest`
/// is the file from the line on, since rustfmt breaks a long `cfg_attr`
/// across lines, and each attribute opening on the line is read to its
/// close. A lint turned off part way through a line, as on a parameter,
/// isn't seen, nor one named rawly, as `r#allow`, nor a byte raw string's
/// words inside an attribute.
pub fn turns_a_rust_lint_off(rest: &str) -> bool {
    let mut tokens = Tokens { text: rest, at: 0 };
    std::iter::from_fn(|| tokens.attribute_on_its_line()).any(|inner| holds_a_lint_off(&inner))
}

/// Whether an attribute, read as its words inside the brackets with
/// whitespace, comments and what its strings say gone, turns a lint off.
/// A `cfg_attr`'s attributes are each read so, its condition among them,
/// since no condition opens as a lint level does.
fn holds_a_lint_off(inner: &str) -> bool {
    if ["allow(", "expect("]
        .iter()
        .any(|level| inner.starts_with(level))
    {
        return true;
    }
    let Some(given) = inner.strip_prefix("cfg_attr(") else {
        return false;
    };
    let mut attributes = Vec::new();
    let (mut depth, mut from) = (0_usize, 0);
    for (at, c) in given.char_indices() {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => {
                attributes.push(&given[from..at]);
                break;
            }
            ')' => depth -= 1,
            ',' if depth == 0 => {
                attributes.push(&given[from..at]);
                from = at + 1;
            }
            _ => {}
        }
    }
    attributes
        .iter()
        .any(|attribute| holds_a_lint_off(attribute))
}

/// Rust's own whitespace, which holds the direction marks but no no-break
/// space.
fn is_rust_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n'
            | '\u{b}'
            | '\u{c}'
            | '\r'
            | ' '
            | '\u{85}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{2028}'
            | '\u{2029}'
    )
}

/// Rust read far enough to find its attributes: what stands between
/// tokens, strings and characters, and the brackets around an attribute.
struct Tokens<'a> {
    text: &'a str,
    at: usize,
}

impl Tokens<'_> {
    fn ahead(&self, said: &str) -> bool {
        self.text[self.at..].starts_with(said)
    }

    fn peek(&self) -> Option<char> {
        self.text[self.at..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.at += c.len_utf8();
        Some(c)
    }

    /// Passes whitespace and comments, and a byte-order mark, which opens a
    /// file as whitespace would, saying whether a line ended among them.
    fn pass_space(&mut self) -> bool {
        let from = self.at;
        loop {
            if self.ahead("//") {
                self.at = self.text[self.at..]
                    .find('\n')
                    .map_or(self.text.len(), |n| self.at + n);
            } else if self.ahead("/*") {
                // Rust's block comments nest.
                let mut depth = 0_usize;
                while self.at < self.text.len() {
                    if self.ahead("/*") {
                        depth += 1;
                        self.at += 2;
                    } else if self.ahead("*/") {
                        depth -= 1;
                        self.at += 2;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        self.bump();
                    }
                }
            } else if self
                .peek()
                .is_some_and(|c| is_rust_space(c) || c == '\u{feff}')
            {
                self.bump();
            } else {
                return self.text[from..self.at].contains('\n');
            }
        }
    }

    /// The words inside the next attribute's brackets, where one opens
    /// before the line ends.
    fn attribute_on_its_line(&mut self) -> Option<String> {
        if self.pass_space() || self.bump()? != '#' {
            return None;
        }
        self.pass_space();
        if self.peek() == Some('!') {
            self.bump();
            self.pass_space();
        }
        if self.bump()? != '[' {
            return None;
        }
        let mut inner = String::new();
        let mut depth = 0_usize;
        loop {
            self.pass_space();
            let follows_a_word = inner.ends_with(|c: char| c.is_alphanumeric() || c == '_');
            match self.peek()? {
                '"' => {
                    self.pass_string();
                    inner.push_str("\"\"");
                }
                'r' if !follows_a_word && self.pass_raw_string() => inner.push_str("\"\""),
                '\'' if self.pass_character() => inner.push_str("''"),
                ']' if depth == 0 => {
                    self.bump();
                    return Some(inner);
                }
                c => {
                    match c {
                        '[' => depth += 1,
                        ']' => depth -= 1,
                        _ => {}
                    }
                    inner.push(c);
                    self.bump();
                }
            }
        }
    }

    /// Passes a string from its opening quote, escapes and all.
    fn pass_string(&mut self) {
        self.bump();
        while let Some(c) = self.bump() {
            match c {
                '\\' => {
                    self.bump();
                }
                '"' => return,
                _ => {}
            }
        }
    }

    /// Passes a raw string from its `r`, saying whether one stood there.
    fn pass_raw_string(&mut self) -> bool {
        let after = &self.text[self.at + 1..];
        let hashes = after.chars().take_while(|&c| c == '#').count();
        if !after[hashes..].starts_with('"') {
            return false;
        }
        let close = format!("\"{}", "#".repeat(hashes));
        let from = self.at + 2 + hashes;
        self.at = self.text[from..]
            .find(&close)
            .map_or(self.text.len(), |n| from + n + close.len());
        true
    }

    /// Passes a character from its opening quote, saying whether one stood
    /// there rather than a lifetime.
    fn pass_character(&mut self) -> bool {
        let after = &self.text[self.at + 1..];
        let mut chars = after.chars();
        let length = match (chars.next(), chars.next()) {
            // The close follows what's escaped, which may itself be a quote.
            (Some('\\'), _) => after
                .get(2..)
                .and_then(|escaped| escaped.find('\''))
                .map(|n| n + 3),
            (Some(c), Some('\'')) => Some(c.len_utf8() + 1),
            _ => None,
        };
        length.is_some_and(|length| {
            self.at += 1 + length;
            true
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::measure::{file, measure};

    #[test]
    fn a_rust_lint_is_counted_off_however_its_attribute_is_written() {
        // Each line opens with a quote, so this file's own lines aren't counted.
        let files = [file(
            "f.rs",
            concat!(
                "#[cfg_attr(test, allow(dead_code))]\n",
                "# [allow(unused)]\n",
                "#![cfg_attr(not(test), expect(clippy::x))]\n",
                "#[cfg_attr(test, derive(Debug))]\n",
                "# ! [ expect ( x ) ]\n",
                "#[cfg_attr(feature = \"x\", deny(y))]\n",
                "let s = \"#[cfg_attr(test, allow(x))]\";\n",
                "#[cfg_attr(test, disallow(x))]\n",
                "#[cfg_attr(test, derive(Debug), allow(x))]\n",
                "#[cfg_attr(a, cfg_attr(b, expect(x)))]\n",
                "#[allow_internal_unstable(x)]\n",
                "#[derive(Debug, allow(x))]\n",
                "#[cfg_attr(test, derive(Debug))] // a, allow(x) later\n",
                "#[cfg_attr(doc, doc = \"a, allow(x) here\")]\n",
                "#[cfg_attr(feature = \"a\\\", allow(b)\", deny(x))]\n",
                "#\u{a0}[allow(x)]\n",
                "#\u{200e}[allow(x)]\n",
                "#[cfg_attr(test, derive(Debug, allow(x)))]\n",
                "#[cfg_attr(\n",
                "    not(test), // a comment's ) and ]\n",
                "    expect(unused)\n",
                ")]\n",
                "\u{200e}\u{feff}\t#[allow(x)]\n",
                "#[derive(Debug)] #[allow(x)]\n",
                "#[derive(Debug)]\n",
                "#[allow(x)]\n",
                "#[cfg_attr(feature = r#\"a\")\"#, allow(x))]\n",
                "#[cfg_attr(feature = r#\"a\", allow(x)\"#, deny(x))]\n",
                "# /* a, allow(y) */ [allow(x)]\n",
                "#[cfg_attr(test, doc = ')', allow(x))]\n",
                "#[cfg_attr(test, doc = '\\\"', allow(x))]\n",
                "#[allow(x)\n",
            ),
        )];
        let health = measure(&["f.rs".to_owned()], &files, &[]);
        assert_eq!(
            health.named["suppressions"],
            [
                "f.rs:1", "f.rs:2", "f.rs:3", "f.rs:5", "f.rs:9", "f.rs:10", "f.rs:17", "f.rs:19",
                "f.rs:23", "f.rs:24", "f.rs:26", "f.rs:27", "f.rs:29", "f.rs:30", "f.rs:31"
            ],
            "a lint turned off through `cfg_attr`, across lines, past another attribute or \
             with Rust's whitespace or a comment in its attribute was missed, or an \
             attribute turning none off, a comment, a string or a lint inside another \
             attribute was counted"
        );
    }
}
