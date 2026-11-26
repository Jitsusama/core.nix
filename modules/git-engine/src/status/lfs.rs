//! What git-lfs writes and runs as, so `convert` can tell a file git-lfs
//! would clean from the pointer the index holds for it: the command a
//! filter driver runs it by, and the pointer it writes, passes through or
//! reads.

use crate::hash::{Oid, Sha256, to_hex};
use crate::odb::Odb;
use std::io;

/// What a pointer says first, naming the version of git-lfs's format.
const VERSION: &str = "version https://git-lfs.github.com/spec/v1";

/// Whether `command` runs git-lfs's `verb`, by any path to it, itself or
/// as `git lfs`, given no more than the file's name: a driver running
/// anything else, or git-lfs's output through another program, might
/// write something other than its pointer. A `filter-process` may be
/// given `--skip`, as `git lfs install --skip-smudge` writes it, which
/// leaves what it cleans alone.
pub(super) fn is_git_lfs(command: &str, verb: &str) -> bool {
    let is = |word: Option<&str>, program: &str| {
        word.is_some_and(|word| word.rsplit('/').next() == Some(program))
    };
    let mut words = command.split_ascii_whitespace();
    let first = words.next();
    (is(first, "git-lfs") || is(first, "git") && words.next() == Some("lfs"))
        && words.next() == Some(verb)
        && match (verb, &words.collect::<Vec<_>>()[..]) {
            ("filter-process", ["--skip", rest @ ..]) | (_, rest) => {
                matches!(rest, [] | ["%f"] | ["--", "%f"])
            }
        }
}

/// The pointer git-lfs writes for these contents.
fn lfs_pointer(raw: &[u8]) -> String {
    let mut sha = Sha256::new();
    sha.update(raw);
    format!(
        "{VERSION}\noid sha256:{}\nsize {}\n",
        to_hex(&sha.finish()),
        raw.len()
    )
}

/// Whether `raw` is an LFS pointer as git-lfs reads one: under its 1024
/// bytes, saying its version first, then an object and a size.
fn is_lfs_pointer(raw: &[u8]) -> bool {
    let Some(text) = (raw.len() < 1024)
        .then(|| std::str::from_utf8(raw).ok())
        .flatten()
    else {
        return false;
    };
    let hex = |hex: &str| hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit());
    let digits = |size: &str| !size.is_empty() && size.bytes().all(|b| b.is_ascii_digit());
    text.lines().next() == Some(VERSION)
        && text
            .lines()
            .any(|line| line.strip_prefix("oid sha256:").is_some_and(hex))
        && text
            .lines()
            .any(|line| line.strip_prefix("size ").is_some_and(digits))
}

/// Whether the index's blob, `oid`, is the LFS pointer git-lfs would write
/// for these contents. Contents that are a pointer already, git-lfs passes
/// through as they are.
pub(super) fn matches_lfs_pointer(odb: &mut Odb, oid: &Oid, raw: &[u8]) -> io::Result<bool> {
    if is_lfs_pointer(raw) {
        return Ok(Oid::of("blob", raw) == *oid);
    }
    let pointer = lfs_pointer(raw);
    if Oid::of("blob", pointer.as_bytes()) == *oid {
        return Ok(true);
    }
    // A pointer with extensions, or one written differently: compare what
    // it says rather than its bytes.
    let Some(blob) = odb.read(oid)? else {
        return Ok(false);
    };
    let text = String::from_utf8_lossy(&blob.data);
    let wanted_oid = pointer.lines().nth(1).unwrap_or_default();
    let wanted_size = pointer.lines().nth(2).unwrap_or_default();
    Ok(text.starts_with(VERSION)
        && text.lines().any(|l| l == wanted_oid)
        && text.lines().any(|l| l == wanted_size))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An object store holding these blobs as loose objects.
    fn store(name: &str, blobs: &[&[u8]]) -> std::path::PathBuf {
        let objects =
            std::env::temp_dir().join(format!("git-engine-lfs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&objects);
        for blob in blobs {
            let hex = Oid::of("blob", blob).hex();
            let dir = objects.join(&hex[..2]);
            std::fs::create_dir_all(&dir).expect("made no object folder");
            let mut raw = format!("blob {}\0", blob.len()).into_bytes();
            raw.extend_from_slice(blob);
            let packed = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6);
            std::fs::write(dir.join(&hex[2..]), packed).expect("wrote no object");
        }
        objects
    }

    #[test]
    fn a_working_file_that_is_a_pointer_already_is_passed_through() {
        let pointer = lfs_pointer(b"hello\n");
        let pointer = pointer.as_bytes();
        let short = b"version https://git-lfs.github.com/spec/v1\noid sha256:abc\nsize 3\n";
        let long = [pointer, &[b'x'; 1024][..]].concat();
        let objects = store("passed", &[b"unread"]);
        let mut odb = Odb::open(&objects);
        for (raw, oid, wanted, what) in [
            (
                pointer,
                Oid::of("blob", pointer),
                true,
                "a pointer, kept as it is",
            ),
            (
                pointer,
                Oid::of("blob", lfs_pointer(pointer).as_bytes()),
                false,
                "a pointer, taken for contents to point at",
            ),
            (
                &short[..],
                Oid::of("blob", lfs_pointer(short).as_bytes()),
                true,
                "contents whose object isn't one a pointer names, pointed at",
            ),
            (
                &long[..],
                Oid::of("blob", lfs_pointer(&long).as_bytes()),
                true,
                "contents of a pointer's 1024 bytes or more, pointed at",
            ),
        ] {
            assert_eq!(
                matches_lfs_pointer(&mut odb, &oid, raw).expect("read no blob"),
                wanted,
                "{what}"
            );
        }
        let _ = std::fs::remove_dir_all(&objects);
    }

    #[test]
    fn an_lfs_pointer_matches_by_what_it_says() {
        assert_eq!(
            lfs_pointer(b"hello\n"),
            "version https://git-lfs.github.com/spec/v1\n\
             oid sha256:5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03\n\
             size 6\n",
            "the pointer isn't the one git-lfs writes"
        );
        let extended = b"version https://git-lfs.github.com/spec/v1\n\
            ext-0-foo sha256:00\n\
            oid sha256:5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03\n\
            size 6\n";
        let other = b"version https://git-lfs.github.com/spec/v1\n\
            oid sha256:5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03\n\
            size 7\n";
        let changed = b"version https://git-lfs.github.com/spec/v1\n\
            oid sha256:5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be04\n\
            size 6\n";
        let objects = store("lfs", &[extended, other, changed]);
        let mut odb = Odb::open(&objects);
        let exact = Oid::of("blob", lfs_pointer(b"hello\n").as_bytes());
        for (oid, wanted, what) in [
            (exact, true, "the pointer git-lfs writes"),
            (
                Oid::of("blob", extended),
                true,
                "a pointer with an extension",
            ),
            (Oid::of("blob", other), false, "a pointer of another size"),
            (
                Oid::of("blob", changed),
                false,
                "a pointer of the same size to another object",
            ),
            (
                Oid::of("blob", b"absent"),
                false,
                "a pointer the store lacks",
            ),
        ] {
            assert_eq!(
                matches_lfs_pointer(&mut odb, &oid, b"hello\n").expect("read no blob"),
                wanted,
                "{what}"
            );
        }
        let _ = std::fs::remove_dir_all(&objects);
    }
}
