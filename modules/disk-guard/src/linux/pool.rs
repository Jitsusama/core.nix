//! How Linux reads which Btrfs pool and subvolume a file lies on: by
//! statx where the kernel says a subvolume's id, through Btrfs's own
//! ioctl where it is older, and always through a handle held from the
//! first look, so a name taken meanwhile is never read twice.

use std::fmt::Write as _;
use std::io;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

const BTRFS_SUPER_MAGIC: u64 = 0x9123_683e;

/// `BTRFS_IOC_FS_INFO`, `_IOR(0x94, 31, struct btrfs_ioctl_fs_info_args)`:
/// the filesystem's id, which names its sysfs directory. Any account may ask.
const BTRFS_IOC_FS_INFO: libc::c_ulong = 0x8400_941f;

/// Which filesystem `path` is on, as its free space is shared: on Btrfs
/// the pool, whose subvolumes each have a device number of their own,
/// and elsewhere the device. A pool's number has its top bit set, which
/// no device number has, so the two never meet. A pipe, a socket or a
/// device holds none of its disk's bytes, so its pool is its own device,
/// read without opening it: a pipe's open would wait for a writer, and a
/// device's driver would be asked what only Btrfs answers. On Btrfs that
/// device is a subvolume's, which is no watched disk's; elsewhere it is
/// its disk's, as for any file there. On Linux 6.10 or later a Btrfs
/// file or folder's pool is read from its filesystem's id and its
/// subvolume's, which Btrfs mixes in, without opening it to read, so no
/// lease or mode can hide it; before 6.10 Btrfs is asked through a
/// reading open, which a lease or a mode can still refuse. Read past any
/// link to it, such as a process's open file, as the kernel follows that
/// link, rather than by the name the link says, which may be gone or name
/// another mount namespace's file.
pub fn pool(path: &Path) -> io::Result<u64> {
    use rustix::fs::OFlags;
    // Found without opening it, which waits for nothing, and read only
    // through what was found, so nothing swapped in between is read: a
    // reading open would break a writer's lease or meet a mode that
    // refuses the guard, and either would leave a writer's file on no
    // disk, out of every freeze.
    let none = rustix::fs::Mode::empty();
    let held = rustix::fs::open(path, OFlags::PATH | OFlags::CLOEXEC, none)?;
    pool_of_held(&held, path, true)
}

/// What `pool` reads of `held`, a handle to `path` opened without reading,
/// read through it alone. Asked whether the kernel names a file's
/// subvolume, and told not to, it reads as a kernel before 6.10 would
/// make it, so a case can drive that branch on any kernel.
fn pool_of_held(held: &rustix::fd::OwnedFd, path: &Path, ask_subvolume: bool) -> io::Result<u64> {
    use rustix::fs::FileType;
    let found = rustix::fs::fstat(held)?;
    let device = found.st_dev;
    let kind = FileType::from_raw_mode(found.st_mode);
    if !matches!(kind, FileType::Directory | FileType::RegularFile) {
        return Ok(device);
    }
    if !is_btrfs(&rustix::fs::fstatfs(held)?) {
        return Ok(device);
    }
    let subvolume = rustix::fs::StatxFlags::from_bits_retain(STATX_SUBVOL);
    let read = rustix::fs::statx(held, "", rustix::fs::AtFlags::EMPTY_PATH, subvolume)?;
    if !ask_subvolume || read.stx_mask & STATX_SUBVOL == 0 {
        // A kernel before 6.10 doesn't say a file's subvolume, so Btrfs
        // itself is asked, through a reading open.
        return btrfs_id_of(held, path).map(|id| pool_of(fsid_halves(id)));
    }
    let fsid = rustix::fs::fstatvfs(held)?.f_fsid;
    Ok(pool_of(statfs_halves(fsid, read.stx_subvol)))
}

/// `STATX_SUBVOL`: asks statx for the Btrfs subvolume a file is in, since
/// Linux 6.10.
const STATX_SUBVOL: u32 = 0x8000;

