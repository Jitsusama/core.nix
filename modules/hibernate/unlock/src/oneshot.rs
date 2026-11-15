//! The next start set, once, to the boot entry this one came from, while an
//! image is made on a machine whose default system boots another kernel,
//! initrd or kernel parameters than the one running.
//!
//! A switch to a new kernel or initrd makes it the default for the next
//! start, but the running kernel stays until a reboot. An image written
//! then would be read at the next start by the new default: Linux refuses
//! an image another kernel wrote and starts afresh, losing what was open,
//! and a new initrd opens it with programs the running system never had,
//! as on 15 November, when #4653 moved the laptop's initrd. systemd-boot's
//! one-shot entry, set to `@current`, starts the entry this start came
//! from once, so the image goes back through the kernel that wrote it, and
//! the start after takes the default again. Anything it can't read, it
//! leaves alone: a machine without systemd-boot resumes as it did.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;

/// The system this start runs.
pub const BOOTED: &str = "/run/booted-system";

/// The system the machine starts by default.
pub const DEFAULT: &str = "/nix/var/nix/profiles/system";

/// What a start of a system takes from it.
const STARTS_WITH: [&str; 3] = ["kernel", "initrd", "kernel-params"];

/// What `system` starts with: the files its kernel and initrd are, and its
/// kernel parameters, or nothing when one can't be read.
fn starts_with(system: &Path) -> Option<Vec<String>> {
    STARTS_WITH
        .iter()
        .map(|part| {
            let path = system.join(part);
            if *part == "kernel-params" {
                fs::read_to_string(path).ok()
            } else {
                fs::canonicalize(path)
                    .ok()
                    .map(|file| file.display().to_string())
            }
        })
        .collect()
}

/// Whether the system at `default` starts with another kernel, initrd or
/// kernel parameters than the one at `booted` did. When either can't be
/// read nothing is known to differ, so it doesn't.
pub fn differs(booted: &Path, default: &Path) -> bool {
    match (starts_with(booted), starts_with(default)) {
        (Some(booted), Some(default)) => booted != default,
        _ => false,
    }
}

/// Before an image is made, when `differs` says so, set the next start to
/// this one's entry with `set` (`bootctl set-oneshot`), keeping in `kept`
/// that it did. Says whether it set it.
pub fn hold(
    booted: &Path,
    default: &Path,
    kept: &Path,
    set: impl FnOnce(&str) -> bool,
) -> io::Result<bool> {
    if !differs(booted, default) || !set("@current") {
        return Ok(false);
    }
    fs::write(kept, "@current\n")?;
    Ok(true)
}

/// Once the machine is back, unset the next start's entry `hold` set, if
/// the start hasn't already taken it, and forget it. A hibernation that
/// rolled back never started again, and a later reboot would otherwise
/// take the older entry. Says whether it unset it.
pub fn let_go(kept: &Path, set: impl FnOnce(&str) -> bool) -> io::Result<bool> {
    match fs::read_to_string(kept) {
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    }
    fs::remove_file(kept)?;
    Ok(set(""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;

    /// A folder of its own for a test, holding two systems that start with
    /// the same kernel and parameters and with initrds `booted` and
    /// `default`.
    fn systems(name: &str, booted: &str, default: &str) -> (PathBuf, PathBuf, PathBuf) {
        let folder = std::env::temp_dir().join(format!(
            "hibernate-unlock-oneshot-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(folder.join("store")).unwrap();
        for file in ["kernel", booted, default] {
            fs::write(folder.join("store").join(file), file).unwrap();
        }
        for (system, initrd) in [("booted", booted), ("default", default)] {
            let at = folder.join(system);
            fs::create_dir_all(&at).unwrap();
            symlink(folder.join("store/kernel"), at.join("kernel")).unwrap();
            symlink(folder.join("store").join(initrd), at.join("initrd")).unwrap();
            fs::write(at.join("kernel-params"), "quiet").unwrap();
        }
        (folder.join("booted"), folder.join("default"), folder)
    }

    #[test]
    fn a_new_initrd_by_default_holds_the_next_start_to_this_ones_entry() {
        let (booted, default, folder) = systems("held", "initrd-a", "initrd-b");
        let kept = folder.join("kept");
        let asked = RefCell::new(Vec::new());
        let set = |entry: &str| {
            asked.borrow_mut().push(entry.to_owned());
            true
        };
        assert!(
            hold(&booted, &default, &kept, set).unwrap(),
            "a default starting another initrd didn't hold the next start"
        );
        assert_eq!(
            asked.borrow().as_slice(),
            ["@current"],
            "the next start wasn't set to this one's entry"
        );
        assert!(
            let_go(&kept, |entry| set(entry)).unwrap(),
            "the held start wasn't let go after the sleep"
        );
        assert_eq!(
            asked.borrow().as_slice(),
            ["@current", ""],
            "letting go didn't unset the next start's entry"
        );
        assert!(!kept.exists(), "what was held was kept past letting it go");
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn a_default_starting_as_this_one_did_holds_nothing() {
        let (booted, default, folder) = systems("same", "initrd-a", "initrd-a");
        let kept = folder.join("kept");
        let held = hold(&booted, &default, &kept, |_| {
            panic!("the next start was set though nothing differs")
        });
        assert!(!held.unwrap(), "a default like this start's held the next one");
        assert!(!let_go(&kept, |_| panic!("an unheld start was let go")).unwrap());
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn a_system_that_cant_be_read_holds_nothing() {
        let (booted, _, folder) = systems("unread", "initrd-a", "initrd-b");
        assert!(
            !differs(&booted, &folder.join("missing")),
            "a default that can't be read was taken to differ"
        );
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn other_kernel_parameters_by_default_differ() {
        let (booted, default, folder) = systems("params", "initrd-a", "initrd-a");
        fs::write(default.join("kernel-params"), "quiet splash").unwrap();
        assert!(
            differs(&booted, &default),
            "a default with other kernel parameters was taken as the same"
        );
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn a_next_start_bootctl_refuses_is_not_kept() {
        let (booted, default, folder) = systems("refused", "initrd-a", "initrd-b");
        let kept = folder.join("kept");
        assert!(
            !hold(&booted, &default, &kept, |_| false).unwrap(),
            "a next start bootctl refused was said to be held"
        );
        assert!(!kept.exists(), "a refused hold was kept, to be let go later");
        fs::remove_dir_all(folder).unwrap();
    }
}
