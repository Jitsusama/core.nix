//! The guard's Nix deletes: what it gives `nix store delete`, what it does
//! when paths come alive or another process writes Nix's database, and what
//! it says it did. Apart from the rest of `garbage`'s scenarios so neither
//! file passes 800 lines.

use crate::garbage::script;
use disk_guard::garbage::delete;
use std::fs;
use std::path::Path;

/// A `nix` whose `store delete` refuses the whole call in Nix's words when
/// any path it's given is marked alive, deleting none, as Determinate Nix
/// 3.21 does, and otherwise deletes them all; it notes each time it's run.
/// Like Nix it looks at the paths in order of their names, which in the
/// store is their hashes', and names the first it finds alive. The paths
/// come on its standard input, one a line, and nowhere else.
fn deleting_nix(dir: &Path) -> std::path::PathBuf {
    script(
        &dir.join("nix"),
        &format!(
            "[ \"$1 $2\" = '--extra-experimental-features nix-command' ] || exit 9\nshift 2\n\
             [ \"$*\" = 'store delete --offline --stdin' ] || exit 9\nset -- $(cat)\n\
             echo \"$*\" >> {}\n\
             for p in $(printf '%s\\n' \"$@\" | sort); do\n\
               [ -e \"$p\" ] || {{ echo \"error: path '$p' is not valid\" >&2; exit 1; }}\n\
               if [ -e \"$p.alive\" ]; then\n\
                 echo \"error: Cannot delete path '$p' since it is still alive. To find out \
                 why, use: nix-store --query --roots and nix-store --query --referrers\" >&2\n\
                 exit 1\n\
               fi\n\
             done\n\
             rm -r \"$@\"\n",
            dir.join("runs").display()
        ),
    );
    dir.join("nix")
}

#[test]
fn a_batch_is_deleted_without_what_came_alive_or_is_already_gone() {
    let dir = tempfile::tempdir().unwrap();
    let nix = deleting_nix(dir.path());
    let path = |name: &str| dir.path().join(name).to_str().unwrap().to_owned();
    for name in ["a", "live", "b"] {
        fs::create_dir(path(name)).unwrap();
    }
    fs::write(path("live.alive"), "").unwrap();
    let batch = [path("gone"), path("a"), path("live"), path("b")];
    let deleted = delete(&nix, &dir.path().join("roots"), &batch, false, || false).unwrap();
    assert!(deleted.ran && !deleted.put_off, "{deleted:?}");
    assert!(!Path::new(&path("a")).exists() && !Path::new(&path("b")).exists());
    assert!(Path::new(&path("live")).exists(), "alive, so kept");
    let runs = fs::read_to_string(dir.path().join("runs")).unwrap();
    assert_eq!(
        runs,
        format!(
            "{a} {live} {b}\n{a} {b}\n",
            a = path("a"),
            live = path("live"),
            b = path("b")
        )
    );
}

#[test]
fn a_batch_that_keeps_coming_alive_is_given_up_after_three_refusals() {
    // Each refusal cost Nix a whole look for roots and deleted nothing, so
    // a batch that meets many is left for a pass that makes them afresh.
    let dir = tempfile::tempdir().unwrap();
    let nix = deleting_nix(dir.path());
    let path = |name: &str| dir.path().join(name).to_str().unwrap().to_owned();
    let names = ["a", "b", "c", "d", "e"];
    for name in names {
        fs::create_dir(path(name)).unwrap();
        fs::write(path(&format!("{name}.alive")), "").unwrap();
    }
    let batch: Vec<String> = names.iter().map(|name| path(name)).collect();
    assert_eq!(
        delete(&nix, &dir.path().join("roots"), &batch, false, || false).unwrap_err(),
        "3 paths came alive since the listing"
    );
    let runs = fs::read_to_string(dir.path().join("runs")).unwrap();
    assert_eq!(runs.lines().count(), 4, "{runs}");
}

