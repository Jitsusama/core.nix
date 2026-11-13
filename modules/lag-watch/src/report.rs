//! The history, summed up: for each system generation and each load band,
//! how often Joel waited and how long the desktop took, so a change is
//! judged by what it did under the same load as before it. `summary` gives
//! the facts, which `text` lays out for Joel and `json` gives a script.

use crate::window::{Band, Window, count};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use terminal_kit::layout::{self, Align};
use terminal_kit::{Pen, Room};

#[derive(Default)]
struct Tally {
    windows: usize,
    troubled: usize,
    niri_p99: Vec<f64>,
    niri_wait: Vec<f64>,
    session_cpu: Vec<f64>,
    app_memory: Vec<f64>,
    kinds: BTreeMap<String, usize>,
    /// For each program carrying a call, its worst thread's wait a run in
    /// each window, and that thread's name.
    calls: BTreeMap<String, Vec<(f64, String)>>,
    /// For each kind, how often each unit was named as the one that
    /// stalled most.
    units: BTreeMap<String, BTreeMap<String, usize>>,
}

/// One system generation's watch, by load.
#[derive(Debug, Serialize)]
pub struct Generation {
    /// Its path in the store.
    pub system: String,
    /// Its name, the path's last part.
    pub name: String,
    /// Each load it ran under, quiet first.
    pub loads: Vec<Load>,
}

/// How the desktop did under one load, on one generation.
#[derive(Debug, Serialize)]
pub struct Load {
    pub band: Band,
    pub windows: usize,
    pub minutes: f64,
    /// The share of its windows in which Joel waited.
    pub waited_percent: f64,
    /// niri's p99 answer, the median of its windows', and the worst.
    pub niri_p99_ms: f64,
    pub niri_p99_worst_ms: f64,
    /// How long niri's main thread waited for a CPU a turn, the median;
    /// this and the stalls are null where no window watched them.
    pub niri_wait_us: Option<f64>,
    pub desktop_cpu_stall_percent: Option<f64>,
    pub apps_memory_stall_percent: Option<f64>,
    /// Each program that carried a call, apart.
    pub calls: Vec<Call>,
    /// What Joel waited on, most often first, the four most often at most.
    pub waits: Vec<Waited>,
}

/// A program's call threads, over the windows it carried a call in.
#[derive(Debug, Serialize)]
pub struct Call {
    pub program: String,
    pub windows: usize,
    /// Its worst thread's wait a run, the median over its windows.
    pub median_wait_us: f64,
    pub worst_wait_us: f64,
    pub worst_thread: String,
}

/// One kind of wait, and how often.
#[derive(Debug, Serialize)]
pub struct Waited {
    /// What it was, in words, without its numbers.
    pub what: String,
    pub windows: usize,
    /// The unit named most often as the one that stalled most, and how
    /// often; null for a kind no unit is named in.
    pub most_often_in: Option<String>,
    pub most_often_windows: Option<usize>,
}

fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

/// The median of what was watched, or none where nothing was.
fn watched(v: &mut [f64]) -> Option<f64> {
    (!v.is_empty()).then(|| median(v))
}

fn worst(v: &[f64]) -> f64 {
    v.iter().copied().fold(0.0, f64::max)
}

/// What a trouble line is about, without its number: the words before the
/// first digit.
fn kind(line: &str) -> String {
    let cut = line
        .find(|c: char| c.is_ascii_digit())
        .unwrap_or(line.len());
    line[..cut].trim().trim_end_matches(',').to_string()
}

/// The unit a trouble line names as the one that stalled most.
fn unit(line: &str) -> Option<&str> {
    let (_, after) = line.split_once(", most in ")?;
    after.split_once(" (").map(|(name, _)| name)
}

/// A figure to three significant digits, as the desktop sets one, and its
/// unit a cell after it.
pub fn figure(value: f64, unit: &str) -> String {
    let digits = match value.abs() {
        v if v >= 100.0 || v == 0.0 => 0,
        v if v >= 10.0 => 1,
        _ => 2,
    };
    let space = if unit == "%" { "" } else { " " };
    format!("{value:.digits$}{space}{unit}")
}

