//! Who is in the Meet call Joel is on, as the google program's daemon
//! follows it: it asks Meet every so often about each call open in a
//! window of its own, and serves each as a `call` record, its people by
//! the names Meet shows them by. Being in the call says more than being
//! invited to it, so a followed Meet's people stand for their side in
//! place of its invitation's (meeting.rs). A Meet open in a browser's
//! tab isn't followed, and keeps its invitation.

use std::path::Path;

use serde_json::{Value, json};

use crate::meeting::{asked, meetings, speaker};

/// Which followed call is the one Joel is on.
#[derive(Debug, PartialEq, Eq)]
enum Which {
    /// The call of this meeting code, its calendar event's link.
    Code(String),
    /// The only call under way, for a call with no event, or an event
    /// with no link.
    Only,
}

/// Who else is or was in the Meet call Joel, `me`, is on, from the google
/// program's daemon in `runtime`: the call its calendar `event` is joined
/// by, or with none, the only one under way. None when no such call is
/// followed, Meet hasn't said who's in it, the event is joined by
/// something other than Meet, or which of them is Joel can't be told.
pub fn on_meet(runtime: &Path, event: Option<&str>, me: &str) -> Option<Vec<String>> {
    let listed = match event {
        None => Vec::new(),
        Some(_) => meetings(&runtime.join("daemons/google.sock")),
    };
    on_meet_of(runtime, &listed, event, me)
}

/// `on_meet`, with the meetings the google daemon `listed` already in
/// hand, as they are once the call on has been found by them.
pub fn on_meet_of(
    runtime: &Path,
    listed: &[Value],
    event: Option<&str>,
    me: &str,
) -> Option<Vec<String>> {
    let which = match event {
        None => Which::Only,
        Some(event) => which(listed, event)?,
    };
    let socket = runtime.join("daemons/google.sock");
    let said = asked(&socket, &json!({"id": 1, "ask": "list", "kind": "call"}))?;
    in_meet(&said, &which, me)
}

/// Which call a calendar `event` is, of the meetings held: the one its
/// Meet link names; the only one under way when it has no link, or isn't
/// held; or none when it's joined by something other than Meet.
fn which(meetings: &[Value], event: &str) -> Option<Which> {
    let link = meetings
        .iter()
        .find(|meeting| meeting["id"] == event)
        .and_then(|meeting| meeting["link"].as_str());
    match link {
        None => Some(Which::Only),
        Some(link) => code(link).map(Which::Code),
    }
}

/// A Meet link's meeting code, as `https://meet.google.com/abc-defg-hij`
/// names it; or none for a link to anywhere else.
fn code(link: &str) -> Option<String> {
    let rest = link.strip_prefix("https://meet.google.com/")?;
    let code = rest.split(['?', '/', '#']).next()?;
    let made = !code.is_empty() && code.chars().all(|c| c.is_ascii_lowercase() || c == '-');
    made.then(|| code.to_owned())
}

