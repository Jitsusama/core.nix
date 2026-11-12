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
//! version's entries go with it (version.rs).
//!
//! This follows jiti 2.7, which pi 1.5 ships. Under FIPS jiti hashes
//! with SHA-256 rather than MD5, which these names don't follow: there
//! the entries are left for the system to clear, as before.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The folder jiti keeps its entries in, under a pi's temporary folder.
#[must_use]
pub fn cache_of(tmp: &Path) -> PathBuf {
    tmp.join("jiti")
}

/// Every name jiti may give the entry for the source at `file`, whatever
/// options it ran with: with a source map or without, with interop or
/// without, as an ES module or CommonJS, and JSX's `x` where it applies.
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
    let hash = &hex(&md5(file.as_os_str().as_encoded_bytes()))[..8];
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

/// Remove from each cache in `caches` every entry jiti may have written
/// for a file under one of `versions`, which must still be there to be
/// listed. Says how many went. A cache that isn't there, or an entry
/// that can't be removed, is passed over: an entry left is only disk.
#[must_use]
pub fn forget(versions: &[PathBuf], caches: &[PathBuf]) -> usize {
    let mut names = HashSet::new();
    for version in versions {
        // pi hands jiti each file by its real path, links resolved.
        let version = fs::canonicalize(version).unwrap_or_else(|_| version.clone());
        for file in sources(&version) {
            names.extend(entry_names(&file));
        }
    }
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
pub fn md5(input: &[u8]) -> [u8; 16] {
    const SHIFTS: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    // floor(abs(sin(i + 1)) * 2^32), as the RFC tabulates them.
    let sines: [u32; 64] = std::array::from_fn(|i| {
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
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
