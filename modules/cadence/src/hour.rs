//! `cadence hour`: the hourly loop Joel asked for on 14 November, "something
//! constantly evaluating how we can ship more and faster while also
//! delivering insane quality, both functionally, and by maintaining clean
//! zones". Each hour it measures what shipped, what broke, what slipped
//! and how clean the code is, each against a target, from what the
//! machine already keeps; names the one biggest drag and files it with
//! its owner in the loop's ledger, due a day on; says the next hour
//! whether it moved; and sends the card to the orchestrator. A miss Joel
//! found himself outranks every figure, and stays open until a check
//! that would have caught it merges.

mod lines;
mod measures;

use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::io;
use std::path::Path;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};
use terminal_kit::Failure;

use crate::findings::{self, Entry};
use crate::mesh::Mesh;
use crate::owe::{self, Owed};
use crate::{DAY, axes, clock, command, failed, folders, message, now_ms, write_whole, wrongly};
use lines::{figure, shown_lines};
use measures::{Class, Measure, drag};

/// Who moves the measures no file names an owner for: the merge net's
/// rules and its quarantine are cadence's keeper's, and the net that
/// finds a broken check is its own owner's.
const NET_RULES: &str = "80ECO2";
const NET: &str = "D5QEKR";

/// How many hours a drag may stay unmoved before the orchestrator is
/// asked to steer it.
const UNMOVED_FOR: u32 = 2;

/// What the hour remembers until the next: its drag, and the misses that
/// were open, so one that closes is told once.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Kept {
    #[serde(default)]
    pub drag: Option<Held>,
    #[serde(default)]
    pub open: Vec<String>,
}

/// The drag as an hour named it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Held {
    pub key: String,
    pub says: String,
    pub owner: String,
    pub now: f64,
    /// How many hours in a row it hasn't moved.
    pub unmoved: u32,
}

/// `cadence hour [--to ADDRESS] [--checkout NAME=PATH]... [--realm NAME]...`:
/// prints the hour's card, and sends it to the address given, the
/// orchestrator's. Each checkout is another repository whose main says
/// which of the misses' checks merged, beside core's and the harness's;
/// each realm's node says whether a machine is crowded beside one with room.
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), Failure> {
    let mut to = None;
    let mut checkouts = Vec::new();
    let mut realms = Vec::new();
    while let Some(flag) = args.next() {
        match (flag.as_str(), args.next()) {
            ("--to", Some(address)) => to = Some(address),
            ("--checkout", Some(named)) if named.contains('=') => checkouts.push(named),
            ("--realm", Some(realm)) => realms.push(realm),
            _ => {
                return Err(wrongly(
                    "hour",
                    format!("{flag} isn't a flag it takes, or has no value"),
                ));
            }
        }
    }
    hour(to.as_deref(), &checkouts, &realms).map_err(failed)
}

fn hour(to: Option<&str>, checkouts: &[String], realms: &[String]) -> io::Result<()> {
    let (_, state_home, folder) = folders()?;
    let now = now_ms() / 1000;
    let mut measures = gather(&folder, &state_home, now, checkouts)?;
    // A realm whose node doesn't answer says nothing of its machines.
    if !realms.is_empty() {
        let answers: Vec<String> = realms
            .iter()
            .filter_map(|realm| command("mycelium", &["--realm", realm, "machines"]).ok())
            .collect();
        measures.push(measures::crowded(&answers, NET_RULES));
    }
    let kept_at = folder.join("hour.json");
    let kept: Kept = fs::read_to_string(&kept_at)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    let (card, next) = compose(
        &clock(now),
        &measures,
        &kept,
        &format!("{} tomorrow", clock(now + DAY)),
    );
    print!("{card}");
    let ledger_at = folder.join("findings.jsonl");
    let mut ledger = findings::read(&ledger_at);
    let told = file(&mut ledger, &measures, &today(now), now);
    findings::keep(&ledger_at, &ledger)?;
    write_whole(&kept_at, &serde_json::to_string(&next)?)?;
    let mesh = Mesh {
        root: state_home.join("pi/agentic-harness.pi"),
    };
    for (owner, text) in told.into_iter().filter(|(owner, _)| !owner.is_empty()) {
        mesh.send(&message(&owner, "finding", text))?;
    }
    if let Some(to) = to {
        mesh.send(&message(to, "fyi", card))?;
    }
    Ok(())
}

