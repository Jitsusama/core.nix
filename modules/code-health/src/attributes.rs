//! Whether a line of Rust turns a lint off, read as Rust reads its
//! attributes: across lines to each one's close, past comments, strings and
//! characters, and into a `cfg_attr`'s attributes, for the suppressions
//! measure, with the gaps `lines_turning_a_lint_off` names. It's apart from
//! measure.rs so it changes only with how Rust writes an attribute, though
//! its cases are read through `measure`, as the figure is.

/// The lines of a Rust file, counted from 1, that open with an attribute
/// turning a lint off: `allow` or `expect`, inner or outer, or a `cfg_attr`
/// holding one among its attributes, since a lint turned off under a
/// condition is off. Only a line opening in code is read, not one inside a
/// comment, string or character begun above it, so the file is walked from
/// its start. A lint turned off part way through a line, as on a parameter
/// or past a comment closing on its line, isn't seen. One named rawly, as
/// `r#allow`, is Rust's own `allow`, so it counts. A first line opening `#!`,
/// past a byte-order mark, is a shebang rustc drops, unless what follows
/// past whitespace and plain comments is a `[`.
pub fn lines_turning_a_lint_off(text: &str) -> Vec<usize> {
    let mut code = Tokens { text, at: 0 };
    if text
        .strip_prefix('\u{feff}')
        .unwrap_or(text)
        .starts_with("#!")
    {
        code.at = text.find("#!").map_or(0, |n| n + 2);
        code.pass_plain_space();
        code.at = if code.ahead("[") {
            0
        } else {
            text.find('\n').unwrap_or(text.len())
        };
    }
    let mut start = 0;
    let mut lines = Vec::new();
    for (number, line) in text.split_inclusive('\n').enumerate() {
        if code.in_code(start) && turns_a_rust_lint_off(&text[start..]) {
            lines.push(number + 1);
        }
        start += line.len();
    }
    lines
}

/// Whether a line of Rust opens with such an attribute. `rest` is the file
/// from the line on, since rustfmt breaks a long `cfg_attr` across lines,
/// and each attribute opening on the line is read to its close.
fn turns_a_rust_lint_off(rest: &str) -> bool {
    let mut tokens = Tokens { text: rest, at: 0 };
    std::iter::from_fn(|| tokens.attribute_on_its_line()).any(|inner| holds_a_lint_off(&inner))
}

/// Whether an attribute, read as its words inside the brackets with
/// whitespace, comments and what its strings say gone, turns a lint off.
/// A `cfg_attr`'s attributes are each read so, its condition among them,
/// since no condition opens as a lint level does. They're split only at
/// commas outside any of their own brackets, of whatever shape.
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
            '(' | '[' | '{' => depth += 1,
            ')' if depth == 0 => {
                attributes.push(&given[from..at]);
                break;
            }
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
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

/// Whether `c` may be part of a name: an ASCII letter, digit or underscore,
/// or any character past ASCII but Rust's whitespace.
fn is_name(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || !c.is_ascii() && !is_rust_space(c)
}

