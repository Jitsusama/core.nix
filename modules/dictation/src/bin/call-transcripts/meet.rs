//! Who is in the Meet call Joel is on, as the google program's daemon
//! follows it: it asks Meet every so often about each call open in a
//! window of its own, and serves each as a `call` record, its people by
//! the names Meet shows them by. Being in the call says more than being
//! invited to it, so a followed Meet's people stand for their side in
//! place of its invitation's (meeting.rs). The shell has a call followed
//! only while the launcher is open, and a little after, so a Meet is
//! known from when it last was; a Meet in a browser's tab isn't followed
//! at all. Where nothing is, the invitation stands.

use std::path::Path;

use serde_json::{Value, json};

use crate::meeting::{Known, asked, meetings, on_tuple, speaker};

/// Where the google program's daemon answers, in the runtime folder.
const GOOGLE: &str = "daemons/google.sock";

/// Which followed call is the one Joel is on.
#[derive(Debug, PartialEq, Eq)]
enum Which {
    /// The call of this meeting code, its calendar event's link.
    Code(String),
    /// The only call under way, for a call with no event, or an event
    /// with no link.
    Only,
}

/// Who a call being recorded is with now, and how that's known, for a
/// call its record says is `known` so and joined by its calendar `event`:
/// a Tuple call's from Tuple, any other's from its Meet, Joel, `me`, left
/// out. A meeting's invitation isn't asked again, since who it asked
/// doesn't change with who comes, but its Meet is, since the google daemon
/// follows it only while it's said to. None when neither says.
pub fn still_on(
    runtime: &Path,
    known: Option<Known>,
    event: Option<&str>,
    me: &str,
) -> Option<(Vec<String>, Option<Known>)> {
    if known == Some(Known::Tuple) {
        return on_tuple(runtime).map(|on| (on.people, on.known));
    }
    on_meet(runtime, event, me).map(|people| (people, Some(Known::Meet)))
}

/// Who else is or was in the Meet call Joel, `me`, is on, from the google
/// program's daemon in `runtime`: the call its calendar `event` is joined
/// by, or with none, the only one under way. None when no such call is
/// followed, Meet hasn't said who's in it, the event is joined by
/// something other than Meet or can't be found, or which of them is Joel
/// can't be told. The meetings are asked for only once a call is
/// followed, which while the launcher is shut none is.
fn on_meet(runtime: &Path, event: Option<&str>, me: &str) -> Option<Vec<String>> {
    let socket = runtime.join(GOOGLE);
    let said = followed(&socket)?;
    let listed = match event {
        None => Vec::new(),
        Some(_) => meetings(&socket),
    };
    in_meet(&said, &which(&listed, event)?, me)
}

/// `on_meet`, with the meetings the google daemon `listed` already in
/// hand, as they are once the call on has been found by them.
pub fn on_meet_of(
    runtime: &Path,
    listed: &[Value],
    event: Option<&str>,
    me: &str,
) -> Option<Vec<String>> {
    let which = which(listed, event)?;
    let said = followed(&runtime.join(GOOGLE))?;
    in_meet(&said, &which, me)
}

/// The google daemon's list of the calls it follows, at `socket`; or none
/// when it follows none, or doesn't answer in time.
fn followed(socket: &Path) -> Option<Value> {
    let said = asked(socket, &json!({"id": 1, "ask": "list", "kind": "call"}))?;
    said["records"]
        .as_array()
        .is_some_and(|records| !records.is_empty())
        .then_some(said)
}