/// Reads every measure from the machine: the orchestrator's misses and
/// asks, the net's quarantine, reverts and results, the tick's card, the
/// three mains, and code health's hourly read.
fn gather(
    folder: &Path,
    state_home: &Path,
    now: i64,
    checkouts: &[String],
) -> io::Result<Vec<Measure>> {
    let text = |path: &Path| fs::read_to_string(path).unwrap_or_default();
    let mission = state_home.join("mission");
    let core = folder.join("core.git");
    let quarantine = command(
        "git",
        &[
            "-C",
            &core.to_string_lossy(),
            "show",
            "main:tools/merge-net/quarantine.json",
        ],
    )
    .unwrap_or_default();
    let card_text = text(&folder.join("card.txt"));
    let [core_log, harness_log, machine_log] = axes::repository::mains(folder, now);
    let mut all = vec![
        measures::checked(&text(&mission.join("misses.jsonl")), folder, checkouts)?,
        measures::slips(&text(&mission.join("asks.jsonl")), now),
        measures::to_first_slice(&text(&mission.join("asks.jsonl")), now),
        measures::undated(&quarantine, &today(now), NET_RULES),
        measures::reverts(&text(&folder.join("reverted.json")), now, NET_RULES),
        measures::broken(&text(&state_home.join("net/results.jsonl")), now, NET),
        landed(&core_log, &harness_log, &machine_log, now),
    ];
    all.extend(measures::merge_to_on(&card_text, NET_RULES));
    if let Some((median, target)) = measures::card_median(&card_text, "Last commit to merge") {
        all.push(Measure {
            key: "commit-to-merge".to_owned(),
            class: Class::Ship,
            says: "median minutes from last commit to merge".to_owned(),
            now: median,
            target: Some(target),
            owner: NET_RULES.to_owned(),
            behind: Vec::new(),
        });
    }
    all.extend(measures::clean(&text(
        &state_home.join("code-health/hour.json"),
    )));
    Ok(all)
}

/// The date at `now` here, as `2026-11-14`.
fn today(now: i64) -> String {
    Timestamp::from_second(now)
        .map(|at| at.to_zoned(TimeZone::system()).date().to_string())
        .unwrap_or_default()
}

/// The changes the three mains landed in the hour, shown unjudged: an
/// hour's count swings with the clock, so no one target holds for it.
fn landed(core: &str, harness: &str, machine: &str, now: i64) -> Measure {
    let [c, h, m] = [core, harness, machine].map(|log| measures::landed(log, now));
    Measure {
        key: "landed".to_owned(),
        class: Class::Ship,
        says: format!(
            "changes landed in the hour (core {c}, the harness {h}, the machine's repository {m})"
        ),
        now: f64::from(u32::try_from(c + h + m).unwrap_or(u32::MAX)),
        target: None,
        owner: String::new(),
        behind: Vec::new(),
    }
}

