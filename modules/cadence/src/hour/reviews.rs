//! The harness's records of review rounds and their fixes, read as
//! merge-net reads them: round files only, and each other machine's mirror
//! beside them, saying why a mirror couldn't be read rather than leaving
//! its rounds out unsaid, which would count the wrong rounds past five.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;
use std::time::SystemTime;

use super::measures::rounds::Mirrored;

/// The text of each record in `folder` written since `since`. A record is
/// a `.json`, as the harness writes one, so a half-written `.tmp` beside
/// it isn't read. A folder not there, or not readable, holds none.
#[must_use]
pub fn written(folder: &Path, since: SystemTime) -> Vec<String> {
    listed(folder, since).unwrap_or_default()
}

/// The records in `folder` written since `since`, or why it can't be listed.
fn listed(folder: &Path, since: SystemTime) -> io::Result<Vec<String>> {
    Ok(fs::read_dir(folder)?
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|e| e == "json"))
        .filter(|entry| {
            entry
                .metadata()
                .and_then(|m| m.modified())
                .is_ok_and(|at| at >= since)
        })
        .filter_map(|entry| fs::read_to_string(entry.path()).ok())
        .collect())
}

/// The records written since `since` mirrored under `root`, a folder (or a
/// link to one) a node, as merge-net's `mirrored` reads them: a hidden
/// name, as a sync tool's own folder, and a stray file beside the nodes
/// are left out. A root not there mirrors nothing, as on a machine no
/// mirror fills. A root or node that can't be listed, or a node holding a
/// runs folder and no records, as if mirrored one folder too high, is said
/// in `trouble`.
#[must_use]
pub fn mirrored(root: &Path, since: SystemTime) -> Mirrored {
    let mut mirrored = Mirrored::default();
    let nodes = match fs::read_dir(root) {
        Ok(nodes) => nodes,
        Err(error) if error.kind() == ErrorKind::NotFound => return mirrored,
        Err(error) => {
            let root = root.display();
            mirrored.trouble.push(format!(
                "the rounds mirrored under {root} can't be listed ({error})"
            ));
            return mirrored;
        }
    };
    let mut names: Vec<String> = nodes
        .flatten()
        .map(|node| node.file_name().to_string_lossy().into_owned())
        .filter(|name| !name.starts_with('.'))
        .collect();
    names.sort();
    for name in names {
        let node = root.join(&name);
        let said = match fs::metadata(&node).map(|found| found.is_dir()) {
            Ok(false) => continue,
            Ok(true) => match listed(&node, since) {
                Ok(runs) if runs.is_empty() && node.join("runs").is_dir() => format!(
                    "the mirror from {name} holds a runs folder and no rounds, as if mirrored \
                     one folder too high"
                ),
                Ok(runs) => {
                    mirrored.runs.extend(runs);
                    continue;
                }
                Err(error) => format!("the mirror from {name} can't be listed ({error})"),
            },
            Err(error) => format!("the mirror from {name} can't be read ({error})"),
        };
        mirrored.trouble.push(said);
    }
    mirrored
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn put(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().expect("a file the test names in a folder"))
            .expect("the test's folder made");
        fs::write(path, text).expect("the test's file written");
    }

    #[test]
    fn mirrors_are_read_as_merge_net_reads_them() {
        let root = std::env::temp_dir().join(format!("cadence-mirrors-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        put(&root.join("mac/one.json"), "one");
        put(&root.join("mac/one.json.tmp"), "half");
        put(&root.join(".sync-tool/two.json"), "hidden");
        put(&root.join("stray.json"), "stray");
        put(&root.join("high/runs/three.json"), "high");
        put(&root.join("old/four.json"), "four");
        let read = mirrored(&root, UNIX_EPOCH);
        assert_eq!(
            read.runs,
            ["one", "four"],
            "a half-written, hidden or stray file was read, or a node's records weren't"
        );
        assert_eq!(read.trouble.len(), 1, "{:?}", read.trouble);
        assert!(
            read.trouble[0].contains("from high") && read.trouble[0].contains("too high"),
            "a node mirrored a folder too high went unsaid: {:?}",
            read.trouble
        );
        let later = SystemTime::now() + Duration::from_secs(60);
        assert!(
            written(&root.join("mac"), later).is_empty(),
            "a record written before the rounds counted began was read"
        );
        let none = mirrored(&root.join("none"), UNIX_EPOCH);
        assert!(
            none.runs.is_empty() && none.trouble.is_empty(),
            "a machine no mirror fills was said to have trouble: {none:?}"
        );
        let unlisted = mirrored(&root.join("stray.json"), UNIX_EPOCH);
        assert!(
            unlisted.trouble[0].contains("can't be listed"),
            "a mirrors folder that can't be listed went unsaid: {unlisted:?}"
        );
        fs::remove_dir_all(&root).expect("the test's folder removed");
    }
}
