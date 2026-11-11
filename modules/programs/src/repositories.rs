//! The git repositories under a folder, and one repository's files, asked
//! for by their keys and served ready for the acts that list them
//! (modules/quickshell/launcher/Repositories.qml), so the shell only
//! reads them. `repositories:<folder>` says each repository with a `.git`
//! four folders deep at most, past no hidden folder, a folder or a
//! worktree's file: its path, when its git last wrote there, as git does
//! whenever it changes its index, its branches or what it fetched, and its
//! page on GitHub, the one written last first, and those with a page there
//! again on their own, in the same order. `files:<repository>` says
//! its files as git would list them, its hidden ones too but not what it
//! ignores, each with when it changed and the folder it's in within the
//! repository, the one changed last first, the newest few thousand.
//!
//! Each is read as a watch or a get names it, and a repositories record is
//! read again only as one of the folders it was read from changes: each
//! `.git` found, and each folder outside a repository a new one could be
//! cloned into, never a repository's own files, so saving a file reads
//! nothing. Nothing is read at rest.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde_json::{Value, json};

use crate::command;

/// How deep a `.git` is looked for under the folder: in
/// `src/github.com/someone/a/.git`, a repository three folders deep.
const DEEPEST: usize = 4;

/// The most of one repository's files its record holds, the ones changed
/// last, since a row is made for each as it's stepped into and the largest
/// repository Joel keeps holds over 23,000. The rest are left to Neovim's
/// own finder.
const MOST_FILES: usize = 5000;

/// Milliseconds in a second: times are said in milliseconds, as the shell
/// keeps them, from the whole seconds a file's change is ranked by, so two
/// written in the same second keep their order by path between two reads.
const SECOND: u64 = 1000;

/// A record and the folders whose changes make it read again.
#[derive(Debug)]
pub struct Read {
    pub record: Value,
    pub heard: Vec<PathBuf>,
}

/// The record a key names, with the folders to hear it change by, or none
/// for a key of neither kind or a path that isn't whole.
#[must_use]
pub fn read(key: &str) -> Option<Read> {
    let whole = |path: &str| path.starts_with('/');
    if let Some(folder) = key.strip_prefix("repositories:").filter(|path| whole(path)) {
        let found = found(Path::new(folder));
        let listed: Vec<Value> = found
            .repositories
            .iter()
            .map(|one| json!({"path": one.path, "at": one.at, "page": one.page}))
            .collect();
        let pages: Vec<&Value> = listed.iter().filter(|one| !one["page"].is_null()).collect();
        return Some(Read {
            record: json!({
                "key": key, "kind": "repositories", "folder": folder,
                "repositories": listed, "pages": pages,
            }),
            heard: found.heard,
        });
    }
    let repository = key.strip_prefix("files:").filter(|path| whole(path))?;
    Some(Read {
        record: json!({
            "key": key, "kind": "files", "repository": repository,
            "files": files(repository),
        }),
        heard: Vec::new(),
    })
}

/// One repository, as it's served.
#[derive(Debug, PartialEq, Eq)]
struct Repository {
    path: String,
    at: u64,
    page: Option<Value>,
}

/// What a walk of a folder found.
#[derive(Debug, Default)]
struct Found {
    repositories: Vec<Repository>,
    heard: Vec<PathBuf>,
}

/// When a path last changed, in whole seconds, without following a link.
fn seconds(path: &Path) -> Option<u64> {
    let changed = fs::symlink_metadata(path).ok()?.modified().ok()?;
    Some(changed.duration_since(UNIX_EPOCH).ok()?.as_secs())
}

