//! `merge-net ready COMMIT`: whether a branch's head may merge, asked
//! before merging by whatever merges it. It may when it sits on origin's
//! main as it is now, and changed-checks built what it named there.
//!
//! On 8 November four checks were red on main by 11:00. changed-checks
//! named each at its merge, but one branch was built before main gained
//! what it collided with, and the others built only some of what was named,
//! or nothing. Both show in nix-offload's record of what passed at each
//! commit (`~/.local/state/nix-offload/passed/COMMIT`), which keeps a line
//! for each changed-checks run sent that passed, with its arguments.

use std::env;
use std::fs;
use std::path::PathBuf;

use serde_json::Value;

/// Whether nix-offload's record for a commit holds a changed-checks run that
/// passed there and built what it named, against a base `on_main` says
/// origin's main holds, so it named everything a merge onto main moves.
/// A run that only named or only kept a base built nothing.
pub fn built_there(record: &str, on_main: &dyn Fn(&str) -> bool) -> bool {
    record
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|line| line["dir"].is_null())
        .filter_map(|line| line["run"].as_array().cloned())
        .any(|run| {
            let words: Vec<&str> = run.iter().filter_map(serde_json::Value::as_str).collect();
            let built = !words
                .iter()
                .any(|word| matches!(*word, "--name-only" | "--keep-base"));
            let base = words
                .iter()
                .position(|word| *word == "--base")
                .and_then(|at| words.get(at + 1));
            built && base.is_none_or(|base| on_main(base))
        })
}

/// What an earlier commit of the same change says of a head with no run of
/// its own.
#[derive(Debug, PartialEq, Eq)]
pub enum Before {
    /// The same change passed at this commit, and main has since moved
    /// none of its files, so the rebase changed nothing it was checked on.
    Clean(String),
    /// The same change passed at this commit, but main has since moved
    /// these of its files, so changed-checks runs again at the head.
    Shares(String, Vec<String>),
    /// No commit of the same change passed.
    Unchecked,
}

/// Whether the head is the same change as one of `candidates`, newest
/// first, that a changed-checks run passed at (its `record`, judged as
/// `built_there` does with `on_main`). `same` says, of a candidate,
/// whether it makes the same change and, if so, which files it shares
/// with what main moved since the base it was checked on, as `shared`
/// works them out. On 8 November a
/// rule let a change merge after a rebase onto main without running its
/// checks again when main's new commits share none of its files, so the
/// gate takes what the rule takes. When they share some, only changed-checks
/// at the head counts, since which checks read a file shows only in their
/// derivations: on 9 November #2504 merged with words.json's checks rerun
/// at its head after main moved two other of its files, which no check
/// that ran read. Nix builds only what the rebase moved, and takes the
/// rest from the builders.
pub fn checked_before(
    candidates: &[String],
    record: &dyn Fn(&str) -> String,
    on_main: &dyn Fn(&str) -> bool,
    same: &dyn Fn(&str) -> Option<Vec<String>>,
) -> Before {
    let mut first_shared = None;
    for candidate in candidates {
        if !built_there(&record(candidate), on_main) {
            continue;
        }
        let Some(shared) = same(candidate) else {
            continue;
        };
        if shared.is_empty() {
            return Before::Clean(candidate.clone());
        }
        first_shared.get_or_insert(Before::Shares(candidate.clone(), shared));
    }
    first_shared.unwrap_or(Before::Unchecked)
}

/// One side of a rebase: the files it changed, the paths it renamed or
/// deleted, and what a file holds on that side.
pub struct Side<'a> {
    pub changed: &'a [String],
    pub gone: &'a [String],
    pub text: &'a dyn Fn(&str) -> String,
}

/// The files a branch shares with what main moved since its base: those
/// both changed, and those either side changed that name a path the other
/// renamed or deleted. On 9 November #2597 renamed a script after #2613,
/// merged since its run, named the old path in a list #2597 never touched,
/// and main broke when #2597 merged with no run of its own.
pub fn shared(branch: &Side, main: &Side) -> Vec<String> {
    let mut shared: Vec<String> = main
        .changed
        .iter()
        .filter(|file| branch.changed.contains(file))
        .cloned()
        .collect();
    for (side, other) in [(main, branch), (branch, main)] {
        for file in side.changed {
            let text = (side.text)(file);
            if !shared.contains(file) && other.gone.iter().any(|path| names(&text, file, path)) {
                shared.push(file.clone());
            }
        }
    }
    shared
}

