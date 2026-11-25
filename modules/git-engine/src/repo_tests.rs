//! The repository's tests, beside it as the object store's are: which
//! config files it reads, and in what order, from where the environment
//! says.

use super::config_files;
use std::path::{Path, PathBuf};

/// The files read for a repository at /r, home /h, in an environment
/// holding `vars`.
fn files(vars: &[(&str, &str)]) -> Vec<PathBuf> {
    config_files(Path::new("/r"), Some(Path::new("/h")), |name| {
        vars.iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| (*value).into())
    })
}

#[test]
fn a_file_named_in_bytes_not_utf_8_is_read_where_named() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let named = |end: &[u8]| OsString::from_vec([b"/", end, b"\xff"].concat());
    let (system, xdg) = (named(b"s"), named(b"x"));
    let read = config_files(Path::new("/r"), Some(Path::new("/h")), |name| match name {
        "GIT_CONFIG_SYSTEM" => Some(system.clone()),
        "XDG_CONFIG_HOME" => Some(xdg.clone()),
        _ => None,
    });
    assert_eq!(
        read[..2],
        [
            PathBuf::from(&system),
            PathBuf::from(&xdg).join("git/config")
        ],
        "a system or XDG folder named in bytes not UTF-8 was passed over"
    );
}

#[test]
fn config_files_are_read_system_first_then_global_then_the_repositorys() {
    let paths = |names: &[&str]| names.iter().map(PathBuf::from).collect::<Vec<_>>();
    assert_eq!(
        files(&[]),
        paths(&[
            "/etc/gitconfig",
            "/h/.config/git/config",
            "/h/.gitconfig",
            "/r/config"
        ]),
        "the files weren't read in git's order"
    );
    assert_eq!(
        files(&[
            ("GIT_CONFIG_SYSTEM", "/s"),
            ("GIT_CONFIG_GLOBAL", "/g"),
            ("XDG_CONFIG_HOME", "/x"),
        ]),
        paths(&["/s", "/g", "/r/config"]),
        "the system or global file wasn't taken from where the environment says"
    );
    assert_eq!(
        files(&[("GIT_CONFIG_NOSYSTEM", "1"), ("XDG_CONFIG_HOME", "/x")]),
        paths(&["/x/git/config", "/h/.gitconfig", "/r/config"]),
        "the system file was read where it's turned off, or XDG's wasn't followed"
    );
}
