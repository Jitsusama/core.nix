use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use std::ffi::OsStr;

use std::io;

use super::{Folder, Folders, Heard, Of, SPENT, Told, Watch, because, folders, told, unwatchable};

/// A folder of the test's own, emptied first, so no two tests share one.
fn place(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("repos-heard-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a folder");
    dir
}

/// A checkout's git folder as git leaves it, written by hand: HEAD, its
/// refs a folder deep, and one tree in `worktrees/`.
fn checkout(at: &Path, name: &str) -> PathBuf {
    let git = at.join(name).join(".git");
    for folder in [
        "refs/heads/joel",
        "refs/remotes/origin",
        "worktrees/say-trees",
        "objects",
    ] {
        fs::create_dir_all(git.join(folder)).expect("a git folder");
    }
    fs::write(git.join("HEAD"), "ref: refs/heads/main\n").expect("HEAD");
    fs::write(git.join("refs/heads/main"), "aaaa\n").expect("a branch");
    fs::write(
        git.join("worktrees/say-trees/HEAD"),
        "ref: refs/heads/joel/x\n",
    )
    .expect("HEAD");
    git
}

/// Watches `folders`, telling what moved on what's returned.
fn hear(folders: &Folders) -> (Heard, mpsc::Receiver<BTreeSet<Of>>) {
    let (tell, told) = mpsc::channel();
    let mut heard = Heard::start(tell).expect("inotify");
    heard.follow(folders, |_| true);
    (heard, told)
}

#[test]
fn a_checkout_is_watched_by_its_git_folder_refs_and_trees_never_its_objects() {
    let at = place("folders");
    let git = checkout(&at, "core.nix");
    let kept = [
        at.join("state/github.json"),
        at.join("state/upstreams.json"),
    ];
    let named = folders([&git], &kept);
    let its = |folder| Watch {
        folder,
        of: Of::Checkout(git.clone()),
    };
    let expected = [
        (git.clone(), its(Folder::Git)),
        (git.join("refs"), its(Folder::Refs)),
        (git.join("refs/heads"), its(Folder::Refs)),
        (git.join("refs/heads/joel"), its(Folder::Refs)),
        (git.join("refs/remotes"), its(Folder::Refs)),
        (git.join("refs/remotes/origin"), its(Folder::Refs)),
        (git.join("worktrees"), its(Folder::Trees)),
        (git.join("worktrees/say-trees"), its(Folder::Git)),
        (
            at.join("state"),
            Watch {
                folder: Folder::Kept(BTreeSet::from([
                    "github.json".to_owned(),
                    "upstreams.json".to_owned(),
                ])),
                of: Of::Kept,
            },
        ),
    ];
    assert_eq!(named.into_iter().collect::<Vec<_>>(), expected);
    fs::remove_dir_all(&at).expect("cleaned");
}

#[test]
fn only_the_names_a_reading_reads_are_heeded() {
    // A tree's own folder in `worktrees/` is a git folder too, whose `gitdir`
    // says where the tree is, as `git worktree move` and `repair` write it.
    for name in ["HEAD", "packed-refs", "config", "worktrees", "gitdir"] {
        assert!(Folder::Git.heeds(name), "{name} is heeded");
    }
    for name in [
        "index",
        "FETCH_HEAD",
        "ORIG_HEAD",
        "COMMIT_EDITMSG",
        "HEAD.lock",
        "objects",
    ] {
        assert!(!Folder::Git.heeds(name), "{name} isn't heeded");
    }
    assert!(Folder::Refs.heeds("main"), "a branch's ref is heeded");
    assert!(
        !Folder::Refs.heeds("main.lock"),
        "a ref's lock isn't heeded, only its rename"
    );
    assert!(
        Folder::Reftable.heeds("tables.list"),
        "a reftable's list is heeded"
    );
    assert!(
        !Folder::Reftable.heeds("0x01-0x02-1234.ref"),
        "a reftable's tables aren't heeded, only the list naming them"
    );
    let kept = Folder::Kept(BTreeSet::from(["github.json".to_owned()]));
    assert!(kept.heeds("github.json"), "a kept record is heeded");
    assert!(
        !kept.heeds(".github.json.new"),
        "a kept record's copy before its rename isn't heeded"
    );
}

