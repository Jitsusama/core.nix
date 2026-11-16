//! `shaper status`: what the shaper holds each uplink to, what each tin
//! of its queues carried each way, and what the shaper spent of the
//! processor. Each way's tins are a table the terminal kit sets, so every
//! label sits over its own column and each unit is said once, in its
//! label; set by hand, the labels drifted off their figures.

use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};

use terminal_kit::layout::{self, Align, Room};
use terminal_kit::{Context, Exit, Failure, Pen};

use crate::rate::{float, whole};
use crate::{LinkStatus, cpu, mbit, qdisc, sh, words};

/// CAKE's four tins, in the order `tc` gives them.
const TINS: [&str; 4] = ["bulk", "best effort", "video", "voice"];

/// How far a link's lines sit in from its name.
const INSET: &str = "  ";

/// The tins' columns, by place, in the order they give way to a narrow
/// terminal: marks first, since CAKE marks only what asks it to, then the
/// peak, then drops. The tin and its packets always stay.
const GIVES_WAY: [usize; 3] = [3, 5, 2];

/// What the shaper holds each uplink to, and what each tin carried.
pub fn status(runtime: &Path, context: &Context) -> Result<(), Failure> {
    let Ok(text) = fs::read_to_string(runtime.join("status.json")) else {
        return Err(Failure::new(Exit::Unavailable, "the shaper isn't running")
            .because(format!("{} holds no status", runtime.display())));
    };
    let first = Spent::now();
    let all: Vec<LinkStatus> = serde_json::from_str(&text).unwrap_or_default();
    let mut blocks = Vec::new();
    if all.is_empty() {
        blocks.push(vec![
            "nothing to shape: no link has a default route".to_owned(),
        ]);
    }
    for link in all {
        blocks.push(said(&link));
        for (way, dev) in [("sent", &link.name), ("received", &link.ifb)] {
            let json = sh(&words(&format!("tc -s -j qdisc show dev {dev}"))).unwrap_or_default();
            blocks.push(tins_table(
                way,
                &qdisc::tins(&json),
                &context.out,
                context.room,
            ));
        }
    }
    // A share of time needs an interval: a second is long enough to hold
    // hundreds of the shaper's wakes and short enough to wait for.
    if let Some(left) = Duration::from_secs(1).checked_sub(first.at.elapsed()) {
        std::thread::sleep(left);
    }
    blocks.push(first.said(&Spent::now()));
    // Written whole at the end, so a reader that stops early, as
    // `grep -q` or `head` does, ends the report rather than panicking it.
    let _ = std::io::stdout().write_all(layout::joined(blocks).as_bytes());
    Ok(())
}

/// A link's name, its network, and what the shaper holds it to and why.
fn said(link: &LinkStatus) -> Vec<String> {
    let mut lines = vec![format!("{}  on {}", link.name, link.network)];
    let mut say = |line: String| lines.push(format!("{INSET}{line}"));
    say(format!("up {}, down {}", mbit(link.up), mbit(link.down)));
    if let Some(why) = link.heard.as_ref().and_then(|heard| heard.why.clone()) {
        say(why);
    }
    if let Some(leader) = &link.follows {
        say(format!("at {leader}'s rate: both reach the same router"));
    }
    // Said only while it holds: a quiet connection needs no line saying so.
    if link.building && link.follows.is_none() {
        say("the connection's queue is building, so the shaper is slowing".to_owned());
    }
    if link.live {
        say("real-time traffic flows, so the connection's queue is kept here".to_owned());
    }
    // The one-way controller sets the rate where it runs, and the older
    // one's held rates stay where it started, so on 14 November the
    // status said 15.4 Mbit/s was held up while 9.9 was the line found.
    let found = link.heard.as_ref().and_then(|heard| heard.plateau);
    if let Some(plateau) = found {
        say(format!(
            "the upload's line, found under load: {}",
            mbit(whole(plateau))
        ));
    } else if link.follows.is_none() {
        say(format!(
            "held under load: up {}, down {}",
            mbit(link.held_up),
            mbit(link.held_down)
        ));
    }
    let pings: Vec<String> = link
        .baselines
        .iter()
        .map(|(host, ms)| format!("{host} {ms:.0} ms"))
        .collect();
    say(format!("quiet round trips: {}", pings.join(", ")));
    lines
}

/// What each tin carried one way, as a table headed by the way, inset
/// under its link. With no tins read it says so rather than heading
/// nothing.
fn tins_table(way: &str, tins: &[qdisc::Tin], pen: &Pen, room: Room) -> Vec<String> {
    if tins.is_empty() {
        return vec![format!("{INSET}{way}: tc showed no tins")];
    }
    let columns = [
        (way, Align::Left),
        ("packets", Align::Right),
        ("dropped", Align::Right),
        ("marked", Align::Right),
        ("delay ms", Align::Right),
        ("peak ms", Align::Right),
    ];
    let ms = |us: u64| format!("{:.1}", float(us) / 1000.0);
    let rows: Vec<Vec<String>> = TINS
        .iter()
        .zip(tins)
        .map(|(name, tin)| {
            vec![
                (*name).to_owned(),
                tin.packets.to_string(),
                tin.drops.to_string(),
                tin.marks.to_string(),
                ms(tin.average_us),
                ms(tin.peak_us),
            ]
        })
        .collect();
    let inside = match room.cells() {
        Some(cells) => Room::Cells(cells.saturating_sub(INSET.len())),
        None => Room::Unbounded,
    };
    layout::table_in(&columns, &rows, pen, inside, &GIVES_WAY)
        .into_iter()
        .map(|line| format!("{INSET}{line}"))
        .collect()
}

