//! code-health reads each repository it's given as one of its refs holds
//! it and writes the day's figures, so `cadence day` holds each against
//! the days before it and a repository getting messier is named the day it
//! does: its long files with no reason, its suppressions, its crowded
//! folders with no reason, its lines of code repeated from elsewhere, the
//! tests its merge net keeps in quarantine, the points its daemons wait on
//! to agree and the lines of its VM tests (measure.rs says how each is
//! counted, attributes.rs how a Rust line turns a lint off, language.rs
//! which language each file is written in); the lints its
//! configs turn off (lints.rs), its files nothing reaches (reached.rs) and
//! its folders that don't say what they hold (notes.rs), as the quality
//! standard has them; its Terraform's files not as `tofu fmt` writes them,
//! what tflint finds, what trivy finds insecure (terraform.rs) and what
//! `tofu validate` refuses or warns of (`terraform_validate.rs`), what of
//! it says nothing of what it is (`terraform_docs.rs`) and what of it
//! nothing tests (`terraform_tests.rs`); and how long the changes it
//! merged in the last day took from first edit to merge (merges.rs).
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
//!         [--owner NAME[/MEASURE|:FOLDER]=OWNER]... [--into FOLDER]
//!
//! A repository is read at `origin/main` unless `--ref` names another, and
//! whole unless `--under` names the folder that's it in a larger one,
//! whose merges are the larger one's, so it isn't timed; `--under` given
//! again names another folder of it, each path then said from the larger
//! one's top. A target or an owner names one of the measures it reads, so
//! a misspelt one is refused rather than left unjudged; an owner of a
//! FOLDER, said from the repository's top as its files are, owns what the
//! hour names in it (options.rs).
//!
//!     code-health stale TREE WAITING
//!
//! holds one measure at a change rather than counting it each hour: it
//! fails naming each comment in the folder TREE that names a path gone
//! (stale.rs), but those the file WAITING lists as `file path`, one a
//! line and once for each mention, and each line listed more often than
//! it's found, so the list only shrinks (tree.rs). The same rule then
//! counts each hour and holds each change.

mod attributes;
mod git;
mod hour;
mod language;
mod lints;
mod measure;
mod merges;
mod notes;
mod options;
mod reached;
mod stale;
mod terraform;
mod terraform_docs;
mod terraform_tests;
mod terraform_validate;
mod tree;

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

/// How a figure is said to a person, by its measure: those of the day's
/// merges are medians, and one of them is minutes.
fn figure_said(measure: &str, figure: f64) -> String {
    match measure {
        "edit-to-merge" => format!("{figure} min, the median"),
        "files-per-change" => format!("{figure}, the median"),
        _ => format!("{figure}"),
    }
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
    .example(
        "code-health stale ~/src/core.nix waiting.txt",
        "fails on a comment naming a path that's gone, but those waiting",
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
        "--owner NAME[/MEASURE|:FOLDER]=OWNER",
        "who owns a repository, one measure of it, or a folder",
    )
    .option(
        "--into FOLDER",
        "where the day's files go; ~/.local/state/code-health",
    )
    .exit(Exit::Failed, "the day's files couldn't be written")
    .exit(Exit::NotFound, "no repository could be read")
    .exit(
        Exit::Refused,
        "stale: a comment names what's gone, or the list what's back",
    )
}

/// `code-health stale TREE WAITING`: each comment in `tree` naming a path
/// that's gone, but those `waiting` lists, said and failed on, as is each
/// line of `waiting` listed more often than it's found.
fn stale_held(context: &Context, tree: &str, waiting: &str) -> Result<(), Failure> {
    let (paths, files) = tree::files(Path::new(tree))
        .map_err(|what| Failure::new(Exit::NotFound, "the tree couldn't be read").because(what))?;
    let listed = fs::read_to_string(waiting).map_err(|why| {
        Failure::new(Exit::NotFound, "the waiting list couldn't be read")
            .because(format!("{waiting}: {why}"))
    })?;
    let listed: Vec<String> = listed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    let (new, gone) = tree::held(&stale::stale(&paths, &files), &listed);
    let mut said = Vec::new();
    if !new.is_empty() {
        said.push("These comments name what isn't there; name where it is now:".to_owned());
        said.extend(new.iter().map(|named| format!("  {named}")));
    }
    if !gone.is_empty() {
        said.push("These wait on less now; list each as often as it's found:".to_owned());
        said.extend(gone.iter().map(|line| format!("  {line}")));
    }
    if said.is_empty() {
        return Ok(());
    }
    // Said as lines, each ended, so the refusal after them starts its own.
    let mut said = said.join("\n");
    said.push('\n');
    context.say(&said).map_err(|error| {
        Failure::new(Exit::Failed, "the answer couldn't be said").because(error.to_string())
    })?;
    Err(
        Failure::new(Exit::Refused, "a comment or the waiting list is stale")
            .because("each is named above"),
    )
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), run)
}

