//! The queues the work waits in, each read from its own record over the
//! last day: how long its waits ran (median, p90 and worst) and how many
//! it let through. Joel asked on 20 November that every queue earn its
//! place, that one still needed regulate itself for the most flow, and
//! that the worst each day get its next change dated. The card shows each
//! queue's p90 wait against a target, with its flow beside it, so a wait
//! that falls only because nothing flows isn't read as better, and the
//! hour asks the owner of the one furthest past for that change
//! (owing.rs).

use super::{Class, Measure, count, lines, seconds};
use crate::stats::spread;

/// Every queue's key starts so, so the hour can file the worst apart.
pub const QUEUE: &str = "queue:";

const DAY: i64 = 86_400;

/// A queue the card reads: its key after [`QUEUE`], what waits in it as
/// the card says it, its owner's address, and the most its p90 wait may be, in
/// seconds.
struct Queue {
    key: &'static str,
    name: &'static str,
    owner: &'static str,
    target: f64,
}

/// A World git run waits for a turn so runs don't collide; a turn longer
/// than a minute is work held for nothing most of the time, since the
/// median run takes seconds.
const WALKS: Queue = Queue {
    key: "world-walks",
    name: "a World git turn",
    owner: "80ECO2",
    target: 60.0,
};

/// A build waits for the machine's cores and memory; two minutes is the
/// wait docs/delivery.md gives a VM run for a builder.
const BUILDS: Queue = Queue {
    key: "builds",
    name: "a build on this machine",
    owner: "80ECO2",
    target: 120.0,
};

/// A send waits for a lent builder's slot: docs/delivery.md wants a VM run
/// waiting under two minutes for one.
const SENDS: Queue = Queue {
    key: "sends",
    name: "a send to a builder",
    owner: "80ECO2",
    target: 120.0,
};

/// A train's riders wait from ready to landed; docs/delivery.md wants a
/// change merged 20 minutes after its last commit.
const TRAINS: Queue = Queue {
    key: "trains",
    name: "core's merge train",
    owner: "80ECO2",
    target: 1200.0,
};

/// Each wait in `outcomes`, the harness's priority workflow's records,
/// of the kind `kind` asked within the day before `now`. A run that never
/// got its turn waited until it ended, or is still waiting, so a stuck
/// turn counts rather than hiding.
fn turns(outcomes: &str, kind: &str, now: i64) -> Vec<f64> {
    lines(outcomes)
        .filter(|o| o["kind"].as_str() == Some(kind))
        .filter_map(|o| {
            let asked = seconds(&o["asked"])?;
            (asked > now - DAY && asked <= now).then_some((asked, o))
        })
        .map(|(asked, o)| match o["waitedS"].as_f64() {
            Some(waited) if !o["started"].is_null() => waited,
            _ => {
                let until = seconds(&o["ended"]).unwrap_or(now).min(now);
                count(usize::try_from(until - asked).unwrap_or_default())
            }
        })
        .collect()
}

/// Each field `field` of the lines in `text` whose `at` falls within the
/// day before `now`.
fn within(text: &str, field: &str, now: i64) -> Vec<f64> {
    lines(text)
        .filter(|line| {
            line["at"]
                .as_i64()
                .is_some_and(|at| at > now - DAY && at <= now)
        })
        .filter_map(|line| line[field].as_f64())
        .collect()
}

/// The measure of `queue` from its waits over the day, or none when the
/// record held no wait in it, since a queue nothing passed through says
/// nothing of its wait. Its flow is how many passed through.
fn measure(queue: &Queue, waits: &[f64]) -> Option<Measure> {
    let s = spread(waits)?;
    Some(Measure {
        key: format!("{QUEUE}{}", queue.key),
        class: Class::Queues,
        says: format!(
            "p90 seconds waiting for {} (median {:.0}, worst {:.0}; {} in a day)",
            queue.name, s.median, s.max, s.n
        ),
        now: s.p90,
        target: Some(queue.target),
        owner: queue.owner.to_owned(),
        behind: Vec::new(),
    })
}

/// Each queue's measure at `now`, from the priority workflow's
/// `outcomes`, nix-offload's `sends` and the merge train's `trains`; a
/// record not read, or holding nothing for the day, gives none.
#[must_use]
pub fn queues(outcomes: &str, sends: &str, trains: &str, now: i64) -> Vec<Measure> {
    [
        (&WALKS, turns(outcomes, "world-walk", now)),
        (&BUILDS, turns(outcomes, "build", now)),
        (&SENDS, within(sends, "waited", now)),
        (&TRAINS, within(trains, "seconds", now)),
    ]
    .iter()
    .filter_map(|(queue, waits)| measure(queue, waits))
    .collect()
}

/// The queue furthest past its target, which the hour asks for its next
/// change; none when every queue is in.
#[must_use]
pub fn worst(measures: &[Measure]) -> Option<&Measure> {
    measures
        .iter()
        .filter(|m| m.key.starts_with(QUEUE))
        .filter_map(|m| m.past().map(|past| (m, past)))
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(m, _)| m)
}

#[cfg(test)]
mod tests;
