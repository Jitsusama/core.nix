//! What the line carried over the last day, as the home view says it
//! under THE INTERNET: each way's bytes, average, share and busiest hour.

use terminal_kit::Pen;
use terminal_kit::layout::{self, Room};

use crate::figures;
use crate::model::{Hour, Network};

/// What the line carried over the last day, each way: its bytes, the
/// hours' average speed and its share of the line's, and the busiest
/// hour's, so a line full all day reads apart from one busy for an hour.
/// Nothing, when the source counted no hours.
pub(super) fn carried(network: &Network, pen: &Pen, room: Room) -> Vec<String> {
    let Some(internet) = network.internet.as_ref() else {
        return Vec::new();
    };
    let Some(carried) = internet
        .carried
        .as_ref()
        .filter(|day| !day.hours.is_empty())
    else {
        return Vec::new();
    };
    let way = |bytes: u64, line: Option<f64>, mbps: fn(&Hour) -> f64| {
        let hours: Vec<f64> = carried.hours.iter().map(mbps).collect();
        #[expect(clippy::cast_precision_loss, reason = "a day holds a few dozen hours")]
        let average = hours.iter().sum::<f64>() / hours.len() as f64;
        let busiest = hours.iter().copied().fold(0.0, f64::max);
        let share = line
            .filter(|line| *line > 0.0)
            .map(|line| format!(", {:.0}% of its speed", average / line * 100.0))
            .unwrap_or_default();
        format!(
            "{}, averaging {}{share}; {} in its busiest hour",
            gigabytes(bytes),
            figures::rate(average),
            figures::rate(busiest)
        )
    };
    let down = way(carried.down_bytes, internet.down_mbps, |hour| {
        hour.down_mbps
    });
    let up = way(carried.up_bytes, internet.up_mbps, |hour| hour.up_mbps);
    let mut lines = layout::paragraph("What the line carried over the last day:", 0, room)
        .into_iter()
        .map(|line| pen.role("text", &line))
        .collect::<Vec<_>>();
    lines.extend(layout::facts(
        &[("down", Some(down.as_str())), ("up", Some(up.as_str()))],
        "",
        pen,
        room,
    ));
    lines
}

/// Bytes as a person reads them, in decimal units as a provider bills:
/// whole gigabytes from a hundred, a tenth below, megabytes under one.
#[expect(
    clippy::cast_precision_loss,
    reason = "a day's bytes stay far below 2^52"
)]
fn gigabytes(bytes: u64) -> String {
    let gb = bytes as f64 / 1e9;
    if gb >= 100.0 {
        format!("{gb:.0} GB")
    } else if gb >= 1.0 {
        format!("{gb:.1} GB")
    } else {
        format!("{:.0} MB", gb * 1000.0)
    }
}
