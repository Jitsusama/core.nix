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
//! hold, and a comment after code on its own line isn't read in any
//! language. A shell heredoc's body is a command's text, so none of it is
//! comment, and shell is read as bash reads it, with quotes inside a
//! substitution inside quotes, so a quote read wrong hides no comment
//! beyond it. A docstring, a block comment, a heredoc, a string or a shell
//! quote the file never closes is taken for a misreading, so it hides
//! nothing after it. And a
//! path is gone only below a folder the repository holds under its top
//! one, such as `modules/a/`: `tools/token.c` or `tests/x.nix` is as likely
//! another project's, or a test's stand-in tree, and git's `refs/heads` is
//! no path at all. A path cut just before a hyphen, as
//! `tests/vm/screens/screens-` is, names where names begin rather than one
//! file, so it's there while any path begins with it, hyphen and all:
//! `screens.nix` beside it begins only with its letters. It's there too
//! when found whole, the dash then punctuation, as in `home.nix- which`.

use std::collections::{HashMap, HashSet, VecDeque};

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
/// quotes on, past the code before it. A docstring, a block comment, a
/// heredoc, a string or a shell quote the file never closes is taken for
/// a misreading, as a `/*` in a string or a `<<` the shell reads otherwise
/// would be, and the file is read again as if its line opened nothing, so
/// it hides nothing after it.
fn comments(file: &File) -> Vec<(usize, &str)> {
    let extension = extension(&file.path);
    let (mark, blocks) = match extension {
        "rs" | "js" | "ts" | "qml" | "c" | "h" => ("//", true),
        "nix" => ("#", true),
        "sh" | "bash" | "py" | "toml" | "yaml" | "yml" => ("#", false),
        _ => return Vec::new(),
    };
    // Each reading names a line not named before, since a line named opens
    // nothing, so there are no more readings than lines.
    let mut unopened = HashSet::new();
    loop {
        match read(file, extension, (mark, blocks), &unopened) {
            Ok(lines) => return lines,
            Err(opener) => assert!(
                unopened.insert(opener),
                "line {opener} of {} opened something though named",
                file.path
            ),
        }
    }
}

