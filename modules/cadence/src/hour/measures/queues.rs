//! The queues the work waits in, each read from its own record over the
//! last day: how long its waits ran (median, p90 and worst) and how many
//! it let through. Joel asked on 20 November that every queue earn its
//! place, that one still needed regulate itself for the most flow, and
//! that the worst each day get its next change dated. The card shows each
//! queue's p90 wait against a target, with its flow beside it, so a wait
//! that falls only because nothing flows isn't read as better, and the
//! hour asks the owner of the one furthest past for that change
//! (owing.rs).
//!
//! The priority workflow and nix-offload write a wait only once it ends,
//! so a turn or a send still waiting shows only when it ends; the card
//! says their flow as how many ended. The train keeps each rider aboard in
//! a file of its own from when it boarded, so a rider still aboard counts
//! its wait so far, and a stuck train shows at once.

use super::{Class, Measure, count, lines, seconds};
use crate::stats::spread;

/// Every queue's key starts so, so the hour can file the worst apart.
pub const QUEUE: &str = "queue:";

/// Said after a queue that let nothing through in the day: it says
/// nothing of its wait, and its ask closes, since it held no work.
pub const QUIET: &str = ": none through in a day";

/// Said after a queue whose record couldn't be read: it says nothing, and
/// its ask stays open, since the hour can't tell.
pub const UNREAD: &str = " (its record unread)";

const DAY: i64 = 86_400;

/// 80ECO2 keeps all four queues' code: the harness's priority workflow,
/// nix-offload and merge-net's train.
const KEEPER: &str = "80ECO2";

/// A queue the card reads: its key after [`QUEUE`], what waits in it as
/// the card says it, and the most its p90 wait may be, in seconds.
struct Queue {
    key: &'static str,
    name: &'static str,
    target: f64,
}

/// A World git run waits for a turn so runs don't collide; a turn longer
/// than a minute is work held for nothing most of the time, since the
/// median run takes seconds.
const WALKS: Queue = Queue {
    key: "world-walks",
    name: "a World git turn",
    target: 60.0,
};

/// A build waits for the machine's cores and memory; two minutes is the
/// wait docs/delivery.md gives a VM run for a builder.
const BUILDS: Queue = Queue {
    key: "builds",
    name: "a build on this machine",
    target: 120.0,
};

/// A VM run sent away waits for a lent builder's slot: docs/delivery.md
/// wants it waiting under two minutes for one. Other sends mostly wait
/// for nothing, so they'd hide the VM runs' waits, as the builders
/// measure says.
const SENDS: Queue = Queue {
    key: "sends",
    name: "a VM run's builder",
    target: 120.0,
};

/// A train's rider waits from boarding to landed; docs/delivery.md wants a
/// change merged 20 minutes after its last commit, which boarding follows.
const TRAINS: Queue = Queue {
    key: "trains",
    name: "core's merge train, aboard",
    target: 1200.0,
};

/// A queue's waits over the day, and how many passed through it, said as
/// the card says them.
struct Waits {
    waits: Vec<f64>,
    flow: String,
}

/// Each wait in `outcomes`, the harness's priority workflow's records, of
/// the kind `kind` that ended within the day before `now`. Each line is a
/// job that ended, written then, with how long it waited for its turn,
/// given up or given.
fn turns(outcomes: &str, kind: &str, now: i64) -> Waits {
    let waits: Vec<f64> = lines(outcomes)
        .filter(|o| o["kind"].as_str() == Some(kind))
        .filter(|o| seconds(&o["ended"]).is_some_and(|at| at > now - DAY && at <= now))
        .filter_map(|o| {
            o["waitedS"].as_f64().or_else(|| {
                let asked = seconds(&o["asked"])?;
                let given = seconds(&o["started"]).or_else(|| seconds(&o["ended"]))?;
                Some(count(usize::try_from(given - asked).unwrap_or_default()))
            })
        })
        .collect();
    let flow = format!("{} ended in a day", waits.len());
    Waits { waits, flow }
}

