//! `merge-net built`, on a branch: the record that a machine's system was
//! built with the change at its head, which `ready` asks for before a
//! change merges whose build takes a builder past half an hour.
//!
//! On 26 November #6748's kernel held every switch from 17:14 to about
//! 22:15, since each main after it compiled the kernel again before a
//! machine could take it. Velocity's ruling that day has such a change
//! merge, alone or by train, only once a machine's system with it has been
//! built on a builder, so the compile is paid before main holds it. Which
//! changes those are is `must-run.txt`'s `system` areas: the example
//! machines' kernel and Qt, by their derivations, so a kernel patch, a
//! nixpkgs bump and an overlay under every package each count.
//!
//! Core names no machine, so it builds none: the machine repository's own
//! command builds its system with core at the head, on a builder, and
//! `merge-net built DRV` records the derivation it built under the head, in
//! `built/` under the net's state folder, `{at, drv}`. A record at an
//! earlier commit of the same change speaks for a rebased head, as a pass
//! does.

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use crate::state::{self, short};
use crate::{gated, git, rules};

/// Record that the system `drv` was built with the change at the branch's
/// head.
pub fn built(drv: &str) -> Result<ExitCode, String> {
    // A tree with edits holds what the head doesn't, so a build of it
    // speaks for no commit.
    if !git(&["status", "--porcelain"])?.is_empty() {
        return Err(
            "the tree has changes the head doesn't hold, so a system built from it speaks \
             for no commit; commit them, build again, and record that"
                .to_owned(),
        );
    }
    let head = git(&["rev-parse", "HEAD"])?;
    let drv = said_drv(drv, &|path| Path::new(path).exists())?;
    let root = state::State::from_env().root;
    keep(&root, &head, &drv, state::now())
        .map_err(|error| format!("{drv} wasn't recorded in {}: {error}", root.display()))?;
    println!(
        "built: {} has a machine's system built with it, {drv}",
        short(&head)
    );
    Ok(ExitCode::SUCCESS)
}

/// `drv` as a system's derivation in the store, which `present` says is
/// there, or why it isn't one.
fn said_drv(drv: &str, present: &dyn Fn(&str) -> bool) -> Result<String, String> {
    let name = drv
        .strip_prefix("/nix/store/")
        .filter(|name| !name.contains('/'))
        .and_then(|name| name.strip_suffix(".drv"))
        .ok_or_else(|| format!("{drv} isn't a derivation in /nix/store"))?;
    if !name.contains("-system-") {
        return Err(format!(
            "{drv} isn't a machine's system, whose derivation is named for it, as \
             nixos-system-NAME or darwin-system-NAME is"
        ));
    }
    if !present(drv) {
        return Err(format!("{drv} isn't in this machine's store"));
    }
    Ok(drv.to_owned())
}

/// Keep, under `root`, that `drv` was built with `head` at `at`.
fn keep(root: &Path, head: &str, drv: &str, at: u64) -> std::io::Result<()> {
    let folder = root.join("built");
    fs::create_dir_all(&folder)?;
    let said = serde_json::json!({ "at": at, "drv": drv });
    fs::write(folder.join(head), format!("{said}\n"))
}

/// The derivation kept under `root` as built with any of `commits`.
fn recorded(root: &Path, commits: &[&str]) -> Option<String> {
    commits.iter().find_map(|commit| {
        let text = fs::read_to_string(root.join("built").join(commit)).ok()?;
        let said: serde_json::Value = serde_json::from_str(&text).ok()?;
        said["drv"].as_str().map(str::to_owned)
    })
}

/// The `system` areas a change moves, by what a pass speaking for it
/// named as moved, from each of `records`, nix-offload's records of its
/// passes, and the change's `files`.
pub fn needed(records: &[String], files: &[String]) -> Vec<String> {
    let areas = rules::areas();
    let mut needed: Vec<String> = records
        .iter()
        .filter_map(|record| gated::named(record, "", crate::SYSTEM, &|_| true))
        .flat_map(|said| rules::first(&areas, files, Some(&said), None).system)
        .collect();
    needed.sort();
    needed.dedup();
    needed
}

