//! Where the compiled models are kept, so that only the ones in use stay.
//!
//! OpenVINO names each compiled model by a hash of what made it and never
//! removes one, and on the NPU each of Whisper's is about 0.9 GB. A new
//! model, a new build of dictation or OpenVINO, or a word added to Joel's
//! words (which sizes the decoder's first step) each compiles another, so
//! the old ones would pile up. So each set of those gets a folder of its
//! own, and once a load from it succeeds, everything else in the cache goes.

use std::fs;
use std::path::{Path, PathBuf};

/// The folder in `cache` for models compiled from `parts`, named by a hash
/// that is the same in every build, so a rebuild with the same parts finds it.
pub fn folder(cache: &Path, parts: &[&[u8]]) -> PathBuf {
    // FNV-1a, with a zero byte after each part so no two lists of parts
    // run together into the same bytes.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for &byte in part.iter().chain(&[0]) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    cache.join(format!("{hash:016x}"))
}

/// Removes everything in `cache` but `kept`, saying what went. A failure is
/// only said, since what's left costs disk, not a take.
pub fn forget_others(cache: &Path, kept: &Path) {
    let Ok(entries) = fs::read_dir(cache) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path == kept {
            continue;
        }
        let removed = if path.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        match removed {
            Ok(()) => eprintln!(
                "dictation: removed {}, compiled for another",
                path.display()
            ),
            Err(error) => eprintln!("dictation: couldn't remove {}: {error}", path.display()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("dictation-cache-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_same_parts_find_the_same_folder() {
        let cache = Path::new("/cache");
        assert_eq!(
            folder(cache, &[b"model", b"NPU", b"Tectonix"]),
            folder(cache, &[b"model", b"NPU", b"Tectonix"]),
        );
    }

    #[test]
    fn a_new_word_or_a_moved_boundary_finds_another_folder() {
        let cache = Path::new("/cache");
        let words = folder(cache, &[b"model", b"NPU", b"Tectonix"]);
        assert_ne!(
            words,
            folder(cache, &[b"model", b"NPU", b"Tectonix Sarnia"])
        );
        assert_ne!(folder(cache, &[b"ab", b"c"]), folder(cache, &[b"a", b"bc"]),);
    }

    #[test]
    fn everything_but_the_kept_folder_goes() {
        let cache = scratch("forget");
        let kept = cache.join("kept");
        fs::create_dir_all(kept.join("genai")).unwrap();
        fs::write(kept.join("genai/model.blob"), b"kept").unwrap();
        fs::create_dir_all(cache.join("old/genai")).unwrap();
        fs::write(cache.join("old/genai/model.blob"), b"old").unwrap();
        fs::write(cache.join("loose.blob"), b"old").unwrap();

        forget_others(&cache, &kept);

        let left: Vec<_> = fs::read_dir(&cache)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .collect();
        assert_eq!(left, vec![kept.clone()]);
        assert_eq!(fs::read(kept.join("genai/model.blob")).unwrap(), b"kept");
        fs::remove_dir_all(&cache).unwrap();
    }

    #[test]
    fn a_missing_cache_is_left_alone() {
        let cache = scratch("missing").join("not-there");
        forget_others(&cache, &cache.join("kept"));
        assert!(!cache.exists());
    }
}
