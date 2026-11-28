//! What `ready` answers, case by case: what a run built, what an earlier
//! pass of the same change speaks for, and which shared files stand apart.

use super::*;

const MAIN: &str = "aaaa";

fn on_main(commit: &str) -> bool {
    commit == MAIN
}

fn nothing(_: &str) -> String {
    String::new()
}

/// What a branch changing `files` shares with main moving `moved`,
/// with nothing renamed on either side.
fn sharing(files: &[String], moved: Option<Vec<String>>) -> Option<Vec<String>> {
    moved.map(|moved| {
        let branch = Side {
            changed: files,
            gone: &[],
            text: &nothing,
        };
        let main = Side {
            changed: &moved,
            gone: &[],
            text: &nothing,
        };
        shared(&branch, &main)
    })
}

fn owned(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn a_head_behind_main_speaks_for_itself_first() {
    // On 15 November #4791 rebased four times in 20 minutes as main
    // moved eight times, none of them in its files, and each rebase
    // ran nothing again: its own run spoke for it as well as before.
    assert_eq!(
        speakers("head", true, &owned(&["earlier"])),
        owned(&["head", "earlier"]),
        "a head behind main wasn't judged by its own run first"
    );
    assert_eq!(
        speakers("head", true, &owned(&["earlier", "head"])),
        owned(&["head", "earlier"]),
        "a head behind main that was a candidate spoke twice"
    );
    assert_eq!(
        speakers("head", false, &owned(&["head", "earlier"])),
        owned(&["earlier"]),
        "a head on main spoke for itself, which its own record already judged"
    );
}

#[test]
fn a_file_naming_a_path_the_other_side_renamed_is_shared() {
    // #2597's case: the branch renamed a script, and main since moved a
    // list naming its old path, which the branch never touched.
    let list = "tests/rules/every-script-waiting.nix";
    let at_main = |file: &str| {
        if file == list {
            "{ \"tests/pi-rings-low-and-says-it.sh\" = 21; }".to_owned()
        } else {
            String::new()
        }
    };
    let renamed = owned(&["tests/pi-rings-low-and-says-it.sh"]);
    let branch_files = owned(&["tests/pi-rings-low.sh", "modules/pi/home.nix"]);
    let main_files = owned(&[list, "docs/glossary.md"]);
    let branch = Side {
        changed: &branch_files,
        gone: &renamed,
        text: &nothing,
    };
    let main = Side {
        changed: &main_files,
        gone: &[],
        text: &at_main,
    };
    assert_eq!(shared(&branch, &main), owned(&[list]));
    // The other way round: main renamed a module's file, and the
    // branch's file names it from its own folder, as a Nix path does.
    let at_head = |file: &str| {
        if file == "modules/pi/home.nix" {
            "{ imports = [ ./rings.nix ]; }".to_owned()
        } else {
            String::new()
        }
    };
    let gone = owned(&["modules/pi/rings.nix"]);
    let branch = Side {
        changed: &branch_files,
        gone: &[],
        text: &at_head,
    };
    let main = Side {
        changed: &main_files,
        gone: &gone,
        text: &nothing,
    };
    assert_eq!(shared(&branch, &main), owned(&["modules/pi/home.nix"]));
    // Naming another file of that folder isn't naming the gone one.
    let at_head = |_: &str| "{ imports = [ ./ringside.nix ]; }".to_owned();
    let branch = Side {
        changed: &branch_files,
        gone: &[],
        text: &at_head,
    };
    assert_eq!(shared(&branch, &main), Vec::<String>::new());
}

#[test]
fn a_run_that_built_what_it_named_is_ready() {
    let record = "{\"check\":\"checks.x86_64-linux.b\",\"dir\":null}\n\
                  {\"dir\":null,\"printed\":\"changed  b\\n\",\
                  \"run\":[\"--system\",\"x86_64-linux\"]}\n";
    assert!(
        built_there(record, &on_main),
        "a run that built what it named wasn't taken as ready"
    );
}

#[test]
fn a_run_that_only_named_or_kept_a_base_is_not() {
    let named = "{\"dir\":null,\"printed\":\"\",\
                 \"run\":[\"--name-only\",\"--system\",\"x86_64-linux\"]}\n\
                 {\"dir\":null,\"printed\":\"\",\
                 \"run\":[\"--keep-base\",\"--system\",\"x86_64-linux\"]}\n\
                 {\"check\":\"checks.x86_64-linux.b\",\"dir\":null}\n";
    assert!(
        !built_there(named, &on_main),
        "a run that only named or kept a base was taken as built"
    );
    assert!(!built_there("", &on_main), "nothing passed there");
}

#[test]
fn a_run_against_a_base_main_lacks_is_not() {
    let run = |base: &str| {
        format!(
            "{{\"dir\":null,\"printed\":\"\",\"run\":[\"--base\",\"{base}\",\
             \"--system\",\"x86_64-linux\"]}}\n"
        )
    };
    assert!(
        built_there(&run(MAIN), &on_main),
        "a run against a base main holds wasn't taken as built"
    );
    assert!(
        !built_there(&run("bbbb"), &on_main),
        "a base on the branch skips what came before it"
    );
}

#[test]
fn the_same_change_checked_before_a_clean_rebase_is_ready() {
    let passed = "{\"dir\":null,\"printed\":\"\",\"run\":[\"--system\",\"x86_64-linux\"]}\n";
    let record = |commit: &str| {
        if commit == "old" || commit == "older" {
            passed.to_owned()
        } else {
            String::new()
        }
    };
    let files = vec!["a.rs".to_owned(), "b.md".to_owned()];
    let candidates = vec!["other".to_owned(), "old".to_owned(), "older".to_owned()];
    // Main moved other files since: the rebase changed nothing checked.
    let moved = |commit: &str| (commit == "old").then(|| vec!["c.nix".to_owned()]);
    assert_eq!(
        checked_before(&candidates, &record, &on_main, &|at: &str| sharing(
            &files,
            moved(at)
        )),
        Before::Clean("old".to_owned())
    );
    // Main moved one of its files: changed-checks runs again at the
    // head, whatever single checks passed there.
    let moved =
        |commit: &str| (commit == "old").then(|| vec!["b.md".to_owned(), "c.nix".to_owned()]);
    assert_eq!(
        checked_before(&candidates, &record, &on_main, &|at: &str| sharing(
            &files,
            moved(at)
        )),
        Before::Shares("old".to_owned(), vec!["b.md".to_owned()])
    );
    // Only single checks passed at a later commit of the same change:
    // they don't say what read the shared file.
    let rerun = "{\"check\":\"checks.x86_64-linux.formatting\",\"dir\":null}\n";
    let record_rechecked = |commit: &str| match commit {
        "old" => passed.to_owned(),
        "rebased" => rerun.to_owned(),
        _ => String::new(),
    };
    let candidates = vec!["rebased".to_owned(), "old".to_owned()];
    let moved = |commit: &str| match commit {
        "old" => Some(vec!["b.md".to_owned(), "c.nix".to_owned()]),
        "rebased" => Some(vec!["c.nix".to_owned()]),
        _ => None,
    };
    assert_eq!(
        checked_before(&candidates, &record_rechecked, &on_main, &|at: &str| {
            sharing(&files, moved(at))
        }),
        Before::Shares("old".to_owned(), vec!["b.md".to_owned()])
    );
    // changed-checks passed at that later commit, and main moved none of
    // the change's files since its base.
    let record_rerun = |commit: &str| match commit {
        "old" | "rebased" => passed.to_owned(),
        _ => String::new(),
    };
    assert_eq!(
        checked_before(&candidates, &record_rerun, &on_main, &|at: &str| sharing(
            &files,
            moved(at)
        )),
        Before::Clean("rebased".to_owned())
    );
    let candidates = vec!["other".to_owned(), "old".to_owned(), "older".to_owned()];
    // An earlier commit of another change, or the same change that
    // never passed, says nothing.
    let moved = |commit: &str| (commit == "other").then(Vec::new);
    assert_eq!(
        checked_before(&candidates, &record, &on_main, &|at: &str| sharing(
            &files,
            moved(at)
        )),
        Before::Unchecked
    );
}

#[test]
fn another_change_is_told_apart_by_its_id_before_mains_moves_are_read() {
    let read = std::cell::Cell::new(0);
    let base_of = |candidate: &str| Some(format!("{candidate}-base"));
    let id_of = |_: &str, candidate: &str| {
        Some(if candidate == "same" { "id" } else { "other" }.to_owned())
    };
    let mains_moves = |base: String| {
        read.set(read.get() + 1);
        Some(base)
    };
    assert_eq!(
        when_same("another", "id", base_of, id_of, mains_moves),
        None,
        "a change with another id counted as the same"
    );
    assert_eq!(read.get(), 0, "it read what main moved for another change");
    assert_eq!(
        when_same("same", "id", base_of, id_of, mains_moves),
        Some("same-base".to_owned()),
        "the same change wasn't read from its own base"
    );
}

#[test]
fn the_same_change_passed_at_two_commits_still_shares_what_main_moved() {
    let passed = "{\"dir\":null,\"printed\":\"\",\"run\":[\"--system\",\"x86_64-linux\"]}\n";
    let files = vec!["a.rs".to_owned(), "b.md".to_owned()];
    let candidates = vec!["rebased".to_owned(), "old".to_owned(), "other".to_owned()];
    let moved = |commit: &str| match commit {
        "rebased" => Some(vec!["b.md".to_owned()]),
        "old" => Some(vec!["a.rs".to_owned(), "b.md".to_owned()]),
        _ => None,
    };
    let both = |commit: &str| match commit {
        "rebased" | "old" | "other" => passed.to_owned(),
        _ => String::new(),
    };
    assert_eq!(
        checked_before(&candidates, &both, &on_main, &|at: &str| sharing(
            &files,
            moved(at)
        )),
        Before::Shares("rebased".to_owned(), vec!["b.md".to_owned()]),
        "passed at two commits of the same change, it merged as it stood over main's move"
    );
    // Passed once, and another change passed: one run still asks.
    let once = |commit: &str| match commit {
        "rebased" | "other" => passed.to_owned(),
        _ => String::new(),
    };
    assert_eq!(
        checked_before(&candidates, &once, &on_main, &|at: &str| sharing(
            &files,
            moved(at)
        )),
        Before::Shares("rebased".to_owned(), vec!["b.md".to_owned()])
    );
}

#[test]
fn mains_commit_pushing_a_shared_file_past_a_checks_limit_is_refused() {
    // #6727's case, 26 November: the change passed at two commits, then
    // main moved a file it shared, and the two merged past what the
    // formatting check allows, so it went red on main at 11:07. The
    // stand-in check fails a file past 500 lines, and main takes the
    // merged file unchecked only when ready says the earlier pass
    // speaks for the head; otherwise a run at the head, or the train's
    // at the merged tree, reads it first and refuses it.
    const LIMIT: usize = 500;
    let file = "modules/quickshell/launcher/Presented.js";
    let holds = |lines: usize| lines <= LIMIT;
    let (base, own, mains) = (488, 495, 496);
    let merged = own + mains - base;
    assert!(
        holds(own) && holds(mains) && !holds(merged),
        "the stand-in's check didn't pass on each side alone and fail merged"
    );
    let passed = "{\"dir\":null,\"printed\":\"\",\"run\":[\"--system\",\"x86_64-linux\"]}\n";
    let files = owned(&[file, "modules/quickshell/Launcher.qml"]);
    let candidates = owned(&["rebased", "old"]);
    let record = |_: &str| passed.to_owned();
    let moved = |_: &str| Some(owned(&[file, "docs/glossary.md"]));
    let before = checked_before(&candidates, &record, &on_main, &|at: &str| {
        sharing(&files, moved(at))
    });
    assert!(
        !before.speaks_for_the_head(),
        "ready let a head merge as it stands over main's move past a check's limit: \
         {merged} lines on main"
    );
    assert_eq!(
        before,
        Before::Shares("rebased".to_owned(), owned(&[file])),
        "the file main pushed past the limit wasn't named for a run at the head"
    );
}

#[test]
fn passes_sharing_mains_moves_are_read_no_further_than_two() {
    // Each read of a sharing pass diffs what main moved since its base,
    // so a change that passed four times in a day reads two of its
    // sharing passes. Another change's pass, and a commit with no
    // pass, count for nothing.
    let passed = "{\"dir\":null,\"printed\":\"\",\"run\":[\"--system\",\"x86_64-linux\"]}\n";
    let files = owned(&["a.rs"]);
    let candidates = owned(&["third", "another", "unpassed", "second", "first", "clean"]);
    let record = |at: &str| {
        if at == "unpassed" {
            String::new()
        } else {
            passed.to_owned()
        }
    };
    let read = std::cell::RefCell::new(Vec::new());
    let before = checked_before(&candidates, &record, &on_main, &|at| {
        read.borrow_mut().push(at.to_owned());
        match at {
            "another" => None,
            "clean" => sharing(&files, Some(Vec::new())),
            _ => sharing(&files, Some(files.clone())),
        }
    });
    assert_eq!(before, Before::Shares("third".to_owned(), files.clone()));
    assert_eq!(
        *read.borrow(),
        owned(&["third", "another", "second"]),
        "another change's pass counted toward the two, or a third sharing pass was read"
    );
}

#[test]
fn only_a_clean_pass_speaks_for_the_head() {
    // The train lands a recut candidate on an earlier pass only so.
    assert!(
        Before::Clean("old".to_owned()).speaks_for_the_head(),
        "a pass main moved none of its files under didn't speak for the head"
    );
    assert!(
        !Before::Shares("old".to_owned(), owned(&["b.md"])).speaks_for_the_head(),
        "a pass main moved a shared file under spoke for the head"
    );
    assert!(
        !Before::Unchecked.speaks_for_the_head(),
        "no pass at all spoke for the head"
    );
}

#[test]
fn a_file_naming_what_main_took_away_is_named_whatever_ran() {
    let files = vec!["tests/list.nix".to_owned()];
    let moved = vec!["tests/old.sh".to_owned()];
    let names_old = |_: &str| "[ ./old.sh ]".to_owned();
    let branch = Side {
        changed: &files,
        gone: &[],
        text: &names_old,
    };
    let main = Side {
        changed: &moved,
        gone: &moved,
        text: &nothing,
    };
    assert_eq!(names_gone(&branch, &main), files);
    let branch = Side {
        changed: &files,
        gone: &[],
        text: &nothing,
    };
    assert_eq!(names_gone(&branch, &main), Vec::<String>::new());
}

#[test]
fn check_descriptions_changed_at_different_checks_stand_apart() {
    // #2763's case: each side described a check of its own.
    let entry = |name: &str| format!("  {name} = ''\n    proves\n  '';\n");
    let file = "check-descriptions.nix";
    let base = format!("{{\n{}}}\n", entry("b"));
    let ours = format!("{{\n{}{}}}\n", entry("b"), entry("c"));
    let theirs = format!("{{\n{}{}}}\n", entry("a"), entry("b"));
    let head = format!("{{\n{}{}{}}}\n", entry("a"), entry("b"), entry("c"));
    let files = owned(&[file]);
    let (at_head, at_main) = (|_: &str| head.clone(), |_: &str| theirs.clone());
    let branch = Side {
        changed: &files,
        gone: &[],
        text: &at_head,
    };
    let main = Side {
        changed: &files,
        gone: &[],
        text: &at_main,
    };
    let (at_base, checked) = (|_: &str| base.clone(), |_: &str| ours.clone());
    assert!(
        stands_apart(file, &branch, &main, &at_base, &checked),
        "two checks described apart were counted as one file"
    );
    assert!(
        !stands_apart("other.nix", &branch, &main, &at_base, &checked),
        "another Nix file was read as check descriptions"
    );
    let main = Side {
        changed: &files,
        gone: &[],
        text: &|_: &str| ours.replace("proves", "holds"),
    };
    assert!(
        !stands_apart(file, &branch, &main, &at_base, &checked),
        "both sides' changes to one check stood apart"
    );
}

#[test]
fn a_folders_checks_changed_at_different_checks_stand_apart() {
    // #2807's case: the branch added a check while main changed another.
    let entry = |name: &str, does: &str| format!("\n  {name} = {does};\n");
    let file = "modules/shell/checks.nix";
    let around = |entries: &str| format!("{{ pkgs }}:\nlet\n  x = 1;\nin\n{{{entries}}}\n");
    let base = around(&entry("a", "x"));
    let ours = around(&(entry("a", "x") + &entry("b", "x")));
    let theirs = around(&entry("a", "x + 1"));
    let head = around(&(entry("a", "x + 1") + &entry("b", "x")));
    let files = owned(&[file]);
    let (at_head, at_main) = (|_: &str| head.clone(), |_: &str| theirs.clone());
    let branch = Side {
        changed: &files,
        gone: &[],
        text: &at_head,
    };
    let main = Side {
        changed: &files,
        gone: &[],
        text: &at_main,
    };
    let (at_base, checked) = (|_: &str| base.clone(), |_: &str| ours.clone());
    assert!(
        stands_apart(file, &branch, &main, &at_base, &checked),
        "two checks changed apart in a folder's checks.nix were counted as one file"
    );
    assert!(
        !stands_apart(
            "modules/shell/tests.nix",
            &branch,
            &main,
            &at_base,
            &checked
        ),
        "another Nix file was read as checks"
    );
}

#[test]
fn lists_of_modules_given_a_line_each_stand_apart() {
    // 12 November's case: two changes each listed a module of its own.
    let base = "[\n  ./a\n  ./c\n]\n";
    let (ours, theirs) = ("[\n  ./a\n  ./b\n  ./c\n]\n", "[\n  ./0\n  ./a\n  ./c\n]\n");
    let head = "[\n  ./0\n  ./a\n  ./b\n  ./c\n]\n";
    let (at_head, at_main) = (|_: &str| head.to_owned(), |_: &str| theirs.to_owned());
    let (at_base, checked) = (|_: &str| base.to_owned(), |_: &str| ours.to_owned());
    for (file, list) in LISTS
        .iter()
        .map(|file| (*file, true))
        .chain([("a.nix", false)])
    {
        let files = owned(&[file]);
        let (branch, main) = (
            Side {
                changed: &files,
                gone: &[],
                text: &at_head,
            },
            Side {
                changed: &files,
                gone: &[],
                text: &at_main,
            },
        );
        assert_eq!(
            stands_apart(file, &branch, &main, &at_base, &checked),
            list,
            "{file}, given a line by each side, was judged wrongly"
        );
    }
}

#[test]
fn a_run_in_another_folder_of_the_repository_is_not() {
    let record = "{\"dir\":\"sub\",\"printed\":\"\",\"run\":[\"--system\",\"x86_64-linux\"]}\n";
    assert!(
        !built_there(record, &on_main),
        "a run in another folder spoke for the repository's root"
    );
}
