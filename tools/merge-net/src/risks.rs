//! What the net tells the dates ledger: a line for each risk it sees to
//! when a pull request merges, so the ledger marks each date it touches as
//! at risk when the risk happens, not once the date has passed. On 27
//! November a train split at 10:23 was seen to miss a date only at 11:31.
//!
//! Each risk is one JSON line appended to `risks.jsonl` in the mission's
//! folder under XDG's state (`~/.local/state/mission/risks.jsonl`), or to
//! the file `MERGE_NET_RISKS` names, for tests:
//! `{at, pr, repo, risk, run, eta_added_min, why}`. `at` is RFC 3339 in the
//! machine's own zone, with its offset; `pr` is the pull request's number,
//! or null where none was found; `repo` is the slug the net watches; `risk`
//! is `refused`, `red`, `split` or `step-failed`; `run` is the train's
//! candidate, or the commit `ready` was asked of; `eta_added_min` is the
//! table's estimate below; `why` is one plain line.
//!
//! Lines are only ever appended, never rewritten, and each in one write of
//! under 4 KB, so two writers at once leave whole lines. The ledger is
//! told, not asked: a line that can't be written is said once on stderr
//! and never fails what the net was doing.

use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::json;

/// A risk the net sees to when a pull request merges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Risk {
    /// `ready` refused a head.
    Refused,
    /// The train went red on a candidate, and a rider fell to run again
    /// alone.
    Red,
    /// The train split a candidate, so its riders wait on another run.
    Split,
    /// A step of `ready` failed to run at all.
    StepFailed,
}

/// The minutes each risk adds to a pull request's date, which the ledger
/// takes as an estimate, not a measure: about what a run again alone
/// costs after a refusal, a run alone and its wait after a red, the next
/// candidate's run after a split, and asking `ready` again after a step
/// that couldn't run.
const ADDED_MINUTES: [(Risk, u64); 4] = [
    (Risk::Refused, 30),
    (Risk::Red, 60),
    (Risk::Split, 45),
    (Risk::StepFailed, 20),
];

impl Risk {
    /// The risk's name, as the ledger reads it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Refused => "refused",
            Self::Red => "red",
            Self::Split => "split",
            Self::StepFailed => "step-failed",
        }
    }

    /// The minutes it adds to a date, from [`ADDED_MINUTES`].
    pub fn added_minutes(self) -> u64 {
        ADDED_MINUTES
            .iter()
            .find(|(risk, _)| *risk == self)
            .map_or(0, |(_, minutes)| *minutes)
    }
}

/// One risk, as the net saw it.
#[derive(Clone, Copy, Debug)]
pub struct Seen<'a> {
    /// The pull request it falls on, where one was found.
    pub pr: Option<u64>,
    pub risk: Risk,
    /// The train's candidate, or the commit `ready` was asked of.
    pub run: &'a str,
    pub why: &'a str,
}

/// How many bytes of `why` a line keeps, and of `run`: escaped, a `why`
/// this long stays well under the 4 KB a line must keep to, so two
/// appends at once can't interleave.
const WHY_BYTES: usize = 1500;
const RUN_BYTES: usize = 200;

/// The line a risk is told as, seen `at`, on the repository `repo`.
pub fn line(at: &str, repo: &str, seen: &Seen) -> String {
    let said = json!({
        "at": at,
        "pr": seen.pr,
        "repo": repo,
        "risk": seen.risk.name(),
        "run": plain(seen.run, RUN_BYTES),
        "eta_added_min": seen.risk.added_minutes(),
        "why": plain(seen.why, WHY_BYTES),
    });
    format!("{said}\n")
}

/// `text` as one plain line of at most `bytes` bytes: each run of spaces
/// and line ends one space, cut short at a character's edge.
fn plain(text: &str, bytes: usize) -> String {
    let mut line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.len() > bytes {
        let mut end = bytes;
        while !line.is_char_boundary(end) {
            end -= 1;
        }
        line.truncate(end);
    }
    line
}

