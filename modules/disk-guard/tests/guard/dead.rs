use crate::garbage::{LISTING, script};
use disk_guard::garbage::list;
use std::fs;
use std::path::{Path, PathBuf};

/// A store under `dir` holding `entries`, each a folder, and a `nix` and
/// `nix-store` beside it. `nix` says Nix's `keep-outputs` and
/// `keep-derivations` from files of those names, `false` unless a test
/// writes otherwise, and describes each path it's asked about.
/// `nix-store` prints the roots in `roots`, or fails while `roots-fail`
/// stands, and as it reads them a path no root names arrives in the store,
/// `zzzz-new`, which a listing reading the store first never sees. It
/// calls the paths in `invalid` not valid and refuses to say what they
/// keep or what built them, says each path keeps itself and what
/// `kept/<name>` lists, gives each path's deriver from `derivers/<name>`,
/// and marks a run of the locked listing in `locked`. While `fail-once`
/// stands, its first question of what paths keep fails with the first
/// of them gone, as a root a build let go of is; while `gone-after`
/// names a path, that path goes once what paths keep has been said.
fn unlocked_nix(dir: &Path, entries: &[&str]) -> (PathBuf, PathBuf) {
    let store = dir.join("store");
    for entry in entries {
        fs::create_dir_all(store.join(entry)).unwrap();
    }
    fs::create_dir_all(dir.join("kept")).unwrap();
    fs::create_dir_all(dir.join("derivers")).unwrap();
    for (file, words) in [
        ("roots", ""),
        ("invalid", ""),
        ("keep-outputs", "false\n"),
        ("keep-derivations", "false\n"),
    ] {
        fs::write(dir.join(file), words).unwrap();
    }
    let at = dir.display();
    script(
        &dir.join("nix-store"),
        &format!(
            "case \"$1 $2\" in\n\
             '--gc --print-roots') [ -e {at}/roots-fail ] && exit 1\n\
               mkdir -p {at}/store/zzzz-new; cat {at}/roots ;;\n\
             '--check-validity --print-invalid') shift 2\n\
               for p in \"$@\"; do grep -qxF \"$p\" {at}/invalid && echo \"$p\"; done; true ;;\n\
             '--query --requisites') shift 2\n\
               [ -e {at}/fail-once ] && {{\n\
                 rm {at}/fail-once; echo \"$1\" >> {at}/invalid\n\
                 echo 'error: gone' >&2; exit 1; }}\n\
               for p in \"$@\"; do\n\
                 grep -qxF \"$p\" {at}/invalid &&\n\
                   {{ echo \"error: $p is not valid\" >&2; exit 1; }}\n\
                 echo \"$p\"; [ -f \"{at}/kept/${{p##*/}}\" ] && cat \"{at}/kept/${{p##*/}}\"\n\
               done\n\
               [ -f {at}/gone-after ] && cat {at}/gone-after >> {at}/invalid; true ;;\n\
             '--query --deriver') shift 2\n\
               for p in \"$@\"; do\n\
                 grep -qxF \"$p\" {at}/invalid &&\n\
                   {{ echo \"error: path '$p' is not valid\" >&2; exit 1; }}\n\
                 if [ -f \"{at}/derivers/${{p##*/}}\" ]; then cat \"{at}/derivers/${{p##*/}}\";\n\
                 else echo unknown-deriver; fi\n\
               done ;;\n\
             *) [ \"$*\" = '{LISTING}' ] || exit 9; touch {at}/locked ;;\n\
             esac\n"
        ),
    );
    script(
        &dir.join("nix"),
        &format!(
            "[ \"$1 $2\" = '--extra-experimental-features nix-command' ] || exit 9\nshift 2\n\
             if [ \"$1 $2\" = 'config show' ]; then\n\
               case \"$3\" in keep-outputs|keep-derivations) cat {at}/$3; exit 0 ;; esac; exit 9\n\
             fi\n\
             [ \"$1 $2 $3\" = 'path-info --json --offline' ] || exit 9\nshift 3\n\
             printf '{{'; sep=''\n\
             for p in \"$@\"; do\n\
               printf '%s\"%s\":{{\"narSize\":1,\"registrationTime\":5,\"references\":[]}}' \"$sep\" \"$p\"\n\
               sep=,\n\
             done\n\
             printf '}}'\n"
        ),
    );
    (dir.join("nix"), store)
}