/// Whether `text`, the file at `file`, names `path`: whole, from the
/// repository's root, or from the file's own folder, as a Nix path does.
fn names(text: &str, file: &str, path: &str) -> bool {
    let folder = file.rfind('/').map_or("", |at| &file[..=at]);
    text.contains(path)
        || path
            .strip_prefix(folder)
            .is_some_and(|near| !folder.is_empty() && text.contains(&format!("./{near}")))
}

/// Whether `file`, which the branch shares with main, is a JSON file both
/// changed `apart`, so it stands as two files would. One naming a path
/// either side renamed or deleted stays shared, as `shared` holds. `base`
/// gives a file where the branch was checked from, and `checked` where it
/// was checked at.
pub fn stands_apart(
    file: &str,
    branch: &Side,
    main: &Side,
    base: &dyn Fn(&str) -> String,
    checked: &dyn Fn(&str) -> String,
) -> bool {
    let json = std::path::Path::new(file)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
    let both = [branch, main]
        .iter()
        .all(|side| side.changed.iter().any(|changed| changed == file));
    if !json || !both {
        return false;
    }
    let head = (branch.text)(file);
    let names_gone = branch
        .gone
        .iter()
        .chain(main.gone)
        .any(|path| head.contains(path.as_str()));
    !names_gone && apart(&base(file), &checked(file), &(main.text)(file), &head)
}

/// Whether a JSON file both sides changed was changed apart: what the
/// branch changed from `base` and what main changed since touch no key in
/// common, and the rebased `head` differs from each side only where the
/// other changed it. Two such changes stand as two files would, which the
/// gate already lets merge with no new run, the net catching how they
/// meet. On 9 November #2571 was refused four times in two hours, each
/// time for main moving words.json, a 300 KB file of the launcher's words
/// that a quarter of main's merges change, during its own run; each of the
/// 23 merges to it that day changed different words from the one before.
/// A file any side leaves unreadable isn't apart.
#[must_use]
pub fn apart(base: &str, branch: &str, main: &str, head: &str) -> bool {
    let read = |text: &str| serde_json::from_str::<Value>(text).ok();
    let (Some(base), Some(branch), Some(main), Some(head)) =
        (read(base), read(branch), read(main), read(head))
    else {
        return false;
    };
    let ours = changed(&base, &branch);
    let theirs = changed(&base, &main);
    let within = |paths: &[Vec<String>], of: &[Vec<String>]| {
        paths
            .iter()
            .all(|path| of.iter().any(|whole| path.starts_with(whole)))
    };
    !ours
        .iter()
        .any(|a| theirs.iter().any(|b| a.starts_with(b) || b.starts_with(a)))
        && within(&changed(&main, &head), &ours)
        && within(&changed(&branch, &head), &theirs)
}

/// The paths of the keys at which `after` differs from `before`. An array
/// whose items are objects each with its own `id` is read as keyed by it,
/// since adding one moves every index after it, and its order counts as
/// one change to the whole; any other array is read by index, and one
/// that changes length is changed whole.
fn changed(before: &Value, after: &Value) -> Vec<Vec<String>> {
    let mut paths = Vec::new();
    differ(before, after, &mut Vec::new(), &mut paths);
    paths
}

/// The paths under one key, one side or both having it.
fn under(
    key: String,
    before: Option<&Value>,
    after: Option<&Value>,
    at: &mut Vec<String>,
    paths: &mut Vec<Vec<String>>,
) {
    at.push(key);
    match (before, after) {
        (Some(before), Some(after)) => differ(before, after, at, paths),
        _ => paths.push(at.clone()),
    }
    at.pop();
}