#[test]
fn a_branch_landing_is_told_as_its_checkouts_and_an_index_written_is_not() {
    let at = place("landing");
    let git = checkout(&at, "core.nix");
    let other = checkout(&at, "notes");
    let (_heard, told) = hear(&folders([&git, &other], &[]));

    fs::write(git.join("index"), "written at a status").expect("an index");
    assert!(
        told.recv_timeout(Duration::from_millis(300)).is_err(),
        "an index written isn't heard"
    );

    // As git moves a branch: its lock written, then renamed into place.
    fs::write(git.join("refs/heads/main.lock"), "bbbb\n").expect("a lock");
    fs::rename(
        git.join("refs/heads/main.lock"),
        git.join("refs/heads/main"),
    )
    .expect("landed");
    let moved = told
        .recv_timeout(Duration::from_secs(5))
        .expect("a branch landing is heard");
    assert_eq!(moved, BTreeSet::from([Of::Checkout(git.clone())]));
    fs::remove_dir_all(&at).expect("cleaned");
}

#[test]
fn a_queue_overflowing_tells_every_watch_held_since_what_it_dropped_is_unknown() {
    let core = Watch {
        folder: Folder::Git,
        of: Of::Checkout(PathBuf::from("/src/core.nix/.git")),
    };
    let world = Watch {
        folder: Folder::Refs,
        of: Of::Checkout(PathBuf::from("/src/world/.git")),
    };
    let kept = Watch {
        folder: Folder::Kept(BTreeSet::from(["github.json".to_owned()])),
        of: Of::Kept,
    };
    let held = [&core, &world, &kept];

    let quiet = told([Told::Moved(&core, Some(OsStr::new("index")))], held);
    assert_eq!(quiet, BTreeSet::new(), "the index written is not heeded");
    let overflowed = told(
        [
            Told::Moved(&core, Some(OsStr::new("index"))),
            Told::Overflowed,
        ],
        held,
    );
    assert_eq!(
        overflowed,
        BTreeSet::from([core.of.clone(), world.of.clone(), Of::Kept]),
        "an overflow tells every checkout held and the timers' files"
    );
}

#[test]
fn a_kept_file_written_is_told_as_the_timers() {
    let at = place("kept");
    fs::create_dir_all(at.join("state")).expect("a state folder");
    let (_heard, told) = hear(&folders([], &[at.join("state/github.json")]));

    fs::write(at.join("state/github.json"), "{}").expect("kept");
    let moved = told
        .recv_timeout(Duration::from_secs(5))
        .expect("a kept file is heard");
    assert_eq!(moved, BTreeSet::from([Of::Kept]));
    fs::remove_dir_all(&at).expect("cleaned");
}

#[test]
fn a_folder_of_refs_made_after_is_heard_once_its_checkout_is_renewed() {
    let at = place("followed");
    let git = checkout(&at, "core.nix");
    let (mut heard, told) = hear(&folders([&git], &[]));
    let watched = heard.watched();

    fs::create_dir_all(git.join("refs/heads/new")).expect("a folder of refs");
    assert!(
        told.recv_timeout(Duration::from_secs(5)).is_ok(),
        "the folder's making is heard"
    );
    heard.follow(&folders([&git], &[]), |of| *of == Of::Checkout(git.clone()));
    assert_eq!(heard.watched(), watched + 1, "the new folder is watched");
    while told.try_recv().is_ok() {}

    fs::write(git.join("refs/heads/new/branch"), "cccc\n").expect("a branch in it");
    assert!(
        told.recv_timeout(Duration::from_secs(5)).is_ok(),
        "a branch in the new folder is heard"
    );
    fs::remove_dir_all(&at).expect("cleaned");
}

#[test]
fn a_folder_made_again_in_its_place_is_heard_once_renewed() {
    let at = place("made-again");
    let git = checkout(&at, "core.nix");
    let (mut heard, told) = hear(&folders([&git], &[]));

    fs::remove_dir_all(git.join("refs/heads/joel")).expect("a folder of refs gone");
    fs::create_dir_all(git.join("refs/heads/joel")).expect("and made again");
    assert!(
        told.recv_timeout(Duration::from_secs(5)).is_ok(),
        "heard going"
    );
    heard.follow(&folders([&git], &[]), |_| true);
    while told.try_recv().is_ok() {}

    fs::write(git.join("refs/heads/joel/y"), "dddd\n").expect("a branch in it");
    assert!(
        told.recv_timeout(Duration::from_secs(5)).is_ok(),
        "a branch in the folder made again is heard"
    );
    fs::remove_dir_all(&at).expect("cleaned");
}