/// Each repository under a folder, the one whose git wrote last first and
/// two alike by their paths, and the folders to hear them change by: each
/// `.git` folder, and each folder walked outside a repository.
fn found(folder: &Path) -> Found {
    let mut found = Found::default();
    let mut under = vec![(folder.to_path_buf(), 1, false)];
    while let Some((at, depth, inside)) = under.pop() {
        let Ok(read) = fs::read_dir(&at) else {
            continue;
        };
        let entries: Vec<fs::DirEntry> = read.flatten().collect();
        let repository = entries.iter().any(|entry| entry.file_name() == ".git");
        if !inside && !repository {
            found.heard.push(at.clone());
        }
        for entry in entries {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if entry.file_name() == ".git" {
                if let Some(changed) = seconds(&path) {
                    let page = kind.is_dir().then(|| page(&path.join("config"))).flatten();
                    found.repositories.push(Repository {
                        path: at.display().to_string(),
                        at: changed * SECOND,
                        page,
                    });
                }
                if kind.is_dir() {
                    found.heard.push(path);
                }
            } else if kind.is_dir() && depth < DEEPEST && !hidden(&entry.file_name()) {
                under.push((path, depth + 1, inside || repository));
            }
        }
    }
    found
        .repositories
        .sort_by(|a, b| b.at.cmp(&a.at).then_with(|| a.path.cmp(&b.path)));
    found.heard.sort();
    found
}

/// Whether a folder is hidden, as a repository's worktrees and caches
/// are, which hold no repository of their own to list.
fn hidden(name: &std::ffi::OsStr) -> bool {
    name.as_encoded_bytes().starts_with(b".")
}

/// A repository's page on GitHub, from its config's remotes: origin's,
/// where it's there, since that's the one pushed to, then upstream's, as a
/// fork names what it came from, then the first other remote there, or
/// none for a repository with none on GitHub.
fn page(config: &Path) -> Option<Value> {
    let text = fs::read_to_string(config).ok()?;
    let there: Vec<(String, (String, String))> = remotes(&text)
        .into_iter()
        .filter_map(|(name, url)| Some((name, github(&url)?)))
        .collect();
    let (_, (owner, name)) = there
        .iter()
        .find(|(remote, _)| remote == "origin")
        .or_else(|| there.iter().find(|(remote, _)| remote == "upstream"))
        .or_else(|| there.first())?;
    Some(json!({
        "address": format!("https://github.com/{owner}/{name}"),
        "name": format!("{owner}/{name}"),
    }))
}

/// A config's remotes, each its name and url, in the file's order. A url is
/// a remote's only under its heading, since a submodule's has one too.
fn remotes(config: &str) -> Vec<(String, String)> {
    let mut named: Option<String> = None;
    let mut all = Vec::new();
    for line in config.lines().map(str::trim) {
        if line.starts_with('[') {
            named = line
                .strip_prefix("[remote")
                .and_then(|rest| rest.trim().strip_suffix(']'))
                .and_then(|rest| rest.trim().strip_prefix('"'))
                .and_then(|rest| rest.strip_suffix('"'))
                .filter(|name| !name.is_empty() && !name.contains('"'))
                .map(str::to_owned);
            continue;
        }
        let url = line
            .strip_prefix("url")
            .and_then(|rest| rest.trim_start().strip_prefix('='))
            .map(str::trim);
        if let (Some(url), Some(name)) = (url, &named) {
            all.push((name.clone(), url.to_owned()));
        }
    }
    all
}

/// A remote's owner and name on GitHub, however git was told it: by SSH's
/// short form or its URL, or by HTTPS, git's own protocol or a name and
/// password, `.git` or a slash after them or not; none for one elsewhere.
fn github(url: &str) -> Option<(String, String)> {
    let without_user = |rest: &str| -> String {
        match rest.split_once('@') {
            Some((user, after)) if !user.is_empty() && !user.contains('/') => after.to_owned(),
            _ => rest.to_owned(),
        }
    };
    let scheme = ["ssh://", "git://", "https://", "http://"]
        .iter()
        .find_map(|scheme| url.strip_prefix(scheme));
    let path = if let Some(rest) = scheme {
        let rest = without_user(rest);
        let (host, path) = rest.split_once('/')?;
        let port = host.strip_prefix("github.com")?;
        let numbered = port
            .strip_prefix(':')
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()));
        if !(port.is_empty() || numbered) {
            return None;
        }
        path.to_owned()
    } else {
        without_user(url).strip_prefix("github.com:")?.to_owned()
    };
    let path = path.strip_suffix('/').unwrap_or(&path);
    let (owner, name) = path.split_once('/')?;
    let name = name.strip_suffix(".git").unwrap_or(name);
    let named = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
    };
    (named(owner) && named(name)).then(|| (owner.to_owned(), name.to_owned()))
}

