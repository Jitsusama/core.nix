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
use crate::{LinkStatus, cpu, lanes, mbit, qdisc, sh, words};

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
        blocks.push(said(&link, context.room.for_prose()));
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
    blocks.push(
        first
            .said(&Spent::now(), context.room)
            .iter()
            .flat_map(|line| set(line, 0, context.room.for_prose()))
            .collect(),
    );
    // Written whole at the end, so a reader that stops early, as
    // `grep -q` or `head` does, ends the report rather than panicking it.
    let _ = std::io::stdout().write_all(layout::joined(blocks).as_bytes());
    Ok(())
}

/// A link's name, its network, and what the shaper holds it to and why,
/// set in `room`.
fn said(link: &LinkStatus, room: Room) -> Vec<String> {
    let mut lines = vec![format!("{}  on {}", link.name, link.network)];
    let mut say = |line: String| lines.extend(set(&line, INSET.len(), room));
    say(format!("up {}, down {}", mbit(link.up), mbit(link.down)));
    for line in link.lanes.as_ref().map(lanes_said).unwrap_or_default() {
        say(line);
    }
    if let Some(why) = link.heard.as_ref().and_then(|heard| heard.why.clone()) {
        say(why);
    }
    if let Some(leader) = &link.follows {
        say(format!("at {leader}'s rate: both reach the same router"));
    }
    if link.live {
        say("real-time traffic flows: a call's or a screen share's".to_owned());
    }
    let found = link.heard.as_ref().and_then(|heard| heard.plateau);
    if let Some(plateau) = found {
        say(format!(
            "the upload's line, found under load: {}",
            mbit(whole(plateau))
        ));
    }
    let pings: Vec<String> = link
        .baselines
        .iter()
        .map(|(host, ms)| format!("{host} {ms:.0} ms"))
        .collect();
    // Parted by the kit's item mark, so a wrap gives way between round
    // trips and never leaves a host on one line and its time on the next.
    say(format!("quiet round trips: {}", pings.join(layout::ITEMS)));
    lines
}

/// A line of status set `inset` cells in, wrapped at whole words to the
/// room and hung two cells further under its first words, so where one
/// line ends and the next begins reads at a glance, as disk-guard's
/// holders do. Into a pipe it stays whole on its line.
fn set(line: &str, inset: usize, room: Room) -> Vec<String> {
    layout::hung(line, inset + layout::GAP, room)
        .split('\n')
        .enumerate()
        .map(|(at, part)| {
            let lead = if at == 0 { inset } else { 0 };
            format!("{}{part}", " ".repeat(lead))
        })
        .collect()
}

