//! The near side leaving a job's files in the handover: which tracked
//! names it takes, its archives, the list of them, and the bytes it says
//! they take; and the cache the far line finds that handover in.

use super::handing::pinning;
use super::{Left, far_line, far_run, kind, scratch, work};
use crate::far::{
    Folders, HANDOVER, LARGEST_FILE, LEFT, MOST_BYTES, TREE, archive, gathered, leave, left,
    tracked,
};
use std::path::Path;

#[test]
fn a_name_is_the_tree_s_own_only_through_folders_never_through_a_link() {
    let root = scratch("own-folders");
    std::fs::create_dir_all(root.join("a/b")).unwrap();
    std::fs::write(root.join("file"), "").unwrap();
    // A link to a folder of the tree's own is refused as well: only the
    // folder itself is looked at, never where a link leads.
    std::os::unix::fs::symlink(root.join("a"), root.join("d")).unwrap();
    std::os::unix::fs::symlink(root.join("a/b"), root.join("a/c")).unwrap();
    // Two names are asked twice: a folder passed once still passes, and one
    // refused is refused again, so a cache that kept every folder it looked
    // at, own or not, fails.
    let mut folders = Folders::default();
    for (path, own) in [
        ("top", true),
        ("a/b/f", true),
        ("a/b/f", true),
        ("d/f", false),
        ("d/f", false),
        ("a/c/f", false),
        ("a/c/g", false),
        ("gone/f", false),
        ("file/f", false),
    ] {
        assert_eq!(
            folders.own(&root, path),
            own,
            "{path} read as own: {}",
            !own
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_tracked_name_outside_the_tree_is_refused_though_it_reads_as_hidden() {
    // Even where its kind keeps the name home, so it would otherwise be
    // passed over as an excluded file is.
    for path in ["../x", "a/../b", "./a", "a/.", "/etc/passwd"] {
        let last = [path.rsplit('/').next().unwrap_or(path).to_owned()];
        for excludes in [&[][..], &last] {
            assert!(
                tracked(path, excludes).is_err(),
                "{path:?} was passed over, not refused, past {excludes:?}"
            );
        }
    }
    // A hidden name within the tree stays home without a word, even one
    // no list could carry.
    for path in [".env", "a/.git/config", ".a\tb"] {
        assert_eq!(tracked(path, &[]), Ok(false), "{path:?} wasn't kept home");
    }
    assert_eq!(
        tracked("src/lib.rs", &["lib.rs".to_owned()]),
        Ok(false),
        "an excluded file wasn't kept home"
    );
    // One that travels, but that no list could carry, is refused.
    for path in ["a\tb", "src/a\nb"] {
        assert!(
            tracked(path, &[]).is_err_and(|why| why.contains("a tab or a newline")),
            "{path:?} wasn't refused for what no list carries"
        );
    }
    assert_eq!(
        tracked("src/lib.rs", &[]),
        Ok(true),
        "a source file didn't travel"
    );
}

#[test]
fn what_is_left_is_its_archives_each_listed_at_its_size() {
    let root = scratch("left");
    std::fs::write(root.join("x"), "xyz").unwrap();
    let job = root.join("job");
    let bytes = left(&root, ["x"], &job).unwrap();
    let size = std::fs::metadata(job.join(TREE).join("0.tar"))
        .unwrap()
        .len();
    // A header, the data's block and the two that close it.
    assert_eq!(size, 4 * 512, "one small file's archive came to {size}");
    assert_eq!(bytes, size, "what was left was said to come to {bytes}");
    assert_eq!(
        std::fs::read_to_string(job.join(LEFT)).unwrap(),
        format!("{size}\t0.tar\nend\n"),
        "an archive wasn't listed at the size it was written at"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn what_is_left_holds_each_file_to_what_a_shared_file_may_be() {
    let root = scratch("left-largest");
    // Sparse, so a byte past the most takes no room on the disk.
    std::fs::File::create(root.join("x"))
        .unwrap()
        .set_len(LARGEST_FILE + 1)
        .unwrap();
    let job = root.join("job");
    let refused = left(&root, ["x"], &job);
    // Refused by its size as it is opened. Were the two bounds swapped, it
    // would be read whole and refused as past the total instead.
    let opened = format!(
        "{} is {} bytes, past the {LARGEST_FILE} bytes one shared file may be",
        root.join("x").display(),
        LARGEST_FILE + 1
    );
    assert!(
        refused
            .as_ref()
            .is_err_and(|error| error.to_string() == opened),
        "a file past the most a shared file may be wasn't refused as it was opened: {refused:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A repository at `root` tracking `files`, each sparse at its size, so
/// none takes room on the disk, and tracked without git reading it.
fn tracking(root: &Path, files: &[(String, u64)]) {
    let git = |words: &[&str]| {
        let status = std::process::Command::new("git")
            .args(words)
            .current_dir(root)
            .status()
            .unwrap();
        assert!(status.success(), "git {words:?} failed");
    };
    git(&["init", "--quiet"]);
    for (name, size) in files {
        std::fs::File::create(root.join(name))
            .unwrap()
            .set_len(*size)
            .unwrap();
        // The empty file's blob, which nothing reads: only the names count.
        let entry = format!("100644,e69de29bb2d1d6434b8b29ae775ad8c2e48c5391,{name}");
        git(&["update-index", "--add", "--cacheinfo", &entry]);
    }
}

#[test]
fn a_file_is_gathered_while_its_archive_fits_a_shared_file_and_not_a_byte_past() {
    // A file whose entry and close come to just the most a shared file
    // may be, and one a byte larger, which takes another block.
    let fits = LARGEST_FILE - archive::CLOSED - 512;
    for (size, takes) in [(fits, true), (fits + 1, false)] {
        let root = scratch("gathered-largest");
        tracking(&root, &[("f".to_owned(), size)]);
        let found = gathered(&root, &kind(""));
        if takes {
            assert_eq!(
                found,
                Ok(vec!["f".to_owned()]),
                "a file that fits wasn't gathered"
            );
        } else {
            assert!(
                found
                    .as_ref()
                    .is_err_and(|error| error.starts_with("f is past the 64 MiB")),
                "a file a byte past what fits was gathered: {found:?}"
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[test]
fn files_are_gathered_while_their_archives_fit_a_shared_folder_and_not_a_byte_past() {
    // Fifteen files each fill an archive's room, and a sixteenth takes
    // what comes to just what a shared folder carries with the closes
    // `archived` counts at worst, 33 for 16 rooms' worth, not the 16
    // these files make; a byte more takes another block.
    let full = LARGEST_FILE - archive::CLOSED - 512;
    let last = LARGEST_FILE - 18 * archive::CLOSED - 512;
    let count = 16;
    let taken = 15 * archive::taken("00", full) + archive::taken("15", last);
    assert_eq!(
        archive::archived(taken, LARGEST_FILE),
        MOST_BYTES,
        "the files no longer come to just what a shared folder carries"
    );
    for (size, takes) in [(last, true), (last + 1, false)] {
        let root = scratch("gathered-most");
        let files: Vec<_> = (0..count)
            .map(|at| (format!("{at:02}"), if at + 1 < count { full } else { size }))
            .collect();
        tracking(&root, &files);
        let found = gathered(&root, &kind(""));
        if takes {
            assert_eq!(
                found.map(|found| found.len()),
                Ok(count),
                "files within what a shared folder carries weren't all gathered"
            );
        } else {
            let past = format!("past the {MOST_BYTES} a shared folder carries");
            assert!(
                found.as_ref().is_err_and(|error| {
                    error.starts_with(&format!("its {count} files")) && error.ends_with(&past)
                }),
                "files past what a shared folder carries were gathered: {found:?}"
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[test]
fn what_is_left_over_several_archives_is_said_to_take_them_all() {
    let root = scratch("left-several");
    let sizes = [("a", 600), ("b", 1), ("c", 1500)];
    for (name, size) in sizes {
        std::fs::write(root.join(name), vec![b'x'; size]).unwrap();
    }
    let job = root.join("job");
    // Room for the largest file's entry and the close, so each goes in its
    // own archive, each of a size of its own.
    let most = archive::taken("c", 1500) + archive::CLOSED;
    let bytes = leave(&root, sizes.map(|(name, _)| name), &job, most, u64::MAX).unwrap();
    let parts: Vec<u64> = ["0.tar", "1.tar", "2.tar"]
        .iter()
        .map(|name| std::fs::metadata(job.join(TREE).join(name)).unwrap().len())
        .collect();
    assert_eq!(
        bytes,
        parts.iter().sum::<u64>(),
        "archives of {parts:?} bytes were said to take {bytes}"
    );
    assert_eq!(
        std::fs::read_to_string(job.join(LEFT)).unwrap(),
        format!(
            "{}\t0.tar\n{}\t1.tar\n{}\t2.tar\nend\n",
            parts[0], parts[1], parts[2]
        ),
        "the archives weren't each listed at the size it was written at"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn both_sides_take_a_cache_or_home_that_isn_t_absolute_as_none() {
    use crate::far::steps::{CACHE, cache};
    use std::ffi::OsStr;
    use std::path::PathBuf;
    let line = format!("{CACHE}\nprintf %s \"$cache\"");
    // The gate names dash, the strictest sh a far machine may have.
    let sh = std::env::var("ELSEWHERE_FAR_SH").unwrap_or_else(|_| "sh".to_owned());
    for (xdg, home, found) in [
        (Some("rel/cache"), Some("/h"), "/h/.cache"),
        (Some(""), Some("/h"), "/h/.cache"),
        (None, Some("/h"), "/h/.cache"),
        (Some("/c"), Some("/h"), "/c"),
        (Some("/c"), None, "/c"),
        (None, Some(""), ""),
        (None, Some("rel"), ""),
        (Some("rel/cache"), None, ""),
    ] {
        let mut far = std::process::Command::new(&sh);
        far.args(["-c", &line]);
        for (name, value) in [("XDG_CACHE_HOME", xdg), ("HOME", home)] {
            match value {
                Some(value) => far.env(name, value),
                None => far.env_remove(name),
            };
        }
        let said = far.output().unwrap().stdout;
        let here = cache(xdg.map(OsStr::new), home.map(OsStr::new));
        assert_eq!(
            (String::from_utf8_lossy(&said).into_owned(), here),
            (
                found.to_owned(),
                Some(found).filter(|it| !it.is_empty()).map(PathBuf::from)
            ),
            "a cache of {xdg:?} and a home of {home:?} weren't found where the line finds them"
        );
    }
}

#[test]
fn a_far_machine_with_no_cache_leaves_the_job_untaken_as_its_own_fault() {
    use crate::far::{Heard, UNTAKEN, script};
    let command = ["true"].map(str::to_owned);
    let line = script(
        "QEST-20261115-ABC123",
        "1",
        0,
        ".s",
        Path::new(""),
        &kind(""),
        &command,
    );
    // The gate names dash, the strictest sh a far machine may have.
    let sh = std::env::var("ELSEWHERE_FAR_SH").unwrap_or_else(|_| "sh".to_owned());
    // Run where a relative home would put its cache, so one made there
    // shows.
    let at = scratch("no-cache");
    let far = std::process::Command::new(sh)
        .args(["-c", &line])
        .current_dir(&at)
        .env("HOME", "rel")
        .env_remove("XDG_CACHE_HOME")
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&far.stdout);
    let why = String::from_utf8_lossy(&far.stderr);
    let machine = std::process::Command::new("uname")
        .arg("-n")
        .output()
        .unwrap();
    let machine = String::from_utf8_lossy(&machine.stdout);
    // The wait for files that never come also ends untaken, so only the
    // guard's own word shows the guard ended it, before any wait.
    assert_eq!(
        (
            far.status.code(),
            said.lines().any(|it| it == Heard::Untaken.line("1")),
            why.lines().map(str::to_owned).collect::<Vec<_>>(),
        ),
        (
            Some(i32::from(UNTAKEN)),
            true,
            vec![format!(
                "elsewhere: no cache is found on {}",
                machine.trim_end()
            )]
        ),
        "a far machine with no cache didn't leave the job untaken by its guard: {said}"
    );
    assert_eq!(
        std::fs::read_dir(&at).unwrap().count(),
        0,
        "a far machine with no cache made something where its line ran"
    );
    let _ = std::fs::remove_dir_all(&at);
}

thread_local! {
    /// Whether `far_line_in` runs the line with a `HOME` that names no
    /// folder, its cache named by `XDG_CACHE_HOME`, as `home_gone` sets it.
    pub(super) static HOME_GONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// What `run` answers, with each far line it runs given a `HOME` that
/// names no folder.
fn home_gone<T>(run: impl FnOnce() -> T) -> T {
    /// Clears `HOME_GONE` as it drops, so a `run` that panics leaves no
    /// home gone for the tests its thread runs after.
    struct Back;
    impl Drop for Back {
        fn drop(&mut self) {
            HOME_GONE.set(false);
        }
    }
    HOME_GONE.set(true);
    let _back = Back;
    run()
}

// A cache named by XDG_CACHE_HOME is a cache though the home it names is
// gone, as a service account's may be, so the job runs from it, and its
// copy and run's file are freed from it too: stepping into that home
// would fail and keep them, so nothing the line clears steps there.
#[test]
fn a_far_machine_whose_home_is_gone_still_frees_its_copy() {
    let (code, err, home) = home_gone(|| far_run(Left::Whole, &[("x", "x")], "true"));
    assert!(
        code == Some(0) && !work(&home).join("1").exists() && !work(&home).join("1.run").exists(),
        "a far machine whose home is gone didn't run the job or kept its copy ({code:?}): {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

// As above for a far line stopped by a signal, whose clearing steps out
// of the copy before it takes it away.
#[test]
fn a_far_machine_whose_home_is_gone_still_frees_its_copy_once_stopped() {
    let (code, err, home) = home_gone(|| far_run(Left::Whole, &[("x", "x")], "kill -TERM $PPID"));
    let signal = std::fs::read_to_string(home.join("signal")).unwrap();
    assert!(
        signal == "15"
            && !home.join("gone").exists()
            && !work(&home).join("1").exists()
            && !work(&home).join("1.run").exists(),
        "a far machine whose home is gone wasn't stopped or kept its copy ({code:?}): {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

// As above for a far line that brings nothing back past a link at its
// handover it can't clear, which steps out of the copy before it takes
// it away. The link leads beside the home that's gone, never into it, so
// that home stays gone.
#[test]
fn a_far_machine_whose_home_is_gone_still_frees_its_copy_past_a_link_it_can_t_clear() {
    let job = format!(
        "mkdir out && printf a > out/a && beside=${{XDG_CACHE_HOME%/*}} &&
         mkdir \"$beside/beyond\" && mv ../../{HANDOVER}/1 \"$beside/moved\" &&
         ln -s \"$beside/beyond\" ../../{HANDOVER}/1"
    );
    let (rm, refused) = pinning();
    let programs = [(rm, refused.as_str())];
    let (code, err, home) =
        home_gone(|| far_line(Left::Whole, &[("x", "x")], &job, &kind(""), None, &programs));
    assert!(
        code == Some(0)
            && err.contains("a link it left at its handover couldn't be cleared, so nothing")
            && !home.join("gone").exists()
            && !work(&home).join("1").exists()
            && !work(&home).join("1.run").exists(),
        "a far machine whose home is gone didn't run the job past a link it can't clear, or \
         kept its copy ({code:?}): {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}
