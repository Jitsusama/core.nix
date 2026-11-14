//! `cadence axes --served WORDS`: the axes as the launcher's Spec rows
//! (served.rs), printed as one line of JSON, `{"rows": [...], "head":
//! {...}}`, for cadence-spec (../../spec) to serve. The shell never rereads
//! a file rewritten each tick (Joel, 2026-11-10: "the daemons need to be
//! the data layer"), so the daemon holds the socket and asks this for the
//! rows only while Spec is watched, at most once in five minutes.
//!
//! A read that finds no figure anywhere, as when the account's records
//! can't be reached, says nothing of how Joel stands, so it prints nothing
//! and exits 1, and the daemon keeps the rows it last served (80ECO2,
//! 2026-11-14, 2768bcbb). Each read says how much processor it took, the
//! figure 80ECO2 asked to be held under a second.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use serde_json::{Value, json};

use super::Axis;
use super::readings::{gather, read};
use super::served;
use super::words::Words;
use crate::{folders, now_ms};

/// The clock ticks a second /proc counts processor time in, which Linux
/// fixes at 100 whatever the kernel's own tick.
const TICKS: f64 = 100.0;

/// `cadence axes --served WORDS`, given what follows `--served`.
pub fn run(mut args: impl Iterator<Item = String>) -> ExitCode {
    let (Some(vocabulary), None) = (args.next().map(PathBuf::from), args.next()) else {
        eprintln!("usage: cadence axes --served WORDS");
        return ExitCode::from(2);
    };
    let started = Instant::now();
    // Without the launcher's words it would serve ids rather than words.
    let words = match fs::read_to_string(&vocabulary)
        .map_err(|why| why.to_string())
        .and_then(|text| serde_json::from_str::<Value>(&text).map_err(|why| why.to_string()))
    {
        Ok(vocabulary) => Words::new(&vocabulary),
        Err(why) => {
            eprintln!(
                "cadence: can't read the words {}: {why}",
                vocabulary.display()
            );
            return ExitCode::FAILURE;
        }
    };
    let (home, state_home, folder) = match folders() {
        Ok(folders) => folders,
        Err(err) => {
            eprintln!("cadence: {err}");
            return ExitCode::FAILURE;
        }
    };
    let data_home =
        std::env::var_os("XDG_DATA_HOME").map_or_else(|| home.join(".local/share"), PathBuf::from);
    let quests = data_home.join("pi/agentic-harness.pi/quest-workflow/quests");
    let axes = read(&gather(&folder, &state_home, now_ms() / 1000));
    let ended = if read_nothing(&axes) {
        eprintln!("cadence: the axes read nothing; Spec keeps its last rows");
        ExitCode::FAILURE
    } else {
        let made = served::made(&axes, &titles(&quests, &axes), &words);
        println!("{}", json!({"rows": made.rows, "head": made.head}));
        ExitCode::SUCCESS
    };
    if let Some(used) = processor() {
        let took = started.elapsed().as_secs_f64();
        eprintln!(
            "cadence: read the axes in {:.2} s of processor, {took:.2} s in all",
            used / TICKS
        );
    }
    ended
}

/// Whether no reading of any axis found a figure.
fn read_nothing(axes: &[Axis]) -> bool {
    axes.iter()
        .all(|axis| axis.readings.iter().all(|r| r.now.is_none()))
}

/// The processor time this process has used, in clock ticks, from
/// /proc/self/stat's user and system fields (the 14th and 15th).
fn processor() -> Option<f64> {
    let stat = fs::read_to_string("/proc/self/stat").ok()?;
    // The command's name may hold spaces, so fields count from its end.
    let after = stat.rsplit_once(')')?.1;
    let mut fields = after.split_whitespace().skip(11);
    let user: f64 = fields.next()?.parse().ok()?;
    let system: f64 = fields.next()?.parse().ok()?;
    Some(user + system)
}

/// The title of each owner's quest, by address, from the H1 of its
/// README in `quests`, whose folders end in the address
/// (QEST-20261111-1JQPPC is 1JQPPC's). An owner with none is left out,
/// and served by address.
fn titles(quests: &Path, axes: &[Axis]) -> BTreeMap<String, String> {
    let owners: BTreeSet<&str> = axes
        .iter()
        .flat_map(|axis| axis.readings.iter().map(|r| r.owner))
        .collect();
    let Ok(folders) = fs::read_dir(quests) else {
        return BTreeMap::new();
    };
    folders
        .filter_map(Result::ok)
        .filter_map(|folder| {
            let name = folder.file_name().into_string().ok()?;
            let address = name.rsplit_once('-')?.1;
            owners.get(address)?;
            let readme = fs::read_to_string(folder.path().join("README.md")).ok()?;
            Some((address.to_owned(), title(&readme)?))
        })
        .collect()
}

/// A README's H1, past its frontmatter.
fn title(readme: &str) -> Option<String> {
    let body = readme
        .strip_prefix("---\n")
        .and_then(|front| front.split_once("\n---\n"))
        .map_or(readme, |(_, body)| body);
    body.lines()
        .find_map(|line| line.strip_prefix("# "))
        .map(|title| title.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quest_is_titled_by_its_first_heading_past_its_frontmatter() {
        let readme = "---\nid: QEST-1\n# not a heading in yaml\n---\n\n# The Loop\n\n## Purpose\n";
        assert_eq!(title(readme), Some("The Loop".to_owned()));
        assert_eq!(
            title("---\nid: x\n---\n# The Loop\n"),
            Some("The Loop".to_owned())
        );
        assert_eq!(title("no heading"), None);
    }

    #[test]
    fn a_read_that_finds_nothing_is_told_from_one_that_finds_a_figure() {
        use crate::axes::Reading;
        let axis = |now| Axis {
            name: "scale",
            findings: Vec::new(),
            readings: vec![Reading::new("{} pi windows", now, "", "OSPHT4")],
        };
        assert!(
            read_nothing(&[axis(None), axis(None)]),
            "no figure anywhere"
        );
        assert!(!read_nothing(&[axis(None), axis(Some(3.0))]), "one figure");
    }

    #[test]
    fn processor_time_is_read_from_proc() {
        assert!(
            processor().is_some(),
            "/proc/self/stat gives this process's time"
        );
    }
}