/// The history summed up, the generations in the order they were first
/// seen.
pub fn summary(windows: &[Window]) -> Vec<Generation> {
    let mut order: Vec<String> = Vec::new();
    let mut tallies: BTreeMap<(String, Band), Tally> = BTreeMap::new();
    for w in windows {
        if !order.contains(&w.system) {
            order.push(w.system.clone());
        }
        let t = tallies.entry((w.system.clone(), w.band)).or_default();
        t.windows += 1;
        if !w.trouble.is_empty() {
            t.troubled += 1;
        }
        t.niri_p99.push(w.niri.p99_ms);
        if let Some(n) = w.waits.get("niri") {
            t.niri_wait.push(n.mean_us);
        }
        if let Some(s) = w.stalls.get("session") {
            t.session_cpu.push(s.cpu_some);
        }
        if let Some(a) = w.stalls.get("app") {
            t.app_memory.push(a.memory_full);
        }
        for (program, worst) in &w.call {
            t.calls
                .entry(program.clone())
                .or_default()
                .push((worst.wait_us, worst.thread.clone()));
        }
        for line in &w.trouble {
            *t.kinds.entry(kind(line)).or_default() += 1;
            if let Some(u) = unit(line) {
                *t.units
                    .entry(kind(line))
                    .or_default()
                    .entry(u.to_string())
                    .or_default() += 1;
            }
        }
    }
    order
        .into_iter()
        .map(|system| {
            let loads = [Band::Quiet, Band::Busy, Band::Loaded]
                .into_iter()
                .filter_map(|band| {
                    tallies
                        .remove(&(system.clone(), band))
                        .map(|tally| load(band, tally))
                })
                .collect();
            Generation {
                name: system.rsplit('/').next().unwrap_or(&system).to_string(),
                system,
                loads,
            }
        })
        .collect()
}

fn load(band: Band, mut t: Tally) -> Load {
    let calls = t
        .calls
        .iter()
        .map(|(program, waits)| {
            let mut us: Vec<f64> = waits.iter().map(|(us, _)| *us).collect();
            let (worst_wait_us, worst_thread) = waits
                .iter()
                .max_by(|a, b| a.0.total_cmp(&b.0))
                .map_or((0.0, String::new()), |(us, name)| (*us, name.clone()));
            Call {
                program: program.clone(),
                windows: waits.len(),
                median_wait_us: median(&mut us),
                worst_wait_us,
                worst_thread,
            }
        })
        .collect();
    let mut kinds: Vec<(&String, &usize)> = t.kinds.iter().collect();
    kinds.sort_by(|a, b| b.1.cmp(a.1));
    let waits = kinds
        .into_iter()
        .take(4)
        .map(|(what, windows)| {
            let most = t
                .units
                .get(what)
                .and_then(|u| u.iter().max_by_key(|(_, n)| **n));
            Waited {
                what: what.clone(),
                windows: *windows,
                most_often_in: most.map(|(unit, _)| unit.clone()),
                most_often_windows: most.map(|(_, n)| *n),
            }
        })
        .collect();
    Load {
        band,
        windows: t.windows,
        minutes: count(t.windows) * 10.0 / 60.0,
        waited_percent: count(t.troubled) / count(t.windows) * 100.0,
        niri_p99_ms: median(&mut t.niri_p99),
        niri_p99_worst_ms: worst(&t.niri_p99),
        niri_wait_us: watched(&mut t.niri_wait),
        desktop_cpu_stall_percent: watched(&mut t.session_cpu),
        apps_memory_stall_percent: watched(&mut t.app_memory),
        calls,
        waits,
    }
}

/// A figure, or that nothing watched it.
fn unwatched(value: Option<f64>, unit: &str) -> String {
    value.map_or_else(|| "not watched".to_string(), |value| figure(value, unit))
}

/// The band's word, as the glossary's states are written.
fn word(band: Band) -> String {
    format!("{band:?}").to_lowercase()
}

