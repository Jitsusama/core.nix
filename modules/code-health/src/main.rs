//! code-health reads each repository it's given as one of its refs holds
//! it and writes the day's figures, so `cadence day` holds each against
//! the days before it and a repository getting messier is named the day it
//! does: its long files with no reason, its suppressions, its crowded
//! folders with no reason, its lines of code repeated from elsewhere, the
//! tests its merge net keeps in quarantine, the points its daemons wait
//! on to agree and the lines of its VM tests (measure.rs says how each is
//! counted); the lints its configs turn off (lints.rs), its files nothing
//! reaches (reached.rs) and its folders that don't say what they hold
//! (notes.rs), as the quality standard has them; its Terraform's files
//! not as `tofu fmt` writes them, what tflint finds, what trivy finds
//! insecure (terraform.rs) and what `tofu validate` refuses or warns
//! of (`terraform_validate.rs`), what of it says nothing of what it is
//! (`terraform_docs.rs`) and what of it nothing tests
//! (`terraform_tests.rs`); and how long the changes it merged in the last
//! day took from first edit to merge (merges.rs).
//!
//! It writes `<date>.json` in its folder as `{repo: {measure: figure}}`,
//! the form cadence reads, `<date>-named.json` beside it naming the first
//! 200 of what each figure counted, so a figure that rose is traced to its
//! files, and `<date>-files.json` counting all of them by file. A
//! repository it can't read is said and left out, as cadence leaves out
//! any source it can't read; it fails only when it reads none. It answers
//! with each repository's figures and the files it wrote, wrapped to the
//! terminal, or as `code-health.day/1` under `--json`. It writes
//! `hour.json` too, each figure held against the target `--target` gives
//! it, beside the hour before's and its owner (hour.rs), for cadence's
//! hourly read; as it runs each hour, the day's files say the last hour.
//!
//!     code-health --repo NAME=CHECKOUT... [--ref NAME=REF]...
//!         [--under NAME=FOLDER]... [--target NAME/MEASURE=FIGURE]...
//!         [--owner NAME[/MEASURE]=OWNER]... [--into FOLDER]
//!
//! A repository is read at `origin/main` unless `--ref` names another, and
//! whole unless `--under` names the folder that's it in a larger one,
//! whose merges are the larger one's, so it isn't timed; `--under` given
//! again names another folder of it, each path then said from the larger
//! one's top. A target or an
//! owner names one of MEASURES, so a misspelt one is refused rather than
//! left unjudged.

mod git;
mod hour;
mod lints;
mod measure;
mod merges;
mod notes;
mod reached;
mod terraform;
mod terraform_docs;
mod terraform_tests;
mod terraform_validate;

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use jiff::{Timestamp, Zoned};
use serde_json::json;
use terminal_kit::{Context, Exit, Failure, Help, Pen, Room, layout};

/// The merges timed: those of the last day, so an hour's few merges don't
/// swing their medians.
const DAY: i64 = 24 * 60 * 60;

/// The flags it takes, each with a value.
const FLAGS: [&str; 6] = [
    "--repo", "--ref", "--under", "--target", "--owner", "--into",
];

/// Every measure it reads (measure.rs and merges.rs).
const MEASURES: [&str; 20] = [
    "crowded-folders",
    "daemons-waiting",
    "duplicated-lines",
    "edit-to-merge",
    "files-per-change",
    "folders-without-notes",
    "lints-off",
    "long-files",
    "quarantined-tests",
    "shared-files",
    "suppressions",
    "tf-invalid",
    "tf-undocumented",
    "tf-unformatted",
    "tf-untested",
    "tf-warnings",
    "tflint-findings",
    "trivy-findings",
    "unreached-files",
    "vm-test-lines",
];

/// How a figure is said to a person, by its measure: those of the day's
/// merges are medians, and one of them is minutes.
fn figure_said(measure: &str, figure: f64) -> String {
    match measure {
        "edit-to-merge" => format!("{figure} min, the median"),
        "files-per-change" => format!("{figure}, the median"),
        _ => format!("{figure}"),
    }
}

/// A repository to read: where its checkout is, which ref, which folder
/// of it, and what it's judged by.
#[derive(Debug, Default, PartialEq)]
struct Repo {
    checkout: String,
    reference: String,
    under: Vec<String>,
    judged: hour::Judged,
}

