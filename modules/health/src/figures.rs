//! The day's figures, worked out from the tally and written beside it, so
//! cadence day holds the machine to its bars and its past days without
//! counting anything a second way. Health judges nothing itself.
//!
//! Each figure says its side: Joel's, what he waited for or saw, or ours,
//! a stand-in for it, such as a count of errors he may never have seen or
//! a key pi-keys modelled. Only his side's figures can say a spec is met.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::day::{Boot, Day};
use crate::show::KINDS;
use crate::typing;

/// Whose side a figure is from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    /// What Joel waited for or saw, timed as he did it.
    His,
    /// A stand-in for it.
    Ours,
}

/// The figures, by the start of their names, that are what Joel waited
/// for or saw: his boots and shutdowns, the moments that went worst for
/// him, the launcher as it answered his keys, the lines he typed, and the
/// asks shown to him.
const HIS: [&str; 8] = [
    "boot.",
    "shutdown.",
    "feel.",
    "launcher.",
    "words.",
    "asks.shown",
    "asks.unread-yeses",
    "asks.again",
];

/// Whose side the figure `name` is from.
#[must_use]
pub fn side(name: &str) -> Side {
    if HIS.iter().any(|his| name.starts_with(his)) {
        Side::His
    } else {
        Side::Ours
    }
}

/// Each figure of the day, by its name: the slowest boot and shutdown in
/// seconds, each kind's errors, the minutes lost in the day's worst
/// moments, the most each watched program held, the agents that died, and
/// the asks for a secret: shown, said yes to unread, asked again, avoidable;
/// and the most any disk was in use, when a guard said any; each part of
/// the launcher's median, p99, slowest and share past its bar, and the worst
/// p99 of typing into pi, when any were timed. A kind with no errors is
/// written as none, so its days have a median.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "a figure only needs to be near: seconds, mebibytes and counts"
)]
pub fn of(day: &Day) -> BTreeMap<String, f64> {
    let mut figures = BTreeMap::new();
    let seconds = |micros: i64| micros as f64 / 1e6;
    if let Some(slowest) = day.boots.values().filter_map(Boot::to_ready).max() {
        figures.insert("boot.seconds".to_owned(), seconds(slowest));
    }
    if let Some(slowest) = day.boots.values().filter_map(Boot::shutdown).max() {
        figures.insert("shutdown.seconds".to_owned(), seconds(slowest));
    }
    for kind in KINDS {
        let errors: u64 = day
            .things
            .values()
            .filter(|thing| thing.kind == Some(kind))
            .map(|thing| thing.errors)
            .sum();
        figures.insert(format!("{}.errors", kind.name()), errors as f64);
    }
    let lost: f64 = day.feel.iter().map(|pain| pain.lost_seconds).sum();
    figures.insert("feel.lost-minutes".to_owned(), lost / 60.0);
    for (name, memory) in &day.memory {
        figures.insert(
            format!("memory.{name}.most-mib"),
            memory.most_kib as f64 / 1024.0,
        );
    }
    figures.insert("agents.died".to_owned(), day.agents.died.len() as f64);
    let asks = &day.asks;
    figures.insert("asks.shown".to_owned(), asks.shown as f64);
    figures.insert("asks.unread-yeses".to_owned(), asks.unread_yeses as f64);
    figures.insert("asks.again".to_owned(), asks.again as f64);
    figures.insert(
        "asks.avoidable-percent".to_owned(),
        asks.avoidable_percent(),
    );
    if !day.disks.filesystems.is_empty() {
        figures.insert(
            "disk.most-used-percent".to_owned(),
            day.disks.most_used_percent(),
        );
    }
    for part in day.launcher.parts() {
        let timed = part.timed;
        if let (Some(median), Some(p99)) = (timed.median(), timed.p99()) {
            let mut put = |figure: &str, value: f64| {
                figures.insert(format!("launcher.{}.{figure}", part.name), value);
            };
            put("median-ms", median as f64);
            put("p99-ms", p99 as f64);
            put("slowest-ms", timed.slowest_ms as f64);
            put("late-percent", timed.late_percent());
        }
    }
    if let Some(worst) = typing::worst(&day.typing) {
        figures.insert("typing.worst-p99-ms".to_owned(), worst.p99_ms);
    }
    figures
}

