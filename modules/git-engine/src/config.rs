//! git's configuration, read as config.c reads it: sections with optional
//! quoted subsections, quoted values with escapes and continued lines, and
//! the includes, unconditional or on the repository's folder or branch,
//! then what a parent git passed on as `-c` or `GIT_CONFIG_COUNT`.
//! The system file is read where git's environment names it, or else at
//! `/etc/gitconfig`: asking git where it was built to look would start a
//! program, which the engine never does, so a git built to look elsewhere
//! has its system file left unread.

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
    /// A config of one file's text, as a test writes it.
    #[cfg(test)]
    pub fn parsed(text: &str) -> Config {
        Config {
            entries: parse(text.as_bytes()),
            ..Config::default()
        }
    }

    /// Each of `files` in turn, then `parameters`, which git reads after
    /// every file.
    pub fn load(
        files: &[PathBuf],
        parameters: &[(String, Option<String>)],
        context: &Context,
    ) -> Config {
        let mut config = Config {
            branch: context.branch.map(str::to_string),
            ..Config::default()
        };
        for file in files {
            config.read(file, context, 0);
        }
        config.entries.extend_from_slice(parameters);
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

    /// The subsections `section` holds keys under, as `foo` is for
    /// `filter.foo.clean`, each once.
    pub fn subsections(&self, section: &str) -> Vec<&str> {
        let mut subsections: Vec<&str> = Vec::new();
        for (key, _) in &self.entries {
            let subsection = key
                .strip_prefix(section)
                .and_then(|rest| rest.strip_prefix('.'))
                .and_then(|rest| rest.rsplit_once('.'))
                .map(|(subsection, _)| subsection);
            if let Some(subsection) = subsection
                && !subsections.contains(&subsection)
            {
                subsections.push(subsection);
            }
        }
        subsections
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

/// The system-wide file git reads, from its environment as `var` gives it:
/// none where `nosystem` is true, else where `system` names, where git
/// has such a variable, none where it is empty, else `default`.
pub fn system_file(
    var: impl Fn(&str) -> Option<String>,
    nosystem: &str,
    system: Option<&str>,
    default: &str,
) -> Option<PathBuf> {
    if var(nosystem).is_some_and(|value| parse_bool(Some(&value)) == Some(true)) {
        return None;
    }
    match system.and_then(&var) {
        Some(path) if path.is_empty() => None,
        Some(path) => Some(PathBuf::from(path)),
        None => Some(PathBuf::from(default)),
    }
}

/// What a parent git passed on to the commands it runs, from its
/// environment as `var` gives it, in the order config.c reads them:
/// `GIT_CONFIG_COUNT`'s pairs, then each `-c`, which `GIT_CONFIG_PARAMETERS`
/// holds shell-quoted, as `'key'='value'`, `'key'=` for a bare key, which
/// is true, or `'key'` and `'key=value'` as older gits wrote them. A pair
/// git would die on is left out, and the pairs of `GIT_CONFIG_COUNT` end at
/// the first missing, where git dies.
pub fn parameters(var: impl Fn(&str) -> Option<String>) -> Vec<(String, Option<String>)> {
    let mut pairs = Vec::new();
    let count = var("GIT_CONFIG_COUNT").and_then(|count| count.parse::<usize>().ok());
    for i in 0..count.unwrap_or(0) {
        let (Some(key), Some(value)) = (
            var(&format!("GIT_CONFIG_KEY_{i}")),
            var(&format!("GIT_CONFIG_VALUE_{i}")),
        ) else {
            break;
        };
        if let Some(key) = canonical(&key) {
            pairs.push((key, Some(value)));
        }
    }
    let Some(text) = var("GIT_CONFIG_PARAMETERS") else {
        return pairs;
    };
    let mut rest = text.as_str();
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_ascii_whitespace());
        if rest.is_empty() {
            return pairs;
        }
        let Some((word, after)) = unquoted(rest) else {
            return pairs;
        };
        let (key, value, after) = if let Some(after) = after.strip_prefix('=') {
            // An `=` with nothing after it is how git writes a bare key.
            if after.is_empty() || after.starts_with(|c: char| c.is_ascii_whitespace()) {
                (word, None, after)
            } else {
                let Some((value, after)) = unquoted(after) else {
                    return pairs;
                };
                (word, Some(value), after)
            }
        } else {
            match word.split_once('=') {
                Some((key, value)) => (key.to_owned(), Some(value.to_owned()), after),
                None => (word, None, after),
            }
        };
        if let Some(key) = canonical(&key) {
            pairs.push((key, value));
        }
        rest = after;
    }
}

