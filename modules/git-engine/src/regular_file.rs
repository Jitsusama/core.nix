//! Opening a file of git's that must be a plain one: a ref, packed-refs, a
//! loose object, a reftable, a commit-graph or a pack's index. A FIFO there
//! would hold the prompt till something wrote to it, and a device such as
//! /dev/zero would give bytes without end, so each is opened without waiting
//! on a writer and refused unless it's a regular file, as git refuses one it
//! can't read as a ref.

use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

/// The regular file at `path`, or None where nothing is; anything else
/// there, a FIFO, a device or a folder, is refused as data that won't read.
pub(crate) fn open(path: &Path) -> io::Result<Option<File>> {
    let file = match OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOCTTY)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if file.metadata()?.is_file() {
        Ok(Some(file))
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} isn't a file", path.display()),
        ))
    }
}

/// The regular file at `path`, for a reader to whom nothing there is an
/// error like any other.
pub(crate) fn present(path: &Path) -> io::Result<File> {
    open(path)?.ok_or_else(|| missing(path))
}

/// The bytes of the regular file at `path`, for a reader to whom nothing
/// there is an error like any other.
pub(crate) fn read_present(path: &Path) -> io::Result<Vec<u8>> {
    read(path)?.ok_or_else(|| missing(path))
}

fn missing(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("{} isn't there", path.display()),
    )
}

/// The bytes of the regular file at `path`, or None where nothing is.
pub(crate) fn read(path: &Path) -> io::Result<Option<Vec<u8>>> {
    let Some(mut file) = open(path)? else {
        return Ok(None);
    };
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(Some(bytes))
}

#[cfg(test)]
mod tests {
    use super::{present, read, read_present};
    use std::path::Path;

    #[test]
    fn only_a_regular_file_is_read() {
        let folder = crate::pack_index::tests::unique("regular");
        std::fs::create_dir_all(&folder).expect("the temp folder takes a folder");
        let plain = folder.join("plain");
        std::fs::write(&plain, b"ref").expect("the temp folder takes a file");
        let fifo = folder.join("fifo");
        let made = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("mkfifo runs");
        assert!(made.success(), "mkfifo made no FIFO at {}", fifo.display());
        let read = |path: &Path| read(path).map_err(|error| error.kind());
        let found = [
            read(&plain),
            read(&folder.join("none")),
            read(&fifo),
            read(Path::new("/dev/zero")),
            read(&folder),
        ];
        let kind = |result: std::io::Result<Vec<u8>>| result.map_err(|error| error.kind());
        let opened = |path: &Path| kind(present(path).map(|_| Vec::new()));
        let either = [
            kind(read_present(&plain)),
            kind(read_present(&folder.join("none"))),
            kind(read_present(&fifo)),
            opened(&plain),
            opened(&folder.join("none")),
            opened(&fifo),
        ];
        std::fs::remove_dir_all(&folder).expect("the temp folder gives its folder back");
        let gone = Err(std::io::ErrorKind::NotFound);
        let refused = Err(std::io::ErrorKind::InvalidData);
        assert_eq!(
            either,
            [
                Ok(b"ref".to_vec()),
                gone.clone(),
                refused.clone(),
                Ok(Vec::new()),
                gone,
                refused.clone()
            ],
            "a reader to whom nothing there is an error didn't read a plain file, took a \
             missing one for something else, or read or waited on a FIFO"
        );
        assert_eq!(
            found,
            [
                Ok(Some(b"ref".to_vec())),
                Ok(None),
                refused.clone(),
                refused.clone(),
                refused
            ],
            "a plain file wasn't read, a missing one wasn't none, or a FIFO, a device or \
             a folder was read, or waited on, as a file"
        );
    }

    /// Makes a FIFO at `path`, which nothing will ever write to.
    fn fifo(path: &Path) {
        let folder = path.parent().expect("a FIFO is made in a folder");
        std::fs::create_dir_all(folder).expect("the temp folder takes a folder");
        let made = std::process::Command::new("mkfifo")
            .arg(path)
            .status()
            .expect("mkfifo runs");
        assert!(made.success(), "mkfifo made no FIFO at {}", path.display());
    }

    // Each reader of git's own files meets a FIFO where its file should be.
    // One that waited on it would wait for ever, so they run apart and the
    // test fails by its message once they've taken far longer than reading
    // a few small files can.
    #[test]
    fn no_reader_of_gits_files_waits_on_a_fifo() {
        use crate::hash::Oid;
        use crate::reftable::Value;
        let git = crate::pack_index::tests::unique("fifos");
        let main = "1111111111111111111111111111111111111111";
        let stash = "2222222222222222222222222222222222222222";
        std::fs::create_dir_all(git.join("refs/heads/folder"))
            .expect("the temp folder takes a folder");
        std::fs::write(
            git.join("packed-refs"),
            format!("{main} refs/heads/main\n{main} refs/heads/folder\n"),
        )
        .expect("the temp folder takes a file");
        std::fs::write(git.join("refs/stash"), stash).expect("the temp folder takes a file");
        let cache = git.join("cache");
        for path in [
            git.join("refs/heads/main"),
            git.join("logs/refs/stash"),
            git.join("reftable/tables.list"),
            git.join("objects/info/commit-graphs/commit-graph-chain"),
            git.join("objects/pack/pack-a.idx"),
            git.join("objects/pack/multi-pack-index"),
            cache.join("git-engine/walks"),
        ] {
            fifo(&path);
        }
        let (send, heard) = std::sync::mpsc::channel();
        let readers = git.clone();
        std::thread::spawn(move || {
            let git = readers;
            let refs = crate::refs::Refs::open(&git, &git, false)
                .expect("loose refs open with a FIFO among them");
            let found = (refs.read("refs/heads/main"), refs.read("refs/heads/folder"));
            let stashes = refs.stashes();
            let reftable = crate::reftable::Stack::read(&git.join("reftable"))
                .map(|_| ())
                .map_err(|error| error.kind());
            let _graph = crate::graph::Graph::open(&git.join("objects"));
            let pack = git.join("objects/pack");
            let index = crate::pack_index::Index::open(&pack.join("pack-a.idx"))
                .map(|_| ())
                .map_err(|error| error.kind());
            let midx = crate::pack_index::Midx::open(&pack.join("multi-pack-index"))
                .map(|_| ())
                .map_err(|error| error.kind());
            let (a, b) = (Oid([1; 20]), Oid([2; 20]));
            let mut walks = crate::walks::Walks::new(Some(cache), &git);
            let before = walks.get(a, b);
            walks.put(a, b, (1, 2));
            let _ = send.send((found, stashes, reftable, index, midx, before));
        });
        let heard = heard.recv_timeout(std::time::Duration::from_secs(30));
        let _ = std::fs::remove_dir_all(&git);
        let packed = Oid::from_hex(main.as_bytes()).map(Value::Oid);
        let refused = Err(std::io::ErrorKind::InvalidData);
        assert_eq!(
            heard.expect("a reader of git's files was still waiting on a FIFO after 30 s"),
            ((packed.clone(), packed), 0, refused, refused, refused, None),
            "a loose ref that isn't a file wasn't passed over for the packed one, a stash \
             log that isn't one was counted, or a reftable list or a pack's index that \
             isn't a file was read"
        );
    }
}
