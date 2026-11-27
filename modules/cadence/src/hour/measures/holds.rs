//! Live holds on the mesh with no dated fix to end them. On 24 November
//! holds stopped agents working all evening while nothing said when the
//! fix that ends each would land, and the hour saw none of it.
//!
//! A hold lives as the harness keeps it: until its holder answers or
//! replaces it, an answer ends it on the person's word, or two hours
//! pass. It's dated when the asks list's keeper names it by its short id
//! in an open ask's `hold` with an `expected` time still to come, each
//! ask's latest line deciding, since the keeper moves `hold` to a hold's
//! replacement. A date passed with the hold still on dates it no longer.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use jiff::Timestamp;

use super::cells::short_id;
use super::{Class, Measure, count, seconds, short};
use crate::mesh::{Mesh, Message, stamped_at};

/// The key the count of undated holds takes.
const KEY: &str = "holds-undated";

/// How long the harness keeps a hold no one ends or replaces.
const LASTS: i64 = 2 * 3600;

/// Who answers for the holds when none is undated: velocity sends them.
const OWNER: &str = "1OGSE7";

/// Who answers for a mesh cadence can't read: the measure's keeper.
const KEEPER: &str = "1JQPPC";

/// The measure, from the mesh's messages of the last two hours under
/// `state_home`'s harness folder and the asks list `asks`. With no
/// messages folder there, the measure is past its target, rather than
/// reading no holds as none undated.
#[must_use]
pub fn read(state_home: &Path, asks: &str, now: i64) -> Measure {
    let mesh = Mesh {
        root: state_home.join("pi/agentic-harness.pi"),
    };
    if !mesh.root.join("mesh-workflow/messages").is_dir() {
        return Measure {
            now: 1.0,
            owner: KEEPER.to_owned(),
            behind: vec!["no mesh messages folder here, so no hold could be read".to_owned()],
            ..holds(&[], "", now)
        };
    }
    holds(&mesh.sent_since((now - LASTS) * 1000), asks, now)
}

/// The holds in `sent` still live at `now` that no ask in `asks` dates,
/// oldest first, each named by its short id, its opening words and its
/// holder, who answers for the measure while it holds the oldest. Its
/// holder ends a hold by answering or replacing it, and anyone by an
/// answer saying it ends it, as the harness reads each.
#[must_use]
pub fn holds(sent: &[Message], asks: &str, now: i64) -> Measure {
    let mut ended: BTreeSet<(&str, Option<&str>)> = BTreeSet::new();
    for m in sent {
        if let Some(re) = m.re.as_deref() {
            let by = (!m.ends_hold).then_some(m.from.as_str());
            ended.insert((short_id(re), by));
        }
        if let Some(replaced) = m.supersedes.as_deref() {
            ended.insert((short_id(replaced), Some(m.from.as_str())));
        }
    }
    let dated = dated(asks, now);
    let mut undated: Vec<(i64, &Message)> = sent
        .iter()
        .filter(|m| m.intent == "hold")
        .filter(|m| {
            let id = short_id(&m.id);
            !ended.contains(&(id, None)) && !ended.contains(&(id, Some(m.from.as_str())))
        })
        .filter(|m| !dated.contains(short_id(&m.id)))
        .filter_map(|m| Some((sent_at(m)?, m)))
        .filter(|&(at, _)| at > now - LASTS && at <= now)
        .collect();
    undated.sort_by_key(|&(at, _)| at);
    let clean = |text: &str| text.replace(char::is_control, " ");
    Measure {
        key: KEY.to_owned(),
        class: Class::Slip,
        says: "live holds with no fix dated to end them".to_owned(),
        now: count(undated.len()),
        target: Some(0.0),
        owner: undated
            .first()
            .map_or_else(|| OWNER.to_owned(), |(_, m)| clean(&m.from)),
        behind: undated
            .iter()
            .map(|(_, m)| {
                let (id, from) = (clean(short_id(&m.id)), clean(&m.from));
                format!("[{id}] {} · {from}", short(&clean(&m.text)))
            })
            .collect(),
    }
}

/// When a message was sent, in seconds: its `at`, or else its id's stamp.
fn sent_at(m: &Message) -> Option<i64> {
    m.at.parse::<Timestamp>()
        .ok()
        .map(Timestamp::as_second)
        .or_else(|| stamped_at(&m.id).map(|ms| ms / 1000))
}

/// The holds the asks list `text` dates at `now`, by short id: each named in
/// `hold` on an ask's latest line, keyed by when and what was asked,
/// that is neither landed nor closed and gives an `expected` time still
/// to come.
fn dated(text: &str, now: i64) -> BTreeSet<String> {
    let mut latest = BTreeMap::new();
    for ask in super::asks(text) {
        let asked = [ask["at"].to_string(), ask["words"].to_string()];
        let open = ask["landed"].is_null() && ask["closed"].is_null();
        let due = seconds(&ask["expected"]).is_some_and(|expected| expected > now);
        let hold = ask["hold"]
            .as_str()
            .filter(|_| open && due)
            .map(|hold| short_id(hold).to_owned());
        latest.insert(asked, hold);
    }
    latest.into_values().flatten().collect()
}

#[cfg(test)]
mod tests;