/// Where the ledger reads risks from: the file `MERGE_NET_RISKS` names, or
/// `mission/risks.jsonl` under XDG's state, `~/.local/state` unless
/// `XDG_STATE_HOME` names another. Each is given as the environment holds
/// it, so a test reads it without changing its own.
pub fn file_from(
    named: Option<OsString>,
    state: Option<OsString>,
    home: Option<OsString>,
) -> Option<PathBuf> {
    let given = |value: Option<OsString>| value.filter(|value| !value.is_empty());
    if let Some(named) = given(named) {
        return Some(PathBuf::from(named));
    }
    let state = given(state)
        .map(PathBuf::from)
        .or_else(|| given(home).map(|home| PathBuf::from(home).join(".local/state")))?;
    Some(state.join("mission/risks.jsonl"))
}

/// Append `line` to `file`, its folder made where it isn't, in one write.
/// A file opened to append moves each write to its end whole, so a line
/// under 4 KB lands whole beside another writer's.
pub fn append(file: &Path, line: &str) -> io::Result<()> {
    if let Some(folder) = file
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty())
    {
        fs::create_dir_all(folder)?;
    }
    let mut opened = OpenOptions::new().append(true).create(true).open(file)?;
    let wrote = opened.write(line.as_bytes())?;
    if wrote == line.len() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "only {wrote} of the line's {} bytes were written",
            line.len()
        )))
    }
}

/// Where a command tells the ledger of the risks it sees, for the
/// repository it watches.
pub struct Risks {
    file: Option<PathBuf>,
    repo: String,
    /// Whether a line that couldn't be written was said, so a command that
    /// sees many risks says so once.
    said: AtomicBool,
}

impl Risks {
    /// The ledger's file as the environment names it, for the repository
    /// `repo`, its slug.
    pub fn from_env(repo: &str) -> Self {
        let file = file_from(
            std::env::var_os("MERGE_NET_RISKS"),
            std::env::var_os("XDG_STATE_HOME"),
            std::env::var_os("HOME"),
        );
        Self::new(file, repo)
    }

    /// The ledger's file at `file`, none where it can't be found, for the
    /// repository `repo`.
    pub fn new(file: Option<PathBuf>, repo: &str) -> Self {
        Self {
            file,
            repo: repo.to_owned(),
            said: AtomicBool::new(false),
        }
    }

    /// Tell the ledger of `seen`, now, saying on stderr once that it
    /// couldn't.
    pub fn tell(&self, seen: &Seen) {
        self.tell_saying(seen, &now(), &mut |said| eprintln!("merge-net: {said}"));
    }

    /// Tell the ledger of `seen`, seen `at`, giving `say` the one line
    /// saying it couldn't, the first time it can't.
    pub fn tell_saying(&self, seen: &Seen, at: &str, say: &mut dyn FnMut(&str)) {
        let told = match &self.file {
            Some(file) => append(file, &line(at, &self.repo, seen))
                .map_err(|error| format!("{}: {error}", file.display())),
            None => Err("no XDG_STATE_HOME or HOME says where it goes".to_owned()),
        };
        if let Err(why) = told
            && !self.said.swap(true, Ordering::Relaxed)
        {
            say(&format!(
                "the dates ledger wasn't told of a {} risk at {}, and isn't told of this \
                 command's others: {why}",
                seen.risk.name(),
                seen.run
            ));
        }
    }
}

/// What this command said last as `ready`'s refusal, for the line that
/// tells the ledger of it. `ready` refuses from many places, and each
/// says why on stderr as it goes; this keeps the last of them. It ends
/// as nothing when `ready` ends by boarding the train, or holding a head
/// for an earlier gate, which end 1 as a refusal does but cost its date
/// no run again.
static REFUSAL: Mutex<String> = Mutex::new(String::new());

/// Keep `said` as what `ready` refused for.
pub fn heard(said: &str) {
    if let Ok(mut refusal) = REFUSAL.lock() {
        said.trim_start_matches("merge-net: ")
            .clone_into(&mut refusal);
    }
}

/// Forget any refusal kept, since `ready` boarded the head on the train,
/// which is how a head merges now, not a risk to its date.
pub fn boarded() {
    if let Ok(mut refusal) = REFUSAL.lock() {
        refusal.clear();
    }
}

/// What `ready` last said it refused for, or nothing.
pub fn refusal() -> String {
    REFUSAL
        .lock()
        .map(|refusal| refusal.clone())
        .unwrap_or_default()
}

