//! `cadence review [--to ADDRESS]...`: the week's noise and blind spots,
//! every Friday evening to the improvement system's owner and the
//! orchestrator. Noise is what nobody acted on or what came again, by
//! sender (noise.rs). Blind spots are where the axes and Joel disagree:
//! pains he had in an area no reading from his side names, axes with no
//! reading from his side at all, and readings in spec in an area his
//! pains named that week. A pain is the pain-points detector's record or
//! a line health heard him sound pained in, each put in the area its
//! words name, by the table below.

mod noise;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde_json::Value;

use super::messages::{self, Note};
use super::readings::{gather as gather_axes, read};
use super::{Axis, Reading};
use crate::mesh::Mesh;
use crate::{DAY, folders, message, now_ms};
use noise::{Tallies, unacted_words};

/// The places Joel meets, each with the words that name it in his lines,
/// a pain's place, or what a reading says. A pain whose words name none
/// of them is put in the area its place names, which no reading names, so
/// it's shown as a pain with no axis until the table learns it.
const AREAS: &[(&str, &[&str])] = &[
    ("the launcher", &["launcher", "drawer", "drawers"]),
    ("pi", &["pi", "session", "sessions", "agent", "agents"]),
    (
        "chrome",
        &["chrome", "browser", "page", "pages", "tab", "tabs"],
    ),
    ("notices", &["notice", "notices"]),
    ("the void", &["void"]),
    ("the terminal", &["terminal", "kitty"]),
    ("windows", &["window", "windows", "niri"]),
    ("typing", &["typing", "keyboard", "keys", "dictation"]),
    (
        "builds and merges",
        &["build", "builds", "merge", "merges", "switch"],
    ),
    // Most of his words that named no area asked why the work was slow
    // or waited on him. It comes last, so a line naming a part he uses
    // goes there first.
    (
        "the work's pace",
        &[
            "slow",
            "slowing",
            "faster",
            "long",
            "wait",
            "waiting",
            "parking",
            "lazy",
            "orchestrator",
            "land",
        ],
    ),
];

/// The most senders the page lists, the noisiest first.
const SENDERS: usize = 12;

/// Everything the review is read from, so it's tested without the
/// machine.
#[derive(Debug, Default)]
pub struct Sources {
    /// `notices/notices.json`.
    pub notices: String,
    /// Each file under the harness's `looks/`.
    pub looks: Vec<String>,
    /// The mesh's `landings.json`.
    pub landings: String,
    pub messages: Vec<Note>,
    /// The pain-points detector's day files of the week, a record a line.
    pub points: Vec<String>,
    /// health's day files of the week.
    pub health: Vec<String>,
    pub axes: Vec<Axis>,
    /// The week's start and end, in seconds.
    pub since: i64,
    pub now: i64,
}

/// The pains of one area over the week.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Pain {
    pub times: u32,
    /// One of them, in his words or the detector's.
    pub example: String,
}

/// Where the axes and Joel disagree.
#[derive(Debug, Default, PartialEq)]
pub struct Blind {
    /// Areas his pains named that no reading from his side names.
    pub no_axis: Vec<(String, Pain)>,
    /// Axes with no reading from his side.
    pub unmeasured: Vec<&'static str>,
    /// A reading from his side in spec, its axis, and the area his pains
    /// named that it names.
    pub in_spec: Vec<String>,
}

/// `cadence review [--to ADDRESS]...`: prints the week's review, and sends
/// it to each address given.
pub fn run(mut args: impl Iterator<Item = String>) -> ExitCode {
    let mut to = Vec::new();
    while let Some(flag) = args.next() {
        let ("--to", Some(address)) = (flag.as_str(), args.next()) else {
            eprintln!("usage: cadence review [--to ADDRESS]...");
            return ExitCode::from(2);
        };
        to.push(address);
    }
    let sent = folders().and_then(|(_, state_home, folder)| {
        let now = now_ms() / 1000;
        let page = page(&gather(&folder, &state_home, now));
        print!("{page}");
        let mesh = Mesh {
            root: state_home.join("pi/agentic-harness.pi"),
        };
        to.iter()
            .try_for_each(|address| mesh.send(&message(address, "fyi", page.clone())))
    });
    match sent {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("cadence: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Reads the week ending at `now` from the machine. The mesh's week is
/// read without the axes' kept day, every file opened once, which takes
/// seconds once a week.
fn gather(folder: &Path, state_home: &Path, now: i64) -> Sources {
    let since = now - 7 * DAY;
    let text = |path: PathBuf| fs::read_to_string(path).unwrap_or_default();
    let pi = state_home.join("pi/agentic-harness.pi");
    let first = Timestamp::from_second(since)
        .map(|t| {
            t.to_zoned(TimeZone::system())
                .strftime("%Y-%m-%d")
                .to_string()
        })
        .unwrap_or_default();
    // Day files are named by their date, so a week's are chosen by name.
    let dated = |folder: PathBuf, extension: &str| -> Vec<String> {
        files(&folder, extension)
            .into_iter()
            .filter(|p| {
                p.file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.len() == first.len() && s >= first.as_str())
            })
            .map(text)
            .collect()
    };
    Sources {
        notices: text(state_home.join("notices/notices.json")),
        looks: files(&pi.join("looks"), "json")
            .into_iter()
            .map(text)
            .collect(),
        landings: text(pi.join("mesh-workflow/landings.json")),
        messages: messages::read(&pi, since, None),
        points: dated(state_home.join("pain-points"), "jsonl"),
        health: dated(state_home.join("health"), "json"),
        axes: read(&gather_axes(folder, state_home, now)),
        since,
        now,
    }
}

fn files(folder: &Path, extension: &str) -> Vec<PathBuf> {
    fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == extension))
        .collect()
}

