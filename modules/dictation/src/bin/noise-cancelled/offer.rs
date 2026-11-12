//! Choose by Ear offered once for a microphone at a spot Joel has never
//! picked for it at, after a call there, as a row in Now he can take or
//! put off. A new microphone, or one at a new spot, cleans as the best
//! guess says (chosen.rs); the offer is how his ear gets a say.
//!
//! It's made only once a call has used the microphone there, a recording
//! of at least [`CALL`], since the shell's level meter and a program
//! testing a device record for a moment. It stands in `offer`, beside the
//! picks, only while nothing records from a microphone and no refusal
//! holds, such as a meeting being near, so it never asks during a call or
//! before one; it's withdrawn while one does and stands again once none
//! do. Taken or put off, its microphone and spot are kept in `offered`
//! and never offered again, and a pick by ear there withdraws it too.
//!
//! `offer` is one JSON object: the microphone's name and node, the spot's
//! name and key, and whether the microphone or the spot is the new one.
//! The trial tells it to the launcher as it comes and goes, with when it
//! was made, and For You shows it as a thing to do.

use std::fmt::Write as _;
use std::path::Path;
use std::sync::{Arc, mpsc};
use std::time::{Duration, UNIX_EPOCH};

use dictation::daemon::Shared;
use dictation::watch::{self, Change};
use serde_json::{Value, json};

use crate::chosen::{Chosen, written};
use crate::refusals::Refusal;
use crate::spot::Spot;
use crate::trial::Trial;

/// How long a microphone is recorded from before it counts as a call.
pub const CALL: Duration = Duration::from_mins(1);

/// Choose by Ear, offered for one microphone at one spot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    /// The microphone's node name, which its picks are kept under.
    pub node: String,
    /// The microphone's name, as Sound says it.
    pub microphone: String,
    pub spot: Spot,
    pub why: Why,
}

/// Why Choose by Ear is offered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// The microphone has never been picked for anywhere.
    Microphone,
    /// It's been picked for, but never at this spot.
    Spot,
    /// The call's room was `by` dB louder, or quieter under nought, than
    /// the one its pick here was made in, whose floor was `from` (room.rs).
    Noise { by: i32, from: i32 },
}

/// The offer waiting to stand, and what keeps it from standing.
pub struct Offers {
    refusals: Vec<Refusal>,
    waiting: Option<Offer>,
    standing: bool,
}

impl Offers {
    /// Offers kept beside `chosen`'s picks, kept from standing while any
    /// of `refusals` holds; one standing as the service starts stays.
    pub fn new(chosen: &Chosen, refusals: Vec<Refusal>) -> Self {
        let waiting = chosen
            .beside("offer")
            .and_then(|file| std::fs::read_to_string(file).ok())
            .and_then(|said| read(&said));
        Self {
            refusals,
            standing: waiting.is_some(),
            waiting,
        }
    }

    /// After `microphone`, by its `node`, was recorded from at `spot` for
    /// `lasted`, its room's floor `floor`: waits to offer Choose by Ear for
    /// it there, if that was a call, it's never been picked for there, and
    /// it's never been offered there. The newest call's waits in place of
    /// any before.
    pub fn heard(
        &mut self,
        chosen: &Chosen,
        node: &str,
        microphone: &str,
        spot: &Spot,
        lasted: Duration,
        floor: Option<i32>,
    ) {
        let _ = floor;
        if lasted < CALL || chosen.picked(node, Some(&spot.key)) || offered(chosen, node, &spot.key)
        {
            return;
        }
        self.waiting = Some(Offer {
            node: node.to_owned(),
            microphone: microphone.to_owned(),
            spot: spot.clone(),
            why: if chosen.picked(node, None) {
                Why::Spot
            } else {
                Why::Microphone
            },
        });
    }

    /// While something records: withdraws the offer, to stand again once
    /// nothing does.
    pub fn withheld(&mut self, chosen: &Chosen) {
        if self.standing {
            self.standing = false;
            removed(chosen);
        }
    }

    /// While nothing records: lets the offer stand unless a refusal holds,
    /// and forgets it once it's been taken or picked for.
    pub fn stand(&mut self, chosen: &Chosen) {
        let Some(offer) = &self.waiting else {
            return;
        };
        if chosen.picked(&offer.node, Some(&offer.spot.key))
            || offered(chosen, &offer.node, &offer.spot.key)
        {
            self.waiting = None;
            self.withheld(chosen);
            return;
        }
        let standing = !self.refusals.iter().any(Refusal::holds);
        if standing == self.standing {
            return;
        }
        self.standing = standing;
        if !standing {
            removed(chosen);
            return;
        }
        let Some(file) = chosen.beside("offer") else {
            return;
        };
        if let Err(why) = written(&file, &said(offer)) {
            eprintln!("noise-cancelled: Choose by Ear couldn't be offered: {why}");
            self.standing = false;
        }
    }
}

/// Takes the offer standing, as Joel took it or put it off: its
/// microphone and spot are kept as offered, and it's withdrawn.
pub fn taken(chosen: &Chosen) -> Result<(), String> {
    match standing_offer(chosen) {
        Some(offer) => settled(chosen, &offer.node, &offer.spot.key),
        None => Ok(()),
    }
}

