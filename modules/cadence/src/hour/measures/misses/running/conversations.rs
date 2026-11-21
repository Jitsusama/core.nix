//! Each running conversation's own process, read as a host. Core's host
//! module starts every conversation in a unit of its own,
//! `pi-conversation-KEY@SLOT`, from the same file as the hosts' unit, on
//! the host's program and runtime, and a host's restart stops none of
//! them, so a conversation started before an install or a switch keeps the
//! program, unit and runtime it started with however often its host
//! restarts.

use std::path::PathBuf;

use super::hosts::{runtime, shown_host};
use super::{Listed, Standing};
use crate::command;

/// Each running conversation as a host: the runtime its process loads its
/// pi from, and when that process began; `None` when systemd can't list
/// them.
pub fn conversations() -> Option<Vec<Listed>> {
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
        "MainPID",
        "-p",
        "ExecMainStartTimestamp",
        "-p",
        "ActiveEnterTimestamp",
    ];
    args.extend(&running);
    let shown = command("systemctl", &args).ok()?;
    Some(started(&shown, running.len(), |pid| {
        std::fs::read(format!("/proc/{pid}/environ"))
            .ok()
            .and_then(|environ| runtime(&environ))
    }))
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
/// one to each unit asked, its runtime as `runtime_of` reads its process;
/// when the blocks don't match the units, each is unread rather than given
/// to the wrong unit.
fn started(shown: &str, count: usize, runtime_of: impl Fn(u32) -> Option<PathBuf>) -> Vec<Listed> {
    let blocks: Vec<&str> = shown
        .split("\n\n")
        .filter(|block| !block.trim().is_empty())
        .collect();
    let read = blocks.len() == count;
    (0..count)
        .map(|at| {
            let (pid, started) = blocks
                .get(at)
                .filter(|_| read)
                .map_or((None, None), |block| shown_host(block));
            (pid.and_then(&runtime_of), started, Standing::Running)
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
        // A conversation's unit is active once its process execs, so both
        // times read the same second.
        let shown = concat!(
            "MainPID=41\nExecMainStartTimestamp=@1795256700\nActiveEnterTimestamp=@1795256700\n\n",
            "MainPID=0\nExecMainStartTimestamp=\nActiveEnterTimestamp=\n",
        );
        let runtime_of = |pid: u32| Some(PathBuf::from(format!("/nix/store/runtime-{pid}")));
        assert_eq!(
            started(shown, 2, runtime_of),
            [
                (
                    Some(PathBuf::from("/nix/store/runtime-41")),
                    Some(1_795_256_700),
                    Standing::Running
                ),
                (None, None, Standing::Running),
            ],
            "each conversation's start and runtime weren't read from its own block and \
             process, or one active the second it began read as never ready"
        );
        assert_eq!(
            started(shown, 3, runtime_of),
            vec![(None, None, Standing::Running); 3],
            "blocks not matching the units asked were given to them"
        );
    }
}
