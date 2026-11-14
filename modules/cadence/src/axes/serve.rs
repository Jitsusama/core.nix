//! `cadence serve`: serves the axes to the launcher's Spec place over the
//! socket systemd holds for it, as ready rows (served.rs), each watch told
//! only the rows that moved. The shell never rereads a file rewritten each
//! tick (Joel, 2026-11-10: "the daemons need to be the data layer"), so the
//! rows are made here.
//!
//! It reads the axes only while something watches Spec, and at most once
//! in five minutes, the pace cadence's own tick keeps; a watch begun
//! between reads is answered from the rows last made. A read that finds
//! nothing, or fails, leaves the last rows served rather than an empty
//! Spec (80ECO2, 2026-11-14, 2768bcbb). Each read says in the journal how
//! much processor it took.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::panic;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use daemon_serve::asks::Domain;
use daemon_serve::serve::{self, Shared};
use ready_rows::kept::Kept;
use serde_json::{Value, json};

use super::Axis;
use super::readings::{gather, read};
use super::served::{self, PLACE};
use super::words::Words;
use crate::{folders, now_ms};

/// The daemon as the launcher knows it: Spec's rows and its headline.
const DOMAIN: Domain = Domain {
    name: "cadence-spec",
    kinds: &["row", "head"],
    sole: &[],
    named_only: &[],
};

pub const USAGE: &str = "cadence serve --vocabulary WORDS --socket SOCKET";

/// How long the rows last made stand before a watch has them read again.
const FRESH: Duration = Duration::from_mins(5);

/// The clock ticks a second /proc counts processor time in, which Linux
/// fixes at 100 whatever the kernel's own tick.
const TICKS: f64 = 100.0;

/// `cadence serve --vocabulary WORDS --socket SOCKET`.
pub fn run(args: impl Iterator<Item = String>) -> ExitCode {
    let args: Vec<String> = args.collect();
    let named = |flag: &str| {
        let at = args.iter().position(|one| one == flag)?;
        args.get(at + 1).map(PathBuf::from)
    };
    let (Some(vocabulary), Some(socket)) = (named("--vocabulary"), named("--socket")) else {
        eprintln!("usage: {USAGE}");
        return ExitCode::from(2);
    };
    // Started without the launcher's words, it would serve ids rather
    // than words, so it says why and stops for systemd to start again.
    let words = match fs::read_to_string(&vocabulary)
        .map_err(|why| why.to_string())
        .and_then(|text| serde_json::from_str::<Value>(&text).map_err(|why| why.to_string()))
    {
        Ok(vocabulary) => Words::new(&vocabulary),
        Err(why) => {
            eprintln!(
                "cadence: can't read the vocabulary {}: {why}",
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
    let shared = Shared::new(DOMAIN, Box::new(|_| None));
    let (wake, woken) = mpsc::channel();
    shared.on_watches(Box::new(move || {
        let _ = wake.send(());
    }));
    match serve::listen(&socket) {
        Ok(listener) => serve::accept(std::sync::Arc::clone(&shared), listener),
        Err(why) => {
            eprintln!("cadence: can't listen on {}: {why}", socket.display());
            return ExitCode::FAILURE;
        }
    }
    let mut kept = Kept::default();
    let mut last: Option<Instant> = None;
    loop {
        let watched = shared.watches("row", PLACE) || shared.watches("head", PLACE);
        if watched && last.is_none_or(|at| at.elapsed() >= FRESH) {
            last = Some(Instant::now());
            if let Some(made) = timed(|| reading(&folder, &state_home, &quests, &words))
                && !kept.take(made.rows, vec![made.head]).is_empty()
            {
                shared.change(kept.all().cloned().collect(), json!({}));
            }
        }
        // Woken as a watch begins or ends, else when the rows go stale.
        let wait = last.map_or(FRESH, |at| FRESH.saturating_sub(at.elapsed()));
        if let Err(mpsc::RecvTimeoutError::Disconnected) = woken.recv_timeout(wait) {
            return ExitCode::SUCCESS;
        }
    }
}

/// Spec's rows as the axes read now, or none where the read failed or
/// found nothing at all, so the last rows stay served.
fn reading(folder: &Path, state_home: &Path, quests: &Path, words: &Words) -> Option<served::Made> {
    let now = now_ms() / 1000;
    let axes = panic::catch_unwind(|| read(&gather(folder, state_home, now)));
    let Ok(axes) = axes else {
        eprintln!("cadence: reading the axes failed; Spec keeps its last rows");
        return None;
    };
    if read_nothing(&axes) {
        eprintln!("cadence: the axes read nothing; Spec keeps its last rows");
        return None;
    }
    Some(served::made(&axes, &titles(quests, &axes), words))
}

/// Whether no reading of any axis found a figure, as when the account's
/// records can't be reached, which says nothing of how Joel stands.
fn read_nothing(axes: &[Axis]) -> bool {
    axes.iter()
        .all(|axis| axis.readings.iter().all(|r| r.now.is_none()))
}

/// Runs `read`, saying in the journal how much processor it took, the
/// figure 80ECO2 asked to be held under a second a read.
fn timed<T>(read: impl FnOnce() -> T) -> T {
    let (before, started) = (processor(), Instant::now());
    let made = read();
    if let (Some(before), Some(after)) = (before, processor()) {
        let used = (after - before) / TICKS;
        let took = started.elapsed().as_secs_f64();
        eprintln!("cadence: read the axes in {used:.2} s of processor, {took:.2} s in all");
    }
    made
}

/// The processor time this process has used so far, in clock ticks, from
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
