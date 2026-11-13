//! `eero speeds`: every speed test eero kept, newest first, each way's
//! spark over them, oldest to newest, and its lowest, mean and highest.

use super::clock::{self, Clock};
use super::figures::{maybe, rate, unknown};
use super::graph::spark;
use super::text::{Align, Ink, Line, Page, Table, muted, plain};
use super::{counted, head, titled};
use crate::model::{HomeNetwork, SpeedTest};

#[derive(serde::Serialize)]
struct Figures<'a> {
    speed_test: &'a SpeedTest,
    speed_tests: &'a [SpeedTest],
}

pub fn json(model: &HomeNetwork) -> Result<String, String> {
    let net = &model.internet;
    super::json::slice(
        model,
        "eero.speeds/1",
        Figures {
            speed_test: &net.speed_test,
            speed_tests: &net.speed_tests,
        },
    )
}

fn mbit(mbps: f64) -> Line {
    rate(mbps)
}

pub fn page(model: &HomeNetwork, clock: &Clock, ink: &Ink) -> Page {
    let mut page = Page::default();
    let tests = &model.internet.speed_tests;
    head(
        &mut page,
        model,
        clock,
        titled(model, Some(counted(tests.len(), "speed test"))),
    );

    let mut summary = Table::new(&[
        Align::Left,
        Align::Left,
        Align::Point,
        Align::Point,
        Align::Point,
    ]);
    summary.header(&["", "", "lowest", "mean", "highest"]);
    for (word, way) in [("down", 0), ("up", 1)] {
        // Oldest first, so the spark reads left to right as time did.
        let values: Vec<f64> = tests
            .iter()
            .rev()
            .filter_map(|test| {
                if way == 0 {
                    test.down_mbps
                } else {
                    test.up_mbps
                }
            })
            .collect();
        let lowest = values.iter().copied().reduce(f64::min);
        let highest = values.iter().copied().reduce(f64::max);
        #[expect(clippy::cast_precision_loss, reason = "a count of tests")]
        let mean = (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64);
        summary.row(vec![
            vec![muted(word)],
            spark(&values, ink.coloured()),
            maybe(lowest, mbit),
            maybe(mean, mbit),
            maybe(highest, mbit),
        ]);
    }
    page.extend(summary.lines(0));
    page.blank();

    let mut table = Table::new(&[Align::Left, Align::Left, Align::Point, Align::Point]);
    table.header(&["tested", "", "down", "up"]);
    for test in tests {
        let at = test.at.as_deref().and_then(clock::parse);
        table.row(vec![
            at.map_or_else(unknown, |at| vec![plain(clock.minute(at))]),
            at.map_or_else(Vec::new, |at| vec![muted(clock.ago(at))]),
            maybe(test.down_mbps, mbit),
            maybe(test.up_mbps, mbit),
        ]);
    }
    page.extend(table.lines(0));
    page
}
