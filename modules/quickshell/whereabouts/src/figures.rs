//! Where the laptop is, as Location says it (rows.rs): its position to no
//! more places than it knows, how near it is, when it was found, and its
//! plate's facts (VKCF3Z's ruling, 2026-11-08), as launcher/Here.js said
//! them in the shell before the daemon served Location's rows. Every word
//! comes from the vocabulary (words.rs). A time is sent as `{at, as}` for
//! the shell to say against its own clock, so nothing here changes as the
//! minutes pass; numbers are said as the launcher's English locale says
//! them, a comma between thousands.

use serde_json::{Value, json};

use crate::fix::Fix;
use crate::words::Words;

const KILOMETRE: f64 = 1000.0;
const SECOND_MS: u64 = 1000;
const DAY_MS: u64 = 86_400_000;
/// A degree's decimal places each a tenth of the last, from about 111 km
/// for none: a position claims the nearest order of magnitude to how near
/// it is, switching where its accuracy is a third of the way between
/// them, at about 35 m, 350 m, 3.5 km and 35 km.
const BANDS: [f64; 4] = [35.0, 350.0, 3500.0, 35_000.0];
/// The significant figures a distance or a speed is said to.
const FIGURES: i32 = 3;
/// The widest a fix may be for its zone to move the clock, where its zone
/// holds round its circle and where nothing says, as clock-follows decides
/// (clock-follows/src/main.rs's TRUSTED and NEAR).
const TRUSTED: f64 = 50_000.0;
const NEAR: f64 = 10_000.0;
/// Metres a second in kilometres an hour, as a road says a speed.
const KM_H: f64 = 3.6;

/// How a figure is rounded: up, so it never claims nearer than it knows,
/// and whether its trailing zeros are kept, as a measure keeps them so its
/// width holds as it changes (VKCF3Z, 2026-11-10).
#[derive(Clone, Copy)]
struct How {
    up: bool,
    zeros: bool,
}

/// How near a fix is rounds up and drops its zeros (Y2YBTS's review of
/// #2785).
const WITHIN: How = How {
    up: true,
    zeros: false,
};
/// A speed is a measure, to the nearest, keeping its zeros.
const MEASURED: How = How {
    up: false,
    zeros: true,
};

/// The decimals a position may claim for its accuracy, in metres: four
/// under 35 m, none from 35 km.
pub fn places(accuracy: f64) -> usize {
    BANDS.len() - BANDS.iter().filter(|band| accuracy >= **band).count()
}