/// How the upload is shared between its lanes, Joel's first, and the
/// agents' flows waiting on each, said only while some wait.
fn lanes_said(said: &lanes::Said) -> Vec<String> {
    // The unit is said once, at the end, as the tins' table says each
    // once in its label.
    let figure = |kbit: u64| format!("{:.1}", float(kbit) / 1_000.0);
    let mut lines = vec![format!(
        "the upload's lanes: yours {}, first; the VPN's {}; agents' direct {} Mb/s",
        figure(said.joel),
        figure(said.vpn),
        figure(said.agents)
    )];
    let flows = |n: u32| {
        if n == 1 {
            "1 flow".to_owned()
        } else {
            format!("{n} flows")
        }
    };
    let waiting: Vec<String> = [
        (said.waiting_vpn, "in the VPN"),
        (said.waiting_direct, "direct"),
    ]
    .into_iter()
    .filter(|&(n, _)| n > 0)
    .enumerate()
    // The noun is said once, with the first lane's count.
    .map(|(at, (n, lane))| {
        if at == 0 {
            format!("{} {lane}", flows(n))
        } else {
            format!("{n} {lane}")
        }
    })
    .collect();
    if !waiting.is_empty() {
        lines.push(format!("agents waiting: {}", waiting.join(", ")));
    }
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

    /// What was spent between this and `later`, as lines of status set
    /// in `room`.
    fn said(&self, later: &Self, _room: Room) -> Vec<String> {
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
            lines[0].contains("DELAY ms") && lines[0].contains("PEAK ms"),
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
                lines[0].contains("MARKED") && lines[0].contains("PEAK ms"),
                "{:?}",
                lines[0]
            );
        }
    }

    fn shared(waiting_vpn: u32, waiting_direct: u32) -> lanes::Said {
        lanes::Said {
            joel: 1_740,
            vpn: 6_310,
            agents: 1_460,
            waiting_vpn,
            waiting_direct,
        }
    }

    #[test]
    fn the_lanes_say_yours_first_then_the_vpn_s_then_agents_direct() {
        assert_eq!(
            lanes_said(&shared(12, 3)),
            [
                "the upload's lanes: yours 1.7, first; the VPN's 6.3; agents' direct 1.5 Mb/s",
                "agents waiting: 12 flows in the VPN, 3 direct",
            ],
            "the lanes weren't said as Joel reads them"
        );
    }

    #[test]
    fn agents_waiting_on_one_lane_are_said_on_that_lane_alone() {
        let lines = lanes_said(&shared(0, 1));
        assert_eq!(
            lines.get(1).map(String::as_str),
            Some("agents waiting: 1 flow direct"),
            "{lines:?}"
        );
    }

    #[test]
    fn no_agent_waiting_leaves_the_waiting_line_out() {
        assert_eq!(
            lanes_said(&shared(0, 0)).len(),
            1,
            "a line said that no agent waits"
        );
    }

    /// A link as it reads on a busy afternoon, its upload held for the
    /// VPN's tunnel: a why of 118 cells, longer than the 103 that ran
    /// off an 80-column terminal on 19 November.
    fn held() -> LinkStatus {
        LinkStatus {
            name: "wlp0s20f3".to_owned(),
            ifb: "ifb4wlp0s20f3".to_owned(),
            network: "6c:ae:f6:c2:d6:b2".to_owned(),
            up: 10_800,
            down: 2_000_000,
            state: "steady".to_owned(),
            baselines: vec![
                ("1.0.0.1".to_owned(), 17.0),
                ("1.1.1.1".to_owned(), 16.0),
                ("8.8.8.8".to_owned(), 16.0),
                ("9.9.9.9".to_owned(), 58.0),
            ],
            follows: Some("enp0s13f0u1u4".to_owned()),
            live: true,
            straight: false,
            heard: Some(crate::live::Heard {
                why: Some(
                    "Upload 10.8 Mb/s: the VPN waited 89 ms inside its tunnel, so its own \
                     queue is held to 38% of its lane; held so for 36 s."
                        .to_owned(),
                ),
                plateau: Some(9_900.0),
                ..crate::live::Heard::default()
            }),
            lanes: Some(shared(12, 3)),
        }
    }

    #[test]
    fn a_link_s_lines_fit_the_room_from_sixty_columns_up() {
        for cells in [60, 80, 100, 200] {
            let text = said(&held(), Room::Cells(cells)).join("\n");
            assert_eq!(
                layout::overrun(&text, Room::Cells(cells)),
                None,
                "at {cells} columns a link's line runs past the edge"
            );
        }
    }

    #[test]
    fn a_line_too_long_for_the_room_hangs_under_its_first_words() {
        let lines = said(&held(), Room::Cells(80));
        let at = lines
            .iter()
            .position(|line| line.starts_with("  Upload"))
            .expect("the why is said");
        let next = &lines[at + 1];
        assert!(
            next.starts_with("    ") && !next.starts_with("     "),
            "{next:?} doesn't hang two cells in from {:?}",
            lines[at]
        );
    }

    #[test]
    fn a_round_trip_s_host_stays_beside_its_time() {
        for cells in [60, 80] {
            let lines = said(&held(), Room::Cells(cells));
            for (host, ms) in held().baselines {
                let trip = format!("{host} {ms:.0} ms");
                assert!(
                    lines.iter().any(|line| line.contains(&trip)),
                    "at {cells} columns {trip:?} was parted: {lines:#?}"
                );
            }
        }
    }

    #[test]
    fn into_a_pipe_each_of_a_link_s_lines_stays_whole() {
        let why = held().heard.and_then(|heard| heard.why).expect("a why");
        let lines = said(&held(), Room::Unbounded);
        assert!(
            lines.contains(&format!("{INSET}{why}")),
            "the why was parted into a pipe: {lines:#?}"
        );
        // The VM test greps these two, so each must stay on one line.
        for start in ["  quiet round trips: 1.0.0.1 17 ms", "  real-time traffic flows"] {
            assert!(
                lines.iter().any(|line| line.starts_with(start)),
                "no line starts {start:?}: {lines:#?}"
            );
        }
        assert!(
            lines.iter().all(|line| !line.starts_with("    ")),
            "a line was hung into a pipe: {lines:#?}"
        );
    }

    #[test]
    fn a_line_that_fits_the_room_stays_on_one_line() {
        let line = "  at enp0s13f0u1u4's rate: both reach the same router";
        let lines = said(&held(), Room::Cells(layout::width(line)));
        assert!(
            lines.iter().any(|each| each == line),
            "a line that fit its room was folded: {lines:#?}"
        );
    }

    #[test]
    fn however_wide_the_terminal_a_link_s_lines_are_set_at_eighty() {
        let lines = said(&held(), Room::Cells(200));
        assert!(
            lines.iter().all(|line| layout::width(line) <= layout::MEASURE),
            "a line ran past the measure at 200 columns: {lines:#?}"
        );
        assert!(
            lines.iter().any(|line| line.starts_with("    ")),
            "the why wasn't wrapped at the measure: {lines:#?}"
        );
    }

    #[test]
    fn a_network_s_name_draws_no_escape() {
        let link = LinkStatus {
            network: "6c:ae\u{1b}]0;owned\u{7}:f6".to_owned(),
            ..held()
        };
        let lines = said(&link, Room::Cells(80));
        assert!(
            !lines[0].contains('\u{1b}') && !lines[0].contains('\u{7}'),
            "the heading drew a control: {:?}",
            lines[0]
        );
    }

    /// A second of the shaper's spending, as two readings of the
    /// processor and its services.
    fn spent() -> (Spent, Spent) {
        let at = Instant::now();
        let reading = |softirq: u64, total: u64, shaper: u64| Spent {
            at,
            cores: cpu::ticks(&format!(
                "cpu  {total} 0 0 0 0 0 {softirq} 0\ncpu0 {total} 0 0 0 0 0 {softirq} 0"
            )),
            services: vec![Some(shaper), Some(shaper / 10)],
        };
        let mut later = reading(30, 1_100, 62_000);
        later.at = at + Duration::from_secs(1);
        (reading(10, 100, 0), later)
    }

    #[test]
    fn the_processor_s_lines_fit_the_room_and_the_measure() {
        let (first, later) = spent();
        for cells in [40, 60, 80, 200] {
            let lines = first.said(&later, Room::Cells(cells));
            assert!(
                lines.iter().any(|line| line.starts_with("softirq")),
                "no softirq line: {lines:#?}"
            );
            let most = cells.min(layout::MEASURE);
            assert!(
                lines.iter().all(|line| layout::width(line) <= most),
                "at {cells} columns a processor line ran past {most}: {lines:#?}"
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