fn differ(before: &Value, after: &Value, at: &mut Vec<String>, paths: &mut Vec<Vec<String>>) {
    match (before, after) {
        (Value::Object(a), Value::Object(b)) => {
            for key in a.keys().chain(b.keys().filter(|key| !a.contains_key(*key))) {
                under(format!(".{key}"), a.get(key), b.get(key), at, paths);
            }
        }
        (Value::Array(a), Value::Array(b)) => match (by_id(a), by_id(b)) {
            (Some(a), Some(b)) => {
                let kept = |from: &[(&str, &Value)], to: &[(&str, &Value)]| -> Vec<String> {
                    from.iter()
                        .filter(|(id, _)| to.iter().any(|(other, _)| other == id))
                        .map(|(id, _)| (*id).to_owned())
                        .collect()
                };
                if kept(&a, &b) != kept(&b, &a) {
                    paths.push(at.clone());
                    return;
                }
                let mut ids: Vec<&str> = a.iter().map(|(id, _)| *id).collect();
                ids.extend(
                    b.iter()
                        .map(|(id, _)| *id)
                        .filter(|id| find(&a, id).is_none()),
                );
                for id in ids {
                    under(format!("[id={id}]"), find(&a, id), find(&b, id), at, paths);
                }
            }
            _ if a.len() == b.len() => {
                for (index, (a, b)) in a.iter().zip(b).enumerate() {
                    under(format!("[{index}]"), Some(a), Some(b), at, paths);
                }
            }
            _ => paths.push(at.clone()),
        },
        _ if before != after => paths.push(at.clone()),
        _ => {}
    }
}

/// The item with `id` among `items`.
fn find<'a>(items: &[(&str, &'a Value)], id: &str) -> Option<&'a Value> {
    items
        .iter()
        .find(|(other, _)| *other == id)
        .map(|(_, item)| *item)
}

/// An array's items by their `id`, when each is an object with its own.
fn by_id(items: &[Value]) -> Option<Vec<(&str, &Value)>> {
    let keyed: Vec<(&str, &Value)> = items
        .iter()
        .map(|item| item["id"].as_str().map(|id| (id, item)))
        .collect::<Option<_>>()?;
    let mut ids: Vec<&str> = keyed.iter().map(|(id, _)| *id).collect();
    ids.sort_unstable();
    ids.dedup();
    (!keyed.is_empty() && ids.len() == keyed.len()).then_some(keyed)
}

/// A list each change adds a line to, which on a busy day main moves in
/// the minutes a run at a rebased head takes: on 9 November #2576 was
/// refused five times after a green run, each time because main had moved
/// check-descriptions.nix or quarantine.json meanwhile.
#[derive(Debug, PartialEq, Eq)]
pub enum List {
    /// Read by the one check named, which says whether the list holds.
    Judged(String),
    /// The net's quarantine list, which no check builds from: the net,
    /// changed-checks and run-subtests read it from main, so it holds
    /// when it reads as a list of tests, each once.
    Quarantine,
}

/// The list at `path`, or none for a file that isn't one.
pub fn list(path: &str) -> Option<List> {
    if path == crate::world::QUARANTINE {
        return Some(List::Quarantine);
    }
    if path == "check-descriptions.nix" {
        return Some(List::Judged("every-check-says-what-it-proves".to_owned()));
    }
    path.strip_prefix("tests/")
        .and_then(|name| name.strip_suffix("-waiting.nix"))
        .filter(|check| !check.contains('/'))
        .map(|check| List::Judged(check.to_owned()))
}

/// What shared lists say of a head, when every shared file is one.
#[derive(Debug, PartialEq, Eq)]
pub enum Lists {
    /// Some shared file is no list, so changed-checks runs again.
    NotAll,
    /// Each list holds, and these checks of theirs haven't passed at the
    /// head yet; none left means it may merge.
    Need(Vec<String>),
    /// The quarantine list no longer reads as one, saying why.
    Broken(String),
}

/// Whether `shared` are all lists, and what they still need at a head
/// whose `record` says what passed there and whose quarantine list holds
/// `quarantine`. A list's own check is all that reads it, so it's all
/// that runs again, which takes a minute where changed-checks takes ten.
pub fn lists(shared: &[String], record: &str, quarantine: &str) -> Lists {
    let passed: Vec<String> = record
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|line| line["dir"].is_null())
        .filter_map(|line| line["check"].as_str().map(str::to_owned))
        .collect();
    let mut need = Vec::new();
    for file in shared {
        match list(file) {
            None => return Lists::NotAll,
            Some(List::Quarantine) => {
                if let Err(why) = holds_tests(quarantine) {
                    return Lists::Broken(why);
                }
            }
            Some(List::Judged(check)) => {
                let built = passed
                    .iter()
                    .any(|run| run.starts_with("checks.") && run.ends_with(&format!(".{check}")));
                if !built && !need.contains(&check) {
                    need.push(check);
                }
            }
        }
    }
    Lists::Need(need)
}

