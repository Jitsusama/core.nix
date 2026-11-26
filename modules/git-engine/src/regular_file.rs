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
/// The type is known only from the file opened, so the open comes first:
/// a device whose opening does something has it done, though nothing is
/// read from it.
/// `O_NONBLOCK` stays set on the file returned, which changes nothing for
/// a regular file's reads.
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
pub(crate) mod tests {
    use super::{present, read, read_present};
    use std::path::Path;

    /// What `work` answers, on a thread of its own, or Err where it took
    /// past 30 s or panicked. A reader that waited on a FIFO would wait for
    /// ever, so each runs apart, and a test fails by its message rather
    /// than hanging; the thread is left to the test's process to end.
    pub(crate) fn apart<T: Send + 'static>(
        work: impl FnOnce() -> T + Send + 'static,
    ) -> Result<T, std::sync::mpsc::RecvTimeoutError> {
        let (send, heard) = std::sync::mpsc::channel();
        std::thread::spawn(move || send.send(work()));
        heard.recv_timeout(std::time::Duration::from_secs(30))
    }

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
        let (none, readers) = (folder.join("none"), folder.clone());
        let heard = apart(move || {
            let read = |path: &Path| read(path).map_err(|error| error.kind());
            let found = [
                read(&plain),
                read(&none),
                read(&fifo),
                read(Path::new("/dev/zero")),
                read(&readers),
            ];
            let kind = |result: std::io::Result<Vec<u8>>| result.map_err(|error| error.kind());
            let opened = |path: &Path| kind(present(path).map(|_| Vec::new()));
            let either = [
                kind(read_present(&plain)),
                kind(read_present(&none)),
                kind(read_present(&fifo)),
                opened(&plain),
                opened(&none),
                opened(&fifo),
            ];
            (found, either)
        });
        std::fs::remove_dir_all(&folder).expect("the temp folder gives its folder back");
        let (found, either) =
            heard.expect("a read of a FIFO or a device was still waiting after 30 s");
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
                refused
            ],
            "a reader to whom nothing there is an error didn't read a plain file, took a \
             missing one for something else, or read or waited on a FIFO"
        );
        let refused = Err(std::io::ErrorKind::InvalidData);
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
    pub(crate) fn fifo(path: &Path) {
        let folder = path.parent().expect("a FIFO is made in a folder");
        std::fs::create_dir_all(folder).expect("the temp folder takes a folder");
        let made = std::process::Command::new("mkfifo")
            .arg(path)
            .status()
            .expect("mkfifo runs");
        assert!(made.success(), "mkfifo made no FIFO at {}", path.display());
    }

    // Each reader of git's own files meets a FIFO where its file should be,
    // apart, as above.
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
        let readers = git.clone();
        let heard = apart(move || {
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
            (found, stashes, reftable, index, midx, before)
        });
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

    // packed-refs, a loose tag, the alternates and a loose object each a
    // FIFO, as the readers of the refs, the tags and the objects meet them,
    // apart, as above.
    #[test]
    fn no_reader_of_tags_or_objects_waits_on_a_fifo() {
        use crate::hash::Oid;
        use crate::reftable::Value;
        let git = crate::pack_index::tests::unique("fifo-tags");
        let main = "1111111111111111111111111111111111111111";
        std::fs::create_dir_all(git.join("refs/heads")).expect("the temp folder takes a folder");
        std::fs::create_dir_all(git.join("refs/tags")).expect("the temp folder takes a folder");
        std::fs::write(git.join("refs/heads/main"), main).expect("the temp folder takes a file");
        std::fs::write(git.join("refs/tags/v2"), main).expect("the temp folder takes a file");
        let object = Oid([3; 20]);
        let hex = object.hex();
        for path in [
            git.join("packed-refs"),
            git.join("refs/tags/v1"),
            git.join("objects/info/alternates"),
            git.join("objects").join(&hex[..2]).join(&hex[2..]),
        ] {
            fifo(&path);
        }
        let readers = git.clone();
        let heard = apart(move || {
            let git = readers;
            let main = crate::refs::Refs::open(&git, &git, false)
                .map(|refs| refs.read("refs/heads/main"))
                .map_err(|error| error.kind());
            let (mut stamps, mut peeled) = (Vec::new(), std::collections::HashMap::new());
            let tags = crate::tags::files(&git, &mut stamps, &mut peeled);
            let mut odb = crate::odb::Odb::open(&git.join("objects"));
            let kind = odb.kind(&object).map(|_| ()).map_err(|error| error.kind());
            let read = odb.read(&object).map(|_| ()).map_err(|error| error.kind());
            (main, tags, kind, read)
        });
        let _ = std::fs::remove_dir_all(&git);
        let oid = Oid::from_hex(main.as_bytes()).expect("main's line is an oid");
        let refused = Err(std::io::ErrorKind::InvalidData);
        assert_eq!(
            heard
                .expect("a reader of refs, tags or objects was still waiting on a FIFO after 30 s"),
            (
                Ok(Some(Value::Oid(oid))),
                vec![("v2".to_owned(), oid)],
                refused,
                refused
            ),
            "a loose ref beside a packed-refs that isn't a file wasn't read, a loose tag \
             that isn't one was named, or a loose object that isn't a file was kinded or \
             read"
        );
    }

    // A worktree's .git file, a commondir and a HEAD each a FIFO, as finding
    // a repository meets them; two packs, one a multi-pack-index names, each
    // a FIFO beside a sound index; and a reftable's one table a FIFO: apart,
    // as above.
    #[test]
    fn no_reader_of_where_a_repository_is_nor_of_its_packs_waits_on_a_fifo() {
        use crate::hash::Oid;
        use crate::pack_index::tests::{midx, sorted, version_2};
        use crate::repo::{Location, discover};
        let top = crate::pack_index::tests::unique("fifo-places");
        let linked = top.join("linked");
        fifo(&linked.join(".git"));
        let shared = top.join("shared");
        let git = shared.join(".git");
        fifo(&git.join("commondir"));
        fifo(&git.join("HEAD"));
        let pack = git.join("objects/pack");
        for name in ["pack-a", "pack-b"] {
            fifo(&pack.join(format!("{name}.pack")));
            std::fs::write(pack.join(format!("{name}.idx")), version_2(&sorted()))
                .expect("the temp folder takes a file");
        }
        std::fs::write(
            pack.join("multi-pack-index"),
            midx(&["pack-a.idx"], 1, None),
        )
        .expect("the temp folder takes a file");
        fifo(&git.join("reftable/t.ref"));
        std::fs::write(git.join("reftable/tables.list"), "t.ref\n")
            .expect("the temp folder takes a file");
        let found = Location {
            workdir: shared.clone(),
            git_dir: git.clone(),
            common: std::fs::canonicalize(&git).expect("the temp folder has a real path"),
        };
        let (linked_at, shared_at, readers) = (linked.clone(), shared.clone(), git.clone());
        let heard = apart(move || {
            let git = readers;
            let linked = discover(&linked_at).map(|location| location.workdir);
            let shared = discover(&shared_at);
            let refs = crate::refs::Refs::open(&git, &git, false)
                .expect("loose refs open with none there");
            let head = shared
                .as_ref()
                .map(|location| crate::repo::head(location, &refs, false));
            let object = crate::odb::Odb::open(&git.join("objects"))
                .read(&Oid(sorted()[0]))
                .map(|object| object.is_some())
                .map_err(|error| error.kind());
            let table = crate::reftable::Stack::read(&git.join("reftable"))
                .map(|_| ())
                .map_err(|error| error.kind());
            (linked, shared, head, object, table)
        });
        let _ = std::fs::remove_dir_all(&top);
        let (linked_found, shared_found, at_head, object, table) = heard.expect(
            "a reader of where a repository is, its HEAD, its packs or its reftable was still \
             waiting on a FIFO after 30 s",
        );
        assert_ne!(
            linked_found,
            Some(linked),
            "a .git file that isn't a file named a repository"
        );
        assert_eq!(
            (shared_found, at_head, object, table),
            (
                Some(found),
                Some(None),
                Ok(false),
                Err(std::io::ErrorKind::InvalidData)
            ),
            "a commondir that isn't a file wasn't passed over for the git folder itself, a \
             HEAD that isn't one was read, a pack that isn't one was read from, or a \
             reftable's table that isn't one was read"
        );
    }
}
