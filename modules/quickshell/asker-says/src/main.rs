//! Says what a process asking for Joel's password says about itself: the
//! quest and the session an agent runs under, so polkit's prompt can say
//! which agent asks below the command it would run (ISXEG8, 2026-11-15).
//!
//! Usage: asker-says PID. It reads `/proc/PID/environ` as Joel, and from it
//! only `PI_QUEST` and `PI_MESH_SESSION`, by those two names and no other,
//! so nothing else a process holds, a key or a token, is ever read into an
//! answer or a failure. The shell never reads the environment itself, since
//! that would put every secret beside them in its heap.
//!
//! A process that holds neither, as run0 started by a script under an agent
//! may not, is asked through its parents in turn, up to the first that holds
//! one, stopping at the account's manager, at a process of another account,
//! whose environment Joel can't read, or after 64. A value not in the shape
//! the harness gives, a quest's id or a session's UUID, is left out rather
//! than said, so a process can't put words of its own on the prompt.
//!
//! With the quest, it answers that quest's short title, as the launcher's
//! Agents and Spec name a quest, from the README the harness keeps for it,
//! since an id means nothing to Joel at a password prompt (VKCF3Z,
//! 2026-11-15). The id is read only in its shape, so it names a folder
//! under the quests' and nowhere else.
//!
//! What it answers is only what the process says: any process can set its
//! own `PI_QUEST`, so the prompt shows it as said, never as proof, and the
//! command stays what Joel judges by. A process that's gone, or that says
//! neither, answers neither, and the prompt shows no session line.
//!
//! `ASKER_SAYS_PROC` names a stand-in for /proc, and `ASKER_SAYS_QUESTS` one
//! for the quests' folder, for tests.

use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{fs, str};

use serde_json::{Value, json};
use terminal_kit::{Exit, Failure, Help};

/// The quest's name in a process's environment.
const QUEST: &str = "PI_QUEST";
/// The session's name in a process's environment.
const SESSION: &str = "PI_MESH_SESSION";
/// How many processes up the walk asks before it stops, far past any
/// agent's depth: an agent's bash, a script, run0.
const DEEPEST: usize = 64;
/// The account's manager, `systemd --user`, as its stat names it, and init,
/// where every walk ends: past it, everything is the account's own, never
/// one agent's.
const MANAGER: &str = "systemd";

/// Where the harness keeps each quest's folder, under the account's data.
const QUESTS: &str = "pi/agentic-harness.pi/quest-workflow/quests";

/// What a process says of itself, which process said it, and the title of
/// the quest it names.
#[derive(Debug, Default, PartialEq)]
struct Says {
    quest: Option<String>,
    session: Option<String>,
    by: Option<u32>,
    title: Option<String>,
}

impl Says {
    fn answer(&self) -> Value {
        json!({
            "schema": "asker-says.says/1",
            "quest": self.quest,
            "session": self.session,
            "by": self.by,
            "title": self.title,
        })
    }

    fn said(&self) -> String {
        match (&self.quest, &self.session) {
            (None, None) => "it says no quest or session\n".to_owned(),
            (quest, session) => format!(
                "quest {}{}, session {}\n",
                quest.as_deref().unwrap_or("unsaid"),
                self.title
                    .as_deref()
                    .map_or_else(String::new, |title| format!(" ({title})")),
                session.as_deref().unwrap_or("unsaid")
            ),
        }
    }
}

/// Whether a value is a quest's id as the harness gives it, such as
/// QEST-20261110-H6EDLC.
fn quest_shaped(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    matches!(parts.as_slice(), ["QEST", date, short]
        if date.len() == 8
            && date.bytes().all(|byte| byte.is_ascii_digit())
            && short.len() == 6
            && short.bytes().all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit()))
}