/// Now, in RFC 3339 with the machine's own offset.
fn now() -> String {
    let seconds = i64::try_from(crate::state::now()).unwrap_or_default();
    local_rfc3339(seconds)
}

/// The moment `seconds` after 1970 began, in RFC 3339 as the machine's
/// zone reads it, with its offset: `2026-11-27T10:23:00-05:00`. Where the
/// C library can't place it, it's said in UTC, which RFC 3339 reads alike.
pub fn local_rfc3339(seconds: i64) -> String {
    let Some(local) = terminal_kit::local::at(seconds) else {
        return terminal_kit::time::rfc3339(seconds);
    };
    // The zone's offset is how far its clock reads from UTC's at the
    // moment, taken to the minute, as a leap second's 60th reads one off.
    let read = days_from_civil(
        i64::from(local.year),
        i64::from(local.month),
        i64::from(local.day),
    ) * 86_400
        + i64::from(local.hour) * 3_600
        + i64::from(local.minute) * 60
        + i64::from(local.second.min(59));
    let minutes = (read - seconds + 30).div_euclid(60);
    let sign = if minutes < 0 { '-' } else { '+' };
    format!(
        "{}T{:02}:{:02}:{:02}{sign}{:02}:{:02}",
        local.date(),
        local.hour,
        local.minute,
        local.second,
        minutes.abs() / 60,
        minutes.abs() % 60
    )
}