/// The words of `text` as the table matches them: lower case, split at
/// anything but a letter, a digit or a plus.
fn tokens(text: &str) -> BTreeSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '+')
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Every area `text` names.
fn areas(text: &str) -> Vec<&'static str> {
    let words = tokens(text);
    AREAS
        .iter()
        .filter(|(_, names)| names.iter().any(|n| words.contains(*n)))
        .map(|(area, _)| *area)
        .collect()
}

/// The week's pains by area: each pain-points record since `since` by
/// its place, and each of health's pained lines by the first area it
/// names. A line naming none is counted under "" rather than guessed.
fn pains(points: &[String], health: &[String], since: i64) -> BTreeMap<String, Pain> {
    let mut pains: BTreeMap<String, Pain> = BTreeMap::new();
    let mut add = |area: String, times: u32, example: String| {
        let pain = pains.entry(area).or_default();
        if pain.example.is_empty() {
            pain.example = example.chars().take(100).collect();
        }
        pain.times += times;
    };
    for point in points
        .iter()
        .flat_map(|text| text.lines())
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|p| p["at"].as_i64().is_some_and(|at| at >= since))
    {
        let said = |key: &str| point[key].as_str().unwrap_or_default().to_owned();
        let place = said("place");
        let area = areas(&place)
            .first()
            .map_or_else(|| place.clone(), |a| (*a).to_owned());
        add(
            area,
            1,
            format!("{place}: {}, {}", said("trying"), said("slowed")),
        );
    }
    for day in health
        .iter()
        .filter_map(|text| serde_json::from_str::<Value>(text).ok())
    {
        let Some(things) = day["things"].as_object() else {
            continue;
        };
        let words = things.values().filter(|t| t["kind"] == "words");
        for (line, seen) in words.filter_map(|t| t["signatures"].as_object()).flatten() {
            let times = seen["count"]
                .as_u64()
                .map_or(1, |n| u32::try_from(n).unwrap_or(u32::MAX));
            let area = areas(line)
                .first()
                .map_or_else(String::new, |a| (*a).to_owned());
            add(area, times, line.clone());
        }
    }
    pains
}

/// Where `axes` and the week's `pains` disagree.
fn blind(axes: &[Axis], pains: &BTreeMap<String, Pain>) -> Blind {
    let his = || {
        axes.iter()
            .flat_map(|axis| axis.readings.iter().map(move |r| (axis.name, r)))
            .filter(|(_, r)| r.judges())
    };
    let named: BTreeSet<&str> = his().flat_map(|(_, r)| areas(r.says)).collect();
    let in_spec = his()
        .filter(|(_, r)| r.bar.is_some() && !r.out())
        .flat_map(|(axis, r)| {
            areas(r.says).into_iter().filter_map(move |area| {
                let pain = pains.get(area)?;
                Some(format!(
                    "{axis}: {}; his pains named {area} {} times",
                    r.words(),
                    pain.times
                ))
            })
        })
        .collect();
    let mut no_axis: Vec<(String, Pain)> = pains
        .iter()
        .filter(|(area, _)| !named.contains(area.as_str()))
        .map(|(area, pain)| (area.clone(), pain.clone()))
        .collect();
    no_axis.sort_by_key(|(_, pain)| std::cmp::Reverse(pain.times));
    Blind {
        no_axis,
        unmeasured: axes
            .iter()
            .filter(|axis| !axis.readings.iter().any(Reading::judges))
            .map(|axis| axis.name)
            .collect(),
        in_spec,
    }
}