/// Whether a value is a session's UUID, in lower-case hex as the harness
/// gives it.
fn session_shaped(value: &str) -> bool {
    let lengths: Vec<usize> = value.split('-').map(str::len).collect();
    lengths == [8, 4, 4, 4, 12]
        && value
            .bytes()
            .all(|byte| byte == b'-' || byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// The value of `name` in an environment's NUL-ended entries, if it's there
/// and in the shape `shaped` wants. Every other entry is passed over unread.
fn named(environ: &[u8], name: &str, shaped: fn(&str) -> bool) -> Option<String> {
    environ.split(|byte| *byte == 0).find_map(|entry| {
        let value = entry.strip_prefix(name.as_bytes())?.strip_prefix(b"=")?;
        str::from_utf8(value)
            .ok()
            .filter(|value| shaped(value))
            .map(str::to_owned)
    })
}

/// A README's short title from its frontmatter, else its H1 past the
/// frontmatter, as cadence's Spec reads one (cadence/src/axes/spec.rs).
fn title(readme: &str) -> Option<String> {
    let (front, body) = readme
        .strip_prefix("---\n")
        .and_then(|front| front.split_once("\n---\n"))
        .unwrap_or(("", readme));
    let short = front
        .lines()
        .find_map(|line| line.strip_prefix("short:"))
        .map(|short| short.trim().trim_matches('"').trim())
        .filter(|short| !short.is_empty());
    if let Some(short) = short {
        return Some(short.to_owned());
    }
    body.lines()
        .find_map(|line| line.strip_prefix("# "))
        .map(|title| title.trim().to_owned())
        .filter(|title| !title.is_empty())
}

/// The title of the quest `quest` names, from its README in `quests`; the
/// id is in its shape, so it names one folder there.
fn titled(quests: &Path, quest: &str) -> Option<String> {
    title(&fs::read_to_string(quests.join(quest).join("README.md")).ok()?)
}

/// A process's name and its parent's pid, from its stat. The name sits in
/// brackets and may hold spaces or brackets itself, so the fields after it
/// are read from the last closing one.
fn parent(stat: &str) -> Option<(&str, u32)> {
    let open = stat.find('(')?;
    let close = stat.rfind(')')?;
    let name = stat.get(open + 1..close)?;
    let mut after = stat.get(close + 1..)?.split_whitespace();
    let _state = after.next()?;
    let up = after.next()?.parse().ok()?;
    Some((name, up))
}

/// What the process `pid` says of itself, or the nearest of its parents
/// that says anything, read from `proc`, with its quest's title from
/// `quests`.
fn says(proc: &Path, quests: &Path, pid: u32) -> Says {
    let mut at = pid;
    for _ in 0..DEEPEST {
        let process = proc.join(at.to_string());
        let Ok(stat) = fs::read_to_string(process.join("stat")) else {
            break;
        };
        let Some((name, up)) = parent(&stat) else {
            break;
        };
        if name == MANAGER {
            break;
        }
        let Ok(environ) = fs::read(process.join("environ")) else {
            break;
        };
        let quest = named(&environ, QUEST, quest_shaped);
        let session = named(&environ, SESSION, session_shaped);
        if quest.is_some() || session.is_some() {
            let title = quest.as_deref().and_then(|quest| titled(quests, quest));
            return Says {
                quest,
                session,
                by: Some(at),
                title,
            };
        }
        if up <= 1 {
            break;
        }
        at = up;
    }
    Says::default()
}

fn help() -> Help {
    Help::new(
        "asker-says",
        "says the quest, its title and the session a process says it runs under",
    )
    .command("PID", "what PID, or the nearest parent, says")
    .example(
        "asker-says --json 4242",
        "what polkit's prompt asks of the process asking",
    )
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let [pid] = context.args.as_slice() else {
            return Err(Failure::new(Exit::Usage, "it takes one PID").try_this("asker-says --help"));
        };
        let Ok(pid) = pid.parse::<u32>() else {
            return Err(
                Failure::new(Exit::Usage, "a PID is a number").try_this("asker-says --help")
            );
        };
        let proc =
            env::var_os("ASKER_SAYS_PROC").map_or_else(|| PathBuf::from("/proc"), PathBuf::from);
        let quests = env::var_os("ASKER_SAYS_QUESTS").map_or_else(
            || {
                env::var_os("XDG_DATA_HOME")
                    .map(PathBuf::from)
                    .or_else(|| {
                        env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
                    })
                    .unwrap_or_default()
                    .join(QUESTS)
            },
            PathBuf::from,
        );
        let said = says(&proc, &quests, pid);
        let written = if context.format() == &terminal_kit::Format::Text {
            context.say(&said.said())
        } else {
            context.answer(&said.answer())
        };
        written.map_err(|error| {
            Failure::new(Exit::Failed, "its answer couldn't be written").because(error.to_string())
        })
    })
}

#[cfg(test)]
mod tests;
