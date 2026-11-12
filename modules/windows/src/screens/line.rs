//! A lit screen's settings as the : line's :setl takes them (VKCF3Z,
//! 2026-11-09), as Screens.js's lineSet took them until 11 November
//! 2026: scale=1.5, refresh=59.951 (or 60, where one mode rounds to it),
//! resolution=2560x1440 and rotation=upright, each one of its setting's
//! choices, and Variable Refresh a switch, :setl variablerefresh and
//! novariablerefresh. What each stands at and offers is served on the
//! screen's row, so the line completes and says them as they are, and
//! what's typed is found here, in an ask of the screen's: a value it
//! doesn't offer is refused in words saying what it does, as "1.3× not
//! set · HP offers 1× to 3×", and nothing is asked of niri.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use super::acts::{Act, Set};
use super::settings::{self, Choice, Setting};
use super::{Words, mode_of, named_of, runs, scale};
use crate::interface::refused;

/// What a multiple is typed with after it, "1.5x" or "1.5×".
const MULTIPLE_UNITS: [char; 3] = ['x', 'X', '\u{d7}'];

/// Two multiples closer than this are one.
const SAME_MULTIPLE: f64 = 0.001;

/// The settings the : line takes on the lit screen `output`, by their
/// options: each choice's where it stands, as it's typed, and the values
/// it completes from, each with its choice's words beside; Variable
/// Refresh's whether it's on.
#[must_use]
pub fn of(output: &Value, words: &Words) -> Value {
    let mut line = Map::new();
    for setting in settings::settings(output, words) {
        line.insert(
            settings::option(&words.told.part(setting.id)),
            json!({"kind": "choice", "now": setting.typed, "values": values(&setting.choices)}),
        );
    }
    if output["vrr_supported"] == true && mode_of(output).is_some() {
        line.insert(
            settings::option(&words.told.part("variable")),
            json!({"kind": "switch", "now": output["vrr_enabled"] == true}),
        );
    }
    Value::Object(line)
}

/// A setting's values as the line completes them, two modes of one size
/// and rate said once.
fn values(choices: &[Choice]) -> Vec<Value> {
    let mut said: Vec<&str> = Vec::new();
    choices
        .iter()
        .filter(|choice| {
            let new = !said.contains(&choice.typed.as_str());
            said.push(&choice.typed);
            new
        })
        .map(|choice| json!({"value": choice.typed, "note": runs(&[&choice.name, &choice.note])}))
        .collect()
}

/// The act an ask of a screen's asks: what its row's act said, or the
/// setting the line typed, as `{"do": "line", "output", "option",
/// "typed"}`; or the answer to `id` refusing it.
///
/// # Errors
///
/// The answer to `id` where the ask names no act, the line names no
/// screen or option there is, or what it typed isn't offered.
pub fn asked(
    id: &Value,
    said: &Value,
    outputs: &BTreeMap<String, Value>,
    words: Option<&Words>,
) -> Result<Act, Value> {
    if said["do"] != "line" {
        return Act::asked(said)
            .ok_or_else(|| refused(id, "unknown-act", "the ask names no screen act", ""));
    }
    let Some(words) = words else {
        return Err(refused(id, "no-rows", "no words are read, so no rows", ""));
    };
    let name = said["output"].as_str().unwrap_or_default();
    let Some(output) = outputs
        .get(name)
        .filter(|output| output["logical"].is_object())
    else {
        return Err(refused(
            id,
            "unplugged",
            &format!("niri lists no lit screen {name}"),
            &words.is_unplugged.replace("{name}", name),
        ));
    };
    let option = said["option"].as_str().unwrap_or_default();
    if option == settings::option(&words.told.part("variable")) {
        let on = said["typed"] == true;
        return Ok(Act::Set(Set {
            output: name.to_owned(),
            action: json!({"Vrr": {"vrr": {"vrr": on, "on_demand": false}}}),
            said: words.told.part("variable"),
            done: if on { "on" } else { "off" }.to_owned(),
        }));
    }
    let Some(setting) = settings::settings(output, words)
        .into_iter()
        .find(|setting| settings::option(&words.told.part(setting.id)) == option)
    else {
        return Err(refused(
            id,
            "unknown-option",
            &format!("{name} has no setting {option}"),
            "",
        ));
    };
    let typed = match &said["typed"] {
        Value::String(typed) => typed.clone(),
        other => other.to_string(),
    };
    match found(&setting, &typed) {
        Some(choice) => Ok(Act::Set(Set {
            output: name.to_owned(),
            action: choice.action.clone(),
            said: choice.name.clone(),
            done: "set".to_owned(),
        })),
        None => Err(refused(
            id,
            "not-offered",
            &format!("{name} offers no {option} {typed}"),
            &not_offered(
                &setting,
                option,
                &typed,
                &named_of(outputs, name, words),
                words,
            ),
        )),
    }
}

