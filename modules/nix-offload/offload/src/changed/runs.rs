//! Brings home what changed-checks keeps of each run on a lent builder:
//! the line of `changed-checks/runs.jsonl` saying how long each side's
//! evaluation and the build took. A builder is lent with nothing kept and
//! given back after 15 idle minutes, taking that file with it, so where a
//! gate's time went was read only by hand from one still lent: on 12
//! November its 277 runs that day took a median of 14.7 minutes, half of
//! it evaluating the change's own tree. Each run's lines come home to
//! `nix-offload/changed-runs.jsonl`, beside `there.jsonl`, for the daily
//! read.
//!
//! The lines a run added are those past the file's length before it, so a
//! builder lent again for the next run never sends the same ones twice.
//! Runs there overlap, nearly every one on 12 November, so what a run
//! finds past its length can hold another's line, written meanwhile; a
//! line already home is passed over, and every line comes home once, with
//! the run that wrote it or one beside it. A line still being written has
//! no end yet and is left for the run writing it. Bringing them is never
//! the run's to fail on: a copy that fails is said, and the run's status
//! stands.

use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::there::{self, Builder, Sending};

/// The file of runs on the far side, as its shell finds it: changed-checks
/// keeps it under XDG's state folder.
const FAR: &str = "\"${XDG_STATE_HOME:-$HOME/.local/state}/changed-checks/runs.jsonl\"";

/// How much of the end of the file at home is read to tell a line already
/// there: about two days of runs, far longer than runs beside each other
/// last, so the read stays small however long the file grows.
const LATELY: u64 = 1 << 20;

/// How long the file of runs is on the builder `lent` reaches now, or none
/// when ssh couldn't say.
pub fn length_on(lent: &Builder, scratch: &Path) -> Option<u64> {
    let asking = Sending {
        command: format!("wc -c 2>/dev/null < {FAR} || echo 0"),
        bundle: None,
    };
    let (ran, printed) = there::printed_on(lent, &asking, scratch, "ssh").ok()?;
    if ran.status != 0 {
        return None;
    }
    printed.trim().parse().ok()
}

/// Brings home to `home` the lines past `from` in the file of runs on the
/// builder `lent` reaches, saying why when it can't.
pub fn bring(lent: &Builder, scratch: &Path, from: Option<u64>, home: &Path) {
    let couldnt = |why: &str| {
        eprintln!(
            "nix-offload: couldn't bring changed-checks' runs home from {}: {why}",
            lent.host
        );
    };
    let Some(from) = from else {
        couldnt("its length before the run couldn't be read");
        return;
    };
    // A run that kept nothing, as one refused at once, leaves no file.
    let asking = Sending {
        command: format!("[ ! -e {FAR} ] || tail -c +{} {FAR}", from + 1),
        bundle: None,
    };
    match there::printed_on(lent, &asking, scratch, "ssh") {
        Ok((ran, found)) if ran.status == 0 => {
            if let Err(why) = kept(&found, home) {
                couldnt(&format!("{}: {why}", home.display()));
            }
        }
        Ok((ran, _)) => couldnt(&format!("reading it there exited {}", ran.status)),
        Err(why) => couldnt(&why),
    }
}

/// Adds to `home` each whole line of `found` that it doesn't hold lately,
/// and says how many it added.
pub fn kept(found: &str, home: &Path) -> io::Result<usize> {
    let lately = held_lately(home)?;
    let mut held: HashSet<&str> = lately.lines().collect();
    let mut adding = String::new();
    let whole = found.rsplit_once('\n').map_or("", |(whole, _)| whole);
    for line in whole.lines().filter(|line| !line.trim().is_empty()) {
        if held.insert(line) {
            adding.push_str(line);
            adding.push('\n');
        }
    }
    if adding.is_empty() {
        return Ok(0);
    }
    if let Some(folder) = home.parent() {
        fs::create_dir_all(folder)?;
    }
    // One write, so a run beside this one never splits a line of it.
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(home)?
        .write_all(adding.as_bytes())?;
    Ok(adding.lines().count())
}

/// The last [`LATELY`] bytes of `home`, or nothing when there's no file
/// yet.
fn held_lately(home: &Path) -> io::Result<String> {
    let mut file = match fs::File::open(home) {
        Ok(file) => file,
        Err(why) if why.kind() == io::ErrorKind::NotFound => return Ok(String::new()),
        Err(why) => return Err(why),
    };
    let length = file.metadata()?.len();
    file.seek(SeekFrom::Start(length.saturating_sub(LATELY)))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests;
