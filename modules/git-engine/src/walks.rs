//! Ahead and behind counts already walked, kept where every shell's engine
//! finds them: how far two commits are from each other never changes, and
//! in World one count can take seconds to walk with the commit-graph cold.
//! Each line is a repository, the two commits and the two counts:
//!
//! ```text
//! <the repository's git folder, hashed> <local> <upstream> <ahead> <behind>
//! ```
//!
//! Lines are appended in one write each, so engines in several shells can
//! share the file; one that won't read is passed over. A count with nothing
//! ahead also starts the next one when only the upstream moves on.

use crate::hash::{Oid, Sha1, to_hex};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Past this size the file starts again, rather than grow for ever.
const LIMIT: u64 = 1 << 20;

pub struct Walks {
    file: Option<PathBuf>,
    repository: String,
    known: Option<Known>,
}

/// The counts read, and for each local commit the newest upstream it was
/// counted against with nothing ahead, and how far behind that was.
#[derive(Default)]
struct Known {
    counts: HashMap<(Oid, Oid), (usize, usize)>,
    newest: HashMap<Oid, (Oid, usize)>,
}

impl Known {
    fn insert(&mut self, local: Oid, upstream: Oid, counts: (usize, usize)) {
        self.counts.insert((local, upstream), counts);
        if counts.0 == 0 {
            self.newest.insert(local, (upstream, counts.1));
        }
    }
}

impl Walks {
    /// The counts for the repository whose shared git folder this is,
    /// kept under the cache folder given, or none at all for a shallow
    /// repository, whose counts stop where its history does.
    pub fn new(cache: Option<PathBuf>, common: &Path) -> Walks {
        let shallow = common.join("shallow").exists();
        let mut hash = Sha1::new();
        hash.update(common.as_os_str().as_encoded_bytes());
        Walks {
            file: cache
                .filter(|_| !shallow)
                .map(|dir| dir.join("git-engine/walks")),
            repository: to_hex(&hash.finish()),
            known: None,
        }
    }

    /// Where the cache folder is: `XDG_CACHE_HOME`, or `~/.cache`.
    pub fn folder(home: Option<&Path>) -> Option<PathBuf> {
        std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .filter(|dir| dir.is_absolute())
            .or_else(|| home.map(|home| home.join(".cache")))
    }

    pub fn get(&mut self, local: Oid, upstream: Oid) -> Option<(usize, usize)> {
        self.known().counts.get(&(local, upstream)).copied()
    }

    /// The newest upstream `local` was counted against with nothing ahead
    /// of it, and how many commits behind it was. An upstream that only
    /// moved on from there leaves it that many behind, and the commits
    /// the upstream moved by more.
    pub fn newest_level(&mut self, local: Oid) -> Option<(Oid, usize)> {
        self.known().newest.get(&local).copied()
    }

    fn known(&mut self) -> &Known {
        if self.known.is_none() {
            self.known = Some(self.load());
        }
        self.known.get_or_insert_with(Known::default)
    }

    pub fn put(&mut self, local: Oid, upstream: Oid, counts: (usize, usize)) {
        let known = self.known.get_or_insert_with(Known::default);
        known.insert(local, upstream, counts);
        let Some(file) = &self.file else {
            return;
        };
        let line = format!(
            "{} {} {} {} {}\n",
            self.repository,
            local.hex(),
            upstream.hex(),
            counts.0,
            counts.1
        );
        // A count that isn't kept is walked again; nothing else is lost.
        if let Some(dir) = file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if std::fs::metadata(file).is_ok_and(|meta| meta.len() > LIMIT) {
            let _ = std::fs::remove_file(file);
        }
        if let Ok(mut out) = OpenOptions::new().create(true).append(true).open(file) {
            let _ = out.write_all(line.as_bytes());
        }
    }

    /// What the file holds, read in the order it was written, so the
    /// newest upstream for each commit is the last one counted.
    fn load(&self) -> Known {
        let mut known = Known::default();
        let Some(text) = self.file.as_ref().and_then(|file| std::fs::read(file).ok()) else {
            return known;
        };
        for ((local, upstream), counts) in text
            .split(|&c| c == b'\n')
            .filter_map(|line| parse(line, self.repository.as_bytes()))
        {
            known.insert(local, upstream, counts);
        }
        known
    }
}

fn parse(line: &[u8], repository: &[u8]) -> Option<((Oid, Oid), (usize, usize))> {
    let mut parts = line.split(|&c| c == b' ');
    if parts.next()? != repository {
        return None;
    }
    let local = Oid::from_hex(parts.next()?)?;
    let upstream = Oid::from_hex(parts.next()?)?;
    let number = |part: &[u8]| std::str::from_utf8(part).ok()?.parse().ok();
    let ahead = number(parts.next()?)?;
    let behind = number(parts.next()?)?;
    parts
        .next()
        .is_none()
        .then_some(((local, upstream), (ahead, behind)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_are_found_again_by_another_engine_for_the_same_repository() {
        let dir = std::env::temp_dir().join(format!("git-engine-walks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let a = Oid([1; 20]);
        let b = Oid([2; 20]);
        let mut first = Walks::new(Some(dir.clone()), Path::new("/one/.git"));
        assert_eq!(first.get(a, b), None);
        first.put(a, b, (3, 16775));
        let mut second = Walks::new(Some(dir.clone()), Path::new("/one/.git"));
        assert_eq!(second.get(a, b), Some((3, 16775)));
        assert_eq!(second.get(b, a), None);
        let mut elsewhere = Walks::new(Some(dir.clone()), Path::new("/two/.git"));
        assert_eq!(elsewhere.get(a, b), None);
        // A torn or foreign line is passed over.
        let file = dir.join("git-engine/walks");
        let mut out = OpenOptions::new().append(true).open(&file).unwrap();
        out.write_all(b"garbage\nhalf a li").unwrap();
        let mut third = Walks::new(Some(dir.clone()), Path::new("/one/.git"));
        assert_eq!(third.get(a, b), Some((3, 16775)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_newest_upstream_with_nothing_ahead_is_found_again() {
        let dir = std::env::temp_dir().join(format!("git-engine-level-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let [local, older, newer, apart] = [1, 2, 3, 4].map(|byte| Oid([byte; 20]));
        let mut first = Walks::new(Some(dir.clone()), Path::new("/one/.git"));
        assert_eq!(first.newest_level(local), None);
        first.put(local, older, (0, 5));
        first.put(local, newer, (0, 7));
        // One with commits of its own ahead says nothing of the rest.
        first.put(local, apart, (2, 1));
        assert_eq!(first.newest_level(local), Some((newer, 7)));
        assert_eq!(first.newest_level(newer), None);
        let mut second = Walks::new(Some(dir.clone()), Path::new("/one/.git"));
        assert_eq!(second.newest_level(local), Some((newer, 7)));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
