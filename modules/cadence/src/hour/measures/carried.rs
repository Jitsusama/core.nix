//! Where the realm's sessions run, machine by machine, from each realm's
//! `mycelium sessions` answer: how many are busy and how many listen on
//! each, with the busy ones on this machine, the one Joel works at,
//! judged. On 23 November he found 30 sessions busy here and none on the
//! cloud machines, the morning after he asked for load to be spread off
//! it ("WE NEED TO FOCUS ON SPREADING LOAD NOW!", 22 November), and
//! nothing had said so.

use std::collections::BTreeMap;

use serde_json::Value;

use super::{Class, Measure, count};

/// Who moves sessions between machines and sets the order work starts in
/// (WCJQSG, 23 November).
const OWNER: &str = "1OGSE7";

/// The most sessions busy on the machine Joel works at, so his apps and
/// calls keep their room, the rest running elsewhere or waiting (WCJQSG,
/// 23 November).
const BUSY_HERE: f64 = 10.0;

/// One machine's sessions, as the nodes list them.
#[derive(Default)]
struct Carried {
    busy: usize,
    listening: usize,
}

/// How a listing says a session stands, least lively first. Idle is the
/// node's word for a session between turns, and only one whose presence
/// listens is woken by a message; parked and ended ones hold no room.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Lively {
    Resting,
    Listening,
    Busy,
}

impl Lively {
    fn of(session: &Value) -> Self {
        match (
            session["state"].as_str(),
            session["presence"]["state"].as_str(),
        ) {
            (Some("busy"), _) => Self::Busy,
            (Some("idle"), Some("listening")) => Self::Listening,
            _ => Self::Resting,
        }
    }
}

/// One session as the realms list it: how lively, on which machine, none
/// where it names none, and whether it was matched across realms by its
/// id or address.
struct Listing {
    lively: Lively,
    machine: String,
    matched: bool,
}

/// The measure, from each realm's `sessions` answer beside its name and
/// each realm's `machines` answer, on `here`, the machine the hour runs
/// on: its busy sessions against `BUSY_HERE`, with each machine's busy and
/// listening behind, busiest first. A machine a realm names but no session
/// runs on is named too, so an idle cloud machine shows, and sessions
/// naming no machine are a row of their own among them.
///
/// Whatever may leave the count here short is said: a realm's node didn't
/// answer, no node names this machine, or busy sessions name no machine.
/// It is unjudged while that could put the count past the most: always for
/// a silent realm; for a machine no node names, when every busy session
/// could be here; for busy sessions naming no machine, when they with this
/// machine's would pass it. A count surely past the most is judged all the
/// same, since what wasn't counted could only add to it; sessions with no
/// id or address count towards that only where one realm answered, as
/// two realms may each list one.
#[must_use]
pub fn carried(sessions: &[(String, String)], machines: &[String], here: &str) -> Measure {
    let answered: Vec<Vec<Value>> = machines
        .iter()
        .filter_map(|answer| crate::elsewhere::machines(answer))
        .collect();
    let machines_read = !answered.is_empty();
    let mut each: BTreeMap<String, Carried> = answered
        .into_iter()
        .flatten()
        .filter_map(|machine| machine["name"].as_str().map(str::to_owned))
        .map(|name| (name, Carried::default()))
        .collect();
    let (listed, unread) = listings(sessions, here);
    let named_here = each.contains_key(here) || listed.iter().any(|l| l.machine == here);
    each.entry(here.to_owned()).or_default();
    let shown = each.len();
    let one_realm = sessions.len() - unread.len() <= 1;
    let mut unplaced = Carried::default();
    let mut sure = 0;
    for listing in listed {
        let busy_here = listing.lively == Lively::Busy && listing.machine == here;
        sure += usize::from(busy_here && (listing.matched || one_realm));
        let one = if listing.machine.is_empty() {
            &mut unplaced
        } else {
            each.entry(listing.machine).or_default()
        };
        match listing.lively {
            Lively::Busy => one.busy += 1,
            Lively::Listening => one.listening += 1,
            Lively::Resting => {}
        }
    }
    let busy = each.values().map(|one| one.busy).sum::<usize>();
    let mine = each.get(here).map_or(0, |one| one.busy);
    let mut unsure = Vec::new();
    let mut short = sessions.is_empty() || !unread.is_empty();
    if sessions.is_empty() {
        unsure.push("no realm was asked".to_owned());
    }
    unsure.extend(silent(&unread));
    if !sessions.is_empty() && !named_here {
        short |= count(busy + unplaced.busy) > BUSY_HERE;
        let unnamed = if machines_read {
            format!("no node names this machine, {here}")
        } else {
            format!("no machines read, nor a session naming this machine, {here}")
        };
        unsure.push(unnamed);
    }
    if unplaced.busy > 0 {
        short |= count(mine + unplaced.busy) > BUSY_HERE;
        unsure.push(format!("{} busy naming no machine", unplaced.busy));
    }
    let target = (!short || count(sure) > BUSY_HERE).then_some(BUSY_HERE);
    let why = if unsure.is_empty() {
        String::new()
    } else {
        format!(" ({})", unsure.join("; "))
    };
    let says = format!("sessions busy on this machine, of {busy} busy on {shown} machines{why}");
    Measure {
        key: "sessions-here".to_owned(),
        class: Class::Apps,
        says,
        now: count(mine),
        target,
        owner: OWNER.to_owned(),
        behind: rows(each, unplaced),
    }
}

