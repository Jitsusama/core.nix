//! Whether this machine reaches every network its node probes for it, as
//! its own record says, and what the journal says when that changes: with
//! none ready, a machine unsure of its own reach may be the one cut off.

use serde::{Deserialize, Serialize};

/// The probe every node adds to say whether a call is on here, which says
/// nothing of what this machine reaches.
const CALL: &str = "call";

/// Whether this machine's own record, in what `mycelium machines` answers,
/// says it reaches every network its node probes for it (`Some(true)`) or
/// not (`Some(false)`); `None` when it declares none, or has no record.
#[must_use]
pub fn reach(machines: &str, this: &str) -> Option<bool> {
    crate::elsewhere::machines(machines)
        .unwrap_or_default()
        .iter()
        .filter(|machine| machine["name"] == this)
        .filter_map(|machine| machine["reached"].as_object())
        .find_map(|reached| {
            let mut networks = reached
                .iter()
                .filter(|(probe, _)| *probe != CALL)
                .peekable();
            networks.peek()?;
            Some(networks.all(|(_, one)| one.as_bool() == Some(true)))
        })
}

/// Two realms' words on whether this machine reaches every network its
/// node probes, as one: a network one realm's record says it misses
/// outranks another's saying it reaches them all, and a realm with no
/// word leaves the other's standing.
#[must_use]
pub fn reaches(was: Option<bool>, realm: Option<bool>) -> Option<bool> {
    match (was, realm) {
        (Some(false), _) | (_, Some(false)) => Some(false),
        (_, Some(true)) => Some(true),
        (was, None) => was,
    }
}

/// How sure this machine is of its own reach, as a tick reads it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Reach {
    /// Its record says it reaches every network its node probes.
    #[default]
    Sure,
    /// Its record says it misses one.
    Misses,
    /// Its record says nothing of what it reaches.
    Unsaid,
    /// A realm's node didn't answer, and those that did read no network
    /// missed, so the silent one may hold the record that says.
    Unheard,
}

impl Reach {
    /// What a tick read, given whether every realm answered and what those
    /// that did say `here`: a network missed is read whoever was silent,
    /// as no other realm's word outranks it.
    #[must_use]
    pub fn of(heard_all: bool, here: Option<bool>) -> Self {
        match (heard_all, here) {
            (_, Some(false)) => Self::Misses,
            (false, _) => Self::Unheard,
            (true, Some(true)) => Self::Sure,
            (true, None) => Self::Unsaid,
        }
    }
}

/// The journal's word when this machine's reach reads otherwise than it
/// last did, from `was` to `now`, so each change is said once rather than
/// each tick. A tick a node didn't answer reads nothing sure of it, and each
/// realm's silence is said on its own line, so it says nothing here and leaves
/// what was last read standing, to be weighed against the tick after.
pub fn heard(this: &str, was: &mut Reach, now: Reach) -> Option<String> {
    let then = "so with none ready no machine is told down until it does";
    let line = match (*was, now) {
        (_, Reach::Unheard) => return None,
        (was, now) if was == now => None,
        (_, Reach::Sure) => Some(format!(
            "{this} reads every network its node probes as reached again, so a machine down \
             can be told"
        )),
        (_, Reach::Misses) => Some(format!(
            "{this} doesn't read every network its node probes as reached, {then}"
        )),
        (_, Reach::Unsaid) => Some(format!(
            "{this}'s record says nothing of the networks it reaches, {then}"
        )),
    };
    *was = now;
    line
}
