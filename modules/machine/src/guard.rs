//! What the shell asks of the account's disk guard: keeping a folder from
//! its clearing for Joel's reason (`disk-keep`), letting it go again
//! (`disk-unkeep`), clearing what it would take of one now
//! (`disk-clear`), and letting a writer it froze carry on (`disk-thaw`).
//! Each is the guard's own `keep`, `unkeep`, `clear` or `thaw`,
//! run here on the act rather than spawned by the shell, so the shell
//! spawns nothing (Y2YBTS, 2026-11-12). The guard writes its keeps and
//! asks in its own state folder and marks its status before it ends, so
//! the status the shell hears has moved by the time the act answers.
//!
//! The guard is asked for JSON, as every command of ours answers a
//! program (docs/commands.md): its facts where it did the act, and its
//! error's parts where it didn't. The sentence Joel reads in the foot is
//! worded here from those facts, so a change to the guard's text for a
//! person never changes what the shell shows.

use std::process::{Command, Stdio};

use serde_json::Value;

/// Where the guard's program is, when it isn't `disk-guard` on the path,
/// as a test gives its stand-in.
pub const DISK_GUARD: &str = "MACHINE_DISK_GUARD";

/// Why an act wasn't done: its cause, for the journal, and what Joel
/// reads of it, the guard's own what and why where it ran and refused, or
/// nothing where it couldn't be asked, which the shell words itself.
#[derive(Debug, PartialEq, Eq)]
pub struct Untold {
    pub code: &'static str,
    pub message: String,
    pub said: String,
}

fn untold(code: &'static str, message: impl Into<String>) -> Untold {
    Untold {
        code,
        message: message.into(),
        said: String::new(),
    }
}

/// The guard's words for an act, or why the act can't be one: each but a
/// thaw needs its folder, and a keep its reason. Its answer is asked for
/// as JSON, and the folder follows `--`, so one beginning with a dash is
/// still a folder.
pub fn words(act: &str, said: &Value) -> Result<Vec<String>, Untold> {
    let asked = |words: &[&str]| {
        ["--json"]
            .iter()
            .chain(words)
            .map(|word| (*word).to_owned())
            .collect()
    };
    if act == "disk-thaw" {
        return Ok(asked(&["thaw"]));
    }
    let text = |name: &str| said[name].as_str().unwrap_or("").to_owned();
    let path = text("path");
    if path.is_empty() {
        return Err(untold("bad-act", "a disk guard's act needs its folder"));
    }
    match act {
        "disk-keep" => {
            let reason = text("reason");
            if reason.trim().is_empty() {
                return Err(untold("bad-act", "keeping a folder needs its reason"));
            }
            Ok(asked(&["keep", "--", &path, &reason]))
        }
        "disk-unkeep" => Ok(asked(&["unkeep", "--", &path])),
        "disk-clear" => Ok(asked(&["clear", "--", &path])),
        _ => Err(untold("unknown-act", format!("no act {act:?} here"))),
    }
}

/// The sentence for what the guard did, worded from the facts it
/// answered: a line a unit thawed, or one for the folder it kept, let go
/// or is clearing.
pub fn sentence(answer: &Value) -> Option<String> {
    let text = |name: &str| answer[name].as_str();
    let said = match text("schema")? {
        "disk-guard.keep/1" => format!("Kept {}: {}.", text("path")?, text("reason")?),
        "disk-guard.unkeep/1" if answer["was_kept"] == true => {
            format!("No longer kept: {}.", text("path")?)
        }
        "disk-guard.unkeep/1" => format!("{} wasn't kept.", text("path")?),
        "disk-guard.clear/1" => format!(
            "Clearing {}; the guard starts on its next step.",
            text("path")?
        ),
        "disk-guard.thaw/1" => {
            let thawed: Vec<String> = answer["thawed"]
                .as_array()?
                .iter()
                .filter_map(Value::as_str)
                .map(|unit| format!("Thawed {unit}."))
                .collect();
            if thawed.is_empty() {
                "Nothing is frozen.".to_owned()
            } else {
                thawed.join("\n")
            }
        }
        _ => return None,
    };
    Some(said)
}

/// What Joel reads of the guard's refusal, from the error it said: what
/// failed, and why where it said why.
pub fn refusal(error: &Value) -> Option<String> {
    let what = error["what"].as_str()?;
    Some(match error["why"].as_str() {
        Some(why) => format!("{what}: {why}"),
        None => what.to_owned(),
    })
}