/// Whether the quarantine list reads as a list of tests, each once.
fn holds_tests(quarantine: &str) -> Result<(), String> {
    let entries: Vec<serde_json::Value> =
        serde_json::from_str(quarantine).map_err(|error| format!("it isn't a list: {error}"))?;
    let mut tests = Vec::new();
    for entry in &entries {
        let test = entry["test"]
            .as_str()
            .ok_or_else(|| format!("an entry names no test: {entry}"))?;
        if tests.contains(&test) {
            return Err(format!("{test} is on it twice"));
        }
        tests.push(test);
    }
    Ok(())
}

/// The commits nix-offload kept a passed record for in the last day,
/// newest first.
pub fn recent_records() -> Vec<String> {
    let Some(folder) = record_of("") else {
        return Vec::new();
    };
    let day_ago = std::time::SystemTime::now() - std::time::Duration::from_hours(24);
    let mut kept: Vec<(std::time::SystemTime, String)> = fs::read_dir(folder)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let at = entry.metadata().ok()?.modified().ok()?;
            let name = entry.file_name().into_string().ok()?;
            (at >= day_ago && name.len() == 40).then_some((at, name))
        })
        .collect();
    kept.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
    kept.into_iter().map(|(_, name)| name).collect()
}

/// The file nix-offload keeps what passed at `commit` in.
pub fn record_of(commit: &str) -> Option<PathBuf> {
    let state = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))?;
    Some(state.join("nix-offload/passed").join(commit))
}

