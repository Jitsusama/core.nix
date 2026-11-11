//! What the launcher's Screens asks done to a screen, done here on niri
//! and answered in the launcher's words, so the shell asks niri nothing
//! for it: a lit screen focused, a screen turned on, by name where it's
//! only dark, turned off or unplugged, one of its settings set to a
//! choice, and every screen turned off. A row serves its acts as
//! `{"ask": "screen", "do": <act>, "output": <connector>}`, a choice's
//! with niri's action on the output beside; `do` rather than `act`, since
//! the shell would say an `act` in the row's becoming line
//! (ReadyRows.qml's asked).

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
    /// The screen of this connector goes dark by name, still connected
    /// and laid out, so its hub stays powered.
    Dark(String),
    /// The screen of this connector leaves niri's outputs, as an older
    /// niri, which darkens none by name, turns one off.
    Off(String),
    /// The screen of this connector leaves niri's outputs, its windows
    /// moving to the others: the bigger step beside darkening it.
    Unplug(String),
    /// Every screen goes dark, each still connected, so a monitor's hub
    /// and network card stay powered.
    Sleep,
    /// A setting of a lit screen set as a choice of it says (settings.rs).
    Set(Set),
}

/// A setting's choice as its row asks it: the screen's connector, niri's
/// own action on that output, what the choice is said as, for a refusal's
/// words, and which words say it wasn't done: "set" for a setting's
/// choice, "on" or "off" for a switch turned so.
#[derive(Clone, Debug, PartialEq)]
pub struct Set {
    pub output: String,
    pub action: Value,
    pub said: String,
    pub done: String,
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
            "dark" => output().map(Act::Dark),
            "off" => output().map(Act::Off),
            "unplug" => output().map(Act::Unplug),
            "sleep" => Some(Act::Sleep),
            "set" => Some(Act::Set(Set {
                output: output()?,
                action: Some(&said["action"])
                    .filter(|action| action.is_object())?
                    .clone(),
                said: said["said"].as_str()?.to_owned(),
                done: said["done"].as_str()?.to_owned(),
            })),
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
            Act::Dark(output) => ("dark", Some(output)),
            Act::Off(output) => ("off", Some(output)),
            Act::Unplug(output) => ("unplug", Some(output)),
            Act::Sleep => ("sleep", None),
            Act::Set(set) => {
                return json!({"ask": "screen", "do": "set", "output": set.output,
                    "action": set.action, "said": set.said, "done": set.done});
            }
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
            Act::Focus(output)
            | Act::On(output)
            | Act::Wake(output)
            | Act::Dark(output)
            | Act::Off(output)
            | Act::Unplug(output)
            | Act::Set(Set { output, .. }) => Some(output),
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
            Act::Dark(output) => json!({"Action": {"PowerOffMonitor": {"output": output}}}),
            Act::Off(output) | Act::Unplug(output) => {
                json!({"Output": {"output": output, "action": "Off"}})
            }
            Act::Sleep => json!({"Action": {"PowerOffMonitors": {}}}),
            Act::Set(set) => json!({"Output": {"output": set.output, "action": set.action}}),
        }
    }
}

