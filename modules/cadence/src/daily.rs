//! `cadence day`: reads the day just ended from the machine's records,
//! keeps it as `day/<date>.json` beside the days before, holds it against
//! them, writes the note to `day/<date>.txt`, and sends it to the orchestrator.
//! Each finding goes into the ledger (findings.rs), and each owner is
//! told of its new ones.
//! A program does it, not a session, so the loop costs no model turns.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::builders;
use crate::day::{self, Day};
use crate::findings;
use crate::findings::{gone, hubs};
use crate::gate;
use crate::health;
use crate::holders;
use crate::judge;
use crate::machine;
use crate::mesh::{self, Mesh};
use crate::speed;
use crate::{DAY, command, message, write_whole};
use crate::{passed, phases};

/// Whom the day is told to: the orchestrator its note, each owner its
/// findings by the prefix of their keys, and the watcher what no owner has.
/// None of them given, as when Joel types it, the note is only printed.
#[derive(Default)]
pub struct Told {
    pub orchestrator: Option<String>,
    pub watcher: Option<String>,
    pub owners: BTreeMap<String, String>,
}

/// Reads, keeps and judges the day that ends at `now`, brings the ledger
/// up to it, and tells each of `told` its part.
pub fn run(folder: &Path, state_home: &Path, told: &Told, now: i64) -> io::Result<()> {
    let days = folder.join("day");
    fs::create_dir_all(&days)?;
    let mesh = Mesh {
        root: state_home.join("pi/agentic-harness.pi"),
    };
    let known = mesh.known();
    let held = |unit: &str| holders::owner(unit, &told.owners, &known).is_some();
    let today = read(state_home, now - DAY, now, &held);
    let before = kept(&days, now);
    let judged = judge::judge(&today, &before);
    let kept_at = folder.join("findings.jsonl");
    let mut ledger = findings::read(&kept_at);
    // An owner with no session to answer leaves its findings with the
    // watcher, and the orchestrator hears which flags to change.
    let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .map(|b| b.trim().to_owned())
        .unwrap_or_default();
    let rosters = mesh.rostered(now);
    let running = |p: &mesh::Presence| {
        mesh::running(p, &boot, &rosters, |pid| {
            Path::new(&format!("/proc/{pid}")).exists()
        })
    };
    let gone = gone::owners(&told.owners, &ledger, &mesh.all(), running, now);
    gone::update_without(
        &mut ledger,
        &today,
        &judged.worse,
        &told.owners,
        &known,
        &gone,
    );
    // A typed run tells nobody, so it marks nobody told or late.
    let sending = told.orchestrator.is_some();
    let (late, put) = if sending {
        put_late(&mesh, &mut ledger, state_home, running, now)?
    } else {
        (Vec::new(), 0)
    };
    let asks = if sending {
        findings::tell(&mut ledger, &today.date, told.watcher.as_deref())
    } else {
        Vec::new()
    };
    findings::keep(&kept_at, &ledger)?;
    // Its own line past the note's room, so it crowds out no finding.
    let mut note = judge::note(&today, &judged, before.len(), &late);
    if let Some(line) = gone::flags(&told.owners, &gone, told.watcher.as_deref()) {
        note.push_str("- ");
        note.push_str(&line);
        note.push_str(".\n");
    }
    match put {
        0 => {}
        1 => note.push_str("- 1 late finding went back to its subject's hub as a need.\n"),
        n => {
            let _ = writeln!(
                note,
                "- {n} late findings went back to their subjects' hubs as needs."
            );
        }
    }
    let mut record = serde_json::to_value(&today)?;
    record["worse"] = serde_json::to_value(&judged.worse)?;
    record["better"] = serde_json::to_value(&judged.better)?;
    write_whole(
        &days.join(format!("{}.json", today.date)),
        &serde_json::to_string_pretty(&record)?,
    )?;
    write_whole(&days.join(format!("{}.txt", today.date)), &note)?;
    print!("{note}");
    if let Some(orchestrator) = &told.orchestrator {
        // Findings, so an owner reads them with its next turn rather than
        // being woken for them, which costs a cold start each.
        for (to, text) in asks {
            mesh.send(&message(&to, "finding", text))?;
        }
        mesh.send(&message(orchestrator, "fyi", note))?;
    }
    Ok(())
}