fn help() -> Help {
    Help::new(
        "code-health",
        "writes the day's figures of how messy each repository is",
    )
    .example(
        "code-health --repo core=~/src/core.nix",
        "reads core at origin/main, for cadence",
    )
    .example(
        "code-health --repo zone=~/world --ref zone=main --under zone=home/me",
        "reads one folder of a larger repository",
    )
    .option(
        "--repo NAME=CHECKOUT",
        "a repository to read, by the name cadence says",
    )
    .option("--ref NAME=REF", "the ref it's read at; origin/main")
    .option(
        "--under NAME=FOLDER",
        "the folder that's it in a larger one",
    )
    .option(
        "--target NAME/MEASURE=FIGURE",
        "the most a measure should be, for the hour's file",
    )
    .option(
        "--owner NAME[/MEASURE]=OWNER",
        "who owns a repository, or one measure of it",
    )
    .option(
        "--into FOLDER",
        "where the day's files go; ~/.local/state/code-health",
    )
    .exit(Exit::Failed, "the day's files couldn't be written")
    .exit(Exit::NotFound, "no repository could be read")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), run)
}

fn run(context: &Context) -> Result<(), Failure> {
    let (repos, into) = parse(&context.args)
        .map_err(|what| Failure::new(Exit::Usage, what).try_this("code-health --help"))?;
    let into = into.unwrap_or_else(state);
    let mut figures = BTreeMap::new();
    let mut named = BTreeMap::new();
    let mut left_out = BTreeMap::new();
    for (name, repo) in &repos {
        match git::files_in(&repo.checkout, &repo.reference, &repo.under) {
            Ok((paths, files)) => {
                let mut health = measure::measure(&paths, &files);
                put_terraform(&mut health, &paths, &files).unwrap_or_else(|error| {
                    let said = format!("{name}'s Terraform left out: {error}");
                    let _ = context.tell("caution", "warning", &said);
                });
                if repo.under.is_empty() {
                    let since = Timestamp::now().as_second() - DAY;
                    match git::merged(&repo.checkout, &repo.reference, since) {
                        Ok(merged) => {
                            if let Some((figure, named)) = merges::edit_to_merge(&merged) {
                                health.figures.insert("edit-to-merge", figure);
                                health.named.insert("edit-to-merge", named);
                            }
                            if let Some((figure, named)) = merges::shared_files(&merged) {
                                health.figures.insert("shared-files", figure);
                                health.named.insert("shared-files", named);
                            }
                            if let Some((figure, named)) = merges::files_per_change(&merged) {
                                health.figures.insert("files-per-change", figure);
                                health.named.insert("files-per-change", named);
                            }
                        }
                        Err(error) => {
                            let _ = context.tell(
                                "caution",
                                "warning",
                                &format!("{name}'s merges left out: {error}"),
                            );
                        }
                    }
                }
                figures.insert(name.clone(), health.figures);
                named.insert(name.clone(), health.named);
            }
            Err(error) => {
                let _ = context.tell("caution", "warning", &format!("{name} left out: {error}"));
                left_out.insert(name.clone(), error);
            }
        }
    }
    if figures.is_empty() {
        return Err(Failure::new(Exit::NotFound, "no repository could be read")
            .because("each was left out, as its line above says"));
    }
    let now = Zoned::now();
    let date = now.strftime("%Y-%m-%d").to_string();
    write(&into, &date, &figures, &named).map_err(|what| Failure::new(Exit::Failed, what))?;
    let judged = repos
        .into_iter()
        .map(|(name, repo)| (name, repo.judged))
        .collect();
    let before = fs::read_to_string(into.join("hour.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok());
    let at = now.timestamp().to_string();
    let earlier = earlier_counts(&into, &date);
    let hour = hour::hour(
        &at,
        &figures,
        &named,
        &judged,
        before.as_ref(),
        earlier.as_ref(),
    );
    write_whole(&into, "hour.json", &hour).map_err(|what| Failure::new(Exit::Failed, what))?;
    let day = Day {
        date: &date,
        into: &into,
        figures: &figures,
        left_out: &left_out,
    };
    if context.format().is_json() {
        context.answer(&day.json())
    } else {
        context.say(&day.said(&context.out, context.room))
    }
    .map_err(|error| {
        Failure::new(Exit::Failed, "the answer couldn't be said").because(error.to_string())
    })
}

/// Each Terraform measure of a repository's files, where it holds any;
/// the tools' reads are left out whole when one of them can't run.
fn put_terraform(
    health: &mut measure::Health,
    paths: &[String],
    files: &[measure::File],
) -> Result<(), String> {
    if let Some(undocumented) = terraform_docs::undocumented(files) {
        let count = undocumented.len();
        health.put("tf-undocumented", undocumented, count);
    }
    if let Some(untested) = terraform_tests::untested(paths) {
        let count = untested.len();
        health.put("tf-untested", untested, count);
    }
    if let Some(read) = terraform::read(files) {
        let read = read?;
        let count = read.unformatted.len();
        health.put("tf-unformatted", read.unformatted, count);
        let count = read.findings.len();
        health.put("tflint-findings", read.findings, count);
        let count = read.insecure.len();
        health.put("trivy-findings", read.insecure, count);
        let count = read.invalid.len();
        health.put("tf-invalid", read.invalid, count);
        let count = read.warned.len();
        health.put("tf-warnings", read.warned, count);
    }
    Ok(())
}

/// What a day's read found, as it's answered.
struct Day<'a> {
    date: &'a str,
    into: &'a Path,
    figures: &'a BTreeMap<String, BTreeMap<&'static str, f64>>,
    left_out: &'a BTreeMap<String, String>,
}

impl Day<'_> {
    fn files(&self) -> [PathBuf; 4] {
        [
            self.into.join(format!("{}.json", self.date)),
            self.into.join(format!("{}-named.json", self.date)),
            self.into.join(format!("{}-files.json", self.date)),
            self.into.join("hour.json"),
        ]
    }

    /// The figures for a script, as unrounded as they're written.
    fn json(&self) -> serde_json::Value {
        json!({
            "schema": "code-health.day/1",
            "date": self.date,
            "written": self.files(),
            "figures": self.figures,
            "left_out": self.left_out,
        })
    }

    /// Each repository's figures as facts under its name, a measure's
    /// words parted where its name joins them, and the files written last.
    fn said(&self, pen: &Pen, room: Room) -> String {
        let mut blocks: Vec<Vec<String>> = self
            .figures
            .iter()
            .map(|(repo, figures)| {
                let said: Vec<(String, String)> = figures
                    .iter()
                    .map(|(measure, figure)| {
                        (measure.replace('-', " "), figure_said(measure, *figure))
                    })
                    .collect();
                let facts: Vec<(&str, Option<&str>)> = said
                    .iter()
                    .map(|(label, value)| (label.as_str(), Some(value.as_str())))
                    .collect();
                std::iter::once(pen.title(repo))
                    .chain(
                        layout::facts(&facts, "", pen, room.less(2))
                            .into_iter()
                            .map(|line| format!("  {line}")),
                    )
                    .collect()
            })
            .collect();
        let written = self
            .files()
            .map(|file| file.display().to_string())
            .join(" and ");
        blocks.push(layout::paragraph(
            &format!("Written to {written}."),
            0,
            room.for_prose(),
        ));
        layout::joined(blocks)
    }
}

