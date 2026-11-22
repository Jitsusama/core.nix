//! `merge-net needs`, on a branch: whether its change runs VM tests before
//! it merges, rather than after on the net, and which. It names each VM
//! test whose area the change touches (`rules/`) and each that owns one
//! of its files (`owners/`), and keeps only those whose derivation moved
//! from main, read from a run that passed at a clean head where one did
//! (`gated.rs`) and asked of changed-checks otherwise.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use crate::rules::lock;
use crate::state::{self, short};
use crate::{SYSTEM, comments, gated, git, macs, owners, ready, rules, world};

/// Say whether the branch's change runs VM tests before merging, and which,
/// then each of a Mac's comparisons its gate passed without (`macs.rs`).
pub fn needs(base: &str) -> Result<ExitCode, String> {
    let answered = answer(base);
    // A tree with edits holds what no gate at its head saw, so its record
    // speaks only of the commit.
    let clean = git(&["status", "--porcelain"]).is_ok_and(|status| status.is_empty());
    if clean && let Ok(head) = git(&["rev-parse", "HEAD"]) {
        print!("{}", macs::said(&ready::record(&head), short(&head)));
    }
    answered
}

/// Say whether the branch's change runs VM tests before merging, and which.
fn answer(base: &str) -> Result<ExitCode, String> {
    let began = state::now();
    let head = git(&["rev-parse", "HEAD"])?;
    let merge_base = git(&["merge-base", "HEAD", base])?;
    let mut files: Vec<String> = git(&["diff", "--name-only", &merge_base])?
        .lines()
        .chain(git(&["ls-files", "--others", "--exclude-standard"])?.lines())
        .map(str::to_owned)
        .collect();
    files.sort();
    files.dedup();
    let files = comments::reaching(files, |file| git(&["diff", &merge_base, "--", file]).ok());
    let areas = rules::areas();
    let known = known_vm_tests();
    let root = git(&["rev-parse", "--show-toplevel"])?;
    let (files, lock_moved) = lock_left(files, &merge_base, Path::new(&root));
    let owning = owning(&root, &files);
    let clean = git(&["status", "--porcelain"]).is_ok_and(|status| status.is_empty());
    let came_before = |commit: &str| {
        Command::new("git")
            .args(["merge-base", "--is-ancestor", commit, &merge_base])
            .status()
            .is_ok_and(|status| status.success())
    };
    // What a gate that passed here says moved is read before deciding
    // nothing runs, since it costs no evaluation and a change can move a
    // machine's initrd from a file no area names. A rebased head has no run
    // of its own, but an earlier commit of the same change whose files main
    // hasn't moved since speaks for it, as it does for `ready`.
    let record = ready::record(&head);
    let kept = gated::kept(&record, clean, &merge_base, &came_before, || {
        crate::earlier::pass(&head, &git(&["rev-parse", base]).ok()?)
    });
    let named = kept.as_ref().map(|(said, _)| said.as_str());
    if nothing_asked(
        &owning,
        &rules::first(&areas, &files, named, known.as_deref()),
        named,
        lock_moved,
    ) {
        println!("no: nothing here needs its VM tests before merging; the net runs them after");
        gated::keep(&head, began, gated::Asked::Nothing);
        return Ok(ExitCode::SUCCESS);
    }
    // Only a test whose derivation moved from main can give a result main
    // hasn't, so changed-checks is asked which moved; it costs an
    // evaluation, which only a change touching an area or a VM test's
    // files pays, and none whose gate already passed at its head.
    let (said, asked) = gated::moved(kept, || moved_from(base));
    gated::keep(&head, began, asked);
    match asked {
        gated::Asked::Gate => println!(
            "(a run that passed at {} named what moved, so none was evaluated)",
            short(&head)
        ),
        gated::Asked::Earlier => println!(
            "(a run that passed at an earlier commit of the same change, whose files main \
             hasn't moved since, named what moved, so none was evaluated)"
        ),
        gated::Asked::Changed | gated::Asked::Nothing => {}
    }
    if said.is_none() {
        println!("(changed-checks couldn't say which VM tests moved, so every one named runs)");
    }
    let files = lock_kept_unsaid(files, lock_moved, said.as_deref());
    let found = rules::first(&areas, &files, said.as_deref(), known.as_deref());
    let moved = said.as_deref().map(rules::moved);
    let owning: Vec<String> = owning
        .into_iter()
        .filter(|test| moved.as_ref().is_none_or(|moved| moved.contains(test)))
        .collect();
    // The list is read from the base, main, so a branch can't quarantine
    // its own failure.
    let list = git(&["show", &format!("{base}:{}", world::QUARANTINE)])
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    if found.tests.is_empty() && found.there.is_empty() && owning.is_empty() {
        println!(
            "no: none of the VM tests the change touches moved from {base}, so each would \
             give main's result; the net runs the rest after"
        );
        return Ok(ExitCode::SUCCESS);
    }
    println!("yes:");
    print!("{}", said_owners(&owning, base, &list));
    if found.touched.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }
    println!("the change touches");
    for (area, hits) in &found.touched {
        println!("  {area}: {}", hits.join(" "));
    }
    for area in &found.there {
        println!(
            "send one VM test through the changed {area}, end to end: run one VM test's \
             subtests with run-subtests --there, with the change's own {area} first on PATH"
        );
    }
    if found.tests.is_empty() {
        println!("no VM test it runs moved from {base}, so none runs whole first");
        return Ok(ExitCode::SUCCESS);
    }
    println!("run these VM tests whole before merging, since they moved from {base}:");
    for test in &found.tests {
        println!("{}", rules::named(test, &list));
    }
    if known.is_none() {
        println!("(the names are patterns: no list of the checks was found to match them against)");
    }
    Ok(ExitCode::SUCCESS)
}

