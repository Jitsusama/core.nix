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
        blocks.push(said(&link, context.room));
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
    blocks.push(first.said(&Spent::now(), context.room));
    // Written whole at the end, so a reader that stops early, as
    // `grep -q` or `head` does, ends the report rather than panicking it.
    let _ = std::io::stdout().write_all(layout::joined(blocks).as_bytes());
    Ok(())
}

/// A link's name, its network, and what the shaper holds it to and why,
/// set in `room` at the measure prose keeps to.
fn said(link: &LinkStatus, room: Room) -> Vec<String> {
    let room = room.for_prose();
    // The names are the kernel's and iproute2's text, which the shaper
    // doesn't control, so they're drawn through the kit's filter, which
    // keeps a control from reaching the terminal.
    let mut lines = vec![format!(
        "{}  on {}",
        layout::one_line(&link.name),
        layout::one_line(&link.network)
    )];
    // Each line of status hangs, so where one ends and the next begins
    // reads at a glance, as disk-guard's holders do.
    let mut say = |line: String| lines.extend(layout::hanging(&line, INSET.len(), room));
    say(format!("up {}, down {}", mbit(link.up), mbit(link.down)));
    for line in link.lanes.as_ref().map(lanes_said).unwrap_or_default() {
        say(line);
    }
    if let Some(why) = link.heard.as_ref().and_then(|heard| heard.why.clone()) {
        say(why);
    }
    // The controller's why speaks of its own rates, so a call's hold on
    // the download, which it didn't ask, is said on its own. A hold whose
    // download couldn't be turned through the IFB still goes straight in,
    // so its queue stands in the router's, and is said so (download hold
    // round 3's F16).
    if let Some(room) = link.room {
        say(if link.straight {
            format!(
                "a call is on: the download would be held to {}, but it couldn't be turned \
                 through its queue here, so it still queues at the router",
                mbit(room)
            )
        } else {
            format!(
                "a call is on: the download is held to {}, so its queue stands here, \
                 where the call goes first",
                mbit(room)
            )
        });
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
    fn said(&self, later: &Self, room: Room) -> Vec<String> {
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
            .iter()
            .flat_map(|line| layout::hanging(line, 0, room.for_prose()))
            .collect()
    }
}

#[cfg(test)]
mod tests;
