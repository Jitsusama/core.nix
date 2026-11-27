//! How a check's build over the base ends: built against red first's
//! budget, and judged from its exit and nix's log.

use std::fs;
use std::io::Read;
use std::process::Stdio;
use std::time::{Duration, Instant};

use crate::building;
use crate::checks::Derivations;

/// How long red first's builds over the base may take in all. A test
/// written for the change may wait on what only the change does, so over
/// the base it may never end; and red first runs before the run passes,
/// so a build with no end would hold a correct change's gate for good.
/// Ten minutes is half the twenty a change's gate aims to take, for a step
/// that only says what it finds.
const BUDGET: Duration = Duration::from_secs(600);

/// The variable setting that budget in seconds instead, as a case does.
const BUDGET_SET: &str = "CHANGED_CHECKS_RED_FIRST_SECONDS";

/// How often a build over the base is looked at to see whether it ended:
/// often enough that the budget ends close to its deadline.
const LOOKED: Duration = Duration::from_millis(200);

/// How a check's build over the base ended, from its exit and nix's log.
#[derive(Debug, PartialEq, Eq)]
pub enum Built {
    /// It built, so its tests pass there.
    Passed,
    /// A builder ran it and it failed, as a test failing does.
    Red,
    /// It failed without a builder failing, as when one is lost, a
    /// download fails or the disk fills, which says nothing of its tests.
    Unbuilt,
    /// It was still building when the budget ran out, so it was stopped,
    /// which says nothing of its tests either.
    Late,
}

/// How long red first's builds over the base may take in all, as
/// `CHANGED_CHECKS_RED_FIRST_SECONDS` sets it, or ten minutes.
pub fn budget() -> Duration {
    std::env::var(BUDGET_SET)
        .ok()
        .and_then(|seconds| seconds.parse().ok())
        .map_or(BUDGET, Duration::from_secs)
}

/// How a build over the base that `passed` or not, saying `log`, ended.
/// Only nix's own lines say a builder failed it, not a builder's, which
/// `-L` gives as `name> ...` and a test may fill with any words.
pub fn built(passed: bool, log: &str) -> Built {
    let builders = |line: &str| {
        line.split_once("> ")
            .is_some_and(|(name, _)| !name.is_empty() && !name.contains(char::is_whitespace))
    };
    if passed {
        Built::Passed
    } else if log
        .lines()
        .any(|line| !builders(line) && line.contains("failed with exit code"))
    {
        Built::Red
    } else {
        Built::Unbuilt
    }
}

/// Build the check `name` over the base, stopping it at `deadline` and
/// keeping nix's log out of the run's: a test failing there as it should
/// prints nix's words for a failed build, which merge-net reads as a check
/// of the change failing. The log is kept in the cache, and named when it
/// says nothing of the tests.
pub fn built_over(
    flake: &str,
    system: &str,
    name: &str,
    on_base: &Derivations,
    deadline: Instant,
) -> Result<Built, String> {
    let mut nix = building::building_here(flake, system, &[name], on_base)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("running nix: {error}"))?;
    let mut said = nix
        .stderr
        .take()
        .ok_or_else(|| "nix's log wasn't piped".to_owned())?;
    // Read beside the build, so a long log can't fill the pipe and stall
    // nix while it's waited on.
    let reading = std::thread::spawn(move || {
        let mut log = Vec::new();
        let _ = said.read_to_end(&mut log);
        log
    });
    let passed = loop {
        if let Some(ended) = nix
            .try_wait()
            .map_err(|error| format!("waiting on nix: {error}"))?
        {
            break ended.success();
        }
        if Instant::now() >= deadline {
            // nix has its daemon build, which stops a build once the one
            // asking for it is gone. Its log isn't waited for, since a
            // builder left behind may hold the pipe open.
            let _ = nix.kill();
            let _ = nix.wait();
            return Ok(Built::Late);
        }
        std::thread::sleep(LOOKED);
    };
    let log = reading.join().unwrap_or_default();
    let log = String::from_utf8_lossy(&log);
    let ended = built(passed, &log);
    if ended == Built::Unbuilt {
        let folder = crate::cache().join("changed-checks");
        // Runs judging the same check at once each keep their own.
        let kept = folder.join(format!("red-first-{}-{name}.log", std::process::id()));
        let _ = fs::create_dir_all(&folder).and_then(|()| fs::write(&kept, &*log));
        eprintln!(
            "changed-checks: {name} couldn't be built over the base, though no builder failed \
             it (nix's words are in {}), so its tests weren't asked to fail there",
            kept.display()
        );
    }
    Ok(ended)
}