/// The services whose CPU status shows, by their cgroups.
const SPENDERS: [(&str, &str); 2] = [
    (
        "the shaper",
        "/sys/fs/cgroup/system.slice/shaper.service/cpu.stat",
    ),
    (
        "its recorder",
        "/sys/fs/cgroup/system.slice/shaper-record.service/cpu.stat",
    ),
];

/// What the processor had spent at one moment, for `shaper status`.
struct Spent {
    at: Instant,
    cores: Vec<cpu::Ticks>,
    services: Vec<Option<u64>>,
}

impl Spent {
    fn now() -> Self {
        let stat = fs::read_to_string("/proc/stat").unwrap_or_default();
        let services = SPENDERS
            .iter()
            .map(|(_, path)| cpu::used_usec(&fs::read_to_string(path).unwrap_or_default()))
            .collect();
        Self {
            at: Instant::now(),
            cores: cpu::ticks(&stat),
            services,
        }
    }

    /// What was spent between this and `later`, as lines of status.
    fn said(&self, later: &Self) -> Vec<String> {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a second or two, far under u64's microseconds"
        )]
        let elapsed = (later.at - self.at).as_micros() as u64;
        let spent: Vec<String> = SPENDERS
            .iter()
            .zip(self.services.iter().zip(&later.services))
            .filter_map(|((name, _), pair)| match pair {
                (Some(b), Some(a)) => Some(format!(
                    "{name} {:.1}%",
                    cpu::of_a_core(*b, *a, elapsed) * 100.0
                )),
                _ => None,
            })
            .collect();
        let mut lines = Vec::new();
        if !spent.is_empty() {
            lines.push(format!("cpu: {} of a core", spent.join(", ")));
        }
        if let Some((all, busiest)) = cpu::softirq(&self.cores, &later.cores) {
            lines.push(format!(
                "softirq, where CAKE runs: {:.1}% of the machine, {:.1}% of its busiest core",
                all * 100.0,
                busiest * 100.0
            ));
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Four tins as CAKE counts them on a busy upload, with figures of
    /// every width a day gives: a packet count in the millions beside
    /// tins that carried none.
    fn busy() -> Vec<qdisc::Tin> {
        let tin = |packets, drops, marks, average_us, peak_us| qdisc::Tin {
            packets,
            bytes: packets * 1200,
            backlog: 0,
            drops,
            marks,
            average_us,
            peak_us,
            flows: 0,
        };
        vec![
            tin(4_812_007, 1_204, 0, 12_400, 48_900),
            tin(912_334, 17, 3, 1_800, 9_100),
            tin(0, 0, 0, 0, 0),
            tin(88_210, 0, 0, 300, 2_200),
        ]
    }

    /// Where each of a line's cells ends, a cell being words set apart
    /// from the next by two spaces or more.
    fn ends(line: &str) -> Vec<usize> {
        let mut ends = Vec::new();
        let mut gap = 0;
        for (at, c) in line.char_indices() {
            if c == ' ' {
                gap += 1;
                if gap == 2 && at >= 2 {
                    ends.push(at - 1);
                }
            } else {
                gap = 0;
            }
        }
        if gap < 2 && !line.trim().is_empty() {
            ends.push(line.trim_end().len());
        }
        ends
    }

    #[test]
    fn every_figures_label_ends_where_its_figures_end() {
        for cells in [60, 80, 100, 200] {
            let lines = tins_table("sent", &busy(), &Pen::plain(), Room::Cells(cells));
            // The first column's words end where each word does; every
            // column after it is figures, flush right under its label.
            let head: Vec<usize> = ends(&lines[0]).into_iter().skip(1).collect();
            for line in &lines[1..] {
                let figures: Vec<usize> = ends(line).into_iter().skip(1).collect();
                assert_eq!(
                    figures, head,
                    "at {cells} columns, {line:?} doesn't end its figures under {:?}",
                    lines[0]
                );
            }
        }
    }

    #[test]
    fn every_figure_has_a_label_of_its_own() {
        let lines = tins_table("sent", &busy(), &Pen::plain(), Room::Cells(200));
        let labels = ends(&lines[0]).len();
        for line in &lines[1..] {
            assert_eq!(
                ends(line).len(),
                labels,
                "{line:?} holds a figure with no label over it, under {:?}",
                lines[0]
            );
        }
    }

    #[test]
    fn a_unit_is_said_once_in_its_label() {
        let lines = tins_table("sent", &busy(), &Pen::plain(), Room::Cells(200));
        assert!(
            lines[0].contains("DELAY MS") && lines[0].contains("PEAK MS"),
            "{:?}",
            lines[0]
        );
        for line in &lines[1..] {
            assert!(!line.contains("ms"), "{line:?} says a unit its label says");
        }
    }

    #[test]
    fn it_fits_the_room_from_sixty_columns_up() {
        for cells in [60, 80, 100, 200] {
            for line in tins_table("received", &busy(), &Pen::plain(), Room::Cells(cells)) {
                assert!(
                    line.chars().count() <= cells,
                    "{line:?} runs past {cells} columns"
                );
            }
        }
    }

    #[test]
    fn at_eighty_columns_and_up_every_figure_stays() {
        for cells in [80, 100, 200] {
            let lines = tins_table("sent", &busy(), &Pen::plain(), Room::Cells(cells));
            assert!(
                lines[0].contains("MARKED") && lines[0].contains("PEAK MS"),
                "{:?}",
                lines[0]
            );
        }
    }

    #[test]
    fn no_tins_read_is_said_rather_than_headed() {
        let lines = tins_table("sent", &[], &Pen::plain(), Room::Cells(80));
        assert_eq!(
            lines,
            ["  sent: tc showed no tins"],
            "a table with no rows was headed"
        );
    }
}