/// The repositories asked for, by name, and the folder to write into.
fn parse(arguments: &[String]) -> Result<(BTreeMap<String, Repo>, Option<PathBuf>), String> {
    let mut repos: BTreeMap<String, Repo> = BTreeMap::new();
    let mut refs = BTreeMap::new();
    let mut unders = BTreeMap::new();
    let mut targets = Vec::new();
    let mut owners = Vec::new();
    let mut into = None;
    let mut given = arguments.iter();
    while let Some(flag) = given.next() {
        if !FLAGS.contains(&flag.as_str()) {
            return Err(format!(
                "no flag called {flag}; the flags are {}",
                FLAGS.join(", ")
            ));
        }
        let value = given
            .next()
            .ok_or_else(|| format!("{flag} wants a value"))?;
        if flag == "--into" {
            into = Some(PathBuf::from(value));
            continue;
        }
        let (name, said) = value
            .split_once('=')
            .ok_or_else(|| format!("{flag} wants NAME=VALUE, not {value}"))?;
        let (name, said) = (name.to_owned(), said.to_owned());
        match flag.as_str() {
            "--repo" => {
                let repo = Repo {
                    checkout: said,
                    reference: "origin/main".to_owned(),
                    ..Repo::default()
                };
                if repos.insert(name.clone(), repo).is_some() {
                    return Err(format!("{name} is named twice"));
                }
            }
            "--ref" => {
                refs.insert(name, said);
            }
            "--under" => {
                // An empty folder is the whole repository, as none is.
                let folders: &mut Vec<String> = unders.entry(name).or_default();
                if !said.is_empty() {
                    folders.push(said);
                }
            }
            "--target" => {
                let figure = said
                    .parse::<f64>()
                    .map_err(|_| format!("--target {name} wants a figure, not {said}"))?;
                targets.push((name, figure));
            }
            "--owner" => {
                owners.push((name, said));
            }
            _ => unreachable!("FLAGS names every flag but --into, taken above"),
        }
    }
    for (name, reference) in refs {
        let repo = repos
            .get_mut(&name)
            .ok_or_else(|| format!("--ref names {name}, which no --repo does"))?;
        repo.reference = reference;
    }
    for (name, under) in unders {
        let repo = repos
            .get_mut(&name)
            .ok_or_else(|| format!("--under names {name}, which no --repo does"))?;
        repo.under = under;
    }
    for (named, figure) in targets {
        let (name, measure) = measured(&named, "--target")?;
        let measure = measure.ok_or_else(|| format!("--target wants NAME/MEASURE, not {named}"))?;
        judged(&mut repos, name, "--target")?
            .targets
            .insert(measure.to_owned(), figure);
    }
    for (named, owner) in owners {
        let (name, measure) = measured(&named, "--owner")?;
        let judged = judged(&mut repos, name, "--owner")?;
        match measure {
            Some(measure) => {
                judged.owners.insert(measure.to_owned(), owner);
            }
            None => judged.owner = Some(owner),
        }
    }
    if repos.is_empty() {
        return Err("no --repo given".to_owned());
    }
    Ok((repos, into))
}