/// The day as written: its tally, and its figures beside it under
/// `figures` and each one's side under `sides`, which a day read back
/// passes over, since they're worked out again from the tally.
pub fn written(day: &Day) -> serde_json::Result<Vec<u8>> {
    let mut whole = serde_json::to_value(day)?;
    let figures = of(day);
    let sides: BTreeMap<&String, Side> = figures.keys().map(|name| (name, side(name))).collect();
    whole["sides"] = serde_json::to_value(sides)?;
    whole["figures"] = serde_json::to_value(&figures)?;
    serde_json::to_vec(&whole)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::Died;
    use crate::day::{Kind, Thing};
    use crate::disk::Filesystem;
    use crate::memory::Memory;
    use crate::typing::Sample;

    #[test]
    fn the_days_figures_come_from_its_tally() {
        let mut day = Day::new("2026-11-12");
        day.boots.insert(
            "a".into(),
            Boot {
                started: 1,
                firmware: 9_000_000,
                loader: 1_000_000,
                asked: Some(2_000_000),
                opened: Some(22_000_000),
                ready: Some(37_000_000),
                stopping: Some(100_000_000),
                ended: Some(104_500_000),
                ..Boot::default()
            },
        );
        day.things.insert(
            "chrome".into(),
            Thing {
                kind: Some(Kind::Programs),
                errors: 7,
                ..Thing::default()
            },
        );
        day.memory.insert(
            "pi".into(),
            Memory {
                most_kib: 30 << 20,
                ..Memory::default()
            },
        );
        day.agents.died.insert("s".into(), Died::default());
        day.asks.shown = 4;
        day.asks.again = 2;
        day.disks.filesystems.insert(
            "/".into(),
            Filesystem {
                size: 200,
                least_free: 30,
                ..Filesystem::default()
            },
        );
        let figures = of(&day);
        let figure = |name: &str| figures.get(name).copied();
        assert_eq!(
            (
                figure("boot.seconds"),
                figure("shutdown.seconds"),
                figure("programs.errors"),
                figure("devices.errors"),
                figure("memory.pi.most-mib"),
                figure("agents.died"),
                figure("asks.shown"),
                figure("asks.again"),
                figure("disk.most-used-percent"),
            ),
            (
                Some(27.0),
                Some(4.5),
                Some(7.0),
                Some(0.0),
                Some(30_720.0),
                Some(1.0),
                Some(4.0),
                Some(2.0),
                Some(85.0)
            ),
            "a figure wasn't worked out from the day's tally: {figures:?}"
        );
    }

    #[test]
    fn the_launcher_and_typing_are_figured_when_timed() {
        let mut day = Day::new("2026-11-12");
        day.launcher.took(
            "launcher: 4 drawers in home presented a median 42 ms from the key, the slowest \
             134 ms, the p99 134 ms, 3 within 50 ms",
        );
        day.typing = vec![
            Sample {
                keys: 400,
                p99_ms: 21.0,
                ..Sample::default()
            },
            Sample {
                keys: 400,
                p99_ms: 9.0,
                ..Sample::default()
            },
        ];
        let figures = of(&day);
        let figure = |name: &str| figures.get(name).copied();
        assert_eq!(
            (
                figure("launcher.drawers.median-ms"),
                figure("launcher.drawers.p99-ms"),
                figure("launcher.drawers.slowest-ms"),
                figure("launcher.drawers.late-percent"),
                figure("launcher.keys.median-ms"),
                figure("typing.worst-p99-ms"),
            ),
            (
                Some(42.0),
                Some(134.0),
                Some(134.0),
                Some(25.0),
                None,
                Some(21.0)
            ),
            "the launcher's parts or typing in pi weren't figured from the day: {figures:?}"
        );
    }

    #[test]
    fn only_what_joel_waited_for_or_saw_is_his() {
        let sides = [
            "boot.seconds",
            "launcher.keys.late-percent",
            "words.errors",
            "asks.again",
            "asks.avoidable-percent",
            "programs.errors",
            "typing.worst-p99-ms",
            "memory.pi.most-mib",
        ]
        .map(side);
        assert_eq!(
            sides,
            [
                Side::His,
                Side::His,
                Side::His,
                Side::His,
                Side::Ours,
                Side::Ours,
                Side::Ours,
                Side::Ours
            ],
            "a stand-in was counted as Joel's side, or what he waited for as ours"
        );
    }

    #[test]
    fn the_day_is_written_with_its_figures_and_read_back_without() {
        let mut day = Day::new("2026-11-12");
        day.agents.died.insert("s".into(), Died::default());
        let bytes = written(&day).expect("the day wasn't written");
        let read: serde_json::Value = serde_json::from_slice(&bytes).expect("not JSON");
        assert_eq!(
            read["figures"]["agents.died"],
            serde_json::json!(1.0),
            "the written day has no figures beside its tally: {read}"
        );
        assert_eq!(
            (
                &read["sides"]["agents.died"],
                &read["sides"]["boot.seconds"]
            ),
            (&serde_json::json!("ours"), &serde_json::Value::Null),
            "a figure written says no side, or a figure not written does: {read}"
        );
        let back: Day = serde_json::from_slice(&bytes).expect("the written day doesn't read back");
        assert_eq!(back, day, "the day read back isn't the day written");
    }
}
