//! Where jiti, the loader pi transpiles a package's TypeScript with,
//! keeps what it transpiled, and what it names each entry.
//!
//! jiti names an entry by the source's folder, its stem and a hash of
//! its whole path, and keeps it in `<tmp>/jiti` of the pi that wrote it:
//!
//!   <folder>-<stem>[+map][.i].<md5 of the path, 8 hex>.(mjs|cjs)[x]
//!
//! An entry is read again only while its last line carries the source's
//! hash, so a stale one costs a miss, never wrong code. Each version
//! here sits at a path of its own, so every swap leaves every entry of
//! the version before unread: on 12 November the first start of a pi
//! after a swap spent 13.2 s transpiling in jiti, and the next 0.7 s,
//! and 1.6 GB of entries no pi would read again had built up. The
//! warm (warm.rs) writes a version's entries before the swap, and a
//! version's entries go with it (version.rs). Entries no version kept
//! here wrote, from worktrees and scratch pis, go once a week old
//! (`age_out`, run from each update).
//!
//! This follows jiti 2.7, which pi 1.5 ships. Under FIPS jiti hashes
//! with SHA-256 rather than MD5, which these names don't follow: there
//! the entries are left for the system to clear, as before.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The folder jiti keeps its entries in, under a pi's temporary folder.
#[must_use]
pub fn cache_of(tmp: &Path) -> PathBuf {
    tmp.join("jiti")
}

/// Every name jiti may give the entry for the source at `file`, whatever
/// options it ran with: with a source map or without, with interop or
/// without, as `.mjs` or `.cjs`, and JSX's `x` where it applies.
#[must_use]
pub fn entry_names(file: &Path) -> Vec<String> {
    let Some(name) = file.file_name().and_then(|name| name.to_str()) else {
        return Vec::new();
    };
    let folder = file
        .parent()
        .and_then(Path::file_name)
        .and_then(|folder| folder.to_str())
        .unwrap_or("");
    // The stem stops at the last dot, unless that dot starts the name.
    let stem = match name.rfind('.') {
        Some(dot) if dot > 0 => &name[..dot],
        _ => name,
    };
    let hash = path_hash(file);
    let mut names = Vec::new();
    for map in ["", "+map"] {
        for interop in ["", ".i"] {
            for kind in [".mjs", ".cjs"] {
                let entry = format!("{folder}-{stem}{map}{interop}.{hash}{kind}");
                if name.ends_with('x') {
                    names.push(format!("{entry}x"));
                }
                names.push(entry);
            }
        }
    }
    names
}

/// The part of an entry's name jiti takes from its source's path: the
/// first 8 hex of its MD5.
fn path_hash(file: &Path) -> String {
    hex(&md5(file.as_os_str().as_encoded_bytes()))[..8].to_owned()
}

/// The path's hash in an entry's name, the part before its last dot.
fn hash_in(name: &str) -> Option<&str> {
    name.rsplit('.').nth(1)
}

/// Remove from each cache in `caches` every entry jiti may have written
/// for a file under one of `versions`, which must still be there to be
/// listed. Says how many went. A cache that isn't there, or an entry
/// that can't be removed, is passed over: an entry left is only disk.
#[must_use]
pub fn forget(versions: &[PathBuf], caches: &[PathBuf]) -> usize {
    let names = names_of(versions);
    if names.is_empty() {
        return 0;
    }
    let mut removed = 0;
    for cache in caches {
        let Ok(entries) = fs::read_dir(cache) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_str().is_some_and(|name| names.contains(name))
                && fs::remove_file(entry.path()).is_ok()
            {
                removed += 1;
            }
        }
    }
    removed
}

/// What an age-out removed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Aged {
    pub files: usize,
    pub bytes: u64,
}

