//! code-health reads each repository it's given as one of its refs holds
//! it and writes the day's figures, so `cadence day` holds each against
//! the days before it and a repository getting messier is named the day it
//! does: its long files with no reason, its suppressions, its crowded
//! folders with no reason, its lines of code repeated from elsewhere, the
//! tests its merge net keeps in quarantine and the points its daemons wait
//! on to agree (measure.rs says how each is counted), and how long the
//! changes it merged in the last day took from first edit to merge
//! (merges.rs).
//!
//! It writes `<date>.json` in its folder as `{repo: {measure: figure}}`,
//! the form cadence reads, and `<date>-named.json` beside it naming what
//! each figure counted, so a figure that rose is traced to its files. A
//! repository it can't read is said and left out, as cadence leaves out
//! any source it can't read; it fails only when it reads none. It answers
//! with each repository's figures and the files it wrote, wrapped to the
//! terminal, or as `code-health.day/1` under `--json`.
//!
//!     code-health --repo NAME=CHECKOUT... [--ref NAME=REF]...
//!         [--under NAME=FOLDER]... [--into FOLDER]
//!
//! A repository is read at `origin/main` unless `--ref` names another, and
//! whole unless `--under` names the folder that's it in a larger one,
//! whose merges are the larger one's, so it isn't timed.

mod git;
mod measure;
mod merges;

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use jiff::{Timestamp, Zoned};
use serde_json::json;
use terminal_kit::{Context, Exit, Failure, Help, Pen, Room, layout};

/// The merges timed: those of the last day, as the read runs each day.
const DAY: i64 = 24 * 60 * 60;

/// The flags it takes, each with a value.
const FLAGS: [&str; 4] = ["--repo", "--ref", "--under", "--into"];

/// How a figure is said to a person, by its measure: those of the day's
/// merges are medians, and one of them is minutes.
fn figure_said(measure: &str, figure: f64) -> String {
    match measure {
        "edit-to-merge" => format!("{figure} min, the median"),
        "files-per-change" => format!("{figure}, the median"),
        _ => format!("{figure}"),
    }
}

/// A repository to read: where its checkout is, which ref, and which
/// folder of it.
#[derive(Debug, PartialEq)]
struct Repo {
    checkout: String,
    reference: String,
    under: String,
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
        match git::files(&repo.checkout, &repo.reference, &repo.under) {
            Ok((paths, files)) => {
                let mut health = measure::measure(&paths, &files);
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
    let date = Zoned::now().strftime("%Y-%m-%d").to_string();
    write(&into, &date, &figures, &named).map_err(|what| Failure::new(Exit::Failed, what))?;
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

/// What a day's read found, as it's answered.
struct Day<'a> {
    date: &'a str,
    into: &'a Path,
    figures: &'a BTreeMap<String, BTreeMap<&'static str, f64>>,
    left_out: &'a BTreeMap<String, String>,
}

impl Day<'_> {
    fn files(&self) -> [PathBuf; 2] {
        [
            self.into.join(format!("{}.json", self.date)),
            self.into.join(format!("{}-named.json", self.date)),
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
                    under: String::new(),
                };
                if repos.insert(name.clone(), repo).is_some() {
                    return Err(format!("{name} is named twice"));
                }
            }
            "--ref" => {
                refs.insert(name, said);
            }
            "--under" => {
                unders.insert(name, said);
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
    if repos.is_empty() {
        return Err("no --repo given".to_owned());
    }
    Ok((repos, into))
}

/// code-health's folder in the account's state.
fn state() -> PathBuf {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_default()
        .join("code-health")
}

/// Write the day's figures and names, each through a file beside it, so
/// cadence never reads half of one.
fn write(
    into: &std::path::Path,
    date: &str,
    figures: &impl serde::Serialize,
    named: &impl serde::Serialize,
) -> Result<(), String> {
    let failed = |error: &dyn std::fmt::Display| format!("{}: {error}", into.display());
    fs::create_dir_all(into).map_err(|error| failed(&error))?;
    let figures = serde_json::to_string_pretty(figures).map_err(|error| failed(&error))?;
    let named = serde_json::to_string_pretty(named).map_err(|error| failed(&error))?;
    for (file, text) in [
        (format!("{date}.json"), figures),
        (format!("{date}-named.json"), named),
    ] {
        let path = into.join(&file);
        let writing = into.join(format!(".{file}.new"));
        fs::write(&writing, text + "\n").map_err(|error| failed(&error))?;
        fs::rename(&writing, &path).map_err(|error| failed(&error))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
                under: "home/me".to_owned(),
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
}