/// The folders a repository's owners are given that hold no file read,
/// each warned of, since what they should own goes to another.
fn stray_owners(
    context: &Context,
    name: &str,
    repo: &options::Repo,
    paths: &[String],
) -> Vec<String> {
    let read = match repo.under.as_slice() {
        [] => repo.reference.clone(),
        [under] => format!(
            "{} under {under} (whose folders count from inside it)",
            repo.reference
        ),
        unders => format!(
            "{} under {} (whose folders count from the repository's top)",
            repo.reference,
            unders.join(", ")
        ),
    };
    let stray = hour::astray(&repo.judged.folders, paths);
    for folder in &stray {
        let said = format!(
            "{name}'s owner of {folder:?} owns nothing: no file read at {read} is given to \
             it, so what it should own goes to the owner of a folder above it, else the \
             measure's or {name}'s"
        );
        let _ = context.tell("caution", "warning", &said);
    }
    stray.into_iter().map(str::to_owned).collect()
}

fn run(context: &Context) -> Result<(), Failure> {
    if let [mode, tree, waiting] = context.args.as_slice()
        && mode == "stale"
    {
        return stale_held(context, tree, waiting);
    }
    let (repos, into) = options::parse(&context.args)
        .map_err(|what| Failure::new(Exit::Usage, what).try_this("code-health --help"))?;
    let into = into.unwrap_or_else(state);
    let mut figures = BTreeMap::new();
    let mut named = BTreeMap::new();
    let mut left_out = BTreeMap::new();
    for (name, repo) in &repos {
        match git::files_in(&repo.checkout, &repo.reference, &repo.under) {
            Ok((paths, files)) => {
                let stray = stray_owners(context, name, repo, &paths);
                let roots = git::roots(&repo.under);
                let mut health = measure::measure(&paths, &files, &roots);
                let count = stray.len();
                health.put(hour::STRAY_OWNERS, stray, count);
                put_terraform(&mut health, &paths, &files).unwrap_or_else(|error| {
                    let said = format!("{name}'s Terraform left out: {}", joined(&error));
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
                                &format!("{name}'s merges left out: {}", joined(&error)),
                            );
                        }
                    }
                }
                figures.insert(name.clone(), health.figures);
                named.insert(name.clone(), health.named);
            }
            Err(error) => {
                let said = format!("{name} left out: {}", joined(&error));
                let _ = context.tell("caution", "warning", &said);
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
    let days = newest_days(&into);
    let hour = hour::hour(
        &at,
        &figures,
        &named,
        &judged,
        before.as_ref(),
        earlier.as_ref(),
        &days,
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

/// What a tool said, its lines joined with semicolons, since the kit
/// holds a warning to one line and would otherwise run them together. A
/// line ending in a colon already leads into the next, so only a space
/// follows it.
fn joined(said: &str) -> String {
    let mut out = String::new();
    for line in said.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if !out.is_empty() {
            out.push_str(if out.ends_with(':') { " " } else { "; " });
        }
        out.push_str(line);
    }
    out
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

/// The newest `hour::DAYS` records of the day's figures, today's among
/// them, oldest first; one that can't be read is left out.
fn newest_days(into: &Path) -> Vec<serde_json::Value> {
    let Ok(entries) = fs::read_dir(into) else {
        return Vec::new();
    };
    let mut dated: Vec<String> = entries
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|file| {
            file.strip_suffix(".json")
                .is_some_and(|day| jiff::civil::Date::strptime("%Y-%m-%d", day).is_ok())
        })
        .collect();
    dated.sort();
    let newest = &dated[dated.len().saturating_sub(hour::DAYS)..];
    newest
        .iter()
        .filter_map(|file| serde_json::from_str(&fs::read_to_string(into.join(file)).ok()?).ok())
        .collect()
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

    #[test]
    fn a_tools_lines_are_joined_for_a_warning() {
        assert_eq!(
            joined("tofu validate ended 2 in infra:\n\n  Error: bad\nmore\n"),
            "tofu validate ended 2 in infra: Error: bad; more",
            "a tool's lines weren't each kept, joined on one line"
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

    #[test]
    fn the_newest_days_are_the_last_week_of_figures_oldest_first() {
        let folder = tempfile::tempdir().expect("no folder to write in");
        let into = folder.path();
        for day in 1..=9 {
            fs::write(
                into.join(format!("2026-11-0{day}.json")),
                format!("{{\"d\": {day}}}"),
            )
            .expect("a day's figures weren't written");
        }
        fs::write(into.join("2026-11-08.json"), "half").expect("it wasn't written");
        fs::write(into.join("2026-11-09-files.json"), "{\"d\": 0}")
            .expect("counts weren't written");
        fs::write(into.join("hour.json"), "{}").expect("the hour wasn't written");
        let days: Vec<_> = newest_days(into)
            .iter()
            .map(|day| day["d"].clone())
            .collect();
        assert_eq!(
            days,
            [3, 4, 5, 6, 7, 9].map(|day| json!(day)),
            "the newest week of figures wasn't read, oldest first, without what's unreadable"
        );
    }
}
