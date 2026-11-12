//! Whether two changes to one pytest file touch different cases, so the
//! gate can count them as two files. pytest runs each `test_` function on
//! its own, so a change to one case reaches no other, while a helper, a
//! constant, a fixture or the imports may reach every case.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::{OUTSIDE, apart_read};

/// Whether `file` is one read as cases, should it import pytest: a test
/// file named for what it tests, as tests/launcher-on-niri-test.py is.
pub fn is_cases(file: &str) -> bool {
    file.ends_with("-test.py")
}

/// Whether a pytest file both sides changed was changed apart: neither
/// side changed the lines outside its entries, nor changed or took away
/// an entry that isn't a case, the entries each side added or changed
/// share no name, and the rebased `head` differs from each side only
/// where the other changed it. By 09:50 on 12 November, the launcher's
/// pytest file on niri had moved on main 13 times that day, and merge-net
/// had held 11 gates back on it and asked 2 to run again, though 19 of
/// its 22 changes since the 11th added or changed only cases, or added
/// helpers of their own.
#[must_use]
pub fn cases_apart(base: &str, branch: &str, main: &str, head: &str) -> bool {
    would_cases_be_apart(base, branch, main)
        && apart_read(
            &|text| entries(text).map(as_value),
            base,
            branch,
            main,
            head,
        )
}

/// Whether two changes from `base`, `one` and `other`, would meet as
/// `cases_apart` asks once either is rebased onto the other: what a gate
/// holding a change back on a pytest file is asked, before the rebase
/// there is to judge.
#[must_use]
pub fn would_cases_be_apart(base: &str, one: &str, other: &str) -> bool {
    let Some(base) = entries(base) else {
        return false;
    };
    let read = |text: &str| touched(&base, &entries(text)?);
    match (read(one), read(other)) {
        (Some(one), Some(other)) => one.is_disjoint(&other),
        _ => false,
    }
}

/// A file's lines by the name of the top-level entry they belong to.
type Entries = BTreeMap<String, String>;

/// The names of the entries `after` added, changed or took away from
/// `base`, or none where it changed what every case may read: the lines
/// outside the entries, kept under `OUTSIDE`, an entry not a case, or
/// one added that `reaches_every_case`.
fn touched(base: &Entries, after: &Entries) -> Option<BTreeSet<String>> {
    let bound = code_words(base.get(OUTSIDE)?);
    let mut names = BTreeSet::new();
    for (name, lines) in base {
        if after.get(name) == Some(lines) {
            continue;
        }
        if !name.starts_with("test_") {
            return None;
        }
        names.insert(name.clone());
    }
    for (name, lines) in after {
        if base.contains_key(name) {
            continue;
        }
        if !name.starts_with("test_") && reaches_every_case(name, lines, &bound) {
            return None;
        }
        names.insert(name.clone());
    }
    Some(names)
}

/// Whether an entry added under `name`, not a case, reaches every case
/// all the same: an autouse fixture, a hook or the file's marks, which
/// pytest runs or reads for every case, or a name `bound` by the code
/// outside the entries, as an import binds one, which it rebinds for
/// each case reading it.
fn reaches_every_case(name: &str, lines: &str, bound: &BTreeSet<&str>) -> bool {
    name.starts_with("pytest") || lines.contains("autouse") || bound.contains(name)
}

/// The words of `outside`'s code: every line not within a triple-quoted
/// string, so a docstring's words bind nothing.
fn code_words(outside: &str) -> BTreeSet<&str> {
    let mut quoted = false;
    let mut words = BTreeSet::new();
    for line in outside.lines() {
        let edge = opens_or_closes(line);
        if !quoted && !edge {
            words.extend(
                line.split(|at: char| !at.is_ascii_alphanumeric() && at != '_')
                    .filter(|word| !word.is_empty()),
            );
        }
        quoted ^= edge;
    }
    words
}

/// Whether `line` opens or closes a triple-quoted string.
fn opens_or_closes(line: &str) -> bool {
    (line.matches("\"\"\"").count() + line.matches("'''").count()) % 2 == 1
}

