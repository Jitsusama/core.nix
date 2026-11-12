//! What each axis is read by: every source as text, each parsed by a
//! function of its own so it's tested without the machine, and the one
//! function that reads them all from the machine, cheaply.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde::Deserialize;
use serde_json::Value;

use super::{Axis, Bar, Reading, shown};
use super::{counted, landings, plans};
use crate::day::Day;
use crate::mesh::{self, Mesh};
use crate::{DAY, command};

/// Everything the axes are read from, as text, so reading them is tested
/// without the machine.
#[derive(Debug, Default)]
pub struct Sources {
    /// The daily measures, oldest first.
    pub measures: Vec<String>,
    pub card: String,
    /// cadence's newest day, when one has been kept.
    pub day: Option<Day>,
    /// The systemd units failed now, one a line.
    pub failed: String,
    /// The mesh's messages of the last day, and its presences that say
    /// live for a pi that's gone.
    pub messages: Vec<Note>,
    pub ghosts: Option<usize>,
    /// `$XDG_RUNTIME_DIR/jobs/quests.json`.
    pub quests: String,
    /// Each file under `mission/waiting/`.
    pub waiting: Vec<String>,
    /// `niri msg -j windows`, when niri answered.
    pub windows: Option<String>,
    /// The shell's launcher lines since midnight.
    pub launcher: String,
    /// `notices/notices.json`.
    pub notices: String,
    /// `/proc/pressure/cpu` and `/proc/meminfo`.
    pub pressure: String,
    pub meminfo: String,
    /// What spend is judged by: the changes that reached him, and the bill.
    pub landings: landings::Sources,
    /// The harness's changes, read the same way.
    pub harness: landings::Sources,
    /// The active quests and whether they're worked on and planned.
    pub plans: plans::Sources,
    /// What code-health and health have already counted.
    pub counted: counted::Sources,
    /// Now, in seconds, and the moment the reading's day began.
    pub now: i64,
    pub midnight: i64,
}

/// A mesh message, as far as the axes read one.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Note {
    pub id: String,
    pub intent: String,
    #[serde(default)]
    pub re: Option<String>,
}

/// The nine axes, read from `s`.
#[must_use]
pub fn read(s: &Sources) -> Vec<Axis> {
    let (over, keys) = launcher(&s.launcher);
    vec![
        resources(s),
        time(s),
        quality(s),
        collaboration(s),
        axis(
            "security",
            [
                vec![counted::musts(&s.counted, s.now)],
                counted::asks(&s.counted),
            ]
            .concat(),
        ),
        organization(s),
        scale(s),
        axis(
            "polish",
            vec![
                Reading::new(
                    "{} launcher keys over 50 ms today",
                    Some(count(over)),
                    "",
                    "Y2YBTS",
                )
                .bar(Bar::AtMost(0.0))
                .detail(Some(format!("of {keys} keys")))
                .from("the shell's journal")
                .his(),
            ],
        ),
        stress(s),
    ]
}

/// Why a dollar figure is held back: the daily measure, spend-watch and
/// the bill tell different stories of a day, and none goes before Joel
/// until QCXZEB, who owns spend, says which is true.
const DISAGREE: &str = "its sources disagree, and QCXZEB is settling which is true";

fn resources(s: &Sources) -> Axis {
    // Spend is the bill per core change that reached him and stayed, beside
    // how many reached him, so a cheaper day that brought fewer is out.
    let (bill, changes, _) = landings::readings(&s.landings, s.now, &TimeZone::system());
    let figure = |key: &str| s.day.as_ref().and_then(|d| d.figures.get(key).copied());
    let (_, swap) = crate::records::memory(&s.meminfo);
    let now = crate::records::avg300(&s.pressure);
    axis(
        "resources",
        vec![
            bill,
            changes,
            landings::harness(&s.harness, s.now, &TimeZone::system()),
            Reading::new(
                "spent on cache rewrites after compaction {}",
                s.measures.last().and_then(|m| after_compaction(m)),
                "$",
                "QCXZEB",
            )
            .bar(Bar::AtMost(100.0))
            .from("the daily measure's cache rewrites")
            .held(DISAGREE),
            Reading::new(
                "agents' CPU pressure {} over the day",
                figure("pressure.cpu.jobs"),
                "%",
                "5FWIUS",
            )
            .bar(Bar::AtMost(15.0))
            .detail(now.map(|n| {
                format!(
                    "the machine's {} now, {} GiB of swap",
                    shown(n, "%"),
                    shown(swap, "")
                )
            }))
            .from("cadence's day, lag-watch's figures"),
            Reading::new(
                "{} machine-wide out-of-memory kills over the day",
                figure("oom.machine"),
                "",
                "80ECO2",
            )
            .bar(Bar::AtMost(0.0))
            .from("cadence's day, the kernel's journal"),
            counted::one_call(s.measures.last().map(String::as_str)),
        ],
    )
}

