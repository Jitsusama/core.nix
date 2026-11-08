//! How the machine sleeps (`sleep:main`): the ways it can, the one it
//! takes, how each sleep since it started went as the kernel counts them
//! in /sys/power/suspend_stats, and what logind says, as main.rs last
//! asked it: whether the session is idle and since when, what holds sleep
//! or idling off, and when the last sleep began and ended.

use serde_json::{Value, json};

use crate::Root;
use crate::supply::chosen;

const STATS: &str = "/sys/power/suspend_stats";

/// The steps a sleep can fail at, as the kernel names their counts.
const STEPS: [&str; 8] = [
    "freeze",
    "prepare",
    "suspend",
    "suspend_late",
    "suspend_noirq",
    "resume_noirq",
    "resume_early",
    "resume",
];

/// The record, `logind` what logind last said, or null.
pub fn record(root: &Root, logind: &Value) -> Value {
    let stat = |file: &str| root.number(&format!("{STATS}/{file}"));
    let (mode, modes) = chosen(root.said("/sys/power/mem_sleep"));
    let failures: serde_json::Map<String, Value> = STEPS
        .iter()
        .map(|step| ((*step).to_owned(), json!(stat(&format!("failed_{step}")))))
        .collect();
    json!({
        "key": "sleep:main",
        "kind": "sleep",
        "mode": mode,
        "modes": modes,
        "states": root.said("/sys/power/state").map(|s| s.split_whitespace().map(str::to_owned).collect::<Vec<_>>()),
        "successes": stat("success"),
        "failures": stat("fail"),
        "failed_at": failures,
        "last_failed_device": root.said(&format!("{STATS}/last_failed_dev")),
        "last_failed_step": root.said(&format!("{STATS}/last_failed_step")),
        "last_failed_errno": root
            .said(&format!("{STATS}/last_failed_errno"))
            .and_then(|e| e.parse::<i64>().ok())
            .filter(|e| *e != 0),
        "last_hardware_sleep_us": stat("last_hw_sleep"),
        "longest_hardware_sleep_us": stat("max_hw_sleep"),
        "total_hardware_sleep_us": stat("total_hw_sleep"),
        "logind": logind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_is_counted_by_its_step() {
        let root = std::env::temp_dir().join(format!("machine-sleep-{}", std::process::id()));
        let stats = root.join("sys/power/suspend_stats");
        std::fs::create_dir_all(&stats).unwrap();
        std::fs::write(root.join("sys/power/mem_sleep"), "[s2idle] deep\n").unwrap();
        std::fs::write(stats.join("success"), "12\n").unwrap();
        std::fs::write(stats.join("failed_freeze"), "2\n").unwrap();
        std::fs::write(stats.join("last_failed_step"), "freeze\n").unwrap();
        std::fs::write(stats.join("last_failed_errno"), "0\n").unwrap();
        let record = record(&Root::new(&root), &Value::Null);
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(record["mode"], "s2idle");
        assert_eq!(record["modes"], json!(["s2idle", "deep"]));
        assert_eq!(record["successes"], json!(12));
        assert_eq!(record["failed_at"]["freeze"], json!(2));
        assert_eq!(record["failed_at"]["resume"], Value::Null);
        assert_eq!(record["last_failed_step"], "freeze");
        assert_eq!(record["last_failed_errno"], Value::Null, "0 is no error");
    }
}
