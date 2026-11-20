//! What a call is, from what this machine already knows, with no network:
//! a Tuple call from tuple-call, the daemon following Tuple's, asked on
//! its socket who it's with; or a meeting from the google program's
//! daemon, asked on its socket for the meetings it holds, the one on now,
//! with who's in its Meet where the daemon follows that (meet.rs).
//! Each call's folder is named by when it started and what it was, so the
//! file picker and search find it by its title.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};

/// How early a meeting can be joined and still be the one on.
const EARLY_MS: u64 = 10 * 60 * 1000;
/// How long a daemon is given to answer. Each answers from what it holds,
/// so a second is long; a call starting is written down either way, under
/// a meeting's name or a bare time.
const ANSWERS_WITHIN: Duration = Duration::from_secs(1);

/// What a call was.
#[derive(Debug, PartialEq, Default)]
pub struct Meeting {
    pub title: String,
    /// The calendar event, for a meeting.
    pub event: Option<String>,
    /// Who it's with, by address, for a Tuple call.
    pub person: Option<String>,
    /// The Google account a meeting is on, by address, only when the
    /// google program's daemon said the same one just before and just
    /// after listing its meetings, so the meeting is known to be that
    /// account's; none otherwise, and for a Tuple call.
    pub account: Option<String>,
    /// The others on it, by name, Joel left out, as `known` says they're
    /// known; none when nothing says who they are.
    pub people: Vec<String>,
    pub known: Option<Known>,
}

/// How the others on a call are known: from the Tuple call or the Meet
/// itself, so they're on it, or from the meeting's invitation, so they
/// were asked, which is less.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Known {
    Tuple,
    Meet,
    Invitation,
}

impl Known {
    /// What a call's record says it as.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tuple => "tuple",
            Self::Meet => "meet",
            Self::Invitation => "invitation",
        }
    }

    /// What a call's record said, or none for a record from before it
    /// was kept, or one that says something else.
    pub fn read(said: &str) -> Option<Self> {
        match said {
            "tuple" => Some(Self::Tuple),
            "meet" => Some(Self::Meet),
            "invitation" => Some(Self::Invitation),
            _ => None,
        }
    }
}

/// The call on at `now`, from what's in `runtime`; a Tuple call first,
/// since it has people only while one is on, and a meeting otherwise,
/// its people those in its Meet where that says, Joel, `me`, left out,
/// and its invitation's where it doesn't.
pub fn on(runtime: &Path, now: u64, me: &str) -> Meeting {
    if let Some(call) = on_tuple(runtime) {
        return call;
    }
    let before = signed_in(runtime);
    let listed = meetings(&runtime.join("daemons/google.sock"));
    // Asked again only when it can matter, so a daemon that doesn't answer
    // holds a call's start for two asks rather than three.
    let mut on = meeting(&listed, now)
        .map(|on| Meeting {
            account: before.filter(|before| signed_in(runtime).as_ref() == Some(before)),
            ..on
        })
        .unwrap_or_default();
    if let Some(people) = crate::meet::on_meet_of(runtime, &listed, on.event.as_deref(), me) {
        on.people = people;
        on.known = Some(Known::Meet);
    }
    on
}

/// The Tuple call on, from tuple-call in `runtime`, or none.
pub fn on_tuple(runtime: &Path) -> Option<Meeting> {
    tuple_call(&runtime.join("daemons/tuple-call.sock"))
        .as_ref()
        .and_then(tuple)
}

/// The address of the Google account the meetings are from, as the google
/// program's daemon in `runtime` serves it; or none when it isn't there,
/// doesn't answer in time or can't say whose account it holds.
pub fn signed_in(runtime: &Path) -> Option<String> {
    let said = asked(
        &runtime.join("daemons/google.sock"),
        &json!({"id": 1, "ask": "get", "keys": ["account:google"]}),
    )?;
    said["records"][0]["account"]["address"]
        .as_str()
        .map(str::to_owned)
}

/// tuple-call's record of the call; or none when it isn't there or
/// doesn't answer in time.
fn tuple_call(socket: &Path) -> Option<Value> {
    let said = asked(
        socket,
        &json!({"id": 1, "ask": "get", "keys": ["call:current"]}),
    )?;
    said["records"].get(0).cloned()
}