/// A number to `decimals` places, a comma between its thousands, as
/// Qt's English locale writes it.
pub fn number(value: f64, decimals: usize) -> String {
    let written = format!("{:.*}", decimals, value.abs());
    let (whole, part) = written
        .split_once('.')
        .map_or((written.as_str(), None), |(whole, part)| {
            (whole, Some(part))
        });
    let mut grouped = String::new();
    for (at, digit) in whole.chars().enumerate() {
        if at > 0 && (whole.len() - at) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    let none = written.bytes().all(|byte| byte == b'0' || byte == b'.');
    let sign = if value < 0.0 && !none { "-" } else { "" };
    match part {
        Some(part) => format!("{sign}{grouped}.{part}"),
        None => format!("{sign}{grouped}"),
    }
}

/// A position as degrees and hemispheres, one figure: "12.3° N · 98.8° W",
/// the hemispheres' letters the vocabulary's. A degree that rounds to none
/// has no hemisphere to say.
pub fn coordinates(fix: &Fix, words: &Words) -> String {
    let decimals = places(fix.accuracy);
    let degrees = |value: f64, positive: &str, negative: &str| {
        let shown = format!("{:.*}", decimals, value.abs());
        let none = shown.bytes().all(|byte| byte == b'0' || byte == b'.');
        let side = if none {
            String::new()
        } else if value < 0.0 {
            format!(" {}", words.said(negative, &[]))
        } else {
            format!(" {}", words.said(positive, &[]))
        };
        format!("{shown}\u{b0}{side}")
    };
    format!(
        "{} \u{b7} {}",
        degrees(fix.latitude, "north", "south"),
        degrees(fix.longitude, "east", "west")
    )
}

/// A figure rounded, and its text.
struct Figured {
    value: f64,
    text: String,
}

/// A figure to three significant figures, rounding up where `how.up`
/// says it mustn't claim less than it is, else to the nearest, and with
/// its trailing zeros where `how.zeros` keeps them: "1.01", "26" or
/// "26.0", "1,240". Past three whole digits the rest round to tens and
/// hundreds, and the rounding is kept whole either side of the point, so
/// a float never says 1,239.9999 for 1,240.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a figure's count of whole digits is a few hundred at most"
)]
fn figured(value: f64, how: How) -> Figured {
    let whole = (value.max(1.0).log10().floor() as i32 + 1).max(1);
    let decimals = FIGURES - whole;
    let round = |scaled: f64| {
        if how.up {
            (scaled - 1e-9).ceil()
        } else {
            (scaled + 0.5).floor()
        }
    };
    let rounded = if decimals >= 0 {
        round(value * 10f64.powi(decimals)) / 10f64.powi(decimals)
    } else {
        round(value / 10f64.powi(-decimals)) * 10f64.powi(-decimals)
    };
    let written = rounded
        .to_string()
        .split_once('.')
        .map_or(0, |(_, part)| part.len());
    let most = usize::try_from(decimals).unwrap_or(0);
    // A zero has no significant digits, so keeps no zeros either: "0 km/h",
    // never a precision nothing gave (VKCF3Z, 2026-11-10).
    let kept = if rounded.abs() < f64::EPSILON {
        0
    } else if how.zeros {
        most
    } else {
        most.min(written)
    };
    Figured {
        value: rounded,
        text: number(rounded, kept),
    }
}

/// How near the laptop is to where it's said to be, to three significant
/// figures, rounded up, in whichever of metres and kilometres keeps it
/// under a thousand: "within 80 m", "within 1.01 km", "within 1,240 km".
pub fn within(metres: f64, words: &Words) -> String {
    let near = figured(metres, WITHIN);
    if near.value < KILOMETRE {
        return words.said("metres", &[("n", &near.text)]);
    }
    let far = figured(metres / KILOMETRE, WITHIN);
    words.said("kilometres", &[("n", &far.text)])
}

/// Why the clock isn't following a place in another zone, the last item
/// of the row's note, which gives way first: near a time zone's edge where
/// its circle keeps another time, the fix's own width where it's too wide,
/// and nothing where the clock would follow it (VKCF3Z, 2026-11-09).
pub fn unfollowed(fix: &Fix, words: &Words) -> String {
    if fix.zone_holds == Some(false) {
        return words.said("nearEdge", &[]);
    }
    let widest = if fix.zone_holds == Some(true) {
        TRUSTED
    } else {
        NEAR
    };
    if !fix.accuracy.is_finite() || fix.accuracy <= widest {
        return String::new();
    }
    words.said("foundWithin", &[("within", &within(fix.accuracy, words))])
}

/// When it was found, as a time: how long ago within a day, else its day
/// and time, as the launcher says a moment ("Saturday 06:46", "28 Oct
/// 09:30"). The form turns once a day has passed (`turns`).
pub fn located(fix: &Fix, now_ms: u64) -> Value {
    let at = fix.when.saturating_mul(SECOND_MS);
    let form = if now_ms.saturating_sub(at) < DAY_MS {
        "ago"
    } else {
        "moment"
    };
    json!({"at": at, "as": form})
}

/// When `located` says this fix another way, or none once it has.
pub fn turns(fix: &Fix, now_ms: u64) -> Option<u64> {
    let turned = fix.when.saturating_mul(SECOND_MS).saturating_add(DAY_MS);
    (now_ms < turned).then_some(turned)
}

/// What found it, as a plate's eyebrow says where then what, as WARP's
/// "via WARP · tunnel" does (VKCF3Z, 2026-11-09): "via Wi-Fi · location",
/// "found · location" where GeoClue named no source the words know, and
/// "set by hand · location" for Joel's own.
pub fn eyebrow(fix: &Fix, words: &Words) -> String {
    if fix.hand {
        return words.said("viaHand", &[]);
    }
    fix.source
        .as_deref()
        .map(|source| format!("via-{source}"))
        .filter(|id| words.has(id))
        .map_or_else(|| words.said("via", &[]), |id| words.said(&id, &[]))
}