/// Rust read far enough to find its attributes: what stands between
/// tokens, strings and characters, and the brackets around an attribute.
struct Tokens<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Tokens<'a> {
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

    /// Moves on to `start`, past the code between, saying whether `start`
    /// lies in code rather than inside a comment, string or character
    /// opened before it.
    fn in_code(&mut self, start: usize) -> bool {
        while self.at < start {
            if self.ahead("//") || self.ahead("/*") {
                self.pass_comment();
            } else if self.ahead("\"") {
                self.pass_string();
            } else if self.pass_raw_name().is_none()
                && !(self.ahead("r") && self.opens_raw() && self.pass_raw_string())
                && !(self.ahead("'") && self.pass_character())
            {
                self.bump();
            }
        }
        self.at == start
    }

    /// Passes the comment opening here: a line comment to its line's end,
    /// a block comment to its close, since Rust's nest.
    fn pass_comment(&mut self) {
        if self.ahead("//") {
            self.at = self.text[self.at..]
                .find('\n')
                .map_or(self.text.len(), |n| self.at + n);
            return;
        }
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
    }

    /// Passes what rustc passes after a shebang's `#!` in deciding whether
    /// it is one: Rust's whitespace and plain comments, but no byte-order
    /// mark or doc comment.
    fn pass_plain_space(&mut self) {
        loop {
            if (self.ahead("//") || self.ahead("/*")) && !self.at_doc_comment() {
                self.pass_comment();
            } else if self.peek().is_some_and(is_rust_space) {
                self.bump();
            } else {
                return;
            }
        }
    }

    /// Whether a doc comment opens here: `//!`, `/*!`, or `///` or `/**`
    /// but for `////`, `/***` and `/**/`, which are plain.
    fn at_doc_comment(&self) -> bool {
        let rest = &self.text[self.at..];
        rest.starts_with("//!")
            || rest.starts_with("/*!")
            || rest.starts_with("///") && !rest.starts_with("////")
            || rest.starts_with("/**") && !rest.starts_with("/***") && !rest.starts_with("/**/")
    }

    /// Passes whitespace and comments, and a byte-order mark, which opens a
    /// file as whitespace would, saying whether a line ended among them;
    /// with `to_line_end`, it stops at the first that does, so a long run
    /// of comments below a line isn't walked for each.
    fn pass_space(&mut self, to_line_end: bool) -> bool {
        let mut ended = false;
        while !(to_line_end && ended) {
            let from = self.at;
            if self.ahead("//") || self.ahead("/*") {
                self.pass_comment();
            } else if self
                .peek()
                .is_some_and(|c| is_rust_space(c) || c == '\u{feff}')
            {
                self.bump();
            } else {
                return ended;
            }
            ended = ended || self.text[from..self.at].contains('\n');
        }
        true
    }

    /// Passes a raw name, `r#` and a name, saying what it was, so its last
    /// letter isn't read as an `r` opening a raw string. One a name runs
    /// into is none: its `r` ends a number's suffix.
    fn pass_raw_name(&mut self) -> Option<&'a str> {
        let (text, from) = (self.text, self.at);
        if !self.ahead("r#")
            || !text[from + 2..].chars().next().is_some_and(is_name)
            || text[..from].chars().next_back().is_some_and(is_name)
        {
            return None;
        }
        self.at += 2;
        self.pass_name();
        Some(&text[from..self.at])
    }

    /// Passes the name here, as a string's suffix or the rest of a raw
    /// name.
    fn pass_name(&mut self) {
        while self.peek().is_some_and(is_name) {
            self.bump();
        }
    }

    /// Whether the `r` here may open a raw string: no name or lifetime runs
    /// into it, but for a byte or C string's `b` or `c`, since Rust
    /// reserves any other name written right against a string.
    fn opens_raw(&self) -> bool {
        let named = |c: char| is_name(c) || c == '\'';
        let mut before = self.text[..self.at].chars().rev();
        match before.next() {
            Some('b' | 'c') => !before.next().is_some_and(named),
            Some(c) => !named(c),
            None => true,
        }
    }

    /// The words inside the next attribute's brackets, where one opens
    /// before the line ends.
    fn attribute_on_its_line(&mut self) -> Option<String> {
        if self.pass_space(true) || self.bump()? != '#' {
            return None;
        }
        self.pass_space(false);
        if self.peek() == Some('!') {
            self.bump();
            self.pass_space(false);
        }
        if self.bump()? != '[' {
            return None;
        }
        let mut inner = String::new();
        let mut depth = 0_usize;
        loop {
            self.pass_space(false);
            if let Some(name) = self.pass_raw_name() {
                inner.push_str(&name[2..]);
                continue;
            }
            match self.peek()? {
                '"' => {
                    self.pass_string();
                    inner.push_str("\"\"");
                }
                'r' if self.opens_raw() && self.pass_raw_string() => inner.push_str("\"\""),
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

    /// Passes a string from its opening quote, escapes and all, and its
    /// suffix.
    fn pass_string(&mut self) {
        self.bump();
        while let Some(c) = self.bump() {
            match c {
                '\\' => {
                    self.bump();
                }
                '"' => return self.pass_name(),
                _ => {}
            }
        }
    }

    /// Passes a raw string from its `r`, and its suffix, saying whether one
    /// stood there.
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
        self.pass_name();
        true
    }

    /// Passes a character from its opening quote, and its suffix, saying
    /// whether one stood there rather than a lifetime.
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
        let Some(length) = length else {
            return false;
        };
        self.at += 1 + length;
        self.pass_name();
        true
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

    #[test]
    fn an_attribute_holding_brackets_closes_at_its_own() {
        let files = [file("g.rs", "#[my_attr[x]] #[allow(y)]\n")];
        let health = measure(&["g.rs".to_owned()], &files, &[]);
        assert_eq!(
            health.named["suppressions"],
            ["g.rs:1"],
            "a bracket inside an attribute closed it, so the lint turned off past it was missed"
        );
    }

    #[test]
    fn a_raw_string_after_a_letter_or_a_word_is_read_raw() {
        let files = [file(
            "r.rs",
            concat!(
                "#[cfg_attr(test, doc = br\"\\\", allow(x))]\n",
                "#[cfg_attr(test, doc = cr\"\\\", allow(x))]\n",
                "#[cfg_attr(test, path(a r\"\\\"), allow(x))]\n",
            ),
        )];
        let health = measure(&["r.rs".to_owned()], &files, &[]);
        assert_eq!(
            health.named["suppressions"],
            ["r.rs:1", "r.rs:2", "r.rs:3"],
            "a byte or C raw string, or a raw string after a word, was read as a plain one, \
             so its closing quote was taken as escaped and the lint turned off past it missed"
        );
    }

    #[test]
    fn a_raw_names_last_letter_opens_no_raw_string() {
        let files = [file(
            "n.rs",
            concat!(
                "m!(r#r\"\\\";\n#[allow(x)]\n\");\n#[allow(x)]\n",
                "m!('r#r\"\\\";\n#[allow(x)]\n\");\n#[allow(x)]\n",
                "#[cfg_attr(test, m(r#r\"\\\"]\"), allow(x))]\n",
                "m!(1r#r\"\\\");\n#[allow(x)]\n// \");\n",
                "m!('a'r#r\"\\\");\n#[allow(x)]\n// \");\n",
            ),
        )];
        let health = measure(&["n.rs".to_owned()], &files, &[]);
        assert_eq!(
            health.named["suppressions"],
            ["n.rs:4", "n.rs:8", "n.rs:9", "n.rs:11", "n.rs:14"],
            "a raw name or lifetime's last r was read as opening a raw string, so a string \
             after it was misread, or a number's or character's suffix r and a # as a raw name"
        );
    }

    #[test]
    fn a_lint_named_rawly_is_counted_off() {
        let files = [file(
            "p.rs",
            concat!(
                "#[r#allow(x)]\n",
                "#[cfg_attr(test, r#expect(x))]\n",
                "#![r#allow(x)]\n",
            ),
        )];
        let health = measure(&["p.rs".to_owned()], &files, &[]);
        assert_eq!(
            health.named["suppressions"],
            ["p.rs:1", "p.rs:2", "p.rs:3"],
            "a lint level named rawly, which rustc reads as the level itself, was missed"
        );
    }

    #[test]
    fn a_suffix_opens_no_raw_string_and_rusts_whitespace_ends_a_name() {
        // Lines 2 and 6 sit in a string after a string's suffix ending r;
        // each third line from 10 to 28 follows a raw string after Rust's
        // whitespace past ASCII, and line 33 one closing at its second hash.
        let files = [file(
            "s.rs",
            concat!(
                "m!(\"a\"r\"\\\";\n#[allow(x)]\n\");\n#[allow(x)]\n",
                "m!(r\"a\"r\"\\\";\n#[allow(x)]\n\");\n#[allow(x)]\n",
                "m!(\u{85}r\"\\\");\n#[allow(x)]\n// \");\n",
                "m!(\u{200e}r\"\\\");\n#[allow(x)]\n// \");\n",
                "m!(\u{200f}r\"\\\");\n#[allow(x)]\n// \");\n",
                "m!(\u{2028}r\"\\\");\n#[allow(x)]\n// \");\n",
                "m!(\u{2029}r\"\\\");\n#[allow(x)]\n// \");\n",
                "m!(\u{2028}br\"\\\");\n#[allow(x)]\n// \");\n",
                "m!(r#x\u{2028}r\"\\\");\n#[allow(x)]\n// \");\n",
                "const R: &str = r##\"a \"# b\n#[allow(x)]\n\"##;\n#[allow(x)]\n",
            ),
        )];
        let health = measure(&["s.rs".to_owned()], &files, &[]);
        assert_eq!(
            health.named["suppressions"],
            [
                "s.rs:4", "s.rs:8", "s.rs:10", "s.rs:13", "s.rs:16", "s.rs:19", "s.rs:22",
                "s.rs:25", "s.rs:28", "s.rs:33"
            ],
            "a string's suffix was read as opening a raw string, Rust's whitespace past \
             ASCII as part of a name, or a raw string as closed at fewer hashes than \
             opened it"
        );
    }

    #[test]
    fn a_shebang_is_dropped_and_a_bang_opening_an_attribute_is_none() {
        let files = [
            file(
                "t.rs",
                "\u{feff}#!/x \"\n/* \"\n#[allow(x)]\n*/\n#[allow(x)]\n",
            ),
            file("u.rs", "#!/x \"\n#[allow(x)]\n// \"\n#[allow(x)]\n"),
            file("v.rs", "#! /* a */ [allow(x)]\n#[allow(x)]\n"),
            file("w.rs", "\u{feff}#![allow(x)]\n"),
        ];
        let paths = ["t.rs", "u.rs", "v.rs", "w.rs"].map(str::to_owned);
        let health = measure(&paths, &files, &[]);
        assert_eq!(
            health.named["suppressions"],
            ["t.rs:5", "u.rs:2", "u.rs:4", "v.rs:1", "v.rs:2", "w.rs:1"],
            "a shebang's quote or comment was read as opening one, or a `#!` opening an \
             attribute, past a byte-order mark or not, was dropped as a shebang"
        );
    }

    #[test]
    fn a_doc_comment_or_byte_order_mark_past_a_bang_makes_it_a_shebang() {
        let files = [
            file("a.rs", "#!/// a\n[allow(x)]\n"),
            file("b.rs", "#!//// a\n[allow(x)]\n"),
            file("c.rs", "#!//! a\n[allow(x)]\n"),
            file("d.rs", "#!/** */[ \"\n#[allow(x)]\n// \"\n"),
            file("e.rs", "#!/*** */[allow(x)]\n"),
            file("f.rs", "#!/**/[allow(x)]\n"),
            file("g.rs", "#!/*! */[allow(x)]\n"),
            file("h.rs", "#!\u{feff}[ \"\n#[allow(x)]\n// \"\n"),
        ];
        let paths = [
            "a.rs", "b.rs", "c.rs", "d.rs", "e.rs", "f.rs", "g.rs", "h.rs",
        ]
        .map(str::to_owned);
        let health = measure(&paths, &files, &[]);
        assert_eq!(
            health.named["suppressions"],
            ["b.rs:1", "d.rs:2", "e.rs:1", "f.rs:1", "h.rs:2"],
            "a doc comment or byte-order mark after a `#!` was passed as rustc passes only \
             whitespace and plain comments there, or a plain comment was taken for a doc one"
        );
    }

    #[test]
    fn only_a_line_opening_in_code_is_read() {
        // Each even line from 2 to 34 turns a lint off; those in code,
        // right after what would hide them from a walk misreading it, count.
        let files = [file(
            "k.rs",
            concat!(
                "/*\n#[allow(x)]\n*/\n#[allow(x)]\n",
                "const S: &str = \"\n#[allow(x)]\n\";\n#[allow(x)]\n",
                "const R: &str = r#\"\n#[allow(x)]\n\"#;\n#[allow(x)]\n",
                "/* /* */\n#[allow(x)]\n*/\n#[allow(x)]\n",
                "const Q: char = '\"'; const B: &[u8] = br\"\\\";\n#[allow(x)]\n",
                "m!('outer\"\\\"\n#[allow(x)]\n\");\n#[allow(x)]\n",
                "m!('r\"\\\"\n#[allow(x)]\n\");\n#[allow(x)]\n",
                "m!('br\"\\\"\n#[allow(x)]\n\");\n#[allow(x)]\n",
                "m!(xbr\"\\\"\n#[allow(x)]\n\");\n#[allow(x)]\n",
            ),
        )];
        let health = measure(&["k.rs".to_owned()], &files, &[]);
        assert_eq!(
            health.named["suppressions"],
            [
                "k.rs:4", "k.rs:8", "k.rs:12", "k.rs:16", "k.rs:18", "k.rs:22", "k.rs:26",
                "k.rs:30", "k.rs:34"
            ],
            "a line opening inside a comment, string or raw string begun above was counted, \
             or one opening in code past a character, a byte raw string or a lifetime missed"
        );
    }

    #[test]
    fn a_cfg_attr_splits_only_at_commas_outside_its_brackets() {
        let files = [file(
            "h.rs",
            concat!(
                "#[cfg_attr(test, doc[a, allow(x)])]\n",
                "#[cfg_attr(test, doc{a, allow(x)})]\n",
                "#[cfg_attr(test, doc[a], allow(y))]\n",
                "#[cfg_attr(test, doc{a}, allow(y))]\n",
                "#[cfg_attr(test, a}, allow(y))]\n",
            ),
        )];
        let health = measure(&["h.rs".to_owned()], &files, &[]);
        assert_eq!(
            health.named["suppressions"],
            ["h.rs:3", "h.rs:4", "h.rs:5"],
            "a cfg_attr was split inside an attribute's brackets, or not read on past their \
             close, or a close it never opened stopped its reading"
        );
    }
}