/// Which call a calendar `event` is, of the meetings held: the one its
/// Meet link names; the only one under way for a call with no event, or
/// an event with no link; or none when it's joined by something other
/// than Meet, or isn't among them, since then its link can't be read.
fn which(meetings: &[Value], event: Option<&str>) -> Option<Which> {
    let Some(event) = event else {
        return Some(Which::Only);
    };
    let meeting = meetings.iter().find(|meeting| meeting["id"] == event)?;
    match meeting["link"].as_str() {
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
/// the google daemon's `call` records says, leaving out Joel, `me`, the
/// one named his whole name. Only a call Meet has said is under way
/// counts, so nobody known is never taken for nobody there, and only one
/// Joel is still in, so a Meet he's left isn't the call he's on. Two
/// people of his name, or none, leave it untold, since then a voice of
/// his could be counted as theirs, or theirs as his; and so does nobody
/// else in it yet, which would say less than the invitation.
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
    let (mut joels, mut still_in, mut others) = (0, false, Vec::new());
    for person in call["people"].as_array()? {
        let Some((said, name)) = person["name"]
            .as_str()
            .and_then(|said| Some((said, speaker(said)?)))
        else {
            continue;
        };
        if is_named(said, me) {
            joels += 1;
            still_in = person["left"].is_null();
        } else {
            others.push(name);
        }
    }
    (joels == 1 && still_in && !others.is_empty()).then_some(others)
}

/// Whether `said` is the name `me`, word for word, in any case.
fn is_named(said: &str, me: &str) -> bool {
    let words = |name: &str| {
        name.split_whitespace()
            .map(str::to_lowercase)
            .collect::<Vec<_>>()
    };
    let me = words(me);
    !me.is_empty() && words(said) == me
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::path::PathBuf;
    use std::sync::mpsc::{self, Receiver};

    use super::*;

    const AT: i64 = 1_800_000_000_000;
    const JOEL: &str = "Joel Gerber";

    fn person(name: &str) -> Value {
        json!({"name": name, "joined": AT})
    }

    fn call(code: &str, people: &[&str]) -> Value {
        let people: Vec<Value> = people.iter().map(|name| person(name)).collect();
        json!({"code": code, "granted": true, "known": true, "started": AT, "people": people})
    }

    fn listed(calls: &[Value]) -> Value {
        let records: Vec<Value> = calls
            .iter()
            .map(|call| {
                json!({"key": format!("call:{}", call["code"]), "kind": "call", "call": call})
            })
            .collect();
        json!({"id": 1, "records": records})
    }

    /// A runtime folder whose google daemon gives these `answers` in turn,
    /// one to each ask, and says the kind each ask was of.
    fn standing_in(name: &str, answers: Vec<Value>) -> (PathBuf, Receiver<String>) {
        let runtime = std::env::temp_dir().join(format!("meet-{name}-{}", std::process::id()));
        std::fs::create_dir_all(runtime.join("daemons")).unwrap();
        let socket = runtime.join(GOOGLE);
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).unwrap();
        let (kinds, asks) = mpsc::channel();
        std::thread::spawn(move || {
            for mut answer in answers {
                let (stream, _) = listener.accept().unwrap();
                let mut line = String::new();
                BufReader::new(&stream).read_line(&mut line).unwrap();
                let asked: Value = serde_json::from_str(&line).unwrap();
                kinds
                    .send(asked["kind"].as_str().unwrap_or_default().to_owned())
                    .unwrap();
                answer["id"] = asked["id"].clone();
                writeln!(&stream, "{answer}").unwrap();
            }
        });
        (runtime, asks)
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
        assert_eq!(
            which(&held, Some("e1")),
            Some(Which::Code("abc-defg-hij".into())),
            "an event's Meet link didn't name its call"
        );
        assert_eq!(
            which(&held, Some("e2")),
            None,
            "a Zoom meeting was looked for in Meet"
        );
        assert_eq!(
            which(&held, Some("e3")),
            Some(Which::Only),
            "an event with no link"
        );
        assert_eq!(
            which(&held, Some("gone")),
            None,
            "an event whose link can't be read was taken for the only Meet"
        );
        assert_eq!(which(&[], None), Some(Which::Only), "a call with no event");
    }

    #[test]
    fn the_people_in_a_meet_are_everyone_but_joel() {
        let said = listed(&[
            call("abc-defg-hij", &[JOEL, "Ada Lovelace", "Bo *Diddley*"]),
            call("kkk-llll-mmm", &[JOEL, "Grace Hopper"]),
        ]);
        assert_eq!(
            in_meet(&said, &Which::Code("abc-defg-hij".into()), JOEL),
            Some(vec!["Ada Lovelace".to_owned(), "Bo Diddley".to_owned()]),
            "the call its code names wasn't read, or Joel was left in"
        );
        assert_eq!(
            in_meet(&said, &Which::Only, JOEL),
            None,
            "one of two calls under way was taken for the one Joel is on"
        );
        let alone = listed(&[call("abc-defg-hij", &["joel  gerber", "Voice 1"])]);
        assert_eq!(
            in_meet(&alone, &Which::Only, JOEL),
            Some(vec!["\"Voice 1\"".to_owned()]),
            "Joel's name in another case wasn't his, or a voice's name wasn't quoted"
        );
        let namesake = listed(&[call("abc-defg-hij", &[JOEL, "Joel Smith"])]);
        assert_eq!(
            in_meet(&namesake, &Which::Only, JOEL),
            Some(vec!["Joel Smith".to_owned()]),
            "someone of Joel's first name was taken for him"
        );
    }

    #[test]
    fn a_meet_is_untold_where_joel_cant_be_told_or_meet_hasnt_said() {
        let unknown = |me: &str, people: &[&str]| {
            in_meet(&listed(&[call("abc-defg-hij", people)]), &Which::Only, me)
        };
        assert_eq!(
            unknown(JOEL, &[JOEL, JOEL, "Ada"]),
            None,
            "two of Joel's name, and one was taken for him"
        );
        assert_eq!(
            unknown("Grace Hopper", &[JOEL, "Ada"]),
            None,
            "nobody was Joel, and everyone was taken as the others"
        );
        assert_eq!(
            unknown("", &[JOEL, "Ada"]),
            None,
            "an account with no name was taken for someone's"
        );
        assert_eq!(
            unknown(JOEL, &[JOEL]),
            None,
            "Joel alone in a Meet put nobody in place of the invitation"
        );
        let mut gone = call("abc-defg-hij", &[JOEL, "Ada"]);
        gone["people"][0]["left"] = json!(AT + 60_000);
        assert_eq!(
            in_meet(&listed(&[gone]), &Which::Only, JOEL),
            None,
            "a Meet Joel has left was taken for the call he's on"
        );
        let mut not_yet = call("abc-defg-hij", &[JOEL, "Ada"]);
        not_yet["known"] = json!(false);
        assert_eq!(
            in_meet(&listed(&[not_yet]), &Which::Only, JOEL),
            None,
            "a call Meet hasn't said is under way was taken as one"
        );
        let mut ungranted = call("abc-defg-hij", &[JOEL, "Ada"]);
        ungranted["granted"] = json!(false);
        assert_eq!(
            in_meet(&listed(&[ungranted]), &Which::Only, JOEL),
            None,
            "a call the sign-in can't read was taken as one"
        );
        assert_eq!(
            in_meet(&listed(&[]), &Which::Code("abc-defg-hij".into()), JOEL),
            None,
            "a call that isn't followed"
        );
    }

    #[test]
    fn looked_at_again_a_call_takes_its_own_meet_and_asks_no_more_than_it_must() {
        let meetings = json!({"records": [
            {"key": "meeting:e1", "kind": "meeting", "meeting":
                {"id": "e1", "link": "https://meet.google.com/abc-defg-hij"}},
            {"key": "meeting:e2", "kind": "meeting", "meeting":
                {"id": "e2", "link": "https://zoom.us/j/123"}}]});
        let calls = listed(&[
            call("kkk-llll-mmm", &[JOEL, "Grace Hopper"]),
            call("abc-defg-hij", &[JOEL, "Ada Lovelace"]),
        ]);
        let (runtime, asks) = standing_in("linked", vec![calls.clone(), meetings.clone()]);
        assert_eq!(
            still_on(&runtime, Some(Known::Invitation), Some("e1"), JOEL),
            Some((vec!["Ada Lovelace".to_owned()], Some(Known::Meet))),
            "an invited call didn't take its own Meet's people"
        );
        assert_eq!(
            asks.try_iter().collect::<Vec<_>>(),
            ["call", "meeting"],
            "the daemon wasn't asked for its calls, then its meetings"
        );
        let (zoom, _) = standing_in("zoom", vec![calls.clone(), meetings]);
        assert_eq!(
            still_on(&zoom, Some(Known::Invitation), Some("e2"), JOEL),
            None,
            "a Zoom meeting took a followed Meet's people"
        );
        let none = json!({"id": 1, "records": []});
        let (shut, asks) = standing_in("shut", vec![none, calls]);
        assert_eq!(
            still_on(&shut, Some(Known::Meet), Some("e1"), JOEL),
            None,
            "a call was said to be on with no Meet followed"
        );
        assert_eq!(
            asks.try_iter().collect::<Vec<_>>(),
            ["call"],
            "the meetings were asked for with no call followed"
        );
    }
}
