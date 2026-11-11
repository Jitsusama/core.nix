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
//! socket, and Copy, of its position, last. They're handed over in that
//! order and placed by the ready-rows crate's Kept, which keeps each
//! row's order where it can, so a row that comes or goes moves no other.
//! A figure copies what it says, and an act is `{key, ask}` or `{key,
//! copy}`: the shell reads its verb, and what it asks Joel, by its key
//! (ready-rows.md's acts).
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
        "thing": ROW,
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

/// One of Location's own rows, saying `side`, drawn as a figure is.
fn row(id: &str, name: &str, side: &Value) -> Value {
    json!({
        "key": format!("row:location:{id}"),
        "thing": format!("location:{id}"),
        "place": "location",
        "kind": "location",
        "parent": ON,
        "depth": 1,
        "name": name,
        "hue": "voice",
        "side": side,
    })
}

/// One of Location's own rows saying a figure, which it copies in the
/// words it says, as every figure does.
fn figure(id: &str, name: &str, said: &str) -> Value {
    let mut row = row(id, name, &json!([{"said": said}]));
    row["acts"] = json!([{"key": "verb:copy", "copy": said}]);
    row
}

/// One of Location's acts, as a row of its own: what it does, `done`, an
/// ask of this daemon or words to copy, keyed by the act.
fn act(id: &str, words: &Words, side: &Value, done: (&str, Value)) -> Value {
    let named = words.act(id);
    let key = format!("verb:location:{id}");
    let (form, what) = done;
    let mut row = row(id, &named.name, side);
    row["thing"] = json!(key);
    row["glyph"] = json!(named.glyph);
    row["hue"] = json!("action");
    row["acts"] = json!([{"key": key, form: what}]);
    row
}

/// The rows inside Location's.
fn inside(here: Option<&Fix>, words: &Words, now: u64, side: &str, how_near: &str) -> Vec<Value> {
    let town = here.map_or_else(
        || side.to_owned(),
        |fix| {
            fix.place
                .clone()
                .unwrap_or_else(|| words.said("unnamed", &[]))
        },
    );
    let mut rows = vec![figure("town", words.name("town"), &town)];
    if let Some(country) = here.and_then(|fix| fix.country.as_deref()) {
        rows.push(figure("country", words.name("country"), country));
    }
    let knows = match here {
        None => words.said("notFound", &[]),
        Some(fix) if fix.hand => words.said("hand", &[]),
        Some(_) => words.said("knows", &[("within", how_near)]),
    };
    rows.push(figure("knows", words.name("knows"), &knows));
    // When it was found is a time the shell words, so it copies only as
    // the shell says it (Now.qml).
    if let Some(fix) = here {
        rows.push(row(
            "located",
            words.name("located"),
            &json!([located(fix, now)]),
        ));
    }
    // Where GeoClue can't find the laptop, or finds it wrong, Joel names
    // its town, the words the shell asks him for handed over with the ask;
    // and asks GeoClue again once he's moved on.
    let ask = |act: &str| ("ask", json!({"ask": "act", "act": act}));
    rows.push(act("set", words, &json!([]), ask("set")));
    if here.is_some_and(|fix| fix.hand) {
        rows.push(act("again", words, &json!([]), ask("again")));
    }
    // An act to copy, last as Copy always is, of what the town alone
    // can't say: where on a map it is.
    if let Some(fix) = here {
        let where_on_a_map = position(fix);
        let side = json!([{"said": where_on_a_map}]);
        rows.push(act("copy", words, &side, ("copy", json!(where_on_a_map))));
    }
    rows
}

#[cfg(test)]
#[path = "rows_tests.rs"]
mod tests;
