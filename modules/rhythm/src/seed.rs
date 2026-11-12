//! The minutes Joel was at the machine before rhythm first ran, so its
//! first read has weeks behind it rather than nothing: when he ran a
//! command in a shell, from zsh's history, and when he wrote to a pi
//! session, from pi's session logs. Only the times are read, never what
//! was typed or said.
//!
//! A pi log writes much in a person's place that no person typed, so only
//! a message entry of the user's counts. Of those, one opening `[` is the
//! harness's own word (`[Automated message ...]`), as is one opening as
//! `NOT_TYPED` says, or as one session's to another (`From P9R1KI (window
//! 4): ...`); and a session's first is often the brief another session
//! spawned it with, so none of them does. The harness marks what's typed
//! only from November 2026, so the logs before need telling apart by how
//! they open.

use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::clock::iso;

/// How what the harness writes in a person's place opens, in pi's logs.
const NOT_TYPED: [&str; 9] = [
    "[",
    "The context was compacted",
    "Compacting the context",
    "You were restarted",
    "You are a child of",
    "Verification found",
    "Load quest ",
    "Run exactly this bash command",
    "Reply with the single word",
];

/// Whether a user message's text is one a person typed, as far as how it
/// opens can say.
fn typed(text: &str) -> bool {
    let from_a_session = text
        .strip_prefix("From ")
        .and_then(|rest| rest.split_once(" (window "))
        .is_some_and(|(address, _)| {
            address.len() == 6
                && address
                    .chars()
                    .all(|letter| letter.is_ascii_uppercase() || letter.is_ascii_digit())
        });
    !from_a_session && !NOT_TYPED.iter().any(|opening| text.starts_with(opening))
}

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

/// The moments Joel wrote to one pi session, from its log's lines, in
/// seconds since 1970. The lines come one at a time, since a log can run
/// to hundreds of megabytes.
pub fn pi_session(log: impl Iterator<Item = String>) -> Vec<i64> {
    log.filter(|line| line.contains("\"type\":\"message\"") && line.contains("\"role\":\"user\""))
        .skip(1)
        .filter(|line| {
            line.split_once("\"text\":\"")
                .is_some_and(|(_, text)| typed(text))
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
                && let Ok(log) = std::fs::File::open(&path)
            {
                found.extend(pi_session(
                    BufReader::new(log).lines().map_while(Result::ok),
                ));
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
            entry(
                "user",
                "the brief it was spawned with",
                "2026-11-09T20:00:00.000Z",
            ),
            entry("user", "how is it going?", "2026-11-09T20:01:55.906Z"),
            entry("assistant", "fine", "2026-11-09T20:02:00.000Z"),
            entry(
                "user",
                "[Automated message, no human input]",
                "2026-11-09T20:03:00.000Z",
            ),
            concat!(
                "{\"type\":\"custom_message\",\"customType\":\"mesh\",",
                "\"timestamp\":\"2026-11-09T20:04:00.000Z\"}"
            )
            .to_owned(),
            entry("user", "thanks", "2026-11-09T20:05:00.000Z"),
        ]
        .join("\n");
        let day = 20_766 * 86_400 + 20 * 3600;
        assert_eq!(
            pi_session(log.lines().map(str::to_owned)),
            vec![day + 60 + 55, day + 5 * 60],
            "his two messages, not the brief, the harness or the mesh"
        );
    }

    #[test]
    fn what_the_harness_and_other_sessions_say_in_his_place_is_not_his() {
        let entry = |text: &str, at: &str| {
            format!(
                "{{\"type\":\"message\",\"timestamp\":\"{at}\",\"message\":{{\"role\":\"user\",\
                 \"content\":[{{\"type\":\"text\",\"text\":\"{text}\"}}]}}}}"
            )
        };
        let log = [
            entry("the brief", "2026-11-09T20:00:00.000Z"),
            entry(
                "The context was compacted to keep this session affordable.",
                "2026-11-09T20:01:00.000Z",
            ),
            entry(
                "From P9R1KI (window 4): Joel says go",
                "2026-11-09T20:02:00.000Z",
            ),
            entry(
                "You were restarted to free memory.",
                "2026-11-09T20:03:00.000Z",
            ),
            entry("Compacting the context failed", "2026-11-09T20:04:00.000Z"),
            entry("Verification found 2 errors", "2026-11-09T20:05:00.000Z"),
            entry(
                "Load quest QEST-20261108-1H56ZJ",
                "2026-11-09T20:06:00.000Z",
            ),
            entry(
                "You are a child of orchestrator P9R1KI",
                "2026-11-09T20:07:00.000Z",
            ),
            entry("From here on, go faster", "2026-11-09T20:08:00.000Z"),
        ]
        .join("\n");
        let day = 20_766 * 86_400 + 20 * 3600;
        assert_eq!(
            pi_session(log.lines().map(str::to_owned)),
            vec![day + 8 * 60],
            "only the message he typed, which happens to open with From"
        );
    }
}
