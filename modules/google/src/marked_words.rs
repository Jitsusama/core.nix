//! What every place's tests say their rows in: the launcher's own
//! vocabulary, as the account's words.json holds it, which the check hands
//! them (package.nix's `words`) rather than a copy kept here; and that
//! vocabulary with every word marked, so a case can find any word a row
//! shows that isn't the vocabulary's.

use serde_json::{Map, Value};

/// The mark each of the vocabulary's words starts with.
pub const MARK: char = 'X';

/// The launcher's vocabulary, as the account's words.json holds it.
pub fn vocabulary() -> Value {
    let path = std::env::var("LAUNCHER_WORDS")
        .expect("LAUNCHER_WORDS names no vocabulary; google-reads-the-calendar hands it one");
    let read = std::fs::read_to_string(&path).expect("the launcher's words didn't read");
    serde_json::from_str(&read).expect("the launcher's words aren't JSON")
}

/// The whole vocabulary with every word in it marked, its ids aside, so
/// each thing is still found by its own.
pub fn marked(value: &Value) -> Value {
    match value {
        Value::String(said) => Value::String(mark(said)),
        Value::Array(each) => Value::Array(each.iter().map(marked).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(field, value)| {
                    let kept = if field == "id" {
                        value.clone()
                    } else {
                        marked(value)
                    };
                    (field.clone(), kept)
                })
                .collect::<Map<_, _>>(),
        ),
        other => other.clone(),
    }
}

/// Each run of letters in a form marked, but a `{name}` it fills.
pub fn mark(form: &str) -> String {
    let mut said = String::new();
    let mut filled = false;
    let mut lettered = false;
    for letter in form.chars() {
        match letter {
            '{' => filled = true,
            '}' => filled = false,
            _ => {}
        }
        if !filled && letter.is_alphabetic() && !lettered {
            said.push(MARK);
        }
        lettered = letter.is_alphabetic();
        said.push(letter);
    }
    said
}

/// The runs of letters in what's shown that aren't the vocabulary's, once
/// what the things shown say themselves, `own`, is taken out where each
/// stands whole, the longest first, so a name within another's goes as
/// part of it and an id never takes letters from a word.
pub fn unmarked(said: &str, own: &[String]) -> Vec<String> {
    let mut own: Vec<&String> = own.iter().filter(|own| !own.is_empty()).collect();
    own.sort_by_key(|own| std::cmp::Reverse(own.len()));
    let left = own
        .iter()
        .fold(said.to_owned(), |left, own| without(&left, own));
    left.split(|letter: char| !letter.is_alphabetic())
        .filter(|run| !run.is_empty() && !run.starts_with(MARK))
        .map(str::to_owned)
        .collect()
}

/// `said` with each whole `own` in it, one no letter or figure runs on
/// from at either end, taken out.
fn without(said: &str, own: &str) -> String {
    let joined = |letter: Option<char>| letter.is_some_and(char::is_alphanumeric);
    let mut left = String::new();
    let mut rest = said;
    while let Some(at) = rest.find(own) {
        let before = rest[..at].chars().next_back();
        let after = rest[at + own.len()..].chars().next();
        let whole = !joined(before) && !joined(after);
        left.push_str(&rest[..at]);
        left.push_str(if whole { " " } else { own });
        rest = &rest[at + own.len()..];
    }
    left.push_str(rest);
    left
}

/// Each shown word that isn't the vocabulary's, by where it shows, for a
/// case to fail on whole.
pub fn written_in(shown: &[(String, String)], own: &[String]) -> Vec<String> {
    shown
        .iter()
        .filter_map(|(at, said)| {
            let unmarked = unmarked(said, own);
            (!unmarked.is_empty()).then(|| format!("{at}: {said:?} says {unmarked:?}"))
        })
        .collect()
}

#[test]
fn a_word_written_in_is_found() {
    let own = vec!["Ada".to_owned(), "Ada Lovelace".to_owned()];
    assert_eq!(
        unmarked("XTelling Ada Lovelace, PIN 4", &own),
        ["PIN"],
        "a word not the vocabulary's is found, but a thing's own isn't"
    );
    assert_eq!(
        unmarked(
            "Xin Design (Xshared Xdrive)",
            &["dr".to_owned(), "Design".to_owned()]
        ),
        Vec::<String>::new(),
        "an id took letters from a word"
    );
    assert_eq!(mark("{n}st"), "{n}Xst");
    assert_eq!(
        mark("on Calendar \u{b7} meeting"),
        "Xon XCalendar \u{b7} Xmeeting"
    );
}