/// The figures only some sources give, as GPS does, and only while held:
/// altitude to the metre and heading to the degree east of north, in
/// GeoClue's own units, and speed in kilometres an hour, to three
/// significant figures, so 4.26 m/s reads "15.3 km/h" (VKCF3Z,
/// 2026-11-10). Each is empty where it isn't held.
fn measures(fix: &Fix, words: &Words) -> [(&'static str, String); 3] {
    let said = |id: &str, value: Option<f64>, figure: &dyn Fn(f64) -> String| {
        value
            .filter(|value| value.is_finite())
            .map_or_else(String::new, |value| {
                words.said(id, &[("n", &figure(value))])
            })
    };
    [
        (
            "altitude",
            said("altitude", fix.altitude, &|value| number(value, 0)),
        ),
        (
            "speed",
            said("speed", fix.speed, &|value| {
                figured(value * KM_H, MEASURED).text
            }),
        ),
        (
            "heading",
            said("heading", fix.heading, &|value| number(value, 0)),
        ),
    ]
}

/// What its source couldn't give of the three, in one sentence the plate's
/// body says, naming the source as its eyebrow does: "Wi-Fi gives no
/// altitude, speed or heading." Nothing when all three are held, nor for a
/// place set by hand, which no source found (VKCF3Z, 2026-11-09).
pub fn left_out(fix: &Fix, words: &Words) -> String {
    if fix.hand {
        return String::new();
    }
    let mut missing: Vec<String> = [
        ("altitude", fix.altitude),
        ("speed", fix.speed),
        ("heading", fix.heading),
    ]
    .iter()
    .filter(|(_, value)| !value.is_some_and(f64::is_finite))
    .map(|(id, _)| words.said(&format!("no-{id}"), &[]))
    .collect();
    let Some(last) = missing.pop() else {
        return String::new();
    };
    let listed = if missing.is_empty() {
        last
    } else {
        format!("{} {} {last}", missing.join(", "), words.said("or", &[]))
    };
    let blanks = [("missing", listed.as_str())];
    fix.source
        .as_deref()
        .and_then(|source| words.tells(&format!("leftOut-{source}"), &blanks))
        .or_else(|| words.tells("leftOut", &blanks))
        .unwrap_or_default()
}

/// Its plate's facts, from what moves most: when it was found, how near,
/// where on a map, and how high, fast and which way where a source gives
/// them, its country, then its zone, the identifier it is, last. The town
/// is the row's side, so the plate never says it again; how near goes
/// while the row's note says it, since a drawer beside its row never says
/// a thing twice (VKCF3Z, 2026-11-09).
pub fn facts(fix: &Fix, now_ms: u64, words: &Words, note_says_how_near: bool) -> Vec<Value> {
    let fact = |id: &str, said: String| json!({"id": id, "name": words.name(id), "said": said, "identifies": false});
    let mut located = located(fix, now_ms);
    located["id"] = json!("located");
    located["name"] = json!(words.name("located"));
    let mut facts = vec![located];
    if !fix.hand && !note_says_how_near {
        facts.push(fact("accuracy", within(fix.accuracy, words)));
    }
    facts.push(fact("coordinates", coordinates(fix, words)));
    for (id, said) in measures(fix, words) {
        if !said.is_empty() {
            facts.push(fact(id, said));
        }
    }
    if let Some(country) = fix.country.as_deref().filter(|country| !country.is_empty()) {
        facts.push(fact("country", country.to_owned()));
    }
    if let Some(zone) = fix.zone.as_deref().filter(|zone| !zone.is_empty()) {
        let mut zone = fact("zone", zone.to_owned());
        zone["identifies"] = json!(true);
        facts.push(zone);
    }
    facts
}

#[cfg(test)]
#[path = "figures_tests.rs"]
mod tests;
