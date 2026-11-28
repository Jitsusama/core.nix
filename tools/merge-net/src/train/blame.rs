//! Which riders a red candidate's failed checks are theirs, by what each
//! rider's own pass named as moved, so the train drops those riders at
//! once rather than halving through all of them. A check one crate or
//! one area owns, such as `jobs-sorter-passes-clippy`, can only be broken
//! by a rider that moved it: a check whose derivation no rider moved
//! builds as main's, and changed-checks builds only what moved. A failure
//! none of them owns, or a log naming none, is left to the halving.
//!
//! A check every rider moved, as a lint over every file is, is put on the
//! riders by the files its own words name instead: on 27 November
//! `every-failed-read-says-why` went red three times in one train, one
//! rider's each time, and each red halved 27 riders. A rider owns such a
//! red when its own diff from main touches a file the check names that it
//! doesn't name at the base too, since a file main already fails is no
//! rider's. A red naming no such file, one naming a file no rider
//! touched, or one whose every rider touches a file it names, is left to
//! the halving as before.
//!
//! A candidate whose every failed check is on the quarantine list runs
//! once more before anyone is blamed, since such a check is known to fail
//! by itself.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

/// What nix says of each derivation it couldn't build, as
/// `error: Cannot build '/nix/store/<hash>-<name>.drv'.`, and of those a
/// build asked for, as `error: build of '<path>.drv^*', ... failed`.
const UNBUILT: [&str; 2] = ["error: Cannot build ", "error: build of "];

/// What changed-checks says of a quarantined check that failed outside
/// what its entry excuses, after its name.
const UNEXCUSED: &str = "which main's quarantine doesn't excuse";

/// What changed-checks says of a quarantined check whose failure its
/// entry excuses, after its name.
const EXCUSED: &str = "so it blocks nothing";

/// What changed-checks heads its table of the checks a run moved with.
const TABLE: &str = "KIND";

/// Derivations whose names aren't their check's.
const NAMED_APART: [(&str, &str); 1] = [("treefmt-check", "formatting")];

/// What nix's `-L` puts between a derivation's name and each line its
/// builder said, as `every-failed-read-says-why> ...`.
const SAID_BY: &str = "> ";

/// What a rule says after a file it faults as a whole, as
/// `modules/cadence/spec/src/main.rs holds 1 read whose failure ...`.
const HOLDS: &str = "holds";

/// What follows a file a rule faults at one of its lines, before the
/// line's number, as `modules/cadence/spec/src/main.rs:222: ...`.
const AT_LINE: char = ':';

/// The checks the words of a failed gate say failed, or none when they
/// name no failed build, as when the gate failed before building or a
/// builder was lost. A failed derivation counts only when changed-checks'
/// last table names it as a check the change moved, so the parts and
/// files a check is built from, which fail beneath it, aren't counted:
/// the check they failed beneath is. A check changed-checks excused, by
/// main's quarantine, blocked nothing, so it isn't counted either.
#[must_use]
pub fn failed(words: &str) -> Option<BTreeSet<String>> {
    let table = words
        .rfind(&format!("\n{TABLE} "))
        .map_or("", |at| &words[at + 1..]);
    let checks = moved(table);
    let unbuilt = words
        .lines()
        .filter_map(|line| UNBUILT.iter().find_map(|said| line.strip_prefix(said)))
        .flat_map(|said| said.split('\''))
        .filter_map(|word| {
            word.strip_suffix(".drv^*")
                .or_else(|| word.strip_suffix(".drv"))
        })
        .filter_map(|path| path.rsplit('/').next())
        .filter_map(|name| {
            name.split_once('-')
                .map(|(_, name)| check_of(name, &checks))
        });
    let said_of = |what: &'static str| {
        words
            .lines()
            .filter(move |line| line.contains(what))
            .filter_map(|line| line.strip_prefix("changed-checks: "))
            .filter_map(|said| said.split(' ').next())
            .map(str::to_owned)
    };
    let excused: BTreeSet<String> = said_of(EXCUSED).collect();
    let found: BTreeSet<String> = unbuilt
        .chain(said_of(UNEXCUSED))
        .filter(|check| checks.contains(check) && !excused.contains(check))
        .collect();
    (!found.is_empty()).then_some(found)
}

