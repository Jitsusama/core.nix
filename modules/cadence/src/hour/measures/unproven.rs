//! Joel's asks landed with nothing showing he has what he asked for: each
//! landed over two hours ago, and its probe, the check its keeper names in
//! his terms, hasn't passed here since. On 26 November, of 586 asks marked
//! landed, one recorded him using the thing, as done was counted at the
//! merge and a switch checks the machine's health, never a given change.
//! So each is asked of the cell that answers for its whole result: run the
//! probe, fix what it shows, or say why it can't pass.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use serde_json::Value;

use super::{Class, DAY, Measure, count, first_owner, lines, seconds, short};
use crate::tracking::{self, Tracking};

/// The measure's key.
pub const KEY: &str = "asks-unproven";

/// Where the probe runs are kept, under the state folder: one line each,
/// naming the ask by its id, when it ran, whether it passed, the switch it
/// ran after and who ran it. Every cell here writes as the same person, so
/// a run is taken as its keeper's probe runner wrote it; who may write one
/// is that runner's to hold, as a cell is asked to run the probe itself.
pub const PROBES: &str = "mission/probes.jsonl";

/// Who keeps the asks list and writes each ask's probe, so answers for an
/// ask with no cell named and for the probes not yet written.
pub const KEEPER: &str = "J7XHQN";

/// What the line counting the week's asks naming no probe opens with, the
/// same from hour to hour, so its keeper is asked once while any is left.
pub const UNNAMED: &str = "his asks landed in the last week naming no probe";

/// How long after landing an ask may go with no probe passing.
const GRACE: i64 = 2 * 3600;

/// How far back a landing is still asked after one by one: older asks are
/// the probes' backfill to judge, counted on one line, not a week's worth
/// of questions at once.
const SINCE: i64 = 7 * DAY;

/// The measure, from the asks list `asks`, the switches `switches` and the
/// probe runs kept under `state_home` at `now`. A record that can't be read
/// is unread, never none passed, and so is one that's gone or emptied: its
/// keeper writes it, so a lost one would otherwise ask again of every ask
/// proven.
#[must_use]
pub fn read(state_home: &Path, asks: &str, switches: &str, now: i64) -> Measure {
    let probes = match fs::read_to_string(state_home.join(PROBES)) {
        Ok(text) if text.trim().is_empty() => Err("none kept".to_owned()),
        Ok(text) => Ok(text),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Err("none kept".to_owned()),
        Err(err) => Err(err.to_string()),
    };
    unproven(
        asks,
        probes.as_deref().map_err(String::as_str),
        switches,
        now,
    )
}

/// One probe run, as read from its line.
struct Run {
    at: i64,
    passed: bool,
    switch: Option<i64>,
}

/// A landed ask unproven: when it landed, its line and who answers for it.
type Unproven = (i64, String, String);

