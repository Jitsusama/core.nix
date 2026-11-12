//! Where the compiled models are kept, so that only the ones in use stay.
//!
//! OpenVINO names each compiled model by a hash of what made it and never
//! removes one, and on the NPU each of Whisper's is about 0.9 GB. A new
//! model, a new OpenVINO or `GenAI`, a new GPU or NPU driver, or a word added
//! to Joel's words (which sizes the decoder's first step) each compiles
//! another, so the old ones would pile up. So each set of those gets a
//! folder of its own, and once a load from it succeeds, everything else in
//! the cache goes.
//!
//! The folder follows the OpenVINO and `GenAI` that compile, not this build.
//! Five programs share this crate, so keyed on its own path, a change to
//! noise cancelling or a call's transcript threw Whisper's models away, and
//! dictation then spent 45 to 112 s compiling them again after six
//! switches in five hours, its first take waiting. What our code asks
//! OpenVINO for is in OpenVINO's own name for each model, so a change to it
//! still compiles anew, beside the old one, which goes with the folder.

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

/// Where the machine's GPU and NPU drivers are, which OpenVINO loads from
/// outside this build, so a new driver doesn't change this build's path.
const DRIVERS: &str = "/run/opengl-driver";

/// The folder in `cache` for models compiled from `parts` by this build's
/// OpenVINO on the drivers the machine has now. A new OpenVINO or driver
/// compiles anew, and the folder compiled for the old one then goes with
/// the rest.
pub fn kept(cache: &Path, parts: &[&[u8]]) -> PathBuf {
    let compiler = compiled_with(option_env!("DICTATION_COMPILED_WITH"));
    let drivers = fs::canonicalize(DRIVERS).unwrap_or_default();
    folder_for(cache, parts, &compiler, &drivers)
}

/// What compiles the models: the OpenVINO and `GenAI` the build names
/// (package.nix), or, built by hand without them, this program itself, so
/// a hand build never reuses a folder it can't vouch for.
fn compiled_with(named: Option<&str>) -> PathBuf {
    match named {
        Some(named) if !named.is_empty() => PathBuf::from(named),
        _ => std::env::current_exe().unwrap_or_default(),
    }
}

fn folder_for(cache: &Path, parts: &[&[u8]], compiler: &Path, drivers: &Path) -> PathBuf {
    let mut all = parts.to_vec();
    all.push(compiler.as_os_str().as_encoded_bytes());
    all.push(drivers.as_os_str().as_encoded_bytes());
    folder(cache, &all)
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
    fn a_new_openvino_or_new_drivers_find_another_folder() {
        let cache = Path::new("/cache");
        let parts: [&[u8]; 2] = [b"model", b"GPU"];
        let (openvino, drivers) = (
            Path::new("/nix/store/a-openvino /nix/store/a-genai"),
            Path::new("/nix/store/a-drivers"),
        );
        let now = folder_for(cache, &parts, openvino, drivers);
        assert_eq!(now, folder_for(cache, &parts, openvino, drivers));
        assert_ne!(
            now,
            folder_for(
                cache,
                &parts,
                Path::new("/nix/store/a-openvino /nix/store/b-genai"),
                drivers
            ),
            "a new GenAI compiled into the old one's folder"
        );
        assert_ne!(
            now,
            folder_for(cache, &parts, openvino, Path::new("/nix/store/b-drivers")),
            "new drivers compiled into the old ones' folder"
        );
    }

    #[test]
    fn the_folder_follows_openvino_not_this_build() {
        let named = "/nix/store/a-openvino /nix/store/a-genai";
        assert_eq!(compiled_with(Some(named)), PathBuf::from(named));
        assert_ne!(
            compiled_with(Some(named)),
            std::env::current_exe().unwrap(),
            "a rebuild of the programs beside dictation would throw its models away"
        );
    }

    #[test]
    fn a_hand_build_keys_on_itself() {
        let this = std::env::current_exe().unwrap();
        assert_eq!(compiled_with(None), this);
        assert_eq!(compiled_with(Some("")), this);
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