/// A file's comment lines, read with the comment `mark` and `blocks`, or
/// the line that opened a docstring, a block comment, a heredoc, a string
/// or a shell quote the file never closes. No line in `unopened` opens one.
fn read<'a>(
    file: &'a File,
    extension: &str,
    (mark, blocks): (&str, bool),
    unopened: &HashSet<usize>,
) -> Result<Vec<(usize, &'a str)>, usize> {
    // The quotes a docstring opened with, which alone close it, while
    // one is open, and the same for a string in code, whose lines aren't
    // comment.
    let mut docstring: Option<&str> = None;
    let mut string: Option<&str> = None;
    let mut block = false;
    // The words closing the heredocs a line of shell opened, in the order
    // their bodies follow, each with whether `<<-` lets tabs indent it.
    let mut heredocs: VecDeque<(String, bool)> = VecDeque::new();
    // What a line of shell left open, innermost last: in a quote, its
    // lines up to the close are a string's, so neither comment nor opening
    // a heredoc.
    let mut quoted: Vec<Shell> = Vec::new();
    // Whether a docstring may stand on the next line of code: at a
    // module's top, or first in a def's or a class's body, past the colon
    // ending its header. Any other colon, as an if's or a dict key's, opens
    // no docstring, so a string there is code.
    let mut starts = true;
    // The logical line the code is in: the brackets it holds open, whether
    // a backslash carries it on, and whether it opened as a header.
    let mut logical = Logical::default();
    let mut lines = Vec::new();
    // The line that last opened a docstring, a block comment, a heredoc, a
    // string or a quote, which is the one still open at the file's end, if
    // any is.
    let mut opener = 0;
    // A byte-order mark opens a file as nothing at all.
    let content = file.text.strip_prefix('\u{feff}').unwrap_or(&file.text);
    let closers = Closers::of(content);
    for (number, line) in content.lines().enumerate() {
        let opens_here = !unopened.contains(&number);
        // Whether this line opened the code string open past it.
        let mut fresh = false;
        let trimmed = line.trim_start();
        let comment = if let Some(heredoc) = heredocs.front() {
            // A heredoc's body is text a command reads, though a line of it
            // may look like a comment.
            if ends_heredoc(line, heredoc) {
                heredocs.pop_front();
            }
            None
        } else if let Some(quotes) = docstring {
            (docstring, string) = docstring_line(line, quotes, opens_here);
            fresh = string.is_some();
            starts = false;
            Some(line)
        } else if block {
            block = !line.contains("*/");
            Some(line)
        } else if let Some((quotes, past)) =
            opens(trimmed).filter(|_| extension == "py" && starts && opens_here)
        {
            (docstring, string) = docstring_line(&trimmed[past..], quotes, true);
            starts = false;
            logical = Logical::default();
            opener = number;
            Some(line)
        } else if blocks && trimmed.starts_with("/*") && opens_here {
            block = !trimmed[2..].contains("*/");
            opener = number;
            Some(line)
        } else {
            // Only a file's first line can be its shebang; past it, `#!` is
            // read as any line is.
            let shebang = number == 0 && line.starts_with("#!");
            let comment = Shell::code(quoted.last().copied())
                && ((trimmed.starts_with(mark) && !shebang)
                    || (extension == "nix" && trimmed.starts_with('*')));
            // A blank line, a comment or a shebang leaves where a statement
            // starts as it was. No line a string runs on through is comment.
            if string.is_some()
                || (extension == "py" && !comment && !trimmed.is_empty() && !shebang)
            {
                // A line a string runs on into carries its logical line on
                // rather than beginning one.
                let scanned = scan(line, string, opens_here);
                if string.is_none() {
                    logical.begin(scanned.code);
                }
                let colon = header_colon(&logical, &scanned);
                let header = colon
                    .and_then(|colon| docstring_past(&line[colon + 1..]))
                    .filter(|_| opens_here);
                // The string open past the line is its own unless it's the
                // one it started in, still unclosed.
                fresh = scanned.open.is_some() && (string.is_none() || scanned.closed);
                string = scanned.open;
                // The brackets a line closes count though it leaves a
                // string open, so the logical line is carried before asking
                // for one.
                let ends = logical.ends_a_header(&scanned);
                starts = string.is_none() && ends;
                // Past its colon a header's logical line is its body's, whose
                // colons end no header, and as the line runs on past it,
                // `starts` holds no docstring may follow.
                if colon.is_some() {
                    logical.header = false;
                }
                if let Some((text, quotes, past)) = header {
                    (docstring, string) = docstring_line(&text[past..], quotes, true);
                    opener = number;
                }
                header.map(|(text, _, _)| text)
            } else {
                if matches!(extension, "sh" | "bash") && !comment {
                    // A line named still closes what it started inside,
                    // but opens neither heredoc nor quote. A heredoc no
                    // later line closes is a misreading, known at once.
                    let (words, left_open) = shell_line(line, &mut quoted, opens_here);
                    let words: Vec<_> = words
                        .into_iter()
                        .filter(|heredoc| closers.close_after(heredoc, number))
                        .collect();
                    if !words.is_empty() || left_open {
                        opener = number;
                    }
                    heredocs.extend(words);
                }
                comment.then_some(line)
            }
        };
        // A string the line opened and left open is an opening as a
        // docstring's is; a line named opens none.
        if fresh {
            opener = number;
        }
        if let Some(comment) = comment {
            lines.push((number + 1, comment));
        }
    }
    if docstring.is_some()
        || block
        || !heredocs.is_empty()
        || string.is_some()
        || !quoted.is_empty()
    {
        Err(opener)
    } else {
        Ok(lines)
    }
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

/// Where on its line the colon ending a def's or a class's header stands:
/// the first outside every bracket and string, so a parameter's, a
/// lambda's or a default's is passed by.
fn header_colon(logical: &Logical, scanned: &Scanned) -> Option<usize> {
    if !logical.header {
        return None;
    }
    scanned
        .colons
        .iter()
        .find(|(_, depth)| logical.depth + depth == 0)
        .map(|&(colon, _)| colon)
}

/// The docstring standing first in `body`, a header's line past its colon,
/// as `class K: """Says why."""`'s does: its text from its quotes on, the
/// quotes, and where past them it begins. A statement there, as
/// `return """x"""`, holds none.
fn docstring_past(body: &str) -> Option<(&str, &'static str, usize)> {
    let text = body.trim_start();
    opens(text).map(|(quotes, past)| (text, quotes, past))
}

/// The docstring and the code string left open by a line read from inside
/// a docstring opened with `quotes`: the docstring still, until its own
/// quotes close it unescaped, and then any string the code past them
/// opens, where the line `opens` one.
fn docstring_line(
    line: &str,
    quotes: &'static str,
    opens: bool,
) -> (Option<&'static str>, Option<&'static str>) {
    let scanned = scan(line, Some(quotes), opens);
    if scanned.closed {
        (None, scanned.open)
    } else {
        (Some(quotes), None)
    }
}

/// A line of Python code read past its strings and their escapes. `open`
/// holds the quotes of a string the line starts inside, as one the line
/// before left open. A line that `opens` no string reads the quotes of
/// one it would leave open as code, and the code past them, so the
/// brackets it closes there still close.
fn scan<'a>(line: &'a str, mut open: Option<&'static str>, opens: bool) -> Scanned<'a> {
    let started = open.is_some();
    let mut closed = false;
    let mut depth = 0;
    let mut colons = Vec::new();
    // Where the string the line opened begins, past its quotes, while it's
    // open.
    let mut opened = None;
    let mut at = 0;
    loop {
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
                opened = Some(at);
                continue;
            }
            at += c.len_utf8();
        }
        // A one-quote string runs on past its line only where a backslash
        // ends it.
        let left = open.filter(|quotes| quotes.len() == 3 || line.ends_with('\\'));
        match opened {
            Some(past) if !opens && left.is_some() => {
                at = past;
                open = None;
            }
            _ => {
                return Scanned {
                    code: line,
                    open: left,
                    closed,
                    depth,
                    colons,
                };
            }
        }
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

/// Where a line of shell stands, as bash reads it: in a quote, or in the
/// code of a substitution, inside which quotes open anew, as they do in
/// `"$(echo "a b")"`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shell {
    /// `'...'`, which only its quote closes, a backslash being its own.
    Single,
    /// `$'...'`, where a backslash keeps a quote from closing it.
    Ansi,
    /// `"..."`, where a backslash escapes, and `$(`, `${` and a backtick
    /// open code again.
    Double,
    /// `$(...)`, code, with how many brackets of its own it holds open,
    /// which close before it does.
    Command(usize),
    /// A backtick's command, code until the next backtick.
    Backtick,
    /// `${...}`, where quotes open as in code but a `#` is the
    /// parameter's, and `}` closes it.
    Parameter,
}