/// Whether `needs` may answer no without asking changed-checks which VM
/// tests moved: no VM test owns a file of the change, `found` touches no
/// area, and no lock was let through unread, since a lock's inputs can move
/// a machine's initrd, which only a passed gate's record (`named`) or an
/// evaluation says.
fn nothing_asked(
    owning: &[String],
    found: &rules::Needs,
    named: Option<&str>,
    lock_moved: bool,
) -> bool {
    owning.is_empty() && found.touched.is_empty() && (named.is_some() || !lock_moved)
}

/// `files` with the lock put back where it was let through but
/// changed-checks couldn't say what moved (`said`), since only that says
/// whether a let-through input reached a machine's initrd; so the lock's
/// area names every VM test, as it did before any lock was let through.
fn lock_kept_unsaid(mut files: Vec<String>, lock_moved: bool, said: Option<&str>) -> Vec<String> {
    if lock_moved && said.is_none() {
        println!(
            "({} runs its VM tests first after all: what its inputs moved couldn't be read)",
            lock::LOCK
        );
        files.push(lock::LOCK.to_owned());
        files.sort();
    }
    files
}

/// `files` as `lock_decided` leaves them, reading each file as
/// `merge_base` holds it in the repo at `root`, or as its working copy
/// does, and printing what it says; with whether it let the lock through.
fn lock_left(files: Vec<String>, merge_base: &str, root: &Path) -> (Vec<String>, bool) {
    let had_lock = files.iter().any(|file| file == lock::LOCK);
    let (files, said) = lock_decided(
        files,
        |path| crate::git_at(root, &["show", &format!("{merge_base}:{path}")]).ok(),
        |path| fs::read_to_string(root.join(path)).ok(),
    );
    if let Some(said) = said {
        println!("{said}");
    }
    let moved = had_lock && !files.iter().any(|file| file == lock::LOCK);
    (files, moved)
}

