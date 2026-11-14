//! The notice in front: the one the arrival line shows (the shell's
//! NoticeLine.qml), chosen here rather than in the shell, since ordering
//! is the daemons' (Joel, 2026-11-12: "the daemons need to be the data
//! layer, and they need to serve the launcher"; Y2YBTS, 2026-11-13). It's
//! served as one record beside the notices' own, made again with them, so
//! one read, withdrawn, resting or gone leaves the front as it changes.

use serde_json::{Value, json};

use crate::notice::{Notice, Urgency};

/// The front's record key.
pub const KEY: &str = "front:notices";

/// Whether it still stands on the screen asking something of Joel: open,
/// not resting off the screen, and neither read nor withdrawn.
fn waits(notice: &Notice) -> bool {
    notice.live && !notice.resting && !notice.read && !notice.withdrawn
}

/// When it was last brought to the front: said again with something new,
/// or as it came for one that stays until it's answered. One that's
/// neither waits its turn behind the one before it.
fn fronted(notice: &Notice) -> Option<i64> {
    notice.raised_at.or(notice.urgent.then_some(notice.at))
}

/// Those waiting in the order the line takes them: each brought to the
/// front, the last first, then the rest in the order they came, as the
/// shell's stack kept them (Stack.js).
fn order(notices: &[Notice]) -> Vec<&Notice> {
    let mut waiting: Vec<&Notice> = notices.iter().filter(|notice| waits(notice)).collect();
    waiting.sort_by_key(|notice| match fronted(notice) {
        Some(at) => (false, -at, notice.id),
        None => (true, notice.at, notice.id),
    });
    waiting
}

/// The record served: `{ key, kind: "front", front }`, where `front` is
/// the id of the one in front, the id of the first critical one, which
/// alone shows while notices are held (the shell's Quiet.qml), each null
/// where there's none, and how many wait in all.
pub fn record(notices: &[Notice]) -> Value {
    let order = order(notices);
    let critical = order
        .iter()
        .find(|notice| notice.urgency == Urgency::Critical);
    json!({
        "key": KEY,
        "kind": "front",
        "front": {
            "id": order.first().map(|notice| notice.id),
            "critical": critical.map(|notice| notice.id),
            "waiting": order.len(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open(id: u32, at: i64) -> Notice {
        Notice {
            id,
            at,
            live: true,
            ..Notice::default()
        }
    }

    fn front(notices: &[Notice]) -> Value {
        record(notices)["front"].clone()
    }

    #[test]
    fn the_oldest_waiting_is_in_front_and_every_one_waiting_is_counted() {
        let said = front(&[open(3, 30), open(2, 20), open(1, 10)]);
        let wanted = json!({ "id": 1, "critical": null, "waiting": 3 });
        assert_eq!(said, wanted, "the first to come isn't in front");
    }

    #[test]
    fn one_read_withdrawn_resting_or_gone_leaves_the_front() {
        let read = Notice {
            read: true,
            ..open(1, 10)
        };
        let withdrawn = Notice {
            withdrawn: true,
            ..open(2, 20)
        };
        let resting = Notice {
            resting: true,
            ..open(3, 30)
        };
        let gone = Notice {
            live: false,
            ..open(4, 40)
        };
        let said = front(&[open(5, 50), gone, resting, withdrawn, read]);
        let wanted = json!({ "id": 5, "critical": null, "waiting": 1 });
        assert_eq!(said, wanted, "one done with stayed in front");
    }

    #[test]
    fn one_that_stays_or_was_raised_comes_to_the_front_the_last_first() {
        let urgent = Notice {
            urgent: true,
            ..open(2, 20)
        };
        let raised = Notice {
            raised_at: Some(40),
            ..open(1, 10)
        };
        let notices = [open(4, 50), raised.clone(), urgent.clone()];
        let said = front(&notices)["id"].clone();
        assert_eq!(said, json!(1), "the one raised last isn't in front");
        let later = Notice { at: 60, ..urgent };
        let said = front(&[open(4, 50), raised, later])["id"].clone();
        assert_eq!(said, json!(2), "a later one that stays isn't in front");
    }

    #[test]
    fn the_first_critical_is_named_for_while_notices_are_held() {
        let critical = |id, at| Notice {
            urgency: Urgency::Critical,
            ..open(id, at)
        };
        let said = front(&[critical(3, 30), open(2, 20), open(1, 10)]);
        let wanted = json!({ "id": 1, "critical": 3, "waiting": 3 });
        assert_eq!(said, wanted, "the critical one first in line isn't named");
    }

    #[test]
    fn none_waiting_has_no_front() {
        let said = record(&[]);
        let wanted = json!({
            "key": "front:notices",
            "kind": "front",
            "front": { "id": null, "critical": null, "waiting": 0 },
        });
        assert_eq!(said, wanted, "a front was served with nothing waiting");
    }
}