/// The check a derivation called `name` is, of the `checks` a run moved:
/// itself, when it's one of them, as `kernel-settings-hold-<model>-1958`
/// is with its model's number, its version after the last dash taken
/// off, or the check it's named apart from.
fn check_of(name: &str, checks: &BTreeSet<String>) -> String {
    if checks.contains(name) {
        return name.to_owned();
    }
    if let Some((_, check)) = NAMED_APART.iter().find(|(drv, _)| *drv == name) {
        return (*check).to_owned();
    }
    match name.rsplit_once('-') {
        Some((check, version)) if version.starts_with(|c: char| c.is_ascii_digit()) => {
            check.to_owned()
        }
        _ => name.to_owned(),
    }
}

/// The checks a pass's `printed` words, changed-checks' first table,
/// name as moved: its rows, each a kind and a check, up to the first line
/// that isn't one, as nix-offload's words after it aren't.
#[must_use]
pub fn moved(printed: &str) -> BTreeSet<String> {
    printed
        .lines()
        .skip_while(|line| !line.starts_with(TABLE))
        .skip(1)
        .map_while(row)
        .map(str::to_owned)
        .collect()
}

/// The check a line of changed-checks' table names, as `changed  <check>`,
/// or none when the line isn't a row.
fn row(line: &str) -> Option<&str> {
    let mut words = line.split_whitespace();
    let (kind, check) = (words.next()?, words.next()?);
    (words.next().is_none() && kind.chars().all(|c| c.is_ascii_lowercase()) && !check.contains(':'))
        .then_some(check)
}

/// The checks every changed-checks pass in nix-offload's `record` of a
/// commit named as moved, or none when it kept no table.
#[must_use]
pub fn moved_in(record: &str) -> Option<BTreeSet<String>> {
    let found: BTreeSet<String> = record
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|line| line["printed"].as_str().map(moved))
        .flatten()
        .collect();
    (!found.is_empty()).then_some(found)
}

/// A rider as a red is put on it: its head, the checks its own pass
/// moved, and the files its own diff from main touched, either none where
/// it can't be read, so the rider stays suspected.
#[derive(Clone, Debug)]
pub struct Suspect {
    pub head: String,
    pub moved: Option<BTreeSet<String>>,
    pub touched: Option<BTreeSet<String>>,
}

impl Suspect {
    /// Whether `check` may be this rider's by what its pass moved.
    fn moved(&self, check: &str) -> bool {
        self.moved
            .as_ref()
            .is_none_or(|moved| moved.contains(check))
    }

    /// Whether `path` may be this rider's by what its diff touched.
    fn touches(&self, path: &str) -> bool {
        self.touched
            .as_ref()
            .is_none_or(|touched| touched.contains(path))
    }
}

/// Of `riders`, the heads whose riders `failed` is theirs: for each failed
/// check, each rider that moved it, or whose pass can't say, and for one
/// every rider moved, each rider touching a file `named` says the check's
/// words name and main doesn't fail. None when a failed check is no
/// rider's, or every rider's, so the search halves as before.
#[must_use]
pub fn owners(
    failed: &BTreeSet<String>,
    riders: &[Suspect],
    named: &BTreeMap<String, BTreeSet<String>>,
) -> Option<Vec<String>> {
    let owning: Vec<Vec<&Suspect>> = failed
        .iter()
        .map(|check| owners_of(check, riders, named.get(check)))
        .collect();
    if owning.iter().any(Vec::is_empty) {
        return None;
    }
    let heads: Vec<String> = riders
        .iter()
        .filter(|rider| {
            owning
                .iter()
                .any(|owners| owners.iter().any(|owner| owner.head == rider.head))
        })
        .map(|rider| rider.head.clone())
        .collect();
    (heads.len() < riders.len()).then_some(heads)
}

/// The riders of `riders` whose red `check` may be: those that moved it,
/// unless every rider did, when those touching the files its words name,
/// `paths`, are, so long as each of them is some rider's. A file no rider
/// touched says the red isn't told by the files it names, so then it
/// stays every rider's.
fn owners_of<'a>(
    check: &str,
    riders: &'a [Suspect],
    paths: Option<&BTreeSet<String>>,
) -> Vec<&'a Suspect> {
    let moved: Vec<&Suspect> = riders.iter().filter(|rider| rider.moved(check)).collect();
    let Some(paths) = paths.filter(|paths| !paths.is_empty()) else {
        return moved;
    };
    let told = paths
        .iter()
        .all(|path| riders.iter().any(|rider| rider.touches(path)));
    if moved.len() < riders.len() || !told {
        return moved;
    }
    riders
        .iter()
        .filter(|rider| paths.iter().any(|path| rider.touches(path)))
        .collect()
}

