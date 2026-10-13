use disk_guard::garbage::{Batch, Dead, batches, delete, list, parse};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Writes an executable shell script and waits until it can be run. A test
/// thread that forked while it was being written holds it open for writing
/// until its child execs, and running it before then fails with "Text file
/// busy". Once it runs, no writer is left. Every script here refuses an
/// argument it doesn't know before doing anything.
pub(crate) fn script(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\n{body}")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    for _ in 0..500 {
        let tried = std::process::Command::new(path)
            .arg("--are-you-busy")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        match tried {
            Err(error) if error.raw_os_error() == Some(libc::ETXTBSY) => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            _ => return,
        }
    }
    panic!("{} stayed busy", path.display());
}

/// A `nix-store` that lists `count` dead paths, and a `nix` that tells of
/// each it's asked about and notes how many it was asked at once.
fn fake_nix(dir: &Path, count: usize) -> std::path::PathBuf {
    script(
        &dir.join("nix-store"),
        &format!(
            "[ \"$*\" = '--gc --print-dead' ] || exit 9\n\
             echo 'finding garbage collector roots...' >&2\n\
             i=0; while [ $i -lt {count} ]; do echo /nix/store/p$i; i=$((i+1)); done\n"
        ),
    );
    script(
        &dir.join("nix"),
        &format!(
            "[ \"$1 $2\" = '--extra-experimental-features nix-command' ] || exit 9\nshift 2\n\
             [ \"$1 $2\" = 'path-info --json' ] || exit 9\nshift 2\n\
             echo $# >> {}\n\
             printf '{{'; sep=''\n\
             for p in \"$@\"; do\n\
               printf '%s\"%s\":{{\"narSize\":1,\"registrationTime\":5,\"references\":[]}}' \"$sep\" \"$p\"\n\
               sep=,\n\
             done\n\
             printf '}}'\n",
            dir.join("asked").display()
        ),
    );
    dir.join("nix")
}

#[test]
fn the_dead_are_listed_and_described_in_chunks_the_kernel_takes() {
    let dir = tempfile::tempdir().unwrap();
    let nix = fake_nix(dir.path(), 2500);
    let dead = list(&nix, false).unwrap();
    assert_eq!(dead.len(), 2500);
    assert!(
        dead.iter()
            .any(|path| path.path == "/nix/store/p2499" && path.size == 1)
    );
    let asked = fs::read_to_string(dir.path().join("asked")).unwrap();
    assert_eq!(asked, "1000\n1000\n500\n");
}