/// The days from 1970's first to the date `year`-`month`-`day`, by Howard
/// Hinnant's civil arithmetic, as the kit's dates are reckoned.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let from_march = (month + 9) % 12;
    let of_year = (153 * from_march + 2) / 5 + day - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * 146_097 + of_cycle - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// A folder of the test's own.
    fn folder(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("merge-net-risks-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        folder
    }

    #[test]
    fn each_risk_is_told_in_the_ledgers_shape() {
        let at = "2026-11-27T10:23:00-05:00";
        for (risk, name, minutes, pr) in [
            (Risk::Refused, "refused", 30, Some(6900)),
            (Risk::Red, "red", 60, Some(6901)),
            (Risk::Split, "split", 45, Some(6902)),
            (Risk::StepFailed, "step-failed", 20, None),
        ] {
            let seen = Seen {
                pr,
                risk,
                run: "0123456789abcdef0123456789abcdef01234567",
                why: "the candidate failed\nchanged-checks",
            };
            let told = line(at, "Jitsusama/core.nix", &seen);
            assert!(
                told.ends_with('\n') && told.matches('\n').count() == 1,
                "a {name} risk wasn't told as one line: {told:?}"
            );
            let read: Value = serde_json::from_str(&told).unwrap();
            let expected = json!({
                "at": at,
                "pr": pr,
                "repo": "Jitsusama/core.nix",
                "risk": name,
                "run": "0123456789abcdef0123456789abcdef01234567",
                "eta_added_min": minutes,
                "why": "the candidate failed changed-checks",
            });
            assert_eq!(read, expected, "a {name} risk read back otherwise");
            assert_eq!(
                serde_json::to_string(&read).unwrap(),
                told.trim_end(),
                "a {name} risk's line isn't what its fields write again"
            );
        }
    }

    #[test]
    fn a_long_why_is_cut_so_the_line_stays_under_4_kb() {
        let why = "\u{e9}\"".repeat(4000);
        let seen = Seen {
            pr: Some(1),
            risk: Risk::Red,
            run: &"r".repeat(4000),
            why: &why,
        };
        let told = line("2026-11-27T10:23:00-05:00", "Jitsusama/core.nix", &seen);
        assert!(told.len() < 4096, "a line ran to {} bytes", told.len());
        assert!(
            serde_json::from_str::<Value>(&told).is_ok(),
            "a line cut short no longer reads as JSON: {told}"
        );
    }

    #[test]
    fn two_writers_at_once_leave_whole_lines() {
        let folder = folder("together");
        let file = folder.join("mission/risks.jsonl");
        let lines = 200;
        std::thread::scope(|scope| {
            for writer in ["one", "two"] {
                let file = &file;
                scope.spawn(move || {
                    let why = format!("{writer} {}", "w".repeat(2000));
                    let risks = Risks::new(Some(file.clone()), "Jitsusama/core.nix");
                    for at in 0..lines {
                        let seen = Seen {
                            pr: Some(at),
                            risk: Risk::Split,
                            run: writer,
                            why: &why,
                        };
                        risks.tell_saying(&seen, "2026-11-27T10:23:00-05:00", &mut |said| {
                            panic!("a write failed: {said}");
                        });
                    }
                });
            }
        });
        let written = fs::read_to_string(&file).unwrap();
        let read: Vec<Value> = written
            .lines()
            .map(|line| serde_json::from_str(line).expect("a line came apart"))
            .collect();
        for writer in ["one", "two"] {
            assert_eq!(
                read.iter().filter(|line| line["run"] == writer).count(),
                200,
                "{writer}'s lines weren't all there, whole"
            );
        }
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_file_that_cant_be_written_is_said_once_and_the_command_goes_on() {
        let folder = folder("unwritable");
        fs::create_dir_all(&folder).unwrap();
        let blocker = folder.join("mission");
        fs::write(&blocker, "a file where the folder would be").unwrap();
        let risks = Risks::new(Some(blocker.join("risks.jsonl")), "Jitsusama/core.nix");
        let mut said = Vec::new();
        for run in ["c1", "c2"] {
            let seen = Seen {
                pr: None,
                risk: Risk::Red,
                run,
                why: "it fell",
            };
            risks.tell_saying(&seen, "2026-11-27T10:23:00-05:00", &mut |line| {
                said.push(line.to_owned());
            });
        }
        assert_eq!(
            said.len(),
            1,
            "two failed writes weren't said once: {said:?}"
        );
        assert!(
            said[0].contains("dates ledger") && said[0].contains(&blocker.display().to_string()),
            "the failed write didn't say what and where: {}",
            said[0]
        );
        let nowhere = Risks::new(None, "Jitsusama/core.nix");
        let mut said = Vec::new();
        let seen = Seen {
            pr: None,
            risk: Risk::Refused,
            run: "c3",
            why: "it was refused",
        };
        nowhere.tell_saying(&seen, "2026-11-27T10:23:00-05:00", &mut |line| {
            said.push(line.to_owned());
        });
        assert_eq!(
            said.len(),
            1,
            "a ledger found nowhere wasn't said: {said:?}"
        );
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn the_ledger_is_where_the_variable_or_xdgs_state_says() {
        let given = |value: &str| Some(OsString::from(value));
        assert_eq!(
            file_from(given("/t/r.jsonl"), given("/s"), given("/h")),
            Some(PathBuf::from("/t/r.jsonl")),
            "MERGE_NET_RISKS didn't name the file"
        );
        assert_eq!(
            file_from(given(""), given("/s"), given("/h")),
            Some(PathBuf::from("/s/mission/risks.jsonl")),
            "the file wasn't under XDG_STATE_HOME"
        );
        assert_eq!(
            file_from(None, None, given("/h")),
            Some(PathBuf::from("/h/.local/state/mission/risks.jsonl")),
            "the file wasn't under the home's state"
        );
        assert_eq!(
            file_from(None, None, None),
            None,
            "a file was found nowhere"
        );
    }

    #[test]
    fn a_moment_is_said_with_its_zones_offset() {
        let seconds = 1_795_792_980;
        let said = local_rfc3339(seconds);
        let (clock, offset) = said.split_at(said.len() - 6);
        let sign = if offset.starts_with('-') { -1 } else { 1 };
        let hours: i64 = offset[1..3].parse().unwrap();
        let minutes: i64 = offset[4..6].parse().unwrap();
        let field = |from: usize, to: usize| clock[from..to].parse::<i64>().unwrap();
        let read = days_from_civil(field(0, 4), field(5, 7), field(8, 10)) * 86_400
            + field(11, 13) * 3_600
            + field(14, 16) * 60
            + field(17, 19);
        assert_eq!(
            read - sign * (hours * 3_600 + minutes * 60),
            seconds,
            "{said} isn't the moment it was given"
        );
        assert_eq!(days_from_civil(1970, 1, 1), 0, "1970 didn't begin at day 0");
        assert_eq!(
            days_from_civil(2026, 11, 27),
            20_784,
            "27 November 2026 wasn't day 20,784"
        );
    }
}