/// A screen's acts, as its row serves them, each `{key, ask}` with its
/// note where it says one: Return's first, focusing a lit screen or
/// turning any other on, then the ways a lit one turns off. `stands` is
/// how it stands ("lit", "dark" or "off"), `last` whether no other
/// screen is lit, and `each` whether niri darkens one screen by name.
///
/// Darkening comes before unplugging, the bigger step, and the last lit
/// screen is never unplugged, or nothing would show the launcher; its
/// darkening waits for the key that asked to come up, since niri lights
/// every screen at any key once all are off by name, so it's served
/// `waits` and the launcher closes for it. An older niri only unplugs,
/// as its Turn It Off, as Displays.qml offered them.
#[must_use]
pub fn of(name: &str, stands: &str, last: bool, each: bool, words: &Words) -> Vec<Value> {
    let act = |key: &str, act: &Act| json!({"key": key, "ask": act.ask()});
    let name = name.to_owned();
    match stands {
        "off" => return vec![act("verb:display-on", &Act::On(name))],
        "dark" => return vec![act("verb:display-on", &Act::Wake(name))],
        _ => {}
    }
    let mut acts = vec![act("displays:focus", &Act::Focus(name.clone()))];
    if each {
        let mut dark = act("verb:display-off", &Act::Dark(name.clone()));
        if last {
            dark["note"] = Value::String(words.wakes.clone());
            dark["waits"] = Value::Bool(true);
        }
        acts.push(dark);
    }
    if !last {
        acts.push(if each {
            let mut unplug = act("verb:display-unplug", &Act::Unplug(name));
            unplug["note"] = Value::String(words.moves.clone());
            unplug
        } else {
            act("verb:display-off", &Act::Off(name))
        });
    }
    acts
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

/// The journal's line for an act that waited for the key, which was
/// answered done before niri was asked, so the journal is the only place
/// niri not answering or refusing it shows: the line names the act and
/// says what niri said. None where niri did it.
#[must_use]
pub fn unheard(act: &Act, answer: Option<&Value>) -> Option<String> {
    let ask = act.ask();
    let done = ask["do"].as_str().unwrap_or_default();
    let named = act
        .output()
        .map_or_else(|| done.to_owned(), |output| format!("{done} {output}"));
    match answer {
        None => Some(format!("<4>niri didn't answer the screen act {named}")),
        Some(answer) => answer["Err"]
            .as_str()
            .map(|why| format!("<4>niri refused the screen act {named}: {why}")),
    }
}

/// The answer to the ask of `id`: done, or why not, in the launcher's
/// words where it has them for the act. A screen turned on, off or
/// unplugged, or a setting set, that niri refuses says niri's own words
/// for why, its choice named where it's a setting's ("1.5× not set ·
/// invalid custom mode"), begun in
/// lower case with no full stop closing them, as the foot's words are
/// ("Built-in Screen not turned on · no such output"), and one gone
/// before niri came to it says it's unplugged, both as Displays.qml said
/// them; focusing and every screen off leave the launcher to say it
/// couldn't. `named` is the screen's row's name.
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
    let not_done = match act {
        Act::On(_) | Act::Wake(_) => Some(&words.not_on),
        Act::Dark(_) | Act::Off(_) => Some(&words.not_off),
        Act::Unplug(_) => Some(&words.not_unplugged),
        Act::Set(set) => Some(match set.done.as_str() {
            "on" => &words.not_on,
            "off" => &words.not_off,
            _ => &words.not_set,
        }),
        Act::Focus(_) | Act::Sleep => None,
    };
    // A setting's refusal says the choice, as its row does, rather than
    // the screen.
    let thing = match act {
        Act::Set(set) => set.said.as_str(),
        _ => named,
    };
    if let Some(why) = answer["Err"].as_str() {
        let why = why.trim().trim_end_matches('.');
        let mut letters = why.chars();
        let why: String = letters
            .next()
            .map(|first| first.to_lowercase().chain(letters).collect())
            .unwrap_or_default();
        let said = not_done.map(|words| words.replace("{thing}", thing).replace("{why}", &why));
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

    /// Scale's 1.5× on DP-3, as its choice's row asks it.
    fn scaled() -> Act {
        Act::Set(Set {
            output: "DP-3".to_owned(),
            action: json!({"Scale": {"scale": {"Specific": 1.5}}}),
            said: "1.5×".to_owned(),
            done: "set".to_owned(),
        })
    }

    fn every_act() -> [Act; 8] {
        [
            Act::Focus("eDP-1".to_owned()),
            Act::On("DP-3".to_owned()),
            Act::Wake("DP-3".to_owned()),
            Act::Dark("DP-3".to_owned()),
            Act::Off("DP-3".to_owned()),
            Act::Unplug("DP-3".to_owned()),
            Act::Sleep,
            scaled(),
        ]
    }

    #[test]
    fn each_act_comes_back_as_its_row_serves_it() {
        for act in every_act() {
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
            json!({"ask": "screen", "do": "darken", "output": "DP-3"}),
            json!({"ask": "screen", "do": "set", "output": "DP-3", "said": "1×",
                "done": "set"}),
            json!({"ask": "screen"}),
        ] {
            assert_eq!(Act::asked(&said), None, "{said} was taken for an act");
        }
    }

    #[test]
    fn niri_is_asked_each_act_in_its_own_words() {
        let asked: Vec<Value> = every_act().iter().map(Act::request).collect();
        assert_eq!(
            asked,
            [
                json!({"Action": {"FocusMonitor": {"output": "eDP-1"}}}),
                json!({"Output": {"output": "DP-3", "action": "On"}}),
                json!({"Action": {"PowerOnMonitor": {"output": "DP-3"}}}),
                json!({"Action": {"PowerOffMonitor": {"output": "DP-3"}}}),
                json!({"Output": {"output": "DP-3", "action": "Off"}}),
                json!({"Output": {"output": "DP-3", "action": "Off"}}),
                json!({"Action": {"PowerOffMonitors": {}}}),
                json!({"Output": {"output": "DP-3",
                    "action": {"Scale": {"scale": {"Specific": 1.5}}}}}),
            ],
            "niri was asked otherwise"
        );
    }

    /// Each act as `[key, do, note, waits]`, so a case reads as the
    /// screen's step-in would.
    fn offered(name: &str, stands: &str, last: bool, each: bool) -> Vec<Value> {
        of(name, stands, last, each, &words())
            .iter()
            .map(|act| json!([act["key"], act["ask"]["do"], act["note"], act["waits"]]))
            .collect()
    }

    #[test]
    fn beside_another_lit_a_screen_darkens_then_unplugs() {
        assert_eq!(
            offered("DP-2", "lit", false, true),
            [
                json!(["displays:focus", "focus", null, null]),
                json!(["verb:display-off", "dark", null, null]),
                json!([
                    "verb:display-unplug",
                    "unplug",
                    "its windows move to the other screens",
                    null
                ]),
            ],
            "the bigger step isn't after darkening"
        );
    }

    #[test]
    fn the_last_lit_screen_darkens_once_the_key_is_up_and_never_unplugs() {
        assert_eq!(
            offered("eDP-1", "lit", true, true),
            [
                json!(["displays:focus", "focus", null, null]),
                json!(["verb:display-off", "dark", "a key wakes it", true]),
            ],
            "nothing would show the launcher"
        );
    }

    #[test]
    fn a_screen_not_lit_only_turns_on() {
        assert_eq!(
            [
                offered("DP-2", "dark", false, true),
                offered("DP-2", "off", false, true)
            ],
            [
                [json!(["verb:display-on", "wake", null, null])],
                [json!(["verb:display-on", "on", null, null])],
            ],
            "a dark screen is turned on by name, an unplugged one put back"
        );
    }

    #[test]
    fn an_older_niri_only_unplugs_as_turn_it_off_and_keeps_the_last_on() {
        assert_eq!(
            [
                offered("DP-2", "lit", false, false),
                offered("eDP-1", "lit", true, false)
            ],
            [
                vec![
                    json!(["displays:focus", "focus", null, null]),
                    json!(["verb:display-off", "off", null, null]),
                ],
                vec![json!(["displays:focus", "focus", null, null])],
            ],
            "an older niri darkens by name"
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
    fn a_screen_niri_wont_turn_off_or_unplug_says_which() {
        let answer = json!({"Err": "No such output."});
        let said: Vec<Value> = [Act::Dark("DP-3".to_owned()), Act::Unplug("DP-3".to_owned())]
            .iter()
            .map(|act| {
                answered(&json!(5), act, Some(&answer), "DELL", &words())["error"]["said"].clone()
            })
            .collect();
        assert_eq!(
            said,
            [
                "DELL not turned off \u{b7} no such output",
                "DELL not unplugged \u{b7} no such output"
            ],
            "the refusals were said otherwise"
        );
    }

    #[test]
    fn a_choice_niri_wont_set_says_the_choice_and_why() {
        let answer = json!({"Err": "Invalid custom mode."});
        assert_eq!(
            answered(&json!(6), &scaled(), Some(&answer), "DELL", &words())["error"]["said"],
            "1.5× not set · invalid custom mode",
            "a setting's refusal named the screen rather than its choice"
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
    fn a_waiting_act_niri_didnt_do_is_journalled_by_name() {
        let refused = json!({"Err": "No such output."});
        let lines = [
            unheard(&Act::Sleep, None),
            unheard(&Act::Dark("eDP-1".to_owned()), Some(&refused)),
            unheard(&Act::Sleep, Some(&json!({"Ok": "Handled"}))),
        ];
        assert_eq!(
            lines,
            [
                Some("<4>niri didn't answer the screen act sleep".to_owned()),
                Some("<4>niri refused the screen act dark eDP-1: No such output.".to_owned()),
                None
            ],
            "the journal was told otherwise"
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
