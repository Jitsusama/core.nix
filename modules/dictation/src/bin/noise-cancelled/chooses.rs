//! Noise cancelling choosing its own cleaner for a microphone at a spot,
//! from how its calls there scored (plan item 17). Each call is judged in
//! the shadow (shadow.rs), and once the last two calls on a microphone at a
//! spot both say one cleaner would have cleaned better than what ran, by
//! [`MARGIN`] overall, the next call there cleans with it. It changes only
//! between calls, the steadiest moment there is, so nothing a call hears
//! changes under it.
//!
//! Joel's ear comes first: where he's picked by ear, it never chooses, and
//! it chooses nowhere until the scorer has shown it hears as he does, its
//! best matching [`AGREED`] of his [`NEWEST`] picks by ear (Choose by
//! Ear's `scored`). By chance alone, with four ways to pick from, that
//! happens once in about 70 (p = 0.014). Until then it only says what it
//! would have chosen. What it chooses is kept in `chose`, beside the
//! picks, a line a choice as the picks are.

use serde_json::Value;

use crate::chosen::Chosen;
use crate::denoise::Cleaner;
use crate::spot::Spot;

/// How far over what ran, overall, a cleaner scores in each of two calls.
pub const MARGIN: f64 = 0.2;

/// How many of Joel's newest picks by ear the agreement is counted over.
pub const NEWEST: usize = 20;

/// How many of those the scorer's best matches before it chooses.
pub const AGREED: usize = 10;

/// How many of Joel's newest picks in `scored` the scorer's best matched,
/// and how many were counted.
pub fn agreement(scored: &str) -> (usize, usize) {
    let picks: Vec<Value> = calls(scored).collect();
    let newest = &picks[picks.len().saturating_sub(NEWEST)..];
    let matched = newest
        .iter()
        .filter(|pick| {
            let picked = pick.get("picked").and_then(Value::as_str);
            picked.is_some() && picked == best(pick, true).as_deref()
        })
        .count();
    (matched, newest.len())
}

/// The cleaner the last two calls judged on `node` at the spot keyed `key`
/// both say cleans better than what ran, by [`MARGIN`] overall, or none.
pub fn better(judged: &str, node: &str, key: &str) -> Option<Cleaner> {
    let here: Vec<Value> = calls(judged)
        .filter(|call| {
            call.get("node").and_then(Value::as_str) == Some(node)
                && call.get("key").and_then(Value::as_str) == Some(key)
        })
        .collect();
    let [.., before, last] = here.as_slice() else {
        return None;
    };
    let first = over(before)?;
    (over(last)? == first).then_some(first)
}

/// The cleaner a judged call says scores best, when that isn't what ran
/// and beats it by [`MARGIN`] overall. Raw is passed over, since a twin
/// always cleans with one of the cleaners.
fn over(call: &Value) -> Option<Cleaner> {
    let running = call.get("running").and_then(Value::as_str)?;
    let best = best(call, false)?;
    if best == running {
        return None;
    }
    let lead = overall(call, &best)? - overall(call, running)?;
    if lead < MARGIN {
        return None;
    }
    Cleaner::named(&best).ok()
}

/// The way a line's marks score best overall, raw among them or not.
fn best(line: &Value, raw: bool) -> Option<String> {
    line.get("marks")?
        .as_object()?
        .iter()
        .filter(|(way, _)| raw || *way != "raw")
        .filter_map(|(way, _)| Some((way, overall(line, way)?)))
        .max_by(|(_, one), (_, other)| one.total_cmp(other))
        .map(|(way, _)| way.clone())
}

/// The overall mark a line gives `way`.
fn overall(line: &Value, way: &str) -> Option<f64> {
    line.get("marks")?.get(way)?.get("overall")?.as_f64()
}

/// Every line of `kept` that reads as a record, oldest first.
fn calls(kept: &str) -> impl Iterator<Item = Value> + '_ {
    kept.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(Value::is_object)
}

/// Once a call on `node` at `spot` is judged: chooses the cleaner its
/// last two calls there say is better, unless Joel picked by ear there or
/// the scorer hasn't agreed with his ear yet. Answers what it chose.
pub fn decided(chosen: &Chosen, node: &str, spot: &Spot) -> Option<Cleaner> {
    if chosen.picked(node, Some(&spot.key)) {
        return None;
    }
    let read = |name: &str| {
        chosen
            .beside(name)
            .and_then(|file| std::fs::read_to_string(file).ok())
            .unwrap_or_default()
    };
    let cleaner = better(&read("judged"), node, &spot.key)?;
    if chosen.cleaner(Some(node), Some(spot)) == cleaner {
        return None;
    }
    let (matched, counted) = agreement(&read("scored"));
    if matched < AGREED {
        eprintln!(
            "noise-cancelled: would clean {node} at {} with {cleaner}, but the scorer has \
             matched only {matched} of Joel's {counted} newest picks by ear",
            spot.name
        );
        return None;
    }
    if let Err(why) = chosen.chose_itself(node, spot, cleaner) {
        eprintln!("noise-cancelled: keeping what it chose: {why}");
        return None;
    }
    eprintln!(
        "noise-cancelled: chose {cleaner} for {node} at {}, two calls there scoring it \
         {MARGIN} over what ran",
        spot.name
    );
    Some(cleaner)
}

#[cfg(test)]
#[path = "chooses_tests.rs"]
mod tests;