/// Joel's own asks in the asks list `text` landed over two hours before
/// `now` whose latest probe run since landing, in `probes`, didn't pass, or
/// which have none. Those landed in the last week naming a probe are each a
/// line, oldest landing first, saying so when no switch in `switches` has
/// come since it landed. Older ones count on one line of their own, the
/// keeper's, as the probes' backfill is, and so do those naming no probe,
/// since its keeper writes the probe and no cell can run one unwritten; so
/// none stays out of the figure, nor asks a cell for what isn't its own.
/// Each ask is read from its latest line by its id, else when it was asked
/// and its words, as the list writes an ask whole each time; a line with no
/// id is the same ask as one with an id and its time and words. One he
/// confirmed himself, whatever `confirmed` says, needs no probe, nor does
/// one put down or closed in any other way; one landed waits on no ruling,
/// as the parked measure reads it, so a ruling it was parked on before
/// can't keep it out. Each line names who answers for the ask's whole
/// result, its `answers`, else its owner, else the list's keeper. Until any ask in the
/// list names a probe, or while the runs can't be read, the measure says so
/// and goes unjudged, so no ask is asked for a probe its keeper hasn't
/// written yet and an unread record never reads as none passed.
#[must_use]
pub fn unproven(text: &str, probes: Result<&str, &str>, switches: &str, now: i64) -> Measure {
    let mut latest: BTreeMap<String, Value> = BTreeMap::new();
    for ask in lines(text) {
        // A twin the keeper folded into a kept record carries the record's
        // id and a `~`, so it neither counts again nor stands in for it.
        let key = match ask["id"].as_str().filter(|id| !id.is_empty()) {
            Some(id) if id.contains('~') => continue,
            Some(id) => id.to_owned(),
            None => said(&ask),
        };
        latest.insert(key, ask);
    }
    // An ask first written with no id and later with one is one ask; the
    // list writes an id on every line it writes now, so one with none is
    // only ever the older.
    let ided: Vec<String> = latest
        .iter()
        .filter(|(key, ask)| **key != said(ask))
        .map(|(_, ask)| said(ask))
        .collect();
    for key in ided {
        latest.remove(&key);
    }
    let named = latest.values().any(names_probe);
    let runs = probes.map_err(ToOwned::to_owned).and_then(runs);
    if let Err(why) = &runs {
        eprintln!("cadence: the probe runs are unread, {why}; a tear its keeper must mend");
    }
    let by_ask = runs.as_ref().ok();
    let switched: Vec<i64> = lines(switches).filter_map(|s| seconds(&s["at"])).collect();
    let (older, week): (Vec<_>, Vec<_>) = latest
        .values()
        .filter_map(|ask| Some((ask, unproven_ask(ask, by_ask, &switched, now)?)))
        .partition(|(_, (landed, _, _))| *landed < now - SINCE);
    let (unnamed, mut open): (Vec<_>, Vec<_>) =
        week.into_iter().partition(|(ask, _)| !names_probe(ask));
    open.sort_by_key(|(_, (landed, _, _))| *landed);
    let figure = count(open.len() + unnamed.len() + older.len());
    let mut open: Vec<Unproven> = open.into_iter().map(|(_, unproven)| unproven).collect();
    // The keeper is asked for the probes it hasn't written; the older asks
    // are its backfill, already dated, so that line holds no "; " and is
    // never filed.
    if !unnamed.is_empty() {
        let n = unnamed.len();
        let line = format!("{UNNAMED}; {n} of them · {KEEPER}");
        open.push((0, line, KEEPER.to_owned()));
    }
    if !older.is_empty() {
        let n = older.len();
        let line = format!("{n} older than a week, left to the probes' backfill · {KEEPER}");
        open.push((0, line, KEEPER.to_owned()));
    }
    let says = "his asks landed over 2 h ago with no probe passing since";
    let unjudged = match (&runs, named) {
        (Err(why), _) => Some(format!("the probe runs unread: {}", clean(why))),
        (Ok(_), false) => Some("no ask names a probe yet".to_owned()),
        (Ok(_), true) => None,
    };
    Measure {
        key: KEY.to_owned(),
        class: Class::Functional,
        says: unjudged
            .as_ref()
            .map_or_else(|| says.to_owned(), |why| format!("{says} ({why})")),
        now: figure,
        target: unjudged.is_none().then_some(0.0),
        owner: open.first().map(|(_, _, o)| o.clone()).unwrap_or_default(),
        behind: open.into_iter().map(|(_, line, _)| line).collect(),
    }
}

/// The runs in `probes` by the id of the ask each names, as several asks
/// can share the moment they were asked. A line that won't parse, or isn't
/// a run naming its ask, when it ran and whether it passed, leaves them
/// unread: a torn failing run after a pass would otherwise read as proven,
/// and a record in another shape as no probe having run.
fn runs(probes: &str) -> Result<BTreeMap<String, Vec<Run>>, String> {
    let mut by_ask: BTreeMap<String, Vec<Run>> = BTreeMap::new();
    for (n, text) in probes.lines().enumerate() {
        if text.trim().is_empty() {
            continue;
        }
        let line: Value = serde_json::from_str(text)
            .map_err(|err| format!("line {} won't parse: {err}", n + 1))?;
        // A switch it names is a number, or a later failure would read as
        // the one its cell closed.
        let switch = &line["switch"];
        let (Some(ask), Some(at), Some(passed), true) = (
            line["ask"].as_str(),
            seconds(&line["at"]),
            line["passed"].as_bool(),
            switch.is_null() || switch.is_i64(),
        ) else {
            return Err(format!(
                "line {} isn't a run naming its ask, when it ran, whether it passed and \
                 any switch by number",
                n + 1
            ));
        };
        by_ask.entry(ask.to_owned()).or_default().push(Run {
            at,
            passed,
            switch: switch.as_i64(),
        });
    }
    Ok(by_ask)
}

