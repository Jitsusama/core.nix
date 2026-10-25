use disk_guard::garbage::{Batch, Dead, Listing, delete, list, parse, temp_roots};
use disk_guard::units::float;
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
             [ \"$1 $2 $3\" = 'path-info --json --offline' ] || exit 9\nshift 3\n\
             echo $# >> {}\n\
             for p in \"$@\"; do case \"$p\" in */.*)\n\
               echo \"error: getting status of \\\"$p\\\": No such file or directory\" >&2; exit 1;;\n\
             esac; done\n\
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
fn a_reference_into_a_later_chunk_is_held_by_place() {
    let dir = tempfile::tempdir().unwrap();
    let nix = fake_nix(dir.path(), 0);
    script(
        &dir.path().join("nix-store"),
        "[ \"$*\" = '--gc --print-dead' ] || exit 9\n\
         i=0; while [ $i -lt 1500 ]; do echo /nix/store/p$i; i=$((i+1)); done\n",
    );
    // The first path refers to the last, a chunk later, and to a live one;
    // the second went before it was described.
    script(
        &nix,
        "[ \"$1 $2\" = '--extra-experimental-features nix-command' ] || exit 9\nshift 2\n\
         [ \"$1 $2 $3\" = 'path-info --json --offline' ] || exit 9\nshift 3\n\
         printf '{'; sep=''\n\
         for p in \"$@\"; do refs='[]'\n\
           if [ \"$p\" = /nix/store/p1 ]; then printf '%s\"%s\":null' \"$sep\" \"$p\"; sep=,; continue; fi\n\
           [ \"$p\" = /nix/store/p0 ] && refs='[\"/nix/store/p1499\",\"/nix/store/p1\",\"/nix/store/live\"]'\n\
           printf '%s\"%s\":{\"narSize\":1,\"registrationTime\":5,\"references\":%s}' \"$sep\" \"$p\" \"$refs\"\n\
           sep=,\n\
         done\n\
         printf '}'\n",
    );
    let dead = list(&nix, false).unwrap();
    assert_eq!(dead.len(), 1499);
    assert_eq!(dead[1498].path, "/nix/store/p1499");
    assert_eq!(dead[0].refers, [1498]);
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

#[test]
fn what_isnt_a_store_path_in_the_store_is_not_described() {
    // Nix keeps temporary files in the store's directory, which the
    // listing names among the dead and which are gone a moment later.
    let dir = tempfile::tempdir().unwrap();
    let nix = fake_nix(dir.path(), 0);
    script(
        &dir.path().join("nix-store"),
        "[ \"$*\" = '--gc --print-dead' ] || exit 9\n\
         echo /nix/store/p0\n\
         echo /nix/store/.tmp-link-2410616-37796915\n\
         echo /nix/store/p1\n",
    );
    let dead = list(&nix, false).unwrap();
    let paths: Vec<&str> = dead.iter().map(|dead| dead.path.as_str()).collect();
    assert_eq!(paths, ["/nix/store/p0", "/nix/store/p1"]);
}

/// A `nix` whose `store delete` refuses the whole call in Nix's words when
/// any path it's given is marked alive, deleting none, as Determinate Nix
/// 3.21 does, and otherwise deletes them all; it notes each time it's run.
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
    delete(&nix, &batch, false).unwrap();
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
        delete(&nix, &batch, false).unwrap_err(),
        "3 paths came alive since the listing"
    );
    let runs = fs::read_to_string(dir.path().join("runs")).unwrap();
    assert_eq!(runs.lines().count(), 4, "{runs}");
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

/// The batches a listing of `all` makes, all of it still there.
fn batches(all: &[Dead], now: f64, keep: f64, budget: u64, most: usize) -> Vec<Batch> {
    Listing::new(0.0, all.to_vec()).batches(|_| true, now, keep, budget, most)
}

#[test]
fn a_listing_holds_each_path_s_references_among_the_dead_by_place() {
    // Only a dead reference can hold a path back, and a place costs far
    // less than a name: a store's listing holds 80,000 paths.
    let listing = Listing::new(
        5.0,
        vec![
            dead("app", 1.0, 1, &["lib", "live-glibc", "app"]),
            dead("lib", 2.0, 1, &["live-glibc"]),
        ],
    );
    crate::assert_exactly(listing.listed_at, 5.0);
    assert_eq!(listing.dead[0].refers, [1]);
    assert_eq!(listing.dead[1].refers, Vec::<u32>::new());
    assert_eq!(listing.dead[1].path, store("lib"));
}

#[test]
fn what_a_temporary_root_holds_is_held_back_with_all_it_reaches() {
    // An evaluation holds the derivation it wrote, which refers to the
    // derivation it builds from, so Nix would refuse both. What refers to
    // the held one, and what's apart from it, can still go.
    let listing = Listing::new(
        0.0,
        vec![
            dead("app.drv", 1.0, 1, &["lib.drv"]),
            dead("lib.drv", 2.0, 1, &["source"]),
            dead("source", 3.0, 1, &[]),
            dead("wrapper.drv", 1.0, 1, &["app.drv"]),
            dead("apart", 1.0, 1, &[]),
        ],
    );
    let held = std::collections::HashSet::from([store("app.drv"), store("elsewhere")]);
    let mut back: Vec<&str> = listing.held_back(&held).into_iter().collect();
    back.sort_unstable();
    assert_eq!(back, [store("app.drv"), store("lib.drv"), store("source")]);
}

#[test]
fn temporary_roots_are_read_from_each_process_s_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("101"), "/nix/store/a\0/nix/store/b\0").unwrap();
    // A file the process is still writing ends with a path cut short.
    fs::write(dir.path().join("102"), "/nix/store/c\0/nix/st").unwrap();
    fs::write(dir.path().join("103"), "").unwrap();
    let mut roots: Vec<String> = temp_roots(dir.path()).into_iter().collect();
    roots.sort_unstable();
    assert_eq!(
        roots,
        ["/nix/st", "/nix/store/a", "/nix/store/b", "/nix/store/c"]
    );
    assert!(temp_roots(&dir.path().join("nowhere")).is_empty());
}