/// Puts each late finding in `ledger` to its subject's hub, as the node
/// names hubs, keeping a need for each in the mesh's day file, and gives
/// back the lines for the orchestrator of those no hub took, and how many
/// went to hubs. With no node to ask, every late finding is a line.
fn put_late(
    mesh: &Mesh,
    ledger: &mut [findings::Entry],
    state_home: &Path,
    running: impl Fn(&mesh::Presence) -> bool,
    now: i64,
) -> io::Result<(Vec<String>, usize)> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
    let realm = std::env::var("MYCELIUM_REALM").ok();
    let hubs = runtime
        .and_then(|r| hubs::socket(&r, state_home, realm))
        .map(|socket| hubs::listed(&socket))
        .unwrap_or_default();
    let live = |address: &str| mesh.presence(address).is_some_and(|p| running(&p));
    let (put, late) = hubs::put_late(ledger, &hubs, live, now);
    let cells = mesh.known().len();
    hubs::keep(
        &mesh.root,
        &hubs::needs(&put, crate::FROM, now * 1000, cells),
    )?;
    Ok((late, put.len()))
}

/// The days kept before the one ending at `now`: the newest `BEFORE` that
/// ended at least half a day and at most eight days before it, so a run
/// again the same evening isn't held against itself.
fn kept(days: &Path, now: i64) -> Vec<Day> {
    let mut kept: Vec<Day> = fs::read_dir(days)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|e| e == "json"))
        .filter_map(|entry| serde_json::from_slice::<Day>(&fs::read(entry.path()).ok()?).ok())
        .filter(|d| d.to <= now - DAY / 2 && d.to > now - 8 * DAY)
        .collect();
    kept.sort_by_key(|d| std::cmp::Reverse(d.to));
    kept.truncate(judge::BEFORE);
    kept
}

/// The day from `from` to `to`, read from each source; a source that
/// can't be read leaves its figures out rather than failing the day.
/// The launcher's figures, from the lines the shell's unit writes.
fn launcher(since: &str, until: &str) -> BTreeMap<String, f64> {
    let shell = command(
        "journalctl",
        &[
            "--user",
            "-u",
            "quickshell.service",
            "-o",
            "cat",
            "--no-pager",
            since,
            until,
            "--grep=launcher: ",
        ],
    );
    speed::launcher(&shell.unwrap_or_default())
}

/// Puts the figures of what a change waits on before it merges into
/// `day`: the builders, merge-net's answers, how long gates took and
/// where that time went.
fn gates(day: &mut Day, state_home: &Path) {
    builders::read(day, state_home);
    gate::read(day, state_home);
    passed::read(day, state_home);
    phases::read(day, state_home);
}

/// The runs systemd-run started from a day before `from`, so a run begun
/// the day before that failed in this one is known too; one begun before
/// that is told when it fails, as is every failure when none can be read.
fn systemd_runs(from: i64, until: &str) -> holders::Runs {
    let started = command(
        "journalctl",
        &[
            "-o",
            "json",
            "--output-fields=MESSAGE,UNIT,USER_UNIT,INVOCATION_ID,USER_INVOCATION_ID,\
             _PID,_SYSTEMD_CGROUP",
            "--no-pager",
            &format!("--since=@{}", from - DAY),
            until,
            &format!("MESSAGE_ID={}", holders::STARTED),
            "--grep=\\[systemd-run\\] ",
        ],
    );
    let uid = fs::metadata("/proc/self").map_or(u32::MAX, |m| m.uid());
    holders::systemd_runs(&started.unwrap_or_default(), uid)
}

/// Each unit that failed between `since` and `until`, from systemd's
/// message for a unit that failed, the account's units too, but for runs.
fn failed(
    since: &str,
    until: &str,
    runs: &holders::Runs,
    held: &dyn Fn(&str) -> bool,
) -> BTreeMap<String, u32> {
    let failed = command(
        "journalctl",
        &[
            "-o",
            "json",
            "--no-pager",
            since,
            until,
            "MESSAGE_ID=d9b373ed55a64feb8242e02dbe79a49c",
        ],
    );
    day::failed_units(&failed.unwrap_or_default(), runs, held)
}

