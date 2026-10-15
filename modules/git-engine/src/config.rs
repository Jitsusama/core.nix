//! git's configuration, read as config.c reads it: sections with optional
//! quoted subsections, quoted values with escapes and continued lines, and
//! the includes, unconditional or on the repository's folder or branch.
//! The system file is left out: it sits wherever git was built to look, and
//! nothing the engine asks is set there.

use crate::stamp::{Stamp, Stamps};
use crate::wildmatch;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct Config {
    /// Every value in the order git would read it, keyed as
    /// `section.subsection.name` with the section and name in lower case.
    entries: Vec<(String, Option<String>)>,
    /// Every file read or looked for, so a change to any is noticed.
    pub stamps: Stamps,
    /// Whether an `includeIf` asks which branch is checked out, and so
    /// whether this has to be read again when another one is.
    pub asks_branch: bool,
    /// The branch this was read for.
    pub branch: Option<String>,
}

/// What an `includeIf` can ask about.
pub struct Context<'a> {
    pub git_dir: &'a Path,
    pub branch: Option<&'a str>,
    pub home: Option<&'a Path>,
}

impl Config {
    pub fn load(files: &[PathBuf], context: &Context) -> Config {
        let mut config = Config {
            branch: context.branch.map(str::to_string),
            ..Config::default()
        };
        for file in files {
            config.read(file, context, 0);
        }
        config
    }

    /// The last value set for a key, as git takes it.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entry(key).and_then(|value| value.as_deref())
    }

    fn entry(&self, key: &str) -> Option<&Option<String>> {
        self.entries
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    pub fn get_all<'a>(&'a self, key: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.entries
            .iter()
            .filter(move |(k, _)| k == key)
            .filter_map(|(_, v)| v.as_deref())
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.entry(key)
            .and_then(|value| parse_bool(value.as_deref()))
    }

    fn read(&mut self, file: &Path, context: &Context, depth: usize) {
        self.stamps.push((file.to_path_buf(), Stamp::of(file)));
        if depth > 10 {
            return;
        }
        let Ok(text) = std::fs::read(file) else {
            return;
        };
        let base = file.parent().unwrap_or(Path::new("/"));
        for (key, value) in parse(&text) {
            if key.starts_with("includeif.onbranch:") {
                self.asks_branch = true;
            }
            let include = if key == "include.path" {
                value.as_deref()
            } else if let Some(rest) = key.strip_prefix("includeif.")
                && let Some(condition) = rest.strip_suffix(".path")
                && holds(condition, base, context)
            {
                value.as_deref()
            } else {
                None
            };
            if let Some(path) = include {
                let path = expand(path, base, context.home);
                self.entries.push((key, value));
                self.read(&path, context, depth + 1);
            } else {
                self.entries.push((key, value));
            }
        }
    }
}

pub fn parse_bool(value: Option<&str>) -> Option<bool> {
    let Some(value) = value else {
        return Some(true);
    };
    match value.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" => Some(true),
        "false" | "no" | "off" | "" => Some(false),
        number => number.parse::<i64>().ok().map(|n| n != 0),
    }
}

/// An include's path: `~/` from home, relative to the file naming it.
fn expand(path: &str, base: &Path, home: Option<&Path>) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = home
    {
        home.join(rest)
    } else {
        base.join(path)
    }
}

fn holds(condition: &str, base: &Path, context: &Context) -> bool {
    if let Some(pattern) = condition.strip_prefix("gitdir:") {
        gitdir_matches(pattern, base, context, false)
    } else if let Some(pattern) = condition.strip_prefix("gitdir/i:") {
        gitdir_matches(pattern, base, context, true)
    } else if let Some(pattern) = condition.strip_prefix("onbranch:") {
        let Some(branch) = context.branch else {
            return false;
        };
        let mut pattern = pattern.to_string();
        if pattern.ends_with('/') {
            pattern.push_str("**");
        }
        wildmatch::matches(pattern.as_bytes(), branch.as_bytes(), true, false)
    } else {
        false
    }
}