#[test]
fn what_an_evaluation_begun_since_the_batch_holds_is_never_offered_to_nix() {
    // Each refusal costs Nix a whole look for roots and deletes nothing, and
    // a temporary root read just before costs a folder's read. On 28
    // October every batch of 2,000 derivations met more than three that an
    // evaluation begun since held, and the system guard spent a core
    // giving them up and listing again, freeing nothing for hours.
    let dir = tempfile::tempdir().unwrap();
    let nix = deleting_nix(dir.path());
    let path = |name: &str| dir.path().join(name).to_str().unwrap().to_owned();
    let names = ["p0", "p1", "p2", "p3", "p4", "p5", "p6", "p7", "p8", "p9"];
    let held = ["p1", "p3", "p5", "p7"];
    for name in names {
        fs::create_dir(path(name)).unwrap();
    }
    let roots = dir.path().join("roots");
    fs::create_dir(&roots).unwrap();
    let mut rooted = String::new();
    for name in held {
        fs::write(path(&format!("{name}.alive")), "").unwrap();
        rooted.push_str(&path(name));
        rooted.push('\0');
    }
    fs::write(roots.join("4242"), rooted).unwrap();
    let batch: Vec<String> = names.iter().map(|name| path(name)).collect();
    delete(&nix, &roots, &batch, false, || false).unwrap();
    let left: Vec<&str> = names
        .into_iter()
        .filter(|name| Path::new(&path(name)).exists())
        .collect();
    assert_eq!(left, held, "only the held stay");
    let runs = fs::read_to_string(dir.path().join("runs")).unwrap();
    assert_eq!(runs.lines().count(), 1, "one call, without them: {runs}");
}

#[test]
fn a_delete_that_fails_otherwise_says_why() {
    let dir = tempfile::tempdir().unwrap();
    script(
        &dir.path().join("nix"),
        "echo 'error: unexpected end-of-file' >&2; exit 1\n",
    );
    let real = dir.path().join("x");
    fs::create_dir(&real).unwrap();
    let failed = delete(
        &dir.path().join("nix"),
        &dir.path().join("roots"),
        &[real.to_str().unwrap().to_owned()],
        false,
        || false,
    );
    assert_eq!(
        failed.unwrap_err(),
        "nix failed: error: unexpected end-of-file"
    );
    let nothing = delete(
        &dir.path().join("nix"),
        &dir.path().join("roots"),
        &["/nowhere/at/all".to_owned()],
        false,
        || false,
    )
    .unwrap();
    assert!(nothing.refused.is_empty(), "refused what had gone");
    assert!(!nothing.ran, "ran Nix with nothing left to delete");
}

#[test]
fn a_delete_nix_ran_whose_paths_then_came_alive_says_it_ran() {
    let dir = tempfile::tempdir().unwrap();
    let nix = deleting_nix(dir.path());
    let live = dir.path().join("live").to_str().unwrap().to_owned();
    fs::create_dir(&live).unwrap();
    fs::write(format!("{live}.alive"), "").unwrap();
    let deleted = delete(&nix, &dir.path().join("roots"), &[live], false, || false).unwrap();
    assert!(
        deleted.ran,
        "a delete that cost Nix a run said it never ran"
    );
}

#[test]
fn a_delete_stops_before_running_nix_again_while_another_process_writes() {
    // What comes alive since the batch was made, an evaluation roots, and
    // the same evaluation writes Nix's database.
    let dir = tempfile::tempdir().unwrap();
    let nix = deleting_nix(dir.path());
    let path = |name: &str| dir.path().join(name).to_str().unwrap().to_owned();
    for name in ["a", "live"] {
        fs::create_dir(path(name)).unwrap();
    }
    fs::write(path("live.alive"), "").unwrap();
    let asked = std::cell::Cell::new(0);
    let writing = || {
        asked.set(asked.get() + 1);
        true
    };
    let batch = [path("a"), path("live")];
    let deleted = delete(&nix, &dir.path().join("roots"), &batch, false, writing).unwrap();
    assert!(deleted.put_off && deleted.ran, "{deleted:?}");
    assert!(
        Path::new(&path("a")).exists(),
        "Nix ran again under a writer"
    );
    let runs = fs::read_to_string(dir.path().join("runs")).unwrap();
    assert_eq!(runs.lines().count(), 1, "{runs}");
    assert_eq!(asked.get(), 1, "asked about a writer before the first run");
}

#[test]
fn a_name_holding_a_space_is_never_given_to_nix() {
    // Nix reads its paths one a word, so such a name would reach it as two.
    let dir = tempfile::tempdir().unwrap();
    let nix = deleting_nix(dir.path());
    let path = |name: &str| dir.path().join(name).to_str().unwrap().to_owned();
    for name in ["a", "b c"] {
        fs::create_dir(path(name)).unwrap();
    }
    delete(
        &nix,
        &dir.path().join("roots"),
        &[path("a"), path("b c")],
        false,
        || false,
    )
    .unwrap();
    assert!(
        Path::new(&path("b c")).exists(),
        "a spaced name went to Nix"
    );
    let runs = fs::read_to_string(dir.path().join("runs")).unwrap();
    assert_eq!(runs, format!("{}\n", path("a")));
}