/// A repository's files as git lists them, tracked or not, but not what
/// it ignores, each `{path, at, within}`, the one changed last first and
/// two alike by their paths, the newest [`MOST_FILES`]; none for a folder
/// git can't read.
fn files(repository: &str) -> Vec<Value> {
    let git = std::env::var("PROGRAMS_GIT").unwrap_or_else(|_| "git".to_owned());
    let argv = [
        git,
        "-C".to_owned(),
        repository.to_owned(),
        "ls-files".to_owned(),
        "-z".to_owned(),
        "--cached".to_owned(),
        "--others".to_owned(),
        "--exclude-standard".to_owned(),
    ];
    let Ok(printed) = command::run(&argv) else {
        return Vec::new();
    };
    let mut dated: Vec<(u64, String, String)> = printed
        .split('\0')
        .filter(|name| !name.is_empty())
        .filter_map(|name| {
            let path = format!("{repository}/{name}");
            let about = fs::symlink_metadata(&path).ok()?;
            about.is_file().then_some(())?;
            let within = name.rsplit_once('/').map_or("", |(folder, _)| folder);
            Some((seconds(Path::new(&path))?, path, within.to_owned()))
        })
        .collect();
    dated.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    dated.dedup_by(|a, b| a.1 == b.1);
    dated
        .into_iter()
        .take(MOST_FILES)
        .map(|(changed, path, within)| {
            json!({"path": path, "at": changed * SECOND, "within": within})
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of its own for one case, made empty.
    fn folder(case: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("repositories-{case}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn only_a_whole_path_of_either_kind_is_read() {
        assert!(read("repositories:src").is_none(), "a path from nowhere");
        assert!(read("files:src/alpha").is_none(), "a path from nowhere");
        assert!(read("folder:/src").is_none(), "another kind");
    }

    #[test]
    fn a_repository_is_said_with_its_page_and_heard_by_its_git() {
        let src = folder("page");
        let git = src.join("alpha/.git");
        fs::create_dir_all(&git).unwrap();
        fs::write(
            git.join("config"),
            "[core]\n\tbare = false\n[remote \"origin\"]\n\turl = git@github.com:a/b.git\n",
        )
        .unwrap();
        fs::create_dir_all(src.join("alpha/docs")).unwrap();
        let found = found(&src);
        let paths: Vec<&str> = found
            .repositories
            .iter()
            .map(|one| one.path.as_str())
            .collect();
        assert_eq!(
            paths,
            [src.join("alpha").display().to_string()],
            "one repository, once"
        );
        assert_eq!(
            found.repositories[0].page,
            Some(json!({"address": "https://github.com/a/b", "name": "a/b"})),
            "its page on GitHub"
        );
        assert_eq!(
            found.heard,
            [src.clone(), git],
            "the folder a repository could be cloned into and its git, never its own files"
        );
        fs::remove_dir_all(&src).unwrap();
    }

    #[test]
    fn the_repositories_go_the_one_written_last_first_then_by_path() {
        let src = folder("order");
        for name in ["old", "new", "also"] {
            fs::create_dir_all(src.join(name).join(".git")).unwrap();
        }
        let set = |name: &str, seconds: u64| {
            let file = fs::File::open(src.join(name).join(".git")).unwrap();
            file.set_modified(UNIX_EPOCH + std::time::Duration::from_secs(seconds))
                .unwrap();
        };
        set("old", 1_700_000_000);
        set("new", 1_700_000_500);
        set("also", 1_700_000_500);
        let said: Vec<(String, u64)> = found(&src)
            .repositories
            .into_iter()
            .map(|one| {
                (
                    one.path.rsplit('/').next().unwrap_or_default().to_owned(),
                    one.at,
                )
            })
            .collect();
        assert_eq!(
            said,
            [
                ("also".to_owned(), 1_700_000_500_000),
                ("new".to_owned(), 1_700_000_500_000),
                ("old".to_owned(), 1_700_000_000_000),
            ],
            "the one written last first, two alike by their paths, in milliseconds"
        );
        fs::remove_dir_all(&src).unwrap();
    }

    #[test]
    fn a_remote_is_one_only_under_its_heading() {
        let config = "\turl = https://stray.example/x\n[remote \"origin\"]\n\
                      \turl = git@github.com:Jitsusama/a.git\n[remote \"mirror\"]\n\
                      \x20  url=https://gitlab.com/j/a\n[submodule \"vendor\"]\n\
                      \turl = https://github.com/v/vendor\n\turlish = no\n";
        assert_eq!(
            remotes(config),
            [
                (
                    "origin".to_owned(),
                    "git@github.com:Jitsusama/a.git".to_owned()
                ),
                ("mirror".to_owned(), "https://gitlab.com/j/a".to_owned()),
            ],
            "each remote's url in the file's order, none from elsewhere"
        );
    }

    #[test]
    fn a_page_on_github_is_read_however_git_was_told_it() {
        let urls = [
            (
                "git@github.com:Jitsusama/core.nix.git",
                Some("Jitsusama/core.nix"),
            ),
            ("github.com:Jitsusama/core.nix", Some("Jitsusama/core.nix")),
            (
                "ssh://git@github.com/Jitsusama/core.nix",
                Some("Jitsusama/core.nix"),
            ),
            ("ssh://git@github.com:22/o/r.git", Some("o/r")),
            ("https://github.com/o/r.git", Some("o/r")),
            ("https://github.com/o/r/", Some("o/r")),
            ("https://joel:token@github.com/o/r", Some("o/r")),
            ("git://github.com/o/r", Some("o/r")),
            ("https://gitlab.com/o/r", None),
            ("https://github.com/o", None),
            ("https://github.com.evil.example/o/r", None),
            ("https://github.com/o/r/tree", None),
            ("/home/joel/src/r", None),
        ];
        for (url, meant) in urls {
            let said = github(url).map(|(owner, name)| format!("{owner}/{name}"));
            assert_eq!(said.as_deref(), meant, "the page of {url}");
        }
    }

    #[test]
    fn origins_page_goes_first_then_upstreams_then_the_first_there() {
        let src = folder("pages");
        let config = src.join("config");
        let cases = [
            (
                "[remote \"upstream\"]\nurl = https://github.com/up/r\n\
                 [remote \"origin\"]\nurl = git@github.com:me/r.git\n",
                Some("me/r"),
            ),
            (
                "[remote \"mine\"]\nurl = https://github.com/mine/r\n\
                 [remote \"upstream\"]\nurl = https://github.com/up/r\n\
                 [remote \"origin\"]\nurl = https://gitlab.com/me/r\n",
                Some("up/r"),
            ),
            (
                "[remote \"work\"]\nurl = https://gitlab.com/w/r\n\
                 [remote \"mine\"]\nurl = https://github.com/mine/r\n",
                Some("mine/r"),
            ),
            ("[remote \"work\"]\nurl = https://gitlab.com/w/r\n", None),
        ];
        for (text, meant) in cases {
            fs::write(&config, text).unwrap();
            let said =
                page(&config).map(|page| page["name"].as_str().unwrap_or_default().to_owned());
            assert_eq!(said.as_deref(), meant, "the page of {text}");
        }
        fs::remove_dir_all(&src).unwrap();
    }
}