/// The hour's card, at `at` and with the drag due at `due`, and what the
/// next hour is to remember.
#[must_use]
pub fn compose(at: &str, measures: &[Measure], kept: &Kept, due: &str) -> (String, Kept) {
    let mut card = format!("cadence hour at {at}\n");
    for class in [
        Class::Ship,
        Class::Miss,
        Class::Slip,
        Class::Functional,
        Class::Clean,
    ] {
        let of_class: Vec<&Measure> = measures.iter().filter(|m| m.class == class).collect();
        let _ = writeln!(card, "{}:", capitalised(class.name()));
        if of_class.is_empty() {
            let _ = writeln!(card, "  not read yet");
        }
        for l in shown_lines(class, &of_class) {
            let _ = writeln!(card, "  {l}");
        }
    }
    let (followed, unmoved) = follow(kept.drag.as_ref(), measures);
    let chosen = drag(measures);
    match chosen {
        Some(m) => {
            let _ = writeln!(
                card,
                "Drag: {}, {} against at most {}, to {} by {due}.",
                m.says,
                figure(m.now),
                figure(m.target.unwrap_or_default()),
                if m.class == Class::Miss {
                    "each miss's owner"
                } else if !by_owner(m).is_empty() {
                    "each file's owner"
                } else {
                    owner_or_none(&m.owner)
                }
            );
        }
        None => {
            let _ = writeln!(card, "Drag: none, every judged measure is in.");
        }
    }
    if let Some(followed) = followed {
        let _ = writeln!(card, "{followed}");
    }
    let open: Vec<String> = measures
        .iter()
        .filter(|m| m.class == Class::Miss)
        .flat_map(|m| m.behind.clone())
        .collect();
    if !open.is_empty() {
        let _ = writeln!(
            card,
            "Why didn't our checks catch these? Each stays open until its owner names the \
             check that would have, and it merges:"
        );
        for miss in &open {
            let _ = writeln!(card, "  - {miss}");
        }
    }
    for closed in kept.open.iter().filter(|miss| !open.contains(miss)) {
        // Only the owner follows the last mark; a miss's words may hold one.
        let what = closed
            .rsplit_once(" · ")
            .map_or(closed.as_str(), |(what, _)| what);
        let _ = writeln!(card, "For Joel: fixed, and a check now catches it: {what}.");
    }
    let next = Kept {
        drag: chosen.map(|m| Held {
            key: m.key.clone(),
            says: m.says.clone(),
            // Each miss has its own owner, the oldest's no more than the rest.
            owner: if m.class == Class::Miss {
                "each miss's owner".to_owned()
            } else {
                m.owner.clone()
            },
            now: m.now,
            unmoved: kept
                .drag
                .as_ref()
                .filter(|held| held.key == m.key)
                .map_or(0, |_| unmoved),
        }),
        open,
    };
    (card, next)
}

/// What became of last hour's drag, and how many hours it has now gone
/// unmoved: back in, moved down, or not moved, which past two hours asks
/// the orchestrator to steer it.
fn follow(last: Option<&Held>, measures: &[Measure]) -> (Option<String>, u32) {
    let Some(last) = last else {
        return (None, 0);
    };
    let now = measures.iter().find(|m| m.key == last.key);
    match now {
        Some(m) if m.past().is_none() => (
            Some(format!("Last hour's drag, {}, is back in.", last.says)),
            0,
        ),
        None => (
            Some(format!(
                "Last hour's drag, {}, wasn't read this hour.",
                last.says
            )),
            last.unmoved,
        ),
        Some(m) if m.now < last.now => (
            Some(format!(
                "Last hour's drag, {}, moved from {} to {}.",
                last.says,
                figure(last.now),
                figure(m.now)
            )),
            0,
        ),
        Some(m) => {
            let unmoved = last.unmoved + 1;
            let steer = if unmoved >= UNMOVED_FOR {
                " Orchestrator: please steer it."
            } else {
                ""
            };
            // A drag that grew hasn't moved down either, so it counts on; at
            // 01:10 on 15 November one that went from 7 to 8 read "still 8".
            let how = if m.now > last.now {
                format!("grew from {} to {}", figure(last.now), figure(m.now))
            } else {
                format!(
                    "hasn't moved in {unmoved} {}: still {}",
                    if unmoved == 1 { "hour" } else { "hours" },
                    figure(m.now)
                )
            };
            (
                Some(format!(
                    "Last hour's drag, {}, {how}, with {}.{steer}",
                    last.says,
                    owner_or_none(&last.owner)
                )),
                unmoved,
            )
        }
    }
}

fn owner_or_none(owner: &str) -> &str {
    if owner.is_empty() { "no owner" } else { owner }
}

