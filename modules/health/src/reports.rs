//! A Mac's crash reports, read into the crashes a Linux machine's core
//! dumps are. macOS says no line naming the program when one crashes; it
//! writes a report into a `DiagnosticReports` folder instead, so the follower
//! reads each new report there.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use serde_json::Value;

use crate::journal::{Entry, Whose};
use crate::unified;

/// Where the system keeps its reports; the account's are in the same place
/// under its home.
const REPORTS: &str = "Library/Logs/DiagnosticReports";

/// The kind of report a program's crash is. macOS writes others beside it,
/// such as what a program spent of the disk or the processor, which aren't
/// crashes.
const CRASH: &str = "309";

/// How a report writes its moment: local time, to the hundredth, with its
/// offset.
const WRITTEN: &str = "%Y-%m-%d %H:%M:%S%.f %z";

/// The folders macOS writes crash reports into: the system's, for every
/// account's daemons and the system's own, and the account's.
#[must_use]
pub fn folders(home: Option<&Path>) -> Vec<PathBuf> {
    let mut folders = vec![Path::new("/").join(REPORTS)];
    folders.extend(home.map(|home| home.join(REPORTS)));
    folders
}

/// The crashes the reports in `folders` tell of that aren't in `seen`,
/// each report kept in `seen` once read. A report that doesn't read yet is
/// left unseen, since macOS may be writing it still.
pub fn look(folders: &[PathBuf], seen: &mut BTreeSet<PathBuf>) -> Vec<Entry> {
    let mut crashes = Vec::new();
    for folder in folders {
        let Ok(listed) = fs::read_dir(folder) else {
            continue;
        };
        for path in listed.flatten().map(|file| file.path()) {
            if path.extension().is_none_or(|kind| kind != "ips") || seen.contains(&path) {
                continue;
            }
            let Ok(report) = fs::read_to_string(&path) else {
                continue;
            };
            let Some((what, body)) = report.split_once('\n') else {
                continue;
            };
            let Ok(Value::Object(what)) = serde_json::from_str::<Value>(what) else {
                continue;
            };
            if what.get("bug_type").and_then(Value::as_str) == Some(CRASH)
                && serde_json::from_str::<Value>(body).is_err()
            {
                continue;
            }
            seen.insert(path);
            crashes.extend(parse(&report));
        }
    }
    crashes
}