/// Each VM run's wait in nix-offload's `sends` that ended within the day
/// before `now`.
fn sends(text: &str, now: i64) -> Waits {
    let waits: Vec<f64> = lines(text)
        .filter(|line| line["on"].as_str().is_some_and(|on| on.contains("vm")))
        .filter(|line| {
            line["at"]
                .as_i64()
                .is_some_and(|at| at > now - DAY && at <= now)
        })
        .filter_map(|line| line["waited"].as_f64())
        .collect();
    let flow = format!("{} ended in a day", waits.len());
    Waits { waits, flow }
}

/// Each rider's wait on the train: each still `aboard`, by when it
/// boarded, so far, and each landed within the day before `now`, from
/// when it boarded, where the landing says so. Its flow is the riders
/// landed, since a landing can carry several.
fn trains(landed: &str, aboard: &[i64], now: i64) -> Waits {
    let landings: Vec<serde_json::Value> = lines(landed)
        .filter(|line| {
            line["at"]
                .as_i64()
                .is_some_and(|at| at > now - DAY && at <= now)
        })
        .collect();
    let riders: usize = landings
        .iter()
        .map(|line| line["heads"].as_array().map_or(0, Vec::len))
        .sum();
    let mut waits: Vec<f64> = landings
        .iter()
        .flat_map(|line| {
            let at = line["at"].as_i64().unwrap_or_default();
            line["boarded"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_i64)
                .map(move |boarded| at - boarded)
        })
        .chain(aboard.iter().map(|boarded| now - boarded))
        .map(|waited| count(usize::try_from(waited).unwrap_or_default()))
        .collect();
    waits.sort_by(f64::total_cmp);
    let flow = format!("{riders} landed in a day, {} aboard", aboard.len());
    Waits { waits, flow }
}

/// The measure of `queue` from what its record held over the day: none
/// read says so, unjudged; nothing through says so, unjudged; else its
/// p90 against its target.
fn measure(queue: &Queue, waits: Option<Waits>) -> Measure {
    let unjudged = |says: String| Measure {
        key: format!("{QUEUE}{}", queue.key),
        class: Class::Queues,
        says,
        now: 0.0,
        target: None,
        owner: KEEPER.to_owned(),
        behind: Vec::new(),
    };
    let Some(Waits { waits, flow }) = waits else {
        return unjudged(format!("p90 seconds waiting for {}{UNREAD}", queue.name));
    };
    let Some(s) = spread(&waits) else {
        return unjudged(format!("seconds waiting for {}{QUIET}", queue.name));
    };
    Measure {
        now: s.p90,
        target: Some(queue.target),
        ..unjudged(format!(
            "p90 seconds waiting for {} (median {:.0}, worst {:.0}; {flow})",
            queue.name, s.median, s.max
        ))
    }
}

/// What the hour read of each queue's record: the text of each, or none
/// where it couldn't be read, and when each rider aboard the train
/// boarded.
pub struct Records<'a> {
    pub outcomes: Option<&'a str>,
    pub sends: Option<&'a str>,
    pub landed: Option<&'a str>,
    pub aboard: Option<&'a [i64]>,
}

/// Each queue's measure at `now`, from the priority workflow's outcomes,
/// nix-offload's sends and the merge train's landings and riders.
#[must_use]
pub fn queues(read: &Records, now: i64) -> Vec<Measure> {
    let train = read
        .landed
        .zip(read.aboard)
        .map(|(landed, aboard)| trains(landed, aboard, now));
    vec![
        measure(&WALKS, read.outcomes.map(|o| turns(o, "world-walk", now))),
        measure(&BUILDS, read.outcomes.map(|o| turns(o, "build", now))),
        measure(&SENDS, read.sends.map(|s| sends(s, now))),
        measure(&TRAINS, train),
    ]
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

/// Whether the ask filed for the queue keyed `key` is answered: its p90
/// back in, or nothing through it in a day, since a queue that held no
/// work, or went away as its ask invites, has nothing left to change. One
/// whose record couldn't be read keeps it open.
#[must_use]
pub fn answered(key: &str, measures: &[Measure]) -> bool {
    measures.iter().find(|m| m.key == key).is_none_or(|m| {
        if m.says.ends_with(QUIET) {
            return true;
        }
        m.target.is_some() && m.past().is_none()
    })
}

#[cfg(test)]
mod tests;
