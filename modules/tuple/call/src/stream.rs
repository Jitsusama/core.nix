//! What a line of Tuple's stream says, as `tuple --format json state
//! follow --raw` writes them: first one `state` line holding everything
//! Tuple's daemon knows, every contact included, over two megabytes; then
//! a line for each thing that changes, each naming its type first, of
//! which only the call Joel's in, `current_call`, is read. Every other
//! line is passed over by its first few bytes, unparsed.

use std::collections::HashMap;

use serde_json::Value;

/// Someone Tuple knows, by the id it knows them by.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Contact {
    pub name: String,
    pub email: String,
}

/// Someone in a call, Joel or another, and how they're in it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Person {
    /// The id Tuple knows them by, or "" for Joel himself.
    pub id: String,
    pub name: String,
    pub email: String,
    /// Tuple's word for their connection: connected, signaling and the like.
    pub connection: String,
    /// Whether their voice isn't sent.
    pub muted: bool,
    /// Whether it's their screen they share, rather than only their voice.
    pub sharing: bool,
    pub camera: bool,
}

/// The call Joel's in, as Tuple says it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Live {
    /// When it connected, in milliseconds since the epoch, once it has.
    pub since: Option<i64>,
    /// Joel himself.
    pub joel: Person,
    /// Everyone else in it, in Tuple's order.
    pub people: Vec<Person>,
}

/// What one line says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    /// Everything Tuple knows: everyone it knows by their ids, whether
    /// anyone's signed in, and the call, when the line holds one.
    State {
        contacts: HashMap<String, Contact>,
        signed_in: bool,
        call: Option<Option<Live>>,
    },
    /// The call as it is now, or that there's none.
    Call(Option<Live>),
    /// Anything else.
    Other,
}

/// How far into a line its type is looked for: Tuple writes it first.
const HEAD: usize = 48;

/// What a line of Tuple's stream says.
pub fn read(line: &str) -> Line {
    let head = &line.as_bytes()[..line.len().min(HEAD)];
    let named = |kind: &[u8]| head.windows(kind.len()).any(|window| window == kind);
    let state = named(b"\"type\":\"state\"");
    if !state && !named(b"\"type\":\"current_call\"") {
        return Line::Other;
    }
    let Ok(said) = serde_json::from_str::<Value>(line) else {
        return Line::Other;
    };
    let data = said.get("data").unwrap_or(&Value::Null);
    match said.get("type").and_then(Value::as_str) {
        Some("current_call") => Line::Call(live(data)),
        Some("state") => Line::State {
            contacts: contacts(data),
            signed_in: data.get("sign_in").and_then(Value::as_str) == Some("signed_in"),
            call: data.get("current_call").map(live),
        },
        _ => Line::Other,
    }
}

/// An id as Tuple gives one, a number or a string.
fn id(value: Option<&Value>) -> String {
    match value {
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::String(text)) => text.clone(),
        _ => String::new(),
    }
}

fn words<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

/// Someone's name, as fully as Tuple says it.
fn name(value: &Value) -> String {
    ["full_name", "short_name"]
        .into_iter()
        .map(|key| words(value, key))
        .find(|name| !name.is_empty())
        .unwrap_or("")
        .to_owned()
}

fn contacts(data: &Value) -> HashMap<String, Contact> {
    data.get("contacts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|contact| {
            (
                id(contact.get("id")),
                Contact {
                    name: name(contact),
                    email: words(contact, "email").to_owned(),
                },
            )
        })
        .filter(|(id, _)| !id.is_empty())
        .collect()
}

fn person(value: &Value) -> Person {
    Person {
        id: id(value.get("id")),
        name: name(value),
        email: words(value, "email").to_owned(),
        connection: words(value, "connection_state").to_owned(),
        muted: value.get("audio_enabled").and_then(Value::as_bool) != Some(true),
        sharing: words(value, "share_state") == "screen",
        camera: value.get("webcam_enabled").and_then(Value::as_bool) == Some(true),
    }
}

