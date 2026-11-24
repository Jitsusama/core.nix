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
//! its wait so far, and a stuck train shows at once; a rider that left
//! without landing, refused or stepped off, counts its wait to when it
//! left, so a train shedding riders doesn't read better than it is.
//!
//! A queue is quiet when nothing went through it, and quiet keeps its
//! ask: a turn or a send is written only once it ends, so a queue wedged
//! all day reads as one that held no work. One that let work through
//! without saying how long it waited is unjudged and keeps its ask too.
//! The train's quiet keeps it too: one that can't start turns each change
//! away to merge alone and keeps no record of it, so it reads as idle.
//! Each is judged by its p90 however few its waits: with fewer than
//! seventeen that is the worst or the second worst, so the card says they
//! were few, but a stuck wait on a quiet day was still waited.

use super::{Class, Measure, count, lines, seconds};
use crate::stats::spread;

/// Every queue's key starts so, so the hour can file the worst apart.
pub const QUEUE: &str = "queue:";

/// Said after a queue that let nothing through in the day: it says
/// nothing of its wait, and its ask stays open, since a record written as
/// each wait ends can't tell a queue that held no work from a wedged one,
/// nor the train's record an idle train from one that can't start.
pub const QUIET: &str = ": none through in a day";

/// Said after a queue whose record couldn't be read: it says nothing, and
/// its ask stays open, since the hour can't tell.
pub const UNREAD: &str = " (its record unread)";

const DAY: i64 = 86_400;

/// The net's keeper keeps all four queues' code: the harness's priority
/// workflow, nix-offload and merge-net's train.
const KEEPER: &str = crate::hour::NET_RULES;

/// The fewest waits whose p90 stands apart from their worst two, by
/// `stats::quantile`'s rank, round(0.9 * (n - 1)): from 7 to 16 it is the
/// second worst. Below it, the card says the waits were few.
const P90_FROM: usize = 17;

/// A queue the card reads: its key after [`QUEUE`], what waits in it as
/// the card says it, and the most its p90 wait may be, in seconds.
pub struct Queue {
    pub key: &'static str,
    name: &'static str,
    pub target: f64,
}

/// Every queue the card reads, so its targets can be checked against the
/// definition of good.
#[cfg(test)]
pub const ALL: [&Queue; 4] = [&WALKS, &BUILDS, &SENDS, &TRAINS];

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

/// A queue's waits over the day, how many passed through it, whether or
/// not their waits were said, and that flow as the card says it.
struct Passed {
    waits: Vec<f64>,
    through: usize,
    flow: String,
}

/// Each wait in `outcomes`, the harness's priority workflow's records, of
/// the kind `kind` that ended within the day before `now`. Each line is a
/// job that ended, written then, with how long it waited for its turn,
/// given up or given.
fn turns(outcomes: &str, kind: &str, now: i64) -> Passed {
    let ended: Vec<serde_json::Value> = lines(outcomes)
        .filter(|o| o["kind"].as_str() == Some(kind))
        .filter(|o| seconds(&o["ended"]).is_some_and(|at| at > now - DAY && at <= now))
        .collect();
    let waits: Vec<f64> = ended
        .iter()
        .filter_map(|o| {
            o["waitedS"].as_f64().or_else(|| {
                let asked = seconds(&o["asked"])?;
                let given = seconds(&o["started"]).or_else(|| seconds(&o["ended"]))?;
                Some(count(usize::try_from(given - asked).unwrap_or_default()))
            })
        })
        .collect();
    let through = ended.len();
    let flow = format!("{through} ended in a day");
    Passed {
        waits,
        through,
        flow,
    }
}

/// Each VM run's wait in nix-offload's `sends` that ended within the day
/// before `now`.
fn sends(text: &str, now: i64) -> Passed {
    let ended: Vec<serde_json::Value> = lines(text)
        .filter(|line| line["on"].as_str().is_some_and(|on| on.contains("vm")))
        .filter(|line| {
            line["at"]
                .as_i64()
                .is_some_and(|at| at > now - DAY && at <= now)
        })
        .collect();
    let waits: Vec<f64> = ended
        .iter()
        .filter_map(|line| line["waited"].as_f64())
        .collect();
    let through = ended.len();
    let flow = format!("{through} ended in a day");
    Passed {
        waits,
        through,
        flow,
    }
}

/// Each rider's wait on the train: each still `aboard`, by when it
/// boarded, so far, and each that landed, fell or stepped off within the
/// day before `now`, from when it boarded to then, where the line says so. Its flow
/// is the riders, since a landing can carry several.
fn trains(landed: &str, aboard: &[i64], now: i64) -> Passed {
    let landings: Vec<serde_json::Value> = lines(landed)
        .filter(|line| {
            line["at"]
                .as_i64()
                .is_some_and(|at| at > now - DAY && at <= now)
        })
        .collect();
    let riders = |field: &str| -> usize {
        landings
            .iter()
            .map(|line| line[field].as_array().map_or(0, Vec::len))
            .sum()
    };
    let (landed, left) = (riders("heads"), riders("fell") + riders("off"));
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
    let flow = format!(
        "{landed} landed in a day, {left} left unlanded, {} aboard",
        aboard.len()
    );
    Passed {
        waits,
        through: landed + left + aboard.len(),
        flow,
    }
}

/// The measure of `queue` from what its record held over the day: none
/// read says so, unjudged; nothing through says so, unjudged; work through
/// with no wait said says so, unjudged; else its p90 against its target,
/// said to be of few waits below [`P90_FROM`].
fn measure(queue: &Queue, waits: Option<Passed>) -> Measure {
    let unjudged = |says: String| Measure {
        key: format!("{QUEUE}{}", queue.key),
        class: Class::Queues,
        says,
        now: 0.0,
        target: None,
        owner: KEEPER.to_owned(),
        behind: Vec::new(),
    };
    let Some(Passed {
        waits,
        through,
        flow,
    }) = waits
    else {
        return unjudged(format!("p90 seconds waiting for {}{UNREAD}", queue.name));
    };
    if through == 0 {
        return unjudged(format!("seconds waiting for {}{QUIET}", queue.name));
    }
    let Some(s) = spread(&waits) else {
        return unjudged(format!(
            "seconds waiting for {}: {flow}, no wait said",
            queue.name
        ));
    };
    let judged = |now: f64, says: String| Measure {
        now,
        target: Some(queue.target),
        ..unjudged(says)
    };
    let few = if waits.len() < P90_FROM {
        ", few waits"
    } else {
        ""
    };
    judged(
        s.p90,
        format!(
            "p90 seconds waiting for {} (median {:.0}, worst {:.0}; {flow}{few})",
            queue.name, s.median, s.max
        ),
    )
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
/// back in, or the queue gone from the hour, as its ask invites. One
/// quiet, unread or unjudged keeps it open, since none says the wait came
/// back.
#[must_use]
pub fn answered(key: &str, measures: &[Measure]) -> bool {
    measures
        .iter()
        .find(|m| m.key == key)
        .is_none_or(|m| m.target.is_some() && m.past().is_none())
}

#[cfg(test)]
mod tests;
