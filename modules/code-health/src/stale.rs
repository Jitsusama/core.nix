//! The comments naming a path that's gone, read from the files alone: a
//! comment or docstring saying where something lives is a guide, and one
//! left behind when that thing moved sends its reader looking for nothing.
//! A path is named when it starts at one of the repository's top folders,
//! and it's there when it's found from the top or from any folder above
//! the file naming it, since a module's comments name its own `tests/`.
//!
//! core's comments-name-what-exists holds the same rule at each change,
//! through `code-health stale` (tree.rs).
//!
//! It reads generously, so a path it names is gone but a gone path may
//! pass. Only comments are read: a path in a string is often a test's
//! fixture, meant not to be there, and markdown is the docs check's to
//! hold. A shell heredoc's body is a command's text, so none of it is
//! comment, and a docstring or block comment the file never closes is
//! none either, since its language refuses the file. And a path is gone
//! only below a folder the repository holds under its top one, such as
//! `modules/a/`: `tools/token.c` or `tests/x.nix` is as likely another
//! project's, or a test's stand-in tree, and git's `refs/heads` is no path
//! at all. A path cut just before a hyphen, as `tests/vm/screens/screens-`
//! is, names where names begin rather than one file, so it's there while
//! any path begins with it, hyphen and all: `screens.nix` beside it begins
//! only with its letters. It's there too when found whole, the dash then
//! punctuation, as in `home.nix- which`.

use std::collections::HashSet;

use crate::measure::{File, extension};

/// Every comment of `files` naming a path `paths` doesn't hold, as the
/// file, its line and the path named.
pub fn stale(paths: &[String], files: &[File]) -> Vec<String> {
    let mut there: HashSet<&str> = HashSet::new();
    let mut tops: HashSet<&str> = HashSet::new();
    for path in paths {
        there.insert(path);
        let mut folder = path.as_str();
        while let Some((parent, _)) = folder.rsplit_once('/') {
            there.insert(parent);
            folder = parent;
        }
        if path.contains('/') && !folder.starts_with('.') {
            tops.insert(folder);
        }
    }
    let found = |file: &str, named: &str| {
        let mut folder = file;
        there.contains(named)
            || std::iter::from_fn(|| {
                let (parent, _) = folder.rsplit_once('/')?;
                folder = parent;
                Some(parent)
            })
            .any(|folder| there.contains(format!("{folder}/{named}").as_str()))
    };
    // Rare enough that reading every path for each is cheap. Each start
    // keeps the hyphen `named` was cut before.
    let begun = |file: &str, named: &str| {
        let mut folder = file;
        let above = std::iter::from_fn(|| {
            let (parent, _) = folder.rsplit_once('/')?;
            folder = parent;
            Some(format!("{parent}/{named}-"))
        });
        let starts: Vec<String> = std::iter::once(format!("{named}-")).chain(above).collect();
        paths
            .iter()
            .any(|path| starts.iter().any(|start| path.starts_with(start.as_str())))
    };
    let mut gone = Vec::new();
    for file in files {
        for (number, comment) in comments(file) {
            for (named, prefix) in named(comment, &tops) {
                // The folder under the top one the path names.
                let below = named
                    .match_indices('/')
                    .nth(1)
                    .map_or(named, |(at, _)| &named[..at]);
                // A path there whole passes even when a dash follows it
                // as punctuation.
                let here = found(&file.path, named) || (prefix && begun(&file.path, named));
                if there.contains(below) && !here {
                    gone.push(format!("{}:{number}: {named}", file.path));
                }
            }
        }
    }
    gone
}

