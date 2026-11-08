//! Whether memory is stalling now, so another evaluation can wait for it.
//!
//! A run's evaluations each hold a few gigabytes that Nix never gives
//! back. Two of them at once on 29 October, at 3.6 and 3.9 GB, came
//! while memory was already full, and Joel's launcher stalled with them.
//! The machine queue gives the run its turn by the memory free when it
//! starts, but nothing it starts inside that turn waits; so the run waits
//! itself before each further evaluation, by the stall the kernel reports
//! rather than by a count of evaluations.

use std::fs;
use std::path::{Path, PathBuf};

/// The stall, as PSI's "some" over ten seconds, in percent, at which no
/// further evaluation begins: the machine queue's own limit for starting
/// work (`MOST_MEMORY_PRESSURE` in the harness's queue).
pub const STALLED_AT: f64 = 10.0;

/// Whether memory stalls now, on the whole machine or in the group of
/// work this process runs in, where what it starts adds its share. A
/// reading that can't be had is no stall, as on a machine without PSI.
/// `CHANGED_CHECKS_PRESSURE` names the one file read in their place, so
/// a test stands apart from the machine it runs on: read there, a
/// loaded machine's stall held a test's second evaluation past its ten
/// seconds on 3 November.
pub fn memory_stalled() -> bool {
    let read = match std::env::var_os(PRESSURE) {
        Some(file) => fs::read_to_string(file).into_iter().collect(),
        None => readings(Path::new("/proc"), Path::new("/sys/fs/cgroup")),
    };
    read.iter().any(|text| stalled(text))
}

/// What names the pressure file read in place of the machine's.
const PRESSURE: &str = "CHANGED_CHECKS_PRESSURE";

/// The pressure files to read: the machine's, and the one of the parent of
/// this process's own cgroup.
fn readings(proc: &Path, cgroups: &Path) -> Vec<String> {
    let mut files = vec![proc.join("pressure").join("memory")];
    if let Some(parent) = own_cgroup(proc).and_then(|own| own.parent().map(Path::to_path_buf)) {
        files.push(
            cgroups
                .join(parent.strip_prefix("/").unwrap_or(&parent))
                .join("memory.pressure"),
        );
    }
    files
        .iter()
        .filter_map(|file| fs::read_to_string(file).ok())
        .collect()
}

/// This process's cgroup, as `/proc/self/cgroup`'s unified line names it.
fn own_cgroup(proc: &Path) -> Option<PathBuf> {
    let text = fs::read_to_string(proc.join("self").join("cgroup")).ok()?;
    text.lines()
        .find_map(|line| line.strip_prefix("0::"))
        .map(PathBuf::from)
}

/// Whether a pressure file's "some" line says ten seconds' stall at or past
/// [`STALLED_AT`].
pub fn stalled(text: &str) -> bool {
    some(text, "avg10=").is_some_and(|stall| stall >= STALLED_AT)
}

/// The CPU stall, as PSI's "some" over a minute, in percent, at or past
/// which the machine isn't idle: the person's work stays ahead of a run
/// that no builder took. On 7 November the net's evaluations and a
/// session's, run here while the builders didn't answer, held the machine
/// near 70% with Joel at it.
pub const BUSY_AT: f64 = 15.0;

/// Whether the machine's CPU is busy now. A reading that can't be had is
/// idle, as on a machine without PSI.
pub fn cpu_busy() -> bool {
    let file =
        std::env::var_os(CPU).map_or_else(|| PathBuf::from("/proc/pressure/cpu"), PathBuf::from);
    busy(&fs::read_to_string(file).unwrap_or_default())
}

/// Whether a run nowhere else took waits rather than run here: not asked
/// to run here, and the machine busy.
pub fn waits_for_idle(here: bool, busy: bool) -> bool {
    !here && busy
}

/// What names the CPU pressure file read in place of the machine's.
const CPU: &str = "CHANGED_CHECKS_CPU_PRESSURE";

/// Whether a CPU pressure file's "some" line says a minute's stall at or
/// past [`BUSY_AT`].
pub fn busy(text: &str) -> bool {
    some(text, "avg60=").is_some_and(|stall| stall >= BUSY_AT)
}

/// The "some" line's value of `field`, such as `avg10=`.
fn some(text: &str, field: &str) -> Option<f64> {
    text.lines()
        .find(|line| line.starts_with("some "))
        .and_then(|line| {
            line.split_whitespace()
                .find_map(|word| word.strip_prefix(field))
                .and_then(|value| value.parse::<f64>().ok())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CALM: &str = "some avg10=1.25 avg60=0.40 avg300=0.10 total=12\nfull avg10=0.00 avg60=0.00 avg300=0.00 total=3\n";
    // As a workstation read at 12:23 on 29 October, while the launcher stalled.
    const PRESSED: &str = "some avg10=25.10 avg60=18.02 avg300=6.44 total=98123\nfull avg10=15.03 avg60=9.11 avg300=3.20 total=50123\n";

    #[test]
    fn a_stall_past_the_queues_limit_is_one() {
        assert!(stalled(PRESSED));
        assert!(!stalled(CALM));
        assert!(stalled("some avg10=10.00 avg60=0 avg300=0 total=1\n"));
    }

    #[test]
    fn a_minutes_cpu_stall_past_fifteen_is_busy() {
        assert!(busy("some avg10=3.00 avg60=15.00 avg300=40.00 total=1\n"));
        assert!(!busy("some avg10=80.00 avg60=14.90 avg300=40.00 total=1\n"));
        assert!(!busy(""), "no reading is idle");
    }

    #[test]
    fn a_run_no_builder_took_waits_while_the_machine_is_busy() {
        assert!(waits_for_idle(false, true));
        assert!(!waits_for_idle(false, false), "an idle machine runs it");
        assert!(!waits_for_idle(true, true), "--here runs it anyway");
    }

    #[test]
    fn what_cant_be_read_is_no_stall() {
        assert!(!stalled(""));
        assert!(!stalled("full avg10=99.00 avg60=0 avg300=0 total=1\n"));
        assert!(!stalled("some avg10=lots\n"));
    }

    #[test]
    fn the_work_groups_pressure_is_read_beside_the_machines() {
        let root = crate::scratch::Scratch::new("pressure");
        let proc = root.join("proc");
        let cgroups = root.join("cgroups");
        let group = cgroups.join("user.slice/jobs.slice/jobs-work.slice");
        fs::create_dir_all(proc.join("pressure")).unwrap();
        fs::create_dir_all(proc.join("self")).unwrap();
        fs::create_dir_all(&group).unwrap();
        fs::write(proc.join("pressure/memory"), CALM).unwrap();
        fs::write(
            proc.join("self/cgroup"),
            "0::/user.slice/jobs.slice/jobs-work.slice/work-7.scope\n",
        )
        .unwrap();
        fs::write(group.join("memory.pressure"), PRESSED).unwrap();

        let read = readings(&proc, &cgroups);

        assert_eq!(read, [CALM, PRESSED]);
        assert!(read.iter().any(|text| stalled(text)));
    }
}