/// The choice a setting's typed words name: a multiple or a rate with
/// its unit or without, a size by x or ×, a turn by its typed word in any
/// case, and a whole rate the one mode at its size that rounds to it, so
/// refresh=60 sets 59.951 where none runs at 60.
fn found<'a>(setting: &'a Setting, typed: &str) -> Option<&'a Choice> {
    let given = typed.trim();
    let choices = &setting.choices;
    match setting.id {
        "scale" => {
            let multiple: f64 = given
                .trim_end_matches(MULTIPLE_UNITS)
                .trim_end()
                .parse()
                .ok()?;
            choices.iter().find(|choice| {
                choice
                    .typed
                    .parse::<f64>()
                    .is_ok_and(|each| (each - multiple).abs() < SAME_MULTIPLE)
            })
        }
        "refresh" => {
            let rate = unhertzed(given);
            let exact = choices.iter().find(|choice| choice.typed == rate);
            if exact.is_some() || !rate.chars().all(|digit| digit.is_ascii_digit()) {
                return exact;
            }
            let whole: f64 = rate.parse().ok()?;
            let mut near = choices.iter().filter(|choice| {
                choice
                    .typed
                    .parse::<f64>()
                    .is_ok_and(|each| (each.round() - whole).abs() < SAME_MULTIPLE)
            });
            let first = near.next()?;
            near.all(|other| other.typed == first.typed)
                .then_some(first)
        }
        "resolution" => {
            let (width, height) = sized(given)?;
            let typed = format!("{width}x{height}");
            choices.iter().find(|choice| choice.typed == typed)
        }
        _ => choices
            .iter()
            .find(|choice| choice.typed == given.to_lowercase()),
    }
}

/// A rate as typed, its "Hz" left off, in any case.
fn unhertzed(given: &str) -> &str {
    let unit = given.len().saturating_sub(2);
    match given.get(unit..) {
        Some(hz) if hz.eq_ignore_ascii_case("hz") => given[..unit].trim_end(),
        _ => given,
    }
}

/// A size typed by x or ×, as "2560x1440" or "2560 × 1440".
fn sized(given: &str) -> Option<(u32, u32)> {
    let (width, height) = given.split_once(MULTIPLE_UNITS)?;
    Some((width.trim().parse().ok()?, height.trim().parse().ok()?))
}

/// The words refusing what was typed: that it isn't set, or that nothing
/// was, and what the screen, named as its row is, offers instead. A
/// setting of figures says its least to its most, whatever order niri
/// lists its modes in; Rotation, whose eight have no order between, says
/// each.
fn not_offered(setting: &Setting, option: &str, typed: &str, named: &str, words: &Words) -> String {
    let figure = |choice: &&Choice| -> f64 {
        choice
            .typed
            .split('x')
            .map(|side| side.parse::<f64>().unwrap_or(0.0))
            .product()
    };
    let mut offered: Vec<&Choice> = setting.choices.iter().collect();
    let turns = setting.id == "rotation";
    if !turns {
        offered.sort_by(|one, other| figure(one).total_cmp(&figure(other)));
    }
    // A turn is offered as it's typed, its words for what it does beside
    // only as the line completes.
    let said: Vec<&str> = offered
        .iter()
        .map(|choice| if turns { &choice.typed } else { &choice.name }.as_str())
        .collect();
    let offers = match said.as_slice() {
        [first, .., last] if !turns => words
            .offers_range
            .replace("{first}", first)
            .replace("{last}", last),
        _ => said.join(", "),
    };
    let refusal = if typed.trim().is_empty() {
        words.none_given.clone()
    } else {
        words
            .not_offered
            .replace("{value}", &typed_said(setting.id, typed))
    };
    refusal
        .replace("{setting}", option)
        .replace("{screen}", named)
        .replace("{offers}", &offers)
}

/// A value typed as its setting says one, for the words refusing it: 1.3
/// as "1.3×", 75 as "75 Hz", a size as "2560 × 1440".
fn typed_said(id: &str, typed: &str) -> String {
    let given = typed.trim();
    match id {
        "scale" => given
            .trim_end_matches(MULTIPLE_UNITS)
            .trim_end()
            .parse::<f64>()
            .ok()
            .filter(|multiple| multiple.is_finite())
            .map_or_else(|| given.to_owned(), scale),
        "refresh" => format!("{} Hz", unhertzed(given)),
        "resolution" => sized(given).map_or_else(
            || given.to_owned(),
            |(width, height)| format!("{width} \u{d7} {height}"),
        ),
        _ => given.to_owned(),
    }
}

#[cfg(test)]
mod tests;
