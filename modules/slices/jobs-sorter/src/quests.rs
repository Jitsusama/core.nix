//! Work for a quest goes in a slice of its own, weighted by the quest.
//!
//! pi's commands, and the subagents it starts, carry `PI_QUEST`, the id of
//! the quest they work for, and every pi session keeps a file that maps
//! each open quest's id to a cgroup weight by its priority. Under a slice
//! named by `--by-quest`, a piece whose root names a quest the file lists
//! gets its scope in that quest's slice beside the others,
//! jobs-work-qest20261024abc123.slice for QEST-20261024-ABC123 under
//! jobs-work.slice, the id folded to letters and digits since systemd
//! reads a dash in a slice's name as nesting. The slice takes the quest's
//! weight for CPU and disk alike. A weight is a share, never a cap: a
//! quest alone still has all of what its parent slice gets, and only
//! shares it by weight when other quests want the same at once.
//!
//! The disk's share acts only where the disk is shared by io.cost or the
//! BFQ scheduler. A disk run on io.latency targets with no scheduler, as
//! optimus's NVMe drive is, orders whole slices by latency but not one
//! quest over another, so there the weight waits for io.cost.
//!
//! Work naming no quest, or one the file doesn't list, stays where it
//! went before. The file is read again whenever it changes, and a quest
//! whose weight changed is weighed again.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

/// The environment variable that names a piece of work's quest.
const QUEST_ENV: &[u8] = b"PI_QUEST=";

/// The weights cgroup v2 accepts.
const WEIGHTS: std::ops::RangeInclusive<u64> = 1..=10_000;

/// Reads the weights file: one flat JSON object of quest ids to whole
/// numbers, as the harness writes it. Anything else is no file at all,
/// so a half-understood one never weighs work wrongly.
pub fn parse(text: &str) -> Option<HashMap<String, u64>> {
    let mut rest = text.trim_start().strip_prefix('{')?.trim_start();
    let mut weights = HashMap::new();
    if let Some(after) = rest.strip_prefix('}') {
        return after.trim().is_empty().then_some(weights);
    }
    loop {
        let (key, after) = rest.strip_prefix('"')?.split_once('"')?;
        if key.contains('\\') {
            return None;
        }
        let after = after.trim_start().strip_prefix(':')?.trim_start();
        let digits = after.len() - after.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        let weight: u64 = after[..digits].parse().ok()?;
        weights.insert(key.to_string(), weight);
        let after = after[digits..].trim_start();
        if let Some(end) = after.strip_prefix('}') {
            return end.trim().is_empty().then_some(weights);
        }
        rest = after.strip_prefix(',')?.trim_start();
    }
}

/// The quest a process's environment, as /proc/<pid>/environ holds it,
/// names.
pub fn quest_in(environ: &[u8]) -> Option<&str> {
    environ
        .split(|byte| *byte == 0)
        .find_map(|pair| pair.strip_prefix(QUEST_ENV))
        .and_then(|quest| std::str::from_utf8(quest).ok())
        .filter(|quest| !quest.is_empty())
}

/// The slice a quest's work goes in under `base`, or `None` for an id
/// that wouldn't make a slice name of its own.
pub fn slice_for(base: &str, quest: &str) -> Option<String> {
    let base = base.strip_suffix(".slice")?;
    if !quest.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }
    let folded: String = quest
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    (!folded.is_empty()).then(|| format!("{base}-{folded}.slice"))
}