/// A `NAME/MEASURE` split, or a bare `NAME`, its measure one it reads.
fn measured<'a>(named: &'a str, flag: &str) -> Result<(&'a str, Option<&'a str>), String> {
    match named.split_once('/') {
        Some((name, measure)) if MEASURES.contains(&measure) => Ok((name, Some(measure))),
        Some((_, measure)) => Err(format!(
            "{flag} names {measure}, which isn't a measure; the measures are {}",
            MEASURES.join(", ")
        )),
        None => Ok((named, None)),
    }
}

/// What the repository a flag names is judged by.
fn judged<'a>(
    repos: &'a mut BTreeMap<String, Repo>,
    name: &str,
    flag: &str,
) -> Result<&'a mut hour::Judged, String> {
    repos
        .get_mut(name)
        .map(|repo| &mut repo.judged)
        .ok_or_else(|| format!("{flag} names {name}, which no --repo does"))
}

/// code-health's folder in the account's state.
fn state() -> PathBuf {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_default()
        .join("code-health")
}

/// Write the day's figures, the first `NAMED` of each measure's names and
/// how many of all its names each file holds, each through a file beside
/// it, so cadence never reads half of one.
fn write(
    into: &std::path::Path,
    date: &str,
    figures: &impl serde::Serialize,
    named: &BTreeMap<String, BTreeMap<&'static str, Vec<String>>>,
) -> Result<(), String> {
    let failed = |error: &dyn std::fmt::Display| format!("{}: {error}", into.display());
    fs::create_dir_all(into).map_err(|error| failed(&error))?;
    let each = |say: &dyn Fn(&[String]) -> serde_json::Value| -> serde_json::Value {
        named
            .iter()
            .map(|(repo, measures)| {
                let measures = measures
                    .iter()
                    .map(|(measure, names)| ((*measure).to_owned(), say(names)));
                (
                    repo.clone(),
                    measures.collect::<serde_json::Map<_, _>>().into(),
                )
            })
            .collect::<serde_json::Map<_, _>>()
            .into()
    };
    let first = each(&|names| json!(&names[..names.len().min(measure::NAMED)]));
    let counts = each(&|names| json!(hour::per_file(names.iter().map(String::as_str))));
    let say = |value: &serde_json::Value| {
        serde_json::to_string_pretty(value).map_err(|error| failed(&error))
    };
    let figures = serde_json::to_string_pretty(figures).map_err(|error| failed(&error))?;
    for (file, text) in [
        (format!("{date}.json"), figures),
        (format!("{date}-named.json"), say(&first)?),
        (format!("{date}-files.json"), say(&counts)?),
    ] {
        written(into, &file, &text)?;
    }
    Ok(())
}