/// The meetings the google program's daemon holds; or none when it isn't
/// there or doesn't answer in time.
pub fn meetings(socket: &Path) -> Vec<Value> {
    let Some(said) = asked(socket, &json!({"id": 1, "ask": "list", "kind": "meeting"})) else {
        return Vec::new();
    };
    said["records"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|record| record["meeting"].clone())
        .collect()
}

/// A daemon's answer to one ask, by the daemons' interface: one line out
/// and one back; or none when it isn't there or doesn't answer in time.
pub fn asked(socket: &Path, ask: &Value) -> Option<Value> {
    let stream = UnixStream::connect(socket).ok()?;
    stream.set_read_timeout(Some(ANSWERS_WITHIN)).ok()?;
    stream.set_write_timeout(Some(ANSWERS_WITHIN)).ok()?;
    writeln!(&stream, "{ask}").ok()?;
    let mut line = String::new();
    BufReader::new(&stream).read_line(&mut line).ok()?;
    serde_json::from_str(&line).ok()
}

/// A Tuple call, named by who it's with.
fn tuple(call: &Value) -> Option<Meeting> {
    let with = call["people"].as_array()?;
    let first = with.first()?;
    let name = |person: &Value| person["name"].as_str().unwrap_or_default().to_owned();
    let mut title = name(first);
    if with.len() > 1 {
        title = format!("{title} and {} more", with.len() - 1);
    }
    // Everyone on it counts, named or not, so a call with someone Tuple
    // hasn't named yet is never taken for a call with one other person.
    let people: Vec<String> = with
        .iter()
        .map(|person| {
            named(person, "name")
                .or_else(|| named(person, "email"))
                .unwrap_or_else(|| UNNAMED.to_owned())
        })
        .collect();
    Some(Meeting {
        title,
        event: None,
        person: first["email"].as_str().map(str::to_lowercase),
        account: None,
        known: (!people.is_empty()).then_some(Known::Tuple),
        people,
    })
}

/// The meeting on at `now`: of those started (or about to) and not ended,
/// the one started latest, since Joel can be in only one and has moved on
/// to the newer.
fn meeting(meetings: &[Value], now: u64) -> Option<Meeting> {
    meetings
        .iter()
        .filter(|meeting| {
            let start = meeting["start"].as_u64().unwrap_or(u64::MAX);
            let end = meeting["end"].as_u64().unwrap_or(0);
            start <= now + EARLY_MS && now <= end
        })
        .max_by_key(|meeting| meeting["start"].as_u64())
        .map(|meeting| {
            let people = invited(meeting);
            Meeting {
                title: meeting["title"].as_str().unwrap_or_default().to_owned(),
                event: meeting["id"].as_str().map(str::to_owned),
                person: None,
                account: None,
                known: (!people.is_empty()).then_some(Known::Invitation),
                people,
            }
        })
}

/// What someone on a Tuple call it hasn't named yet is listed as.
const UNNAMED: &str = "someone Tuple hasn't named";

/// The domains of Google's calendars that are no one: a shared or team
/// calendar holding a meeting, and a room or a thing booked for it.
const CALENDARS: [&str; 2] = [
    "@group.calendar.google.com",
    "@resource.calendar.google.com",
];

/// Who else a meeting asked, as the google daemon lists it: who holds it,
/// when that isn't Joel, then its guests, who are never Joel or a room,
/// leaving out those who said no and calendars, which are no one.
fn invited(meeting: &Value) -> Vec<String> {
    let guests = meeting["guests"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|guest| guest["answer"] != "no");
    // Someone listed twice, as organizer and guest or under two names, is
    // one person, known by their address, or by their name with none.
    let mut seen = std::collections::HashSet::new();
    std::iter::once(&meeting["organizer"])
        .chain(guests)
        .filter(|someone| {
            let email = someone["email"].as_str().unwrap_or_default().to_lowercase();
            !CALENDARS.iter().any(|calendar| email.ends_with(calendar))
        })
        .filter_map(|someone| {
            let name = named(someone, "name").or_else(|| named(someone, "email"))?;
            let email = someone["email"].as_str().unwrap_or_default().to_lowercase();
            let known_by = if email.is_empty() {
                name.clone()
            } else {
                email
            };
            seen.insert(known_by).then_some(name)
        })
        .collect()
}

