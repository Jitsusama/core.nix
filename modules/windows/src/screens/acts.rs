//! What the launcher's Screens asks done to a screen, done here on niri
//! and answered in the launcher's words, so the shell asks niri nothing
//! for it: a lit screen focused, a screen turned on, by name where it's
//! only dark, and every screen turned off. A row serves its act as
//! `{"ask": "screen", "do": <act>, "output": <connector>}`; `do` rather
//! than `act`, since the shell would say an `act` in the row's becoming
//! line (ReadyRows.qml's asked).

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};

use super::{ASKING, Words};

/// niri wakes the screens at any key, so the key that asked to turn them
/// all off, still held or let go just after, would wake them at once:
/// they're turned off once it has come up, as Displays.qml waited.
pub const KEY_UP: Duration = Duration::from_millis(750);

/// One act on the screens.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    /// The lit screen of this connector takes the focus.
    Focus(String),
    /// The screen of this connector, out of niri's outputs, comes back.
    On(String),
    /// The screen of this connector, dark but still niri's, lights again.
    Wake(String),
    /// Every screen goes dark, each still connected, so a monitor's hub
    /// and network card stay powered.
    Sleep,
}

impl Act {
    /// The act a client asked, or none for one there isn't.
    #[must_use]
    pub fn asked(said: &Value) -> Option<Act> {
        let output = || {
            said["output"]
                .as_str()
                .filter(|output| !output.is_empty())
                .map(str::to_owned)
        };
        match said["do"].as_str()? {
            "focus" => output().map(Act::Focus),
            "on" => output().map(Act::On),
            "wake" => output().map(Act::Wake),
            "sleep" => Some(Act::Sleep),
            _ => None,
        }
    }

    /// The ask a row serves for it, which comes back as `asked` takes it.
    #[must_use]
    pub fn ask(&self) -> Value {
        let (done, output) = match self {
            Act::Focus(output) => ("focus", Some(output)),
            Act::On(output) => ("on", Some(output)),
            Act::Wake(output) => ("wake", Some(output)),
            Act::Sleep => ("sleep", None),
        };
        match output {
            Some(output) => json!({"ask": "screen", "do": done, "output": output}),
            None => json!({"ask": "screen", "do": done}),
        }
    }

    /// The connector it's done to, where it's one screen's.
    #[must_use]
    pub fn output(&self) -> Option<&str> {
        match self {
            Act::Focus(output) | Act::On(output) | Act::Wake(output) => Some(output),
            Act::Sleep => None,
        }
    }

    /// What niri is asked to do it.
    #[must_use]
    pub fn request(&self) -> Value {
        match self {
            Act::Focus(output) => json!({"Action": {"FocusMonitor": {"output": output}}}),
            Act::Wake(output) => json!({"Action": {"PowerOnMonitor": {"output": output}}}),
            Act::On(output) => json!({"Output": {"output": output, "action": "On"}}),
            Act::Sleep => json!({"Action": {"PowerOffMonitors": {}}}),
        }
    }
}

/// niri's answer to an act, or none where it didn't answer in time.
pub fn done(niri: &Path, act: &Act) -> Option<Value> {
    let mut stream = UnixStream::connect(niri).ok()?;
    stream.set_read_timeout(Some(ASKING)).ok()?;
    writeln!(stream, "{}", act.request()).ok()?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).ok()?;
    serde_json::from_str(&line).ok()
}

