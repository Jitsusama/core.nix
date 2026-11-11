//! A time as a row carries it. Nothing that reads the clock is worded by
//! a daemon, or every row would change each minute: a time travels as
//! `{ "at": <ms since 1970>, "as": <form> }` and the shell words it. One
//! set of forms serves every source, and a form is added only when none
//! says what a source needs (Y2YBTS, 2026-11-10).

use serde_json::{Value, json};

/// How the shell says a time, each named as the ready-row form names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// "just now", "12 min ago", "14:02", "yesterday", as a message's side.
    Came,
    /// words.json's opened, its time said as [`Form::Came`] says it.
    Opened,
    /// "06:06", "yesterday 14:10", "28 Oct 09:30", as a drawer's fact
    /// beside a row whose side already says when.
    Moment,
    /// "Monday 26 October, 14:02", the year when not this one.
    Dated,
    /// "12 min", "15 h", "3 d", as a password's age.
    Ago,
    /// "22 Oct", the year when not this one, "5 Jan 2025", by the
    /// shell's own calendar, as the day an ended quest was last touched.
    Day,
    /// A day alone in a note, the whole phrase the shell's: "today",
    /// "yesterday", "on Thursday" within six days, then "on 28 Oct", as
    /// the Home Network says when its last speed test ran.
    On,
    /// How long it's lasted since, the largest two of days, hours and
    /// minutes, "3 d 2 h", as the machine's uptime or how long a Wi-Fi
    /// card's been connected.
    Lasted,
}

impl Form {
    /// The form's name as the shell reads it.
    pub fn name(self) -> &'static str {
        match self {
            Form::Came => "came",
            Form::Opened => "opened",
            Form::Moment => "moment",
            Form::Dated => "dated",
            Form::Ago => "ago",
            Form::Day => "day",
            Form::On => "on",
            Form::Lasted => "lasted",
        }
    }
}

/// A time to be said in a form, at milliseconds since 1970.
pub fn at(ms: i64, form: Form) -> Value {
    json!({ "at": ms, "as": form.name() })
}

/// A time worded into words of its own, where they hold `{time}`, as a
/// note's "since {time}" is: the time said once, never beside itself.
pub fn said(ms: i64, form: Form, words: &str) -> Value {
    json!({ "at": ms, "as": form.name(), "said": words })
}

/// A meeting's time, said as "now", "in 12 min" or "yesterday 15:00",
/// which needs its end and whether it fills the day as well as its start.
pub fn meeting(ms: i64, until_ms: i64, all_day: bool) -> Value {
    json!({ "at": ms, "as": "meeting", "until": until_ms, "allDay": all_day })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_time_travels_as_its_moment_and_its_form() {
        let forms = [
            Form::Came,
            Form::Opened,
            Form::Moment,
            Form::Dated,
            Form::Ago,
            Form::Day,
            Form::On,
            Form::Lasted,
        ]
        .map(|form| at(1_762_790_400_000, form)["as"].clone());
        assert_eq!(
            forms,
            ["came", "opened", "moment", "dated", "ago", "day", "on", "lasted"]
        );
        assert_eq!(
            said(5, Form::Moment, "since {time}"),
            json!({ "at": 5, "as": "moment", "said": "since {time}" }),
            "a time in words of its own carries them"
        );
        assert_eq!(
            at(5, Form::Came),
            json!({ "at": 5, "as": "came" }),
            "a moment is the milliseconds as given"
        );
        assert_eq!(
            meeting(1, 2, true),
            json!({ "at": 1, "as": "meeting", "until": 2, "allDay": true })
        );
    }
}
