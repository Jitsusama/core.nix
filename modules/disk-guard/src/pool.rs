//! Which filesystem a path is on, as its free space is shared, which is
//! how the daemons tell their filesystems apart. Walks still stop at any
//! change of device, so they never cross into a mount or a subvolume.

use std::io;
use std::path::Path;

/// On Linux a Btrfs pool counts once, whatever subvolumes it holds.
#[cfg(target_os = "linux")]
pub fn pool(path: &Path) -> io::Result<u64> {
    crate::linux::pool(path)
}

/// Elsewhere, for now, the device. On macOS the volumes of one APFS
/// container share its space as subvolumes do, which Phase 6 settles.
#[cfg(not(target_os = "linux"))]
pub fn pool(path: &Path) -> io::Result<u64> {
    use std::os::unix::fs::MetadataExt;
    Ok(std::fs::metadata(path)?.dev())
}
