//! A repository's files as they lie in a folder, for a check holding one
//! measure at each change, where no git is to read a ref from (`git.rs`
//! reads what was merged, for the hourly figures), and what that check
//! finds beside its waiting list.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::measure::{self, File};

/// Every file's path under `tree`, from it, and the text of each a
/// measure reads (`measure::is_read`), as `git::files` gives them: git's
/// own folder is left out, and so are links, which `git::files` leaves
/// out too, as no file of the repository's own. Each is opened by its
/// name as the disk holds it, and named lossily only once read, so a
/// name that isn't UTF-8 is read rather than looked for under another.
pub fn files(tree: &Path) -> Result<(Vec<String>, Vec<File>), String> {
    let mut paths = Vec::new();
    let mut read = Vec::new();
    let mut folders = vec![PathBuf::new()];
    while let Some(folder) = folders.pop() {
        let at = tree.join(&folder);
        let entries =
            fs::read_dir(&at).map_err(|why| format!("{} couldn't be read: {why}", at.display()))?;
        for entry in entries {
            let entry = entry.map_err(|why| format!("{} couldn't be read: {why}", at.display()))?;
            let inside = folder.join(entry.file_name());
            let path = inside.to_string_lossy().into_owned();
            let kind = entry
                .file_type()
                .map_err(|why| format!("{path} couldn't be read: {why}"))?;
            if kind.is_dir() && entry.file_name() != ".git" {
                folders.push(inside);
            } else if kind.is_file() {
                if measure::is_read(&path) {
                    let bytes =
                        fs::read(tree.join(&inside)).map_err(|why| format!("{path}: {why}"))?;
                    read.push(File {
                        path: path.clone(),
                        text: String::from_utf8_lossy(&bytes).into_owned(),
                    });
                }
                paths.push(path);
            }
        }
    }
    paths.sort();
    read.sort_by(|one, other| one.path.cmp(&other.path));
    Ok((paths, read))
}

/// What a check finds held against `waiting`, the list of what still
/// waits on its owner, each a line `file path` with no line number, since
/// one moves with every edit above it, listed once for each mention it
/// holds. Each of `found` (`file:line: path`, as the measure names it)
/// whose file and path are named more often than the list holds them,
/// all its mentions told since none says which is new, and each line the
/// list holds more often than it's found, so the list only shrinks. A line
/// of a file in `unread`, whose comments couldn't be known, stands: none
/// of its file's mentions can be told gone.
pub fn held(found: &[String], unread: &[String], waiting: &[String]) -> (Vec<String>, Vec<String>) {
    let mut found_of: BTreeMap<String, Vec<&String>> = BTreeMap::new();
    for named in found {
        found_of.entry(keyed(named)).or_default().push(named);
    }
    // The path a line names never holds a space, so its file is all
    // before the last.
    let of_unread = |line: &str| {
        line.rsplit_once(' ')
            .is_some_and(|(file, _)| unread.iter().any(|unread| unread == file))
    };
    let mut listed: BTreeMap<&str, usize> = BTreeMap::new();
    for line in waiting.iter().filter(|line| !of_unread(line)) {
        *listed.entry(line.as_str()).or_default() += 1;
    }
    let new = found_of
        .iter()
        .filter(|(key, named)| named.len() > listed.get(key.as_str()).copied().unwrap_or(0))
        .flat_map(|(_, named)| named.iter().map(|named| (*named).clone()))
        .collect();
    let gone = listed
        .iter()
        .filter_map(|(line, times)| {
            let mentions = found_of.get(*line).map_or(0, Vec::len);
            match mentions {
                0 if *times == 1 => Some(format!("{line} (found nowhere: take it off)")),
                0 => Some(format!(
                    "{line} (listed {times} times, found nowhere: take all {times} off)"
                )),
                mentions if mentions < *times => Some(format!(
                    "{line} (listed {times} times, found {mentions}: list it {} fewer)",
                    times - mentions
                )),
                _ => None,
            }
        })
        .collect();
    (new, gone)
}