/// The crash one report holds, or none when it isn't a program's crash:
/// its first line says what it is, and the rest is the crash.
#[must_use]
pub fn parse(report: &str) -> Option<Entry> {
    let (what, body) = report.split_once('\n')?;
    let Ok(Value::Object(what)) = serde_json::from_str::<Value>(what) else {
        return None;
    };
    let text = |name: &str| what.get(name).and_then(Value::as_str);
    if text("bug_type") != Some(CRASH) {
        return None;
    }
    let at = jiff::fmt::strtime::parse(WRITTEN, text("timestamp")?)
        .ok()?
        .to_timestamp()
        .ok()
        .map(Timestamp::as_microsecond)?;
    let program = text("name").or_else(|| text("app_name"))?.to_owned();
    let body = serde_json::from_str::<Value>(body).ok()?;
    let image = body
        .get("procPath")
        .and_then(Value::as_str)
        .unwrap_or_default();
    // A program run from an app is one Joel opened, as an app's lines are
    // his programs'; one run from no app is a daemon's or an agent's tool.
    let whose = if unified::app_of(image).is_some() {
        Whose::His
    } else {
        Whose::Agents
    };
    Some(Entry {
        at,
        // A crash is as grave as systemd's core dump line says it is.
        priority: 2,
        thing: program.clone(),
        message: format!("{program} crashed"),
        ran_in: None,
        crashed: Some(program),
        whose: Some(whose),
        restart: false,
        kernel: false,
        boot: String::new(),
        since_boot: 0,
        mark: None,
        ask: None,
        job: None,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::day::{Day, Kind};
    use crate::journal::Whose;

    /// A report of a program of Joel's crashing, cut to the fields it
    /// always gives: a line of what it is, then the crash itself.
    const APP: &str = concat!(
        r#"{"app_name":"WezTerm","timestamp":"2026-11-13 09:49:52.00 -0500","#,
        r#""app_version":"1","bundleID":"com.github.wez.wezterm","bug_type":"309","#,
        r#""os_version":"macOS 26.1","name":"WezTerm","incident_id":"8E9F"}"#,
        "\n",
        r#"{
  "uptime" : 82000,
  "procPath" : "\/Applications\/Nix Apps\/WezTerm.app\/Contents\/MacOS\/wezterm-gui",
  "pid" : 4301,
  "exception" : {"type" : "EXC_BAD_ACCESS", "signal" : "SIGSEGV"}
}"#
    );

    /// A report of a program that ran from no app crashing.
    const DAEMON: &str = concat!(
        r#"{"app_name":"nix","timestamp":"2026-11-13 04:12:07.00 -0500","#,
        r#""bug_type":"309","name":"nix","incident_id":"1A2B"}"#,
        "\n",
        r#"{
  "procPath" : "\/nix\/store\/abc-nix-2.31\/bin\/nix",
  "pid" : 912
}"#
    );

    /// A report that's no crash: what a program spent, which macOS writes
    /// too.
    const SPENT: &str = concat!(
        r#"{"bug_type":"298","timestamp":"2026-11-13 04:00:00.00 -0500","#,
        r#""name":"disk writes"}"#,
        "\n",
        r#"{"spent" : 1}"#
    );

    #[test]
    fn a_report_is_its_programs_crash_and_says_whose() {
        let app = parse(APP).expect("a crash is an entry");
        assert_eq!(
            app.crashed.as_deref(),
            Some("WezTerm"),
            "the program crashed"
        );
        assert_eq!(app.thing, "WezTerm", "the program names the thing");
        assert_eq!(
            app.at, 1_794_581_392_000_000,
            "the time is read with its offset"
        );
        assert_eq!(app.whose, Some(Whose::His), "an app's program is Joel's");
        let daemon = parse(DAEMON).expect("a crash is an entry");
        assert_eq!(
            daemon.crashed.as_deref(),
            Some("nix"),
            "the program crashed"
        );
        assert_eq!(
            daemon.whose,
            Some(Whose::Agents),
            "a program in no app is ours"
        );
        assert_eq!(parse(SPENT), None, "what a program spent isn't a crash");
        assert_eq!(
            parse("not a report"),
            None,
            "a half-written report isn't one yet"
        );
    }

    #[test]
    fn each_report_is_counted_once_as_a_programs_crash() {
        let system = tempfile::tempdir().expect("a folder");
        let account = tempfile::tempdir().expect("a folder");
        fs::write(system.path().join("nix-2026-11-13-041207.ips"), DAEMON).expect("a report");
        fs::write(system.path().join("disk writes-2026-11-13.ips"), SPENT).expect("a report");
        fs::write(account.path().join("WezTerm-2026-11-13-094952.ips"), APP).expect("a report");
        fs::write(account.path().join("WezTerm.diag"), APP).expect("not a report");
        let folders = [system.path().to_owned(), account.path().to_owned()];
        let mut seen = BTreeSet::new();
        let mut day = Day::new("2026-11-13");
        for entry in look(&folders, &mut seen) {
            day.take(&entry);
        }
        let crashes = |name: &str| {
            day.things
                .get(name)
                .map(|thing| (thing.kind, thing.crashes, thing.his_crashes))
        };
        assert_eq!(
            crashes("WezTerm"),
            Some((Some(Kind::Programs), 1, 1)),
            "Joel's app crashed once: {:?}",
            day.things.keys().collect::<Vec<_>>()
        );
        assert_eq!(
            crashes("nix"),
            Some((Some(Kind::Programs), 1, 0)),
            "nix crashed once, ours"
        );
        assert_eq!(day.things.len(), 2, "what was spent isn't a crash");
        assert!(
            look(&folders, &mut seen).is_empty(),
            "a report read once isn't read again"
        );
    }

    #[test]
    fn a_mac_reads_the_systems_reports_and_the_accounts() {
        assert_eq!(
            folders(Some(Path::new("/Users/someone"))),
            [
                PathBuf::from("/Library/Logs/DiagnosticReports"),
                PathBuf::from("/Users/someone/Library/Logs/DiagnosticReports"),
            ],
            "both folders"
        );
        assert_eq!(
            folders(None),
            [PathBuf::from("/Library/Logs/DiagnosticReports")],
            "the system's alone, with no home"
        );
    }
}