/// The answer to the ask of `id`: done, or why not, in the launcher's
/// words where it has them for the act. A screen turned on that niri
/// refuses says niri's own words for why, begun in lower case with no
/// full stop closing them, as the foot's words are ("Built-in Screen not
/// turned on · no such output"), and one gone before niri came to it
/// says it's unplugged, both as Displays.qml said them; the rest leave
/// the launcher to say it couldn't. `named` is the screen's row's name.
#[must_use]
pub fn answered(
    id: &Value,
    act: &Act,
    answer: Option<&Value>,
    named: &str,
    words: &Words,
) -> Value {
    let refused = |code: &str, said: Option<String>| {
        let mut error = json!({"code": code, "message": code});
        if let Some(said) = said {
            error["said"] = Value::String(said);
        }
        json!({"id": id, "error": error})
    };
    let Some(answer) = answer else {
        return refused("no-answer", None);
    };
    let turning_on = matches!(act, Act::On(_) | Act::Wake(_));
    if let Some(why) = answer["Err"].as_str() {
        let why = why.trim().trim_end_matches('.');
        let mut letters = why.chars();
        let why: String = letters
            .next()
            .map(|first| first.to_lowercase().chain(letters).collect())
            .unwrap_or_default();
        let said = turning_on.then(|| {
            words
                .not_on
                .replace("{thing}", named)
                .replace("{why}", &why)
        });
        return refused("refused", said);
    }
    if answer["Ok"]["OutputConfigChanged"] == "OutputWasMissing" {
        return refused(
            "unplugged",
            Some(words.is_unplugged.replace("{name}", named)),
        );
    }
    json!({"id": id, "done": true})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screens::tests::words;

    #[test]
    fn each_act_comes_back_as_its_row_serves_it() {
        for act in [
            Act::Focus("eDP-1".to_owned()),
            Act::On("DP-3".to_owned()),
            Act::Wake("DP-3".to_owned()),
            Act::Sleep,
        ] {
            assert_eq!(
                Act::asked(&act.ask()),
                Some(act.clone()),
                "{act:?} came back otherwise"
            );
        }
    }

    #[test]
    fn an_act_on_no_screen_is_none() {
        for said in [
            json!({"ask": "screen", "do": "focus"}),
            json!({"ask": "screen", "do": "on", "output": ""}),
            json!({"ask": "screen", "do": "unplug", "output": "DP-3"}),
            json!({"ask": "screen"}),
        ] {
            assert_eq!(Act::asked(&said), None, "{said} was taken for an act");
        }
    }

    #[test]
    fn niri_is_asked_each_act_in_its_own_words() {
        let asked: Vec<Value> = [
            Act::Focus("eDP-1".to_owned()),
            Act::On("DP-3".to_owned()),
            Act::Wake("DP-3".to_owned()),
            Act::Sleep,
        ]
        .iter()
        .map(Act::request)
        .collect();
        assert_eq!(
            asked,
            [
                json!({"Action": {"FocusMonitor": {"output": "eDP-1"}}}),
                json!({"Output": {"output": "DP-3", "action": "On"}}),
                json!({"Action": {"PowerOnMonitor": {"output": "DP-3"}}}),
                json!({"Action": {"PowerOffMonitors": {}}}),
            ],
            "niri was asked otherwise"
        );
    }

    #[test]
    fn what_niri_did_is_answered_done() {
        let words = words();
        for answer in [
            json!({"Ok": "Handled"}),
            json!({"Ok": {"OutputConfigChanged": "Applied"}}),
        ] {
            assert_eq!(
                answered(
                    &json!(4),
                    &Act::On("DP-3".to_owned()),
                    Some(&answer),
                    "DELL",
                    &words
                ),
                json!({"id": 4, "done": true}),
                "{answer} wasn't answered done"
            );
        }
    }

    #[test]
    fn a_screen_niri_wont_turn_on_says_why_in_niris_words() {
        let answer = json!({"Err": "No such output."});
        let said = answered(
            &json!(5),
            &Act::Wake("DP-3".to_owned()),
            Some(&answer),
            "DELL",
            &words(),
        );
        assert_eq!(
            said["error"]["said"], "DELL not turned on \u{b7} no such output",
            "the refusal was said otherwise: {said}"
        );
    }

    #[test]
    fn a_screen_gone_before_niri_came_to_it_is_unplugged() {
        let answer = json!({"Ok": {"OutputConfigChanged": "OutputWasMissing"}});
        let said = answered(
            &json!(6),
            &Act::On("DP-3".to_owned()),
            Some(&answer),
            "DELL",
            &words(),
        );
        assert_eq!(
            said["error"]["said"], "DELL is unplugged",
            "it was said otherwise: {said}"
        );
    }

    #[test]
    fn a_refusal_the_launcher_has_no_words_for_leaves_them_to_it() {
        let words = words();
        let refused = json!({"Err": "No such output."});
        let focused = answered(
            &json!(7),
            &Act::Focus("DP-3".to_owned()),
            Some(&refused),
            "",
            &words,
        );
        let unanswered = answered(&json!(8), &Act::Sleep, None, "", &words);
        assert_eq!(
            [&focused["error"]["said"], &unanswered["error"]["said"]],
            [&Value::Null, &Value::Null],
            "the daemon gave words of its own: {focused} {unanswered}"
        );
    }
}