fn time(s: &Sources) -> Axis {
    let waiting = card_waiting(&s.card);
    let on = card_median(&s.card, "Merge to on").or_else(|| {
        // None landed in the hour: the oldest still waiting is how long
        // merge to on is running.
        waiting.iter().copied().reduce(f64::max)
    });
    axis(
        "time to done",
        vec![
            Reading::new("merge to on {}", on, " min", "80ECO2")
                .bar(Bar::AtMost(18.0))
                .detail(
                    (!waiting.is_empty()).then(|| format!("{} merges not on yet", waiting.len())),
                )
                .from("cadence's card"),
            Reading::new(
                "last commit to merge {}",
                card_median(&s.card, "Last commit to merge"),
                " min",
                "80ECO2",
            )
            .bar(Bar::AtMost(20.0))
            .from("cadence's card"),
            Reading::new(
                "VM wait {}",
                card_median(&s.card, "VM wait"),
                " min",
                "5FWIUS",
            )
            .bar(Bar::AtMost(2.0))
            .from("cadence's card"),
        ],
    )
}

fn quality(s: &Sources) -> Axis {
    let crashes = s.day.as_ref().and_then(|d| d.named.get("crashes"));
    let most = crashes.map(|c| {
        let mut named: Vec<(&String, &u32)> = c.iter().collect();
        named.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        named
            .iter()
            .take(3)
            .map(|(name, n)| format!("{name} {n}"))
            .collect::<Vec<_>>()
            .join(", ")
    });
    let failed: Vec<&str> = s.failed.lines().filter(|l| !l.trim().is_empty()).collect();
    axis(
        "quality",
        vec![
            Reading::new(
                "{} crashes over the day",
                crashes.map(|c| f64::from(c.values().sum::<u32>())),
                "",
                "80ECO2",
            )
            .bar(Bar::AtMost(5.0))
            .detail(most.filter(|m| !m.is_empty()).map(|m| format!("most {m}")))
            .from("cadence's day, coredumpctl"),
            Reading::new(
                "{} units failed now",
                Some(count(failed.len())),
                "",
                "80ECO2",
            )
            .bar(Bar::AtMost(0.0))
            .detail((!failed.is_empty()).then(|| {
                failed
                    .iter()
                    .filter_map(|l| l.split_whitespace().next())
                    .collect::<Vec<_>>()
                    .join(", ")
            }))
            .from("systemctl --failed, the system's and the account's"),
            Reading::new(
                "the net {} behind main",
                card_net(&s.card),
                " min",
                "D5QEKR",
            )
            .bar(Bar::AtMost(60.0))
            .from("cadence's card"),
            landings::readings(&s.landings, s.now, &TimeZone::system()).2,
            counted::regressions(&s.counted),
        ],
    )
}

