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
/// past whitespace and plain comments is a `[`. Rust is read as edition 2021
/// and later lex it; an older edition splits some names written against a
/// `#` or a quote otherwise. Where a comment, string or raw string is left
/// open at the file's end, or a character is left open where rustc stops
/// it, all of which rustc refuses, the line it opens on counts too, since
/// the walk may have read the file otherwise than rustc would and hidden a
/// lint turned off below it. Past a character left open, the walk reads on
/// from where rustc stops it. A file rustc never builds, such as a case
/// kept to fail compiling, counts each such line it holds too.
pub fn lines_turning_a_lint_off(text: &str) -> Vec<usize> {
    let mut code = Tokens {
        text,
        at: 0,
        left_open: Vec::new(),
        name_ends: 0,
    };
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
        // A comment left open past a shebang ends with its line, since
        // rustc reads on from the next.
        code.left_open.clear();
    }
    let mut start = 0;
    let mut lines = Vec::new();
    for (number, line) in text.split_inclusive('\n').enumerate() {
        if code.in_code(start) && turns_a_rust_lint_off(&text[start..]) {
            lines.push(number + 1);
        }
        start += line.len();
    }
    code.in_code(text.len());
    // The walk met each in the order it opens, so each line is counted on
    // from the last.
    let (mut line, mut counted) = (1, 0);
    for from in code.left_open {
        line += text[counted..from].matches('\n').count();
        counted = from;
        lines.push(line);
    }
    lines.sort_unstable();
    lines.dedup();
    lines
}

/// Whether a line of Rust opens with such an attribute. `rest` is the file
/// from the line on, since rustfmt breaks a long `cfg_attr` across lines,
/// and each attribute opening on the line is read to its close.
fn turns_a_rust_lint_off(rest: &str) -> bool {
    let mut tokens = Tokens {
        text: rest,
        at: 0,
        left_open: Vec::new(),
        name_ends: 0,
    };
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
    /// Where each comment, string, raw string or character rustc leaves open
    /// opened, in the order the walk met them.
    left_open: Vec<usize>,
    /// Where the last name the walk passed whole, a suffix or a raw name,
    /// ends, so a `b` ending one is read as rustc reads it, as no byte
    /// character's prefix.
    name_ends: usize,
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
        let (mut depth, from) = (0_usize, self.at);
        while self.at < self.text.len() {
            if self.ahead("/*") {
                depth += 1;
                self.at += 2;
            } else if self.ahead("*/") {
                depth -= 1;
                self.at += 2;
                if depth == 0 {
                    return;
                }
            } else {
                self.bump();
            }
        }
        self.left_open.push(from);
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
        self.name_ends = self.at;
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

    /// Whether the quote here opens a byte character: a `b` no name runs
    /// into stands right before it, as rustc's `c_or_byte_string` reads one,
    /// and ends no suffix or raw name, which rustc has read whole.
    fn opens_byte(&self) -> bool {
        let mut before = self.text[..self.at].chars().rev();
        self.name_ends != self.at
            && before.next() == Some('b')
            && !before.next().is_some_and(is_name)
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
        let from = self.at;
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
        self.left_open.push(from);
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
        if let Some(n) = self.text[from..].find(&close) {
            self.at = from + n + close.len();
            self.pass_name();
        } else {
            self.left_open.push(self.at);
            self.at = self.text.len();
        }
        true
    }

    /// Passes a character from its opening quote, and its suffix, saying
    /// whether one stood there rather than a lifetime, as rustc's
    /// `lifetime_or_char` tells them apart. A quote before a letter, digit or
    /// `_` opens a lifetime unless a quote follows that one character or the
    /// name it opens, which closes a character; so here does one before any
    /// character past ASCII, though rustc reads one before a symbol there as
    /// a character. A byte character's quote, after a `b` no name runs into
    /// and ending no suffix or raw name, opens no lifetime. One left open is
    /// read on from where rustc stops it.
    fn pass_character(&mut self) -> bool {
        let after = &self.text[self.at + 1..];
        let mut chars = after.chars();
        match (chars.next(), chars.next()) {
            (Some(c), Some('\'')) if c != '\\' => {
                self.at += 2 + c.len_utf8();
                self.pass_name();
                return true;
            }
            (Some(c), _)
                if !self.opens_byte()
                    && (c.is_ascii_alphanumeric() || c == '_' || !c.is_ascii()) =>
            {
                let name = after.len() - after[c.len_utf8()..].trim_start_matches(is_name).len();
                if !after[name..].starts_with('\'') {
                    return false;
                }
                self.at += name + 2;
                return true;
            }
            _ => {}
        }
        match single_quoted(after) {
            Ok(length) => {
                self.at += 1 + length;
                self.pass_name();
            }
            Err(stop) => {
                self.left_open.push(self.at);
                self.at += 1 + stop;
            }
        }
        true
    }
}

/// How far a character runs `after` its opening quote, as rustc's
/// `single_quoted_string` reads it, taking each escape and what it escapes
/// as one: to just past its close, or, left open, to the `/`, the line's
/// end no quote follows, or the file's end that stops it.
fn single_quoted(after: &str) -> Result<usize, usize> {
    let mut chars = after.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        match c {
            '\'' => return Ok(at + 1),
            '/' => return Err(at),
            '\n' if chars.peek().is_none_or(|&(_, next)| next != '\'') => return Err(at),
            '\\' => {
                chars.next();
            }
            _ => {}
        }
    }
    Err(after.len())
}

#[cfg(test)]
#[path = "attributes_tests.rs"]
mod tests;