/// A file's comment lines, numbered from one, block comments and
/// docstrings included: none for a language this doesn't know the
/// comments of. A docstring on a header's own line is read from its
/// quotes on, past the code before it.
fn comments(file: &File) -> Vec<(usize, &str)> {
    let extension = extension(&file.path);
    let (mark, blocks) = match extension {
        "rs" | "js" | "ts" | "qml" | "c" | "h" => ("//", true),
        "nix" => ("#", true),
        "sh" | "bash" | "py" | "toml" | "yaml" | "yml" => ("#", false),
        _ => return Vec::new(),
    };
    // The quotes a docstring opened with, which alone close it, while
    // one is open, and the same for a string in code, whose lines aren't
    // comment.
    let mut docstring: Option<&str> = None;
    let mut string: Option<&str> = None;
    let mut block = false;
    // The word closing the heredoc a line of shell opened, while its body
    // is read, and whether `<<-` lets tabs indent it.
    let mut heredoc: Option<(&str, bool)> = None;
    // Whether a docstring may stand on the next line of code: at a
    // module's top, or first in a def's or a class's body, past the colon
    // ending its header. Any other colon, as an if's or a dict key's, opens
    // no docstring, so a string there is code.
    let mut starts = true;
    // The logical line the code is in: the brackets it holds open, whether
    // a backslash carries it on, and whether it opened as a header.
    let mut logical = Logical::default();
    let mut lines = Vec::new();
    // How many lines were read before the last docstring or block comment
    // opened, so one the file never closes can be taken back.
    let mut opened = 0;
    for (number, line) in file.text.lines().enumerate() {
        // A byte-order mark opens a file as nothing at all.
        let line = if number == 0 {
            line.strip_prefix('\u{feff}').unwrap_or(line)
        } else {
            line
        };
        let trimmed = line.trim_start();
        let comment = if let Some((word, tabbed)) = heredoc {
            // A heredoc's body is text a command reads, though a line of it
            // may look like a comment, up to its word alone on a line.
            let last = if tabbed {
                line.trim_start_matches('\t')
            } else {
                line
            };
            if last == word {
                heredoc = None;
            }
            None
        } else if let Some(quotes) = docstring {
            (docstring, string) = docstring_line(line, quotes);
            starts = false;
            Some(line)
        } else if block {
            block = !line.contains("*/");
            Some(line)
        } else if let Some((quotes, past)) = opens(trimmed).filter(|_| extension == "py" && starts)
        {
            (docstring, string) = docstring_line(&trimmed[past..], quotes);
            starts = false;
            logical = Logical::default();
            opened = lines.len();
            Some(line)
        } else if blocks && trimmed.starts_with("/*") {
            block = !trimmed[2..].contains("*/");
            opened = lines.len();
            Some(line)
        } else {
            // Only a file's first line can be its shebang; past it, `#!` is
            // read as any line is.
            let shebang = number == 0 && line.starts_with("#!");
            let comment = (trimmed.starts_with(mark) && !shebang)
                || (extension == "nix" && trimmed.starts_with('*'));
            // A blank line, a comment or a shebang leaves where a statement
            // starts as it was. No line a string runs on through is comment.
            if string.is_some()
                || (extension == "py" && !comment && !trimmed.is_empty() && !shebang)
            {
                // A line a string runs on into carries its logical line on
                // rather than beginning one.
                let scanned = scan(line, string);
                if string.is_none() {
                    logical.begin(scanned.code);
                }
                let header = on_the_header(&logical, &scanned, line);
                string = scanned.open;
                // The brackets a line closes count though it leaves a
                // string open, so the logical line is carried before asking
                // for one.
                let ends = logical.ends_a_header(&scanned);
                starts = string.is_none() && ends;
                // The logical line has ended, and as the line runs on past
                // the header's colon, `starts` holds no docstring may follow.
                if let Some((text, quotes, past)) = header {
                    (docstring, string) = docstring_line(&text[past..], quotes);
                    opened = lines.len();
                }
                header.map(|(text, _, _)| text)
            } else {
                if matches!(extension, "sh" | "bash") && !comment {
                    heredoc = opens_a_heredoc(line);
                }
                comment.then_some(line)
            }
        };
        if let Some(comment) = comment {
            lines.push((number + 1, comment));
        }
    }
    // A docstring or a block comment the file never closes is no comment:
    // its language refuses the file, and what follows the opening may as
    // well be code.
    if docstring.is_some() || block {
        lines.truncate(opened);
    }
    lines
}

