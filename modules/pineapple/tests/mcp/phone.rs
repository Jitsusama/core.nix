//! What Joel's phone is told through the gateway's tool that sends to
//! him alone: an alert is unsure only once the tool was asked, and what
//! anyone in Slack wrote never holds it back.

use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

use pineapple::mcp::Gateway;
use pineapple::phone::{Alert, Missed, tell_self};
use serde_json::json;

use super::{Reply, Seen, gateway, json_reply, methods, opened, scratch, sign_in, status, tool};

/// How the gateway answers a send, given how many calls came before.
type Answer = fn(&Seen, usize) -> Reply;

/// A case: its name, how the gateway answers, what Joel's phone is said
/// to have been told, and how many times the tool is asked.
type Case = (&'static str, Answer, &'static str, usize);

/// The gateway answering a session's opening with Slack's refusal.
fn refused_opening(one: &Seen, _: usize) -> Reply {
    match one.method.as_str() {
        "initialize" => json_reply(&json!({
            "jsonrpc": "2.0", "id": 1, "error": { "message": "invalid_auth" },
        })),
        _ => status(202),
    }
}

/// A proxy answering a session's opening with its own sign-in page.
fn page_at_opening(one: &Seen, _: usize) -> Reply {
    match one.method.as_str() {
        "initialize" => Reply {
            kind: "text/html",
            body: "<html>sign in</html>".to_owned(),
            ..status(200)
        },
        _ => status(202),
    }
}

/// A send whose session the gateway forgot, then whose opening again
/// meets a proxy's sign-in page, so the tool was never asked.
fn forgotten_then_page(one: &Seen, calls: usize) -> Reply {
    match one.method.as_str() {
        "initialize" if calls == 0 => opened("s1"),
        "initialize" => page_at_opening(one, calls),
        "tools/call" => status(404),
        _ => status(202),
    }
}

/// What Joel's phone is told when the tool's first ask is answered
/// `first`, the session opened again for it meets a proxy's sign-in page,
/// and every opening and ask after that goes through, so a resend without
/// the words would be sent.
fn told_after(name: &str, first: u16) -> Result<String, Missed> {
    let dir = scratch(name);
    let openings = AtomicUsize::new(0);
    let (url, _) = gateway(move |one: &Seen, calls| match one.method.as_str() {
        "initialize" if openings.fetch_add(1, Ordering::SeqCst) == 1 => page_at_opening(one, calls),
        "initialize" => opened("s1"),
        "tools/call" if calls == 0 => status(first),
        "tools/call" => tool("sent", false),
        _ => status(202),
    });
    let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
    let alert = Alert {
        title: "Someone stopped your agents",
        words: "stop",
        link: "https://example.slack.com/archives/C1/p1",
    };
    let told = tell_self(&mut gateway, alert);
    let _ = fs::remove_dir_all(&dir);
    told
}

/// A send the gateway answers with JSON-RPC's error `CODE`.
fn rpc<const CODE: i64>(one: &Seen, _: usize) -> Reply {
    match one.method.as_str() {
        "initialize" => opened("s1"),
        "tools/call" => json_reply(&json!({
            "jsonrpc": "2.0", "id": one.id,
            "error": { "code": CODE, "message": "refused" },
        })),
        _ => status(202),
    }
}

/// The gateway saying the send's tools are turned off.
fn turned_off() -> Reply {
    tool("\"slack_send_message_to_self\" is not available", true)
}

/// The send answered as `answer` says of how many came before it.
fn sending(one: &Seen, calls: usize, answer: fn(usize) -> Reply) -> Reply {
    match one.method.as_str() {
        "initialize" => opened("s1"),
        "tools/call" => answer(calls),
        _ => status(202),
    }
}

/// Cases where the gateway, or what's between, answers in its own way.
fn answered_by_the_gateway() -> [Case; 11] {
    [
        (
            "lost",
            |one, n| sending(one, n, |_| status(502)),
            "unsure",
            1,
        ),
        (
            "busy",
            |one, n| sending(one, n, |_| status(503)),
            "unsent",
            1,
        ),
        (
            "many",
            |one, n| sending(one, n, |_| status(429)),
            "unsent",
            1,
        ),
        ("refused-opening", refused_opening, "unsent", 0),
        ("page-opening", page_at_opening, "unsent", 0),
        // Unsent with or without its words, since no opening after the
        // first goes through; told_after's case pins that none is tried.
        ("forgotten-then-page", forgotten_then_page, "unsent", 1),
        // Refused before it ran, each is sent again without the words,
        // then left unsent.
        ("unparsed", rpc::<{ -32_700 }>, "unsent", 2),
        ("invalid", rpc::<{ -32_600 }>, "unsent", 2),
        ("unknown-method", rpc::<{ -32_601 }>, "unsent", 2),
        // A server may say either of what a tool gave back once it ran.
        ("arguments", rpc::<{ -32_602 }>, "unsure", 2),
        ("internal", rpc::<{ -32_603 }>, "unsure", 2),
    ]
}