/// A Btrfs filesystem's id folded to two halves, as its `statfs` folds it
/// before mixing in a subvolume: each big-endian word mixed by exclusive
/// or with the one two words on.
fn fsid_halves(id: [u8; 16]) -> [u32; 2] {
    let word = |at: usize| u32::from_be_bytes(id[at..at + 4].try_into().unwrap());
    [word(0) ^ word(8), word(4) ^ word(12)]
}

/// The halves `fsid_halves` gives, from a file's `statfs` fsid, packed low
/// half first, and the id of the subvolume it's in, which Btrfs mixes into
/// them so each subvolume reads apart.
fn statfs_halves(fsid: u64, subvolume: u64) -> [u32; 2] {
    let half = |bits: u64| u32::try_from(bits & u64::from(u32::MAX)).unwrap();
    [
        half(fsid) ^ half(subvolume >> 32),
        half(fsid >> 32) ^ half(subvolume),
    ]
}

/// A Btrfs pool's number from its folded id, with the top bit set. It is
/// the number the guard gave a pool when it read the id's two halves as
/// little-endian words, each byte-swapped back from the big-endian fold,
/// so a survey kept from before still names its disks.
fn pool_of([low, high]: [u32; 2]) -> u64 {
    (u64::from(low.swap_bytes()) | u64::from(high.swap_bytes()) << 32) | 1 << 63
}

/// Whether `path`, a directory beneath `parent` with a device number of
/// its own, is a Btrfs subvolume of the same pool, reached through the
/// same mount and writable, so a walk may go on into it as into any
/// directory. A mount is another filesystem, or this one seen again, and
/// a read-only snapshot can't be emptied; neither is entered. A kernel
/// that doesn't say which mount a path is on enters nothing.
pub fn subvolume_within(parent: &Path, path: &Path) -> bool {
    let btrfs = |dir: &Path| rustix::fs::statfs(dir).is_ok_and(|stats| is_btrfs(&stats));
    let mount = |dir: &Path| {
        let flags = rustix::fs::AtFlags::SYMLINK_NOFOLLOW;
        let wanted = rustix::fs::StatxFlags::MNT_ID;
        rustix::fs::statx(rustix::fs::CWD, dir, flags, wanted)
            .ok()
            .filter(|stat| stat.stx_mask & wanted.bits() != 0)
            .map(|stat| stat.stx_mnt_id)
    };
    btrfs(parent)
        && btrfs(path)
        && btrfs_id(parent).is_ok_and(|id| btrfs_id(path).is_ok_and(|other| other == id))
        && mount(parent).is_some_and(|id| mount(path) == Some(id))
        && rustix::fs::access(path, rustix::fs::Access::WRITE_OK).is_ok()
}

/// Whether `child`, a directory opened beneath `parent` with a device
/// number of its own, is a writable Btrfs subvolume reached through the
/// same mount: what `subvolume_within` asks of two paths, asked of open
/// directories, so nothing can be swapped in between. One mount is one
/// filesystem, so the pool needn't be asked. A kernel that doesn't say
/// which mount a directory is on lets nothing through.
pub fn subvolume_beneath(parent: &std::os::fd::OwnedFd, child: &std::os::fd::OwnedFd) -> bool {
    let btrfs = |fd| rustix::fs::fstatfs(fd).is_ok_and(|stats| is_btrfs(&stats));
    let mount = |fd| {
        let wanted = rustix::fs::StatxFlags::MNT_ID;
        rustix::fs::statx(fd, "", rustix::fs::AtFlags::EMPTY_PATH, wanted)
            .ok()
            .filter(|stat| stat.stx_mask & wanted.bits() != 0)
            .map(|stat| stat.stx_mnt_id)
    };
    let writable = rustix::fs::accessat(
        child,
        ".",
        rustix::fs::Access::WRITE_OK,
        rustix::fs::AtFlags::empty(),
    );
    btrfs(parent)
        && btrfs(child)
        && mount(parent).is_some_and(|id| mount(child) == Some(id))
        && writable.is_ok()
}

/// Whether a filesystem's statistics say it's Btrfs. The kernel's type is
/// signed, so its bits are read as the unsigned magic number they are.
pub fn is_btrfs(stats: &rustix::fs::StatFs) -> bool {
    stats.f_type.cast_unsigned() == BTRFS_SUPER_MAGIC
}

