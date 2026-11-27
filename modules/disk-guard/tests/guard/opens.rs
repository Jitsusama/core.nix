use crate::discover::file;
use crate::pool::{assert_on_btrfs, subvolume};
use disk_guard::opens::{MOST_GROUPS, Seen, Watch};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

/// Everything the watch sees within `for`, once something has been seen.
fn seen(watch: &Watch, wanted: impl Fn(&[Seen]) -> bool, within: Duration) -> Vec<Seen> {
    let deadline = Instant::now() + within;
    let mut all = Vec::new();
    while Instant::now() < deadline && !wanted(&all) {
        all.extend(watch.wait(Duration::from_millis(100)).unwrap());
    }
    all
}

/// A watch. Where the kernel refuses this account one the guard works
/// without it, but these tests then fail, since a test that can't watch
/// proves nothing by passing.
fn watch() -> Watch {
    Watch::new().unwrap_or_else(|error| panic!("no fanotify here, which these tests need: {error}"))
}

#[test]
fn a_watch_answers_the_handle_of_the_folder_it_marked() {
    // The survey trusts a mark only while the folder there now has the
    // handle the watch answered, so the two must name a folder alike.
    let root = tempfile::tempdir().unwrap();
    let deps = root.path().join("deps");
    fs::create_dir(&deps).unwrap();
    let watch = watch();
    let marked = watch.watch(&deps).unwrap();
    assert_eq!(
        marked,
        disk_guard::opens::handle(&deps).unwrap(),
        "the watch answered a handle the survey wouldn't read of the folder it marked"
    );
    fs::remove_dir(&deps).unwrap();
    fs::create_dir(&deps).unwrap();
    assert_ne!(
        marked,
        disk_guard::opens::handle(&deps).unwrap(),
        "a folder made anew where the marked one stood has its handle"
    );
}

#[test]
fn another_process_opening_an_artifact_is_seen_by_its_path() {
    let root = tempfile::tempdir().unwrap();
    let deps = root.path().join("deps");
    let rlib = deps.join("libalpha-1111111111111111.rlib");
    file(&rlib, "built");
    let watch = watch();
    watch.watch(&deps).unwrap();
    let status = Command::new("cat").arg(&rlib).output().unwrap().status;
    assert!(status.success());
    let opened = Seen::Opened(rlib.clone());
    let all = seen(&watch, |all| all.contains(&opened), Duration::from_secs(5));
    assert!(all.contains(&opened), "{all:?}");
}

#[test]
fn the_guards_own_opens_and_unwatched_ones_are_not_seen() {
    let root = tempfile::tempdir().unwrap();
    let deps = root.path().join("deps");
    let elsewhere = root.path().join("elsewhere/libbeta-2222222222222222.rlib");
    file(&deps.join("libalpha-1111111111111111.rlib"), "built");
    file(&elsewhere, "built");
    let watch = watch();
    watch.watch(&deps).unwrap();
    fs::read(deps.join("libalpha-1111111111111111.rlib")).unwrap();
    Command::new("cat").arg(&elsewhere).output().unwrap();
    // Then one it does see, so the wait has an end that proves it read.
    let marker = deps.join("libgamma-3333333333333333.rlib");
    file(&marker, "built");
    Command::new("cat").arg(&marker).output().unwrap();
    let opened = Seen::Opened(marker.clone());
    let all = seen(&watch, |all| all.contains(&opened), Duration::from_secs(5));
    assert_eq!(all, vec![opened]);
}

#[test]
fn opens_beyond_what_the_queue_holds_are_told_as_lost() {
    let root = tempfile::tempdir().unwrap();
    let deps = root.path().join("deps");
    fs::create_dir(&deps).unwrap();
    // An account's queue holds 16,384 events before it drops the rest, and
    // opens of one file merge into one event, so each open is of its own.
    let make = "seq 1 20000 | sed 's/^/lib/; s/$/.rlib/' | xargs touch";
    let run = |script: &str| {
        let ran = Command::new("sh")
            .args(["-c", script])
            .current_dir(&deps)
            .status();
        assert!(ran.unwrap().success(), "{script}");
    };
    run(make);
    let watch = watch();
    watch.watch(&deps).unwrap();
    run("find . -name '*.rlib' -exec cat {} +");
    let all = seen(
        &watch,
        |all| all.contains(&Seen::Lost),
        Duration::from_secs(5),
    );
    assert!(all.contains(&Seen::Lost), "{} seen, none lost", all.len());
}

#[test]
#[ignore = "needs TMPDIR on Btrfs, which the sandbox and the shell's tmpfs aren't: \
            run with --ignored and TMPDIR on a Btrfs folder"]
fn opens_in_builds_of_two_subvolumes_are_both_seen() {
    // A group marks under one filesystem id, and each subvolume has its
    // own, so on 26 November 40 build folders an hour went unmarked with
    // EXDEV, their builds dated by the hour rather than their opens.
    let root = tempfile::tempdir().unwrap();
    assert_on_btrfs(root.path());
    let watch = watch();
    let mut opened = Vec::new();
    for name in ["one", "two"] {
        subvolume(root.path(), name);
        let deps = root.path().join(name).join("deps");
        let rlib = deps.join("libalpha-1111111111111111.rlib");
        file(&rlib, "built");
        if let Err(error) = watch.watch(&deps) {
            panic!("the build in subvolume {name} wasn't marked: {error}");
        }
        opened.push(Seen::Opened(rlib));
    }
    for open in &opened {
        let Seen::Opened(path) = open else {
            unreachable!("only opens were kept")
        };
        Command::new("cat").arg(path).output().unwrap();
    }
    let all = seen(
        &watch,
        |all| opened.iter().all(|open| all.contains(open)),
        Duration::from_secs(5),
    );
    assert!(
        opened.iter().all(|open| all.contains(open)),
        "an open in one subvolume's build went unseen: {all:?}"
    );
    assert_eq!(
        watch.groups(),
        2,
        "two subvolumes' builds weren't heard through a group each"
    );
}

#[test]
#[ignore = "needs TMPDIR on Btrfs, which the sandbox and the shell's tmpfs aren't: \
            run with --ignored and TMPDIR on a Btrfs folder"]
fn a_watch_makes_no_group_past_its_most() {
    // An account may hold 128 groups, and every work tree is a subvolume,
    // so a watch keeps to half and leaves its other programs room.
    let root = tempfile::tempdir().unwrap();
    assert_on_btrfs(root.path());
    let watch = watch();
    for n in 0..MOST_GROUPS {
        let name = format!("tree-{n}");
        subvolume(root.path(), &name);
        if let Err(error) = watch.watch(&root.path().join(&name)) {
            panic!("subvolume {n}, within the most, wasn't marked: {error}");
        }
    }
    subvolume(root.path(), "past");
    let refused = watch.watch(&root.path().join("past"));
    assert!(
        refused
            .as_ref()
            .is_err_and(|error| error.to_string().contains("past the 64")),
        "a subvolume past the most was answered {refused:?}"
    );
    assert_eq!(
        watch.groups(),
        MOST_GROUPS,
        "a group was made past the most"
    );
    watch
        .watch(&root.path().join("tree-0"))
        .expect("a folder under an id already marked was refused once the most was reached");
}

#[test]
fn a_watch_on_what_is_not_a_directory_is_refused() {
    let watch = watch();
    assert!(watch.watch(Path::new("/nonexistent/deps")).is_err());
}
