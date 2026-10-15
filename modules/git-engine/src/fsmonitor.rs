//! Asks git's file system monitor what has changed in a worktree since the
//! last question, so a question needn't look at every file. The daemon
//! (`git fsmonitor--daemon`) listens on `fsmonitor--daemon.ipc` in the
//! worktree's git folder and speaks pkt-lines: a token goes in, and back
//! comes a new token, a NUL, and either `/` for "anything may have changed"
//! or each path that has, relative to the worktree and ended by a NUL. It
//! writes a cookie file and waits to hear of it before answering, so an
//! answer holds every change made before the question.
//!
//! git keeps its token in the index; the engine never writes the index, so
//! it keeps its own.

use std::collections::HashSet;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// What may have changed since the last question.
pub enum Since {
    Anything,
    Only(Changed),
}

/// The paths the daemon named, each standing for itself and everything
/// under it.
#[derive(Default)]
pub struct Changed {
    paths: HashSet<Vec<u8>>,
    /// The folder holding each path named, whose listing may have changed.
    parents: HashSet<Vec<u8>>,
    /// Every folder holding a path named, however far up.
    ancestors: HashSet<Vec<u8>>,
    /// Whether an attributes file was named, which changes how every file
    /// below it is read.
    pub attributes: bool,
}

fn parent(path: &[u8]) -> &[u8] {
    path.iter()
        .rposition(|&c| c == b'/')
        .map_or(&[][..], |at| &path[..at])
}

impl Changed {
    pub fn of<'a>(paths: impl Iterator<Item = &'a [u8]>) -> Changed {
        let mut changed = Changed::default();
        for path in paths {
            let path = path.strip_suffix(b"/").unwrap_or(path);
            if path.is_empty() {
                continue;
            }
            let name = &path[parent(path).len()..];
            if name.strip_prefix(b"/").unwrap_or(name) == b".gitattributes" {
                changed.attributes = true;
            }
            changed.parents.insert(parent(path).to_vec());
            let mut up = parent(path);
            while !up.is_empty() && changed.ancestors.insert(up.to_vec()) {
                up = parent(up);
            }
            changed.paths.insert(path.to_vec());
        }
        changed
    }

    /// Whether a path, or a folder holding it, was named.
    pub fn touches(&self, path: &[u8]) -> bool {
        if self.paths.is_empty() {
            return false;
        }
        let mut at = path.len();
        loop {
            if self.paths.contains(&path[..at]) {
                return true;
            }
            match path[..at].iter().rposition(|&c| c == b'/') {
                Some(slash) => at = slash,
                None => return false,
            }
        }
    }

    /// Whether what a folder holds may have changed: something in it was
    /// named, or it was, or a folder holding it. `dir` is empty for the
    /// worktree, or ends in a slash.
    pub fn touches_dir(&self, dir: &[u8]) -> bool {
        let dir = dir.strip_suffix(b"/").unwrap_or(dir);
        self.parents.contains(dir) || (!dir.is_empty() && self.touches(dir))
    }

    /// Whether anything in a folder may have changed, however deep: what
    /// it holds, or the folder itself. `dir` is empty for the worktree, or
    /// ends in a slash.
    pub fn touches_under(&self, dir: &[u8]) -> bool {
        let dir = dir.strip_suffix(b"/").unwrap_or(dir);
        if dir.is_empty() {
            return !self.paths.is_empty();
        }
        self.ancestors.contains(dir) || self.touches(dir)
    }
}

pub struct Monitor {
    socket: PathBuf,
    token: Option<String>,
}

/// How long an answer may take before the engine looks for itself.
const PATIENCE: Duration = Duration::from_secs(1);

impl Monitor {
    /// The monitor for the worktree whose git folder this is, when git is
    /// set to use the daemon and it's listening.
    pub fn for_worktree(git_dir: &Path) -> Option<Monitor> {
        let socket = git_dir.join("fsmonitor--daemon.ipc");
        socket.exists().then_some(Monitor {
            socket,
            token: None,
        })
    }

