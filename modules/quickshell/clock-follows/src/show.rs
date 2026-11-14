//! What clock-follows' bare run says of its state folder: the clock's last
//! move and whether it follows Joel, as JSON or in his words. How it's said
//! changes apart from how a move is decided, which main.rs keeps.

use std::path::Path;

use serde_json::{Value, json};
use terminal_kit::{Context, layout, time};

use crate::{Held, Moved, read};

/// What the state folder holds, as JSON: the last move and any zone held,
/// each null where there's none.
pub fn shown(state: &Path) -> Value {
    let moved = read::<Moved>(&state.join("moved.json")).map(|moved| {
        json!({
            "from": moved.from,
            "to": moved.to,
            "place": moved.place,
            "moved_at": i64::try_from(moved.when).ok().map(time::rfc3339),
        })
    });
    let held = read::<Held>(&state.join("held.json")).and_then(|held| held.refused);
    json!({ "schema": "clock-follows.show/1", "moved": moved, "held": held })
}

/// The same for a person, as facts, the move's time said as how long
/// before `now` it was.
pub fn said(state: &Path, now: u64, context: &Context) -> String {
    let moved = read::<Moved>(&state.join("moved.json"));
    let held = read::<Held>(&state.join("held.json")).and_then(|held| held.refused);
    let facts = told(moved.as_ref(), held.as_deref(), now);
    let facts: Vec<(&str, Option<&str>)> = facts
        .iter()
        .map(|(label, value)| (*label, value.as_deref()))
        .collect();
    let mut lines = layout::facts(&facts, "unknown", &context.out, context.room).join("\n");
    lines.push('\n');
    lines
}

/// What's true for Joel, a fact a line: where the clock last moved and
/// whether it follows him now, in his words rather than the state's. No
/// move kept means none since he last set or put back the zone, since a
/// put back takes the move away.
fn told(
    moved: Option<&Moved>,
    held: Option<&str>,
    now: u64,
) -> Vec<(&'static str, Option<String>)> {
    let mut facts = Vec::new();
    match moved {
        Some(moved) => {
            let ago = time::span(now.saturating_sub(moved.when));
            facts.push((
                "moved",
                Some(format!("to {}, {ago} ago", zone_said(&moved.to))),
            ));
            facts.push(("from", Some(zone_said(&moved.from))));
            facts.push(("found in", moved.place.clone()));
        }
        None => facts.push((
            "moved",
            Some("no, the clock keeps the zone it was set to".to_owned()),
        )),
    }
    facts.push((
        "follows",
        Some(held.map_or_else(
            || "where you are; nothing pins it".to_owned(),
            |zone| {
                format!(
                    "where you are, but not to {} till you leave it",
                    zone_said(zone)
                )
            },
        )),
    ));
    facts
}

/// A zone as Now says it, by its city's time: `America/Edmonton` is
/// "Edmonton time". A zone with no city, such as `UTC`, is said whole.
fn zone_said(zone: &str) -> String {
    match zone.rsplit_once('/') {
        Some((_, city)) => format!("{} time", city.replace('_', " ")),
        None => zone.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::write;

    #[test]
    fn a_move_is_told_by_its_cities_and_how_long_ago() {
        let moved = Moved {
            from: "America/Toronto".into(),
            to: "America/St_Johns".into(),
            place: Some("Kelpwater".into()),
            when: 1_000,
        };
        let told = told(Some(&moved), Some("America/Vancouver"), 1_000 + 2 * 3600);
        let line = |label: &str| {
            told.iter()
                .find(|(said, _)| *said == label)
                .and_then(|(_, value)| value.clone())
                .unwrap_or_else(|| panic!("no {label} line in {told:?}"))
        };
        assert_eq!(
            line("moved"),
            "to St Johns time, 2 h ago",
            "the move wasn't told plainly"
        );
        assert_eq!(
            line("from"),
            "Toronto time",
            "where it moved from wasn't told plainly"
        );
        assert_eq!(
            line("follows"),
            "where you are, but not to Vancouver time till you leave it",
            "a zone put back wasn't told as what it means"
        );
    }

    #[test]
    fn a_clock_left_alone_says_so() {
        let told = told(None, None, 0);
        assert_eq!(
            told,
            vec![
                (
                    "moved",
                    Some("no, the clock keeps the zone it was set to".to_owned())
                ),
                ("follows", Some("where you are; nothing pins it".to_owned())),
            ],
            "a clock nothing moved or pins wasn't told plainly"
        );
        assert_eq!(
            zone_said("UTC"),
            "UTC",
            "a zone with no city should be said whole"
        );
    }

    #[test]
    fn the_state_is_shown_with_its_times_and_nulls() {
        let state = std::env::temp_dir().join(format!("clock-follows-show-{}", std::process::id()));
        let _ = fs::remove_dir_all(&state);
        fs::create_dir_all(&state).unwrap();
        let nothing = shown(&state);
        assert_eq!(nothing["schema"], "clock-follows.show/1");
        assert_eq!(nothing["moved"], Value::Null, "nothing moved says null");
        assert_eq!(nothing["held"], Value::Null);
        let moved = Moved {
            from: "America/Toronto".into(),
            to: "America/Edmonton".into(),
            place: None,
            when: 1_794_556_320,
        };
        write(&state.join("moved.json"), &moved).unwrap();
        write(
            &state.join("held.json"),
            &Held {
                refused: Some("America/Vancouver".into()),
            },
        )
        .unwrap();
        let some = shown(&state);
        assert_eq!(
            some["moved"],
            json!({
                "from": "America/Toronto", "to": "America/Edmonton",
                "place": null, "moved_at": "2026-11-13T07:52:00Z",
            })
        );
        assert_eq!(some["held"], "America/Vancouver");
        fs::remove_dir_all(&state).unwrap();
    }
}
