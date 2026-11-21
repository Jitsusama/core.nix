//! Who is in the Meet call Joel is on, as the google program's daemon
//! follows it: it asks Meet every so often about each call open in a
//! window of its own, and serves each as a `call` record, its people by
//! the names Meet shows them by, with when each first joined and last
//! left, which says who each voice may be (presence.rs). Being in the call says more than being
//! invited to it, so a followed Meet's people stand for their side in
//! place of its invitation's (meeting.rs). The shell has a call followed
//! while this daemon records one, which it serves as `recording`
//! (book.rs), or while the launcher is open, and a little after either
//! (the google module's Following.js); a Meet in a browser's tab isn't
//! followed at all. Where nothing is, the invitation stands.

use std::path::Path;

use serde_json::{Value, json};

use crate::meeting::{Known, asked, meetings, on_tuple, speaker};
use crate::presence::Present;

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

/// Who a call is with now, how that's known, and, for a Meet, when each
/// of them was in it; none for a Tuple call, which doesn't say.
#[derive(Debug, PartialEq)]
pub struct Seen {
    pub people: Vec<String>,
    pub known: Option<Known>,
    pub present: Vec<Present>,
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
) -> Option<Seen> {
    if known == Some(Known::Tuple) {
        return on_tuple(runtime).map(|on| Seen {
            people: on.people,
            known: on.known,
            present: Vec::new(),
        });
    }
    on_meet(runtime, event, me).map(|(people, present)| Seen {
        people,
        known: Some(Known::Meet),
        present,
    })
}

/// Who else is or was in the Meet call Joel, `me`, is on, from the google
/// program's daemon in `runtime`: the call its calendar `event` is joined
/// by, or with none, the only one under way. None when no such call is
/// followed, Meet hasn't said who's in it, the event is joined by
/// something other than Meet or can't be found, or which of them is Joel
/// can't be told. The meetings are asked for only once a call is
/// followed, which none is while nothing is recorded and the launcher is
/// shut.
fn on_meet(runtime: &Path, event: Option<&str>, me: &str) -> Option<(Vec<String>, Vec<Present>)> {
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
    in_meet(&said, &which, me).map(|(people, _)| people)
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
/// else in it yet, which would say less than the invitation. Beside them,
/// when each of them first joined and last left, leaving out one Meet
/// gives no time for.
fn in_meet(said: &Value, which: &Which, me: &str) -> Option<(Vec<String>, Vec<Present>)> {
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
    let (mut joels, mut still_in, mut others, mut present) = (0, false, Vec::new(), Vec::new());
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
            if let Some(joined) = person["joined"].as_u64() {
                let left = person["left"].as_u64();
                let name = name.clone();
                present.push(Present { name, joined, left });
            }
            others.push(name);
        }
    }
    (joels == 1 && still_in && !others.is_empty()).then_some((others, present))
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

    /// Who else is in the call `which` names, by name alone.
    fn who_in(said: &Value, which: &Which, me: &str) -> Option<Vec<String>> {
        in_meet(said, which, me).map(|(people, _)| people)
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
            who_in(&said, &Which::Code("abc-defg-hij".into()), JOEL),
            Some(vec!["Ada Lovelace".to_owned(), "Bo Diddley".to_owned()]),
            "the call its code names wasn't read, or Joel was left in"
        );
        assert_eq!(
            who_in(&said, &Which::Only, JOEL),
            None,
            "one of two calls under way was taken for the one Joel is on"
        );
        let alone = listed(&[call("abc-defg-hij", &["joel  gerber", "Voice 1"])]);
        assert_eq!(
            who_in(&alone, &Which::Only, JOEL),
            Some(vec!["\"Voice 1\"".to_owned()]),
            "Joel's name in another case wasn't his, or a voice's name wasn't quoted"
        );
        let namesake = listed(&[call("abc-defg-hij", &[JOEL, "Joel Smith"])]);
        assert_eq!(
            who_in(&namesake, &Which::Only, JOEL),
            Some(vec!["Joel Smith".to_owned()]),
            "someone of Joel's first name was taken for him"
        );
    }

    #[test]
    fn each_other_in_a_meet_is_there_from_their_join_to_their_leave() {
        let mut said = call("abc-defg-hij", &[JOEL, "Ada Lovelace", "Bo Diddley", "Cy"]);
        said["people"][1]["left"] = json!(AT + 60_000);
        said["people"][2]["joined"] = json!(AT + 5_000);
        said["people"][3]["joined"] = Value::Null;
        let (people, present) = in_meet(&listed(&[said]), &Which::Only, JOEL).unwrap();
        assert_eq!(
            people.len(),
            3,
            "someone with no join time wasn't counted in the Meet"
        );
        let at = AT.cast_unsigned();
        assert_eq!(
            present,
            [
                Present {
                    name: "Ada Lovelace".to_owned(),
                    joined: at,
                    left: Some(at + 60_000)
                },
                Present {
                    name: "Bo Diddley".to_owned(),
                    joined: at + 5_000,
                    left: None
                },
            ],
            "who was in when wasn't read, or one with no join time was taken as there"
        );
    }

    #[test]
    fn a_meet_is_untold_where_joel_cant_be_told_or_meet_hasnt_said() {
        let unknown = |me: &str, people: &[&str]| {
            who_in(&listed(&[call("abc-defg-hij", people)]), &Which::Only, me)
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
            who_in(&listed(&[gone]), &Which::Only, JOEL),
            None,
            "a Meet Joel has left was taken for the call he's on"
        );
        let mut not_yet = call("abc-defg-hij", &[JOEL, "Ada"]);
        not_yet["known"] = json!(false);
        assert_eq!(
            who_in(&listed(&[not_yet]), &Which::Only, JOEL),
            None,
            "a call Meet hasn't said is under way was taken as one"
        );
        let mut ungranted = call("abc-defg-hij", &[JOEL, "Ada"]);
        ungranted["granted"] = json!(false);
        assert_eq!(
            who_in(&listed(&[ungranted]), &Which::Only, JOEL),
            None,
            "a call the sign-in can't read was taken as one"
        );
        assert_eq!(
            who_in(&listed(&[]), &Which::Code("abc-defg-hij".into()), JOEL),
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
            Some(Seen {
                people: vec!["Ada Lovelace".to_owned()],
                known: Some(Known::Meet),
                present: vec![Present {
                    name: "Ada Lovelace".to_owned(),
                    joined: AT.cast_unsigned(),
                    left: None
                }],
            }),
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
