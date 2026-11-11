//! Location's rows, ready for the launcher in the form every daemon serves
//! its rows in (Y2YBTS's ready rows, 2026-11-10): worded here from the
//! launcher's vocabulary, so the shell only draws them.
//!
//! Location's own row is `row:on:location`, the one For You's place
//! holds: its town, else its position, and how sure it is, saying so
//! where the place keeps another zone than the clock (Joel, 2026-10-28:
//! "a location area in now ... tied to what weather and any other
//! geo-conscious options pull from"), with its plate as its drawer. The
//! agents daemon places it among For You's rows, one place having one
//! server (Y2YBTS, 2026-11-10).
//!
//! Inside it are Location's own rows, `row:location:<id>`, whose one
//! server is this daemon, in the design's order (VKCF3Z's design,
//! "Location. Inside: Town, How It Knows, Set It by Hand, Find It Again,
//! Copy"): its town and its country, how it knows and when it was
//! located, setting it by hand and finding it again, each an ask on this
//! socket, and Copy, of its position, last. Each holds its place by a
//! number spaced by 1024 that never moves, so a row that comes or goes
//! moves no other.
//!
//! Nothing that reads the clock is worded: when it was found is a time,
//! said by the shell, and only turns from how long ago to its day and
//! time once a day has passed (figures.rs's `turns`).

use serde_json::{Value, json};

use crate::figures::{self, eyebrow, facts, left_out, located, unfollowed, within};
use crate::fix::Fix;
use crate::words::Words;

/// Location's own row, the one For You holds.
pub const ON: &str = "row:on:location";
/// The key every surface knows Location's row by.
const ROW: &str = "on:location";
/// What marks Location, a glyph role in the theme.
const GLYPH: &str = "location";
/// The gap between one row's order and the next's.
const SPACED: u64 = 1024;

/// Every row of Location's, for where the laptop is, the zone the clock
/// keeps, and the time now in milliseconds.
pub fn rows(here: Option<&Fix>, clock_zone: Option<&str>, words: &Words, now: u64) -> Vec<Value> {
    let zone_differs = here
        .and_then(|fix| fix.zone.as_deref())
        .filter(|zone| !zone.is_empty())
        .is_some_and(|zone| clock_zone.is_some_and(|clock| !clock.is_empty() && clock != zone));
    let how_near = here.map_or_else(String::new, |fix| {
        if fix.hand {
            words.said("hand", &[])
        } else {
            within(fix.accuracy, words)
        }
    });
    let side = here.map_or_else(
        || words.said("unknown", &[]),
        |fix| fix.place.clone().unwrap_or_else(|| position(fix)),
    );
    let mut rows = vec![on(here, words, now, &side, &how_near, zone_differs)];
    rows.extend(inside(here, words, now, &side, &how_near));
    rows
}

/// When the rows next say something another way though nothing else
/// changed, as when it was found turns from how long ago to its day.
pub fn turns(here: Option<&Fix>, now: u64) -> Option<u64> {
    here.and_then(|fix| figures::turns(fix, now))
}

/// Its position to two places, as Copy copies it and the row says it
/// where no town is named.
fn position(fix: &Fix) -> String {
    format!("{:.2}, {:.2}", fix.latitude, fix.longitude)
}

