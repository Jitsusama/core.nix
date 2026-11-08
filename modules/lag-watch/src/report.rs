//! The history, summed up: for each system generation and each load band,
//! how often Joel waited and how long the desktop took, so a change is
//! judged by what it did under the same load as before it.

use crate::window::{Band, Window, count};
use std::collections::BTreeMap;
use std::fmt::Write;

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

fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
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

/// The report, in the order the generations were first seen.
pub fn render(windows: &[Window]) -> String {
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
    let mut out = String::new();
    if windows.is_empty() {
        out.push_str("nothing watched yet\n");
        return out;
    }
    for system in &order {
        let name = system.rsplit('/').next().unwrap_or(system);
        let _ = writeln!(out, "{name}");
        for band in [Band::Quiet, Band::Busy, Band::Loaded] {
            let Some(t) = tallies.get_mut(&(system.clone(), band)) else {
                continue;
            };
            let minutes = count(t.windows) * 10.0 / 60.0;
            let _ = writeln!(
                out,
                "  {:<7} {:>6.0} min  waited in {:>4.1}%  \
                 niri answers p99 {:>5.1} ms (worst {:.1}), waits {:>4.0} us; \
                 desktop CPU stall {:.2}%, apps' memory stall {:.2}%",
                format!("{band:?}").to_lowercase(),
                minutes,
                count(t.troubled) / count(t.windows) * 100.0,
                median(&mut t.niri_p99),
                worst(&t.niri_p99),
                median(&mut t.niri_wait),
                median(&mut t.session_cpu),
                median(&mut t.app_memory),
            );
            for (program, waits) in &t.calls {
                let mut us: Vec<f64> = waits.iter().map(|(us, _)| *us).collect();
                let (worst, thread) = waits
                    .iter()
                    .max_by(|a, b| a.0.total_cmp(&b.0))
                    .map_or((0.0, ""), |(us, name)| (*us, name.as_str()));
                let _ = writeln!(
                    out,
                    "            in a call, {program}'s worst thread waited {:.0} us a run \
                     at the median, {worst:.0} at worst ({thread}), in {} windows",
                    median(&mut us),
                    waits.len(),
                );
            }
            let mut kinds: Vec<(&String, &usize)> = t.kinds.iter().collect();
            kinds.sort_by(|a, b| b.1.cmp(a.1));
            for (k, n) in kinds.into_iter().take(4) {
                let most = t
                    .units
                    .get(k)
                    .and_then(|u| u.iter().max_by_key(|(_, n)| **n))
                    .map_or_else(String::new, |(u, n)| format!(", most often in {u} ({n})"));
                let _ = writeln!(out, "            {n:>4} x {k}{most}");
            }
        }
    }
    out
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
        let text = render(&all);
        let first = text.find("aaa-nixos").unwrap();
        assert!(first < text.find("bbb-nixos").unwrap());
        assert!(text.contains("loaded"));
        assert!(text.contains("waited in 50.0%"));
        assert!(text.contains("1 x niri took"));
        assert!(!text.contains("busy"));
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
        assert!(
            render(&all).contains("3 x apps stalled on memory, most often in dev.service (2)"),
            "{}",
            render(&all)
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
        let text = render(&all);
        assert!(
            text.contains(
                "in a call, tuple's worst thread waited 30 us a run at the median, \
                 900 at worst (DecodingQueue), in 3 windows"
            ),
            "{text}"
        );
        assert!(
            text.contains("chrome's worst thread waited 5 us a run at the median"),
            "{text}"
        );
        // A band with no call says nothing of one.
        let loaded = &text[text.find("loaded").unwrap()..];
        assert!(!loaded.contains("in a call"), "{text}");
    }

    #[test]
    fn nothing_watched_says_so() {
        assert_eq!(render(&[]), "nothing watched yet\n");
    }
}
