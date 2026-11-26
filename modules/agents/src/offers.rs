//! Whether Start New Work can start in the cloud, as the launcher asks
//! for it (launcher/Cloud.js): what the realm's node's providers could
//! make, asked as the node's `Offers` (mycelium's PROTOCOL.md) on the line
//! `mycelium offers` sends, so the launcher offers In the Cloud only where
//! a machine can be had (Joel, 2026-11-25: "we need full integation ...
//! i should not have to think to use this"). Only the shell may ask
//! (asker.rs), as for the places, since this daemon is taken for him.

use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};

use crate::asker;
use crate::sprout::{self, Unanswered};

/// The launcher's ask for them.
pub const ASK: &str = "offers";

/// What a refusal of anyone but the shell says.
pub const REFUSED: &str = "only the launcher asks what the cloud offers here";

/// How long the node may take: it asks each provider's program at once,
/// and each ends within 30 seconds (mycelium's offer.rs).
const PATIENCE: Duration = Duration::from_secs(35);

/// The most offers passed on, so a provider listing many sends no more
/// than a row could say.
const MOST_OFFERS: usize = 20;

/// The most characters of a shape's name or a cost passed on.
const MOST_SAID: usize = 80;

/// What the node at `node` says its providers could make,
/// `{"offers": [{"shape", "hourly", "currency"}]}`, a shape's cost left
/// out where its provider gave none, or an error saying why none is known:
/// `asker` says whether the shell asked, told only where there's a node to
/// ask, and `node` is the realm's, none where this machine runs no node.
/// A provider that couldn't say is journalled by the node, not here: an
/// offer from another is still one. A node that doesn't say it takes the
/// cloud as a sprout's need offers none, whatever its providers could
/// make: one from before it would drop the need and start the work here.
pub fn answer(asker: impl FnOnce() -> Result<(), String>, node: Option<&Path>) -> Value {
    let Some(node) = node else {
        return unoffered("this machine runs no node to ask what the cloud offers");
    };
    if let Err(why) = asker() {
        return unoffered(if why == asker::REFUSED { REFUSED } else { &why });
    }
    match sprout::asked(node, &json!("Offers"), PATIENCE) {
        Ok(reply) => match (&reply["Ok"]["Offers"]["offers"], &reply["Err"]) {
            (Value::Array(_), _) if !takes_the_cloud(&reply["Ok"]["Offers"]["needs"]) => {
                unoffered(UNNEEDED)
            }
            (Value::Array(offers), _) => json!({"offers": offers
                .iter()
                .filter_map(offered)
                .take(MOST_OFFERS)
                .collect::<Vec<_>>()}),
            (_, Value::String(why)) => unoffered(&start_of(why)),
            _ => unoffered(&format!(
                "the node's answer couldn't be read: {}",
                start_of(&reply.to_string())
            )),
        },
        Err(Unanswered::Unreached(why)) => {
            unoffered(&format!("the node couldn't be reached: {why}"))
        }
        Err(Unanswered::Silent) => unoffered("the node didn't say what the cloud offers in time"),
    }
}

/// What a node that offers machines but takes no cloud need says, as one
/// from before mycelium's sprouts took needs does.
const UNNEEDED: &str = "this machine's node can't start work in the cloud yet";

/// Whether the node's `needs`, beside its offers, says a sprout may need
/// the cloud.
fn takes_the_cloud(needs: &Value) -> bool {
    needs
        .as_array()
        .is_some_and(|needs| needs.iter().any(|need| need.as_str() == Some("cloud")))
}

/// One offer as the launcher is told it: its shape, and its hourly cost
/// and currency where its provider gave both as words; none where it
/// names no shape.
fn offered(offer: &Value) -> Option<Value> {
    let shape = offer["shape"].as_str().filter(|shape| !shape.is_empty())?;
    let mut told = json!({"shape": start_of(shape)});
    if let (Some(hourly), Some(currency)) = (
        offer["costs"]["hourly"].as_str(),
        offer["costs"]["currency"].as_str(),
    ) {
        told["hourly"] = json!(start_of(hourly));
        told["currency"] = json!(start_of(currency));
    }
    Some(told)
}

fn start_of(said: &str) -> String {
    said.chars().take(MOST_SAID).collect()
}