/// A Btrfs filesystem's id, as its sysfs directory is named.
pub(super) fn btrfs_fsid(path: &Path) -> io::Result<PathBuf> {
    let hex = btrfs_id(path)?.iter().fold(String::new(), |mut hex, byte| {
        // Writing to a String can't fail.
        _ = write!(hex, "{byte:02x}");
        hex
    });
    let fsid = format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    );
    Ok(PathBuf::from(fsid))
}

/// A Btrfs filesystem's 16-byte id, the same from every subvolume.
fn btrfs_id(path: &Path) -> io::Result<[u8; 16]> {
    use rustix::fs::OFlags;
    // Found without opening it for reading; btrfs_id_of opens it only through
    // this handle.
    let found = rustix::fs::open(
        path,
        OFlags::PATH | OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )?;
    btrfs_id_of(&found, path)
}

/// The 16-byte id of the Btrfs filesystem holding `found`, a handle
/// opened without reading, named `path` in what it says. Read through
/// that handle, so the name is never looked up again: a process's open
/// file may be closed and its number reused between two lookups.
fn btrfs_id_of(found: &rustix::fd::OwnedFd, path: &Path) -> io::Result<[u8; 16]> {
    use rustix::fs::OFlags;
    let none = rustix::fs::Mode::empty();
    let kind = rustix::fs::FileType::from_raw_mode(rustix::fs::fstat(found)?.st_mode);
    if !matches!(
        kind,
        rustix::fs::FileType::Directory | rustix::fs::FileType::RegularFile
    ) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is neither a file nor a folder", path.display()),
        ));
    }
    // Opened only as a file or a folder, through the handle's own link, so
    // nothing swapped in under the name is opened: a pipe's open would wait
    // for a writer. A file a writer holds a lease on refuses the open at
    // once, rather than holding the guard while the lease breaks, though its
    // holder is still asked to break it.
    let again = format!("/proc/self/fd/{}", found.as_raw_fd());
    let flags = OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK;
    let fd = rustix::fs::open(again, flags, none)?;
    // struct btrfs_ioctl_fs_info_args is 1024 bytes: max_id and
    // num_devices, then the 16-byte fsid.
    let args = btrfs_fs_info(&fd, 0)?;
    Ok(args[16..32].try_into().unwrap())
}

/// `BTRFS_FS_INFO_FLAG_GENERATION`: asks `BTRFS_IOC_FS_INFO` for the
/// filesystem's generation too.
const BTRFS_FS_INFO_FLAG_GENERATION: u64 = 1 << 1;

/// A Btrfs filesystem's generation, the id of its newest transaction.
pub(super) fn btrfs_generation(fd: &rustix::fd::OwnedFd) -> io::Result<u64> {
    // The flags asked for and answered are the u64 at 48, after the fsid
    // and the sizes, and the generation the u64 after them.
    let args = btrfs_fs_info(fd, BTRFS_FS_INFO_FLAG_GENERATION)?;
    let answered = u64::from_ne_bytes(args[48..56].try_into().unwrap());
    if answered & BTRFS_FS_INFO_FLAG_GENERATION == 0 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "the kernel doesn't say a Btrfs filesystem's generation",
        ));
    }
    Ok(u64::from_ne_bytes(args[56..64].try_into().unwrap()))
}

