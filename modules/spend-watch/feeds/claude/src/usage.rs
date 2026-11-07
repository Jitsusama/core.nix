//! The usage endpoint's answer, of what this feed reads: `five_hour` and
//! `seven_day`, each a `utilization` in percent and a `resets_at` instant.
//! A plan without a window answers it as null or leaves it out, as Ultra
//! does its five hours. The model's own weeks, extra usage and the rest are
//! left alone.

use jiff::Timestamp;
use serde::Deserialize;
use serde_json::Value;

/// How much of each window the plan has used, as the endpoint says it.
#[derive(Debug, Default, PartialEq)]
pub struct Usage {
    pub five_hours: Option<Window>,
    pub week: Option<Window>,
}

/// How much of one window is used, from none to all of it, and when it
/// next resets, when the endpoint says.
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub used: f64,
    pub resets: Option<Timestamp>,
}

/// The usage in an answer's body, or why it doesn't read. A window that
/// doesn't read is left out, as one the plan lacks is, rather than guessed.
pub fn parse(body: &str) -> Result<Usage, String> {
    let answer: Answer =
        serde_json::from_str(body).map_err(|error| format!("its answer doesn't read: {error}"))?;
    Ok(Usage {
        five_hours: answer.five_hour.as_ref().and_then(window),
        week: answer.seven_day.as_ref().and_then(window),
    })
}

/// One window, when it says a share used of none or more, and a reset that
/// reads or none. Past all of it is all of it, since the contract's share
/// stops there.
fn window(said: &Value) -> Option<Window> {
    let percent = said
        .get("utilization")?
        .as_f64()
        .filter(|percent| *percent >= 0.0)?;
    let resets = match said.get("resets_at") {
        None | Some(Value::Null) => None,
        Some(instant) => Some(instant.as_str()?.parse().ok()?),
    };
    Some(Window {
        used: (percent / 100.0).min(1.0),
        resets,
    })
}

#[derive(Deserialize)]
struct Answer {
    five_hour: Option<Value>,
    seven_day: Option<Value>,
}
