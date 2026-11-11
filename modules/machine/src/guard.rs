//! What the shell asks of the account's disk guard: keeping a folder from
//! its clearing for Joel's reason (`disk-keep`), letting it go again
//! (`disk-unkeep`), clearing what it would take of one now
//! (`disk-clear`), and letting a writer it froze carry on (`disk-thaw`).
//! Each is the guard's own `keep`, `unkeep`, `clear` or `thaw`,
//! run here on the act rather than spawned by the shell, so the shell
//! spawns nothing (Y2YBTS, 2026-11-12). The guard writes its keeps and
//! asks in its own state folder and marks its status before it ends, so
//! the status the shell hears has moved by the time the act answers.
//! What it printed is what the act says, as Joel reads it in the foot.

use std::process::{Command, Stdio};

use serde_json::Value;

/// Where the guard's program is, when it isn't `disk-guard` on the path,
/// as a test gives its stand-in.
pub const DISK_GUARD: &str = "MACHINE_DISK_GUARD";

/// Why an act wasn't done: its cause, for the journal, and what Joel
/// reads of it, the guard's own sentence where it ran and refused, or
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

/// What Joel reads of the guard's refusal: the sentence it printed,
/// without the program's name it opens its errors with.
fn refusal_said(printed: &str) -> String {
    printed
        .lines()
        .map(|line| line.strip_prefix("disk-guard: ").unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The guard's words for an act, or why the act can't be one: each but a
/// thaw needs its folder, and a keep its reason.
pub fn words(act: &str, said: &Value) -> Result<Vec<String>, Untold> {
    if act == "disk-thaw" {
        return Ok(vec!["thaw".to_owned()]);
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
            Ok(vec!["keep".to_owned(), path, reason])
        }
        "disk-unkeep" => Ok(vec!["unkeep".to_owned(), path]),
        "disk-clear" => Ok(vec!["clear".to_owned(), path]),
        _ => Err(untold("unknown-act", format!("no act {act:?} here"))),
    }
}

/// Does the act, saying what the guard printed: done where it ended well,
/// and refused, with what it said, where it didn't.
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
    // It prints one sentence, on its output where it did it and on its
    // errors where it couldn't.
    let printed = |bytes: &[u8]| String::from_utf8_lossy(bytes).trim().to_owned();
    let (out, err) = (printed(&ran.stdout), printed(&ran.stderr));
    let both = [out, err]
        .into_iter()
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if ran.status.success() {
        Ok(both)
    } else {
        Err(Untold {
            code: "refused",
            said: refusal_said(&both),
            message: both,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn each_act_is_the_guards_own_words_and_needs_its_folder() {
        let keep = json!({"path": "/home/joel/src", "reason": "the build's"});
        assert_eq!(
            words("disk-keep", &keep),
            Ok(vec![
                "keep".to_owned(),
                "/home/joel/src".to_owned(),
                "the build's".to_owned()
            ])
        );
        assert_eq!(
            words("disk-unkeep", &keep),
            Ok(vec!["unkeep".to_owned(), "/home/joel/src".to_owned()])
        );
        assert_eq!(
            words("disk-clear", &keep),
            Ok(vec!["clear".to_owned(), "/home/joel/src".to_owned()])
        );
        let code = |act: &str, said: Value| words(act, &said).map_err(|no| no.code);
        assert_eq!(code("disk-keep", json!({"path": "/srv"})), Err("bad-act"));
        assert_eq!(code("disk-clear", json!({})), Err("bad-act"));
        assert_eq!(words("disk-thaw", &json!({})), Ok(vec!["thaw".to_owned()]));
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
    fn a_refusal_says_the_guards_sentence_without_its_name() {
        assert_eq!(
            refusal_said("disk-guard: /srv stays: you keep it: the build's"),
            "/srv stays: you keep it: the build's"
        );
        assert_eq!(refusal_said("/srv wasn't kept."), "/srv wasn't kept.");
    }
}