/// Who else is or was in the call `which` names, of the calls a list of
/// the google daemon's `call` records says, leaving out Joel, `me`: the
/// one whose first name is his. Only a call Meet has said is under way
/// counts, so nobody known is never taken for nobody there. Two people
/// whose first name is his, or none, leave it untold, since then a voice
/// of his could be counted as theirs, or theirs as his.
fn in_meet(said: &Value, which: &Which, me: &str) -> Option<Vec<String>> {
    let calls: Vec<&Value> = said["records"]
        .as_array()?
        .iter()
        .map(|record| &record["call"])
        .filter(|call| call["granted"] == true && call["known"] == true)
        .filter(|call| call["started"].is_i64())
        .collect();
    let call = match which {
        Which::Code(code) => *calls.iter().find(|call| call["code"] == code.as_str())?,
        Which::Only => match calls.as_slice() {
            [one] => one,
            _ => return None,
        },
    };
    let names: Vec<String> = call["people"]
        .as_array()?
        .iter()
        .filter_map(|person| speaker(person["name"].as_str()?))
        .collect();
    let joel = |name: &String| {
        name.split_whitespace()
            .next()
            .is_some_and(|first| first.eq_ignore_ascii_case(me))
    };
    if names.iter().filter(|name| joel(name)).count() != 1 {
        return None;
    }
    Some(names.into_iter().filter(|name| !joel(name)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person(name: &str) -> Value {
        json!({"name": name, "joined": 1_800_000_000_000_i64})
    }

    fn call(code: &str, people: &[&str]) -> Value {
        let people: Vec<Value> = people.iter().map(|name| person(name)).collect();
        json!({"code": code, "granted": true, "known": true,
            "started": 1_800_000_000_000_i64, "people": people})
    }

    fn listed(calls: &[Value]) -> Value {
        let records: Vec<Value> = calls
            .iter()
            .map(|call| json!({"key": format!("call:{}", call["code"]), "kind": "call", "call": call}))
            .collect();
        json!({"id": 1, "records": records})
    }

    #[test]
    fn a_meet_link_names_its_code_and_no_other_link_does() {
        assert_eq!(
            code("https://meet.google.com/abc-defg-hij?authuser=1").as_deref(),
            Some("abc-defg-hij"),
            "a Meet link's code wasn't read"
        );
        assert_eq!(
            code("https://zoom.us/j/123"),
            None,
            "a Zoom link was taken for Meet"
        );
        assert_eq!(
            code("https://meet.google.com/ABC\" OR 1"),
            None,
            "a code Meet couldn't make was taken"
        );
        let held = [
            json!({"id": "e1", "link": "https://meet.google.com/abc-defg-hij"}),
            json!({"id": "e2", "link": "https://zoom.us/j/123"}),
            json!({"id": "e3"}),
        ];
        assert_eq!(which(&held, "e1"), Some(Which::Code("abc-defg-hij".into())));
        assert_eq!(
            which(&held, "e2"),
            None,
            "a Zoom meeting was looked for in Meet"
        );
        assert_eq!(
            which(&held, "e3"),
            Some(Which::Only),
            "an event with no link"
        );
        assert_eq!(which(&held, "gone"), Some(Which::Only), "an event not held");
    }

    #[test]
    fn the_people_in_a_meet_are_everyone_but_joel() {
        let said = listed(&[
            call(
                "abc-defg-hij",
                &["Joel Gerber", "Ada Lovelace", "Bo *Diddley*"],
            ),
            call("kkk-llll-mmm", &["Joel Gerber", "Grace Hopper"]),
        ]);
        assert_eq!(
            in_meet(&said, &Which::Code("abc-defg-hij".into()), "Joel"),
            Some(vec!["Ada Lovelace".to_owned(), "Bo Diddley".to_owned()]),
            "the call its code names wasn't read, or Joel was left in"
        );
        assert_eq!(
            in_meet(&said, &Which::Only, "Joel"),
            None,
            "one of two calls under way was taken for the one Joel is on"
        );
        let alone = listed(&[call("abc-defg-hij", &["joel", "Voice 1"])]);
        assert_eq!(
            in_meet(&alone, &Which::Only, "Joel"),
            Some(vec!["\"Voice 1\"".to_owned()]),
            "someone called what a voice is called wasn't quoted"
        );
    }

    #[test]
    fn a_meet_is_untold_where_joel_cant_be_told_or_meet_hasnt_said() {
        let unknown = |which: &str| {
            in_meet(
                &listed(&[call("abc-defg-hij", &["Joel Gerber", "Joel Smith", "Ada"])]),
                &Which::Only,
                which,
            )
        };
        assert_eq!(
            unknown("Joel"),
            None,
            "two Joels, and one was taken for him"
        );
        assert_eq!(
            unknown("Grace"),
            None,
            "nobody was Joel, and everyone was taken as the others"
        );
        let mut not_yet = call("abc-defg-hij", &["Joel Gerber", "Ada"]);
        not_yet["known"] = json!(false);
        assert_eq!(
            in_meet(&listed(&[not_yet]), &Which::Only, "Joel"),
            None,
            "a call Meet hasn't said is under way was taken as one"
        );
        let mut ungranted = call("abc-defg-hij", &["Joel Gerber", "Ada"]);
        ungranted["granted"] = json!(false);
        assert_eq!(
            in_meet(&listed(&[ungranted]), &Which::Only, "Joel"),
            None,
            "a call the sign-in can't read was taken as one"
        );
        assert_eq!(
            in_meet(&listed(&[]), &Which::Code("abc-defg-hij".into()), "Joel"),
            None,
            "a call that isn't followed"
        );
    }
}