fn gitdir_matches(pattern: &str, base: &Path, context: &Context, fold: bool) -> bool {
    let mut pattern = if let Some(rest) = pattern.strip_prefix("~/") {
        let Some(home) = context.home else {
            return false;
        };
        format!("{}/{rest}", home.display())
    } else if let Some(rest) = pattern.strip_prefix("./") {
        format!("{}/{rest}", base.display())
    } else if pattern.starts_with('/') {
        pattern.to_string()
    } else {
        format!("**/{pattern}")
    };
    if pattern.ends_with('/') {
        pattern.push_str("**");
    }
    let real = std::fs::canonicalize(context.git_dir).ok();
    [Some(context.git_dir.to_path_buf()), real]
        .into_iter()
        .flatten()
        .any(|dir| {
            let dir = dir.to_string_lossy();
            wildmatch::matches(pattern.as_bytes(), dir.as_bytes(), true, fold)
        })
}

/// Every `key = value` in a file, in order.
fn parse(text: &[u8]) -> Vec<(String, Option<String>)> {
    let text = text.strip_prefix(b"\xef\xbb\xbf").unwrap_or(text);
    let mut reader = Reader { text, at: 0 };
    let mut section = String::new();
    let mut out = Vec::new();
    while let Some(c) = reader.next() {
        match c {
            b'\n' | b' ' | b'\t' => {}
            b'#' | b';' => reader.skip_line(),
            b'[' => match reader.section() {
                Some(name) => section = name,
                None => break,
            },
            c if c.is_ascii_alphabetic() => {
                let mut name = String::from(c.to_ascii_lowercase() as char);
                while let Some(c) = reader.peek()
                    && (c.is_ascii_alphanumeric() || c == b'-')
                {
                    name.push(c.to_ascii_lowercase() as char);
                    reader.at += 1;
                }
                while matches!(reader.peek(), Some(b' ' | b'\t')) {
                    reader.at += 1;
                }
                let value = match reader.next() {
                    None | Some(b'\n') => None,
                    Some(b'=') => match reader.value() {
                        Some(value) => Some(value),
                        None => break,
                    },
                    Some(b'#' | b';') => {
                        reader.skip_line();
                        None
                    }
                    Some(_) => break,
                };
                if !section.is_empty() {
                    out.push((format!("{section}.{name}"), value));
                }
            }
            _ => break,
        }
    }
    out
}