/// `struct btrfs_ioctl_fs_info_args`, all 1024 bytes of it, asked for with
/// `flags`.
fn btrfs_fs_info(fd: &rustix::fd::OwnedFd, flags: u64) -> io::Result<[u8; 1024]> {
    let mut args = [0u8; 1024];
    args[48..56].copy_from_slice(&flags.to_ne_bytes());
    // SAFETY: args is as large as the kernel writes, and outlives the call.
    if unsafe { libc::ioctl(fd.as_raw_fd(), BTRFS_IOC_FS_INFO as _, args.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::{fsid_halves, pool_of, statfs_halves};

    /// Two Btrfs ids with their bytes apart, so a word read in the wrong
    /// order or from the wrong place gives another number.
    const IDS: [[u8; 16]; 2] = [
        [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x10, 0x32, 0x54, 0x76, 0x98, 0xba,
            0xdc, 0xfe,
        ],
        [
            0xde, 0xad, 0xbe, 0xef, 0, 1, 2, 3, 4, 5, 6, 7, 0xff, 0xee, 0xdd, 0xcc,
        ],
    ];

    /// Subvolume ids, the top-level one, two made later and one whose
    /// high half is set, which only the first half takes.
    const SUBVOLUMES: [u64; 4] = [5, 256, 13_823, 0x1_0000_0102];

    /// The `f_fsid` Btrfs's statfs gives a file in `subvolume`, packed
    /// low half first, as fs/btrfs/super.c builds it: each big-endian word
    /// of the id exclusive-ored with the one two on, then the subvolume's
    /// high half mixed into the first and its low half into the second.
    fn statfs_gives(id: [u8; 16], subvolume: u64) -> u64 {
        let word = |at: usize| u32::from_be_bytes(id[at..at + 4].try_into().unwrap());
        let half = |bits: u64| u32::try_from(bits & u64::from(u32::MAX)).unwrap();
        let low = word(0) ^ word(8) ^ half(subvolume >> 32);
        let high = word(4) ^ word(12) ^ half(subvolume);
        u64::from(low) | u64::from(high) << 32
    }

    #[test]
    fn every_subvolume_of_a_pool_reads_as_the_pool_the_ioctl_reads() {
        for id in IDS {
            let ioctl = pool_of(fsid_halves(id));
            for subvolume in SUBVOLUMES {
                let statx = pool_of(statfs_halves(statfs_gives(id, subvolume), subvolume));
                assert_eq!(statx, ioctl, "subvolume {subvolume} of {id:x?}");
            }
        }
    }

    #[test]
    fn a_pool_keeps_the_number_the_guard_gave_it_before() {
        for id in IDS {
            let halves =
                [&id[..8], &id[8..]].map(|half| u64::from_le_bytes(half.try_into().unwrap()));
            let before = (halves[0] ^ halves[1]) | 1 << 63;
            assert_eq!(pool_of(fsid_halves(id)), before, "the pool of {id:x?}");
        }
    }

    #[test]
    #[ignore = "needs TMPDIR on Btrfs, which the sandbox and the shell's tmpfs aren't, \
                and Linux 6.10 or later, which says a file's subvolume: \
                run with --ignored and TMPDIR on a Btrfs folder"]
    fn a_folder_s_pool_read_by_statx_is_the_one_btrfs_s_ioctl_names() {
        let dir = tempfile::tempdir().unwrap();
        assert_on_btrfs(dir.path());
        // Worked out here from statx's own read, as pool works it out on
        // 6.10 and later, since pool's fallback to the ioctl would agree
        // with the ioctl whatever statx said.
        let subvolume = rustix::fs::StatxFlags::from_bits_retain(super::STATX_SUBVOL);
        let read = rustix::fs::statx(
            rustix::fs::CWD,
            dir.path(),
            rustix::fs::AtFlags::empty(),
            subvolume,
        )
        .unwrap();
        assert_ne!(
            read.stx_mask & super::STATX_SUBVOL,
            0,
            "this kernel doesn't say a file's subvolume"
        );
        let fsid = rustix::fs::statvfs(dir.path()).unwrap().f_fsid;
        let by_statx = pool_of(super::statfs_halves(fsid, read.stx_subvol));
        let ioctl = super::btrfs_id(dir.path()).map(|id| pool_of(fsid_halves(id)));
        assert_eq!(
            Some(by_statx),
            ioctl.ok(),
            "{} by statx and by the ioctl",
            dir.path().display()
        );
        assert_eq!(
            super::pool(dir.path()).ok(),
            Some(by_statx),
            "{} as pool reads it",
            dir.path().display()
        );
    }

    #[test]
    #[ignore = "needs TMPDIR on Btrfs, which the sandbox and the shell's tmpfs aren't: \
                run with --ignored and TMPDIR on a Btrfs folder"]
    fn a_pool_is_read_through_its_handle_after_its_name_is_taken() {
        use rustix::fs::{CWD, FileType, Mode, OFlags};
        let dir = tempfile::tempdir().unwrap();
        assert_on_btrfs(dir.path());
        let expected = pool_of(fsid_halves(super::btrfs_id(dir.path()).unwrap()));
        let name = dir.path().join("file");
        std::fs::write(&name, "").unwrap();
        let found = rustix::fs::open(&name, OFlags::PATH | OFlags::CLOEXEC, Mode::empty()).unwrap();
        // A pipe in its place, whose open asked by name would read as no
        // Btrfs pool, or wait for a writer without NONBLOCK. Any other file
        // on the pool would read as the same pool, so only a name that
        // reads as none tells a read by name from one through the handle.
        let pipe = dir.path().join("pipe");
        rustix::fs::mknodat(CWD, &pipe, FileType::Fifo, Mode::from_raw_mode(0o600), 0).unwrap();
        std::fs::rename(&pipe, &name).unwrap();

        // Both of pool's branches after its open, the one a kernel before
        // 6.10 takes too, each within five seconds, since an open of the
        // pipe without NONBLOCK would never end.
        for ask_subvolume in [false, true] {
            let read = within_five_seconds({
                let found = found.try_clone().unwrap();
                let name = name.clone();
                move || super::pool_of_held(&found, &name, ask_subvolume)
            });
            assert_eq!(
                read.as_ref().map(|read| read.as_ref().ok()),
                Some(Some(&expected)),
                "{} read through its handle, asking its subvolume: {ask_subvolume}: {read:?}",
                name.display()
            );
        }
    }

    /// A file its writer holds a lease on refuses the reopen a kernel
    /// before 6.10 needs, at once rather than once the lease is broken, so
    /// the guard never waits on a writer. The refusal comes before Btrfs is
    /// asked, so it shows on any filesystem.
    #[test]
    fn a_leased_file_refuses_the_reopen_at_once() {
        use rustix::fs::{Mode, OFlags};
        use std::os::fd::AsRawFd;
        let dir = tempfile::tempdir().unwrap();
        let name = dir.path().join("leased");
        let held = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&name)
            .unwrap();
        // SAFETY: plain system calls; a broken lease signals SIGIO, whose
        // default would end the test, so it's ignored.
        let leased = unsafe {
            libc::signal(libc::SIGIO, libc::SIG_IGN);
            libc::fcntl(held.as_raw_fd(), libc::F_SETLEASE, libc::F_WRLCK)
        };
        assert_eq!(leased, 0, "no lease: {}", std::io::Error::last_os_error());
        let found = rustix::fs::open(&name, OFlags::PATH | OFlags::CLOEXEC, Mode::empty()).unwrap();

        let read = within_five_seconds(move || super::btrfs_id_of(&found, &name));

        assert_eq!(
            read.as_ref()
                .map(|read| read.as_ref().map_err(std::io::Error::kind)),
            Some(Err(std::io::ErrorKind::WouldBlock)),
            "a leased file's reopen, refused within five seconds: {read:?}"
        );
        drop(held);
    }

    /// What `read` gives, on a thread of its own, or nothing if it takes
    /// past five seconds, so a read that waits fails its case rather than
    /// hanging the run.
    fn within_five_seconds<T: Send + 'static>(
        read: impl FnOnce() -> T + Send + 'static,
    ) -> Option<T> {
        let (sent, heard) = std::sync::mpsc::channel();
        std::thread::spawn(move || sent.send(read()));
        heard.recv_timeout(std::time::Duration::from_secs(5)).ok()
    }

    /// Fails, saying so, unless `dir` is on Btrfs, so a case run with
    /// TMPDIR elsewhere doesn't blame the kernel.
    fn assert_on_btrfs(dir: &std::path::Path) {
        let stats = rustix::fs::statfs(dir).unwrap();
        assert!(
            super::is_btrfs(&stats),
            "{} isn't on Btrfs; set TMPDIR to a Btrfs folder",
            dir.display()
        );
    }
}
