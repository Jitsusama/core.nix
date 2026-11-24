//! The results journal read a line at a time, so the watch never holds it
//! whole, and the lines of one commit's runs found by its bytes before any
//! is parsed.

use std::fs;
use std::io::{self, BufRead as _, BufReader};
use std::path::{Path, PathBuf};

use super::say_in;

/// The results journal, read a line at a time into one buffer, giving the
/// lines `keep` takes as text, so a line passed over costs no allocation
/// of its own.
pub(super) struct Lines<F> {
    reader: Option<BufReader<fs::File>>,
    line: Vec<u8>,
    keep: F,
    root: PathBuf,
}

impl<F: FnMut(&[u8]) -> bool> Lines<F> {
    /// The journal in the state folder `root`. One that can't be opened,
    /// but for one not yet written, is said in net.log, as a read that
    /// fails part way is.
    pub(super) fn open(root: &Path, keep: F) -> Self {
        let path = root.join("results.jsonl");
        let reader = match fs::File::open(&path) {
            Ok(file) => Some(BufReader::new(file)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => {
                say_in(root, &format!("couldn't open {}: {error}", path.display()));
                None
            }
        };
        Self {
            reader,
            line: Vec::new(),
            keep,
            root: root.to_owned(),
        }
    }
}

impl<F: FnMut(&[u8]) -> bool> Iterator for Lines<F> {
    type Item = String;

    /// The next line taken. A read that fails ends the lines for good,
    /// rather than failing again on each call, as a read that consumed
    /// nothing would. The caller then has only the runs before it, which
    /// can only undercount them, so it's said in net.log for whoever reads
    /// what the net decided from it.
    fn next(&mut self) -> Option<String> {
        let reader = self.reader.as_mut()?;
        loop {
            self.line.clear();
            match reader.read_until(b'\n', &mut self.line) {
                Ok(0) => break,
                Ok(_) if (self.keep)(&self.line) => {
                    return Some(String::from_utf8_lossy(&self.line).into_owned());
                }
                Ok(_) => {}
                Err(error) => {
                    say_in(
                        &self.root,
                        &format!(
                            "reading {} stopped part way, so what was asked of it has \
                             only the runs before: {error}",
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