/// The names of the dead the listing gives, without their store's folder.
fn listed_names(nix: &Path, store: &Path) -> Vec<String> {
    list(nix, store, false)
        .unwrap()
        .into_iter()
        .map(|dead| dead.path.rsplit('/').next().unwrap().to_owned())
        .collect()
}

#[test]
fn the_dead_are_worked_out_without_nixs_lock_where_roots_keep_only_their_closures() {
    // Nix's `--print-dead` holds the lock every other collection waits
    // on for as long as it runs: on 24 November a laptop's ran 24 minutes
    // while builds waited on it. With neither keep- setting on, what's
    // live is the roots' closure, which Nix reads without that lock.
    let dir = tempfile::tempdir().unwrap();
    let (nix, store) = unlocked_nix(
        dir.path(),
        &[
            "aaaa-root",
            "bbbb-needed",
            "cccc-dead",
            "dddd-held-by-a-file",
            "eeee-building",
            ".links",
            "tmp-1234-5678",
        ],
    );
    let s = store.display();
    fs::write(
        dir.path().join("roots"),
        format!(
            "/home/someone/result -> {s}/aaaa-root\n\
             {{temp:42}} -> {s}/eeee-building\n\
             {{censored}} -> {s}/dddd-held-by-a-file/bin/tool\n\
             /proc/1/maps -> /elsewhere/tool\n"
        ),
    )
    .unwrap();
    fs::write(
        dir.path().join("kept/aaaa-root"),
        format!("{s}/bbbb-needed\n"),
    )
    .unwrap();
    // A path a build is making is a root before Nix holds it valid.
    fs::write(dir.path().join("invalid"), format!("{s}/eeee-building\n")).unwrap();
    assert_eq!(
        listed_names(&nix, &store),
        ["cccc-dead"],
        "only what no root keeps, in its closure or by a file within it, is dead"
    );
    assert!(
        !dir.path().join("locked").exists(),
        "the listing ran Nix's locked collector"
    );
}

#[test]
fn a_root_gone_while_asking_what_it_keeps_is_asked_about_again() {
    let dir = tempfile::tempdir().unwrap();
    let (nix, store) = unlocked_nix(dir.path(), &["aaaa-root", "bbbb-needed", "cccc-dead"]);
    let s = store.display();
    fs::write(dir.path().join("roots"), format!("/r -> {s}/aaaa-root\n")).unwrap();
    fs::write(
        dir.path().join("kept/aaaa-root"),
        format!("{s}/bbbb-needed\n"),
    )
    .unwrap();
    fs::write(dir.path().join("fail-once"), "").unwrap();
    // Asked again, the root is no longer valid, so it keeps only itself.
    assert_eq!(
        listed_names(&nix, &store),
        ["bbbb-needed", "cccc-dead"],
        "a root gone mid-question is asked about again, its validity first"
    );
}

#[test]
fn a_root_reading_with_no_root_in_the_store_fails_the_listing() {
    // Every entry would read dead: a Nix printing its roots another way,
    // or a store named otherwise than Nix names it, finds none. A machine
    // that runs always has some, its own system's at least.
    let dir = tempfile::tempdir().unwrap();
    let (nix, store) = unlocked_nix(dir.path(), &["aaaa-system", "cccc-dead"]);
    fs::write(
        dir.path().join("roots"),
        "/proc/1/maps -> /elsewhere/tool\nsomething Nix said otherwise\n",
    )
    .unwrap();
    let listed = list(&nix, &store, false);
    assert!(
        listed.as_ref().is_err_and(|why| why.contains("no root")),
        "a store with no root read listed as dead: {listed:?}"
    );
    assert!(
        !dir.path().join("locked").exists(),
        "it fell back to Nix's locked collector instead of failing"
    );
}

#[test]
fn a_listing_fails_closed_when_the_store_or_its_roots_cant_be_read() {
    let dir = tempfile::tempdir().unwrap();
    let (nix, store) = unlocked_nix(dir.path(), &["cccc-dead"]);
    let s = store.display();
    fs::write(dir.path().join("roots"), format!("/r -> {s}/cccc-dead\n")).unwrap();
    fs::write(dir.path().join("roots-fail"), "").unwrap();
    assert!(
        list(&nix, &store, false).is_err(),
        "a root reading that failed still listed"
    );
    let missing = dir.path().join("elsewhere");
    let listed = list(&nix, &missing, false);
    assert!(
        listed
            .as_ref()
            .is_err_and(|why| why.contains(&missing.display().to_string())),
        "a store folder that isn't there listed: {listed:?}"
    );
    assert!(
        !dir.path().join("locked").exists(),
        "a failure fell back to Nix's locked collector"
    );
}