impl Shell {
    /// Whether what stands `within` this is code, where a line starting
    /// with `#` is a comment: outside every quote, or in a command
    /// substitution's lines.
    fn code(within: Option<Shell>) -> bool {
        matches!(within, None | Some(Shell::Command(_) | Shell::Backtick))
    }
}

/// The words closing the heredocs a line of shell opens, in the order
/// their bodies follow it, each with whether `<<-` lets tabs indent its
/// body, and whether the line leaves open a quote or a substitution it
/// opened. `within` holds what lines before left open, innermost last,
/// which the line starts inside, and is left holding what it leaves open.
/// A line that `opens` nothing still closes what it starts inside, but
/// leaves open nothing of its own and opens no heredoc. A `<<` opens a
/// heredoc only in code, past no comment, and outside the `((` of
/// arithmetic, where it shifts; a here-string's `<<<` has no word after
/// it, so it opens none.
fn shell_line(line: &str, within: &mut Vec<Shell>, opens: bool) -> (Vec<(String, bool)>, bool) {
    let mut heredocs = Vec::new();
    // The fewest things open at any point of the line: what stands above
    // them at its end the line opened.
    let mut kept = within.len();
    let mut arithmetic = 0;
    // Whether a word begins here, as it does where a line or a command
    // substitution begins, or past a blank or an operator.
    let mut word_starts = true;
    let mut at = 0;
    while let Some(c) = line[at..].chars().next() {
        let rest = &line[at..];
        let mut step = c.len_utf8();
        let mut starts = false;
        let innermost = within.last().copied();
        match innermost {
            Some(Shell::Single | Shell::Ansi) if c == '\'' => {
                within.pop();
            }
            Some(Shell::Ansi | Shell::Double) if c == '\\' => step = escaped(rest),
            Some(Shell::Single | Shell::Ansi) => {}
            Some(Shell::Double) if c == '"' => {
                within.pop();
            }
            Some(Shell::Double) => {
                if let Some(opened) = substitution(rest, within) {
                    (step, starts) = opened;
                }
            }
            Some(Shell::Backtick) if c == '`' => {
                within.pop();
            }
            Some(Shell::Parameter) if c == '}' => {
                within.pop();
            }
            Some(Shell::Command(open)) if c == ')' && arithmetic == 0 => {
                if open == 0 {
                    within.pop();
                } else {
                    within.pop();
                    within.push(Shell::Command(open - 1));
                    starts = true;
                }
            }
            Some(Shell::Command(open)) if c == '(' && !rest.starts_with("((") => {
                within.pop();
                within.push(Shell::Command(open + 1));
                starts = true;
            }
            // Code: outside quotes, or in a substitution's.
            _ if c == '\\' => step = escaped(rest),
            _ if c == '\'' => within.push(Shell::Single),
            _ if rest.starts_with("$'") => {
                within.push(Shell::Ansi);
                step = 2;
            }
            _ if c == '"' => within.push(Shell::Double),
            // A `#` opens a comment only where a word begins.
            _ if c == '#' && word_starts && innermost != Some(Shell::Parameter) => break,
            _ if rest.starts_with("((") => {
                arithmetic += 1;
                step = 2;
            }
            _ if rest.starts_with("))") && arithmetic > 0 => {
                arithmetic -= 1;
                step = 2;
            }
            _ if rest.starts_with("<<") && arithmetic == 0 => {
                step = 2;
                if let Some((word, tabbed, length)) = heredoc_word(&rest[2..]) {
                    heredocs.push((word, tabbed));
                    step += length;
                }
            }
            _ => {
                if let Some(opened) = substitution(rest, within) {
                    (step, starts) = opened;
                } else {
                    starts = c.is_whitespace() || ";&|()<>".contains(c);
                }
            }
        }
        kept = kept.min(within.len());
        word_starts = starts;
        at += step;
    }
    if !opens {
        within.truncate(kept);
        heredocs.clear();
    }
    let left_open = within.len() > kept;
    (heredocs, left_open)
}

