//! The programs using the machine most, by their place rather than their
//! name, so the first stays first as programs change: `using:cpu:<rank>`,
//! each program's share of the whole processor between two reads, and
//! `using:memory:<rank>`, the memory each program's processes hold. A
//! program is every process of one name, as the kernel names them.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::{Root, float, processor::TIMES, rounded};

/// How many programs are named: as many as the launcher's card shows at
/// once. Three left Joel asking what came after (2026-10-25).
pub const TOP: usize = 8;
/// The longest a process's name is kept by the kernel.
const NAME_KEPT: usize = 15;
const WRAPPED: &str = "-wrapped";
/// The fields of a process's stat after its name: its state is the
/// first, its user and system time the 12th and 13th, its resident
/// pages the 22nd.
const USER_TIME: usize = 11;
const SYSTEM_TIME: usize = 12;
const RESIDENT: usize = 21;

/// A program by the name it's known by: Nix runs a wrapped program as
/// ".kitty-wrapped", cut short as ".firefox-wrappe" when the name is
/// long.
pub fn program(name: &str) -> String {
    let Some(bare) = name.strip_prefix('.') else {
        return name.to_owned();
    };
    for cut in (1..=WRAPPED.len()).rev() {
        let end = &WRAPPED[..cut];
        let whole = cut == WRAPPED.len() || name.len() == NAME_KEPT;
        if whole && bare.len() > end.len() && bare.ends_with(end) {
            return bare[..bare.len() - end.len()].to_owned();
        }
    }
    bare.to_owned()
}

/// One process, as a read found it.
#[derive(Clone, Debug, PartialEq)]
pub struct Process {
    pub name: String,
    pub ticks: u64,
    pub resident_pages: u64,
}

/// One read of every process, and the whole machine's ticks then.
#[derive(Clone, Debug, PartialEq)]
pub struct Counters {
    pub ticks: u64,
    pub processes: BTreeMap<u32, Process>,
}

/// A process's line of `/proc/<pid>/stat`, its name in brackets that may
/// hold spaces or brackets of its own.
fn process(line: &str) -> Option<Process> {
    let open = line.find(" (")?;
    let close = line.rfind(')')?;
    let name = program(line.get(open + 2..close)?);
    let fields: Vec<&str> = line.get(close + 2..)?.split(' ').collect();
    let at = |i: usize| fields.get(i).and_then(|f| f.parse::<u64>().ok());
    Some(Process {
        name,
        ticks: at(USER_TIME)? + at(SYSTEM_TIME)?,
        resident_pages: at(RESIDENT)?,
    })
}

pub fn counters(root: &Root) -> Counters {
    let stat = root.read("/proc/stat").unwrap_or_default();
    let ticks = stat
        .lines()
        .find_map(|line| line.strip_prefix("cpu "))
        .map_or(0, |fields| {
            fields
                .split_whitespace()
                .take(TIMES.len() - 2)
                .filter_map(|f| f.parse::<u64>().ok())
                .sum()
        });
    let processes = root
        .names("/proc")
        .iter()
        .filter_map(|name| name.parse::<u32>().ok())
        .filter_map(|pid| Some((pid, process(&root.read(&format!("/proc/{pid}/stat"))?)?)))
        .collect();
    Counters { ticks, processes }
}

/// The most of each, by name, at most [`TOP`], none at nothing.
fn top(values: BTreeMap<String, (u64, u64)>) -> Vec<(String, u64, u64)> {
    let mut ranked: Vec<(String, u64, u64)> = values
        .into_iter()
        .map(|(name, (value, count))| (name, value, count))
        .filter(|(_, value, _)| *value > 0)
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked.truncate(TOP);
    ranked
}

/// The records, ranked: the programs that used the processor most between
/// two reads, a process that started between them left out since its time
/// before can't be told, or since boot with no read before; and those
/// holding the most memory now.
pub fn records(now: &Counters, before: Option<&Counters>, page_b: u64) -> Vec<Value> {
    let all = now.ticks.saturating_sub(before.map_or(0, |b| b.ticks));
    let mut ticks: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    let mut held: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for (pid, process) in &now.processes {
        let entry = held.entry(process.name.clone()).or_default();
        entry.0 += process.resident_pages * page_b;
        entry.1 += 1;
        let then = match before {
            None => Some(0),
            Some(before) => before
                .processes
                .get(pid)
                .filter(|then| then.name == process.name)
                .map(|then| then.ticks),
        };
        if let Some(then) = then {
            let entry = ticks.entry(process.name.clone()).or_default();
            entry.0 += process.ticks.saturating_sub(then);
            entry.1 += 1;
        }
    }
    let mut records = Vec::new();
    if all > 0 {
        for (rank, (name, used, count)) in top(ticks).into_iter().enumerate() {
            let share = rounded(float(used) / float(all), 4);
            records.push(json!({
                "key": format!("using:cpu:{}", rank + 1),
                "kind": "using",
                "of": "cpu",
                "rank": rank + 1,
                "name": name,
                "share_ratio": share,
                "processes": count,
            }));
        }
    }
    for (rank, (name, bytes, count)) in top(held).into_iter().enumerate() {
        records.push(json!({
            "key": format!("using:memory:{}", rank + 1),
            "kind": "using",
            "of": "memory",
            "rank": rank + 1,
            "name": name,
            "resident_b": bytes,
            "processes": count,
        }));
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wrapped_program_is_known_by_its_name() {
        assert_eq!(program(".kitty-wrapped"), "kitty");
        assert_eq!(program(".firefox-wrappe"), "firefox");
        assert_eq!(program("niri"), "niri");
        assert_eq!(program(".hidden"), "hidden");
    }

    #[test]
    fn a_name_with_spaces_and_brackets_is_read_whole() {
        let line =
            "42 (Web (Content)) S 1 42 42 0 -1 4194560 0 0 0 0 30 12 0 0 20 0 1 0 100 1000 250 0";
        let read = process(line).unwrap();
        assert_eq!(read.name, "Web (Content)");
        assert_eq!(read.ticks, 42);
        assert_eq!(read.resident_pages, 250);
    }
}