/// Who's been on a call, as it's `kept` and joined by those on it `now`:
/// everyone ever on it, in the order they came, since someone who joins
/// late or leaves early still spoke on it.
pub fn joined(mut kept: Vec<String>, now: &[String]) -> Vec<String> {
    for (at, someone) in now.iter().enumerate() {
        // Those Tuple hasn't named can't be told apart, so each one more
        // on it now than were kept is someone new.
        let on = now[..=at].iter().filter(|other| *other == someone).count();
        let had = kept.iter().filter(|other| *other == someone).count();
        if on > had {
            kept.push(someone.clone());
        }
    }
    kept
}

/// Who's been on a call and how that's known, once what was `kept` hears
/// what's said `now`: being on it beats being asked, so the people a Tuple
/// call or a Meet says replace those its invitation asked; the same source
/// again adds whoever joined (`joined`); and nothing said, or only an
/// invitation after being on it, keeps what was. A Tuple call and a Meet
/// are different calls, so neither replaces the other.
pub fn came(
    kept: (Vec<String>, Option<Known>),
    now: (Vec<String>, Option<Known>),
) -> (Vec<String>, Option<Known>) {
    match (kept.1, now.1) {
        (Some(was), Some(is)) if was == is => (joined(kept.0, &now.0), Some(was)),
        (None | Some(Known::Invitation), Some(_)) => now,
        _ => kept,
    }
}

/// Someone's `field`, as a name a transcript can write a speaker under:
/// on one line, with none of the asterisks that would end the speaker's
/// bold early; or none when that leaves nothing.
fn named(someone: &Value, field: &str) -> Option<String> {
    speaker(someone[field].as_str()?)
}

/// `name` as a transcript can write a speaker under, as `named` says, and
/// in quotes when it's what their side is called unnamed, Others or a
/// voice told apart, so someone who picked such a name for an invitation
/// or Tuple is never taken for one, nor one for them.
pub fn speaker(name: &str) -> Option<String> {
    let name = name
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .filter(|c| *c != '*')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if dictation::transcript::unnamed(&name) {
        return Some(format!("\"{name}\""));
    }
    (!name.is_empty()).then_some(name)
}