/// Why a change that moves `needed` may not merge yet, when no system was
/// recorded built with it at any of `commits`, or `None` when it may.
pub fn refused(needed: &[String], commits: &[&str]) -> Option<String> {
    refused_in(&state::State::from_env().root, needed, commits)
}

fn refused_in(root: &Path, needed: &[String], commits: &[&str]) -> Option<String> {
    if needed.is_empty() || recorded(root, commits).is_some() {
        return None;
    }
    let head = commits.last().copied().unwrap_or_default();
    Some(format!(
        "{} moves {}, which takes a builder past half an hour to build, and no machine's \
         system was recorded built with it; build one with the change on a builder, run \
         merge-net built DRV on the branch with the system's derivation, and merge again",
        short(head),
        needed.join(" and ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DRV: &str = "/nix/store/0123abcd-nixos-system-skiff-26.05.drv";

    /// A state folder of the test's own, emptied first.
    fn root(name: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("merge-net-built-{name}-{}", std::process::id()));
        let _gone = fs::remove_dir_all(&root);
        root
    }

    #[test]
    fn only_a_machines_system_in_the_store_is_recorded() {
        let there = |_: &str| true;
        assert_eq!(said_drv(DRV, &there), Ok(DRV.to_owned()));
        assert!(
            said_drv("/nix/store/0123abcd-linux-7.3.4.drv", &there).is_err(),
            "a kernel's derivation was taken for a system's"
        );
        assert!(
            said_drv("/tmp/0123abcd-nixos-system-skiff.drv", &there).is_err(),
            "a derivation outside the store was taken"
        );
        assert!(
            said_drv("/nix/store/0123abcd-nixos-system-skiff", &there).is_err(),
            "an output was taken for a derivation"
        );
        assert!(
            said_drv(DRV, &|_| false).is_err(),
            "a derivation missing from the store was taken"
        );
    }

    #[test]
    fn a_change_moving_a_long_build_waits_for_a_system_built_with_it() {
        let root = root("waits");
        let needed = vec!["long builds".to_owned()];
        let refused = refused_in(&root, &needed, &["c1", "c2"]);
        assert!(
            refused
                .as_deref()
                .is_some_and(|why| why.contains("long builds") && why.contains("merge-net built")),
            "a kernel change with no system built was let merge: {refused:?}"
        );
        keep(&root, "c1", DRV, 7).expect("the record is kept");
        assert_eq!(
            refused_in(&root, &needed, &["c1", "c2"]),
            None,
            "a system built at an earlier commit of the change didn't speak for it"
        );
        assert_eq!(
            refused_in(&root, &needed, &["c3"]),
            Some(
                "c3 moves long builds, which takes a builder past half an hour to build, and \
                 no machine's system was recorded built with it; build one with the change on \
                 a builder, run merge-net built DRV on the branch with the system's \
                 derivation, and merge again"
                    .to_owned()
            ),
            "a system built with another change spoke for this one"
        );
        assert_eq!(
            refused_in(&root, &[], &["c3"]),
            None,
            "a change moving no long build was held"
        );
        let _gone = fs::remove_dir_all(&root);
    }

    #[test]
    fn what_a_pass_named_says_which_long_builds_a_change_moves() {
        let record = |printed: &str| {
            format!(
                "{}\n",
                serde_json::json!({"dir": null, "printed": printed, "run": ["--base", "m"]})
            )
        };
        let files = vec!["hardware/a/kernel/local.nix".to_owned()];
        assert_eq!(
            needed(
                &[record(
                    "changed  example-laptop\nchanged  example-laptop-kernel\n"
                )],
                &files
            ),
            vec!["long builds".to_owned()],
            "a pass that moved the kernel wasn't read as a long build"
        );
        assert!(
            needed(&[record("changed  example-laptop\n")], &files).is_empty(),
            "a pass that moved no kernel was read as a long build"
        );
        assert!(
            needed(&[], &files).is_empty(),
            "with no pass to read, a long build was guessed"
        );
    }
}
