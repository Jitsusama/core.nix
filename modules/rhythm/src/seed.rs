//! The minutes Joel was at the machine before rhythm first ran, so its
//! first read has weeks behind it rather than nothing: when he ran a
//! command in a shell, from zsh's history, and when he wrote to a pi
//! session, from pi's session logs. Only the times are read, never what
//! was typed or said.
//!
//! A pi log writes much in a person's place that no person typed, so only
//! a message entry of the user's counts. Of those, one opening `[` is the
//! harness's own word (`[Automated message ...]`), and a session's first
//! is often the brief another session spawned it with, so neither does.

use std::path::Path;

use crate::clock::iso;

/// The moments in zsh's history, each a command run, in seconds since
/// 1970. Its extended form opens each entry `: SECONDS:ELAPSED;`.
pub fn zsh(history: &[u8]) -> Vec<i64> {
    history
        .split(|byte| *byte == b'\n')
        .filter_map(|line| {
            let rest = line.strip_prefix(b": ")?;
            let end = rest.iter().position(|byte| !byte.is_ascii_digit())?;
            (rest.get(end) == Some(&b':'))
                .then(|| std::str::from_utf8(&rest[..end]).ok()?.parse().ok())
                .flatten()
        })
        .collect()
}

/// The moments Joel wrote to one pi session, from its log, in seconds
/// since 1970.
pub fn pi_session(log: &str) -> Vec<i64> {
    log.lines()
        .filter(|line| line.contains("\"type\":\"message\"") && line.contains("\"role\":\"user\""))
        .skip(1)
        .filter(|line| {
            line.split_once("\"text\":\"")
                .is_some_and(|(_, text)| !text.starts_with('['))
        })
        .filter_map(|line| iso(line.split_once("\"timestamp\":\"")?.1))
        .collect()
}

/// Every moment the logs under pi's sessions folder hold, from files
/// written since `since`, in seconds since 1970.
pub fn pi_sessions(folder: &Path, since: std::time::SystemTime) -> Vec<i64> {
    let mut found = Vec::new();
    let mut folders = vec![folder.to_path_buf()];
    while let Some(folder) = folders.pop() {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(about) = entry.metadata() else {
                continue;
            };
            if about.is_dir() {
                folders.push(path);
            } else if path.extension().is_some_and(|it| it == "jsonl")
                && about.modified().is_ok_and(|written| written >= since)
                && let Ok(log) = std::fs::read_to_string(&path)
            {
                found.extend(pi_session(&log));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zsh_s_history_gives_each_command_s_moment() {
        let history = b": 1794451086:0;ls\n: 1794454147:3;cd src\nan old line\n: 17x:0;no";
        assert_eq!(
            zsh(history),
            vec![1_794_451_086, 1_794_454_147],
            "the two moments"
        );
    }

    #[test]
    fn a_pi_log_gives_only_what_joel_wrote() {
        let entry = |role: &str, text: &str, at: &str| {
            format!(
                "{{\"type\":\"message\",\"timestamp\":\"{at}\",\"message\":{{\"role\":\"{role}\",\
                 \"content\":[{{\"type\":\"text\",\"text\":\"{text}\"}}]}}}}"
            )
        };
        let log = [
            entry("user", "the brief it was spawned with", "2026-11-09T20:00:00.000Z"),
            entry("user", "how is it going?", "2026-11-09T20:01:55.906Z"),
            entry("assistant", "fine", "2026-11-09T20:02:00.000Z"),
            entry("user", "[Automated message, no human input]", "2026-11-09T20:03:00.000Z"),
            "{\"type\":\"custom_message\",\"customType\":\"mesh\",\"timestamp\":\"2026-11-09T20:04:00.000Z\"}"
                .to_owned(),
            entry("user", "thanks", "2026-11-09T20:05:00.000Z"),
        ]
        .join("\n");
        let day = 20_766 * 86_400 + 20 * 3600;
        assert_eq!(
            pi_session(&log),
            vec![day + 60 + 55, day + 5 * 60],
            "his two messages, not the brief, the harness or the mesh"
        );
    }
}