/// Does the act, saying what the guard did: done where it ended well,
/// and refused, with its error's words, where it didn't.
pub fn told(act: &str, said: &Value) -> Result<String, Untold> {
    let words = words(act, said)?;
    let program = std::env::var(DISK_GUARD).unwrap_or_else(|_| "disk-guard".to_owned());
    let ran = Command::new(&program)
        .args(&words)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| {
            untold(
                "refused",
                format!("the disk guard couldn't be run: {error}"),
            )
        })?;
    let printed = |bytes: &[u8]| String::from_utf8_lossy(bytes).trim().to_owned();
    let (out, err) = (printed(&ran.stdout), printed(&ran.stderr));
    let read = |text: &str| serde_json::from_str::<Value>(text).ok();
    if ran.status.success() {
        return read(&out).as_ref().and_then(sentence).ok_or_else(|| {
            untold(
                "refused",
                format!("the disk guard answered what isn't its act's: {out}"),
            )
        });
    }
    // An error it couldn't say as its own, as from a guard too old to
    // answer JSON, is kept for the journal and left to the shell to word.
    let said = read(&err).as_ref().and_then(refusal).unwrap_or_default();
    Err(Untold {
        code: "refused",
        message: err,
        said,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn owned(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).to_owned()).collect()
    }

    #[test]
    fn each_act_is_the_guards_own_words_and_needs_its_folder() {
        let keep = json!({"path": "/home/joel/src", "reason": "the build's"});
        assert_eq!(
            words("disk-keep", &keep),
            Ok(owned(&[
                "--json",
                "keep",
                "--",
                "/home/joel/src",
                "the build's"
            ]))
        );
        assert_eq!(
            words("disk-unkeep", &keep),
            Ok(owned(&["--json", "unkeep", "--", "/home/joel/src"]))
        );
        assert_eq!(
            words("disk-clear", &keep),
            Ok(owned(&["--json", "clear", "--", "/home/joel/src"]))
        );
        let code = |act: &str, said: Value| words(act, &said).map_err(|no| no.code);
        assert_eq!(code("disk-keep", json!({"path": "/srv"})), Err("bad-act"));
        assert_eq!(code("disk-clear", json!({})), Err("bad-act"));
        assert_eq!(
            words("disk-thaw", &json!({})),
            Ok(owned(&["--json", "thaw"]))
        );
        assert_eq!(
            code("disk-wipe", json!({"path": "/srv"})),
            Err("unknown-act")
        );
        let said = |act: &str, said: Value| words(act, &said).map_err(|no| no.said);
        assert_eq!(
            said("disk-keep", json!({"path": "/srv"})),
            Err(String::new()),
            "a badly asked act leaves its words to the shell"
        );
    }

    #[test]
    fn each_sentence_is_worded_from_the_guards_facts() {
        let cases = [
            (
                json!({"schema": "disk-guard.keep/1", "path": "/srv", "reason": "the build's"}),
                "Kept /srv: the build's.",
            ),
            (
                json!({"schema": "disk-guard.unkeep/1", "path": "/srv", "was_kept": true}),
                "No longer kept: /srv.",
            ),
            (
                json!({"schema": "disk-guard.unkeep/1", "path": "/srv", "was_kept": false}),
                "/srv wasn't kept.",
            ),
            (
                json!({"schema": "disk-guard.clear/1", "path": "/srv"}),
                "Clearing /srv; the guard starts on its next step.",
            ),
            (
                json!({"schema": "disk-guard.thaw/1", "thawed": []}),
                "Nothing is frozen.",
            ),
            (
                json!({"schema": "disk-guard.thaw/1", "thawed": ["a.service", "b.service"]}),
                "Thawed a.service.\nThawed b.service.",
            ),
        ];
        for (answer, said) in cases {
            assert_eq!(sentence(&answer).as_deref(), Some(said), "{answer}");
        }
        assert_eq!(
            sentence(&json!({"schema": "disk-guard.status/1"})),
            None,
            "an answer that isn't an act's is no sentence"
        );
    }

    #[test]
    fn a_refusal_says_the_guards_what_and_why() {
        let error = json!({"schema": "error/1", "what": "/srv stays",
            "why": "you keep it: the build's", "try": "disk-guard unkeep /srv"});
        assert_eq!(
            refusal(&error).as_deref(),
            Some("/srv stays: you keep it: the build's")
        );
        assert_eq!(
            refusal(&json!({"what": "/srv isn't there to clear", "why": null})).as_deref(),
            Some("/srv isn't there to clear")
        );
        assert_eq!(refusal(&json!("not an error")), None);
    }
}