#[test]
fn a_path_gone_before_its_deriver_is_asked_for_costs_the_listing_nothing() {
    // With no lock held, Nix's own collection may delete what a finished
    // build let go of while the walk goes on, and Nix refuses to say what
    // built a path no longer valid.
    let dir = tempfile::tempdir().unwrap();
    let (nix, store) = unlocked_nix(
        dir.path(),
        &["aaaa-out", "bbbb-temp", "cccc-dead", "ffff-out.drv"],
    );
    let s = store.display();
    fs::write(dir.path().join("keep-derivations"), "true\n").unwrap();
    fs::write(
        dir.path().join("roots"),
        format!("/r -> {s}/aaaa-out\n{{temp:7}} -> {s}/bbbb-temp\n"),
    )
    .unwrap();
    fs::write(
        dir.path().join("derivers/aaaa-out"),
        format!("{s}/ffff-out.drv\n"),
    )
    .unwrap();
    fs::write(dir.path().join("gone-after"), format!("{s}/bbbb-temp\n")).unwrap();
    assert_eq!(
        listed_names(&nix, &store),
        ["cccc-dead"],
        "a path gone mid-walk failed the listing or lost a deriver"
    );
}

#[test]
fn with_keep_derivations_what_built_a_live_path_is_kept_with_all_it_needs() {
    // Nix's default, and a laptop's: a derivation stays while a path it
    // built is live, and with it the derivations and sources it names.
    for (setting, dead) in [
        ("true\n", vec!["cccc-dead", "hhhh-other.drv"]),
        ("", vec!["cccc-dead", "hhhh-other.drv"]),
        (
            "false\n",
            vec![
                "cccc-dead",
                "ffff-out.drv",
                "gggg-src",
                "hhhh-other.drv",
                "iiii-dep.drv",
            ],
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (nix, store) = unlocked_nix(
            dir.path(),
            &[
                "aaaa-out",
                "cccc-dead",
                "ffff-out.drv",
                "gggg-src",
                "hhhh-other.drv",
                "iiii-dep.drv",
            ],
        );
        let s = store.display();
        fs::write(dir.path().join("keep-derivations"), setting).unwrap();
        fs::write(dir.path().join("roots"), format!("/r -> {s}/aaaa-out\n")).unwrap();
        fs::write(
            dir.path().join("derivers/aaaa-out"),
            format!("{s}/ffff-out.drv\n"),
        )
        .unwrap();
        fs::write(
            dir.path().join("kept/ffff-out.drv"),
            format!("{s}/gggg-src\n"),
        )
        .unwrap();
        // A source a derivation names may itself be built, by a derivation
        // kept for it in turn, which only a second round finds.
        fs::write(
            dir.path().join("derivers/gggg-src"),
            format!("{s}/iiii-dep.drv\n"),
        )
        .unwrap();
        assert_eq!(
            listed_names(&nix, &store),
            dead,
            "keep-derivations {setting:?} keeps the wrong derivations"
        );
        assert!(
            !dir.path().join("locked").exists(),
            "keep-derivations needs no lock, yet the locked listing ran"
        );
    }
}

#[test]
fn a_nix_keeping_outputs_is_listed_by_its_own_collector() {
    // With keep-outputs Nix keeps every output of a live derivation, even
    // one it no longer builds, which no closure shows. A Nix that can't
    // say is taken to keep them.
    for outputs in ["true\n", ""] {
        let dir = tempfile::tempdir().unwrap();
        let (nix, store) = unlocked_nix(dir.path(), &["cccc-dead"]);
        fs::write(dir.path().join("keep-outputs"), outputs).unwrap();
        assert_eq!(
            listed_names(&nix, &store),
            Vec::<String>::new(),
            "the locked listing here lists nothing"
        );
        assert!(
            dir.path().join("locked").exists(),
            "keep-outputs {outputs:?} must go through Nix's own collector"
        );
    }
}