/// The repository files the builder of `check` faults in a gate's
/// `words`, each line it said reading `<path>:<line>: ...` or
/// `<path> holds ...` after nix's `-L` prefix, which names its derivation.
#[must_use]
pub fn named(words: &str, check: &str) -> BTreeSet<String> {
    let checks = BTreeSet::from([check.to_owned()]);
    words
        .lines()
        .filter_map(|line| line.split_once(SAID_BY))
        .filter(|(derivation, _)| check_of(derivation, &checks) == check)
        .filter_map(|(_, said)| faulted(said))
        .map(str::to_owned)
        .collect()
}

/// The file a rule's line `said` faults, when it names one: its first
/// word, followed by its line's number, or by `holds`.
fn faulted(said: &str) -> Option<&str> {
    let mut words = said.split_whitespace();
    let first = words.next()?;
    let path = match first.split_once(AT_LINE) {
        Some((path, at)) => {
            let number = at.strip_suffix(AT_LINE)?;
            (!number.is_empty() && number.chars().all(|c| c.is_ascii_digit())).then_some(path)?
        }
        None => (words.next() == Some(HOLDS)).then_some(first)?,
    };
    repository_path(path).then_some(path)
}

/// Whether `path` reads as a file of the repository, relative to its root
/// and inside it, rather than a store path or a count.
fn repository_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && path.contains(['/', '.'])
}

/// The files `check`'s words at the candidate, `words`, name that its
/// words at the base, `at_base`, don't: those only the riders could have
/// made it fault.
#[must_use]
pub fn named_anew(words: &str, at_base: &str, check: &str) -> BTreeSet<String> {
    let failing_before = named(at_base, check);
    named(words, check)
        .into_iter()
        .filter(|path| !failing_before.contains(path))
        .collect()
}

/// What `check` said when built at the base in its log at `log`, which
/// the train writes as it judges a red (`flaky.rs`).
#[must_use]
pub fn said_at_base(log: &Path) -> String {
    // A check the train never built at the base names no file there, so
    // every file its words name at the candidate stays the riders' to
    // explain; a file no rider touched still leaves the red to halving.
    String::from_utf8_lossy(&fs::read(log).unwrap_or_default()).into_owned()
}

/// The files the rider at `head` touched since it left main at `base`,
/// `git diff --name-only <base>...<head>` in the train's `checkout`; none
/// where git can't say, so it stays suspected.
#[must_use]
pub fn touched(checkout: &Path, base: &str, head: &str) -> Option<BTreeSet<String>> {
    let ran = Command::new("git")
        // Each name whole and unquoted, ended by a NUL, since git quotes a
        // name with an unusual byte otherwise, which no rule would name.
        .args(["diff", "--name-only", "-z", "--no-renames"])
        .arg(format!("{base}...{head}"))
        .current_dir(checkout)
        .output()
        .ok()
        .filter(|ran| ran.status.success())?;
    Some(
        String::from_utf8_lossy(&ran.stdout)
            .split('\0')
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect(),
    )
}

/// The checks the quarantine list `list` has an entry for, whole or by
/// their subtests.
#[must_use]
pub fn listed(list: &serde_json::Value) -> Vec<String> {
    list.as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry["test"].as_str().map(str::to_owned))
        .collect()
}

