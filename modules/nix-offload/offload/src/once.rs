//! One copy of a store path to a lent builder at a time from this
//! machine. On 27 November several runs sent the same 66 MB flake source
//! to the same builder at once, over the one uplink they shared. The
//! builder's daemon locks a path while one copy adds it, so the others
//! moved nothing but their ssh keepalives until their deadlines, and
//! failed. Now a copy waits for the one already sending its paths to that
//! builder, then sends only what the builder still lacks, since `nix copy`
//! skips what's already there.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How often a waiter looks again at a path another copy is sending.
const LOOKED_AT_EVERY: Duration = Duration::from_millis(250);

/// The most paths one copy holds the locks of, each an open file: a
/// closure of thousands would otherwise reach the account's limit of
/// 1,024 open files. Past it, the rest go without one.
const MOST_HELD: usize = 128;

/// The locks a copy holds on the paths it sends, each let go when this
/// is dropped, as the copy ends.
pub struct Sending {
    held: Vec<fs::File>,
}

impl Sending {
    /// How many of its paths this copy holds the lock of.
    #[cfg(test)]
    fn holds(&self) -> usize {
        self.held.len()
    }
}

/// Takes, in `folder`, the lock of each of `paths` that is a store path,
/// for sending to `host`. A derivation is left out, since it's a file
/// of a few kilobytes whose second copy costs nothing, and past
/// [`MOST_HELD`] the rest are. Each is taken in the order of its name, so
/// two copies never each hold one the other waits on. A lock another copy
/// holds is waited on, said once, for at most `within` all told. A path
/// whose lock can't be had by then, or can't be opened, is sent without
/// one, and that is said too. As a guard on load, it fails open rather
/// than holding a copy back for ever.
#[must_use]
pub fn sending_once(
    folder: &Path,
    host: &str,
    paths: &[&str],
    within: Duration,
    say: &mut dyn FnMut(String),
) -> Sending {
    let mut sending = Sending { held: Vec::new() };
    let keys: BTreeSet<&str> = paths
        .iter()
        .filter(|path| !path.ends_with(".drv"))
        .filter_map(|path| store_key(path))
        .collect();
    if keys.is_empty() {
        return sending;
    }
    let to = folder.join("copying").join(host_key(host));
    if let Err(error) = fs::create_dir_all(&to) {
        say(format!(
            "nix-offload: {} can't be made ({error}), so this copy may send what another \
             already sends to {host}",
            to.display()
        ));
        return sending;
    }
    let began = Instant::now();
    let mut waited = false;
    let mut gave_up = false;
    for key in keys.into_iter().take(MOST_HELD) {
        let got = taken(&to, key, &mut || {
            if gave_up || began.elapsed() >= within {
                gave_up = true;
                return false;
            }
            if !waited {
                waited = true;
                say(format!(
                    "nix-offload: another copy from this machine already sends {} to {host}, \
                     so this waits for it rather than sending it twice",
                    named(paths, key)
                ));
            }
            true
        });
        match got {
            Ok(Some(lock)) => sending.held.push(lock),
            Ok(None) => {}
            Err(error) => say(format!(
                "nix-offload: the lock of {} to {host} can't be taken ({error}), so it goes \
                 without one",
                named(paths, key)
            )),
        }
    }
    if gave_up {
        say(format!(
            "nix-offload: another copy still sends to {host} after {}, so this sends what it \
             lacks too",
            crate::there::lasting(began.elapsed())
        ));
    } else if waited {
        say(format!(
            "nix-offload: the other copy to {host} ended after {}, so this sends only what \
             {host} still lacks",
            crate::there::lasting(began.elapsed())
        ));
    }
    sending
}

/// The lock of the store path whose hash is `key`, in `to`, taken as
/// soon as it's free while `wait` says to go on waiting; none once it
/// says to stop.
fn taken(
    to: &Path,
    key: &str,
    wait: &mut dyn FnMut() -> bool,
) -> std::io::Result<Option<fs::File>> {
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(to.join(key))?;
    loop {
        match lock.try_lock() {
            Ok(()) => return Ok(Some(lock)),
            Err(fs::TryLockError::WouldBlock) => {
                if !wait() {
                    return Ok(None);
                }
                std::thread::sleep(LOOKED_AT_EVERY);
            }
            Err(fs::TryLockError::Error(error)) => return Err(error),
        }
    }
}

/// The hash that names the store path `path`, which its lock is kept
/// under, or none for anything else, such as a path with a folder of its
/// own below it, which `nix copy` would send as the store path above.
fn store_key(path: &str) -> Option<&str> {
    let name = path.strip_prefix("/nix/store/")?;
    let (hash, rest) = name.split_at_checked(32)?;
    let hashed = hash
        .bytes()
        .all(|byte| byte.is_ascii_digit() || byte.is_ascii_lowercase());
    (hashed && rest.starts_with('-') && !rest.contains('/')).then_some(hash)
}

/// The store path among `paths` whose hash is `key`, to say which waits.
fn named<'a>(paths: &[&'a str], key: &str) -> &'a str {
    paths
        .iter()
        .find(|path| store_key(path) == Some(key))
        .copied()
        .unwrap_or_default()
}

