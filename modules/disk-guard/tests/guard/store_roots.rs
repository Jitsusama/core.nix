//! Links a tool keeps under a folder to hold its builds from collection,
//! as the survey finds them through Nix's indirect roots and offers them.

use crate::discover::{dir, file};
use crate::survey::{DAY, find, offers, real, written_ago};
use disk_guard::activity::Process;
use disk_guard::config::Config;
use disk_guard::plan::Class;
use disk_guard::survey::Action;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

const ENVS: &str = r"^(?P<place>.*/\\.dev)/shell-envs/[^/]+$";

/// A config with one store-roots source over `trees`, whose links Nix's
/// indirect roots in `gcroots/auto` name.
fn config(gcroots: &Path, trees: &Path, pattern: &str) -> Config {
    Config::parse(&format!(
        r#"{{ "version": 1, "scope": "user",
              "nix": {{ "roots": "{}" }},
              "sources": {{ "envs": {{ "kind": "store-roots", "roots": ["{}"],
                                      "pattern": "{pattern}", "unused": "1d" }} }} }}"#,
        gcroots.display(),
        trees.display()
    ))
    .unwrap()
}

/// A link at `path` to `to`, written `ago` seconds ago, which Nix's
/// indirect roots name as `name`.
fn rooted(gcroots: &Path, name: &str, path: &Path, to: &str, ago: f64) -> PathBuf {
    dir(path.parent().unwrap());
    symlink(to, path).unwrap();
    written_ago(path, ago);
    symlink(path, gcroots.join("auto").join(name)).unwrap();
    path.to_owned()
}

/// A tree whose tool last wrote its place `ago` seconds ago, as a
/// development shell's `up` mark is written each time it's entered, with
/// its environment's link written as long ago as the tree's oldest.
fn tree(gcroots: &Path, trees: &Path, name: &str, ago: f64) -> PathBuf {
    let place = dir(&trees.join(name).join(".dev"));
    file(&place.join("up"), "");
    written_ago(&place.join("up"), ago);
    let link = rooted(
        gcroots,
        name,
        &place.join("shell-envs/x86_64-linux"),
        "/nix/store/abc-env",
        3.0 * DAY,
    );
    written_ago(&place.join("shell-envs"), 3.0 * DAY);
    written_ago(&place, ago.max(3.0 * DAY));
    link
}

// On one laptop on 12 November, development shells' links in trees a day
// unused held 9.7 GiB of the store, which the shell makes again when next
// entered. Each is dated by the newest write in its tool's place, so a
// shell entered an hour ago is young however old its link.
#[test]
fn a_tools_link_is_offered_as_a_root_dated_by_its_places_last_write() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let gcroots = dir(&home.join("gcroots"));
    dir(&gcroots.join("auto"));
    let trees = dir(&home.join("trees"));
    let idle = tree(&gcroots, &trees, "idle", 3.0 * DAY);
    let entered = tree(&gcroots, &trees, "entered", 3_600.0);
    let found = offers(&config(&gcroots, &trees, ENVS), &[], &|_| false);
    let offered = find(&found, &idle);
    assert_eq!(offered.candidate.class, Class::StoreRoot { unused: DAY });
    assert!(
        offered.candidate.idle > 2.9 * DAY,
        "dated by its place's last write: {}",
        offered.candidate.idle
    );
    assert_eq!(
        offered.action,
        Action::Entry {
            root: trees.clone()
        }
    );
    let young = find(&found, &entered).candidate.idle;
    assert!(
        young < 2.0 * 3_600.0,
        "its place was written an hour ago: {young}"
    );
}

// A tree a process works in may enter its shell again any moment, so its
// link stays whatever its age.
#[test]
fn a_link_in_a_tree_a_process_works_in_is_not_offered() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let gcroots = dir(&home.join("gcroots"));
    dir(&gcroots.join("auto"));
    let trees = dir(&home.join("trees"));
    tree(&gcroots, &trees, "busy", 3.0 * DAY);
    let working = Process {
        pid: 1,
        uid: unsafe { libc::geteuid() },
        cwd: Some(dir(&trees.join("busy/src"))),
        readable: true,
        ..Process::default()
    };
    let found = offers(&config(&gcroots, &trees, ENVS), &[working], &|_| false);
    assert!(found.is_empty(), "a process works in its tree: {found:?}");
}

// Only a link the source names is its tool's: one under its roots that its
// pattern matches and that still links into the store.
#[test]
fn only_a_link_into_the_store_the_source_names_is_offered() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let gcroots = dir(&home.join("gcroots"));
    dir(&gcroots.join("auto"));
    let trees = dir(&home.join("trees"));
    let elsewhere = dir(&home.join("elsewhere"));
    let named = tree(&gcroots, &trees, "named", 3.0 * DAY);
    tree(&gcroots, &elsewhere, "outside", 3.0 * DAY);
    let result = trees.join("built/result");
    rooted(&gcroots, "result", &result, "/nix/store/abc-app", 3.0 * DAY);
    let moved = trees.join("moved/.dev/shell-envs/x86_64-linux");
    rooted(&gcroots, "moved", &moved, "/home/u/env", 3.0 * DAY);
    let found = offers(&config(&gcroots, &trees, ENVS), &[], &|_| false);
    let paths: Vec<_> = found.iter().map(|offer| &offer.candidate.path).collect();
    assert_eq!(paths, [&named], "only the named link into the store");
}