/// What [`scan`] makes of a line of Python.
struct Scanned<'a> {
    /// The code before a `#` outside any string.
    code: &'a str,
    /// The quotes of a string the line leaves open.
    open: Option<&'static str>,
    /// Whether the string the line starts inside closes on it.
    closed: bool,
    /// The brackets the code opens, less those it closes, outside strings.
    depth: isize,
    /// Where each colon outside strings stands in the line, with the
    /// brackets opened before it there, less those closed.
    colons: Vec<(usize, isize)>,
}

/// A Python logical line, which brackets left open or a closing backslash
/// carry across lines, as a header with its parameters on lines of their
/// own is.
#[derive(Default)]
struct Logical {
    depth: isize,
    carried: bool,
    header: bool,
}

impl Logical {
    /// Notes whether a line of code outside any string, opening a logical
    /// line rather than carrying one on, opens a def's or a class's
    /// header.
    fn begin(&mut self, code: &str) {
        if self.depth == 0 && !self.carried {
            let mut words = code.split_whitespace();
            self.header = match words.next() {
                Some("def" | "class") => true,
                Some("async") => words.next() == Some("def"),
                _ => false,
            };
        }
    }

    /// Carries the logical line past `scanned`, saying whether it ended
    /// there as a header does, on its colon.
    fn ends_a_header(&mut self, scanned: &Scanned) -> bool {
        let code = scanned.code.trim_end();
        self.depth = (self.depth + scanned.depth).max(0);
        self.carried = scanned.open.is_none() && code.ends_with('\\');
        self.depth == 0 && !self.carried && self.header && code.ends_with(':')
    }
}

/// The docstring standing past the colon that ends a def's or a class's
/// header on `line`, as `class K: """Says why."""`'s does: its text from
/// its quotes on, the quotes, and where past them it begins. The colon is
/// the first outside every bracket and string, so a parameter's, a
/// lambda's or a default's is passed by, and a statement past it, as
/// `return """x"""`, holds none.
fn on_the_header<'a>(
    logical: &Logical,
    scanned: &Scanned,
    line: &'a str,
) -> Option<(&'a str, &'static str, usize)> {
    if !logical.header {
        return None;
    }
    let colon = scanned
        .colons
        .iter()
        .find(|(_, depth)| logical.depth + depth == 0)?
        .0;
    let text = line[colon + 1..].trim_start();
    opens(text).map(|(quotes, past)| (text, quotes, past))
}

/// The docstring and the code string left open by a line read from inside
/// a docstring opened with `quotes`: the docstring still, until its own
/// quotes close it unescaped, and then any string the code past them opens.
fn docstring_line(
    line: &str,
    quotes: &'static str,
) -> (Option<&'static str>, Option<&'static str>) {
    let scanned = scan(line, Some(quotes));
    if scanned.closed {
        (None, scanned.open)
    } else {
        (Some(quotes), None)
    }
}

