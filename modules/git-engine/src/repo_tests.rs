//! The repository's tests, beside it as the object store's are: which
//! config files it reads, and in what order, from where the environment
//! says.

use super::{Location, SYSTEM_ATTRIBUTES, SYSTEM_CONFIG, config_files, load_config};
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
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
    let named = |end: &[u8]| OsString::from_vec([b"/", end, b"\xff"].concat());
    let (system, xdg) = (named(b"s"), named(b"x"));
    let read = config_files(Path::new("/r"), Some(Path::new("/h")), |name| match name {
        "GIT_CONFIG_SYSTEM" => Some(system.clone()),
        "XDG_CONFIG_HOME" => Some(xdg.clone()),
        _ => None,
    });
    assert_eq!(
        read,
        [
            PathBuf::from(&system),
            PathBuf::from(&xdg).join("git/config"),
            PathBuf::from("/h/.gitconfig"),
            PathBuf::from("/r/config"),
        ],
        "a system or XDG folder named in bytes not UTF-8 was passed over"
    );
}

#[test]
fn an_empty_xdg_folder_is_taken_as_unset() {
    assert_eq!(
        files(&[("GIT_CONFIG_NOSYSTEM", "1"), ("XDG_CONFIG_HOME", "")]),
        ["/h/.config/git/config", "/h/.gitconfig", "/r/config"].map(PathBuf::from),
        "an empty XDG_CONFIG_HOME named a file relative to where the engine runs"
    );
}

/// Names the folder a child of these tests reads its config from, and so
/// that it's the child.
const CHILD: &str = "GIT_ENGINE_TEST_CHILD_FOLDER";

/// The global file, named in bytes that aren't UTF-8 where the file system
/// takes them; a Mac's refuses such a name, so there it's UTF-8.
fn global_in(folder: &Path) -> PathBuf {
    let name: &[u8] = if cfg!(target_os = "macos") {
        b"global"
    } else {
        b"global\xff"
    };
    folder.join(OsStr::from_bytes(name))
}

#[test]
fn config_is_read_from_the_environment_s_own_bytes() {
    // The environment can't be set for one test alone, so the test runs
    // again in a child given it, which reads the config as the engine
    // does on opening a repository.
    if let Some(folder) = std::env::var_os(CHILD) {
        let folder = PathBuf::from(folder);
        let location = Location {
            workdir: folder.clone(),
            git_dir: folder.clone(),
            common: folder,
        };
        let config = load_config(&location, None, None);
        assert_eq!(
            config.get("a.global"),
            Some("yes"),
            "a global file named in bytes not UTF-8 wasn't read"
        );
        assert_eq!(
            config.get("a.passed"),
            Some("yes"),
            "a -c pair was lost to a byte not UTF-8 in another"
        );
        return;
    }
    let folder = std::env::temp_dir().join(format!("git-engine-env-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("the temp folder takes a folder");
    std::fs::write(global_in(&folder), "[a]\n\tglobal = yes\n")
        .expect("the temp folder takes the global file");
    let name = "repo::tests::config_is_read_from_the_environment_s_own_bytes";
    let tests = std::env::current_exe().expect("the tests know where they are");
    let child = std::process::Command::new(tests)
        .args(["--exact", name])
        .env(CHILD, &folder)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", global_in(&folder))
        .env(
            "GIT_CONFIG_PARAMETERS",
            OsString::from_vec(b"'a.passed'='yes' 'a.odd'='\xff'".to_vec()),
        )
        .env_remove("GIT_CONFIG_COUNT")
        .output()
        .expect("the tests run again as a child");
    let _ = std::fs::remove_dir_all(&folder);
    let said = String::from_utf8_lossy(&child.stdout);
    assert!(
        child.status.success() && said.contains("1 passed"),
        "the child reading the environment failed, or ran no test: {said}"
    );
}

#[test]
fn config_files_are_read_system_first_then_global_then_the_repositorys() {
    let paths = |names: &[&str]| names.iter().map(PathBuf::from).collect::<Vec<_>>();
    assert_eq!(
        files(&[]),
        paths(&[
            SYSTEM_CONFIG,
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

#[test]
fn the_system_files_are_where_the_git_built_beside_reads_them() {
    // Only a build beside a git, as the package's, names its files, and
    // only there is the git asked the same; elsewhere they're /etc's.
    if option_env!("GIT_ENGINE_CONFIG_SYSTEM").is_none() {
        assert_eq!(
            (SYSTEM_CONFIG, SYSTEM_ATTRIBUTES),
            ("/etc/gitconfig", "/etc/gitattributes"),
            "a build that named no system files took them from somewhere but /etc"
        );
        return;
    }
    let said = |name: &str| {
        let git = std::process::Command::new("git")
            .args(["var", name])
            .env_remove(name)
            .output()
            .expect("the git built beside is there to ask");
        PathBuf::from(String::from_utf8_lossy(&git.stdout).trim_end())
    };
    assert_eq!(
        files(&[])[0],
        said("GIT_CONFIG_SYSTEM"),
        "the system config wasn't read where git reads it"
    );
    assert_eq!(
        PathBuf::from(SYSTEM_ATTRIBUTES),
        said("GIT_ATTR_SYSTEM"),
        "the system attributes weren't read where git reads them"
    );
}