/// Whether every one of `failed` is on the quarantine list `listed`, the
/// tests its entries name, so the candidate runs again before anyone is
/// blamed. changed-checks already excuses a listed check failing only in
/// its listed subtests, so what's left failed outside them, where a
/// check known to fail by itself may still have.
#[must_use]
pub fn flaky(failed: &BTreeSet<String>, listed: &[String]) -> bool {
    !failed.is_empty() && failed.iter().all(|check| listed.contains(check))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    /// A gate's words: `said`, then changed-checks' verdict over a table
    /// of `checks`.
    fn gate(said: &str, checks: &[&str]) -> String {
        let table = checks.iter().fold(String::new(), |mut table, check| {
            let _ = writeln!(table, "changed  {check}");
            table
        });
        format!("{said}\u{2715} a check the change touched failed\nKIND     CHECK\n{table}")
    }

    // From the train's candidate 9f60a7682dac on 26 November.
    #[test]
    fn a_gates_failed_checks_are_read_from_nixs_last_word() {
        let words = gate(
            "error: Cannot build '/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-home-manager-files\
             .drv'.\n       Reason: 1 dependency failed.\n\
             error: build of '/nix/store/p1wxdcj29kr8cq9pz4j45dbcaksrqss5-jobs-sorter-passes-clippy\
             -0.1.0.drv^*', '/nix/store/waljvg8r2rjp4rjkcac1cs103fh0dd2q-treefmt-check.drv^*', \
             '/nix/store/yjs53dq06gzggf4v4d1l8z9m0bhk1zi9-the-margin-cat-sits-above-its-column\
             .drv^*' failed\n",
            &[
                "formatting",
                "jobs-sorter-passes-clippy",
                "the-margin-cat-sits-above-its-column",
                "bat-reads-its-theme",
            ],
        );
        assert_eq!(
            failed(&words),
            Some(set(&[
                "formatting",
                "jobs-sorter-passes-clippy",
                "the-margin-cat-sits-above-its-column"
            ])),
            "the failed checks weren't read by their checks' names alone"
        );
        assert_eq!(
            failed(&gate("", &["formatting"])),
            None,
            "a gate naming no failed build was read as naming some"
        );
    }

    // From the train's candidates c101c82f66a9 and 083fa77b9003.
    #[test]
    fn a_lone_failure_and_one_beneath_a_check_are_read_as_the_check() {
        let lone = gate(
            "error: Cannot build '/nix/store/g91db4n1if6imcbp0g02yf9l4cp96n7i-the-margin-cat-\
             follows-the-keys-on-niri.drv'.\n       Reason: builder failed with exit code 1.\n\
             changed-checks: the-margin-cat-follows-the-keys-on-niri failed in test_a, which \
             main's quarantine doesn't excuse, so it blocks\n",
            &["the-margin-cat-follows-the-keys-on-niri", "formatting"],
        );
        assert_eq!(
            failed(&lone),
            Some(set(&["the-margin-cat-follows-the-keys-on-niri"])),
            "a lone failure wasn't read"
        );
        let beneath = gate(
            "error: Cannot build '/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-the-launcher-\
             answers-whats-typed-on-niri-part-1.drv'.\n\
             error: Cannot build '/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-the-launcher-\
             answers-whats-typed-on-niri.drv'.\n       Reason: 1 dependency failed.\n",
            &["the-launcher-answers-whats-typed-on-niri"],
        );
        assert_eq!(
            failed(&beneath),
            Some(set(&["the-launcher-answers-whats-typed-on-niri"])),
            "a part's failure wasn't read as its check's"
        );
    }

    #[test]
    fn an_excused_check_and_a_numbered_one_are_read_as_they_were_said() {
        let words = gate(
            "error: Cannot build '/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-the-launcher-\
             answers-whats-typed-on-niri.drv'.\n\
             error: Cannot build '/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-kernel-settings-\
             hold-1958.drv'.\n\
             changed-checks: the-launcher-answers-whats-typed-on-niri failed, but its subtest \
             test_a is on main's quarantine list, so it blocks nothing\n",
            &[
                "the-launcher-answers-whats-typed-on-niri",
                "kernel-settings-hold-1958",
            ],
        );
        assert_eq!(
            failed(&words),
            Some(set(&["kernel-settings-hold-1958"])),
            "an excused check counted, or a check's own number was taken off"
        );
    }

    #[test]
    fn a_pass_names_what_it_moved_from_its_table() {
        let printed = "comparing\nKIND     CHECK\nchanged  formatting\nnew      \
                       jobs-sorter-passes-clippy\n";
        assert_eq!(
            moved(printed),
            set(&["formatting", "jobs-sorter-passes-clippy"]),
            "a pass's table wasn't read whole"
        );
        let followed = format!("{printed}nix-offload: sent 3 builds to vm-4\n\nchanged  later\n");
        assert_eq!(
            moved(&followed),
            set(&["formatting", "jobs-sorter-passes-clippy"]),
            "the lines after the table were read as rows of it"
        );
        let record = format!(
            "{}\n{}\n",
            serde_json::json!({"printed": printed}),
            serde_json::json!({"check": "x", "dir": null})
        );
        assert_eq!(
            moved_in(&record),
            Some(set(&["formatting", "jobs-sorter-passes-clippy"])),
            "a record's passes weren't read"
        );
        assert_eq!(
            moved_in("{\"dir\": null}\n"),
            None,
            "a record with no table named some"
        );
    }

    /// A rider at `head` whose pass moved `moved` and whose diff touched
    /// `touched`, either none where it can't be read.
    fn suspect(head: &str, moved: Option<&[&str]>, touched: Option<&[&str]>) -> Suspect {
        Suspect {
            head: head.to_owned(),
            moved: moved.map(set),
            touched: touched.map(set),
        }
    }

    /// The heads `failed` is put on among `riders`, by what each moved alone.
    fn owners_by_moves(failed: &[&str], riders: &[Suspect]) -> Option<Vec<String>> {
        owners(&set(failed), riders, &BTreeMap::new())
    }

    #[test]
    fn a_failure_falls_only_on_the_riders_that_moved_it() {
        let riders = vec![
            suspect(
                "a",
                Some(&["formatting", "jobs-sorter-passes-clippy"]),
                None,
            ),
            suspect("b", Some(&["formatting", "cadence-passes-clippy"]), None),
            suspect("c", Some(&["formatting"]), None),
        ];
        assert_eq!(
            owners_by_moves(&["jobs-sorter-passes-clippy"], &riders),
            Some(vec!["a".to_owned()]),
            "a crate's red wasn't put on the one rider moving it"
        );
        assert_eq!(
            owners_by_moves(
                &["jobs-sorter-passes-clippy", "cadence-passes-clippy"],
                &riders
            ),
            Some(vec!["a".to_owned(), "b".to_owned()]),
            "two crates' reds weren't put on both their riders"
        );
        assert_eq!(
            owners_by_moves(&["formatting"], &riders),
            None,
            "a red every rider moved was put on all of them rather than searched"
        );
        assert_eq!(
            owners_by_moves(&["every-script-meets-shellcheck-at-its-strictest"], &riders),
            None,
            "a red no rider moved, which is main's, was put on a rider"
        );
    }

    #[test]
    fn a_rider_whose_pass_cant_say_is_suspected() {
        let riders = vec![
            suspect("a", Some(&["formatting"]), None),
            suspect("b", None, None),
            suspect("c", Some(&["cadence-passes-clippy"]), None),
        ];
        assert_eq!(
            owners_by_moves(&["jobs-sorter-passes-clippy"], &riders),
            Some(vec!["b".to_owned()]),
            "a rider with no pass to read was cleared"
        );
    }

    /// The lint every rider moves, as `every-failed-read-says-why` is.
    const LINT: &str = "every-failed-read-says-why";

    /// What the lint said at candidate 6cd83ddb on 27 November, naming a
    /// file one rider touched and one main already failed.
    const LINT_SAID: &str = "every-failed-read-says-why> modules/cadence/spec/src/main.rs \
        holds 1 read whose failure is taken as nothing, 0 of them waiting: say that it \
        couldn't read, ...\n\
        every-failed-read-says-why>   modules/cadence/spec/src/main.rs:222: let read: Value \
        = ...\n\
        every-failed-read-says-why> tools/merge-net/src/train/excused.rs holds 1 read \
        whose ...\n\
        every-failed-read-says-why>   tools/merge-net/src/train/excused.rs:76: \
        fs::read_to_string(...\n\
        every-failed-read-says-why> 725 reads whose failure is taken as nothing, in 329 \
        files.\n";

    /// What the lint said at that candidate's base, faulting the one file.
    const LINT_SAID_AT_BASE: &str = "every-failed-read-says-why> \
        tools/merge-net/src/train/excused.rs holds 1 read whose ...\n\
        every-failed-read-says-why>   tools/merge-net/src/train/excused.rs:76: \
        fs::read_to_string(...\n\
        every-failed-read-says-why> 724 reads whose failure is taken as nothing, in 328 \
        files.\n";

    /// Three riders that each moved the lint, touching `touched` each.
    fn lint_riders(touched: [&[&str]; 3]) -> Vec<Suspect> {
        ["a", "b", "c"]
            .into_iter()
            .zip(touched)
            .map(|(head, touched)| suspect(head, Some(&[LINT, "formatting"]), Some(touched)))
            .collect()
    }

    /// The heads the lint's red is put on among `riders`, by the files
    /// it named at the candidate, `said`, and at the base, `at_base`.
    fn owners_by_paths(said: &str, at_base: &str, riders: &[Suspect]) -> Option<Vec<String>> {
        let named = BTreeMap::from([(LINT.to_owned(), named_anew(said, at_base, LINT))]);
        owners(&set(&[LINT]), riders, &named)
    }

    #[test]
    fn a_lints_words_name_the_files_it_faults() {
        assert_eq!(
            named(LINT_SAID, LINT),
            set(&[
                "modules/cadence/spec/src/main.rs",
                "tools/merge-net/src/train/excused.rs"
            ]),
            "the files the lint faulted weren't read from its words alone"
        );
        assert_eq!(
            named(LINT_SAID, "every-long-file-says-why"),
            BTreeSet::new(),
            "another check's words were read as the lint's"
        );
    }

    #[test]
    fn a_lint_every_rider_moved_falls_on_the_one_touching_what_it_names() {
        let riders = lint_riders([
            &["modules/eero/src/main.rs"],
            &[
                "modules/cadence/spec/src/main.rs",
                "modules/cadence/spec/Cargo.toml",
            ],
            &["docs/testing.md"],
        ]);
        assert_eq!(
            owners_by_paths(LINT_SAID, LINT_SAID_AT_BASE, &riders),
            Some(vec!["b".to_owned()]),
            "a lint's red wasn't put on the one rider touching the file it names"
        );
    }

    #[test]
    fn a_file_the_base_fails_blames_nobody() {
        let riders = lint_riders([
            &["tools/merge-net/src/train/excused.rs"],
            &["modules/cadence/spec/src/main.rs"],
            &["docs/testing.md"],
        ]);
        assert_eq!(
            owners_by_paths(LINT_SAID, LINT_SAID_AT_BASE, &riders),
            Some(vec!["b".to_owned()]),
            "a rider touching a file main already fails was blamed for it"
        );
        assert_eq!(
            owners_by_paths(LINT_SAID_AT_BASE, LINT_SAID_AT_BASE, &riders),
            None,
            "a red naming only what main fails was put on a rider"
        );
    }

    #[test]
    fn a_lint_naming_no_file_halves() {
        let riders = lint_riders([
            &["modules/eero/src/main.rs"],
            &["modules/cadence/spec/src/main.rs"],
            &["docs/testing.md"],
        ]);
        let said = "every-failed-read-says-why> it couldn't read the tree\n";
        assert_eq!(
            owners_by_paths(said, "", &riders),
            None,
            "a lint's red naming no file was put on a rider rather than searched"
        );
    }

    #[test]
    fn a_lint_whose_every_rider_touches_a_file_it_names_halves() {
        let riders = lint_riders([
            &["modules/cadence/spec/src/main.rs"],
            &["modules/cadence/spec/src/main.rs"],
            &["modules/cadence/spec/src/main.rs", "docs/testing.md"],
        ]);
        assert_eq!(
            owners_by_paths(LINT_SAID, LINT_SAID_AT_BASE, &riders),
            None,
            "a lint's red every rider touches was put on all of them rather than searched"
        );
    }

    #[test]
    fn a_lint_naming_a_file_no_rider_touched_halves() {
        let riders = lint_riders([
            &["modules/eero/src/main.rs"],
            &["modules/cadence/spec/src/main.rs"],
            &["docs/testing.md"],
        ]);
        assert_eq!(
            owners_by_paths(LINT_SAID, "", &riders),
            None,
            "a lint's red naming a file no rider touched was put on the others"
        );
    }

    #[test]
    fn only_a_red_wholly_on_the_list_runs_again() {
        let listed = listed(&serde_json::json!([
            {"test": "the-margin-cat-follows-the-keys-on-niri", "subtests": ["test_a"]},
        ]));
        assert!(
            flaky(&set(&["the-margin-cat-follows-the-keys-on-niri"]), &listed),
            "a listed check's red was blamed at once"
        );
        assert!(
            !flaky(
                &set(&["the-margin-cat-follows-the-keys-on-niri", "formatting"]),
                &listed
            ),
            "a red beside a listed one ran again"
        );
        assert!(
            !flaky(&BTreeSet::new(), &listed),
            "a red naming nothing ran again"
        );
    }
}
