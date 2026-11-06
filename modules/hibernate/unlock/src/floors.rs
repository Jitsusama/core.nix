//! The hard memory floors, cgroup v2's `memory.min`, set aside while a
//! hibernation image is made, and put back once the machine is on again.
//!
//! A floor keeps a slice's memory from being taken back, and the kernel
//! keeps to it while it makes room for the image too: 7.3's mm/vmscan.c
//! skips every cgroup under its floor, whatever is reclaiming. The slices
//! keep 2.16 GiB of an 8 GiB laptop under Joel's programs, more than the
//! image may hold beside the reserve, so on 6 November a hibernation rolled
//! back 380,633 pages short with 5 GB of memory to spare. Everything is
//! frozen while the image is made, so a floor guards nothing then. A cgroup
//! is protected only as far as each one above it is, so emptying the top
//! level's floors sets aside every one below them.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;

/// Empties the floor of every cgroup directly under `root` that has one,
/// keeping each in `kept` first, a `<name> <bytes>` line apiece, so the
/// floors can be put back even if the machine never comes back to this
/// run. Says how many it set aside.
pub fn lift(root: &Path, kept: &Path) -> io::Result<usize> {
    let mut floors: Vec<(String, String)> = fs::read_dir(root)?
        .flatten()
        .filter_map(|group| {
            let name = group.file_name().into_string().ok()?;
            let bytes = fs::read_to_string(group.path().join("memory.min")).ok()?;
            let bytes = bytes.trim();
            (!bytes.is_empty() && bytes != "0").then(|| (name, bytes.to_owned()))
        })
        .collect();
    floors.sort();
    if floors.is_empty() {
        return Ok(0);
    }
    let lines: String = floors
        .iter()
        .map(|(name, bytes)| format!("{name} {bytes}\n"))
        .collect();
    fs::write(kept, lines)?;
    for (name, _) in &floors {
        fs::write(root.join(name).join("memory.min"), "0")?;
    }
    Ok(floors.len())
}

/// Puts back every floor `lift` kept, then forgets them. A cgroup gone
/// since, or a line naming anything but one directly under `root`, is
/// skipped. Says how many it put back.
pub fn restore(root: &Path, kept: &Path) -> io::Result<usize> {
    let text = match fs::read_to_string(kept) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    };
    let restored = text
        .lines()
        .filter_map(|line| line.split_once(' '))
        .filter(|(name, _)| !name.is_empty() && !name.contains('/') && !name.starts_with('.'))
        .filter(|(name, bytes)| fs::write(root.join(name).join("memory.min"), bytes).is_ok())
        .count();
    fs::remove_file(kept)?;
    Ok(restored)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A cgroup tree of its own in the temporary folder: each `(path,
    /// floor)` a directory with that `memory.min`.
    fn tree(test: &str, groups: &[(&str, &str)]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("floors-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for (path, floor) in groups {
            fs::create_dir_all(root.join(path)).unwrap();
            fs::write(root.join(path).join("memory.min"), floor).unwrap();
        }
        root
    }

    fn floor(root: &Path, path: &str) -> String {
        fs::read_to_string(root.join(path).join("memory.min")).unwrap()
    }

    #[test]
    fn only_the_top_levels_floors_are_set_aside() {
        let root = tree(
            "lift",
            &[
                ("user.slice", "2323083264\n"),
                ("user.slice/user-1000.slice", "2323083264\n"),
                ("system.slice", "0\n"),
                ("init.scope", "0\n"),
            ],
        );
        let kept = root.join("kept");
        assert_eq!(lift(&root, &kept).unwrap(), 1);
        assert_eq!(floor(&root, "user.slice"), "0");
        assert_eq!(
            floor(&root, "user.slice/user-1000.slice"),
            "2323083264\n",
            "a floor below the top is set aside by its parent's, so it's left as it was"
        );
        assert_eq!(floor(&root, "system.slice"), "0\n");
        assert_eq!(
            fs::read_to_string(&kept).unwrap(),
            "user.slice 2323083264\n"
        );
    }

    #[test]
    fn the_floors_come_back_as_they_were_and_are_forgotten() {
        let root = tree(
            "restore",
            &[
                ("user.slice", "2323083264\n"),
                ("floored.slice", "629145600\n"),
            ],
        );
        let kept = root.join("kept");
        assert_eq!(lift(&root, &kept).unwrap(), 2);
        assert_eq!(restore(&root, &kept).unwrap(), 2);
        assert_eq!(floor(&root, "user.slice"), "2323083264");
        assert_eq!(floor(&root, "floored.slice"), "629145600");
        assert!(
            !kept.exists(),
            "the kept floors are forgotten once put back"
        );
    }

    #[test]
    fn with_no_floors_nothing_is_kept() {
        let root = tree("none", &[("system.slice", "0\n")]);
        let kept = root.join("kept");
        assert_eq!(lift(&root, &kept).unwrap(), 0);
        assert!(!kept.exists());
        assert_eq!(
            restore(&root, &kept).unwrap(),
            0,
            "nothing kept, nothing to put back"
        );
    }

    #[test]
    fn a_kept_line_reaches_only_a_top_level_cgroup() {
        let root = tree("reach", &[("user.slice", "0")]);
        fs::create_dir_all(root.join("user.slice/deeper")).unwrap();
        let kept = root.join("kept");
        fs::write(
            &kept,
            "user.slice/deeper 5\n.. 5\nuser.slice 7\ngone.slice 9\n",
        )
        .unwrap();
        assert_eq!(restore(&root, &kept).unwrap(), 1);
        assert_eq!(floor(&root, "user.slice"), "7");
        assert!(!root.join("user.slice/deeper/memory.min").exists());
    }
}