/// A `nix` whose `store delete` deletes each path in turn, as Nix does,
/// until one marked alive, which it refuses in Nix's words; it notes
/// each time it's run.
fn deleting_nix(dir: &Path) -> std::path::PathBuf {
    script(
        &dir.join("nix"),
        &format!(
            "[ \"$1 $2\" = '--extra-experimental-features nix-command' ] || exit 9\nshift 2\n\
             [ \"$1 $2\" = 'store delete' ] || exit 9\nshift 2\n\
             echo \"$*\" >> {}\n\
             for p in \"$@\"; do\n\
               [ -e \"$p\" ] || {{ echo \"error: path '$p' is not valid\" >&2; exit 1; }}\n\
               if [ -e \"$p.alive\" ]; then\n\
                 echo \"error: Cannot delete path '$p' since it is still alive. To find out \
                 why, use: nix-store --query --roots and nix-store --query --referrers\" >&2\n\
                 exit 1\n\
               fi\n\
               rm -r \"$p\"\n\
             done\n",
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
    delete(&nix, &batch, false).unwrap();
    assert!(!Path::new(&path("a")).exists() && !Path::new(&path("b")).exists());
    assert!(Path::new(&path("live")).exists(), "alive, so kept");
    let runs = fs::read_to_string(dir.path().join("runs")).unwrap();
    assert_eq!(
        runs,
        format!(
            "{a} {live} {b}\n{b}\n",
            a = path("a"),
            live = path("live"),
            b = path("b")
        )
    );
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
        &[real.to_str().unwrap().to_owned()],
        false,
    );
    assert_eq!(
        failed.unwrap_err(),
        "nix failed: error: unexpected end-of-file"
    );
    assert_eq!(
        delete(
            &dir.path().join("nix"),
            &["/nowhere/at/all".to_owned()],
            false
        ),
        Ok(())
    );
}

#[test]
fn a_listing_that_fails_says_why() {
    let dir = tempfile::tempdir().unwrap();
    script(
        &dir.path().join("nix-store"),
        "echo 'error: cannot connect to the daemon' >&2; exit 1\n",
    );
    let failed = list(&dir.path().join("nix"), false).unwrap_err();
    assert!(failed.contains("cannot connect to the daemon"), "{failed}");
    let none = tempfile::tempdir().unwrap();
    assert_eq!(list(&fake_nix(none.path(), 0), false).unwrap(), []);
}

const DAY: f64 = 86400.0;
const NOW: f64 = 100.0 * DAY;
const KEEP: f64 = 3.0 * DAY;

/// A dead path registered `days` ago, of `size` bytes, referring to
/// `references` by name.
fn dead(name: &str, days: f64, size: u64, references: &[&str]) -> Dead {
    Dead {
        path: store(name),
        registered: NOW - days * DAY,
        size,
        references: references.iter().map(|name| store(name)).collect(),
    }
}

fn store(name: &str) -> String {
    format!("/nix/store/{name}")
}

/// Each batch's paths by name, and whether healthy keeps it.
fn names(batches: &[Batch]) -> Vec<(Vec<String>, bool)> {
    batches
        .iter()
        .map(|batch| {
            let names = batch
                .paths
                .iter()
                .map(|path| path.trim_start_matches("/nix/store/").to_owned())
                .collect();
            (names, batch.kept)
        })
        .collect()
}

#[test]
fn old_garbage_goes_oldest_first_in_batches() {
    let all = [
        dead("b", 10.0, 1, &[]),
        dead("a", 20.0, 1, &[]),
        dead("c", 5.0, 1, &[]),
    ];
    let batches = batches(&all, NOW, KEEP, u64::MAX, 2);
    assert_eq!(
        names(&batches),
        [
            (vec!["a".to_owned(), "b".to_owned()], false),
            (vec!["c".to_owned()], false)
        ]
    );
    assert_eq!(batches[0].size, 2);
    assert_eq!(batches[0].newest, NOW - 10.0 * DAY);
}

#[test]
fn garbage_is_as_young_as_the_newest_garbage_referring_to_it() {
    // Deleting a path deletes all that refer to it, so an old library a
    // new build uses is as young as the build.
    let all = [
        dead("lib", 30.0, 1, &[]),
        dead("app", 1.0, 1, &["lib", "live-glibc"]),
    ];
    let batches = batches(&all, NOW, KEEP, u64::MAX, 100);
    assert_eq!(
        names(&batches),
        [(vec!["app".to_owned(), "lib".to_owned()], true)],
        "kept while healthy, the referrer first"
    );
    assert_eq!(batches[0].newest, NOW - DAY);
}

#[test]
fn young_garbage_over_its_budget_goes_largest_first() {
    let all = [
        dead("small", 1.0, 10, &[]),
        dead("image", 2.0, 100, &["tool"]),
        dead("tool", 2.0, 120, &[]),
        dead("toolchain", 1.0, 50, &[]),
    ];
    // 280 bytes of young garbage against a budget of 70. The largest,
    // tool, waits for the image that refers to it.
    let batches = batches(&all, NOW, KEEP, 70, 100);
    assert_eq!(
        names(&batches),
        [
            (vec!["image".to_owned(), "tool".to_owned()], false),
            (vec!["toolchain".to_owned(), "small".to_owned()], true),
        ],
        "a path goes only once nothing left refers to it"
    );
}

#[test]
fn a_large_path_goes_first_with_the_small_ones_referring_to_it() {
    // A disk image, and the little link to it that a test run left.
    let all = [
        dead("image", 1.0, 100, &[]),
        dead("link", 1.0, 1, &["image"]),
        dead("medium", 1.0, 50, &[]),
    ];
    assert_eq!(
        names(&batches(&all, NOW, KEEP, 60, 100)),
        [
            (vec!["link".to_owned(), "image".to_owned()], false),
            (vec!["medium".to_owned()], true),
        ]
    );
}

#[test]
fn a_batch_closes_once_it_holds_enough_to_measure() {
    use disk_guard::garbage::BATCH_BYTES;
    let all = [
        dead("a", 30.0, BATCH_BYTES - 1, &[]),
        dead("b", 20.0, 1, &[]),
        dead("c", 10.0, 1, &[]),
    ];
    assert_eq!(
        names(&batches(&all, NOW, KEEP, 0, 100)),
        [
            (vec!["a".to_owned(), "b".to_owned()], false),
            (vec!["c".to_owned()], false)
        ]
    );
}

#[test]
fn a_path_referring_to_itself_is_still_garbage() {
    let all = [dead("self", 10.0, 1, &["self"])];
    assert_eq!(
        names(&batches(&all, NOW, KEEP, 0, 100)),
        [(vec!["self".to_owned()], false)]
    );
}

#[test]
fn nix_path_info_is_read_for_what_each_dead_path_weighs_and_refers_to() {
    let json = r#"{
        "/nix/store/aaa-app": {"ca": null, "narSize": 4096, "registrationTime": 1700000000,
            "references": ["/nix/store/bbb-lib", "/nix/store/aaa-app"], "version": 1},
        "/nix/store/bbb-lib": {"narSize": 10, "registrationTime": 1600000000, "references": []},
        "/nix/store/ccc-gone": null
    }"#;
    let mut read = parse(json).unwrap();
    read.sort_by(|a, b| a.path.cmp(&b.path));
    assert_eq!(read.len(), 2, "a path gone since the listing is left out");
    assert_eq!(read[0].path, "/nix/store/aaa-app");
    assert_eq!(read[0].size, 4096);
    assert_eq!(read[0].registered, 1_700_000_000.0);
    assert_eq!(
        read[0].references,
        ["/nix/store/bbb-lib", "/nix/store/aaa-app"]
    );
    assert!(
        parse("[]").is_err(),
        "the older list shape isn't guessed at"
    );
    assert!(parse(r#"{"/nix/store/x": {"narSize": 1}}"#).is_err());
}