/// What nix-offload kept as passed at `commit`, or nothing.
pub fn record(commit: &str) -> String {
    record_of(commit)
        .and_then(|file| fs::read_to_string(file).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
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

    const WORDS: &str = r#"{"home": ["on"], "things": [
        {"id": "a", "said": {"x": "1"}}, {"id": "b", "said": {"y": "2"}}]}"#;

    #[test]
    fn a_json_file_changed_at_different_keys_is_apart() {
        // #2571's case: the branch changed one thing's words, and main added
        // another thing, which moves every index after it.
        let branch = WORDS.replace("\"1\"", "\"one\"");
        let main = WORDS.replace("[\n", "[\n{\"id\": \"new\"}, ");
        let head = main.replace("\"1\"", "\"one\"");
        assert!(apart(WORDS, &branch, &main, &head), "main added a thing");
        let main = WORDS.replace("[\"on\"]", "[\"on\", \"off\"]");
        let head = main.replace("\"1\"", "\"one\"");
        assert!(
            apart(WORDS, &branch, &main, &head),
            "a list elsewhere moved"
        );
    }

    #[test]
    fn a_json_file_changed_at_one_key_or_merged_oddly_is_not() {
        let branch = WORDS.replace("\"1\"", "\"one\"");
        let main = WORDS.replace("{\"id\": \"a\", \"said\": {\"x\": \"1\"}}, ", "");
        assert!(
            !apart(WORDS, &branch, &main, &main),
            "main took away what it changed"
        );
        let reordered = r#"{"home": ["on"], "things": [
            {"id": "b", "said": {"y": "2"}}, {"id": "a", "said": {"x": "1"}}]}"#;
        let head = reordered.replace("\"1\"", "\"one\"");
        assert!(
            !apart(WORDS, &branch, reordered, &head),
            "main reordered the things"
        );
        let main = WORDS.replace("[\"on\"]", "[\"on\", \"off\"]");
        let head = main.replace("\"1\"", "\"one\"").replace("\"2\"", "\"two\"");
        assert!(
            !apart(WORDS, &branch, &main, &head),
            "the head changed what neither did"
        );
        assert!(
            !apart(WORDS, &branch, "{", &head),
            "main's side doesn't read"
        );
    }

    #[test]
    fn a_file_naming_a_path_the_other_side_renamed_is_shared() {
        // #2597's case: the branch renamed a script, and main since moved a
        // list naming its old path, which the branch never touched.
        let list = "tests/every-script-waiting.nix";
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
    fn shared_lists_need_only_their_own_checks() {
        let quarantine = "[{\"test\":\"vm-a\"},{\"test\":\"vm-b\"}]";
        let shared = owned(&[
            "check-descriptions.nix",
            "tests/every-fixed-wait-says-why-waiting.nix",
            "tools/merge-net/quarantine.json",
        ]);
        assert_eq!(
            lists(&shared, "", quarantine),
            Lists::Need(owned(&[
                "every-check-says-what-it-proves",
                "every-fixed-wait-says-why"
            ]))
        );
        let record = "{\"check\":\"checks.x86_64-linux.every-fixed-wait-says-why\",\"dir\":null}\n\
                      {\"check\":\"checks.x86_64-linux.every-check-says-what-it-proves\",\
                      \"dir\":null}\n";
        assert_eq!(lists(&shared, record, quarantine), Lists::Need(Vec::new()));
        // A check passed in another folder of the repository says nothing.
        let elsewhere =
            "{\"check\":\"checks.x86_64-linux.every-fixed-wait-says-why\",\"dir\":\"sub\"}\n";
        assert_eq!(
            lists(
                &owned(&["tests/every-fixed-wait-says-why-waiting.nix"]),
                elsewhere,
                ""
            ),
            Lists::Need(owned(&["every-fixed-wait-says-why"]))
        );
        // One shared file that's no list, and changed-checks runs again.
        let mixed = owned(&["check-descriptions.nix", "checks.nix"]);
        assert_eq!(lists(&mixed, record, quarantine), Lists::NotAll);
        assert_eq!(
            lists(
                &owned(&["tests/lib/waiting-waiting.nix/x"]),
                record,
                quarantine
            ),
            Lists::NotAll,
            "a file under tests/ in a folder isn't a list"
        );
    }

    #[test]
    fn a_quarantine_list_that_no_longer_reads_as_one_is_broken() {
        let shared = owned(&["tools/merge-net/quarantine.json"]);
        assert_eq!(
            lists(&shared, "", "[{\"test\":\"vm-a\"}]"),
            Lists::Need(Vec::new())
        );
        for (merged, why) in [
            (
                "[{\"test\":\"vm-a\"}\n{\"test\":\"vm-b\"}]",
                "it isn't a list",
            ),
            (
                "[{\"test\":\"vm-a\"},{\"test\":\"vm-a\"}]",
                "vm-a is on it twice",
            ),
            ("[{\"reason\":\"x\"}]", "an entry names no test"),
        ] {
            let Lists::Broken(said) = lists(&shared, "", merged) else {
                panic!("{merged} read as a list");
            };
            assert!(said.starts_with(why), "{merged}: {said}");
        }
    }

    #[test]
    fn a_run_that_built_what_it_named_is_ready() {
        let record = "{\"check\":\"checks.x86_64-linux.b\",\"dir\":null}\n\
                      {\"dir\":null,\"printed\":\"changed  b\\n\",\
                      \"run\":[\"--system\",\"x86_64-linux\"]}\n";
        assert!(built_there(record, &on_main));
    }

    #[test]
    fn a_run_that_only_named_or_kept_a_base_is_not() {
        let named = "{\"dir\":null,\"printed\":\"\",\
                     \"run\":[\"--name-only\",\"--system\",\"x86_64-linux\"]}\n\
                     {\"dir\":null,\"printed\":\"\",\
                     \"run\":[\"--keep-base\",\"--system\",\"x86_64-linux\"]}\n\
                     {\"check\":\"checks.x86_64-linux.b\",\"dir\":null}\n";
        assert!(!built_there(named, &on_main));
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
        assert!(built_there(&run(MAIN), &on_main));
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
    fn a_run_in_another_folder_of_the_repository_is_not() {
        let record = "{\"dir\":\"sub\",\"printed\":\"\",\"run\":[\"--system\",\"x86_64-linux\"]}\n";
        assert!(!built_there(record, &on_main));
    }
}