#[test]
fn runtime_roots_are_the_store_paths_each_process_has_in_hand() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    let proc = dir.path().join("proc");
    let link = |from: &Path, to: &Path| {
        fs::create_dir_all(from.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(to, from).unwrap();
    };
    // A test VM's virtiofsd, with files of two paths open, one of them a
    // derivation.
    link(&proc.join("101/fd/3"), &store.join("aaa-lib/lib/libz.so"));
    link(&proc.join("101/fd/4"), &store.join("bbb-app.drv"));
    link(
        &proc.join("101/exe"),
        &store.join("ccc-virtiofsd/bin/virtiofsd"),
    );
    link(&proc.join("101/cwd"), Path::new("/home/someone"));
    fs::write(
        proc.join("101/maps"),
        format!(
            "7f00-7f01 r-xp 00000000 00:1f 42    {}\n7f02-7f03 rw-p 00000000 00:00 0\n",
            store.join("ddd-glibc/lib/libc.so.6").display()
        ),
    )
    .unwrap();
    // Not a process, so not read.
    link(&proc.join("self/fd/3"), &store.join("eee-mine"));
    let mut roots: Vec<String> = disk_guard::garbage::runtime_roots(&proc, &store)
        .into_iter()
        .collect();
    roots.sort_unstable();
    let named = |name: &str| store.join(name).to_string_lossy().into_owned();
    assert_eq!(
        roots,
        [
            named("aaa-lib"),
            named("bbb-app.drv"),
            named("ccc-virtiofsd"),
            named("ddd-glibc")
        ]
    );
    assert!(disk_guard::garbage::runtime_roots(&dir.path().join("nowhere"), &store).is_empty());
}

