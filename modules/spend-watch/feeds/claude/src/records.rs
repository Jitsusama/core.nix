//! The contract's records this feed writes, one JSON line each, their
//! fields in the order the contract's examples give them.

use jiff::{Timestamp, Unit};
use serde::Serialize;

/// The one source this feed speaks for: the plan's usage.
const SOURCE: &str = "plan";

/// The action that signs the plan in, by its id.
pub const SIGN_IN: &str = "sign-in";

/// What the plan stands at in one window, as a cap counting a share of it.
pub struct Cap<'a> {
    pub name: &'a str,
    pub label: String,
    pub window_hours: u32,
    pub used: f64,
    pub resets: Option<Timestamp>,
    pub warn: f64,
    pub at: Timestamp,
}

/// How the plan's source is.
#[derive(Clone, Copy)]
pub enum Health {
    Fine,
    Stalled,
    SignedOut,
    Optional,
    Error,
}

pub fn cap(cap: &Cap) -> String {
    let state = if cap.used >= 1.0 {
        "refused"
    } else if cap.used >= cap.warn {
        "notified"
    } else {
        "ok"
    };
    line(&CapLine {
        v: 1,
        kind: "cap",
        name: cap.name,
        label: &cap.label,
        unit: "share",
        used: cap.used,
        window_hours: cap.window_hours,
        resets: cap.resets.map(instant),
        warn: cap.warn,
        state,
        at: instant(cap.at),
    })
}

pub fn health(health: Health, message: &str) -> String {
    let (state, sign_in) = match health {
        Health::Fine => ("fine", None),
        Health::Stalled => ("stalled", None),
        Health::SignedOut => ("signed-out", Some(SIGN_IN)),
        Health::Optional => ("optional", Some(SIGN_IN)),
        Health::Error => ("error", None),
    };
    line(&HealthLine {
        v: 1,
        kind: "health",
        source: SOURCE,
        state,
        message,
        sign_in,
    })
}

pub fn read(bytes: usize, millis: u64) -> String {
    line(&ReadLine {
        v: 1,
        kind: "read",
        source: SOURCE,
        bytes,
        millis,
    })
}

/// pi-claude's own sign-in, in a terminal, since Claude Code asks there
/// for the code the browser gives, and held open so its last word, the
/// sign-in's status, can be read.
pub fn sign_in() -> String {
    line(&ActionLine {
        v: 1,
        kind: "action",
        id: SIGN_IN,
        name: "Sign In to the Claude Plan",
        run: &["kitty", "--hold", "pi-claude", "login"],
        fits: &["sign-in"],
    })
}

/// An instant as the contract writes it, to the nearest second.
fn instant(at: Timestamp) -> String {
    at.round(Unit::Second).unwrap_or(at).to_string()
}

fn line(record: &impl Serialize) -> String {
    serde_json::to_string(record).expect("a record of strings and figures is always JSON")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CapLine<'a> {
    v: u8,
    kind: &'a str,
    name: &'a str,
    label: &'a str,
    unit: &'a str,
    used: f64,
    window_hours: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    resets: Option<String>,
    warn: f64,
    state: &'a str,
    at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthLine<'a> {
    v: u8,
    kind: &'a str,
    source: &'a str,
    state: &'a str,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    sign_in: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadLine<'a> {
    v: u8,
    kind: &'a str,
    source: &'a str,
    bytes: usize,
    millis: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ActionLine<'a> {
    v: u8,
    kind: &'a str,
    id: &'a str,
    name: &'a str,
    run: &'a [&'a str],
    fits: &'a [&'a str],
}
