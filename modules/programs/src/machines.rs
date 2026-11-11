//! What kitty's SSH to a Machine reads (modules/kitty/Kitty.qml), asked
//! for by key as the shell starts, and watched:
//! `machines:<folder>` says the folder's `config` and `known_hosts` as
//! written, each empty where it isn't there; `reached:<history>` says the
//! lines of zsh's extended history that ran ssh, each `: <start>:
//! <elapsed>;` then ssh, `kitten ssh` or `kitty +kitten ssh` and a space,
//! of however many it keeps. Each is read as an ask names it, and again
//! as one of its files is written while its key is watched, so nothing is
//! read at rest. The page still reads machines
//! from the text, a stopgap until Programs serves its own ready rows.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};

/// How each command that ran ssh starts, after the history's stamp.
const SSH: [&str; 3] = ["ssh", "kitten ssh", "kitty +kitten ssh"];

/// The record a key names, or none for a key of neither kind or a path
/// that isn't whole.
#[must_use]
pub fn record(key: &str) -> Option<Value> {
    let whole = |path: &str| path.starts_with('/');
    if let Some(folder) = key.strip_prefix("machines:").filter(|path| whole(path)) {
        let text = |name: &str| text_of(&Path::new(folder).join(name));
        return Some(json!({
            "key": key, "kind": "machines", "folder": folder,
            "config": text("config"), "hosts": text("known_hosts"),
        }));
    }
    let history = key.strip_prefix("reached:").filter(|path| whole(path))?;
    let lines: Vec<String> = text_of(Path::new(history))
        .lines()
        .filter(|line| ran_ssh(line))
        .map(str::to_owned)
        .collect();
    Some(json!({"key": key, "kind": "reached", "history": history, "lines": lines}))
}

/// A file's text, any bytes that aren't UTF-8 replaced, as zsh's history
/// may hold some; empty where it can't be read.
fn text_of(path: &Path) -> String {
    fs::read(path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default()
}

/// Whether a line of zsh's extended history ran ssh.
fn ran_ssh(line: &str) -> bool {
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    let Some((stamp, command)) = line
        .strip_prefix(": ")
        .and_then(|rest| rest.split_once(';'))
    else {
        return false;
    };
    let stamped = stamp
        .split_once(':')
        .is_some_and(|(start, took)| digits(start) && digits(took));
    let command = command.trim_start();
    stamped
        && SSH.iter().any(|ran| {
            command
                .strip_prefix(ran)
                .is_some_and(|after| after.starts_with(char::is_whitespace))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_stamped_line_that_ran_ssh_is_kept() {
        let kept = [
            ": 1700000000:0;ssh tide",
            ": 1700000000:3; kitten ssh -p 2222 reef",
            ": 1:0;kitty +kitten ssh\tshoal",
        ];
        let passed = [
            "ssh tide",
            ": 1700000000:0;sshfs tide:/ /mnt",
            ": 1700000000:0;ssh",
            ": x:0;ssh tide",
            ": 1700000000:0;echo ssh tide",
        ];
        for line in kept {
            assert!(ran_ssh(line), "{line:?} wasn't kept");
        }
        for line in passed {
            assert!(!ran_ssh(line), "{line:?} was kept");
        }
    }

    #[test]
    fn only_a_whole_path_of_either_kind_is_read() {
        assert!(
            record("machines:.ssh").is_none(),
            "a part of a path was read"
        );
        assert!(
            record("reached:history").is_none(),
            "a part of a path was read"
        );
        assert!(record("visited:/db.zo").is_none(), "another kind was read");
        assert_eq!(
            record("machines:/nowhere")
                .map(|record| (record["config"].clone(), record["hosts"].clone())),
            Some((json!(""), json!(""))),
            "files that aren't there didn't say nothing"
        );
    }
}