/// The report as Joel reads it: each generation a title over a table of
/// its loads, then what was waited on, each line naming its load so it
/// stands alone through `grep`.
pub fn text(generations: &[Generation], pen: &Pen, room: Room) -> String {
    if generations.is_empty() {
        return layout::joined(vec![vec![pen.voice("nothing watched yet")]]);
    }
    let columns = [
        ("load", Align::Left),
        ("minutes", Align::Right),
        ("waited", Align::Right),
        ("niri p99", Align::Right),
        ("worst", Align::Right),
        ("niri waits", Align::Right),
        ("desktop cpu", Align::Right),
        ("apps' memory", Align::Right),
    ];
    // Narrowed, the figures a person reads last go first: niri's waits,
    // its worst answer, then the stalls, then the minutes watched. The
    // load, how often Joel waited and niri's p99 say the most.
    let gives_way = [5, 4, 7, 6, 1];
    let mut blocks = Vec::new();
    for generation in generations {
        let rows: Vec<Vec<String>> = generation
            .loads
            .iter()
            .map(|load| {
                vec![
                    word(load.band),
                    format!("{:.0}", load.minutes),
                    figure(load.waited_percent, "%"),
                    figure(load.niri_p99_ms, "ms"),
                    figure(load.niri_p99_worst_ms, "ms"),
                    unwatched(load.niri_wait_us, "µs"),
                    unwatched(load.desktop_cpu_stall_percent, "%"),
                    unwatched(load.apps_memory_stall_percent, "%"),
                ]
            })
            .collect();
        // Its version is at its end and its hash at its start, so it's
        // shortened in its middle.
        let name = layout::shortened(&generation.name, room.cells().unwrap_or(usize::MAX));
        let mut block = vec![pen.title(&name)];
        block.extend(layout::table_in(&columns, &rows, pen, room, &gives_way));
        blocks.push(block);
        let notes: Vec<String> = generation
            .loads
            .iter()
            .flat_map(|load| {
                let band = word(load.band);
                let calls = load.calls.iter().map(move |call| {
                    format!(
                        "in a call, {}'s worst thread waited {} a run at the median, {} at \
                         worst ({}), in {} windows",
                        call.program,
                        figure(call.median_wait_us, "µs"),
                        figure(call.worst_wait_us, "µs"),
                        call.worst_thread,
                        call.windows
                    )
                });
                let waits = load.waits.iter().map(|waited| {
                    let most = waited
                        .most_often_in
                        .as_ref()
                        .zip(waited.most_often_windows)
                        .map_or_else(String::new, |(unit, n)| {
                            format!(", most often in {unit} ({n})")
                        });
                    format!("{} × {}{most}", waited.windows, waited.what)
                });
                calls
                    .chain(waits)
                    .map(move |line| {
                        // Hung after the load, so a long unit's name wraps
                        // under the words rather than past the terminal.
                        let hung = layout::hung(&line, layout::width(&band) + 1, room.for_prose());
                        format!("{} {}", pen.role("text", &band), pen.role("muted", &hung))
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        blocks.push(notes);
    }
    layout::joined(blocks)
}

/// The report for a script, over the hours it read.
pub fn json(generations: &[Generation], hours: u64) -> Value {
    json!({
        "schema": "lag-watch.report/1",
        "hours": hours,
        "generations": generations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::{Answers, Stalls, Worst};

    fn window(system: &str, band: Band, p99: f64, trouble: &[&str]) -> Window {
        let mut stalls = BTreeMap::new();
        stalls.insert("session".to_string(), Stalls::default());
        Window {
            ended: 0,
            seconds: 10.0,
            system: system.into(),
            band,
            stalls,
            memory: BTreeMap::new(),
            waits: BTreeMap::new(),
            call: BTreeMap::new(),
            niri: Answers {
                p99_ms: p99,
                ..Answers::default()
            },
            trouble: trouble.iter().map(ToString::to_string).collect(),
            worst: String::new(),
            who: String::new(),
            since: None,
        }
    }

    #[test]
    fn each_generation_is_summed_by_band_in_the_order_it_was_seen() {
        let all = [
            window(
                "/nix/store/aaa-nixos-system-example-26.11",
                Band::Loaded,
                12.0,
                &["niri took 60 ms to answer"],
            ),
            window(
                "/nix/store/aaa-nixos-system-example-26.11",
                Band::Loaded,
                2.0,
                &[],
            ),
            window(
                "/nix/store/bbb-nixos-system-example-26.11",
                Band::Quiet,
                1.0,
                &[],
            ),
        ];
        let summed = summary(&all);
        let names: Vec<&str> = summed.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "aaa-nixos-system-example-26.11",
                "bbb-nixos-system-example-26.11"
            ]
        );
        let loaded = &summed[0].loads;
        assert_eq!(loaded.len(), 1, "a band with no windows isn't there");
        assert_eq!(loaded[0].band, Band::Loaded);
        assert_eq!(loaded[0].waited_percent, 50.0, "one window of two waited");
        assert_eq!(loaded[0].niri_p99_worst_ms, 12.0, "the worst window's p99");
        assert_eq!(loaded[0].waits[0].what, "niri took");
        assert_eq!(loaded[0].waits[0].windows, 1);
        assert_eq!(loaded[0].waits[0].most_often_in, None);
    }

    #[test]
    fn the_unit_named_most_often_is_given_beside_its_kind() {
        let all = [
            window(
                "gen",
                Band::Busy,
                1.0,
                &["apps stalled on memory 2.25% of the time, most in dev.service (2.20%)"],
            ),
            window(
                "gen",
                Band::Busy,
                1.0,
                &["apps stalled on memory 0.40% of the time, most in dev.service (0.30%)"],
            ),
            window(
                "gen",
                Band::Busy,
                1.0,
                &["apps stalled on memory 0.20% of the time, most in kitty-1.scope (0.20%)"],
            ),
        ];
        let text = text(&summary(&all), &Pen::plain(), Room::Unbounded);
        assert!(
            text.contains("busy 3 × apps stalled on memory, most often in dev.service (2)"),
            "{text}"
        );
    }

    fn in_a_call(band: Band, program: &str, thread: &str, wait_us: f64) -> Window {
        let mut w = window("gen", band, 1.0, &[]);
        w.call.insert(
            program.to_string(),
            Worst {
                thread: thread.to_string(),
                wait_us,
            },
        );
        w
    }

    #[test]
    fn a_call_s_waits_are_summed_for_each_program_apart_by_band() {
        let all = [
            in_a_call(Band::Busy, "tuple", "AudioEncoder", 20.0),
            in_a_call(Band::Busy, "tuple", "Signaling", 30.0),
            in_a_call(Band::Busy, "tuple", "DecodingQueue", 900.0),
            in_a_call(Band::Busy, "chrome", "WebRTC_Worker", 5.0),
            window("gen", Band::Loaded, 1.0, &[]),
        ];
        let text = text(&summary(&all), &Pen::plain(), Room::Unbounded);
        assert!(
            text.contains(
                "busy in a call, tuple's worst thread waited 30.0 µs a run at the median, \
                 900 µs at worst (DecodingQueue), in 3 windows"
            ),
            "{text}"
        );
        assert!(
            text.contains("busy in a call, chrome's worst thread waited 5.00 µs a run"),
            "{text}"
        );
        assert!(!text.contains("loaded in a call"), "{text}");
    }

    #[test]
    fn each_load_is_a_row_of_its_generation_s_table() {
        let all = [window("/nix/store/aaa-gen", Band::Quiet, 3.25, &[])];
        let text = text(&summary(&all), &Pen::plain(), Room::Unbounded);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "aaa-gen");
        assert!(lines[1].starts_with("LOAD   MINUTES"), "{text}");
        assert!(lines[2].starts_with("quiet"), "{text}");
        assert!(lines[2].contains("3.25 ms"), "{text}");
        assert!(
            lines[2].contains("not watched"),
            "no window watched niri's wait: {text}"
        );
    }

    #[test]
    fn a_script_reads_the_same_facts_named_by_their_schema() {
        let all = [window("/nix/store/aaa-gen", Band::Busy, 2.0, &[])];
        let answer = json(&summary(&all), 24);
        assert_eq!(answer["schema"], "lag-watch.report/1");
        assert_eq!(answer["hours"], 24);
        assert_eq!(answer["generations"][0]["loads"][0]["band"], "busy");
        assert_eq!(answer["generations"][0]["loads"][0]["niri_p99_ms"], 2.0);
        assert_eq!(
            answer["generations"][0]["loads"][0]["niri_wait_us"],
            Value::Null,
            "no window watched niri's wait"
        );
    }

    #[test]
    fn figures_keep_three_significant_digits() {
        assert_eq!(figure(3.254, "ms"), "3.25 ms");
        assert_eq!(figure(12.34, "ms"), "12.3 ms");
        assert_eq!(figure(912.6, "µs"), "913 µs");
        assert_eq!(figure(0.5, "%"), "0.50%");
        assert_eq!(figure(0.0, "%"), "0%");
    }

    #[test]
    fn nothing_watched_says_so() {
        assert_eq!(
            text(&[], &Pen::plain(), Room::Unbounded),
            "nothing watched yet\n"
        );
    }
}
