//! `net uplink`: who filled this machine's uplink over the last few
//! seconds, by purpose and whose it is, then the processes and
//! destinations sending most.

use terminal_kit::Pen;
use terminal_kit::layout::{self, Align, Room};

use crate::uplink::{Purpose, Report, Side};

/// How many processes and destinations the page lists; the answer holds them all.
const LISTED: usize = 8;

fn purpose_words(purpose: Purpose) -> &'static str {
    match purpose {
        Purpose::Sessions => "agent sessions' model requests",
        Purpose::Builders => "tunnels to remote builders",
        Purpose::Images => "container image pushes",
        Purpose::Backups => "backups",
        Purpose::Git => "git",
        Purpose::Other => "other programs",
        Purpose::Unknown => "another user's, or ended",
    }
}

fn side_words(side: Side) -> &'static str {
    match side {
        Side::Joel => "yours",
        Side::Agents => "agents'",
        Side::Unknown => "unknown",
    }
}

/// A rate as a person reads it: Mbit/s from one, kbit/s below.
fn rate(kbit: f64) -> String {
    if kbit >= 1000.0 {
        format!("{:.2} Mbit/s", kbit / 1000.0)
    } else {
        format!("{kbit:.0} kbit/s")
    }
}

/// The page: the uplink's whole rate and the shares first, then what it
/// was for, then who sent most and where to.
pub fn uplink(report: &Report, pen: &Pen, room: Room) -> String {
    let seconds = report.to - report.from;
    let link = report.uplink.as_deref().unwrap_or("no default route");
    let head = format!(
        "{link} sent {} over {seconds:.0} s: {}% yours, {}% agents'.",
        rate(report.uplink_kbit),
        report.joel_pct,
        report.agents_pct
    );
    let mut purposes: Vec<Vec<String>> = report
        .purposes
        .iter()
        .map(|share| {
            vec![
                purpose_words(share.what).to_owned(),
                rate(share.kbit),
                format!("{:.0}%", share.pct),
            ]
        })
        .collect();
    purposes.push(vec![
        "not by TCP: UDP, headers, a VPN's wrapping".to_owned(),
        rate(report.unattributed_kbit),
        String::new(),
    ]);
    let processes: Vec<Vec<String>> = report
        .processes
        .iter()
        .take(LISTED)
        .map(|process| {
            vec![
                process.name.clone(),
                process.pid.map_or_else(String::new, |pid| pid.to_string()),
                purpose_words(process.purpose).to_owned(),
                side_words(process.side).to_owned(),
                rate(process.kbit),
                format!("{:.0}%", process.resent_pct),
            ]
        })
        .collect();
    let destinations: Vec<Vec<String>> = report
        .destinations
        .iter()
        .take(LISTED)
        .map(|to| {
            vec![
                to.peer.to_string(),
                rate(to.kbit),
                format!("{:.0}%", to.resent_pct),
            ]
        })
        .collect();
    layout::joined(vec![
        layout::paragraph(&head, 0, room)
            .into_iter()
            .map(|line| pen.role("text", &line))
            .collect(),
        layout::table_in(
            &[
                ("for", Align::Left),
                ("rate", Align::Right),
                ("share", Align::Right),
            ],
            &purposes,
            pen,
            room,
            &[],
        ),
        layout::table_in(
            &[
                ("process", Align::Left),
                ("pid", Align::Right),
                ("for", Align::Left),
                ("whose", Align::Left),
                ("rate", Align::Right),
                ("resent", Align::Right),
            ],
            &processes,
            pen,
            room,
            &[5, 1, 3],
        ),
        layout::table_in(
            &[
                ("to", Align::Left),
                ("rate", Align::Right),
                ("resent", Align::Right),
            ],
            &destinations,
            pen,
            room,
            &[2],
        ),
    ])
}
