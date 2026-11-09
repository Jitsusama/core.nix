//! A process and every process under it, as one: `tree:<pid>`, read only
//! for the pids a client names, never for every process. A program's
//! drawer watches its own process's tree, so what it started (a
//! terminal's shells, a browser's tabs) counts as its own: the share of
//! the whole processor they used between two reads, the memory they hold,
//! and how many processes and threads they are, and which: every pid in
//! it, so a drawer summing several trees counts once a process two hold,
//! as one window's is when it was started from another's.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use crate::{float, rounded, since, using::Counters};

/// The pid a `tree:` key names.
pub fn pid_of(key: &str) -> Option<u32> {
    key.strip_prefix("tree:")?.parse().ok()
}

/// The pid and every pid under it, by each process's parent.
fn under(pid: u32, children: &BTreeMap<u32, Vec<u32>>) -> Vec<u32> {
    let mut found = vec![pid];
    let mut next = 0;
    while let Some(&at) = found.get(next) {
        if let Some(below) = children.get(&at) {
            found.extend(below.iter().filter(|child| **child != at));
        }
        next += 1;
    }
    found
}

/// A record for each of these pids still running. A process under one
/// that started since the read before counts all its time, since it all
/// fell between the two; with no read before, the share is since boot.
pub fn records(
    pids: &BTreeSet<u32>,
    now: &Counters,
    before: Option<&Counters>,
    page_b: u64,
) -> Vec<Value> {
    let mut children: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (pid, process) in &now.processes {
        children.entry(process.parent).or_default().push(*pid);
    }
    let all = now.ticks.saturating_sub(before.map_or(0, |b| b.ticks));
    let over_ms = before.map(|b| now.at_ms.saturating_sub(b.at_ms));
    let mut records = Vec::new();
    for pid in pids {
        let Some(root) = now.processes.get(pid) else {
            continue;
        };
        let tree = under(*pid, &children);
        let (mut used, mut threads, mut pages) = (0, 0, 0);
        for at in &tree {
            let Some(process) = now.processes.get(at) else {
                continue;
            };
            let then = before
                .and_then(|b| b.processes.get(at))
                .filter(|then| then.started == process.started)
                .map_or(0, |then| then.ticks);
            used += process.ticks.saturating_sub(then);
            threads += process.threads;
            pages += process.resident_pages;
        }
        let share = (all > 0).then(|| rounded(float(used) / float(all), 4));
        let mut held = tree.clone();
        held.sort_unstable();
        records.push(json!({
            "key": format!("tree:{pid}"),
            "kind": "tree",
            "pid": pid,
            "name": root.name,
            "share_ratio": share,
            "resident_b": pages * page_b,
            "processes": tree.len(),
            "pids": held,
            "threads": threads,
            "over_ms": over_ms,
            "since": since(over_ms),
        }));
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::using::Process;

    fn process(name: &str, parent: u32, started: u64, ticks: u64) -> Process {
        Process {
            name: name.to_owned(),
            parent,
            started,
            ticks,
            threads: 2,
            resident_pages: 10,
        }
    }

    fn read(at_ms: u64, ticks: u64, processes: &[(u32, Process)]) -> Counters {
        Counters {
            at_ms,
            ticks,
            processes: processes.iter().cloned().collect(),
        }
    }

    #[test]
    fn a_tree_is_its_process_and_every_one_under_it() {
        let before = read(
            0,
            1000,
            &[
                (1, process("systemd", 0, 1, 5)),
                (10, process("kitty", 1, 50, 100)),
                (11, process("zsh", 10, 60, 20)),
            ],
        );
        let now = read(
            1000,
            1400,
            &[
                (1, process("systemd", 0, 1, 5)),
                (10, process("kitty", 1, 50, 140)),
                (11, process("zsh", 10, 60, 40)),
                (12, process("cargo", 11, 900, 100)),
                (20, process("niri", 1, 40, 300)),
            ],
        );
        let said = records(&BTreeSet::from([10, 99]), &now, Some(&before), 4096);
        assert_eq!(said.len(), 1, "a pid gone has no record: {said:?}");
        let tree = &said[0];
        assert_eq!(tree["key"], "tree:10");
        assert_eq!(tree["name"], "kitty");
        assert_eq!(tree["processes"], json!(3));
        assert_eq!(tree["pids"], json!([10, 11, 12]));
        assert_eq!(tree["threads"], json!(6));
        assert_eq!(tree["resident_b"], json!(30 * 4096));
        // 40 + 20 + all 100 of the process started since, of 400 ticks.
        assert_eq!(tree["share_ratio"], json!(0.4));
        assert_eq!(tree["since"], "now");
    }

    #[test]
    fn a_pid_given_again_counts_from_its_own_start() {
        let before = read(0, 1000, &[(10, process("kitty", 1, 50, 900))]);
        let now = read(1000, 1100, &[(10, process("foot", 1, 950, 30))]);
        let said = records(&BTreeSet::from([10]), &now, Some(&before), 4096);
        assert_eq!(said[0]["share_ratio"], json!(0.3));
    }

    #[test]
    fn a_first_read_is_since_boot() {
        let now = read(1000, 1000, &[(10, process("kitty", 1, 50, 100))]);
        let said = records(&BTreeSet::from([10]), &now, None, 4096);
        assert_eq!(said[0]["share_ratio"], json!(0.1));
        assert_eq!(said[0]["since"], "boot");
    }
}
