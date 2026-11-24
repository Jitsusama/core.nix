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

use crate::sprout::{self, Unanswered};

/// The launcher's ask for them.
pub const ASK: &str = "places";

/// How long the node may take to list them: it reads its folders from
/// disk, well within a second, and the launcher lists its quests' folders
/// alone until it answers.
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
        return unlisted(&why);
    }
    match sprout::asked(node, &json!({"Places": {}}), PATIENCE) {
        Ok(reply) => match (&reply["Ok"]["Places"], &reply["Err"]) {
            (Value::Array(places), _) => json!({"places": places}),
            (_, Value::String(why)) => unlisted(why),
            // Only its start is said, as the launcher journals it and a
            // whole answer names Joel's folders.
            _ => unlisted(&format!(
                "the node's answer couldn't be read: {}",
                reply.to_string().chars().take(80).collect::<String>()
            )),
        },
        Err(Unanswered::Unreached(why)) => {
            unlisted(&format!("the node couldn't be reached: {why}"))
        }
        Err(Unanswered::Silent) => unlisted("the node didn't list its places in time"),
    }
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
    fn node(name: &str, reply: &'static str) -> (std::path::PathBuf, thread::JoinHandle<String>) {
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
            ("refused", r#"{"Err": "no home"}"#, "no home"),
            (
                "unknown",
                r#"{"Ok": {"Other": 1}}"#,
                "the node's answer couldn't be read",
            ),
            ("garbled", "not json", "the node's answer couldn't be read"),
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

    #[test]
    fn an_unread_answer_says_only_its_start() {
        let (socket, asked) = node(
            "long",
            concat!(
                r#"{"Ok": {"Other": ["/home/j/src/one", "/home/j/src/two", "#,
                r#""/home/j/src/three", "/home/j/src/four", "/home/j/src/five"]}}"#,
            ),
        );
        let answer = answer(|| Ok(()), Some(&socket));
        let _ = asked.join();
        let _ = std::fs::remove_file(&socket);
        let said = answer["error"]["message"].as_str().unwrap_or_default();
        assert!(
            said.starts_with("the node's answer couldn't be read: ") && !said.contains("five"),
            "only its first 80 characters: {said}"
        );
    }

    #[test]
    fn places_not_the_shells_or_with_no_node_arent_asked() {
        let gone = std::env::temp_dir().join("agents-places-no-node.sock");
        assert_eq!(
            answer(
                || Err("only the launcher lists places here".to_owned()),
                Some(&gone)
            ),
            json!({"error": {"code": "unlisted",
                "message": "only the launcher lists places here"}}),
            "anyone but the shell is refused before the node is asked"
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