fn collaboration(s: &Sources) -> Axis {
    let all = s.messages.len();
    let fyi = s.messages.iter().filter(|m| m.intent == "fyi").count();
    let answered: BTreeSet<&str> = s.messages.iter().filter_map(|m| m.re.as_deref()).collect();
    let asked_by = (s.now - 30 * 60) * 1000;
    let unanswered = s
        .messages
        .iter()
        .filter(|m| m.intent == "question" && !answered.contains(m.id.as_str()))
        .filter(|m| mesh::stamped_at(&m.id).is_some_and(|at| at < asked_by))
        .count();
    let read = all > 0;
    axis(
        "collaboration",
        vec![
            Reading::new(
                "{} of the mesh's messages are fyi",
                read.then(|| percent(fyi, all)),
                "%",
                "85PKB2",
            )
            .bar(Bar::AtMost(40.0))
            .detail(read.then(|| {
                format!(
                    "{} of {} in the last day",
                    shown(count(fyi), ""),
                    shown(count(all), "")
                )
            }))
            .from("the mesh's messages"),
            Reading::new(
                "{} questions with no answer after 30 min",
                read.then_some(count(unanswered)),
                "",
                "85PKB2",
            )
            .bar(Bar::AtMost(0.0))
            .detail(Some(
                "over the last day, an answer from any peer counting".into(),
            ))
            .from("the mesh's messages"),
        ],
    )
}

fn organization(s: &Sources) -> Axis {
    let day_ago = (s.now - DAY) * 1000;
    let mut old = 0;
    let mut minutes = 0.0;
    for file in &s.waiting {
        let Ok(waiting) = serde_json::from_str::<Value>(file) else {
            continue;
        };
        for item in waiting["items"].as_array().into_iter().flatten() {
            minutes += item["minutes"].as_f64().unwrap_or(0.0);
            let raised = item["raisedAt"]
                .as_str()
                .and_then(|at| at.parse::<Timestamp>().ok());
            if raised.is_some_and(|at| at.as_millisecond() < day_ago) {
                old += 1;
            }
        }
    }
    axis(
        "organization",
        vec![
            Reading::new("{} quests at driving", driving(&s.quests), "", "WCJQSG")
                .bar(Bar::AtMost(5.0))
                .from("the jobs sorter's quests.json"),
            Reading::new(
                "{} asks waiting on Joel over a day",
                Some(f64::from(old)),
                "",
                "WCJQSG",
            )
            .bar(Bar::AtMost(0.0))
            .detail(Some(format!(
                "{} min of his time asked in all",
                shown(minutes, "")
            )))
            .from("mission's waiting files")
            .his(),
            plans::reading(&s.plans),
        ],
    )
}

fn scale(s: &Sources) -> Axis {
    axis(
        "scale",
        vec![
            Reading::new(
                "{} pi windows",
                s.windows.as_deref().and_then(pi_windows),
                "",
                "OSPHT4",
            )
            .bar(Bar::AtMost(12.0))
            .from("niri's windows")
            .his(),
            Reading::new(
                "{} sessions listed live whose pi is gone",
                s.ghosts.map(count),
                "",
                "85PKB2",
            )
            .bar(Bar::AtMost(0.0))
            .from("the mesh's presences"),
        ],
    )
}

fn stress(s: &Sources) -> Axis {
    let (cleared, by_hand) = notices(&s.notices, s.midnight * 1000);
    let left = cleared + by_hand;
    axis(
        "stress",
        vec![
            Reading::new(
                "{} of notices cleared themselves today",
                (left > 0).then(|| percent(cleared, left)),
                "%",
                "2RCBOJ",
            )
            .bar(Bar::AtLeast(70.0))
            .detail(Some(format!("{by_hand} he cleared by hand")))
            .from("the notices' record"),
            counted::pained(&s.counted),
        ],
    )
}

fn axis(name: &'static str, readings: Vec<Reading>) -> Axis {
    Axis {
        name,
        readings,
        findings: Vec::new(),
    }
}

fn percent(part: usize, all: usize) -> f64 {
    100.0 * count(part) / count(all)
}

/// A count as a figure. No count here comes near `u32`'s top, which
/// `f64` holds exactly.
pub(super) fn count(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

fn money(text: &str) -> Option<f64> {
    let digits: String = text
        .trim_start_matches('$')
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == ',' || *c == '.')
        .filter(|c| *c != ',')
        .collect();
    digits.parse().ok()
}

/// What cache rewrites after a compaction cost, from the measure's
/// "- after compaction: 1183 times, $474" line.
fn after_compaction(measure: &str) -> Option<f64> {
    let line = measure
        .lines()
        .find_map(|l| l.strip_prefix("- after compaction: "))?;
    money(line.split(", ").nth(1)?)
}

