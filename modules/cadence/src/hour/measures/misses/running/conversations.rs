//! Each running conversation's own process, read as a host for the harness
//! program it runs. Core's host module starts every conversation in a unit
//! of its own, `pi-conversation-KEY@SLOT`, on the host's program, and a
//! host's restart stops none of them, so a conversation started before an
//! install keeps the program it started with however often its host
//! restarts.

use super::{Host, shown_host};
use crate::command;

/// Each running conversation as a host started when its unit last became
/// active; `None` when systemd can't list them.
pub fn conversations() -> Option<Vec<Host>> {
    let units = command(
        "systemctl",
        &[
            "--user",
            "list-units",
            "--type=service",
            "--no-legend",
            "--plain",
            "pi-conversation-*",
        ],
    )
    .ok()?;
    let running = listed(&units);
    if running.is_empty() {
        return Some(Vec::new());
    }
    let mut args = vec![
        "--user",
        "show",
        "--timestamp=unix",
        "-p",
        "ActiveEnterTimestamp",
    ];
    args.extend(&running);
    let shown = command("systemctl", &args).ok()?;
    Some(started(&shown, running.len()))
}

/// The conversations' units a `systemctl list-units` lists as running: a
/// failed one runs nothing, and one starting loads the program as it does.
fn listed(units: &str) -> Vec<&str> {
    units
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let unit = fields
                .next()
                .filter(|unit| unit.starts_with("pi-conversation-") && unit.contains('@'))?;
            let active = fields.nth(1)?;
            (active != "inactive" && active != "failed").then_some(unit)
        })
        .collect()
}

/// Each of `count` conversations as a host, from `systemctl show`'s blocks,
/// one to each unit asked; when the blocks don't match the units, each
/// start is unread rather than given to the wrong unit.
fn started(shown: &str, count: usize) -> Vec<Host> {
    let blocks: Vec<&str> = shown
        .split("\n\n")
        .filter(|block| !block.trim().is_empty())
        .collect();
    let read = blocks.len() == count;
    (0..count)
        .map(|at| Host {
            started: blocks
                .get(at)
                .filter(|_| read)
                .and_then(|block| shown_host(block).1),
            ..Host::default()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_running_conversation_is_read_as_a_host_by_when_it_started() {
        let units = concat!(
            "pi-conversation-work-attended@01a1.service loaded active running Holds one\n",
            "pi-conversation-work-background@01a2.service loaded activating start Holds\n",
            "pi-conversation-work-background@01a3.service loaded failed failed Holds one\n",
            "pi-conversation-work-background@01a4.service loaded inactive dead Holds one\n",
            "pi-host-work-attended.service loaded active running Not one\n",
        );
        assert_eq!(
            listed(units),
            [
                "pi-conversation-work-attended@01a1.service",
                "pi-conversation-work-background@01a2.service"
            ],
            "a failed or stopped conversation, or a host, was read as one running"
        );
        let shown = "ActiveEnterTimestamp=@1795256762\n\nActiveEnterTimestamp=\n";
        let starts = |hosts: Vec<Host>| hosts.iter().map(|host| host.started).collect::<Vec<_>>();
        assert_eq!(
            starts(started(shown, 2)),
            [Some(1_795_256_762), None],
            "each conversation's start wasn't read from its own block"
        );
        assert_eq!(
            starts(started(shown, 3)),
            [None, None, None],
            "blocks not matching the units asked were given to them"
        );
    }
}