/// Each session the realms list, as its liveliest listing says it stands
/// and where it runs, so an ended copy one realm still lists can't hide
/// the busy one another does, as `cells` prefers a live listing; and the
/// realms whose answer couldn't be read. Sessions are matched by id, or by
/// address where a listing gives no id. Of two listings as lively, one on
/// `here` wins, then one naming any machine, so a copy elsewhere or on
/// none can't hide a busy session here. A session listed with neither id
/// nor address can't be matched across realms, so each listing counts.
fn listings<'a>(sessions: &'a [(String, String)], here: &str) -> (Vec<Listing>, Vec<&'a str>) {
    let key = |l: &Listing| (l.lively, l.machine == here, !l.machine.is_empty());
    let mut matched: BTreeMap<String, Listing> = BTreeMap::new();
    let mut each = Vec::new();
    let mut unread = Vec::new();
    for (realm, answer) in sessions {
        let read: Value = serde_json::from_str(answer).unwrap_or_default();
        let Some(all) = read["Ok"]["Sessions"].as_array() else {
            unread.push(realm.as_str());
            continue;
        };
        for session in all {
            let listing = Listing {
                lively: Lively::of(session),
                machine: session["machine"].as_str().unwrap_or_default().to_owned(),
                matched: true,
            };
            let id = ["id", "address"]
                .into_iter()
                .find_map(|key| session[key].as_str().filter(|id| !id.is_empty()));
            match id {
                None => each.push(Listing {
                    matched: false,
                    ..listing
                }),
                Some(id) if matched.get(id).is_none_or(|was| key(was) < key(&listing)) => {
                    matched.insert(id.to_owned(), listing);
                }
                Some(_) => {}
            }
        }
    }
    each.extend(matched.into_values());
    (each, unread)
}

/// The card's rows, busiest first: each machine's busy and listening, with
/// the sessions naming no machine among them, so a card showing only its
/// first few rows keeps the busiest, whichever they are.
fn rows(machines: BTreeMap<String, Carried>, unplaced: Carried) -> Vec<String> {
    let mut rows: Vec<(Option<String>, Carried)> = machines
        .into_iter()
        .map(|(name, one)| (Some(name), one))
        .collect();
    if unplaced.busy + unplaced.listening > 0 {
        rows.push((None, unplaced));
    }
    rows.sort_by(|(a, x), (b, y)| {
        y.busy
            .cmp(&x.busy)
            .then(y.listening.cmp(&x.listening))
            .then(a.cmp(b))
    });
    rows.into_iter()
        .map(|(name, one)| {
            let name = name.as_deref().unwrap_or("no machine named");
            format!("{name}: {} busy, {} listening", one.busy, one.listening)
        })
        .collect()
}

/// Why the realms in `unread` leave the count short, if any are.
fn silent(unread: &[&str]) -> Option<String> {
    match unread {
        [] => None,
        [one] => Some(format!("the {one} realm's node didn't answer")),
        many => Some(format!(
            "the {} realms' nodes didn't answer",
            many.join(" and ")
        )),
    }
}

#[cfg(test)]
mod tests;