fn capitalised(words: &str) -> String {
    let mut chars = words.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// One drag as the ledger files it: its key, owner, words, and what its
/// owner is told.
struct Filed {
    key: String,
    owner: String,
    says: String,
    text: String,
}

/// What the drag files: one entry for its measure, or, for Joel's misses,
/// one for each miss with that miss's own owner, since each owner names
/// the check its own miss needs.
fn filed(m: &Measure) -> Vec<Filed> {
    if m.class == Class::Miss {
        return m
            .behind
            .iter()
            .filter_map(|line| line.rsplit_once(" · "))
            .map(|(what, owner)| Filed {
                key: format!("hour:miss:{what}"),
                owner: owner.to_owned(),
                says: format!("a miss Joel found himself: {what}"),
                text: format!(
                    "cadence hour: a miss Joel found himself is yours: {what}. Which check \
                     would have caught it? It stays open until that check merges; tell the \
                     orchestrator its PR, for the misses ledger's check field."
                ),
            })
            .collect();
    }
    let says = format!(
        "{}: {} against at most {}",
        m.says,
        figure(m.now),
        figure(m.target.unwrap_or_default())
    );
    let owners = by_owner(m);
    if !owners.is_empty() {
        return owners
            .into_iter()
            .map(|(owner, files)| {
                let files = files.join(", ");
                Filed {
                    key: format!("hour:{}:{owner}", m.key),
                    says: format!("{says}, yours: {files}"),
                    text: format!(
                        "cadence hour: the biggest drag this hour is {says}, and these of \
                         its files are yours: {files}. Please reply with their cause and fix \
                         within a day, or why they can't move. It closes by itself once its \
                         figure is back in."
                    ),
                    owner,
                }
            })
            .collect();
    }
    vec![Filed {
        key: format!("hour:{}", m.key),
        owner: m.owner.clone(),
        text: format!(
            "cadence hour: the biggest drag this hour is yours, {says}. Please reply with \
             its cause and fix within a day, or why it can't move. It closes by itself once \
             its figure is back in."
        ),
        says,
    }]
}

/// A clean measure's files by the owner each is said with, where
/// code-health said one; empty otherwise, and for every other class.
fn by_owner(m: &Measure) -> BTreeMap<String, Vec<String>> {
    let mut owners: BTreeMap<String, Vec<String>> = BTreeMap::new();
    if m.class == Class::Clean {
        for (file, owner) in m.behind.iter().filter_map(|line| line.rsplit_once(" · ")) {
            owners
                .entry(owner.to_owned())
                .or_default()
                .push(file.to_owned());
        }
    }
    owners
}

/// Whether the drag filed as `key` is back in: its miss no longer open,
/// or its measure read and within its target; a clean drag filed for each
/// file's owner carries that owner after its measure's key.
fn back_in(key: &str, measures: &[Measure]) -> bool {
    let key = key.strip_prefix("hour:").unwrap_or(key);
    if let Some(what) = key.strip_prefix("miss:") {
        let open = format!("{what} · ");
        return !measures
            .iter()
            .filter(|m| m.class == Class::Miss)
            .flat_map(|m| &m.behind)
            .any(|line| line.starts_with(&open));
    }
    measures
        .iter()
        .find(|m| {
            m.key == key
                || (m.class == Class::Clean
                    && key
                        .strip_prefix(m.key.as_str())
                        .is_some_and(|owner| owner.starts_with(':')))
        })
        .is_some_and(|m| m.past().is_none())
}

/// Files the hour's drag in the loop's ledger as a must of kind `drag`,
/// due a day on, unless it's open there already, and closes each drag
/// filed before that is back in. Says what to tell the owner of each one
/// newly filed.
pub fn file(
    ledger: &mut Vec<Entry>,
    measures: &[Measure],
    date: &str,
    now: i64,
) -> Vec<(String, String)> {
    let closing: Vec<String> = ledger
        .iter()
        .filter(|e| e.open && e.kind.as_deref() == Some("drag") && back_in(&e.key, measures))
        .map(|e| e.key.clone())
        .collect();
    for key in closing {
        owe::settle(ledger, &key);
    }
    let Some(m) = drag(measures) else {
        return Vec::new();
    };
    let mut told = Vec::new();
    for one in filed(m) {
        if ledger.iter().any(|e| e.open && e.key == one.key) {
            continue;
        }
        owe::owe(
            ledger,
            Owed {
                key: one.key,
                axis: m.class.name().to_owned(),
                owner: one.owner.clone(),
                due: now + DAY,
                says: one.says,
                kind: "drag".to_owned(),
            },
            date,
            now,
        );
        told.push((one.owner, one.text));
    }
    told
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measure(key: &str, class: Class, now: f64, target: Option<f64>) -> Measure {
        Measure {
            key: key.to_owned(),
            class,
            says: format!("{key} said"),
            now,
            target,
            owner: "OWNER".to_owned(),
            behind: Vec::new(),
        }
    }

    fn held(key: &str, now: f64, unmoved: u32) -> Kept {
        Kept {
            drag: Some(Held {
                key: key.to_owned(),
                says: format!("{key} said"),
                owner: "OWNER".to_owned(),
                now,
                unmoved,
            }),
            open: Vec::new(),
        }
    }

    #[test]
    fn clean_zones_show_only_what_is_past_and_count_the_rest() {
        let measures = [
            measure("long", Class::Clean, 12.0, Some(10.0)),
            measure("crowded", Class::Clean, 2.0, Some(2.0)),
            measure("notes", Class::Clean, 87.0, None),
            measure("lints", Class::Clean, 9.0, None),
        ];
        let (card, _) = compose("18:10", &measures, &Kept::default(), "18:10 tomorrow");
        assert!(
            card.contains("Clean zones:\n  ✕ long said: 12, at most 10\n  1 more in, 2 unjudged\n"),
            "a clean measure in or unjudged was listed, or the rest weren't counted: {card}"
        );
        let (all_in, _) = compose("18:10", &measures[1..2], &Kept::default(), "18:10 tomorrow");
        assert!(
            all_in.contains("Clean zones:\n  all 1 in\n"),
            "a clean zone with nothing past didn't say so: {all_in}"
        );
    }

    #[test]
    fn the_card_names_the_drag_and_its_owner_and_marks_each_measure() {
        let measures = [
            measure("merge-to-on", Class::Ship, 37.9, Some(18.0)),
            measure("landed", Class::Ship, 12.0, None),
        ];
        let (card, next) = compose("18:10", &measures, &Kept::default(), "18:10 tomorrow");
        assert!(
            card.contains("✕ merge-to-on said: 37.9, at most 18"),
            "{card}"
        );
        assert!(card.contains("· landed said: 12, unjudged"), "{card}");
        assert!(
            card.contains("Drag: merge-to-on said, 37.9 against at most 18, to OWNER"),
            "{card}"
        );
        assert!(
            card.contains("Clean zones:\n  not read yet"),
            "unread is said: {card}"
        );
        assert_eq!(
            next.drag.map(|d| d.unmoved),
            Some(0),
            "a new drag starts unmoved"
        );
    }

    #[test]
    fn a_drag_unmoved_two_hours_asks_the_orchestrator_to_steer_it() {
        let measures = [measure("slips", Class::Slip, 3.0, Some(0.0))];
        let (card, next) = compose("19:10", &measures, &held("slips", 3.0, 1), "");
        assert!(card.contains("hasn't moved in 2 hours"), "{card}");
        assert!(card.contains("Orchestrator: please steer it."), "{card}");
        assert_eq!(next.drag.map(|d| d.unmoved), Some(2), "it counts on");
    }

    #[test]
    fn a_drag_that_grew_says_so_and_counts_as_unmoved() {
        let grew = [measure("slips", Class::Slip, 5.0, Some(0.0))];
        let (card, next) = compose("01:10", &grew, &held("slips", 3.0, 0), "");
        assert!(
            card.contains("Last hour's drag, slips said, grew from 3 to 5, with OWNER."),
            "a drag that got worse was said to hold still: {card}"
        );
        assert_eq!(
            next.drag.map(|d| d.unmoved),
            Some(1),
            "growing is no move down"
        );
    }

    #[test]
    fn a_drag_that_moved_or_came_back_in_starts_over() {
        let moved = [measure("slips", Class::Slip, 2.0, Some(0.0))];
        let (card, next) = compose("19:10", &moved, &held("slips", 3.0, 1), "");
        assert!(card.contains("moved from 3 to 2"), "{card}");
        assert_eq!(
            next.drag.map(|d| d.unmoved),
            Some(0),
            "moving starts it over"
        );
        let back = [measure("slips", Class::Slip, 0.0, Some(0.0))];
        let (card, next) = compose("20:10", &back, &held("slips", 2.0, 0), "");
        assert!(card.contains("is back in"), "{card}");
        assert_eq!(next.drag, None, "nothing past its target is no drag");
    }

    #[test]
    fn a_miss_of_joels_that_closes_is_said_for_him_once() {
        let mut open = measure("misses", Class::Miss, 1.0, Some(0.0));
        open.behind = vec!["the map had no network · KNURBN".to_owned()];
        let kept = Kept {
            drag: None,
            open: vec![
                "the map had no network · KNURBN".to_owned(),
                "History · Reader paused · RCTRF8".to_owned(),
            ],
        };
        let (card, next) = compose("19:10", &[open], &kept, "");
        assert!(
            card.contains("For Joel: fixed, and a check now catches it: History · Reader paused."),
            "a miss's own words are said whole, only its owner cut: {card}"
        );
        assert!(
            !card.contains("For Joel: fixed, and a check now catches it: the map"),
            "{card}"
        );
        assert!(
            card.contains("Why didn't our checks catch these?"),
            "{card}"
        );
        assert_eq!(
            next.open,
            ["the map had no network · KNURBN"],
            "only the open one is kept"
        );
    }

    #[test]
    fn a_drag_is_filed_once_with_its_owner_and_closes_once_back_in() {
        let mut ledger = Vec::new();
        let out = [measure("slips", Class::Slip, 3.0, Some(0.0))];
        let told = file(&mut ledger, &out, "2026-11-14", 1000);
        let owners: Vec<String> = told.into_iter().map(|(to, _)| to).collect();
        assert_eq!(owners, ["OWNER"], "its owner is told");
        assert_eq!(ledger.len(), 1, "filed");
        assert_eq!(ledger[0].due, 1000 + DAY, "due a day on");
        assert!(
            file(&mut ledger, &out, "2026-11-14", 4600).is_empty(),
            "told once"
        );
        assert_eq!(ledger[0].due, 1000 + DAY, "its date doesn't move");
        let back = [measure("slips", Class::Slip, 0.0, Some(0.0))];
        assert!(
            file(&mut ledger, &back, "2026-11-14", 8200).is_empty(),
            "none told"
        );
        assert!(!ledger[0].open, "back in closes it");
    }

    #[test]
    fn a_clean_drag_goes_to_each_files_owner_and_closes_once_back_in() {
        let mut suppressions = measure(
            "clean:core.nix:suppressions",
            Class::Clean,
            12.0,
            Some(10.0),
        );
        suppressions.behind = vec![
            "net/a.rs · PN2VZC".to_owned(),
            "net/b.rs · PN2VZC".to_owned(),
            "x.rs · Y9XISP".to_owned(),
        ];
        let (card, _) = compose("03:10", &[suppressions.clone()], &Kept::default(), "");
        assert!(card.contains("to each file's owner by"), "{card}");
        let mut ledger = Vec::new();
        let told = file(&mut ledger, &[suppressions.clone()], "2026-11-15", 1000);
        let owners: Vec<&str> = told.iter().map(|(to, _)| to.as_str()).collect();
        assert_eq!(owners, ["PN2VZC", "Y9XISP"], "each owner of a file is told");
        assert!(
            told[0].1.contains("net/a.rs, net/b.rs") && !told[0].1.contains("x.rs"),
            "each hears only of its own files: {}",
            told[0].1
        );
        suppressions.now = 10.0;
        file(&mut ledger, &[suppressions], "2026-11-15", 4600);
        assert!(
            ledger.iter().all(|e| !e.open),
            "back in closes every owner's"
        );
    }

    #[test]
    fn each_miss_of_joels_goes_to_its_own_owner_and_closes_once_its_check_is_named() {
        let mut misses = measure("misses", Class::Miss, 2.0, Some(0.0));
        misses.behind = vec![
            "the map had no network · KNURBN".to_owned(),
            "History paused · RCTRF8".to_owned(),
        ];
        let mut ledger = Vec::new();
        let told = file(&mut ledger, &[misses.clone()], "2026-11-14", 1000);
        let owners: Vec<&str> = told.iter().map(|(to, _)| to.as_str()).collect();
        assert_eq!(owners, ["KNURBN", "RCTRF8"], "each its own owner");
        assert!(
            told[0].1.contains("Which check would have caught it?"),
            "{}",
            told[0].1
        );
        misses.behind.remove(1);
        misses.now = 1.0;
        assert!(
            file(&mut ledger, &[misses], "2026-11-14", 4600).is_empty(),
            "told once"
        );
        let open: Vec<&str> = ledger
            .iter()
            .filter(|e| e.open)
            .map(|e| e.key.as_str())
            .collect();
        assert_eq!(
            open,
            ["hour:miss:the map had no network"],
            "History's closed"
        );
    }
}
