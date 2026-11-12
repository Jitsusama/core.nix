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
//! any source it can't read; it fails only when it reads none.
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
use std::path::PathBuf;
use std::process::ExitCode;

use jiff::{Timestamp, Zoned};

/// The merges timed: those of the last day, as the read runs each day.
const DAY: i64 = 24 * 60 * 60;

/// A repository to read: where its checkout is, which ref, and which
/// folder of it.
#[derive(Debug, PartialEq)]
struct Repo {
    checkout: String,
    reference: String,
    under: String,
}

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let (repos, into) = match parse(&arguments) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("code-health: {error}");
            return ExitCode::from(2);
        }
    };
    let into = into.unwrap_or_else(state);
    let mut figures = BTreeMap::new();
    let mut named = BTreeMap::new();
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
                        Err(error) => eprintln!("code-health: {name}'s merges left out: {error}"),
                    }
                }
                eprintln!("code-health: {name}: {:?}", health.figures);
                figures.insert(name.clone(), health.figures);
                named.insert(name.clone(), health.named);
            }
            Err(error) => eprintln!("code-health: {name} left out: {error}"),
        }
    }
    if figures.is_empty() {
        eprintln!("code-health: no repository was read");
        return ExitCode::FAILURE;
    }
    let date = Zoned::now().strftime("%Y-%m-%d").to_string();
    match write(&into, &date, &figures, &named) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("code-health: {error}");
            ExitCode::FAILURE
        }
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
            _ => return Err(format!("{flag} isn't something code-health takes")),
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
