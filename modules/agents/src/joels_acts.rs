//! Joel's acts as the daemon holds them to the shell: which asks are his
//! ([`AS_JOEL`]), which of them the shell's check holds, and what each is
//! told, and journalled, where that check refused it. Apart from main.rs,
//! which serves them (`for_the_shell_alone`), so neither passes 800 lines.

use std::path::Path;

use agents::asker;
use agents::clock;
use agents::wants::Wants;
use serde_json::{Value, json};

/// What the daemon is asked as Joel's: what the node takes as his, a
/// row's act (acts.rs), a line he tells and his words on a landing; and
/// what the shell says of him (`own`), a Not Now, a put-off or forgotten
/// reopening, a landing read, a drawer opened, a shared screen and his
/// windows' places. Said by an agent, the first would steer a session as
/// him, and the rest would hide from him what he should see, or show a
/// shared screen what he'd keep off it (ISXEG8, 26 November).
pub(super) const AS_JOEL: [&str; 15] = [
    "pause",
    "resume",
    "stop",
    "close",
    "wake",
    "talk",
    "tell",
    "feedback",
    "not-now",
    "put-off",
    "forget",
    "read",
    "heard",
    "sharing",
    "keep-places",
];

/// Whether `said` is one of [`AS_JOEL`] the shell's check holds: each but
/// a share's start, which only hides Joel's amounts, so it's taken from
/// anyone. Held to the shell, one refused through a slow user manager
/// left amounts served on a shared screen till the next opening; an
/// agent saying it hides them only until the shell tells the share's
/// end, at that opening (round 000035 on #6796, F20).
pub(super) fn held_to_the_shell(said: &Value) -> bool {
    let starts_a_share = said["ask"] == "sharing" && said["sharing"] == true;
    said["ask"]
        .as_str()
        .is_some_and(|ask| AS_JOEL.contains(&ask))
        && !starts_a_share
}

/// What one of [`AS_JOEL`] is told where the shell's check refused it,
/// in the form each is read: an act as the shell reads a steer
/// refused (Sessions.qml's steered), journalled with what it named, so
/// an agent acting as Joel can be traced to it.
pub(super) fn not_as_joel(why: &str, said: &Value) -> Value {
    eprintln!(
        "agents: a {} for {} wasn't taken: {why}",
        named(&said["ask"]),
        named(&said["address"])
    );
    json!({"error": {"code": "refused", "message": NOT_AS_JOEL}})
}

/// Why one of [`AS_JOEL`] the shell's check refused wasn't done, which
/// Joel reads where his own act was refused because the check couldn't
/// be made.
pub(super) const NOT_AS_JOEL: &str =
    "the agents daemon acts for you only from the launcher, and couldn't tell this came from it";

/// What an answer the shell's check refused is told, as Let Them Start
/// says its own. Joel reads it where his own Return was refused because
/// the check couldn't be made, so it says what the daemon couldn't tell
/// rather than who may answer.
pub(super) const ONLY_THE_SHELL: &str =
    "the agents daemon takes an answer only from the launcher, and couldn't tell this came from it";

/// The most of what a refused ask named (its `ask`, `want` or
/// `address`) its journal line says, its quoted JSON cut short: a want's
/// id and an address are far shorter.
pub(super) const NAMED_AT_MOST: usize = 120;

/// What a refused ask named, as its journal line says it: its quoted
/// JSON, cut to [`NAMED_AT_MOST`], since the asker chose it and could
/// write a line as long as it liked (round 000035 on #6796, F13).
pub(super) fn named(value: &Value) -> String {
    value.to_string().chars().take(NAMED_AT_MOST).collect()
}

/// Joel's answer to one of `wants`, steered through `node` only where
/// `asker` says the shell asked it: the node takes it as his, so an
/// agent answering its own ask through this daemon would answer as him
/// (ISXEG8's R-agent-answers-own-ask).
pub(super) fn answered(
    asker: Result<(), String>,
    wants: &Wants,
    said: &Value,
    node: Option<&Path>,
) -> Value {
    if let Err(why) = asker {
        // Why it was refused is journalled with what it named, beside the
        // line asker.rs writes of who asked, so an agent answering its own
        // ask can be traced to it. What it named is the asker's own, cut
        // short, so no ask writes more than a line.
        let why: &str = if why == asker::REFUSED {
            "who asked isn't the shell"
        } else {
            why.as_str()
        };
        eprintln!(
            "agents: an answer to {} for {} wasn't taken: {why}",
            named(&said["want"]),
            named(&said["address"])
        );
        return json!({"error": {"code": "refused", "message": ONLY_THE_SHELL}});
    }
    // Only an answer reaches here, so the wants always have one to give.
    wants
        .answer(said, node, clock::now_ms())
        .unwrap_or_else(|| json!({"error": {"code": "not-an-ask", "message": "not an answer"}}))
}