/// `ask`'s line and who answers for it, when it's Joel's own, landed in
/// the window before `now`, needs a probe, and its latest run since it
/// landed didn't pass or there is none. An ask with no id has none of the
/// runs, as its keeper writes the id on every line it writes a probe on;
/// with the `runs` unread, none is said to have run. One landed after every
/// switch in `switched` isn't live yet, so its cell knows to chase the
/// switch rather than the code, and a pass from before the first switch
/// since it landed proves nothing, as it ran on what doesn't carry it; with
/// no switch record, any pass since it landed proves it.
fn unproven_ask(
    ask: &Value,
    runs: Option<&BTreeMap<String, Vec<Run>>>,
    switched: &[i64],
    now: i64,
) -> Option<Unproven> {
    let landed = seconds(&ask["landed"])?;
    // Any `closed` puts it out: a twin folded into the ask that carries its
    // probe, one superseded or answered, or one an older pass closed done.
    // No open ask is ever closed.
    let waits = tracking::of(ask) == Some(Tracking::His)
        && unset(&ask["confirmed"])
        && unset(&ask["closed"])
        && landed <= now - GRACE;
    if !waits {
        return None;
    }
    let last = ask["id"]
        .as_str()
        .and_then(|id| runs?.get(id))
        .into_iter()
        .flatten()
        // A run dated past now would stay the latest whatever runs after.
        .filter(|run| run.at >= landed && run.at <= now)
        .max_by_key(|run| run.at);
    let live_from = switched.iter().copied().filter(|&at| at >= landed).min();
    let unswitched = if !switched.is_empty() && live_from.is_none() {
        ", not live yet, as no switch has come since it landed"
    } else {
        ""
    };
    let live = |run: &Run| switched.is_empty() || live_from.is_some_and(|at| run.at >= at);
    let state = match (!names_probe(ask), last) {
        (_, Some(run)) if run.passed && live(run) => return None,
        (_, Some(run)) if run.passed => {
            "its probe passed before a switch could carry it".to_owned()
        }
        (true, _) => "it names no probe".to_owned(),
        (false, None) if runs.is_none() => "its probe runs unread".to_owned(),
        (false, None) => "its probe hasn't run since it landed".to_owned(),
        (false, Some(run)) => match run.switch {
            Some(switch) => format!("its probe failed after switch {switch}"),
            None => "its probe failed".to_owned(),
        },
    };
    let owner = [&ask["answers"], &ask["owner"]]
        .into_iter()
        .filter_map(Value::as_str)
        .map(|owner| first_owner(owner).replace(':', "?"))
        .find(|owner| !owner.is_empty())
        .unwrap_or_else(|| KEEPER.to_owned());
    // Cleaned once shortened, as shortening turns any space into a plain one.
    let words = clean(&short(ask["words"].as_str().unwrap_or_default()));
    let at = clean(ask["at"].as_str().unwrap_or_default());
    let line = format!("{words}, asked {at}; {state}{unswitched} · {owner}");
    Some((landed, line, owner))
}

/// The asks list and the probe runs are other programs' text, so a newline
/// or a separator the owing rules split on can't make a line read as
/// another's.
fn clean(text: &str) -> String {
    // Nor can a backtick or double quote close the span the cell's close
    // command is given in.
    let mut text = text.replace(char::is_control, "?").replace(['`', '"'], "'");
    // Until none is left, as a replaced separator can close another.
    while text.contains("; ") || text.contains(" · ") {
        text = text.replace("; ", ";?").replace(" · ", " ? ");
    }
    text
}

/// Whether an ask names a probe: a field missing, null, false, blank or an
/// empty list or object names none.
fn names_probe(ask: &Value) -> bool {
    match &ask["probe"] {
        Value::Null | Value::Bool(false) => false,
        Value::String(probe) => !probe.trim().is_empty(),
        Value::Array(probe) => !probe.is_empty(),
        Value::Object(probe) => !probe.is_empty(),
        _ => true,
    }
}

/// Whether a field is missing, null, false or empty.
fn unset(field: &Value) -> bool {
    field.is_null() || field.as_str() == Some("") || field.as_bool() == Some(false)
}

/// An ask known by when it was asked and its words, for one with no id.
fn said(ask: &Value) -> String {
    format!("{}\n{}", ask["at"], ask["words"])
}

#[cfg(test)]
mod tests;
