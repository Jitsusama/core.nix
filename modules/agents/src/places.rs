//! Where Start New Work can begin, as the launcher asks for it
//! (launcher/Wheres.js): the places the realm's node says work can begin
//! in, a big repository's zones by name among them, asked as the node's `Places`
//! (mycelium's PROTOCOL.md) on the line `mycelium places` sends, so the
//! launcher's In rows find a zone such as //system/pond by name (Joel,
//! 24 November: "the list of things to start in was shit"). Only the
//! shell may ask (asker.rs), since the node lists them as its view of
//! Joel's folders reaches, and this daemon is taken for him.

use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};

use crate::asker;
use crate::sprout::{self, Unanswered};

/// The launcher's ask for them.
pub const ASK: &str = "places";

/// What a refusal of anyone but the shell says: the asker's own words
/// say only the launcher starts new work, which isn't this ask's.
pub const REFUSED: &str = "only the launcher lists places here";

/// The most places passed on, as the launcher lists no more
/// (Wheres.js's MOST_PLACES), so a node naming more sends no more.
const MOST_PLACES: usize = 2000;

/// How long the node may take to list them: it reads its folders from
/// disk, well within a second, and the launcher keeps the list it last
/// had, or its quests' folders alone, until it answers.
const PATIENCE: Duration = Duration::from_secs(5);

/// The places the node at `node` lists, `{"places": [...]}` in its own
/// fields, or an error saying why there are none: `asker` says whether
/// the shell asked, told only where there's a node to ask, since telling
/// reads who asked from the kernel and its service manager, and `node` is
/// the realm's, none where this machine runs no node. Each place is read
/// again where it's drawn (Wheres.js).
pub fn answer(asker: impl FnOnce() -> Result<(), String>, node: Option<&Path>) -> Value {
    let Some(node) = node else {
        return unlisted("this machine runs no node to list places on");
    };
    if let Err(why) = asker() {
        // Another's ask is refused in this ask's own words; where who
        // asked couldn't be told, why is said as it is.
        return unlisted(if why == asker::REFUSED { REFUSED } else { &why });
    }
    match sprout::asked(node, &json!({"Places": {}}), PATIENCE) {
        Ok(reply) => match (
            &reply["Ok"]["Places"],
            &reply["Err"],
            sprout::refusal_of(&reply),
        ) {
            (Value::Array(places), _, _) => {
                json!({"places": places.iter().take(MOST_PLACES).collect::<Vec<_>>()})
            }
            // Two arms, as an or-pattern can't bind the error's &String and
            // the refusal's &str as one.
            (_, Value::String(why), _) => unlisted(&start_of(why)),
            (_, _, Some((_, why))) => unlisted(&start_of(why)),
            // What sprout::asked reads of a line that isn't JSON.
            _ if reply.is_null() => unlisted("the node's answer couldn't be read: not JSON"),
            _ => unlisted(&format!(
                "the node's answer couldn't be read: {}",
                start_of(&reply.to_string())
            )),
        },
        Err(Unanswered::Unreached(why)) => {
            unlisted(&format!("the node couldn't be reached: {why}"))
        }
        Err(Unanswered::Silent) => unlisted("the node didn't list its places in time"),
    }
}

/// The most of the node's words said: the launcher journals them, and a
/// whole answer may name Joel's folders.
const MOST_SAID: usize = 80;

fn start_of(said: &str) -> String {
    said.chars().take(MOST_SAID).collect()
}

