//! What sits in the Nix store unregistered: the output of a build killed
//! before Nix registered it, which `nix path-info` knows nothing of and
//! nothing registered can refer to. On 11 November, 616 such entries held
//! 67.5 GiB of a laptop's disk, mostly disk images, and the listing of
//! garbage never offered one, since it took a path Nix didn't know for one
//! already gone. `nix store delete` takes them as it takes the dead.

use crate::garbage::Listed;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// How many files under an entry are weighed. A disk image is one file; a
/// half-unpacked source tree may be thousands.
const FILES: usize = 100_000;

/// How Nix's own working files in the store end: the lock a build or a
/// fetch takes on its output, which the listing names among the dead
/// (5,196 of them on 11 November) and which a builder may hold that very
/// moment, and the folders a sandboxed build or a repeat of one works in.
/// They weigh nothing worth a risk.
const WORKING: [&str; 3] = [".lock", ".chroot", ".check"];

/// The unregistered entry at `path` as garbage: dated by when it was last
/// written, since it has no registration, and weighed by the blocks under
/// it, since an image's length is far more than it holds. None once it's
/// gone, and for Nix's own working files beside the paths it builds.
pub fn listed(path: &str) -> Option<Listed> {
    if WORKING.iter().any(|ending| path.ends_with(ending)) {
        return None;
    }
    let meta = Path::new(path).symlink_metadata().ok()?;
    // ctime, not mtime: a path copied in from elsewhere is stamped with
    // the epoch, while its ctime is when it landed here.
    #[expect(
        clippy::cast_precision_loss,
        reason = "seconds since 1970 fit an f64's 52 bits"
    )]
    let registered =
        meta.ctime() as f64 + f64::from(u32::try_from(meta.ctime_nsec()).unwrap_or(0)) / 1e9;
    Some(Listed {
        path: path.to_owned(),
        registered,
        size: blocks(Path::new(path), meta.dev(), &mut 0),
        refers: Vec::new(),
    })
}

/// The bytes the blocks under `path` take, on `device` alone, counting
/// `seen` files against the walk's budget.
fn blocks(path: &Path, device: u64, seen: &mut usize) -> u64 {
    *seen += 1;
    let Ok(meta) = path.symlink_metadata() else {
        return 0;
    };
    if *seen > FILES || meta.dev() != device {
        return 0;
    }
    let under = if meta.is_dir() {
        std::fs::read_dir(path).map_or(0, |entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| blocks(&entry.path(), device, seen))
                .sum()
        })
    } else {
        0
    };
    // st_blocks counts 512-byte units whatever the filesystem's own.
    meta.blocks() * 512 + under
}