/// Remove from each cache in `caches` every entry last written before
/// `before`, except those jiti may have written for a file under one of
/// `kept`, known by the path's hash in its name. Says how many went and
/// their size.
///
/// An entry's own path is in nobody's hands here: worktrees, scratch pis
/// and versions long gone each left theirs, about 400 copies of every
/// file on 12 November. Read times aren't kept where /tmp is noatime, so
/// a write's age is the one sign left: an entry is written again when
/// its source changes, and a pi that read it once has it already. A tree
/// idle that long rebuilds its entries at its next start. What a kept
/// version wrote stays whatever its age, so a pi installed here never
/// starts cold. Its hashes, one a file, are what is held, never every
/// name: on 12 November 57 versions were kept, some 190,000 files. As
/// with `forget`, an entry that can't be read or removed is passed over.
#[must_use]
pub fn age_out(kept: &[PathBuf], caches: &[PathBuf], before: SystemTime) -> Aged {
    let keep = hashes_of(kept);
    let mut aged = Aged::default();
    for cache in caches {
        let Ok(entries) = fs::read_dir(cache) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(about) = entry.metadata() else {
                continue;
            };
            let old = about.is_file() && about.modified().is_ok_and(|written| written < before);
            let kept = entry
                .file_name()
                .to_str()
                .and_then(hash_in)
                .is_some_and(|hash| keep.contains(hash));
            if old && !kept && fs::remove_file(entry.path()).is_ok() {
                aged.files += 1;
                aged.bytes += about.len();
            }
        }
    }
    aged
}

/// Every name jiti may give an entry for a file under one of `versions`.
fn names_of(versions: &[PathBuf]) -> HashSet<String> {
    files_of(versions)
        .iter()
        .flat_map(|file| entry_names(file))
        .collect()
}

/// The path's hash of every file under one of `versions`.
fn hashes_of(versions: &[PathBuf]) -> HashSet<String> {
    files_of(versions)
        .iter()
        .map(|file| path_hash(file))
        .collect()
}

/// Every file under one of `versions`, by the real path pi hands jiti,
/// links resolved.
fn files_of(versions: &[PathBuf]) -> Vec<PathBuf> {
    versions
        .iter()
        .flat_map(|version| sources(&fs::canonicalize(version).unwrap_or_else(|_| version.clone())))
        .collect()
}

/// Every file under `dir`, links not followed: a dependency's link leads
/// into pnpm's store, shared with the versions that stay.
fn sources(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        let Ok(entries) = fs::read_dir(&next) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                stack.push(entry.path());
            } else if kind.is_file() {
                found.push(entry.path());
            }
        }
    }
    found
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// MD5 (RFC 1321), which jiti names entries by. Written here rather than
/// taken from a crate: it hashes paths, never anything secret, and a
/// crate would bring five more into the installer for 40 lines.
#[must_use]
#[expect(
    clippy::many_single_char_names,
    reason = "the RFC names the registers a, b, c and d, and the round's parts f and g"
)]
pub fn md5(input: &[u8]) -> [u8; 16] {
    const SHIFTS: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    // floor(abs(sin(i + 1)) * 2^32), as the RFC tabulates them.
    let sines: [u32; 64] = std::array::from_fn(|i| {
        #[expect(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "i is below 64 and the sine's product below 2^32, so nothing is lost"
        )]
        let k = ((i as f64 + 1.0).sin().abs() * 4_294_967_296.0) as u32;
        k
    });
    let mut message = input.to_vec();
    let bits = (input.len() as u64).wrapping_mul(8);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bits.to_le_bytes());
    let mut state: [u32; 4] = [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476];
    for block in message.as_chunks::<64>().0 {
        let words: [u32; 16] = std::array::from_fn(|i| {
            u32::from_le_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ])
        });
        let [mut a, mut b, mut c, mut d] = state;
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let rotated = a
                .wrapping_add(f)
                .wrapping_add(sines[i])
                .wrapping_add(words[g])
                .rotate_left(SHIFTS[i]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(rotated);
        }
        state = [
            state[0].wrapping_add(a),
            state[1].wrapping_add(b),
            state[2].wrapping_add(c),
            state[3].wrapping_add(d),
        ];
    }
    let mut digest = [0; 16];
    for (i, word) in state.iter().enumerate() {
        digest[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    digest
}
