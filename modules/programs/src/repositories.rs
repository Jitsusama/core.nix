//! The git repositories under a folder, and one repository's files, asked
//! for by their keys, as the acts that list them want them
//! (modules/quickshell/launcher/Repositories.qml): `repositories:<folder>`
//! says each `.git` four folders deep at most, past no hidden folder, a
//! folder or a worktree's file, dated, and each repository's config lines naming a section or a
//! url; `files:<repository>` says its files as git would list them, its
//! hidden ones too but not what it ignores, dated, the newest few
//! thousand. Each is read as a watch or a get names it, never otherwise,
//! so nothing is read at rest. The lines are in the forms the shell read
//! from stat and grep (`%Y\t%n`, and a config's path, a colon, its line),
//! so its reading of them is unchanged.

use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use serde_json::{Value, json};

use crate::command;

/// How deep a `.git` is looked for under the folder: in
/// `src/github.com/someone/a/.git`, a repository three folders deep.
const DEEPEST: usize = 4;

/// The most of one repository's files its record holds, the ones changed
/// last, as the largest repository Joel keeps holds over 23,000.
const MOST_FILES: usize = 5000;

/// The record a key names, or none for a key of neither kind or a path
/// that isn't whole.
#[must_use]
pub fn record(key: &str) -> Option<Value> {
    let whole = |path: &str| path.starts_with('/');
    if let Some(folder) = key.strip_prefix("repositories:").filter(|path| whole(path)) {
        let (dated, configs) = found(Path::new(folder));
        return Some(json!({
            "key": key, "kind": "repositories", "folder": folder,
            "dated": dated, "configs": configs,
        }));
    }
    let repository = key.strip_prefix("files:").filter(|path| whole(path))?;
    Some(json!({
        "key": key, "kind": "files", "repository": repository,
        "dated": files(repository),
    }))
}

/// When a path last changed, in whole seconds, as stat's `%Y` says it,
/// without following a link.
fn seconds(path: &Path) -> Option<u64> {
    let changed = fs::symlink_metadata(path).ok()?.modified().ok()?;
    Some(changed.duration_since(UNIX_EPOCH).ok()?.as_secs())
}

/// Each `.git` under a folder, dated, and the lines of each one's config
/// that name a section or a url, its path before each.
fn found(folder: &Path) -> (Vec<String>, Vec<String>) {
    let mut dated = Vec::new();
    let mut configs = Vec::new();
    let mut under = vec![(folder.to_path_buf(), 1)];
    while let Some((at, depth)) = under.pop() {
        let Ok(read) = fs::read_dir(&at) else {
            continue;
        };
        for entry in read.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if entry.file_name() == ".git" {
                if let Some(changed) = seconds(&path) {
                    dated.push(format!("{changed}\t{}", path.display()));
                }
                if kind.is_dir() {
                    configs.extend(config_lines(&path.join("config")));
                }
            } else if kind.is_dir() && depth < DEEPEST && !hidden(&entry.file_name()) {
                under.push((path, depth + 1));
            }
        }
    }
    dated.sort();
    configs.sort_by(|a, b| config_of(a).cmp(config_of(b)));
    (dated, configs)
}

/// Whether a folder is hidden, as a repository's worktrees and caches
/// are, which hold no repository of their own to list.
fn hidden(name: &std::ffi::OsStr) -> bool {
    name.as_encoded_bytes().starts_with(b".")
}

/// The config a line is of, so a repository's lines stay together and in
/// their file's order.
fn config_of(line: &str) -> &str {
    line.find("/.git/config:").map_or(line, |at| &line[..at])
}

/// A config's lines naming a section or a url, its path, a colon, then
/// the line, as grep -H says them.
fn config_lines(config: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(config) else {
        return Vec::new();
    };
    text.lines()
        .filter(|line| {
            let line = line.trim_start();
            line.starts_with('[')
                || line
                    .strip_prefix("url")
                    .is_some_and(|rest| rest.trim_start().starts_with('='))
        })
        .map(|line| format!("{}:{line}", config.display()))
        .collect()
}

/// A repository's files as git lists them, tracked or not, but not what
/// it ignores, each dated, the one changed last first, the newest
/// [`MOST_FILES`]. None for a folder git can't read.
fn files(repository: &str) -> Vec<String> {
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
    let mut dated: Vec<(u64, String)> = printed
        .split('\0')
        .filter(|name| !name.is_empty())
        .filter_map(|name| {
            let path = format!("{repository}/{name}");
            let about = fs::symlink_metadata(&path).ok()?;
            about.is_file().then_some(())?;
            Some((seconds(Path::new(&path))?, path))
        })
        .collect();
    dated.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    dated.dedup_by(|a, b| a.1 == b.1);
    dated
        .into_iter()
        .take(MOST_FILES)
        .map(|(changed, path)| format!("{changed}\t{path}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_whole_path_of_either_kind_is_read() {
        assert!(record("repositories:src").is_none(), "a path from nowhere");
        assert!(record("files:src/alpha").is_none(), "a path from nowhere");
        assert!(record("folder:/src").is_none(), "another kind");
    }

    #[test]
    fn a_config_says_its_sections_and_urls_alone() {
        let folder = std::env::temp_dir().join(format!("repositories-{}", std::process::id()));
        let git = folder.join("alpha/.git");
        fs::create_dir_all(&git).unwrap();
        fs::write(
            git.join("config"),
            "[core]\n\tbare = false\n[remote \"origin\"]\n\turl = git@github.com:a/b.git\n\
             \tfetch = +refs/heads/*\n\turlish = no\n",
        )
        .unwrap();
        let (dated, configs) = found(&folder);
        let config = git.join("config").display().to_string();
        assert_eq!(
            dated.len(),
            1,
            "one repository wasn't found once: {dated:?}"
        );
        assert_eq!(
            configs,
            [
                format!("{config}:[core]"),
                format!("{config}:[remote \"origin\"]"),
                format!("{config}:\turl = git@github.com:a/b.git"),
            ],
            "a config's sections and urls"
        );
        fs::remove_dir_all(&folder).unwrap();
    }
}