struct Reader<'a> {
    text: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    /// The next character, with a CR before a newline dropped.
    fn next(&mut self) -> Option<u8> {
        let c = *self.text.get(self.at)?;
        self.at += 1;
        if c == b'\r' && self.text.get(self.at) == Some(&b'\n') {
            self.at += 1;
            return Some(b'\n');
        }
        Some(c)
    }

    fn peek(&self) -> Option<u8> {
        self.text.get(self.at).copied()
    }

    fn skip_line(&mut self) {
        while let Some(c) = self.next() {
            if c == b'\n' {
                break;
            }
        }
    }

    /// A section header after its `[`: `name`, `name "sub"` or the old
    /// `name.sub`, whose subsection is folded to lower case.
    fn section(&mut self) -> Option<String> {
        let mut name = String::new();
        loop {
            let c = self.next()?;
            match c {
                b']' => return Some(name),
                b' ' | b'\t' => break,
                c if c.is_ascii_alphanumeric() || c == b'-' || c == b'.' => {
                    name.push(c.to_ascii_lowercase() as char);
                }
                _ => return None,
            }
        }
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.at += 1;
        }
        if self.next()? != b'"' {
            return None;
        }
        let mut sub = Vec::new();
        loop {
            match self.next()? {
                b'\n' => return None,
                b'"' => break,
                b'\\' => sub.push(self.next()?),
                c => sub.push(c),
            }
        }
        if self.next()? != b']' {
            return None;
        }
        Some(format!("{name}.{}", String::from_utf8_lossy(&sub)))
    }

    /// A value after its `=`, to the end of its line or lines.
    fn value(&mut self) -> Option<String> {
        let mut out = Vec::new();
        // Blanks inside a value are kept as written; those at its ends go.
        let (mut quote, mut comment, mut space) = (false, false, Vec::new());
        loop {
            let c = self.next().unwrap_or(b'\n');
            if c == b'\n' {
                if quote {
                    return None;
                }
                return Some(String::from_utf8_lossy(&out).into_owned());
            }
            if comment {
                continue;
            }
            if c.is_ascii_whitespace() && !quote {
                if !out.is_empty() {
                    space.push(c);
                }
                continue;
            }
            if !quote && (c == b';' || c == b'#') {
                comment = true;
                continue;
            }
            out.append(&mut space);
            match c {
                b'\\' => match self.next().unwrap_or(0) {
                    b'\n' => {}
                    b't' => out.push(b'\t'),
                    b'b' => out.push(8),
                    b'n' => out.push(b'\n'),
                    c @ (b'\\' | b'"') => out.push(c),
                    _ => return None,
                },
                b'"' => quote = !quote,
                c => out.push(c),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(text: &str) -> Vec<(String, Option<String>)> {
        parse(text.as_bytes())
    }

    fn one(key: &str, value: &str) -> (String, Option<String>) {
        (key.to_string(), Some(value.to_string()))
    }

    #[test]
    fn reads_sections_and_subsections() {
        let text = "[Core]\n\tAutoCRLF = input\n[branch \"Feature/X\"]\n\tremote = origin\n\
                    [Remote.Origin]\n\turl = u\n";
        assert_eq!(
            values(text),
            vec![
                one("core.autocrlf", "input"),
                one("branch.Feature/X.remote", "origin"),
                one("remote.origin.url", "u"),
            ]
        );
    }

    #[test]
    fn reads_values_as_git_does() {
        let text = "[a]\n x = \"  two  spaces \" # comment\n y = one   two\t three ; c\n \
                    z = a\\\n   b\n w\n v = \"q\\\"uote\\\\\" \n";
        assert_eq!(
            values(text),
            vec![
                one("a.x", "  two  spaces "),
                one("a.y", "one   two\t three"),
                // The blanks after a continuation are the value's own.
                one("a.z", "a   b"),
                ("a.w".to_string(), None),
                one("a.v", "q\"uote\\"),
            ]
        );
    }

    #[test]
    fn stops_at_what_git_would_refuse() {
        assert_eq!(
            values("[a]\n x = 1\n y = \"open\n z = 3\n"),
            vec![one("a.x", "1")]
        );
    }

    #[test]
    fn reads_crlf_files() {
        assert_eq!(values("[a]\r\n x = 1\r\n"), vec![one("a.x", "1")]);
    }

    #[test]
    fn booleans_read_as_git_reads_them() {
        assert_eq!(parse_bool(None), Some(true));
        assert_eq!(parse_bool(Some("Yes")), Some(true));
        assert_eq!(parse_bool(Some("off")), Some(false));
        assert_eq!(parse_bool(Some("")), Some(false));
        assert_eq!(parse_bool(Some("2")), Some(true));
        assert_eq!(parse_bool(Some("maybe")), None);
    }

    #[test]
    fn includes_follow_their_conditions() {
        let dir = std::env::temp_dir().join(format!("git-engine-config-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("repo/.git")).unwrap();
        std::fs::write(dir.join("always"), "[x]\n a = always\n").unwrap();
        std::fs::write(dir.join("here"), "[x]\n b = here\n").unwrap();
        std::fs::write(dir.join("there"), "[x]\n c = there\n").unwrap();
        std::fs::write(dir.join("branch"), "[x]\n d = branch\n").unwrap();
        std::fs::write(
            dir.join("main"),
            "[include]\n path = always\n[includeIf \"gitdir:repo/\"]\n path = here\n\
             [includeIf \"gitdir:other/\"]\n path = there\n\
             [includeIf \"onbranch:work/\"]\n path = branch\n[x]\n a = last\n",
        )
        .unwrap();
        let git_dir = dir.join("repo/.git");
        let context = Context {
            git_dir: &git_dir,
            branch: Some("work/one"),
            home: None,
        };
        let config = Config::load(&[dir.join("main")], &context);
        assert_eq!(config.get("x.a"), Some("last"));
        assert_eq!(config.get("x.b"), Some("here"));
        assert_eq!(config.get("x.c"), None);
        assert_eq!(config.get("x.d"), Some("branch"));
        assert_eq!(config.stamps.len(), 4);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