/// `file:line: path` as `file path`, cut at the last `": "`, since the
/// path the measure names never holds a space, so a file whose name holds
/// `": "`, or `:7: ` itself, keeps its whole name.
fn keyed(named: &str) -> String {
    let cut = named.rsplit_once(": ").and_then(|(place, path)| {
        let (file, line) = place.rsplit_once(':')?;
        let numbered = !line.is_empty() && line.bytes().all(|byte| byte.is_ascii_digit());
        numbered.then_some((file, path))
    });
    cut.map_or_else(|| named.to_owned(), |(file, path)| format!("{file} {path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folders_files_are_read_as_gits_tree_holds_them() {
        let tree = tempfile::tempdir().unwrap();
        let root = tree.path();
        fs::create_dir_all(root.join("modules/a/src")).unwrap();
        fs::create_dir_all(root.join(".git/refs")).unwrap();
        fs::write(
            root.join("modules/a/src/lib.rs"),
            "// modules/a/README.md\n",
        )
        .unwrap();
        fs::write(root.join("modules/a/logo.png"), [0x89, b'P']).unwrap();
        fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        std::os::unix::fs::symlink("src/lib.rs", root.join("modules/a/linked.rs")).unwrap();
        let (paths, read) = files(root).unwrap();
        assert_eq!(
            paths,
            ["modules/a/logo.png", "modules/a/src/lib.rs"],
            "git's own folder or a link was counted a file, or a file was missed"
        );
        let read: Vec<(&str, &str)> = read
            .iter()
            .map(|file| (file.path.as_str(), file.text.as_str()))
            .collect();
        assert_eq!(
            read,
            [("modules/a/src/lib.rs", "// modules/a/README.md\n")],
            "a file no measure reads was read, or code wasn't"
        );
    }

    /// Linux's filesystems hold any bytes in a name, as macOS's refuse to,
    /// so only there can a name that isn't UTF-8 be made.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_name_that_isnt_utf8_is_still_read() {
        use std::os::unix::ffi::OsStrExt;
        let tree = tempfile::tempdir().unwrap();
        let root = tree.path();
        let folder = root.join(std::ffi::OsStr::from_bytes(b"modules/caf\xe9"));
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("lib.rs"), "// modules/gone.rs\n").unwrap();
        let (paths, read) = files(root).unwrap();
        assert_eq!(
            (paths.len(), read.len()),
            (1, 1),
            "a file below a name that isn't UTF-8 wasn't read: {paths:?}"
        );
        assert_eq!(
            read[0].text, "// modules/gone.rs\n",
            "a file below a name that isn't UTF-8 was read as another"
        );
    }

    #[test]
    fn whats_found_is_held_against_its_waiting_list_without_line_numbers() {
        let found = [
            "modules/a/x.rs:3: modules/a/gone.rs".to_owned(),
            "modules/b/y.nix:12: tests/vm/gone.nix".to_owned(),
        ];
        let waiting = [
            "modules/a/x.rs modules/a/gone.rs".to_owned(),
            "modules/c/z.sh tests/fixed.nix".to_owned(),
        ];
        assert_eq!(
            held(&found, &[], &waiting),
            (
                vec!["modules/b/y.nix:12: tests/vm/gone.nix".to_owned()],
                vec!["modules/c/z.sh tests/fixed.nix (found nowhere: take it off)".to_owned()],
            ),
            "a listed path wasn't let wait, an unlisted one was, or a line nothing \
             matches stayed on the list"
        );
    }

    #[test]
    fn what_waits_in_a_file_whose_comments_cant_be_known_stands() {
        // modules/c/z.sh couldn't be read, and modules/c/z.sh.bak, whose
        // name it begins, could.
        let waiting = [
            "modules/c/z.sh tests/gone.nix".to_owned(),
            "modules/c/z.sh tests/gone.nix".to_owned(),
            "modules/c/z.sh.bak tests/gone.nix".to_owned(),
        ];
        assert_eq!(
            held(&[], &["modules/c/z.sh".to_owned()], &waiting),
            (
                Vec::<String>::new(),
                vec!["modules/c/z.sh.bak tests/gone.nix (found nowhere: take it off)".to_owned()],
            ),
            "what waits in a file that couldn't be read was told gone, or what waits in \
             another beginning with its name stood"
        );
    }

    #[test]
    fn a_line_holds_one_mention_however_many_its_file_holds() {
        let found = [
            "modules/a/x.rs:3: modules/a/gone.rs".to_owned(),
            "modules/a/x.rs:9: modules/a/gone.rs".to_owned(),
            "modules/b/y.rs:1: tests/gone.nix".to_owned(),
        ];
        let waiting = [
            "modules/a/x.rs modules/a/gone.rs".to_owned(),
            "modules/b/y.rs tests/gone.nix".to_owned(),
            "modules/b/y.rs tests/gone.nix".to_owned(),
            "modules/b/y.rs tests/gone.nix".to_owned(),
            "modules/c/z.sh tests/fixed.nix".to_owned(),
            "modules/c/z.sh tests/fixed.nix".to_owned(),
        ];
        assert_eq!(
            held(&found, &[], &waiting),
            (
                vec![
                    "modules/a/x.rs:3: modules/a/gone.rs".to_owned(),
                    "modules/a/x.rs:9: modules/a/gone.rs".to_owned(),
                ],
                vec![
                    "modules/b/y.rs tests/gone.nix (listed 3 times, found 1: list it 2 fewer)"
                        .to_owned(),
                    "modules/c/z.sh tests/fixed.nix (listed 2 times, found nowhere: take all \
                     2 off)"
                        .to_owned(),
                ],
            ),
            "a second mention in a listed file waited unlisted, or a line listed more \
             often than it's found stayed, or was told to come off by the wrong count"
        );
    }

    #[test]
    fn a_file_whose_name_holds_a_colon_keeps_its_whole_name() {
        assert_eq!(
            [
                keyed("modules/a/odd: name.rs:4: modules/a/gone.rs"),
                keyed("modules/a/a:7: b.rs:2: modules/a/gone.rs"),
            ],
            [
                "modules/a/odd: name.rs modules/a/gone.rs",
                "modules/a/a:7: b.rs modules/a/gone.rs",
            ],
            "a file's name was cut at a colon inside it"
        );
    }
}
