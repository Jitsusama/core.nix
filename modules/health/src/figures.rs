//! The day's figures, worked out from the tally and written beside it, so
//! cadence day holds the machine to its bars and its past days without
//! counting anything a second way. Health judges nothing itself.

use std::collections::BTreeMap;

use crate::day::{Boot, Day};
use crate::show::KINDS;

/// Each figure of the day, by its name: the slowest boot and shutdown in
/// seconds, each kind's errors, the minutes lost in the day's worst
/// moments, the most each watched program held, the agents that died, and
/// the asks for a secret: shown, said yes to unread, asked again, avoidable;
/// and the most any disk was in use, when a guard said any. A kind with no
/// errors is written as none, so its days have a median.
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
    figures
}

/// The day as written: its tally, and its figures beside it under
/// `figures`, which a day read back passes over, since they're worked out
/// again from the tally.
pub fn written(day: &Day) -> serde_json::Result<Vec<u8>> {
    let mut whole = serde_json::to_value(day)?;
    whole["figures"] = serde_json::to_value(of(day))?;
    serde_json::to_vec(&whole)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::Died;
    use crate::day::{Kind, Thing};
    use crate::disk::Filesystem;
    use crate::memory::Memory;

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
        let back: Day = serde_json::from_slice(&bytes).expect("the written day doesn't read back");
        assert_eq!(back, day, "the day read back isn't the day written");
    }
}
