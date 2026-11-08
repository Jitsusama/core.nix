//! The package a program's command comes from: its command found along
//! the search path, followed through its links to the Nix store, and the
//! store path's name split into a package and its version as Nix splits
//! one (`builtins.parseDrvName`).

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// Where a command is: itself where it names a folder, or else the first
/// file of its name along `search`, a colon between each folder.
fn found(program: &str, search: &str) -> Option<PathBuf> {
    if program.contains('/') {
        return Some(PathBuf::from(program));
    }
    search
        .split(':')
        .filter(|folder| !folder.is_empty())
        .map(|folder| Path::new(folder).join(program))
        .find(|path| path.is_file())
}

/// A store path's name as Nix splits it: the package up to the first `-`
/// followed by something other than a letter, and the version after it.
pub fn split(name: &str) -> (String, Option<String>) {
    let bytes = name.as_bytes();
    for (at, byte) in bytes.iter().enumerate() {
        if *byte == b'-'
            && bytes
                .get(at + 1)
                .is_some_and(|next| !next.is_ascii_alphabetic())
        {
            return (name[..at].to_owned(), Some(name[at + 1..].to_owned()));
        }
    }
    (name.to_owned(), None)
}

/// The package `program` comes from, or null where it isn't found or
/// isn't in the Nix store.
pub fn of(program: &str, search: &str) -> Option<Value> {
    let real = std::fs::canonicalize(found(program, search)?).ok()?;
    let inside = real.strip_prefix("/nix/store").ok()?;
    let top = inside.components().next()?.as_os_str().to_str()?;
    let (_, named) = top.split_once('-')?;
    let (name, version) = split(named);
    Some(json!({
        "name": name,
        "version": version,
        "store_path": format!("/nix/store/{top}"),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_splits_where_nix_splits_it() {
        assert_eq!(
            split("kitty-0.44.0"),
            ("kitty".into(), Some("0.44.0".into()))
        );
        assert_eq!(
            split("google-chrome-142.0.7444.59"),
            ("google-chrome".into(), Some("142.0.7444.59".into()))
        );
        assert_eq!(split("hello"), ("hello".into(), None));
        assert_eq!(
            split("mpv-with-scripts-0.40.0"),
            ("mpv-with-scripts".into(), Some("0.40.0".into()))
        );
    }

    #[test]
    fn a_program_outside_the_store_has_no_package() {
        assert_eq!(of("/bin/sh-that-isnt-there", ""), None);
    }
}