/// The review as it's printed and sent.
fn page(s: &Sources) -> String {
    let mut tallies = Tallies::new();
    noise::notices(&s.notices, s.since, &mut tallies);
    noise::looks(&s.looks, s.since, &mut tallies);
    noise::landings(&s.landings, s.since, &mut tallies);
    noise::messages(&s.messages, s.now - 3600, &mut tallies);
    let pains = pains(&s.points, &s.health, s.since);
    let blind = blind(&s.axes, &pains);
    let week = |at: i64| {
        Timestamp::from_second(at)
            .map(|t| {
                t.to_zoned(TimeZone::system())
                    .strftime("%a %-d %b")
                    .to_string()
            })
            .unwrap_or_default()
    };
    let mut out = format!(
        "cadence review: the week {} to {}\n",
        week(s.since),
        week(s.now)
    );
    out.push_str("\nNoise by sender, what nobody acted on or came again:\n");
    let mut rows: Vec<_> = tallies
        .iter()
        .filter(|(_, t)| t.unacted + t.again > 0)
        .collect();
    rows.sort_by_key(|(_, t)| std::cmp::Reverse(t.unacted + t.again));
    for ((kind, sender), t) in rows.iter().take(SENDERS) {
        let _ = write!(out, "- {sender}: {} {kind}", t.said);
        if t.unacted > 0 {
            let _ = write!(out, ", {} {}", t.unacted, unacted_words(kind));
        }
        if t.again > 0 {
            let _ = write!(out, ", {} said again", t.again);
        }
        out.push('\n');
    }
    if rows.is_empty() {
        out.push_str("- none\n");
    }
    out.push_str("\nPains with no axis, his areas no reading from his side names:\n");
    for (area, pain) in &blind.no_axis {
        let named = if area.is_empty() {
            "named no area"
        } else {
            area
        };
        let _ = writeln!(out, "- {named}, {} times: {}", pain.times, pain.example);
    }
    if blind.no_axis.is_empty() {
        out.push_str("- none\n");
    }
    out.push_str("\nAxes with no measure from his side: ");
    out.push_str(&if blind.unmeasured.is_empty() {
        "none".to_owned()
    } else {
        blind.unmeasured.join(", ")
    });
    out.push_str("\n\nMeasures in spec where his pains named their area:\n");
    for line in &blind.in_spec {
        let _ = writeln!(out, "- {line}");
    }
    if blind.in_spec.is_empty() {
        out.push_str("- none\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::Bar;

    // 00:00 UTC on Thu 12 November 2026.
    const SINCE: i64 = 1_794_441_600;

    fn axis(name: &'static str, readings: Vec<Reading>) -> Axis {
        Axis {
            name,
            readings,
            findings: Vec::new(),
        }
    }

    #[test]
    fn a_pain_goes_in_the_area_its_words_or_place_name() {
        let points = vec![format!(
            "{}\n{}\n",
            format!(
                r#"{{"at":{},"place":"chrome","trying":"a page","slowed":"reloading"}}"#,
                SINCE + 10
            ),
            format!(r#"{{"at":{},"place":"chrome","trying":"old"}}"#, SINCE - 10),
        )];
        let health = vec![
            r#"{"things":{"folder":{"kind":"words","signatures":{
                "why is the launcher so slow":{"count":2},
                "why are you taking so long, why is it waiting on me?":{"count":3},
                "what is this":{"count":1}}},
              "pi bash":{"kind":"tools","signatures":{"launcher":{"count":9}}}}}"#
                .to_owned(),
        ];
        let pains = pains(&points, &health, SINCE);
        assert_eq!(
            pains.get("chrome").map(|p| p.times),
            Some(1),
            "the week's one chrome pain, its older one left out"
        );
        assert_eq!(
            pains.get("the launcher"),
            Some(&Pain {
                times: 2,
                example: "why is the launcher so slow".to_owned()
            }),
            "his line names the launcher, twice; a tool's errors are not his words"
        );
        assert_eq!(
            pains.get("the work's pace").map(|p| p.times),
            Some(3),
            "a line asking why the work is slow or waits on him is the work's pace"
        );
        assert_eq!(
            pains.get("").map(|p| p.times),
            Some(1),
            "a line naming no area is counted apart"
        );
    }

    #[test]
    fn the_axes_and_his_pains_disagree_where_his_side_reads_nothing_or_reads_fine() {
        let axes = vec![
            axis(
                "responsiveness",
                vec![
                    Reading::new("the launcher's middle key {}", Some(28.0), " ms", "Y2YBTS")
                        .bar(Bar::AtMost(50.0))
                        .his(),
                ],
            ),
            axis(
                "scale",
                vec![Reading::new("{} pi windows", Some(107.0), "", "OSPHT4")],
            ),
        ];
        let pains = BTreeMap::from([
            (
                "the launcher".to_owned(),
                Pain {
                    times: 5,
                    example: "looked for something".to_owned(),
                },
            ),
            (
                "pi".to_owned(),
                Pain {
                    times: 11,
                    example: "pi: a session, waiting".to_owned(),
                },
            ),
        ]);
        let blind = blind(&axes, &pains);
        assert_eq!(
            blind.no_axis,
            vec![("pi".to_owned(), pains["pi"].clone())],
            "pi is named only by a stand-in, so no reading from his side has it"
        );
        assert_eq!(blind.unmeasured, vec!["scale"], "scale reads only our side");
        assert_eq!(
            blind.in_spec,
            vec![
                "responsiveness: the launcher's middle key 28 ms, at most 50 ms; \
                 his pains named the launcher 5 times"
                    .to_owned()
            ],
            "the launcher reads fine from his side while he named it"
        );
    }
}