/// A call's folder's name: its start in local time, then what it was,
/// with what a file name can't hold taken out.
pub fn folder(started: u64, title: &str) -> String {
    let when = local(started);
    let title: String = title
        .chars()
        .map(|c| if c == '/' || c.is_control() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let title: String = title.chars().take(80).collect();
    let title = title.trim_start_matches('.');
    if title.is_empty() {
        when
    } else {
        format!("{when} {title}")
    }
}

/// A time in milliseconds as the machine's local date and time, as
/// "2026-10-31 09.30".
fn local(ms: u64) -> String {
    let seconds = i64::try_from(ms / 1000).unwrap_or(i64::MAX);
    terminal_kit::local::at(seconds).map_or_else(
        || "a call".to_owned(),
        |local| format!("{} {:02}.{:02}", local.date(), local.hour, local.minute),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const NOW: u64 = 1_800_000_000_000;
    const MINUTE: u64 = 60_000;

    #[test]
    fn the_meeting_on_is_the_latest_started_of_those_not_ended() {
        fn held(id: &str, title: &str, start: u64, end: u64) -> Value {
            json!({"id": id, "title": title, "start": start, "end": end})
        }
        let google = [
            held("old", "Done", NOW - 90 * MINUTE, NOW - 30 * MINUTE),
            held("long", "All day", NOW - 60 * MINUTE, NOW + 9 * MINUTE),
            held("e1", "Planning", NOW - 5 * MINUTE, NOW + 25 * MINUTE),
            held("later", "Lunch", NOW + 60 * MINUTE, NOW + 90 * MINUTE),
        ];
        let on = meeting(&google, NOW).unwrap();
        assert_eq!(on.title, "Planning");
        assert_eq!(on.event.as_deref(), Some("e1"));
        let early = meeting(&google, NOW - 14 * MINUTE).unwrap();
        assert_eq!(
            early.event.as_deref(),
            Some("e1"),
            "joined a few minutes early"
        );
        assert_eq!(meeting(&[], NOW), None);
        assert_eq!(
            on.people,
            Vec::<String>::new(),
            "a meeting asking nobody else"
        );
        assert_eq!(on.known, None);
    }

    #[test]
    fn a_meeting_knows_who_else_it_asked_but_not_who_said_no() {
        let planning = json!({"id": "e1", "title": "Planning",
        "start": NOW - 5 * MINUTE, "end": NOW + 25 * MINUTE,
        "organizer": {"name": "Grace *Hopper*", "email": "grace@example.com"},
        "guests": [
            {"name": "Ada\nLovelace", "email": "ada@example.com", "answer": "yes"},
            {"name": "", "email": "bo@example.com", "answer": "waiting"},
            {"name": "Cy", "email": "cy@example.com", "answer": "no"},
        ]});
        let on = meeting(&[planning], NOW).unwrap();
        assert_eq!(
            on.people,
            ["Grace Hopper", "Ada Lovelace", "bo@example.com"]
        );
        assert_eq!(on.known, Some(Known::Invitation));
    }

    #[test]
    fn a_calendar_holding_or_asked_to_a_meeting_is_no_one() {
        let standup = json!({"id": "e2", "title": "Standup",
        "start": NOW - 5 * MINUTE, "end": NOW + 25 * MINUTE,
        "organizer": {"name": "Team", "email": "c0ffee@group.calendar.google.com"},
        "guests": [
            {"name": "Ada", "email": "ada@example.com", "answer": "yes"},
            {"name": "Big Room", "email": "R1@Resource.Calendar.Google.com", "answer": "yes"},
        ]});
        let on = meeting(&[standup], NOW).unwrap();
        assert_eq!(on.people, ["Ada"], "only Ada is someone");
    }

    #[test]
    fn someone_invited_twice_is_one_person() {
        let review = json!({"id": "e3", "title": "Review",
        "start": NOW - 5 * MINUTE, "end": NOW + 25 * MINUTE,
        "organizer": {"name": "Ada", "email": "Ada@Example.com"},
        "guests": [
            {"name": "Ada L.", "email": "ada@example.com", "answer": "yes"},
            {"name": "Bo", "answer": "yes"},
            {"name": "Bo", "answer": "maybe"},
        ]});
        let on = meeting(&[review], NOW).unwrap();
        assert_eq!(on.people, ["Ada", "Bo"], "each is counted once");
    }

    #[test]
    fn someone_tuple_has_not_named_still_counts_as_someone_on_the_call() {
        let call = json!({"key": "call:current", "state": "connected", "since": NOW, "people": [
            {"id": "7", "name": "Ada", "email": "ada@example.com"},
            {"id": "8", "name": "", "email": ""},
            {"id": "9", "name": "", "email": "cy@example.com"},
        ]});
        let on = tuple(&call).unwrap();
        assert_eq!(on.people, ["Ada", UNNAMED, "cy@example.com"]);
    }

    #[test]
    fn everyone_ever_on_a_call_is_kept_in_the_order_they_came() {
        let people = |names: &[&str]| names.iter().map(|&name| name.to_owned()).collect();
        let kept = joined(people(&["Ada"]), &people(&["Ada", UNNAMED]));
        assert_eq!(kept, ["Ada", UNNAMED], "someone joined after Ada");
        let kept = joined(kept, &people(&[UNNAMED, UNNAMED]));
        assert_eq!(
            kept,
            ["Ada", UNNAMED, UNNAMED],
            "a second unnamed joined as Ada left"
        );
        let kept = joined(kept, &[]);
        assert_eq!(kept.len(), 3, "those who left are still kept");
    }

    #[test]
    fn someone_called_what_their_side_is_called_unnamed_is_quoted() {
        assert_eq!(
            speaker("Voice  1").as_deref(),
            Some("\"Voice 1\""),
            "a person called Voice 1 would be taken for a voice told apart"
        );
        assert_eq!(
            speaker("Others").as_deref(),
            Some("\"Others\""),
            "a person called Others would be taken for their side unnamed"
        );
        assert_eq!(
            speaker("Ada").as_deref(),
            Some("Ada"),
            "a name like any other was changed"
        );
    }

    #[test]
    fn a_tuple_call_is_named_by_who_its_with() {
        let call = json!({"key": "call:current", "state": "connected", "since": NOW, "people": [
            {"id": "7", "name": "Ada Lovelace", "email": "Ada@Example.com"},
            {"id": "8", "name": "Grace Hopper", "email": "grace@example.com"},
        ]});
        let on = tuple(&call).unwrap();
        assert_eq!(on.title, "Ada Lovelace and 1 more");
        assert_eq!(on.person.as_deref(), Some("ada@example.com"));
        assert_eq!(on.people, ["Ada Lovelace", "Grace Hopper"]);
        assert_eq!(on.known, Some(Known::Tuple));
        let none = json!({"key": "call:current", "state": "ringing", "since": null, "people": []});
        assert_eq!(tuple(&none), None);
    }

    /// The account address the account and the calls are on.
    const ACCOUNT: &str = "jsama@example.com";

    /// A fresh runtime folder with two stand-in daemons: tuple-call,
    /// answering its one ask with `record`, and the google program's,
    /// holding Planning, on now, on `ACCOUNT`.
    fn runtime(name: &str, record: &Value) -> std::path::PathBuf {
        on_accounts(name, record, &json!(ACCOUNT), &json!(ACCOUNT))
    }

    /// A fresh runtime folder as [`runtime`] makes, but whose google
    /// daemon says the account is `before` until it lists its meetings,
    /// and `after` once it has.
    fn on_accounts(
        name: &str,
        record: &Value,
        before: &Value,
        after: &Value,
    ) -> std::path::PathBuf {
        let runtime = std::env::temp_dir().join(format!("meeting-{name}-{}", std::process::id()));
        std::fs::create_dir_all(runtime.join("daemons")).unwrap();
        let call = json!({"records": [record]});
        answering(
            &runtime.join("daemons/tuple-call.sock"),
            "get",
            "keys",
            json!(["call:current"]),
            call,
        );
        let planning =
            json!({"id": "e1", "title": "Planning", "start": NOW - MINUTE, "end": NOW + MINUTE});
        let meetings = json!({"records": [
            {"key": "meeting:e1", "kind": "meeting", "heard": NOW, "meeting": planning},
        ]});
        let whose = json!(["account:google"]);
        serving(
            &runtime.join("daemons/google.sock"),
            vec![
                ("get", "keys", whose.clone(), account(before)),
                ("list", "kind", json!("meeting"), meetings),
                ("get", "keys", whose, account(after)),
            ],
        );
        runtime
    }

    /// The google daemon's answer to an ask for its account, which it
    /// says is `address`'s.
    fn account(address: &Value) -> Value {
        json!({"records": [
            {"key": "account:google", "kind": "account",
             "account": {"standing": "ready", "address": address}},
        ]})
    }

    /// A stand-in daemon at `socket` answering one `ask`, whose `field` must
    /// be `meant`, with `answer` under the ask's id.
    fn answering(
        socket: &Path,
        ask: &'static str,
        field: &'static str,
        meant: Value,
        answer: Value,
    ) {
        serving(socket, vec![(ask, field, meant, answer)]);
    }

    /// A stand-in daemon at `socket` answering one connection after
    /// another, each the next of `answers`: its ask, whose field must be
    /// what's meant, answered under the ask's id.
    fn serving(socket: &Path, answers: Vec<(&'static str, &'static str, Value, Value)>) {
        let _ = std::fs::remove_file(socket);
        let listener = std::os::unix::net::UnixListener::bind(socket).unwrap();
        std::thread::spawn(move || {
            for (ask, field, meant, answer) in answers {
                let (stream, _) = listener.accept().unwrap();
                let mut line = String::new();
                BufReader::new(&stream).read_line(&mut line).unwrap();
                let asked: Value = serde_json::from_str(&line).unwrap();
                assert_eq!(
                    (asked["ask"].as_str(), &asked[field]),
                    (Some(ask), &meant),
                    "the daemon was asked out of turn"
                );
                let mut answer = answer;
                answer["id"] = asked["id"].clone();
                writeln!(&stream, "{answer}").unwrap();
            }
        });
    }

    #[test]
    fn a_meeting_is_kept_as_its_account_s_only_when_it_was_said_before_and_after_its_list() {
        let none = json!({"state": "none", "since": null, "people": []});
        let on_it = on(&runtime("same", &none), NOW, "Joel");
        assert_eq!(
            (on_it.title.as_str(), on_it.account.as_deref()),
            ("Planning", Some(ACCOUNT)),
            "the account said both times is the meeting's"
        );
        for (name, before, after) in [
            ("switched", json!(ACCOUNT), json!("skiff@example.com")),
            ("unheard-before", Value::Null, json!(ACCOUNT)),
            ("unheard-after", json!(ACCOUNT), Value::Null),
        ] {
            let on_it = on(&on_accounts(name, &none, &before, &after), NOW, "Joel");
            assert_eq!(
                on_it.title, "Planning",
                "{name}: the meeting is still the call"
            );
            assert_eq!(
                on_it.account, None,
                "{name}: an account not said alike was kept"
            );
        }
        let call = json!({"state": "connected", "people": [{"name": "Ada", "email": "a@x"}]});
        assert_eq!(
            on(&runtime("tuple-account", &call), NOW, "Joel").account,
            None,
            "a Tuple call is no Google account's"
        );
    }

    #[test]
    fn tuple_call_is_asked_and_a_tuple_call_comes_before_a_meeting() {
        let call = json!({"state": "connected", "people": [{"name": "Ada", "email": "a@x"}]});
        let on = on(&runtime("tuple", &call), NOW, "Joel");
        assert_eq!(on.title, "Ada");
        assert_eq!(on.event, None);
    }

    #[test]
    fn with_no_tuple_call_on_the_meeting_is_the_call() {
        let none = json!({"state": "none", "since": null, "people": []});
        assert_eq!(on(&runtime("none", &none), NOW, "Joel").title, "Planning");
        let gone = runtime("gone", &none);
        std::fs::remove_file(gone.join("daemons/tuple-call.sock")).unwrap();
        assert_eq!(on(&gone, NOW, "Joel").title, "Planning");
    }

    #[test]
    fn a_followed_meet_says_who_s_on_the_meeting_in_place_of_its_invitation() {
        let runtime = std::env::temp_dir().join(format!("meeting-meet-{}", std::process::id()));
        std::fs::create_dir_all(runtime.join("daemons")).unwrap();
        let _ = std::fs::remove_file(runtime.join("daemons/tuple-call.sock"));
        let planning = json!({"id": "e1", "title": "Planning",
            "start": NOW - MINUTE, "end": NOW + MINUTE,
            "link": "https://meet.google.com/abc-defg-hij",
            "guests": [{"name": "Ada", "email": "ada@example.com", "answer": "yes"},
                       {"name": "Bo", "email": "bo@example.com", "answer": "yes"}]});
        let meetings = json!({"records": [{"key": "meeting:e1", "kind": "meeting",
            "meeting": planning}]});
        let calls = json!({"records": [{"key": "call:abc-defg-hij", "kind": "call",
            "call": {"code": "abc-defg-hij", "granted": true, "known": true,
                     "started": NOW - MINUTE,
                     "people": [{"name": "Joel Gerber", "joined": NOW - MINUTE},
                                {"name": "Ada Lovelace", "joined": NOW - MINUTE}]}}]});
        let whose = json!(["account:google"]);
        serving(
            &runtime.join("daemons/google.sock"),
            vec![
                ("get", "keys", whose.clone(), account(&json!(ACCOUNT))),
                ("list", "kind", json!("meeting"), meetings),
                ("get", "keys", whose, account(&json!(ACCOUNT))),
                ("list", "kind", json!("call"), calls),
            ],
        );
        let on_it = on(&runtime, NOW, "Joel");
        assert_eq!(
            (on_it.people, on_it.known),
            (vec!["Ada Lovelace".to_owned()], Some(Known::Meet)),
            "the Meet's one other person didn't stand for the two invited"
        );
        assert_eq!(
            on_it.event.as_deref(),
            Some("e1"),
            "the meeting is still the call"
        );
    }

    #[test]
    fn with_neither_daemon_there_the_call_is_nothing_known() {
        let none = json!({"state": "none", "since": null, "people": []});
        let bare = runtime("bare", &none);
        std::fs::remove_file(bare.join("daemons/tuple-call.sock")).unwrap();
        std::fs::remove_file(bare.join("daemons/google.sock")).unwrap();
        assert_eq!(on(&bare, NOW, "Joel"), Meeting::default());
    }

    #[test]
    fn the_account_signed_in_is_as_the_google_daemon_serves_it_or_unknown() {
        /// A runtime folder whose google daemon answers the account's ask
        /// with `answer`.
        fn serving(name: &str, answer: Value) -> std::path::PathBuf {
            let runtime =
                std::env::temp_dir().join(format!("account-{name}-{}", std::process::id()));
            std::fs::create_dir_all(runtime.join("daemons")).unwrap();
            answering(
                &runtime.join("daemons/google.sock"),
                "get",
                "keys",
                json!(["account:google"]),
                answer,
            );
            runtime
        }
        assert_eq!(
            signed_in(&serving("held", account(&json!("jsama@example.com")))).as_deref(),
            Some("jsama@example.com"),
            "the address served is the one signed in"
        );
        for (name, answer) in [
            ("unheard", account(&Value::Null)),
            ("no-record", json!({"records": []})),
            ("not-text", account(&json!(7))),
        ] {
            assert_eq!(
                signed_in(&serving(name, answer)),
                None,
                "{name}: an account no one can say is unknown"
            );
        }
        let gone = std::env::temp_dir().join(format!("account-gone-{}", std::process::id()));
        assert_eq!(signed_in(&gone), None, "with no daemon, unknown");
    }

    #[test]
    fn a_folder_is_named_by_its_start_and_a_title_a_file_can_hold() {
        let name = folder(NOW, " Q4 / planning\n");
        assert!(name.ends_with(" Q4 planning"), "{name}");
        assert_eq!(name.len(), "2027-01-15 03.20 Q4 planning".len(), "{name}");
        assert!(!folder(NOW, "..").contains(".."));
        assert_eq!(folder(NOW, "").len(), "2027-01-15 03.20".len());
    }

    #[test]
    fn being_on_a_call_beats_being_asked_to_it() {
        let names = |names: &[&str]| {
            names
                .iter()
                .map(|&name| name.to_owned())
                .collect::<Vec<_>>()
        };
        let invited = (names(&["Ada", "Bo"]), Some(Known::Invitation));
        assert_eq!(
            came(invited.clone(), (names(&["Ada"]), Some(Known::Meet))),
            (names(&["Ada"]), Some(Known::Meet)),
            "the Meet's people didn't replace the invitation's"
        );
        let in_meet = (names(&["Ada"]), Some(Known::Meet));
        assert_eq!(
            came(in_meet.clone(), (names(&["Bo"]), Some(Known::Meet))),
            (names(&["Ada", "Bo"]), Some(Known::Meet)),
            "someone who left the Meet was forgotten, or a late joiner missed"
        );
        assert_eq!(
            came(in_meet.clone(), invited.clone()),
            in_meet,
            "a rejoin's invitation undid who was in the Meet"
        );
        assert_eq!(
            came(in_meet.clone(), (names(&["Grace"]), Some(Known::Tuple))),
            in_meet,
            "a Tuple call was taken for the Meet's"
        );
        assert_eq!(
            came(in_meet.clone(), (Vec::new(), None)),
            in_meet,
            "nothing said"
        );
        assert_eq!(
            came((Vec::new(), None), (names(&["Ada"]), Some(Known::Meet))),
            (names(&["Ada"]), Some(Known::Meet)),
            "a call nothing named didn't take the Meet's people"
        );
    }
}