/// Cases where the tool itself answers, turned off, refusing or failing.
fn answered_by_the_tool() -> [Case; 8] {
    [
        (
            "enable-lost",
            |one, n| {
                sending(one, n, |n| {
                    if n % 2 == 0 {
                        turned_off()
                    } else {
                        status(502)
                    }
                })
            },
            "unsent",
            4,
        ),
        (
            "still-off",
            |one, n| {
                sending(one, n, |n| {
                    if n % 3 == 1 {
                        tool("on", false)
                    } else {
                        turned_off()
                    }
                })
            },
            "unsent",
            6,
        ),
        // A refusal quoting words that say the tool is off holds back
        // only the words.
        (
            "quoting-off",
            |one, n| {
                sending(one, n, |n| match n {
                    1 => tool("on", false),
                    3 => tool("sent", false),
                    _ => turned_off(),
                })
            },
            "sent plain",
            4,
        ),
        // Nor when the turn-on they set off fails.
        (
            "quoting-off-refused",
            |one, n| {
                sending(one, n, |n| match n {
                    0 => turned_off(),
                    1 => tool("already on", true),
                    _ => tool("sent", false),
                })
            },
            "sent plain",
            3,
        ),
        (
            "refused",
            |one, n| sending(one, n, |_| tool("Slack said: invalid_auth", true)),
            "unsent",
            2,
        ),
        (
            "quoted",
            |one, n| sending(one, n, |n| tool("refused: we hit the rate limit", n == 0)),
            "sent plain",
            2,
        ),
        (
            "spent",
            |one, n| sending(one, n, |_| tool("Slack said: ratelimited", true)),
            "unsent",
            2,
        ),
        (
            "then-lost",
            |one, n| {
                sending(one, n, |n| {
                    if n == 0 {
                        tool("Slack said: not_allowed", true)
                    } else {
                        status(502)
                    }
                })
            },
            "unsure",
            2,
        ),
    ]
}

#[test]
fn a_send_whose_pass_asked_nothing_is_never_sent_again_without_its_words() {
    for (name, first) in [("forgotten", 404), ("refused", 401)] {
        let told = told_after(&format!("tell-self-{name}-then-page"), first);
        assert!(
            matches!(told, Err(Missed::NotSent(_))),
            "{name}: a pass that asked nothing refused no words, so the alert isn't sent \
             without them once a session opens again: {told:?}"
        );
    }
}

#[test]
fn a_name_from_slack_quoted_back_in_a_failing_holds_no_alert_back() {
    // Slack's word for a rate spent, and its word for a sign-in refused.
    for word in ["rate_limited", "invalid_auth"] {
        let dir = scratch(&format!("tell-self-quoted-{word}"));
        let (url, _) = gateway(|one: &Seen, _| match one.method.as_str() {
            "initialize" => opened("s1"),
            "tools/call" => tool(
                &format!("could not post {}", one.params["arguments"]["message"]),
                true,
            ),
            _ => status(202),
        });
        let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
        let title = format!("Someone stopped agents in owner/{word}");
        let alert = Alert {
            title: &title,
            words: "stop",
            link: "https://example.slack.com/archives/C1/p1",
        };
        let told = tell_self(&mut gateway, alert);
        assert!(
            matches!(told, Err(Missed::Unsure(_))),
            "a failing quoting {word} in the title is the tool's failing: {told:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

#[test]
fn an_alert_is_unsure_only_once_its_tool_was_asked_and_its_words_never_hold_it_back() {
    let alert = Alert {
        title: "Someone stopped your agents",
        words: "we hit the rate limit",
        link: "https://example.slack.com/archives/C1/p1",
    };
    let cases = answered_by_the_gateway()
        .into_iter()
        .chain(answered_by_the_tool());
    for (name, answer, said, sends) in cases {
        let dir = scratch(&format!("tell-self-{name}"));
        let (url, seen) = gateway(answer);
        let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
        let told = tell_self(&mut gateway, alert);
        let heard = match &told {
            Ok(text) if text.contains("(the words are at the link)") => "sent plain",
            Ok(_) => "sent",
            Err(Missed::Unsure(_)) => "unsure",
            Err(Missed::NotSent(_)) => "unsent",
        };
        let asked = methods(&seen)
            .iter()
            .filter(|method| *method == "tools/call")
            .count();
        assert!(
            heard == said && asked == sends,
            "{name}: a send is unsure only once the tool was asked and may have done it, \
             a busy or refused gateway and a failed opening sent nothing, and a tool's \
             failing is judged only once it's asked again without the words: {told:?}, \
             {asked} asked"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