/// The call one of Tuple's `current_call` data says, or none.
fn live(data: &Value) -> Option<Live> {
    if !data.is_object() {
        return None;
    }
    let mut joel = data.get("local").map(person).unwrap_or_default();
    joel.id = String::new();
    Some(Live {
        since: data.get("started_at").and_then(Value::as_i64),
        joel,
        people: data
            .get("participants")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(person)
            .collect(),
    })
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A call's line as Tuple's raw state follow writes one, with Joel's
    /// own state and Ada's, each as (muted, sharing, camera).
    pub fn call(connection: &str, started: bool, own: [bool; 3], other: [bool; 3]) -> String {
        let state = |[muted, sharing, camera]: [bool; 3]| {
            format!(
                "\"audio_enabled\":{},\"share_state\":\"{}\",\"webcam_enabled\":{camera}",
                !muted,
                if sharing { "screen" } else { "audio" }
            )
        };
        format!(
            "{{\"type\":\"current_call\",\"data\":{{\"can_create_url\":true,\"id\":\"c-1\",\
             \"local\":{{{},\"connection_state\":\"{connection}\",\"display\":{{\"id\":0}},\
             \"is_current_user\":true,\"tracks\":{{\"audio\":\"connected\"}}}},\
             \"participants\":[{{{},\"connection_state\":\"{connection}\",\
             \"email\":\"ada@example.com\",\"full_name\":\"Ada Brightwater\",\"id\":7,\
             \"session_id\":\"s-1\"}}],\"sfu_backed\":false{}}}}}",
            state(own),
            state(other),
            if started {
                ",\"started_at\":1792759701457"
            } else {
                ""
            }
        )
    }

    #[test]
    fn a_call_says_how_everyone_in_it_stands() {
        let Line::Call(Some(live)) = read(&call(
            "connected",
            true,
            [false, true, false],
            [true, false, true],
        )) else {
            panic!("no call read");
        };
        assert_eq!(live.since, Some(1_792_759_701_457));
        assert_eq!(
            live.joel,
            Person {
                connection: "connected".into(),
                sharing: true,
                ..Person::default()
            }
        );
        assert_eq!(
            live.people,
            [Person {
                id: "7".into(),
                name: "Ada Brightwater".into(),
                email: "ada@example.com".into(),
                connection: "connected".into(),
                muted: true,
                sharing: false,
                camera: true,
            }]
        );
    }

    #[test]
    fn a_call_still_connecting_has_no_start() {
        let Line::Call(Some(live)) = read(&call("signaling", false, [false; 3], [false; 3])) else {
            panic!("no call read");
        };
        assert_eq!(live.since, None);
        assert_eq!(live.joel.connection, "signaling");
    }

    #[test]
    fn no_call_is_said_as_none() {
        assert_eq!(
            read("{\"type\":\"current_call\",\"data\":null}"),
            Line::Call(None)
        );
    }

    #[test]
    fn the_state_gives_everyone_by_id_and_the_sign_in() {
        let line = "{\"type\":\"state\",\"data\":{\"contacts\":[\
            {\"id\":7,\"full_name\":\"Ada Brightwater\",\"short_name\":\"Ada\",\
            \"email\":\"ada@example.com\"},{\"id\":8,\"full_name\":\"\",\"short_name\":\"Eli\",\
            \"email\":\"eli@example.com\"}],\"sign_in\":\"signed_in\"}}";
        let Line::State {
            contacts,
            signed_in,
            call,
        } = read(line)
        else {
            panic!("no state read");
        };
        assert!(signed_in);
        assert_eq!(call, None);
        assert_eq!(contacts["7"].name, "Ada Brightwater");
        assert_eq!(contacts["8"].name, "Eli");
        assert_eq!(contacts["8"].email, "eli@example.com");
    }

    #[test]
    fn a_state_signed_out_says_so() {
        let Line::State { signed_in, .. } =
            read("{\"type\":\"state\",\"data\":{\"contacts\":[],\"sign_in\":\"signed_out\"}}")
        else {
            panic!("no state read");
        };
        assert!(!signed_in);
    }

    #[test]
    fn other_lines_and_broken_ones_say_nothing() {
        assert_eq!(
            read("{\"type\":\"rooms\",\"data\":{\"current_call\":1}}"),
            Line::Other
        );
        assert_eq!(
            read("{\"type\":\"state\",\"data\":{\"note\":\"current_call\"}}"),
            Line::State {
                contacts: HashMap::new(),
                signed_in: false,
                call: None,
            }
        );
        assert_eq!(read("{\"type\":\"current_call\",\"data\":"), Line::Other);
        assert_eq!(read(""), Line::Other);
    }
}