/// The name of `host`'s folder of locks: the host as ssh is given it,
/// with anything but a letter, a digit, a dot, a dash or an underscore
/// put as an underscore, so a user, a port or a slash can't reach
/// another folder, and with none of them, a name all its own.
fn host_key(host: &str) -> PathBuf {
    let kept: String = host
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if kept.is_empty() || kept.starts_with('.') {
        PathBuf::from(format!("host{kept}"))
    } else {
        PathBuf::from(kept)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "/nix/store/b5ahx7k4fgzx6cpya7ajd4nmbrfx1r7c-source";
    const OTHER: &str = "/nix/store/1nlcy0y7ifjj73b3bvinxc182dk8zh51-source";

    #[test]
    fn a_second_copy_of_a_path_waits_for_the_first() {
        let folder = tempfile::tempdir().expect("a folder");
        let mut said = Vec::new();
        let first = sending_once(
            folder.path(),
            "more",
            &[SOURCE],
            Duration::from_secs(5),
            &mut |line| {
                said.push(line);
            },
        );
        assert!(
            first.holds() == 1 && said.is_empty(),
            "the first copy didn't take its lock alone: {said:?}"
        );
        let at = folder.path().to_owned();
        let second = std::thread::spawn(move || {
            let mut said = Vec::new();
            let sending = sending_once(
                &at,
                "more",
                &[SOURCE, OTHER],
                Duration::from_secs(5),
                &mut |line| {
                    said.push(line);
                },
            );
            (sending.holds(), said)
        });
        std::thread::sleep(Duration::from_millis(600));
        assert!(
            !second.is_finished(),
            "the second copy didn't wait for the first"
        );
        drop(first);
        let (holds, said) = second.join().expect("the second copy");
        assert!(
            holds == 2
                && said.len() == 2
                && said[0].contains(SOURCE)
                && said[0].contains("waits for it rather than sending it twice")
                && said[1].contains("sends only what more still lacks"),
            "the second copy didn't say it waited, then take both: {holds}, {said:?}"
        );
    }

    #[test]
    fn copies_of_other_paths_or_to_other_builders_go_at_once() {
        let folder = tempfile::tempdir().expect("a folder");
        let mut said = Vec::new();
        let within = Duration::from_secs(5);
        let first = sending_once(folder.path(), "more", &[SOURCE], within, &mut |line| {
            said.push(line);
        });
        let other = sending_once(folder.path(), "more", &[OTHER], within, &mut |line| {
            said.push(line);
        });
        let elsewhere = sending_once(folder.path(), "lend", &[SOURCE], within, &mut |line| {
            said.push(line);
        });
        assert!(
            first.holds() + other.holds() + elsewhere.holds() == 3 && said.is_empty(),
            "a copy that shares no path and builder with another waited: {said:?}"
        );
    }

    #[test]
    fn a_wait_past_its_limit_sends_anyway() {
        let folder = tempfile::tempdir().expect("a folder");
        let first = sending_once(
            folder.path(),
            "more",
            &[SOURCE],
            Duration::from_secs(5),
            &mut |_| {},
        );
        let began = Instant::now();
        let mut said = Vec::new();
        let second = sending_once(
            folder.path(),
            "more",
            &[SOURCE, OTHER],
            Duration::from_millis(300),
            &mut |line| {
                said.push(line);
            },
        );
        assert!(
            began.elapsed() < Duration::from_secs(3),
            "the wait ran on past its limit: {:?}",
            began.elapsed()
        );
        assert!(
            second.holds() == 1
                && said.len() == 2
                && said[1].contains("still sends to more after")
                && said[1].contains("sends what it lacks too"),
            "a copy past its wait didn't say it sends anyway, or held what it waited on: {said:?}"
        );
        drop(first);
    }

    #[test]
    fn past_its_most_a_copy_holds_no_more_locks() {
        let folder = tempfile::tempdir().expect("a folder");
        let many: Vec<String> = (0..MOST_HELD + 5)
            .map(|n| format!("/nix/store/{n:0>32}-out"))
            .collect();
        let paths: Vec<&str> = many.iter().map(String::as_str).collect();
        let sending = sending_once(
            folder.path(),
            "more",
            &paths,
            Duration::from_secs(1),
            &mut |_| {},
        );
        assert!(
            sending.holds() == MOST_HELD,
            "a copy of {} paths held {} locks",
            paths.len(),
            sending.holds()
        );
    }

    #[test]
    fn only_store_paths_are_locked_and_hosts_stay_in_their_folder() {
        let folder = tempfile::tempdir().expect("a folder");
        let mut said = Vec::new();
        let odd = [
            "relative",
            "/nix/store/b5ahx7k4fgzx6cpya7ajd4nmbrfx1r7c-source/inner",
            "/nix/store/B5AHX7K4FGZX6CPYA7AJD4NMBRFX1R7C-source",
            "/nix/store/short-source",
            "/nix/store/1nlcy0y7ifjj73b3bvinxc182dk8zh51-source.drv",
        ];
        let none = sending_once(
            folder.path(),
            "more",
            &odd,
            Duration::from_secs(1),
            &mut |line| said.push(line),
        );
        assert!(
            none.holds() == 0 && said.is_empty() && !folder.path().join("copying").exists(),
            "a path that isn't a store path was locked: {said:?}"
        );
        let sending = sending_once(
            folder.path(),
            "../root@builder:22/x",
            &[SOURCE],
            Duration::from_secs(1),
            &mut |line| said.push(line),
        );
        let made: Vec<_> = fs::read_dir(folder.path().join("copying"))
            .expect("the copying folder")
            .map(|entry| entry.expect("an entry").file_name())
            .collect();
        assert!(
            sending.holds() == 1 && made == ["host.._root_builder_22_x"],
            "the host's folder wasn't kept within the copying folder: {made:?}"
        );
    }
}