fn unlisted(why: &str) -> Value {
    json!({"error": {"code": "unlisted", "message": why}})
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::thread;

    use super::*;

    /// A node at a socket of its own, answering one line with `reply`,
    /// and what it was asked.
    fn node(name: &str, reply: &str) -> (std::path::PathBuf, thread::JoinHandle<String>) {
        let reply = reply.to_owned();
        let socket =
            std::env::temp_dir().join(format!("agents-places-{name}-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("a socket to stand in for the node");
        let asked = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("the places are asked of the node");
            let mut answering = stream.try_clone().expect("a stream to answer on");
            let mut line = String::new();
            BufReader::new(stream).read_line(&mut line).expect("a line");
            writeln!(answering, "{reply}").expect("the places answered");
            line
        });
        (socket, asked)
    }

    #[test]
    fn the_places_are_the_nodes_own() {
        let (socket, asked) = node(
            "listed",
            concat!(
                r#"{"Ok": {"Places": [{"folder": "/home/j/big/src/areas/system/pond", "#,
                r#""name": "//system/pond", "kind": "zone", "out": false}]}}"#,
            ),
        );
        let answer = answer(|| Ok(()), Some(&socket));
        assert_eq!(
            answer,
            json!({"places": [{"folder": "/home/j/big/src/areas/system/pond",
                "name": "//system/pond", "kind": "zone", "out": false}]}),
            "the node's places, as it says them"
        );
        let line = asked.join().expect("the node stood in");
        let _ = std::fs::remove_file(&socket);
        assert_eq!(
            serde_json::from_str::<Value>(&line).expect("one line of JSON"),
            json!({"Places": {}}),
            "one Places line, with no hello and no words"
        );
    }

    #[test]
    fn a_node_that_lists_none_says_why() {
        for (name, reply, why) in [
            ("erred", r#"{"Err": "no home"}"#, "no home"),
            (
                "refused",
                r#"{"Refused": {"code": "unheard", "why": "not yours"}}"#,
                "not yours",
            ),
            (
                "refused-ok",
                r#"{"Ok": {"Refused": {"code": "unheard", "why": "not yours"}}}"#,
                "not yours",
            ),
            (
                "unknown",
                r#"{"Ok": {"Other": 1}}"#,
                "the node's answer couldn't be read",
            ),
            (
                "garbled",
                "not json",
                "the node's answer couldn't be read: not JSON",
            ),
        ] {
            let (socket, asked) = node(name, reply);
            let answer = answer(|| Ok(()), Some(&socket));
            assert_eq!(
                answer["error"]["code"],
                json!("unlisted"),
                "{name}: {answer}"
            );
            assert!(
                answer["error"]["message"]
                    .as_str()
                    .is_some_and(|said| said.starts_with(why)),
                "{name}: {answer}"
            );
            let _ = asked.join();
            let _ = std::fs::remove_file(&socket);
        }
    }

    /// What the launcher is told when the node answers `reply`.
    fn told(name: &str, reply: &str) -> String {
        let (socket, asked) = node(name, reply);
        let said = answer(|| Ok(()), Some(&socket));
        let _ = asked.join();
        let _ = std::fs::remove_file(&socket);
        said["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    }

    #[test]
    fn an_unread_answer_says_only_its_start() {
        let said = told(
            "long",
            concat!(
                r#"{"Ok": {"Other": ["/home/j/src/one", "/home/j/src/two", "#,
                r#""/home/j/src/three", "/home/j/src/four", "/home/j/src/five"]}}"#,
            ),
        );
        let unread = "the node's answer couldn't be read: ";
        assert_eq!(
            said.strip_prefix(unread).map(|cut| cut.chars().count()),
            Some(80),
            "only its first 80 characters: {said}"
        );
    }

    #[test]
    fn the_nodes_words_are_cut_at_80_characters() {
        // Of a character two bytes long, so a cut by bytes, or one before
        // or past the most, shows.
        for (shape, length, kept) in [("Err", 80, 80), ("Err", 81, 80), ("Refused", 81, 80)] {
            let words = "\u{e9}".repeat(length);
            let reply = if shape == "Err" {
                json!({"Err": words})
            } else {
                json!({"Refused": {"why": words}})
            };
            let name = format!("{shape}-{length}");
            let said = told(&name, &reply.to_string());
            assert_eq!(said, "\u{e9}".repeat(kept), "{name}");
        }
    }

    #[test]
    fn places_not_the_shells_or_with_no_node_arent_asked() {
        let gone = std::env::temp_dir().join("agents-places-no-node.sock");
        assert_eq!(
            answer(|| Err(asker::REFUSED.to_owned()), Some(&gone)),
            json!({"error": {"code": "unlisted", "message": REFUSED}}),
            "anyone but the shell is refused before the node is asked"
        );
        assert_eq!(
            answer(
                || Err("the kernel names no peer by pidfd".to_owned()),
                Some(&gone)
            )["error"]["message"],
            json!("the kernel names no peer by pidfd"),
            "who asked that couldn't be told is said as it is"
        );
        assert!(
            answer(|| Ok(()), Some(&gone))["error"]["message"]
                .as_str()
                .is_some_and(|why| why.starts_with("the node couldn't be reached: ")),
            "a node that isn't there wasn't reached"
        );
        assert_eq!(
            answer(
                || panic!("who asked is told only where there's a node to ask"),
                None
            )["error"]["message"],
            json!("this machine runs no node to list places on")
        );
    }
}
