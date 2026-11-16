//! A gate's folder in a quest's scratch, as `answer` reads and writes it.
//!
//! `answer` runs as the person, outside the agent fence, and the quest's
//! scratch is the quest's agents' to write, so any name in it may be a
//! link or a FIFO planted to turn a read or a write onto something of the
//! person's (security review, 16 November). So the folder is opened once,
//! from the scratch's root, by a lookup that follows no link and leaves
//! no folder below the root, and every read and write after is relative
//! to that descriptor: a read takes only a plain file, a write makes a
//! new file and follows no link, then names it whole.

use std::ffi::CString;
use std::fs;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::path::{Path, PathBuf};

use super::GATES;

/// A gate's folder, held open.
#[derive(Debug)]
pub struct GateFolder {
    folder: OwnedFd,
    /// Where it is, for what's said.
    shown: PathBuf,
}

/// `name` as C wants it; a gate's names are its own constants.
fn named(name: &str) -> io::Result<CString> {
    CString::new(name).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))
}

/// What a descriptor a call returned holds, or the call's error.
fn opened(fd: libc::c_int) -> io::Result<OwnedFd> {
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        // SAFETY: the call just returned this descriptor, and nothing else
        // owns it.
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }
}

/// Whether `fd` is a plain file.
fn plain(fd: &OwnedFd) -> io::Result<bool> {
    // SAFETY: a stat is plain data, and fstat fills it.
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    // SAFETY: fstat on a descriptor this owns, into a stat this holds.
    if unsafe { libc::fstat(fd.as_raw_fd(), &raw mut stat) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(stat.st_mode & libc::S_IFMT == libc::S_IFREG)
}

impl GateFolder {
    /// The gate folder of `quest` and `commit` under `root`, the folder
    /// each quest's scratch is in, opened through no link.
    ///
    /// # Errors
    ///
    /// When it isn't there, or a link or anything but a folder stands
    /// on the way to it.
    pub fn open(root: &Path, quest: &str, commit: &str) -> io::Result<Self> {
        let beneath = format!("{quest}/{GATES}/{commit}");
        let shown = root.join(&beneath);
        let root_name = CString::new(root.as_os_str().as_encoded_bytes())
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        // SAFETY: open of a path this holds.
        let top = opened(unsafe {
            libc::open(
                root_name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        })?;
        let folder = beneath_no_links(&top, &named(&beneath)?)?;
        Ok(Self { folder, shown })
    }

    /// Whether it holds `name` as a plain file, not a link to one.
    #[must_use]
    pub fn holds(&self, name: &str) -> bool {
        let Ok(name) = named(name) else {
            return false;
        };
        // SAFETY: a stat is plain data, and fstatat fills it.
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        // SAFETY: fstatat relative to a descriptor this owns, of a name
        // this holds, following no link.
        let found = unsafe {
            libc::fstatat(
                self.folder.as_raw_fd(),
                name.as_ptr(),
                &raw mut stat,
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        found == 0 && stat.st_mode & libc::S_IFMT == libc::S_IFREG
    }

    /// Reads at most `largest` bytes of `name`, once: a plain file only,
    /// never a link, a FIFO or a device, refused whole when it's larger.
    ///
    /// # Errors
    ///
    /// When it can't be opened, isn't a plain file or is larger.
    pub fn read(&self, name: &str, largest: u64) -> Result<Vec<u8>, String> {
        let said =
            |why: &dyn std::fmt::Display| format!("{}: {why}", self.shown.join(name).display());
        let c_name = named(name).map_err(|error| said(&error))?;
        // O_NONBLOCK so a FIFO answers at once rather than holding the
        // gate open; a plain file reads the same either way.
        // SAFETY: openat relative to a descriptor this owns.
        let fd = opened(unsafe {
            libc::openat(
                self.folder.as_raw_fd(),
                c_name.as_ptr(),
                libc::O_RDONLY
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK
                    | libc::O_NOCTTY
                    | libc::O_CLOEXEC,
            )
        })
        .map_err(|error| said(&error))?;
        if !plain(&fd).map_err(|error| said(&error))? {
            return Err(said(&"it isn't a plain file"));
        }
        let mut bytes = Vec::new();
        fs::File::from(fd)
            .take(largest + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| said(&error))?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > largest {
            return Err(said(&format!("it's larger than {largest} bytes")));
        }
        Ok(bytes)
    }

    /// Writes `bytes` as `name` whole: to a new hidden file, made here and
    /// following no link, since mycelium carries no hidden file, then
    /// named over whatever had the name.
    ///
    /// # Errors
    ///
    /// When it can't be made, written or named.
    pub fn write_whole(&self, name: &str, bytes: &[u8]) -> Result<(), String> {
        let said = |error: io::Error| format!("{}: {error}", self.shown.join(name).display());
        let (writing, target) = (
            named(&format!(".{name}.writing")).map_err(said)?,
            named(name).map_err(said)?,
        );
        let at = self.folder.as_raw_fd();
        // A stale one, or a link planted in its place, goes first; one that
        // isn't there is what's wanted, and any other error shows at the
        // open below, which makes only a new file.
        // SAFETY: unlinkat relative to a descriptor this owns.
        let _ = unsafe { libc::unlinkat(at, writing.as_ptr(), 0) };
        // SAFETY: openat relative to a descriptor this owns.
        let fd = opened(unsafe {
            libc::openat(
                at,
                writing.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o644,
            )
        })
        .map_err(said)?;
        let mut file = fs::File::from(fd);
        file.write_all(bytes).map_err(said)?;
        file.sync_all().map_err(said)?;
        // SAFETY: renameat within a descriptor this owns.
        if unsafe { libc::renameat(at, writing.as_ptr(), at, target.as_ptr()) } != 0 {
            return Err(said(io::Error::last_os_error()));
        }
        Ok(())
    }
}

/// `path` below `top`, opened as a folder by openat2, following no link
/// and leaving no folder below `top`.
#[cfg(target_os = "linux")]
fn beneath_no_links(top: &OwnedFd, path: &CString) -> io::Result<OwnedFd> {
    // SAFETY: open_how is plain data, and every field is set below.
    let mut how: libc::open_how = unsafe { std::mem::zeroed() };
    how.flags =
        u64::try_from(libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    how.resolve = libc::RESOLVE_BENEATH | libc::RESOLVE_NO_SYMLINKS;
    // SAFETY: openat2 relative to a descriptor this owns, of a path and an
    // open_how this holds, of the size given.
    let fd = unsafe {
        libc::syscall(
            libc::SYS_openat2,
            top.as_raw_fd(),
            path.as_ptr(),
            &raw const how,
            std::mem::size_of::<libc::open_how>(),
        )
    };
    opened(libc::c_int::try_from(fd).unwrap_or(-1))
}

/// Elsewhere no lookup refuses every link on the way, and nothing
/// answers a gate but Linux.
#[cfg(not(target_os = "linux"))]
fn beneath_no_links(_top: &OwnedFd, _path: &CString) -> io::Result<OwnedFd> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "a gate is answered on Linux alone",
    ))
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    const QUEST: &str = "QEST-20261116-ABC123";
    const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

    fn scratch() -> (tempfile::TempDir, PathBuf) {
        let root = tempfile::tempdir().expect("a folder");
        let gate = root.path().join(QUEST).join(GATES).join(COMMIT);
        fs::create_dir_all(&gate).expect("the gate's folder");
        (root, gate)
    }

    #[test]
    fn a_gate_folder_reached_through_a_link_is_refused() {
        let root = tempfile::tempdir().expect("a folder");
        let elsewhere = root.path().join("elsewhere");
        fs::create_dir_all(elsewhere.join(COMMIT)).expect("a folder elsewhere");
        fs::create_dir_all(root.path().join(QUEST).join("handover")).expect("a handover");
        symlink(&elsewhere, root.path().join(QUEST).join(GATES)).expect("a link");
        assert!(
            GateFolder::open(root.path(), QUEST, COMMIT).is_err(),
            "gates as a link should be refused, not followed"
        );
    }

    #[test]
    fn a_request_planted_as_a_link_is_refused() {
        let (root, gate) = scratch();
        let victim = root.path().join("victim");
        fs::write(&victim, "{}").expect("a file of the person's");
        symlink(&victim, gate.join("request.json")).expect("a link");
        let folder = GateFolder::open(root.path(), QUEST, COMMIT).expect("the folder opens");
        assert!(
            folder.read("request.json", 1024).is_err(),
            "a link should be refused rather than read through"
        );
        assert!(!folder.holds("request.json"), "a link isn't a request held");
    }

    #[test]
    fn a_fifo_planted_as_a_bundle_is_refused_at_once() {
        let (root, gate) = scratch();
        let fifo = named(&gate.join("gate.bundle").to_string_lossy()).expect("a name");
        // SAFETY: mkfifo of a path this holds.
        assert_eq!(
            unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) },
            0,
            "a FIFO is made"
        );
        let folder = GateFolder::open(root.path(), QUEST, COMMIT).expect("the folder opens");
        assert!(
            folder.read("gate.bundle", 1024).is_err(),
            "a FIFO should be refused, not waited on"
        );
    }

    #[test]
    fn a_write_planted_as_a_link_leaves_its_target_unchanged() {
        let (root, gate) = scratch();
        let victim = root.path().join("victim");
        fs::write(&victim, "the person's").expect("a file of the person's");
        symlink(&victim, gate.join(".result.json.writing")).expect("a link");
        let folder = GateFolder::open(root.path(), QUEST, COMMIT).expect("the folder opens");
        folder
            .write_whole("result.json", b"{\"exit\":0}")
            .expect("the result is written");
        assert_eq!(
            fs::read_to_string(&victim).expect("the victim reads"),
            "the person's",
            "the link's target should be unchanged"
        );
        assert_eq!(
            fs::read_to_string(gate.join("result.json")).expect("the result reads"),
            "{\"exit\":0}",
            "the result should be written in the gate's folder"
        );
    }

    #[test]
    fn a_plain_file_reads_back_as_written() {
        let (root, _gate) = scratch();
        let folder = GateFolder::open(root.path(), QUEST, COMMIT).expect("the folder opens");
        folder.write_whole("log", b"said").expect("written");
        assert!(folder.holds("log"), "a written file is held");
        assert_eq!(folder.read("log", 4), Ok(b"said".to_vec()), "it reads back");
        assert!(
            folder.read("log", 3).is_err(),
            "past the largest is refused whole"
        );
    }
}