/// `files` without the lock file where its change from the merge base,
/// read by `base`, to the working copy, read by `working`, moves only
/// inputs `lock-after.txt` names, since a broken one can't stop a machine
/// booting and the net runs its VM tests after the merge; with a line
/// saying so, or why the lock stays. The list is read from the merge base
/// alone, so a change can't widen the list it is judged by, and a base
/// without it lets nothing through. A lock either side doesn't hold, or
/// that doesn't read, stays.
fn lock_decided(
    files: Vec<String>,
    base: impl Fn(&str) -> Option<String>,
    working: impl Fn(&str) -> Option<String>,
) -> (Vec<String>, Option<String>) {
    if !files.iter().any(|file| file == lock::LOCK) {
        return (files, None);
    }
    let judged = match (
        base(lock::LOCK_AFTER),
        base(lock::LOCK),
        working(lock::LOCK),
    ) {
        (None, _, _) => Err(format!("the merge base holds no {}", lock::LOCK_AFTER)),
        (_, None, _) => Err("it can't be read at the merge base".to_owned()),
        (_, _, None) => Err("it can't be read in the working copy".to_owned()),
        (Some(after), Some(before), Some(now)) => {
            lock::lock_judged(&before, &now, &lock::lock_after(&after))
        }
    };
    match judged {
        Ok(moved) => (
            files
                .into_iter()
                .filter(|file| file != lock::LOCK)
                .collect(),
            Some(format!(
                "({} moves only {}, which lock-after.txt lets move with its VM tests run after \
                 the merge)",
                lock::LOCK,
                moved.join(" ")
            )),
        ),
        Err(why) => (
            files,
            Some(format!("({} runs its VM tests first: {why})", lock::LOCK)),
        ),
    }
}

/// The VM tests that own one of `files` in the checkout at `root`, saying
/// which the reading couldn't find, each named for any change.
fn owning(root: &str, files: &[String]) -> Vec<String> {
    let Some(owning) = owners::in_tree(Path::new(root), files) else {
        println!("(checks.nix or flake.nix didn't read, so no VM test is named for its files)");
        return Vec::new();
    };
    if !owning.unread.is_empty() {
        println!(
            "({} couldn't be read from checks.nix, so each is named for any change)",
            owning.unread.join(" ")
        );
    }
    owning.tests
}

/// Name the VM tests that own a file the change touches, each of which is
/// the one written to catch it, so it runs before the merge rather than
/// on main.
fn said_owners(owning: &[String], base: &str, list: &serde_json::Value) -> String {
    if owning.is_empty() {
        return String::new();
    }
    // run-subtests with no words lists the subtests and runs none, so each
    // test is given as the two commands that list them and run some: on
    // 15 November a session ran each test as named and passed having run
    // nothing.
    let mut said = format!(
        "run the subtests that reach what it touched in each VM test that owns one of its \
         files and moved from {base}: list them with the first command, then run those with \
         the second, naming words from them, since with no words it runs nothing:\n"
    );
    for test in owning {
        let _ = writeln!(
            said,
            "  {}\n    run-subtests --there {test}\n    run-subtests --there {test} <words>",
            rules::named(test, list)
        );
    }
    said
}

/// What the changed-checks it was built with (the profile has none) says
/// moved between `base` and the working copy, or nothing when it couldn't compare.
fn moved_from(base: &str) -> Option<String> {
    let ran = Command::new(option_env!("MERGE_NET_CHANGED_CHECKS").unwrap_or("changed-checks"))
        .args(rules::changed_checks_asks(base, SYSTEM))
        .stderr(Stdio::inherit())
        .output()
        .ok()?;
    let said = String::from_utf8_lossy(&ran.stdout).into_owned();
    // It fails when a check no longer evaluates, and names it.
    (ran.status.success() || said.lines().any(|line| line.starts_with("broken"))).then_some(said)
}