/// What busctl is given to weigh `slice` at `weight` for CPU and disk, as
/// a runtime property, so it holds while the slice comes and goes until
/// the next boot.
pub fn weigh_call(slice: &str, weight: u64) -> Vec<String> {
    let weight = weight.to_string();
    [
        "--user",
        "--quiet",
        "call",
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
        "SetUnitProperties",
        "sba(sv)",
        slice,
        "true",
        "2",
        "CPUWeight",
        "t",
        &weight,
        "IOWeight",
        "t",
        &weight,
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect()
}

/// The weights file as last read, and what has been weighed from it.
pub struct Weights {
    path: PathBuf,
    modified: Option<SystemTime>,
    by_quest: HashMap<String, u64>,
    /// Each quest slice's weight as last set.
    weighed: HashMap<String, u64>,
    /// Whether a failure to weigh has been said, apart from the sorter's
    /// other failures, so neither kind hides the other.
    failed: bool,
}

impl Weights {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            modified: None,
            by_quest: HashMap::new(),
            weighed: HashMap::new(),
            failed: false,
        }
    }

    /// Takes a weights file's text as the current weights.
    /// One it can't wholly read leaves the weights as they were: it is
    /// most likely mid-write, and the next change brings the whole one.
    pub fn take(&mut self, text: &str) {
        if let Some(by_quest) = parse(text) {
            self.by_quest = by_quest;
        }
    }

    /// Reads the file again if it changed since it was last read.
    pub fn refresh(&mut self) {
        let modified = std::fs::metadata(&self.path)
            .and_then(|meta| meta.modified())
            .ok();
        if modified.is_none() || modified == self.modified {
            return;
        }
        if let Ok(text) = std::fs::read_to_string(&self.path) {
            self.modified = modified;
            self.take(&text);
        }
    }

    /// Where a piece of work in `base` goes and at what weight, from its
    /// root's environment, or `None` to leave it where it would have gone.
    pub fn place(&self, base: &str, environ: &[u8]) -> Option<(String, u64)> {
        let quest = quest_in(environ)?;
        let weight = *self.by_quest.get(quest)?;
        let slice = slice_for(base, quest)?;
        Some((slice, weight.clamp(*WEIGHTS.start(), *WEIGHTS.end())))
    }

    /// Each quest slice this has weighed, with its weight by the file now.
    pub fn current(&self, base: &str) -> Vec<(String, u64)> {
        self.by_quest
            .iter()
            .filter_map(|(quest, weight)| {
                let slice = slice_for(base, quest)?;
                self.weighed
                    .contains_key(&slice)
                    .then(|| (slice, (*weight).clamp(*WEIGHTS.start(), *WEIGHTS.end())))
            })
            .collect()
    }

    /// The slices whose weight differs from what was last set, each with
    /// the weight to set, for a quest slice that has work.
    pub fn unweighed(&self, slices: &[(String, u64)]) -> Vec<(String, u64)> {
        slices
            .iter()
            .filter(|(slice, weight)| self.weighed.get(slice) != Some(weight))
            .cloned()
            .collect()
    }

    /// Notes a slice as weighed at `weight`.
    pub fn weighed(&mut self, slice: &str, weight: u64) {
        self.weighed.insert(slice.to_string(), weight);
    }

    /// Whether this is the first failure to weigh, and so worth saying.
    pub fn first_failure(&mut self) -> bool {
        !std::mem::replace(&mut self.failed, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "{\n\t\"QEST-20261023-R82FBP\": 200,\n\t\"QEST-20261024-ABC123\": 400\n}\n";

    #[test]
    fn a_weighing_failure_is_said_once_whatever_else_failed() {
        let mut weights = Weights::new(PathBuf::from("/nonexistent"));
        assert!(weights.first_failure());
        assert!(!weights.first_failure());
    }

    #[test]
    fn the_harness_file_is_read() {
        let weights = parse(FILE).unwrap();
        assert_eq!(weights.len(), 2);
        assert_eq!(weights["QEST-20261023-R82FBP"], 200);
        assert_eq!(weights["QEST-20261024-ABC123"], 400);
        assert_eq!(parse("{}").unwrap().len(), 0);
    }

    #[test]
    fn a_file_it_cannot_wholly_read_is_no_file() {
        for text in [
            "",
            "[]",
            "{\"QEST-1\": \"200\"}",
            "{\"QEST-1\": 2.5}",
            "{\"QEST-1\": -4}",
            "{\"QEST-1\": 200,}",
            "{\"QEST-1\": 200} trailing",
            "{\"QE\\\"ST\": 200}",
            "{\"QEST-1\": 200",
        ] {
            assert_eq!(parse(text), None, "{text}");
        }
    }

    #[test]
    fn the_quest_is_found_among_the_environment() {
        let environ = b"PATH=/bin\0PI_QUEST=QEST-20261024-ABC123\0TMPDIR=/tmp/x\0";
        assert_eq!(quest_in(environ), Some("QEST-20261024-ABC123"));
        assert_eq!(quest_in(b"PATH=/bin\0NOT_PI_QUEST=QEST-1\0"), None);
        assert_eq!(quest_in(b"PI_QUEST=\0"), None);
        assert_eq!(quest_in(b""), None);
    }

    #[test]
    fn a_quest_slice_sits_under_its_base_with_the_id_folded() {
        assert_eq!(
            slice_for("jobs-work.slice", "QEST-20261024-ABC123").as_deref(),
            Some("jobs-work-qest20261024abc123.slice")
        );
        // Anything but letters, digits and dashes would make another name.
        assert_eq!(slice_for("jobs-work.slice", "QEST-1/../x"), None);
        assert_eq!(slice_for("jobs-work.slice", "QEST-1.slice"), None);
        assert_eq!(slice_for("jobs-work.slice", "---"), None);
        assert_eq!(slice_for("jobs-work", "QEST-1"), None);
    }

    #[test]
    fn a_slice_is_weighed_for_cpu_and_disk_until_boot() {
        let args = weigh_call("jobs-work-qest1.slice", 400);
        let tail: Vec<&str> = args[6..].iter().map(String::as_str).collect();
        assert_eq!(
            tail,
            vec![
                "SetUnitProperties",
                "sba(sv)",
                "jobs-work-qest1.slice",
                "true",
                "2",
                "CPUWeight",
                "t",
                "400",
                "IOWeight",
                "t",
                "400",
            ]
        );
        assert_eq!(&args[..3], ["--user", "--quiet", "call"]);
    }

    #[test]
    fn work_for_a_listed_quest_goes_in_its_slice_at_its_weight() {
        let mut weights = Weights::new(PathBuf::new());
        weights.take(FILE);
        assert_eq!(
            weights.place("jobs-work.slice", b"PI_QUEST=QEST-20261024-ABC123\0"),
            Some(("jobs-work-qest20261024abc123.slice".into(), 400))
        );
    }

    #[test]
    fn work_for_no_quest_or_an_unlisted_one_stays_put() {
        let mut weights = Weights::new(PathBuf::new());
        weights.take(FILE);
        assert_eq!(weights.place("jobs-work.slice", b"PATH=/bin\0"), None);
        assert_eq!(
            weights.place("jobs-work.slice", b"PI_QUEST=QEST-20260101-GONE00\0"),
            None
        );
    }

    #[test]
    fn a_weight_past_what_the_kernel_takes_is_held_to_it() {
        let mut weights = Weights::new(PathBuf::new());
        weights.take("{\"QEST-A\": 0, \"QEST-B\": 99999}");
        assert_eq!(
            weights
                .place("jobs-work.slice", b"PI_QUEST=QEST-A\0")
                .map(|(_, w)| w),
            Some(1)
        );
        assert_eq!(
            weights
                .place("jobs-work.slice", b"PI_QUEST=QEST-B\0")
                .map(|(_, w)| w),
            Some(10_000)
        );
    }

    #[test]
    fn a_file_it_cannot_read_keeps_the_weights_it_had() {
        // A half-written file would otherwise put every quest's work back
        // in the shared slice for a look.
        let mut weights = Weights::new(PathBuf::new());
        weights.take(FILE);
        weights.take("{\"QEST-20261024-ABC1");
        assert!(
            weights
                .place("jobs-work.slice", b"PI_QUEST=QEST-20261024-ABC123\0")
                .is_some()
        );
    }

    #[test]
    fn a_slice_is_weighed_once_and_again_when_its_weight_changes() {
        let mut weights = Weights::new(PathBuf::new());
        let slices = vec![("jobs-work-qesta.slice".to_string(), 200)];
        assert_eq!(weights.unweighed(&slices), slices);
        weights.weighed("jobs-work-qesta.slice", 200);
        assert!(weights.unweighed(&slices).is_empty());
        let raised = vec![("jobs-work-qesta.slice".to_string(), 400)];
        assert_eq!(weights.unweighed(&raised), raised);
    }
}