#[test]
fn what_is_gone_is_left_out_and_holds_nothing_back() {
    // A new app's referrer went since the listing, so the old library is
    // dated by itself again.
    let listing = Listing::new(
        0.0,
        vec![dead("app", 1.0, 1, &["lib"]), dead("lib", 30.0, 1, &[])],
    );
    let made = listing.batches(|dead| !dead.path.ends_with("app"), NOW, KEEP, u64::MAX, 100);
    assert_eq!(names(&made), [(vec!["lib".to_owned()], false)]);
    crate::assert_exactly(made[0].newest, NOW - 30.0 * DAY);
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
    crate::assert_exactly(batches[0].newest, NOW - 10.0 * DAY);
}

#[test]
fn garbage_is_as_young_as_the_newest_garbage_referring_to_it() {
    // A path goes only with or after all that refer to it, so an old
    // library a new build uses is as young as the build.
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
    crate::assert_exactly(batches[0].newest, NOW - DAY);
}

#[test]
fn old_garbage_goes_after_the_garbage_referring_to_it() {
    // Nix refuses to delete a path while a referrer it wasn't given is
    // still there, and a path's date is its newest referrer's, so the two
    // share it: the referrer goes first whatever the names say.
    let all = [
        dead("a-lib", 30.0, 1, &[]),
        dead("z-app", 20.0, 1, &["a-lib"]),
    ];
    assert_eq!(
        names(&batches(&all, NOW, KEEP, 0, 1)),
        [
            (vec!["z-app".to_owned()], false),
            (vec!["a-lib".to_owned()], false)
        ]
    );
}

/// A generator of seeded draws, so a failing case can be run again.
struct Draw(u64);

impl Draw {
    fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % n
    }

    /// A draw below `n`, as an index or a count.
    fn index(&mut self, n: usize) -> usize {
        usize::try_from(self.below(n as u64)).unwrap()
    }
}

#[test]
fn property_every_batch_holds_or_follows_the_garbage_referring_to_it() {
    for seed in 1..=2000u64 {
        let mut draw = Draw(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
        let count = 1 + draw.index(40);
        // Few distinct days, so dates tie as one build's paths do.
        let all: Vec<Dead> = (0..count)
            .map(|at| {
                let references: Vec<String> = (0..draw.below(4))
                    .map(|_| draw.index(count))
                    .filter(|&reference| reference > at)
                    .map(|reference| format!("p{:02}", (reference * 37) % 100))
                    .collect();
                let references: Vec<&str> = references.iter().map(String::as_str).collect();
                dead(
                    &format!("p{:02}", (at * 37) % 100),
                    float(draw.below(6)),
                    1 + draw.below(5),
                    &references,
                )
            })
            .collect();
        let budget = draw.below(60);
        let most = 1 + draw.index(5);
        let made = batches(&all, NOW, KEEP, budget, most);
        let batch_of = |path: &str| {
            made.iter()
                .position(|batch| batch.paths.iter().any(|p| p == path))
                .unwrap_or_else(|| panic!("seed {seed}: {path} is in no batch"))
        };
        let placed: usize = made.iter().map(|batch| batch.paths.len()).sum();
        assert_eq!(placed, all.len(), "seed {seed}: every path once");
        for referrer in &all {
            for reference in &referrer.references {
                if reference == &referrer.path || !all.iter().any(|d| &d.path == reference) {
                    continue;
                }
                let (from, to) = (batch_of(&referrer.path), batch_of(reference));
                assert!(
                    from <= to,
                    "seed {seed}: {reference} in batch {to} before its referrer {} in {from}",
                    referrer.path
                );
            }
        }
        let kept = made.iter().position(|batch| batch.kept);
        if let Some(first) = kept {
            assert!(
                made[first..].iter().all(|batch| batch.kept),
                "seed {seed}: nothing deletable after a kept batch"
            );
        }
    }
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
    crate::assert_exactly(read[0].registered, 1_700_000_000.0);
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