/// The VM tests core has, from the list each new main writes
/// (`~/.local/state/check-names/core.json`), where there is one, since
/// evaluating the flake to name them waits on the machine's memory.
fn known_vm_tests() -> Option<Vec<String>> {
    let path = PathBuf::from(env::var_os("HOME")?).join(".local/state/check-names/core.json");
    let names: serde_json::Value = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    let tests: Vec<String> = names["checks"][SYSTEM]
        .as_array()?
        .iter()
        .filter_map(|name| name.as_str())
        .filter(|name| name.starts_with("vm-"))
        .map(str::to_owned)
        .collect();
    (!tests.is_empty()).then_some(tests)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_owning_test_is_named_with_the_commands_that_run_its_subtests() {
        // On 15 November a session ran each owning test as named, with no
        // words, and each only listed its subtests and passed.
        let said = said_owners(
            &["vm-desktop-a".to_owned()],
            "origin/main",
            &serde_json::Value::Null,
        );
        assert!(
            said.contains("run-subtests --there vm-desktop-a <words>"),
            "the owning test isn't given as a command that runs subtests: {said}"
        );
        assert!(
            said.contains("with no words"),
            "it doesn't warn that no words runs nothing: {said}"
        );
    }

    /// A lock whose root names mycelium and nixpkgs, at the revs given.
    fn lock_at(mycelium: &str, nixpkgs: &str) -> String {
        serde_json::json!({
            "nodes": {
                "root": { "inputs": { "mycelium": "mycelium", "nixpkgs": "nixpkgs" } },
                "mycelium": { "locked": { "rev": mycelium } },
                "nixpkgs": { "locked": { "rev": nixpkgs } },
            },
            "root": "root",
            "version": 7,
        })
        .to_string()
    }

    /// A side of a change holding `lock` and, when given, `after` as its
    /// list.
    fn side(lock: String, after: Option<&str>) -> impl Fn(&str) -> Option<String> {
        let after = after.map(str::to_owned);
        move |path| match path {
            lock::LOCK => Some(lock.clone()),
            lock::LOCK_AFTER => after.clone(),
            _ => None,
        }
    }

    #[test]
    fn a_lock_moving_only_a_listed_input_leaves_the_files() {
        let files = vec![lock::LOCK.to_owned(), "flake.nix".to_owned()];
        let (left, said) = lock_decided(
            files,
            side(lock_at("a", "n"), Some("mycelium\n")),
            side(lock_at("b", "n"), Some("mycelium\n")),
        );
        assert_eq!(left, vec!["flake.nix".to_owned()]);
        assert!(
            said.as_deref()
                .is_some_and(|said| said.contains("moves only mycelium")),
            "it doesn't say the lock was let through: {said:?}"
        );
    }

    #[test]
    fn a_lock_is_judged_by_the_list_its_merge_base_holds() {
        // The change adds nixpkgs to its own list beside a nixpkgs move.
        let files = vec![lock::LOCK.to_owned()];
        let (left, said) = lock_decided(
            files.clone(),
            side(lock_at("a", "n"), Some("mycelium\n")),
            side(lock_at("a", "m"), Some("mycelium\nnixpkgs\n")),
        );
        assert_eq!(
            left, files,
            "a list widened by the change let its lock through"
        );
        assert!(
            said.as_deref().is_some_and(|said| said.contains("nixpkgs")),
            "it doesn't name what kept the lock: {said:?}"
        );
    }

    #[test]
    fn a_merge_base_without_the_list_lets_no_lock_through() {
        let files = vec![lock::LOCK.to_owned()];
        let (left, said) = lock_decided(
            files.clone(),
            side(lock_at("a", "n"), None),
            side(lock_at("b", "n"), Some("mycelium\n")),
        );
        assert_eq!(left, files, "a base with no list let the lock through");
        assert!(
            said.as_deref()
                .is_some_and(|said| said.contains("holds no tools/merge-net/lock-after.txt")),
            "it doesn't say the base lacks the list: {said:?}"
        );
    }

    #[test]
    fn the_list_is_read_from_the_merge_base_not_the_change() {
        // Round 2 of #6039: nothing reached the git read the safety claim
        // rests on, so reading the list from the working copy passed.
        let repo = std::env::temp_dir().join(format!("merge-net-lock-{}", std::process::id()));
        let _ = fs::remove_dir_all(&repo);
        fs::create_dir_all(repo.join("tools/merge-net")).unwrap();
        let in_repo = |arguments: &[&str]| crate::tests::git_in(&repo, arguments);
        in_repo(&["init", "-q", "-b", "main"]);
        fs::write(repo.join(lock::LOCK_AFTER), "mycelium\n").unwrap();
        fs::write(repo.join(lock::LOCK), lock_at("a", "n")).unwrap();
        in_repo(&["add", "."]);
        in_repo(&["commit", "-qm", "base"]);
        let base = in_repo(&["rev-parse", "HEAD"]);
        let files = vec![lock::LOCK.to_owned()];
        // The change commits its widened list, so reading HEAD's list, not
        // the merge base's, would let it through too.
        fs::write(repo.join(lock::LOCK_AFTER), "mycelium\nnixpkgs\n").unwrap();
        fs::write(repo.join(lock::LOCK), lock_at("a", "m")).unwrap();
        in_repo(&["commit", "-qam", "widen"]);
        assert_eq!(
            lock_left(files.clone(), &base, &repo),
            (files.clone(), false),
            "a change widening its own list let its lock through"
        );
        fs::write(repo.join(lock::LOCK), lock_at("b", "n")).unwrap();
        assert_eq!(
            lock_left(files, &base, &repo),
            (Vec::new(), true),
            "a lock moving only what the merge base lists was kept"
        );
        fs::remove_dir_all(&repo).unwrap();
    }

    #[test]
    fn a_lock_neither_side_can_read_stays() {
        let files = vec![lock::LOCK.to_owned()];
        let list_only = |path: &str| (path == lock::LOCK_AFTER).then(|| "mycelium\n".to_owned());
        for (base_has_lock, why) in [(false, "at the merge base"), (true, "in the working copy")] {
            let base = side(lock_at("a", "n"), Some("mycelium\n"));
            let working = side(lock_at("b", "n"), Some("mycelium\n"));
            let (left, said) = if base_has_lock {
                lock_decided(files.clone(), base, list_only)
            } else {
                lock_decided(files.clone(), list_only, working)
            };
            assert_eq!(left, files, "a lock missing {why} was let through");
            assert!(
                said.as_deref().is_some_and(|said| said.contains(why)),
                "it doesn't say the lock can't be read {why}: {said:?}"
            );
        }
    }

    #[test]
    fn a_lock_let_through_with_no_gate_record_still_asks_what_moved() {
        // Round 2 of #6039: a lock moving only mycelium, with no passed gate
        // to read, answered no before changed-checks could say whether the
        // example laptop's initrd moved.
        let found = rules::first(&rules::areas(), &[], None, None);
        assert!(
            !nothing_asked(&[], &found, None, true),
            "a let-through lock with no record answered no unasked"
        );
        assert!(
            nothing_asked(&[], &found, Some(""), true),
            "a let-through lock whose gate record named nothing was asked again"
        );
        assert!(
            nothing_asked(&[], &found, None, false),
            "a change touching nothing, with no lock, was asked of changed-checks"
        );
    }

    #[test]
    fn a_let_through_lock_changed_checks_cant_compare_runs_every_vm_test() {
        // Round 3 of #6039: with no record and changed-checks unable to
        // compare, the let-through lock's area was gone and needs answered
        // no, naming no test though it said every one named runs.
        let known = vec!["vm-boot".to_owned(), "vm-desktop".to_owned()];
        let files = lock_kept_unsaid(Vec::new(), true, None);
        let found = rules::first(&rules::areas(), &files, None, Some(&known));
        assert_eq!(
            found.tests, known,
            "a let-through lock nobody could compare ran fewer than every VM test"
        );
        assert!(
            lock_kept_unsaid(Vec::new(), true, Some("")).is_empty(),
            "a let-through lock changed-checks compared was kept"
        );
        assert!(
            lock_kept_unsaid(Vec::new(), false, None).is_empty(),
            "a change with no let-through lock gained one"
        );
    }

    #[test]
    fn a_change_leaving_the_lock_says_nothing_of_it() {
        let files = vec!["flake.nix".to_owned()];
        let (left, said) = lock_decided(files.clone(), |_| None, |_| None);
        assert_eq!((left, said), (files, None));
    }
}
