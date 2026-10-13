use disk_guard::pool::pool;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

const BTRFS_SUPER_MAGIC: u64 = 0x9123_683e;
/// `BTRFS_IOC_SUBVOL_CREATE`, `_IOW(0x94, 14, struct btrfs_ioctl_vol_args)`,
/// which is 4096 bytes: a descriptor, then the name.
const BTRFS_IOC_SUBVOL_CREATE: libc::c_ulong = 0x5000_940e;

/// Whether `path` is on Btrfs, where subvolumes can be made.
pub(crate) fn on_btrfs(path: &Path) -> bool {
    rustix::fs::statfs(path).unwrap().f_type as u64 == BTRFS_SUPER_MAGIC
}

/// Makes the subvolume `name` in `dir`, which any account owning `dir` may.
pub(crate) fn subvolume(dir: &Path, name: &str) {
    let file = fs::File::open(dir).unwrap();
    let mut args = [0u8; 4096];
    args[8..8 + name.len()].copy_from_slice(name.as_bytes());
    use std::os::fd::AsRawFd;
    // SAFETY: args is the size the kernel reads, and outlives the call.
    let made = unsafe {
        libc::ioctl(
            file.as_raw_fd(),
            BTRFS_IOC_SUBVOL_CREATE as _,
            args.as_ptr(),
        )
    };
    assert_eq!(made, 0, "{}", std::io::Error::last_os_error());
}

#[test]
fn a_filesystem_of_one_volume_is_known_by_its_device() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("inside")).unwrap();
    assert_eq!(
        pool(dir.path()).unwrap(),
        pool(&dir.path().join("inside")).unwrap()
    );
    if !on_btrfs(dir.path()) {
        assert_eq!(
            pool(dir.path()).unwrap(),
            fs::metadata(dir.path()).unwrap().dev()
        );
    }
    assert_ne!(pool(Path::new("/proc")).unwrap(), pool(dir.path()).unwrap());
    assert!(pool(&dir.path().join("gone")).is_err());
}

/// Btrfs gives each subvolume a device number of its own, though all of
/// them share the pool's free space, so the pool is what's known.
#[test]
fn the_subvolumes_of_one_btrfs_pool_are_one_filesystem() {
    let dir = tempfile::tempdir().unwrap();
    if !on_btrfs(dir.path()) {
        eprintln!("skipped: {} isn't on Btrfs", dir.path().display());
        return;
    }
    subvolume(dir.path(), "sub");
    let sub = dir.path().join("sub");
    let device = |path: &Path| fs::metadata(path).unwrap().dev();
    assert_ne!(device(dir.path()), device(&sub), "a subvolume of its own");
    assert_eq!(pool(dir.path()).unwrap(), pool(&sub).unwrap());
    assert_ne!(
        pool(&sub).unwrap(),
        device(&sub),
        "no device number is a pool's"
    );
}