fn read(state_home: &Path, from: i64, to: i64, held: &dyn Fn(&str) -> bool) -> Day {
    let since = format!("--since=@{from}");
    let until = format!("--until=@{to}");
    let mut day = Day {
        date: local_date(to),
        from,
        to,
        ..Day::default()
    };

    // coredumpctl says it found none with a failure, which is no crash.
    let dumps = command(
        "coredumpctl",
        &["--json=short", "list", &since, &until, "--no-pager"],
    );
    day.named.insert(
        "crashes".to_owned(),
        day::crashes(&dumps.unwrap_or_default()),
    );
    let runs = systemd_runs(from, &until);
    day.named
        .insert("failed".to_owned(), failed(&since, &until, &runs, held));

    let kernel = command(
        "journalctl",
        &[
            "-k",
            "-o",
            "short-unix",
            "--no-pager",
            &since,
            &until,
            "--grep=oom-kill:",
        ],
    );
    let (machine, cgroup) = day::oom_kills(&kernel.unwrap_or_default());
    day.figures
        .insert("oom.machine".to_owned(), f64::from(machine));
    day.figures
        .insert("oom.cgroup".to_owned(), f64::from(cgroup));

    // lag-watch names each file by the UTC date its records end on.
    let mut lines = String::new();
    let mut dates = vec![utc_date(from), utc_date(to)];
    dates.dedup();
    for date in dates {
        let path = state_home.join(format!("lag-watch/{date}.jsonl"));
        lines.push_str(&fs::read_to_string(path).unwrap_or_default());
    }
    for (key, value) in day::pressure(&lines, from, to) {
        if day::measures().any(|m| m.key == key) {
            day.figures.insert(key, value);
        }
    }

    day.figures.extend(launcher(&since, &until));

    let updates = command(
        "journalctl",
        &[
            "-u",
            "update-to-main.service",
            "-o",
            "short-unix",
            "--no-pager",
            &since,
            &until,
        ],
    );
    #[expect(clippy::cast_precision_loss, reason = "a time in seconds")]
    let read = day::switches(&updates.unwrap_or_default(), to as f64);
    if let Some(median) = day::median(&read.lengths) {
        day.figures
            .insert("switch.minutes.median".to_owned(), median);
    }
    // One still building counts as long as it has run so far.
    let longest = read.lengths.iter().copied().chain(read.under_way);
    if let Some(longest) = longest.reduce(f64::max) {
        day.figures
            .insert("switch.minutes.longest".to_owned(), longest);
    }
    gates(&mut day, state_home);

    day.figures
        .insert("switch.pieces.here".to_owned(), f64::from(read.here));
    day.figures
        .insert("switch.pieces.heavy".to_owned(), f64::from(read.heavy));

    // code-health writes each repository's figures before the day is read.
    let health = state_home.join(format!("code-health/{}.json", day.date));
    day.figures.extend(health::figures(
        &fs::read_to_string(health).unwrap_or_default(),
    ));
    // health writes its day as it goes, so it's read as far as it's got.
    let machine = state_home.join(format!("health/{}.json", day.date));
    if let Ok(json) = fs::read_to_string(machine) {
        day.figures.extend(machine::figures(&json));
        day.named.insert(
            "unstamped".to_owned(),
            machine::unstamped(&json, &runs, held),
        );
    }
    day
}

fn local_date(at: i64) -> String {
    Timestamp::from_second(at)
        .map(|t| {
            t.to_zoned(TimeZone::system())
                .strftime("%Y-%m-%d")
                .to_string()
        })
        .unwrap_or_default()
}

fn utc_date(at: i64) -> String {
    Timestamp::from_second(at)
        .map(|t| t.strftime("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_kept_today_isnt_held_against_itself() {
        let folder = tempfile::tempdir().expect("a scratch folder");
        let now = 10 * DAY;
        for (name, to) in [
            ("a", now - DAY),
            ("b", now - 2 * DAY),
            ("c", now - 60),
            ("d", now - 9 * DAY),
        ] {
            let day = Day {
                date: name.to_owned(),
                to,
                ..Day::default()
            };
            let text = serde_json::to_string(&day).expect("a day says itself");
            fs::write(folder.path().join(format!("{name}.json")), text).expect("written");
        }
        fs::write(folder.path().join("a.txt"), "a note").expect("written");
        let dates: Vec<String> = kept(folder.path(), now)
            .into_iter()
            .map(|d| d.date)
            .collect();
        assert_eq!(
            dates,
            vec!["a", "b"],
            "a day kept this evening, or one past eight days, was held against today"
        );
    }
}