/// Settles Choose by Ear for `node` at the spot `key`, as a raw pick
/// there does: kept as offered, so it's never offered there, and any offer
/// standing for it withdrawn.
pub fn settled(chosen: &Chosen, node: &str, key: &str) -> Result<(), String> {
    let Some(kept) = chosen.beside("offered") else {
        return Ok(());
    };
    if !offered(chosen, node, key) {
        let mut lines = std::fs::read_to_string(&kept).unwrap_or_default();
        let _ = writeln!(lines, "{node} {key}");
        written(&kept, &lines)?;
    }
    if standing_offer(chosen).is_some_and(|offer| offer.node == node && offer.spot.key == key) {
        removed(chosen);
    }
    Ok(())
}

/// The offer standing beside `chosen`'s picks, if one does.
fn standing_offer(chosen: &Chosen) -> Option<Offer> {
    let said = std::fs::read_to_string(chosen.beside("offer")?).ok()?;
    read(&said)
}

/// Whether Choose by Ear was offered for `node` at the spot `key`.
fn offered(chosen: &Chosen, node: &str, key: &str) -> bool {
    chosen
        .beside("offered")
        .and_then(|file| std::fs::read_to_string(file).ok())
        .is_some_and(|said| {
            said.lines()
                .any(|line| line.split_once(' ') == Some((node, key)))
        })
}

/// The offer standing beside `chosen`'s picks, as the trial tells it,
/// with `since`, when it was made, in milliseconds since the epoch; null
/// while none stands.
pub fn standing(chosen: &Chosen) -> Value {
    let Some(file) = chosen.beside("offer") else {
        return Value::Null;
    };
    let Some(offer) = std::fs::read_to_string(&file)
        .ok()
        .and_then(|said| read(&said))
    else {
        return Value::Null;
    };
    let since = std::fs::metadata(&file)
        .and_then(|about| about.modified())
        .ok()
        .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
        .map(|at| u64::try_from(at.as_millis()).unwrap_or(u64::MAX));
    let mut told = told(&offer);
    told["since"] = json!(since);
    told
}

/// Hears `file` come and go, from the kernel's inotify on its folder, so
/// nothing looks while nothing changes, calling `moved` each time.
pub fn watched(file: &Path, moved: impl Fn() + Send + 'static) -> Result<(), String> {
    let folder = file
        .parent()
        .ok_or_else(|| format!("{} has no folder", file.display()))?;
    let (changes, heard) = mpsc::channel();
    watch::watch(folder, changes)?;
    let file = file.to_owned();
    std::thread::spawn(move || {
        for change in heard {
            let (Change::Came(path) | Change::Went(path)) = change;
            if path == file {
                moved();
            }
        }
    });
    Ok(())
}

/// Tells the trial each time its offer comes or goes, so the launcher hears it as it does.
/// An offer that can't be heard is said once, and Choose by Ear carries on without it.
pub fn told_to(trial: &Arc<Shared<Trial>>, offer: Option<&Path>) {
    let Some(offer) = offer else { return };
    let me = Arc::downgrade(trial);
    let moved = move || {
        if let Some(trial) = me.upgrade() {
            trial.lock().offer_moved();
        }
    };
    if let Err(why) = watched(offer, moved) {
        eprintln!("noise-cancelled: an offer of Choose by Ear won't be heard: {why}");
    }
}

/// The offer as `offer` holds it, one line.
fn said(offer: &Offer) -> String {
    let mut said = told(offer).to_string();
    said.push('\n');
    said
}

/// The offer's fields, as `offer` holds them.
fn told(offer: &Offer) -> Value {
    let mut told = json!({
        "microphone": offer.microphone,
        "node": offer.node,
        "spot": offer.spot.name,
        "key": offer.spot.key,
    });
    told["new"] = match offer.why {
        Why::Microphone => json!("microphone"),
        Why::Spot => json!("spot"),
        Why::Noise { by, from } => {
            told["by"] = json!(by);
            told["from"] = json!(from);
            json!("noise")
        }
    };
    told
}

/// An offer from what `offer` holds, or none when it doesn't read.
fn read(said: &str) -> Option<Offer> {
    let said: Value = serde_json::from_str(said).ok()?;
    let text = |name: &str| said[name].as_str().map(str::to_owned);
    Some(Offer {
        node: text("node")?,
        microphone: text("microphone")?,
        spot: Spot {
            key: text("key")?,
            name: text("spot")?,
        },
        why: match said["new"].as_str() {
            Some("microphone") => Why::Microphone,
            Some("noise") => {
                let decibels = |name: &str| said[name].as_i64().and_then(|n| i32::try_from(n).ok());
                Why::Noise {
                    by: decibels("by")?,
                    from: decibels("from")?,
                }
            }
            _ => Why::Spot,
        },
    })
}

/// Withdraws the offer standing, if one does.
fn removed(chosen: &Chosen) {
    if let Some(file) = chosen.beside("offer") {
        let _ = std::fs::remove_file(file);
    }
}

#[cfg(test)]
#[path = "offer_tests.rs"]
mod tests;