fn unoffered(why: &str) -> Value {
    json!({"error": {"code": "unoffered", "message": why}})
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
            std::env::temp_dir().join(format!("agents-offers-{name}-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("a socket to stand in for the node");
        let asked = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("the offers are asked of the node");
            let mut answering = stream.try_clone().expect("a stream to answer on");
            let mut line = String::new();
            BufReader::new(stream).read_line(&mut line).expect("a line");
            writeln!(answering, "{reply}").expect("the offers answered");
            line
        });
        (socket, asked)
    }

    fn told(name: &str, reply: &str) -> (Value, String) {
        let (socket, asked) = node(name, reply);
        let said = answer(|| Ok(()), Some(&socket));
        let line = asked.join().expect("the node stood in");
        let _ = std::fs::remove_file(&socket);
        (said, line)
    }

    #[test]
    fn the_offers_are_the_providers_shapes_and_costs() {
        let (said, line) = told(
            "offered",
            concat!(
                r#"{"Ok": {"Offers": {"offers": [{"shape": "standard", "provider": "/p", "#,
                r#""facts": {"cores": 8}, "costs": {"hourly": "0.38", "currency": "USD"}}, "#,
                r#"{"shape": "small"}, {"costs": {"hourly": "1"}}], "unanswered": {}, "#,
                r#""needs": ["cloud"]}}}"#,
            ),
        );
        assert_eq!(
            said,
            json!({"offers": [{"shape": "standard", "hourly": "0.38", "currency": "USD"},
                {"shape": "small"}]}),
            "each shape, its cost where given, none naming no shape"
        );
        assert_eq!(
            serde_json::from_str::<Value>(&line).expect("one line of JSON"),
            json!("Offers"),
            "one Offers line, as the node's unit request"
        );
    }

    #[test]
    fn a_node_whose_providers_offer_nothing_offers_none() {
        let (said, _) = told(
            "none",
            concat!(
                r#"{"Ok": {"Offers": {"offers": [], "unanswered": {"/p": "no account"}, "#,
                r#""needs": ["cloud"]}}}"#,
            ),
        );
        assert_eq!(said, json!({"offers": []}));
    }

    #[test]
    fn a_node_that_takes_no_cloud_need_offers_nothing() {
        for (name, needs) in [
            ("old", ""),
            ("other", r#", "needs": ["gpu"]"#),
            ("misshaped", r#", "needs": "cloud""#),
        ] {
            let offered = concat!(
                r#"{"Ok": {"Offers": {"offers": [{"shape": "standard"}], "#,
                r#""unanswered": {}"#
            );
            let (said, _) = told(name, &format!("{offered}{needs}}}}}}}"));
            assert_eq!(
                said,
                json!({"error": {"code": "unoffered", "message": UNNEEDED}}),
                "{name}: a node that would drop the need starts nothing in the cloud"
            );
        }
    }

    #[test]
    fn a_node_that_cant_say_says_why() {
        for (name, reply, why) in [
            ("erred", r#"{"Err": "no providers"}"#, "no providers"),
            (
                "unknown",
                r#"{"Ok": {"Other": 1}}"#,
                "the node's answer couldn't be read",
            ),
            ("garbled", "not json", "the node's answer couldn't be read"),
        ] {
            let (said, _) = told(name, reply);
            assert_eq!(said["error"]["code"], json!("unoffered"), "{name}: {said}");
            assert!(
                said["error"]["message"]
                    .as_str()
                    .is_some_and(|said| said.starts_with(why)),
                "{name}: {said}"
            );
        }
    }

    #[test]
    fn offers_not_the_shells_or_with_no_node_arent_asked() {
        let gone = std::env::temp_dir().join("agents-offers-no-node.sock");
        assert_eq!(
            answer(|| Err(asker::REFUSED.to_owned()), Some(&gone)),
            json!({"error": {"code": "unoffered", "message": REFUSED}}),
            "anyone but the shell is refused before the node is asked"
        );
        assert_eq!(
            answer(
                || panic!("who asked is told only where there's a node to ask"),
                None
            )["error"]["message"],
            json!("this machine runs no node to ask what the cloud offers")
        );
    }
}