/// The shell-quoted word `text` opens with, as git's `sq_quote` writes
/// one: quoted runs, with each `'` or `!` between them as `\'` or `\!`.
fn unquoted(text: &str) -> Option<(String, &str)> {
    let mut word = String::new();
    let mut rest = text.strip_prefix('\'')?;
    loop {
        let (run, after) = rest.split_once('\'')?;
        word.push_str(run);
        let mut chars = after.chars();
        if chars.next() != Some('\\') {
            return Some((word, after));
        }
        word.push(chars.next()?);
        rest = chars.as_str().strip_prefix('\'')?;
    }
}

/// A key as the files' are kept: its section and name in lower case, any
/// subsection as given; none where it lacks either.
fn canonical(key: &str) -> Option<String> {
    let (section, rest) = key.split_once('.')?;
    let (subsection, name) = match rest.rsplit_once('.') {
        Some((subsection, name)) => (Some(subsection), name),
        None => (None, rest),
    };
    if section.is_empty() || name.is_empty() {
        return None;
    }
    let mut canonical = section.to_ascii_lowercase();
    if let Some(subsection) = subsection {
        canonical.push('.');
        canonical.push_str(subsection);
    }
    canonical.push('.');
    canonical.push_str(&name.to_ascii_lowercase());
    Some(canonical)
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
    fn subsections_are_named_once_each_in_the_order_first_given() {
        let config = Config::parsed(
            "[filter \"lfs\"]\n clean = c\n smudge = s\n[filters \"near\"]\n x = 1\n\
             [filter]\n bare = 1\n[filter \"a.b\"]\n process = p\n[diff \"other\"]\n x = 1\n\
             [filter \"lfs\"]\n required = true\n",
        );
        assert_eq!(
            config.subsections("filter"),
            ["lfs", "a.b"],
            "subsections of filter were read wrongly: another section's, the section's own \
             keys, a dotted name or a repeat"
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
        let config = Config::load(&[dir.join("main")], &[], &context);
        assert_eq!(config.get("x.a"), Some("last"));
        assert_eq!(config.get("x.b"), Some("here"));
        assert_eq!(config.get("x.c"), None);
        assert_eq!(config.get("x.d"), Some("branch"));
        assert_eq!(config.stamps.len(), 4);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// What `parameters` reads from an environment holding `vars`.
    fn passed(vars: &[(&str, &str)]) -> Vec<(String, Option<String>)> {
        parameters(|name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_owned())
        })
    }

    #[test]
    fn parameters_read_as_config_c_reads_them() {
        let bare = ("core.bare".to_owned(), None);
        assert_eq!(
            passed(&[(
                "GIT_CONFIG_PARAMETERS",
                "'Core.AutoCRLF'='input' 'core.bare' \t 'Filter.LFS.Clean=c=d'",
            )]),
            [
                one("core.autocrlf", "input"),
                bare.clone(),
                one("filter.LFS.clean", "c=d"),
            ],
            "the new form, a bare key and the old form weren't each read, keys kept as \
             the files' are"
        );
        // As git 2.56 writes `-c core.bare -c core.autocrlf=true -c a.b`.
        assert_eq!(
            passed(&[(
                "GIT_CONFIG_PARAMETERS",
                "'core.bare'= 'core.autocrlf'='true' 'a.b'=",
            )]),
            [
                bare.clone(),
                one("core.autocrlf", "true"),
                ("a.b".to_owned(), None)
            ],
            "a bare key as git now writes it, with an `=` and nothing after, wasn't read"
        );
        assert_eq!(
            passed(&[("GIT_CONFIG_PARAMETERS", "'a.b'='it'\\''s'\\!'' 'a.c'=''")]),
            [one("a.b", "it's!"), one("a.c", "")],
            "a quote, an exclamation or an empty value was misread"
        );
        assert_eq!(
            passed(&[
                ("GIT_CONFIG_PARAMETERS", "'a.x'='last'"),
                ("GIT_CONFIG_COUNT", "3"),
                ("GIT_CONFIG_KEY_0", "A.X"),
                ("GIT_CONFIG_VALUE_0", "first"),
                ("GIT_CONFIG_KEY_1", "nodot"),
                ("GIT_CONFIG_VALUE_1", "v"),
                ("GIT_CONFIG_KEY_2", "a.y"),
            ]),
            [one("a.x", "first"), one("a.x", "last")],
            "the counted pairs weren't read before the -c ones, or one git dies on was kept"
        );
        assert_eq!(
            passed(&[
                ("GIT_CONFIG_COUNT", "18446744073709551615"),
                ("GIT_CONFIG_KEY_0", "a.x"),
                ("GIT_CONFIG_VALUE_0", "0"),
                ("GIT_CONFIG_KEY_2", "a.y"),
                ("GIT_CONFIG_VALUE_2", "2"),
            ]),
            [one("a.x", "0")],
            "the counted pairs went on past the first missing, where git dies"
        );
        assert_eq!(
            passed(&[("GIT_CONFIG_PARAMETERS", "'a.x'='1' unquoted 'a.y'='2'")]),
            [one("a.x", "1")],
            "what followed a word git can't read was kept"
        );
        assert_eq!(passed(&[]), [], "an environment passing nothing gave pairs");
    }

    #[test]
    fn parameters_come_after_every_file() {
        let dir = std::env::temp_dir().join(format!("git-engine-params-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("the temp folder takes a folder");
        std::fs::write(dir.join("config"), "[core]\n autocrlf = false\n")
            .expect("the temp folder takes a file");
        let context = Context {
            git_dir: &dir,
            branch: None,
            home: None,
        };
        let config = Config::load(
            &[dir.join("config")],
            &[one("core.autocrlf", "input")],
            &context,
        );
        std::fs::remove_dir_all(&dir).expect("the temp folder gives its folder back");
        assert_eq!(
            config.get("core.autocrlf"),
            Some("input"),
            "a file's value stood over one passed on the command line"
        );
    }

    #[test]
    fn the_system_file_is_where_the_environment_says() {
        let in_env = |vars: &[(&str, &str)], system: Option<&str>| {
            let vars: Vec<(String, String)> = vars
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect();
            system_file(
                move |name| {
                    vars.iter()
                        .find(|(key, _)| key == name)
                        .map(|(_, value)| value.clone())
                },
                "NOSYSTEM",
                system,
                "/etc/default",
            )
        };
        let at = |vars: &[(&str, &str)]| in_env(vars, Some("SYSTEM"));
        assert_eq!(
            in_env(&[("SYSTEM", "/x")], None),
            Some(PathBuf::from("/etc/default")),
            "a file with no variable to move it was moved"
        );
        assert_eq!(in_env(&[("NOSYSTEM", "yes")], None), None);
        assert_eq!(at(&[]), Some(PathBuf::from("/etc/default")));
        assert_eq!(at(&[("SYSTEM", "/x")]), Some(PathBuf::from("/x")));
        assert_eq!(at(&[("SYSTEM", "")]), None, "an empty path was read");
        assert_eq!(at(&[("NOSYSTEM", "1"), ("SYSTEM", "/x")]), None);
        assert_eq!(
            at(&[("NOSYSTEM", "false"), ("SYSTEM", "/x")]),
            Some(PathBuf::from("/x")),
            "a false NOSYSTEM left the system file out"
        );
    }
}