/// Location's own row: its note says how near it is, or, where the place
/// keeps another zone than the clock, that zone and why the clock isn't
/// following it, as a run that gives way by whole items; or that it isn't
/// found yet.
fn on(
    here: Option<&Fix>,
    words: &Words,
    now: u64,
    side: &str,
    how_near: &str,
    zone_differs: bool,
) -> Value {
    let note = match here {
        None => words.said("notFound", &[]),
        Some(fix) if zone_differs => {
            let zone = fix.zone.as_deref().unwrap_or_default();
            [
                words.said("elsewhere", &[("zone", zone)]),
                unfollowed(fix, words),
            ]
            .into_iter()
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>()
            .join(" \u{b7} ")
        }
        Some(_) => how_near.to_owned(),
    };
    let mut row = json!({
        "key": ON,
        "row": ROW,
        "place": "on",
        "kind": "location",
        "name": words.name,
        "glyph": GLYPH,
        "hue": if zone_differs { "strong" } else { "voice" },
        "depth": 0,
        "side": [{"said": side}],
        "note": note,
    });
    // Its plate: how it was found as its eyebrow, its town as its title,
    // what of it the side doesn't say, and what its source couldn't.
    if let Some(fix) = here {
        let town = fix
            .place
            .as_deref()
            .and_then(|place| place.split(',').next())
            .filter(|town| !town.is_empty());
        row["drawer"] = json!({
            "eyebrow": eyebrow(fix, words),
            "title": town.unwrap_or(&words.name),
            "facts": facts(fix, now, words, !zone_differs),
            "body": left_out(fix, words),
        });
    }
    row
}

/// One of Location's own rows, the `at`th in its order.
fn row(id: &str, at: u64, name: &str, side: Value) -> Value {
    json!({
        "key": format!("row:location:{id}"),
        "row": format!("location:{id}"),
        "place": "location",
        "kind": "location",
        "parent": ON,
        "depth": 1,
        "order": at * SPACED,
        "name": name,
        "side": side,
    })
}

/// One of Location's acts, as a row of its own, asking this daemon `act`.
fn act(id: &str, at: u64, words: &Words, side: Value, ask: Option<Value>) -> Value {
    let named = words.act(id);
    let mut done = json!({
        "key": format!("verb:location:{id}"),
        "name": named.name,
        "verb": named.verb,
        "glyph": named.glyph,
    });
    if let Some(ask) = ask {
        done["ask"] = ask;
    }
    let mut row = row(id, at, &named.name, side);
    row["row"] = json!(format!("verb:location:{id}"));
    row["glyph"] = json!(named.glyph);
    row["hue"] = json!("action");
    row["acts"] = json!([done]);
    row
}

/// The rows inside Location's.
fn inside(here: Option<&Fix>, words: &Words, now: u64, side: &str, how_near: &str) -> Vec<Value> {
    let said = |said: &str| json!([{"said": said}]);
    let town = here.map_or_else(
        || side.to_owned(),
        |fix| {
            fix.place
                .clone()
                .unwrap_or_else(|| words.said("unnamed", &[]))
        },
    );
    let mut rows = vec![row("town", 1, words.name("town"), said(&town))];
    if let Some(country) = here.and_then(|fix| fix.country.as_deref()) {
        rows.push(row("country", 2, words.name("country"), said(country)));
    }
    let knows = match here {
        None => words.said("notFound", &[]),
        Some(fix) if fix.hand => words.said("hand", &[]),
        Some(_) => words.said("knows", &[("within", how_near)]),
    };
    rows.push(row("knows", 3, words.name("knows"), said(&knows)));
    if let Some(fix) = here {
        rows.push(row(
            "located",
            4,
            words.name("located"),
            json!([located(fix, now)]),
        ));
    }
    // Where GeoClue can't find the laptop, or finds it wrong, Joel names
    // its town, and asks GeoClue again once he's moved on.
    let mut set = act(
        "set",
        5,
        words,
        json!([]),
        Some(json!({"ask": "act", "act": "set"})),
    );
    set["acts"][0]["asks"] = json!(words.act("set").asks);
    set["acts"][0]["chip"] = json!(words.name);
    rows.push(set);
    if here.is_some_and(|fix| fix.hand) {
        let again = act(
            "again",
            6,
            words,
            json!([]),
            Some(json!({"ask": "act", "act": "again"})),
        );
        rows.push(again);
    }
    // An act to copy, last as Copy always is, of what the town alone
    // can't say: where on a map it is.
    if let Some(fix) = here {
        let where_on_a_map = position(fix);
        let mut copy = act("copy", 7, words, said(&where_on_a_map), None);
        copy["acts"][0]["copy"] = json!(where_on_a_map);
        rows.push(copy);
    }
    rows
}

#[cfg(test)]
#[path = "rows_tests.rs"]
mod tests;