/// A line of Python code read past its strings and their escapes. `open`
/// holds the quotes of a string the line starts inside, as one the line
/// before left open.
fn scan<'a>(line: &'a str, mut open: Option<&'static str>) -> Scanned<'a> {
    let started = open.is_some();
    let mut closed = false;
    let mut depth = 0;
    let mut colons = Vec::new();
    let mut at = 0;
    while let Some(c) = line[at..].chars().next() {
        let rest = &line[at..];
        if let Some(quotes) = open {
            if c == '\\' {
                at += 1 + rest[1..].chars().next().map_or(0, char::len_utf8);
                continue;
            }
            if rest.starts_with(quotes) {
                at += quotes.len();
                open = None;
                closed = started;
                continue;
            }
        } else if c == '#' {
            return Scanned {
                code: &line[..at],
                open: None,
                closed,
                depth,
                colons,
            };
        } else if "([{".contains(c) {
            depth += 1;
        } else if ")]}".contains(c) {
            depth -= 1;
        } else if c == ':' {
            colons.push((at, depth));
        } else if let Some(quotes) = ["\"\"\"", "'''", "\"", "'"]
            .into_iter()
            .find(|quotes| rest.starts_with(quotes))
        {
            at += quotes.len();
            open = Some(quotes);
            continue;
        }
        at += c.len_utf8();
    }
    Scanned {
        code: line,
        // A one-quote string runs on past its line only where a backslash
        // ends it.
        open: open.filter(|quotes| quotes.len() == 3 || line.ends_with('\\')),
        closed,
        depth,
        colons,
    }
}

/// The quotes a Python docstring opens with at the start of `trimmed`,
/// past any of the letters that make a string raw, bytes or formatted, and
/// where in `trimmed` its text begins.
fn opens(trimmed: &str) -> Option<(&'static str, usize)> {
    let unprefixed = trimmed.trim_start_matches(|c: char| "rRbBfFuU".contains(c));
    let prefix = trimmed.len() - unprefixed.len();
    if prefix > 2 {
        return None;
    }
    ["\"\"\"", "'''"]
        .into_iter()
        .find(|quotes| unprefixed.starts_with(quotes))
        .map(|quotes| (quotes, prefix + quotes.len()))
}

/// The word closing the heredoc a line of shell opens, and whether `<<-`
/// lets tabs indent it. The word may follow spaces, a backslash or a
/// quote; a here-string's `<<<` opens none, nor does a shift inside `((`
/// or one onto a digit, as `let x=1<<2`'s, nor a `<<` an odd count of
/// quotes before it puts in a string.
fn opens_a_heredoc(line: &str) -> Option<(&str, bool)> {
    if line.contains("((") {
        return None;
    }
    let mut from = 0;
    while let Some(at) = line[from..].find("<<").map(|at| from + at) {
        from = at + 2;
        let before = &line[..at];
        if before.matches('"').count() % 2 == 1 || before.matches('\'').count() % 2 == 1 {
            continue;
        }
        let rest = &line[from..];
        let (rest, tabbed) = rest
            .strip_prefix('-')
            .map_or((rest, false), |rest| (rest, true));
        let rest = rest.trim_start();
        let rest = rest.strip_prefix(['\\', '\'', '"']).unwrap_or(rest);
        let length = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        let word = &rest[..length];
        if word.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
            return Some((word, tabbed));
        }
    }
    None
}

/// The paths `text` names that start at one of `tops`, each from the start
/// of a word and cut at its last letter, digit or underscore, and whether
/// it was cut just before a hyphen, naming where names begin.
fn named<'a>(text: &'a str, tops: &HashSet<&str>) -> Vec<(&'a str, bool)> {
    let inside = |c: char| c.is_ascii_alphanumeric() || "_.@+/-".contains(c);
    let before = |c: char| c.is_alphanumeric() || "_./${}-".contains(c);
    let mut paths = Vec::new();
    let mut rest = 0;
    for (start, _) in text.char_indices() {
        if start < rest || text[..start].chars().next_back().is_some_and(before) {
            continue;
        }
        let length = text[start..]
            .find(|c: char| !inside(c))
            .unwrap_or(text.len() - start);
        let word = &text[start..start + length];
        let path = word.trim_end_matches(|c: char| !(c.is_ascii_alphanumeric() || c == '_'));
        let starts = path
            .split_once('/')
            .is_some_and(|(top, below)| !below.is_empty() && tops.contains(top));
        if starts {
            paths.push((path, word[path.len()..].starts_with('-')));
            rest = start + length;
        }
    }
    paths
}

#[cfg(test)]
#[path = "stale_tests.rs"]
mod tests;