/// `entries` as `apart_read` reads them.
fn as_value(entries: Entries) -> Value {
    Value::Object(
        entries
            .into_iter()
            .map(|(name, lines)| (name, Value::String(lines)))
            .collect(),
    )
}

/// A pytest file as the lines of each top-level function, class and
/// assignment by its name, with every other line under `OUTSIDE`: the
/// module's docstring, its imports and any statement. Each entry takes
/// the blank lines, comments and decorators just above it, so adding one
/// after another leaves that one as it was, and every line after it up
/// to the next line at the left margin, other than a closing bracket or
/// a line within a string. A file that doesn't import pytest, gives a
/// name twice or leaves a string open reads as nothing.
fn entries(text: &str) -> Option<Entries> {
    let mut reading = Reading::default();
    for line in text.lines() {
        reading.read(line)?;
    }
    reading.done()
}

#[derive(Default)]
struct Reading {
    entries: Entries,
    outside: String,
    /// The entry being read, and its lines so far.
    open: Option<(String, String)>,
    /// The blank lines, comments and decorators read since the last line
    /// kept, which go with whatever comes after them.
    above: String,
    /// Whether the lines above open with a decorator, whose own lines
    /// run on until the entry it decorates.
    decorating: bool,
    /// Whether the line read last left a triple-quoted string open.
    quoted: bool,
}

impl Reading {
    /// Reads one line, or nothing where it gives a name twice.
    fn read(&mut self, line: &str) -> Option<()> {
        let within = self.quoted || line.starts_with([' ', '\t', ')', ']', '}']);
        self.quoted ^= opens_or_closes(line);
        let above = !within && (line.is_empty() || line.starts_with('#'));
        let decorates = !within && line.starts_with('@');
        if decorates || above || (self.decorating && within) {
            self.decorating |= decorates;
            self.above.push_str(line);
            self.above.push('\n');
            return Some(());
        }
        if !within {
            self.close()?;
            self.open = named(line).map(|name| (name.to_owned(), String::new()));
        }
        self.decorating = false;
        let into = match &mut self.open {
            Some((_, lines)) => lines,
            None => &mut self.outside,
        };
        into.push_str(&std::mem::take(&mut self.above));
        into.push_str(line);
        into.push('\n');
        Some(())
    }

    /// Keeps the entry being read, or nothing where its name is taken.
    fn close(&mut self) -> Option<()> {
        match self.open.take() {
            Some((name, lines)) => self.entries.insert(name, lines).is_none().then_some(()),
            None => Some(()),
        }
    }

    /// The entries read, or nothing where the file isn't pytest's or
    /// leaves a string open.
    fn done(mut self) -> Option<Entries> {
        self.close()?;
        let pytest = self
            .outside
            .lines()
            .any(|line| line == "import pytest" || line.starts_with("from pytest import"));
        if self.quoted || !pytest {
            return None;
        }
        self.outside.push_str(&self.above);
        self.entries.insert(OUTSIDE.to_owned(), self.outside);
        Some(self.entries)
    }
}

/// The name a line at the left margin opens an entry of: a function's,
/// a class's or an assigned name's.
fn named(line: &str) -> Option<&str> {
    let defined = ["async def ", "def ", "class "]
        .iter()
        .find_map(|opens| line.strip_prefix(opens));
    let name = if let Some(rest) = defined {
        rest.split(['(', ':']).next()?
    } else {
        let (name, rest) = line.split_at(line.find([' ', ':', '='])?);
        let rest = rest.strip_prefix(' ').unwrap_or(rest);
        let assigned = rest.starts_with('=') && !rest.starts_with("==");
        let annotated = rest
            .strip_prefix(": ")
            .is_some_and(|kind| !kind.is_empty() && !kind.starts_with('#'));
        (assigned || annotated).then_some(name)?
    };
    let mut bytes = name.bytes();
    (bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'))
    .then_some(name)
}

#[cfg(test)]
mod tests;
