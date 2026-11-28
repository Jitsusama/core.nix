//! The detectors, each a function from how things stand to what it finds.
//! A threshold is set for a stated false-alarm rate on Joel's own history
//! where there's one to set (the burn's, by its run length between false
//! alarms), and many cuts tested at once are held to a false discovery
//! rate (the peers'). Each finding says what drove it and what to do.

use crate::config::Config;
use crate::findings::{Action, Finding, Severity};
use crate::history::History;
use crate::money::{percent, whole};
use crate::picture::Picture;
use std::collections::HashSet;

mod billing;
mod burn;
mod caps;
mod changes;
mod sources;
mod work;
pub use billing::{
    UNBILLED_HELD, UNBILLED_NOT_SETTLING, UNBILLED_UNSETTLED, UNRESOLVED, unbilled, unresolved,
};
pub use burn::{BURN_DAYS, Minutes, burn};
pub use caps::{caps, refusals};
pub use changes::{novelty, prices, regimes, spikes};
pub use sources::{health, said};
pub use work::{peers, stalled, waste};

fn finding(
    key: String,
    detector: &str,
    severity: Severity,
    title: String,
    detail: String,
) -> Finding {
    Finding {
        key,
        detector: detector.into(),
        severity,
        title,
        detail,
        dollars: 0.0,
        surprise: 1.0,
        badge: String::new(),
        note: String::new(),
        session: None,
        quest: None,
        action: None,
        cap: false,
        started: 0.0,
        marked: false,
    }
}

/// A short, stable name for something too long or too private to be a
/// key itself: a credential, a sentence a provider said. FNV-1a, 64 bits.
fn digest(text: &str) -> String {
    let hash = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{hash:016x}")
}

/// What a feed offers for a kind of problem, as the finding's action.
fn offered(picture: &Picture, fits: &str) -> Option<Action> {
    picture
        .actions
        .iter()
        .find(|action| action.fits.iter().any(|fit| fit == fits))
        .map(|action| Action {
            name: action.name.clone(),
            run: action.run.clone(),
        })
}

/// Going to a session, where it runs here: another machine's has no
/// terminal on this one to go to, so its finding offers nothing.
fn go_to(picture: &Picture, session: &str) -> Option<Action> {
    let elsewhere = picture
        .pieces
        .iter()
        .any(|piece| piece.session == session && !piece.machine.is_empty());
    (!elsewhere).then(|| Action {
        name: "Go to the session".into(),
        run: ["spend-watch", "sessions", session, "go"]
            .map(String::from)
            .to_vec(),
    })
}

pub fn all(picture: &Picture, history: &History, config: &Config) -> Result<Vec<Finding>, String> {
    all_keeping(picture, history, config, &mut Minutes::default())
}

/// What's found, taking the minutes a running daemon kept.
pub fn all_keeping(
    picture: &Picture,
    history: &History,
    config: &Config,
    minutes: &mut Minutes,
) -> Result<Vec<Finding>, String> {
    let mut found = Vec::new();
    found.extend(caps(picture));
    found.extend(refusals(picture));
    found.extend(burn(picture, history, config, minutes)?);
    found.extend(waste(picture, config));
    found.extend(peers(picture, config));
    found.extend(stalled(picture, config));
    found.extend(regimes(picture));
    found.extend(novelty(picture, history, config)?);
    found.extend(spikes(picture));
    found.extend(prices(picture));
    found.extend(health(picture));
    found.extend(unbilled(picture, history, config)?);
    found.extend(unresolved(picture, history)?);
    found.extend(said(picture));
    let notes_only: HashSet<&str> = config
        .detectors
        .notes_only
        .iter()
        .map(String::as_str)
        .collect();
    for finding in &mut found {
        if notes_only.contains(finding.detector.as_str()) && !finding.cap {
            finding.severity = finding.severity.min(Severity::Note);
        }
    }
    Ok(found)
}

// What asks for a look says why in the margin's footer, beside one hint,
// so each of these notes is held to what that shows.

fn ending_note(after: f64, fallback: f64) -> String {
    // The side already says when it ends.
    format!("{} likely over {}", percent(after), whole(fallback))
}

fn changed_note(before: Option<f64>, spent: f64) -> String {
    format!(
        "was {}; {} spent",
        before.map_or("no limit".into(), whole),
        whole(spent)
    )
}

fn refused_note(refused: usize, since: &str) -> String {
    format!("{refused} refused since {since}")
}

const NEW_NOTE: &str = "first in thirty days";

fn elsewhere_note(week: f64, month: f64) -> String {
    format!("{} of pi's, usually {}", percent(week), percent(month))
}

#[cfg(test)]
mod tests;
