use super::{SWEEPS_AFTER, land, opened_plain};
use crate::far::MARK;
use crate::far::tests::scratch;
use std::collections::HashSet;
use std::path::Path;

/// The names in `folder`, sorted.
fn names(folder: &Path) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Lands `from` at `into` as a bring-back's first file into its folder.
fn landed(from: &Path, into: &Path) -> std::io::Result<()> {
    land(from, into, &mut HashSet::new())
}

#[test]
fn only_a_plain_file_is_opened_to_land() {
    let root = scratch("opened");
    std::fs::write(root.join("file"), "a").unwrap();
    std::os::unix::fs::symlink(root.join("file"), root.join("link")).unwrap();
    let made = std::process::Command::new("mkfifo")
        .arg(root.join("pipe"))
        .status()
        .unwrap();
    assert!(made.success(), "mkfifo failed");
    assert!(
        opened_plain(&root.join("file")).is_ok(),
        "a plain file wasn't opened"
    );
    assert!(
        opened_plain(&root.join("link")).is_err(),
        "a link was opened"
    );
    // Refused at the look, before any open; one put there after the look
    // is opened without waiting, which no test here can time.
    assert!(
        opened_plain(&root.join("pipe")).is_err(),
        "a pipe was opened"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_file_that_can_t_land_leaves_its_place_as_it_was() {
    let root = scratch("unlanded");
    std::fs::write(root.join("there"), "mine").unwrap();
    std::os::unix::fs::symlink("nowhere", root.join("from")).unwrap();
    assert!(
        landed(&root.join("from"), &root.join("there")).is_err(),
        "a link landed"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("there")).unwrap(),
        "mine",
        "a landing that failed touched what was there"
    );
    assert_eq!(
        names(&root),
        ["from", "there"],
        "a landing that failed left a name of its own behind"
    );
    let _ = std::fs::remove_dir_all(&root);
    // Failing only at its last step, a folder holding a file where it
    // would go, once its own name was made and written.
    let root = scratch("unrenamed");
    std::fs::write(root.join("from"), "new").unwrap();
    std::fs::create_dir(root.join("there")).unwrap();
    std::fs::write(root.join("there/kept"), "mine").unwrap();
    assert!(
        landed(&root.join("from"), &root.join("there")).is_err(),
        "a file landed over a folder"
    );
    assert_eq!(
        names(&root),
        ["from", "there"],
        "a landing that failed at its rename left its own name behind"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_file_lands_under_a_long_name_without_its_special_bits() {
    let root = scratch("long");
    std::fs::write(root.join("from"), "a").unwrap();
    // Setuid where it may be set, which Nix's build sandbox refuses, and a
    // write for anyone either way.
    let allow = |bits| {
        std::fs::set_permissions(
            root.join("from"),
            std::os::unix::fs::PermissionsExt::from_mode(bits),
        )
    };
    allow(0o4777).or_else(|_| allow(0o777)).unwrap();
    let long = "n".repeat(250);
    landed(&root.join("from"), &root.join(&long)).expect("a long name didn't land");
    let mode = std::os::unix::fs::PermissionsExt::mode(
        &std::fs::metadata(root.join(&long)).unwrap().permissions(),
    );
    // What the umask leaves a new file asking for every bit, so the test
    // holds whatever umask runs it.
    let probe = root.join("probe");
    std::os::unix::fs::OpenOptionsExt::mode(
        std::fs::OpenOptions::new().write(true).create_new(true),
        0o777,
    )
    .open(&probe)
    .unwrap();
    let allowed =
        std::os::unix::fs::PermissionsExt::mode(&std::fs::metadata(&probe).unwrap().permissions())
            & 0o777;
    assert_eq!(
        mode & 0o7777,
        0o775 & allowed,
        "it landed with setuid or a write for anyone, or past the umask: {mode:o}"
    );
    // What the umask and the sandbox hide above, said outright.
    assert_eq!(
        super::landing_bits(0o7777),
        0o775,
        "a landing keeps setuid, setgid, sticky or a write for anyone"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_landing_name_a_killed_run_left_is_cleared() {
    let root = scratch("stale");
    std::fs::write(root.join("from"), "new").unwrap();
    let left = |pid: &str| root.join(format!("{MARK}-{pid}-0"));
    // Two left long ago by runs killed as they landed, one under this
    // process's own id, as a pid comes round again; one a live run holds;
    // one just made, which a live run may not have locked yet; and the
    // person's own, named like none of them.
    let own = std::process::id().to_string();
    for path in [
        left("1"),
        left(&own),
        left("2"),
        left("3"),
        root.join(".elsewhere-notes"),
    ] {
        std::fs::write(path, "old").unwrap();
    }
    let long_ago = std::time::SystemTime::now() - SWEEPS_AFTER * 2;
    for pid in ["1", &own, "2"] {
        let file = std::fs::File::options()
            .write(true)
            .open(left(pid))
            .unwrap();
        file.set_modified(long_ago).unwrap();
    }
    let held = std::fs::File::open(left("2")).unwrap();
    held.try_lock().unwrap();
    landed(&root.join("from"), &root.join("into")).expect("a stale name stopped it");
    assert_eq!(
        std::fs::read_to_string(root.join("into")).unwrap(),
        "new",
        "what landed wasn't what came back"
    );
    assert!(
        !left("1").exists() && !left(&own).exists(),
        "a name a killed run left stayed: {:?}",
        names(&root)
    );
    assert!(left("2").exists(), "a live run's landing was swept");
    assert!(left("3").exists(), "a landing just made was swept");
    assert!(
        root.join(".elsewhere-notes").exists(),
        "the person's own file was swept"
    );
    drop(held);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_folder_is_swept_once_a_bring_back() {
    let root = scratch("once");
    std::fs::write(root.join("from"), "new").unwrap();
    let mut swept = HashSet::new();
    land(&root.join("from"), &root.join("a"), &mut swept).unwrap();
    // Left after the first landing, so only a second sweep would take it.
    let left = root.join(format!("{MARK}-1-0"));
    std::fs::write(&left, "old").unwrap();
    let file = std::fs::File::options().write(true).open(&left).unwrap();
    file.set_modified(std::time::SystemTime::now() - SWEEPS_AFTER * 2)
        .unwrap();
    land(&root.join("from"), &root.join("b"), &mut swept).unwrap();
    assert!(left.exists(), "a folder already swept was swept again");
    assert_eq!(swept.len(), 1, "a folder swept was counted twice");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_young_name_left_under_this_process_s_id_is_passed_over() {
    let root = scratch("young");
    std::fs::write(root.join("from"), "new").unwrap();
    // A run killed moments ago under this process's id, as a pid comes
    // round again, left the names this one would take next.
    let next = super::NEXT_LANDING.load(std::sync::atomic::Ordering::Relaxed);
    let own = std::process::id();
    let left: Vec<_> = (next..next + 32)
        .map(|number| root.join(format!("{MARK}-{own}-{number}")))
        .collect();
    for path in &left {
        std::fs::write(path, "old").unwrap();
    }
    landed(&root.join("from"), &root.join("into")).expect("a young leftover stopped it");
    assert_eq!(
        std::fs::read_to_string(root.join("into")).unwrap(),
        "new",
        "what landed wasn't what came back"
    );
    assert!(
        left.iter().all(|path| path.exists()),
        "a young leftover was taken: {:?}",
        names(&root)
    );
    let _ = std::fs::remove_dir_all(&root);
}