    /// What may have changed since the last time this was asked. Anything
    /// that goes wrong means anything may have.
    pub fn ask(&mut self) -> Since {
        // A token from no daemon at all asks for a fresh one.
        let token = self.token.take().unwrap_or_else(|| "builtin:fake".into());
        let Ok(answer) = query(&self.socket, &token) else {
            return Since::Anything;
        };
        let Some(end) = answer.iter().position(|&c| c == 0) else {
            return Since::Anything;
        };
        self.token = Some(String::from_utf8_lossy(&answer[..end]).into_owned());
        let rest = &answer[end + 1..];
        if rest.starts_with(b"/\0") || rest == b"/" {
            return Since::Anything;
        }
        Since::Only(Changed::of(
            rest.split(|&c| c == 0).filter(|path| !path.is_empty()),
        ))
    }
}

/// Connects to a socket, whose path may be longer than a socket address
/// holds: then from its folder, by its name alone, as git does. Everything
/// else here names files by absolute paths, so the step aside is unseen.
fn connect(socket: &Path) -> io::Result<UnixStream> {
    if socket.as_os_str().len() < 100 {
        return UnixStream::connect(socket);
    }
    let (Some(folder), Some(name)) = (socket.parent(), socket.file_name()) else {
        return UnixStream::connect(socket);
    };
    let back = std::env::current_dir()?;
    std::env::set_current_dir(folder)?;
    let stream = UnixStream::connect(name);
    std::env::set_current_dir(back)?;
    stream
}

fn query(socket: &Path, token: &str) -> io::Result<Vec<u8>> {
    let mut stream = connect(socket)?;
    stream.set_read_timeout(Some(PATIENCE))?;
    stream.set_write_timeout(Some(PATIENCE))?;
    let mut request = format!("{:04x}", token.len() + 4).into_bytes();
    request.extend_from_slice(token.as_bytes());
    request.extend_from_slice(b"0000");
    stream.write_all(&request)?;
    let mut answer = Vec::new();
    loop {
        let mut length = [0; 4];
        stream.read_exact(&mut length)?;
        let length = std::str::from_utf8(&length)
            .ok()
            .and_then(|hex| usize::from_str_radix(hex, 16).ok())
            .ok_or_else(|| io::Error::other("not a pkt-line"))?;
        if length == 0 {
            return Ok(answer);
        }
        let mut data = vec![
            0;
            length
                .checked_sub(4)
                .ok_or_else(|| io::Error::other("short pkt-line"))?
        ];
        stream.read_exact(&mut data)?;
        answer.extend_from_slice(&data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_named_path_stands_for_what_is_under_it() {
        let changed = Changed::of([&b"a/b"[..], b"c/", b"d/.gitattributes"].into_iter());
        assert!(changed.touches(b"a/b"));
        assert!(changed.touches(b"a/b/c"));
        assert!(changed.touches(b"c/x"));
        assert!(!changed.touches(b"a/bc"));
        assert!(!changed.touches(b"a"));
        assert!(changed.touches_dir(b"a/"));
        assert!(changed.touches_dir(b""));
        assert!(changed.touches_dir(b"a/b/"));
        assert!(changed.touches_dir(b"c/sub/"));
        assert!(!changed.touches_dir(b"e/"));
        let deep = Changed::of([&b"x/y/z/file"[..]].into_iter());
        assert!(deep.touches_under(b""));
        assert!(deep.touches_under(b"x/"));
        assert!(deep.touches_under(b"x/y/z/"));
        assert!(deep.touches_under(b"x/y/z/file/"));
        assert!(!deep.touches_under(b"x/q/"));
        assert!(!deep.touches_under(b"x/y/zz/"));
        assert!(!Changed::default().touches_under(b""));
        assert!(changed.attributes);
        assert!(!Changed::of([&b"x"[..]].into_iter()).attributes);
    }
}