/// The substitution `rest` opens, pushed onto `within`: how long its
/// opening is, and whether a word begins past it, as one does at a
/// command's start. None for `$((`, arithmetic, which is read as `((`.
fn substitution(rest: &str, within: &mut Vec<Shell>) -> Option<(usize, bool)> {
    if rest.starts_with("$(") && !rest.starts_with("$((") {
        within.push(Shell::Command(0));
        Some((2, true))
    } else if rest.starts_with("${") {
        within.push(Shell::Parameter);
        Some((2, false))
    } else if rest.starts_with('`') {
        within.push(Shell::Backtick);
        Some((1, true))
    } else {
        None
    }
}

/// The lines a heredoc's word might close, each by where it last stands,
/// as it is and with its leading tabs gone, as `<<-` lets them be.
struct Closers<'a> {
    whole: HashMap<&'a str, usize>,
    untabbed: HashMap<&'a str, usize>,
}

impl<'a> Closers<'a> {
    fn of(content: &'a str) -> Closers<'a> {
        let mut closers = Closers {
            whole: HashMap::new(),
            untabbed: HashMap::new(),
        };
        for (number, line) in content.lines().enumerate() {
            closers.whole.insert(line, number);
            closers
                .untabbed
                .insert(line.trim_start_matches('\t'), number);
        }
        closers
    }

    /// Whether a line past `number` could close `heredoc`: when none does,
    /// its `<<` was read wrong, and taken for nothing at once, the file
    /// not read again from its top for it.
    fn close_after(&self, (word, tabbed): &(String, bool), number: usize) -> bool {
        let lines = if *tabbed { &self.untabbed } else { &self.whole };
        lines.get(word.as_str()).is_some_and(|&last| last > number)
    }
}

/// Whether `line` closes the heredoc whose body it's in: its word alone on
/// the line, past any tabs where `<<-` lets them indent it.
fn ends_heredoc(line: &str, (word, tabbed): &(String, bool)) -> bool {
    let last = if *tabbed {
        line.trim_start_matches('\t')
    } else {
        line
    };
    last == word
}

/// How long a backslash opening `rest` is with the character it escapes.
fn escaped(rest: &str) -> usize {
    1 + rest[1..].chars().next().map_or(0, char::len_utf8)
}

/// The word a heredoc's `<<` is followed by in `rest`, read as the shell
/// reads it: past a `-`, which lets tabs indent the body, and any blanks,
/// up to a blank or an operator outside quotes, its quotes and backslashes
/// taken off as its closing line leaves them. With whether tabs may
/// indent, and how much of `rest` it took; none where no word follows.
fn heredoc_word(rest: &str) -> Option<(String, bool, usize)> {
    let (dashless, tabbed) = rest
        .strip_prefix('-')
        .map_or((rest, false), |rest| (rest, true));
    let mut at = rest.len() - dashless.trim_start_matches([' ', '\t']).len();
    let mut word = String::new();
    let mut quote = None;
    while let Some(c) = rest[at..].chars().next() {
        let mut step = c.len_utf8();
        match quote {
            Some(open) if c == open => quote = None,
            None if c == '\\' => {
                step = escaped(&rest[at..]);
                word.push_str(&rest[at + 1..at + step]);
            }
            None if c == '\'' || c == '"' => quote = Some(c),
            None if c.is_whitespace() || ";&|<>()".contains(c) => break,
            // Quoted or not, the rest is the word's.
            Some(_) | None => word.push(c),
        }
        at += step;
    }
    (!word.is_empty()).then_some((word, tabbed, at))
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