#[test]
fn a_folder_that_cant_be_watched_is_counted_until_its_watched_or_no_longer_named() {
    let at = place("refused");
    let git = checkout(&at, "core.nix");
    let mut named = folders([&git], &[]);
    let gone = at.join("core.nix/.git/refs/gone");
    named.insert(
        gone.clone(),
        Watch {
            folder: Folder::Refs,
            of: Of::Checkout(git.clone()),
        },
    );
    let (mut heard, _told) = hear(&named);
    assert_eq!(heard.unwatched(), 1, "a folder gone before it's watched");
    let because = heard.unwatched_because().unwrap_or_default();
    assert!(
        because.starts_with(&format!("{}: ", gone.display())),
        "why it went unwatched doesn't name it: {because}"
    );

    fs::create_dir_all(&gone).expect("made after all");
    heard.follow(&named, |_| true);
    assert_eq!(heard.unwatched(), 0, "a folder watched at last");
    let watched = heard.watched();

    // Renewed once it's gone, its watch fails: it's counted once, as
    // unwatched, its old watch given back.
    fs::remove_dir_all(&gone).expect("gone again");
    heard.follow(&named, |_| true);
    assert_eq!(
        (heard.watched(), heard.unwatched()),
        (watched - 1, 1),
        "a folder that failed its renewal is counted twice"
    );
    named.remove(&gone);
    let missing = at.join("core.nix/.git/refs/never");
    named.insert(
        missing.clone(),
        Watch {
            folder: Folder::Refs,
            of: Of::Checkout(git.clone()),
        },
    );
    heard.follow(&named, |_| true);
    assert_eq!(heard.unwatched(), 1);
    named.remove(&missing);
    heard.follow(&named, |_| true);
    assert_eq!(heard.unwatched(), 0, "a folder no longer named");
    assert_eq!(heard.unwatched_because(), None);
    fs::remove_dir_all(&at).expect("cleaned");
}

#[test]
fn the_timers_folder_before_their_first_write_is_nothing_unheard() {
    let at = place("unkept");
    let git = checkout(&at, "core.nix");
    let kept = at.join("state/repos");
    let named = folders([&git], &[kept.join("github.json")]);
    let (mut heard, _told) = hear(&named);
    assert_eq!(
        (heard.unwatched(), heard.unwatched_because()),
        (0, None),
        "a timers' folder not yet made said as unheard"
    );
    let watched = heard.watched();
    fs::create_dir_all(&kept).expect("the timers' first write");
    heard.follow(&named, |_| true);
    assert_eq!(
        heard.watched(),
        watched + 1,
        "the timers' folder unwatched once made"
    );
    fs::remove_dir_all(&at).expect("cleaned");
}

#[test]
fn spent_watches_are_said_for_what_they_are_before_any_other_reason() {
    let spent = unwatchable(
        Path::new("/home/joel/src/b/.git/refs"),
        &io::Error::from(io::ErrorKind::StorageFull),
    );
    assert_eq!(
        spent,
        format!("/home/joel/src/b/.git/refs: {SPENT}"),
        "spent watches said as a full disk"
    );
    let gone = "/home/joel/src/a/.git/refs: No such file or directory (os error 2)";
    assert_eq!(
        because([gone, spent.as_str()].into_iter()),
        Some(spent.as_str()),
        "a folder gone said before spent watches"
    );
    assert_eq!(
        because([gone].into_iter()),
        Some(gone),
        "the only reason not said"
    );
}

#[test]
fn a_folder_no_longer_named_is_given_back() {
    let at = place("given-back");
    let git = checkout(&at, "core.nix");
    let (mut heard, told) = hear(&folders([&git], &[]));
    heard.follow(&Folders::new(), |_| true);
    assert_eq!(heard.watched(), 0);
    while told.try_recv().is_ok() {}

    fs::write(git.join("HEAD"), "ref: refs/heads/other\n").expect("HEAD");
    assert!(
        told.recv_timeout(Duration::from_millis(300)).is_err(),
        "a checkout no longer read isn't heard"
    );
    fs::remove_dir_all(&at).expect("cleaned");
}
