//! The patterns of ignore and attributes files, parsed and matched as dir.c
//! does: a pattern without a slash matches a name in any folder, one with a
//! slash matches from the folder its file is in, and a trailing slash
//! matches only folders.

use crate::wildmatch;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pattern {
    pub text: Vec<u8>,
    pub negated: bool,
    pub only_dirs: bool,
    /// No slash in it: matched against a path's last part.
    pub basename: bool,
    /// How much of it is literal before the first wildcard.
    literal: usize,
}

fn is_special(c: u8) -> bool {
    matches!(c, b'*' | b'?' | b'[' | b'\\')
}

impl Pattern {
    /// One line of an ignore file, or None for a blank line or a comment.
    pub fn parse(line: &[u8], allow_negation: bool) -> Option<Pattern> {
        let mut line = trim_trailing_spaces(line);
        if line.is_empty() || line[0] == b'#' {
            return None;
        }
        let mut negated = false;
        if allow_negation && line[0] == b'!' {
            negated = true;
            line = &line[1..];
        }
        let mut only_dirs = false;
        if let Some(rest) = line.strip_suffix(b"/") {
            only_dirs = true;
            line = rest;
        }
        if line.is_empty() {
            return None;
        }
        let basename = !line.contains(&b'/');
        let literal = line
            .iter()
            .position(|&c| is_special(c))
            .unwrap_or(line.len());
        Some(Pattern {
            text: line.to_vec(),
            negated,
            only_dirs,
            basename,
            literal,
        })
    }

    /// Whether this matches `path`, relative to the repository's top, where
    /// the pattern's file sits in `base` (empty, or a folder with no
    /// trailing slash).
    pub fn matches(&self, path: &[u8], is_dir: bool, base: &[u8]) -> bool {
        if self.only_dirs && !is_dir {
            return false;
        }
        // A file's patterns speak only for what's under its folder.
        let name = if base.is_empty() {
            path
        } else {
            match path.strip_prefix(base) {
                Some(rest) if rest.first() == Some(&b'/') => &rest[1..],
                _ => return false,
            }
        };
        if self.basename {
            let name = path.rsplit(|&c| c == b'/').next().unwrap_or(path);
            if self.literal == self.text.len() {
                return name == self.text.as_slice();
            }
            if self.text[0] == b'*' && !self.text[1..].iter().any(|&c| is_special(c)) {
                return name.ends_with(&self.text[1..]);
            }
            return wildmatch::matches(&self.text, name, true, false);
        }
        let (pattern, literal) = match self.text.strip_prefix(b"/") {
            Some(rest) => (rest, self.literal.saturating_sub(1)),
            None => (self.text.as_slice(), self.literal),
        };
        if literal > 0 {
            if name.len() < literal || name[..literal] != pattern[..literal] {
                return false;
            }
            if literal == pattern.len() {
                return name.len() == literal;
            }
            return wildmatch::matches(&pattern[literal..], &name[literal..], true, false);
        }
        wildmatch::matches(pattern, name, true, false)
    }
}

/// git drops trailing spaces from a pattern unless a backslash escapes one.
fn trim_trailing_spaces(line: &[u8]) -> &[u8] {
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    let mut end = line.len();
    let mut keep = 0;
    let mut at = 0;
    while at < line.len() {
        match line[at] {
            b' ' => {}
            b'\\' => {
                at += 1;
                keep = (at + 1).min(line.len());
            }
            _ => keep = at + 1,
        }
        at += 1;
        end = keep;
    }
    &line[..end.min(line.len())]
}

pub fn parse_file(text: &[u8], allow_negation: bool) -> Vec<Pattern> {
    text.split(|&c| c == b'\n')
        .filter_map(|line| Pattern::parse(line, allow_negation))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ignored(pattern: &str, path: &str, is_dir: bool, base: &str) -> bool {
        Pattern::parse(pattern.as_bytes(), true).unwrap().matches(
            path.as_bytes(),
            is_dir,
            base.as_bytes(),
        )
    }

    #[test]
    fn a_name_matches_in_any_folder() {
        assert!(ignored("target", "a/b/target", true, ""));
        assert!(ignored("*.o", "a/b/x.o", false, ""));
        assert!(!ignored("*.o", "a/b/x.oo", false, ""));
    }

    #[test]
    fn a_slash_anchors_to_its_files_folder() {
        assert!(ignored("/build", "build", true, ""));
        assert!(!ignored("/build", "a/build", true, ""));
        assert!(ignored("/build", "sub/build", true, "sub"));
        assert!(ignored("doc/*.txt", "sub/doc/a.txt", false, "sub"));
        assert!(!ignored("doc/*.txt", "sub/doc/x/a.txt", false, "sub"));
        assert!(ignored("doc/**/a.txt", "doc/x/y/a.txt", false, ""));
    }

    #[test]
    fn a_trailing_slash_matches_only_folders() {
        assert!(ignored("cache/", "x/cache", true, ""));
        assert!(!ignored("cache/", "x/cache", false, ""));
    }

    #[test]
    fn lines_read_as_git_reads_them() {
        assert!(Pattern::parse(b"# comment", true).is_none());
        assert!(Pattern::parse(b"   ", true).is_none());
        let negated = Pattern::parse(b"!keep", true).unwrap();
        assert!(negated.negated);
        assert_eq!(
            Pattern::parse(b"trailing   ", true).unwrap().text,
            b"trailing"
        );
        assert_eq!(Pattern::parse(b"space\\ ", true).unwrap().text, b"space\\ ");
        assert!(ignored("\\#hash", "#hash", false, ""));
        assert!(ignored("\\!bang", "!bang", false, ""));
    }
}