/// The median the card's line starting `what` says, none when it says
/// none.
fn card_median(card: &str, what: &str) -> Option<f64> {
    let line = card.lines().find(|l| l.starts_with(what))?;
    let rest = line.split("): ").nth(1)?.strip_prefix("median ")?;
    rest.split([',', ' ']).next()?.parse().ok()
}

/// How long each merge not on yet has waited, in minutes, from the card.
fn card_waiting(card: &str) -> Vec<f64> {
    card.lines()
        .find_map(|l| l.trim_start().strip_prefix("not on yet: "))
        .map(|list| {
            list.split(", ")
                .filter_map(|m| m.split_whitespace().nth(1)?.parse().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// How long ago the merge the net tests was made, in minutes: zero when
/// it's caught up with main.
fn card_net(card: &str) -> Option<f64> {
    let line = card.lines().find(|l| l.starts_with("The net "))?;
    if line.starts_with("The net is caught up") {
        return Some(0.0);
    }
    let rest = line.rsplit("merged ").next()?;
    rest.split_whitespace().next()?.parse().ok()
}

/// How many quests are at driving, which the jobs sorter weighs at 400.
fn driving(quests: &str) -> Option<f64> {
    let weights: BTreeMap<String, f64> = serde_json::from_str(quests).ok()?;
    Some(count(weights.values().filter(|w| **w >= 400.0).count()))
}

/// How many of niri's windows are pi's: a session's, a spawned one's or
/// a host's.
fn pi_windows(windows: &str) -> Option<f64> {
    let windows: Vec<Value> = serde_json::from_str(windows).ok()?;
    Some(count(
        windows
            .iter()
            .filter(|w| w["app_id"].as_str().is_some_and(|a| a.starts_with("pi-")))
            .count(),
    ))
}

/// Of the launcher's keys, how many took past 50 ms to their frame, and
/// how many there were, from lines like `launcher: "fi" took 31 ms to its
/// frame`.
fn launcher(journal: &str) -> (usize, usize) {
    let took: Vec<u32> = journal
        .lines()
        .filter_map(|line| {
            let after = line.split("launcher: \"").nth(1)?;
            let ms = after.rsplit_once("\" took ")?.1;
            ms.split_whitespace().next()?.parse().ok()
        })
        .collect();
    (took.iter().filter(|ms| **ms > 50).count(), took.len())
}

/// Of the notices that left since `since` (in milliseconds), how many
/// cleared themselves and how many were dismissed by hand.
fn notices(record: &str, since: i64) -> (usize, usize) {
    let record: Value = serde_json::from_str(record).unwrap_or_default();
    let left = |how: &str| {
        record["notices"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|n| n["at"].as_i64().is_some_and(|at| at >= since))
            .filter(|n| n["left"].as_str() == Some(how))
            .count()
    };
    (left("expired"), left("dismissed"))
}

/// The mesh's messages stamped since `since`, in seconds, from the day
/// folders the harness keeps them in.
fn messages(root: &Path, since: i64) -> Vec<Note> {
    let folder = root.join("mesh-workflow/messages");
    [since, since + DAY]
        .iter()
        .filter_map(|at| Timestamp::from_second(*at).ok())
        .map(|at| folder.join(at.strftime("%Y-%m-%d").to_string()))
        .collect::<BTreeSet<PathBuf>>()
        .iter()
        .flat_map(|day| fs::read_dir(day).into_iter().flatten().flatten())
        .filter_map(|entry| serde_json::from_slice::<Note>(&fs::read(entry.path()).ok()?).ok())
        .filter(|m| mesh::stamped_at(&m.id).is_some_and(|ms| ms >= since * 1000))
        .collect()
}

/// How many presences say busy or listening for a pi that isn't running
/// this boot.
fn ghosts(mesh: &Mesh) -> usize {
    let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .map(|b| b.trim().to_owned())
        .unwrap_or_default();
    mesh.all()
        .iter()
        .filter(|p| p.state == "busy" || p.state == "listening")
        .filter(|p| {
            let this_boot = p
                .owner
                .as_ref()
                .and_then(|o| o.boot_id.as_deref())
                .is_some_and(|b| b == boot);
            !(this_boot && mesh::alive(Some(p), |pid| Path::new(&format!("/proc/{pid}")).exists()))
        })
        .count()
}

/// The newest of cadence's kept days.
fn newest_day(folder: &Path) -> Option<Day> {
    fs::read_dir(folder.join("day"))
        .ok()?
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| serde_json::from_slice::<Day>(&fs::read(e.path()).ok()?).ok())
        .max_by_key(|d| d.to)
}

/// Reads every source from the machine, each cheaply: files, and four
/// commands that answer at once.
#[must_use]
pub fn gather(folder: &Path, state_home: &Path, now: i64) -> Sources {
    let text = |path: PathBuf| fs::read_to_string(path).unwrap_or_default();
    let mut measures: Vec<PathBuf> = fs::read_dir(state_home.join("pi/agentic-harness.pi/measure"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    measures.sort();
    let measures = measures
        .into_iter()
        .rev()
        .take(14)
        .rev()
        .map(text)
        .collect();
    let midnight = Timestamp::from_second(now)
        .ok()
        .and_then(|t| t.to_zoned(TimeZone::system()).start_of_day().ok())
        .map_or(now - DAY, |z| z.timestamp().as_second());
    let failed = ["--system", "--user"]
        .iter()
        .filter_map(|scope| {
            command("systemctl", &[scope, "--failed", "--plain", "--no-legend"]).ok()
        })
        .collect::<String>();
    let mesh = Mesh {
        root: state_home.join("pi/agentic-harness.pi"),
    };
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map_or_else(
        || {
            use std::os::unix::fs::MetadataExt;
            let uid = fs::metadata("/proc/self").map_or(0, |m| m.uid());
            PathBuf::from(format!("/run/user/{uid}"))
        },
        PathBuf::from,
    );
    Sources {
        measures,
        card: text(folder.join("card.txt")),
        day: newest_day(folder),
        failed,
        messages: messages(&mesh.root, now - DAY),
        ghosts: Some(ghosts(&mesh)),
        quests: text(runtime.join("jobs/quests.json")),
        waiting: fs::read_dir(state_home.join("mission/waiting"))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| text(e.path()))
            .collect(),
        windows: command("niri", &["msg", "-j", "windows"]).ok(),
        launcher: command(
            "journalctl",
            &[
                "--user",
                "-u",
                "quickshell",
                &format!("--since=@{midnight}"),
                "-o",
                "cat",
                "--no-pager",
                "--grep=launcher: \".*took",
            ],
        )
        .unwrap_or_default(),
        notices: text(state_home.join("notices/notices.json")),
        pressure: text(PathBuf::from("/proc/pressure/cpu")),
        meminfo: text(PathBuf::from("/proc/meminfo")),
        landings: landings::gather(folder, &runtime, now),
        harness: landings::gather_harness(now),
        plans: plans::gather(),
        counted: counted::gather(folder, state_home, now),
        now,
        midnight,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CARD: &str = "Card at 19:20 (the hour since 18:20)
Core: 18 merged (0 tests or docs only).
Last commit to merge, outside boot (target median 20): median 0.9, p90 1.3, max 1.8 min (n 16).
Merge to on (target median 18): none.
  not on yet: #3438 0 min, #3433 3 min, #3363 177 min
  switches: 0, last at none
VM runs: 71, 61 passed.
VM wait for a builder (target median 2): median 0.0, p90 0.0, max 0.4 min (n 71).
The net tests 6f8501688 (9 merges behind main), merged 28 min ago.
";

    fn measure(spend: &str) -> String {
        format!(
            "# Daily Measure\n\n## Audit\n\nSpend: ${spend} over 115278 turns, up 14% on the day \
             before. Cache reads $3,893.\n\nCache rewrites, by cause:\n- after compaction: 1183 \
             times, $474\n- idle over an hour: 139 times, $192\n"
        )
    }

    #[test]
    fn the_card_says_its_medians_and_what_waits() {
        assert_eq!(card_median(CARD, "Last commit to merge"), Some(0.9));
        assert_eq!(card_median(CARD, "VM wait"), Some(0.0));
        assert_eq!(card_median(CARD, "Merge to on"), None);
        assert_eq!(card_waiting(CARD), vec![0.0, 3.0, 177.0]);
        assert_eq!(card_net(CARD), Some(28.0));
        assert_eq!(card_net("The net is caught up with main.\n"), Some(0.0));
    }

    #[test]
    fn with_none_on_in_the_hour_the_oldest_waiting_is_merge_to_on() {
        let s = Sources {
            card: CARD.into(),
            ..Sources::default()
        };
        let on = &time(&s).readings[0];
        assert_eq!(on.now, Some(177.0));
        assert!(on.out(), "a merge waiting 177 min was in spec");
    }

    #[test]
    fn the_measure_says_rewrites() {
        assert_eq!(after_compaction(&measure("7,293")), Some(474.0));
        assert_eq!(after_compaction("no rewrites here"), None);
    }

    #[test]
    fn with_no_bill_or_changes_spend_is_unread_rather_than_fine() {
        let spend = &resources(&Sources::default()).readings[0];
        assert_eq!(
            spend.words(),
            "the bill per core change that reached you and stayed: no reading yet"
        );
        assert!(!spend.out(), "an unread bill was out");
    }

    #[test]
    fn launcher_keys_past_fifty_ms_are_counted() {
        let journal = r#"qml: launcher: "fi" took 31 ms to its frame, 4 ms searching
qml: launcher: "fir" took 74 ms to its frame, 60 ms searching
qml: launcher: "a \" took 9" took 120 ms to its frame
qml: launcher: drawer for "Firefox" took 80 ms to its frame
"#;
        assert_eq!(launcher(journal), (2, 3));
    }

    #[test]
    fn notices_that_left_today_are_split_by_how() {
        let record = r#"{"next":4,"notices":[
            {"at":5000,"left":"expired"},{"at":6000,"left":"dismissed"},
            {"at":7000,"left":"dismissed"},{"at":100,"left":"expired"},
            {"at":8000,"left":null}]}"#;
        assert_eq!(notices(record, 1000), (1, 2));
    }

    #[test]
    fn driving_quests_and_pi_windows_are_counted() {
        assert_eq!(driving(r#"{"A":400,"B":200,"C":400}"#), Some(2.0));
        let windows = r#"[{"app_id":"pi-session.1"},{"app_id":"slack"},{"app_id":"pi-hosted.x"}]"#;
        assert_eq!(pi_windows(windows), Some(2.0));
    }

    #[test]
    fn a_question_answered_by_anyone_is_not_counted() {
        let old = (10_000 - 3600) * 1000;
        let note = |id: String, intent: &str, re: Option<&str>| Note {
            id,
            intent: intent.into(),
            re: re.map(str::to_owned),
        };
        let asked = format!("{old}-A-1-aa");
        let s = Sources {
            now: 10_000,
            messages: vec![
                note(asked.clone(), "question", None),
                note(format!("{old}-B-1-bb"), "question", None),
                note(format!("{}-C-1-cc", 9_999_000), "question", None),
                note(format!("{old}-D-1-dd"), "fyi", Some(&asked)),
            ],
            ..Sources::default()
        };
        let readings = collaboration(&s).readings;
        assert_eq!(readings[0].now, Some(25.0));
        // B alone: A was answered, and C was asked under 30 min ago.
        assert_eq!(readings[1].now, Some(1.0));
    }

    #[test]
    fn an_old_ask_waiting_on_joel_is_counted() {
        let file = r#"{"v":2,"items":[
            {"id":"a","raisedAt":"2026-11-08T03:41:59.182Z","minutes":5},
            {"id":"b","raisedAt":"2026-11-12T00:00:00Z","minutes":20}]}"#;
        let s = Sources {
            now: 1_794_444_000,
            waiting: vec![file.into()],
            ..Sources::default()
        };
        let asks = &organization(&s).readings[1];
        assert_eq!(asks.now, Some(1.0));
        assert_eq!(
            asks.detail.as_deref(),
            Some("25 min of his time asked in all")
        );
    }
}