/// Write one file whole, through a file beside it.
fn write_whole(into: &Path, file: &str, value: &impl serde::Serialize) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("{}: {error}", into.display()))?;
    written(into, file, &text)
}

/// The newest day's counts by file in `into` from before `date`, if one
/// is there and reads as JSON.
fn earlier_counts(into: &Path, date: &str) -> Option<serde_json::Value> {
    let newest = fs::read_dir(into)
        .ok()?
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|file| {
            file.strip_suffix("-files.json")
                .is_some_and(|day| day < date)
        })
        .max()?;
    let text = fs::read_to_string(into.join(newest)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Put `text` in `file` of `into` at once, so no reader sees half of it.
fn written(into: &Path, file: &str, text: &str) -> Result<(), String> {
    let failed = |error: &dyn std::fmt::Display| format!("{}: {error}", into.display());
    let path = into.join(file);
    let writing = into.join(format!(".{file}.new"));
    fs::write(&writing, format!("{text}\n")).map_err(|error| failed(&error))?;
    fs::rename(&writing, &path).map_err(|error| failed(&error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::NAMED;

    fn said(words: &str) -> Vec<String> {
        words.split(' ').map(str::to_owned).collect()
    }

    #[test]
    fn a_ref_and_folder_go_to_the_repo_they_name() {
        let (repos, into) = parse(&said(
            "--repo core=/src/core --repo zone=/world --ref zone=main --under zone=home/me \
             --into /out",
        ))
        .expect("good arguments were refused");
        assert_eq!(
            repos["zone"],
            Repo {
                checkout: "/world".to_owned(),
                reference: "main".to_owned(),
                under: vec!["home/me".to_owned()],
                judged: hour::Judged::default(),
            },
            "a repository's ref or folder was lost"
        );
        assert_eq!(
            repos["core"].reference, "origin/main",
            "a repository given no ref wasn't read at origin's main"
        );
        assert_eq!(into, Some(PathBuf::from("/out")), "--into was lost");
    }

    #[test]
    fn a_ref_for_no_repo_is_refused() {
        assert!(
            parse(&said("--repo core=/src/core --ref other=main")).is_err(),
            "a ref naming no repository was taken"
        );
        assert!(
            parse(&said("--into /out")).is_err(),
            "no repository was taken"
        );
    }

    #[test]
    fn a_target_and_owner_go_to_the_measure_they_name() {
        let (repos, _) = parse(&said(
            "--repo core=/src/core --target core/long-files=96 --owner core=someone \
             --owner core/suppressions=another",
        ))
        .expect("good targets were refused");
        let judged = &repos["core"].judged;
        assert_eq!(
            judged.targets,
            BTreeMap::from([("long-files".to_owned(), 96.0)]),
            "a target was lost"
        );
        assert_eq!(
            (
                judged.owner.as_deref(),
                judged.owners.get("suppressions").map(String::as_str)
            ),
            (Some("someone"), Some("another")),
            "a repository's or a measure's owner was lost"
        );
    }

    #[test]
    fn every_measure_read_can_be_given_a_target() {
        let quarantine = measure::File {
            path: "quarantine.json".to_owned(),
            text: "{}".to_owned(),
        };
        let health = measure::measure(&[], &[quarantine]);
        for measure in health.figures.keys() {
            assert!(MEASURES.contains(measure), "{measure} isn't in MEASURES");
        }
    }

    #[test]
    fn a_target_for_no_measure_or_repo_is_refused() {
        for given in [
            "--repo core=/src/core --target core/long-file=96",
            "--repo core=/src/core --target other/long-files=96",
            "--repo core=/src/core --target core=96",
            "--repo core=/src/core --target core/long-files=many",
            "--repo core=/src/core --owner other=someone",
        ] {
            assert!(parse(&said(given)).is_err(), "{given} was taken");
        }
    }

    #[test]
    fn an_unknown_flag_is_named_before_its_value_is_read() {
        let refused = parse(&said("--rep core")).expect_err("an unknown flag was taken");
        assert!(
            refused.starts_with("no flag called --rep"),
            "the unknown flag wasn't named: {refused}"
        );
    }

    #[test]
    fn a_merges_figure_says_it_is_a_median() {
        assert_eq!(
            figure_said("edit-to-merge", 40.5),
            "40.5 min, the median",
            "edit to merge should be said in minutes, as a median"
        );
        assert_eq!(
            figure_said("long-files", 98.0),
            "98",
            "a count should be said bare"
        );
    }

    #[test]
    fn the_days_files_hold_the_figures_cadence_reads() {
        let folder = tempfile::tempdir().expect("no folder to write in");
        let figures = BTreeMap::from([("core".to_owned(), BTreeMap::from([("long-files", 3.0)]))]);
        let named = BTreeMap::from([(
            "core".to_owned(),
            BTreeMap::from([("long-files", vec!["a.rs".to_owned()])]),
        )]);
        write(folder.path(), "2026-11-12", &figures, &named).expect("the day wasn't written");
        let written = fs::read_to_string(folder.path().join("2026-11-12.json"))
            .expect("the day's figures weren't written");
        let read: serde_json::Value = serde_json::from_str(&written).expect("not JSON");
        assert_eq!(
            read,
            serde_json::json!({"core": {"long-files": 3.0}}),
            "the figures weren't {{repo: {{measure: figure}}}}"
        );
        assert!(
            folder.path().join("2026-11-12-named.json").exists(),
            "the names weren't written beside the figures"
        );
    }

    #[test]
    fn the_days_names_are_cut_short_and_counted_whole_by_file() {
        let folder = tempfile::tempdir().expect("no folder to write in");
        let figures =
            BTreeMap::from([("core".to_owned(), BTreeMap::from([("suppressions", 0.0)]))]);
        let mut every: Vec<String> = (0..NAMED).map(|line| format!("a.rs:{line}")).collect();
        every.extend(["z.rs:1".to_owned(), "z.rs:2".to_owned()]);
        let named =
            BTreeMap::from([("core".to_owned(), BTreeMap::from([("suppressions", every)]))]);
        write(folder.path(), "2026-11-15", &figures, &named).expect("the day wasn't written");
        let read = |file: &str| -> serde_json::Value {
            let text = fs::read_to_string(folder.path().join(file))
                .unwrap_or_else(|error| panic!("{file} wasn't written: {error}"));
            serde_json::from_str(&text).expect("not JSON")
        };
        assert_eq!(
            read("2026-11-15-named.json")["core"]["suppressions"]
                .as_array()
                .map(Vec::len),
            Some(NAMED),
            "the day's names weren't cut at {NAMED}"
        );
        assert_eq!(
            read("2026-11-15-files.json"),
            json!({"core": {"suppressions": {"a.rs": NAMED, "z.rs": 2}}}),
            "each file's count wasn't of every name, past the cut too"
        );
    }

    #[test]
    fn the_day_before_is_the_newest_record_of_counts_before_today() {
        let folder = tempfile::tempdir().expect("no folder to write in");
        let into = folder.path();
        assert_eq!(
            earlier_counts(into, "2026-11-15"),
            None,
            "an empty folder gave a day before"
        );
        for (date, said) in [
            ("2026-11-12", "12"),
            ("2026-11-14", "14"),
            ("2026-11-15", "15"),
        ] {
            fs::write(
                into.join(format!("{date}-files.json")),
                format!("{{\"d\": {said}}}"),
            )
            .expect("a day's counts weren't written");
        }
        fs::write(into.join("2026-11-13.json"), "{\"d\": 13}").expect("figures weren't written");
        fs::write(into.join("2026-11-14-named.json"), "{\"d\": 0}").expect("names weren't written");
        fs::write(into.join("hour.json"), "{}").expect("the hour wasn't written");
        assert_eq!(
            earlier_counts(into, "2026-11-15"),
            Some(json!({"d": 14})),
            "the day before wasn't the newest record of counts before today"
        );
        fs::write(into.join("2026-11-14-files.json"), "half").expect("it wasn't written");
        assert_eq!(
            earlier_counts(into, "2026-11-15"),
            None,
            "a day before that isn't JSON was read as one"
        );
    }
}
