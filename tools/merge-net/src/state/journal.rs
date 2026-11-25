//! The results journal read a line at a time, so the watch never holds it
//! whole, and the lines of one commit's runs found by its bytes before any
//! is parsed.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead as _, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::say_in;

/// The results journal, read a line at a time into one buffer, giving the
/// lines `keep` takes as text, so a line passed over costs no allocation
/// of its own.
pub(super) struct Lines<F> {
    reader: Option<BufReader<fs::File>>,
    line: Vec<u8>,
    keep: F,
    root: PathBuf,
    failed: bool,
}

impl<F: FnMut(&[u8]) -> bool> Lines<F> {
    /// The journal in the state folder `root`. One that can't be opened,
    /// but for one not yet written, is said in net.log and counts as
    /// failed, as a read that fails part way does.
    pub(super) fn open(root: &Path, keep: F) -> Self {
        let path = root.join("results.jsonl");
        let mut failed = false;
        let reader = match fs::File::open(&path) {
            Ok(file) => Some(BufReader::new(file)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => {
                failed = true;
                say_unrepeated(
                    root,
                    &format!(
                        "couldn't open {}, so the net reads it as holding no run: {error}",
                        path.display()
                    ),
                );
                None
            }
        };
        Self {
            reader,
            line: Vec::new(),
            keep,
            root: root.to_owned(),
            failed,
        }
    }

    /// Whether the journal couldn't be opened, or a read of it failed, so
    /// the lines given were fewer than it holds. A caller then answers as
    /// though it held none, as the journal read whole did: a prefix could
    /// keep a commit's failure and drop the pass after it, and blame a
    /// merge for a test that passed, where no runs at all leaves it
    /// unknown.
    pub(super) const fn failed(&self) -> bool {
        self.failed
    }

    /// Whether the lines have ended for good, so a later call reads nothing.
    #[cfg(test)]
    pub(super) const fn ended(&self) -> bool {
        self.reader.is_none()
    }
}

impl<F: FnMut(&[u8]) -> bool> Iterator for Lines<F> {
    type Item = String;

    /// The next line taken. A read that fails ends the lines for good,
    /// rather than failing again on each call, as a read that consumed
    /// nothing would.
    fn next(&mut self) -> Option<String> {
        let reader = self.reader.as_mut()?;
        loop {
            self.line.clear();
            match reader.read_until(b'\n', &mut self.line) {
                Ok(0) => {
                    // Read to its end, the journal's last fault is over, so
                    // the same fault coming back later is said again.
                    forget_fault(&self.root);
                    break;
                }
                Ok(_) if (self.keep)(&self.line) => {
                    return Some(String::from_utf8_lossy(&self.line).into_owned());
                }
                Ok(_) => {}
                Err(error) => {
                    self.failed = true;
                    say_unrepeated(
                        &self.root,
                        &format!(
                            "reading {} stopped part way, so the net reads it as holding \
                             no run: {error}",
                            self.root.join("results.jsonl").display()
                        ),
                    );
                    break;
                }
            }
        }
        self.reader = None;
        None
    }
}

/// The last fault with each state folder's journal said in its net.log by
/// this process.
static LAST_SAID: Mutex<BTreeMap<PathBuf, String>> = Mutex::new(BTreeMap::new());

/// Forget the last fault said for `root`'s journal, once a read of it has
/// reached its end.
fn forget_fault(root: &Path) {
    LAST_SAID
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(root);
}

/// Say `what` in the net.log under `root` unless it was the last fault
/// said there, and no read has reached the journal's end since. A journal that stays unreadable is asked of on every query,
/// and net.log is never trimmed; its time is also the hang guard's sign
/// that the watch lives, which a line per query would keep fresh for a
/// watch that's stuck.
fn say_unrepeated(root: &Path, what: &str) {
    let mut said = LAST_SAID
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if said.get(root).map(String::as_str) != Some(what) {
        say_in(root, what);
        said.insert(root.to_owned(), what.to_owned());
    }
}

/// The bytes a journal line naming `commit` holds, or none when any line
/// could be of it: an empty commit, which `windows` can't look for, or
/// one holding what JSON writes escaped, whose bytes in the journal
/// differ from its own. The commits recorded are git's, in hex.
pub(super) fn sought(commit: &str) -> Option<Vec<u8>> {
    let plain = !commit.is_empty()
        && commit
            .bytes()
            .all(|byte| byte >= 0x20 && byte != b'"' && byte != b'\\');
    plain.then(|| commit.as_bytes().to_owned())
}

/// Whether `line` could be of the commit whose bytes are `sought`.
pub(super) fn names(line: &[u8], sought: Option<&[u8]>) -> bool {
    sought.is_none_or(|commit| line.windows(commit.len()).any(|part| part == commit))
}
