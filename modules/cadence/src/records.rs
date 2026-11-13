//! The raw records the card and the watch read, each parsed from the text
//! its source writes, so the parsing is tested without the machine.

use serde::Deserialize;

/// One merge to core's main.
#[derive(Debug, Clone, PartialEq)]
pub struct Merge {
    pub sha: String,
    /// When it merged, in seconds since the epoch.
    pub at: i64,
    /// The pull request it merged, when its subject names one.
    pub pr: Option<u32>,
    /// When its branch's last commit was made, which a change's own checks
    /// and review run from.
    pub last_commit: Option<i64>,
    /// The files it changed against main before it.
    pub files: Vec<String>,
}

impl Merge {
    /// What the card calls it.
    #[must_use]
    pub fn name(&self) -> String {
        self.pr
            .map_or_else(|| self.sha.chars().take(8).collect(), |n| format!("#{n}"))
    }

    /// Whether it changed only tests and docs, which no machine runs.
    #[must_use]
    pub fn tests_only(&self) -> bool {
        !self.files.is_empty() && self.files.iter().all(|f| tests_or_docs(f))
    }

    /// Whether it touched what boots or installs a machine, whose VM tests
    /// run whole before a merge, so its time to merge is measured apart.
    #[must_use]
    pub fn boot(&self) -> bool {
        self.files.iter().any(|f| boot(f))
    }
}

fn tests_or_docs(path: &str) -> bool {
    path.starts_with("tests/")
        || path.starts_with("docs/")
        || std::path::Path::new(path)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"))
        || path == "check-descriptions.nix"
        || path == "checks.nix"
        || path.ends_with("/checks.nix")
        || path.contains("/tests/")
        || path.contains("/test/")
}

fn boot(path: &str) -> bool {
    const AREAS: [&str; 11] = [
        "boot",
        "initrd",
        "kernel",
        "disk",
        "luks",
        "tpm",
        "secure-boot",
        "install",
        "stick",
        "greeter",
        "lock",
    ];
    path.starts_with("hardware/")
        || path == "flake.lock"
        || path.starts_with("modules/nix-offload")
        || path
            .strip_prefix("modules/")
            .is_some_and(|rest| AREAS.iter().any(|area| rest.starts_with(area)))
}

/// Merges from `git log --first-parent --merges --format='%H %ct %P %s'`:
/// the sha, when it merged, its two parents and its subject, with neither
/// the last commit's time nor the files filled in.
#[must_use]
pub fn parse_merges(log: &str) -> Vec<(Merge, String, String)> {
    log.lines()
        .filter_map(|line| {
            let mut words = line.splitn(5, ' ');
            let sha = words.next()?.to_owned();
            let at = words.next()?.parse().ok()?;
            let main = words.next()?.to_owned();
            let branch = words.next()?.to_owned();
            let subject = words.next().unwrap_or_default();
            let pr = subject
                .split_once('#')
                .and_then(|(_, rest)| rest.split(|c: char| !c.is_ascii_digit()).next())
                .and_then(|digits| digits.parse().ok());
            let merge = Merge {
                sha,
                at,
                pr,
                last_commit: None,
                files: Vec::new(),
            };
            Some((merge, main, branch))
        })
        .collect()
}

/// One look the follower took at main: a switch, or a build that came out
/// as what the machine already runs, which puts a merge on as surely.
#[derive(Debug, Clone, PartialEq)]
pub struct Look {
    pub at: i64,
    /// The core it built.
    pub core: String,
    /// Whether it switched the machine, rather than finding it unchanged.
    pub switched: bool,
}

/// The follower's state now, from its journal.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Follower {
    pub looks: Vec<Look>,
    /// When its newest ask began, while it hasn't ended.
    pub going: Option<i64>,
    /// Whether its newest answer put a switch off for a meeting or a call,
    /// a wait chosen rather than a fault.
    pub meeting: bool,
    /// When the first of the updates failing since the last that ended
    /// well failed, and why the newest did, while none has ended well.
    pub failed: Option<(i64, String)>,
    /// When an update was refused for a daemon whose restart waits for a
    /// hold, and why, until an update ends otherwise. The follower asks no
    /// more after one, so only a person's ask with a hold sent moves the
    /// machine on.
    pub held: Option<(i64, String)>,
}

/// How update-to-main's refusal for a daemon whose restart waits for a hold
/// begins, as its update.rs says it (`NEEDS_HOLD`).
pub const NEEDS_HOLD: &str = "switching would change";

/// A line update-to-main writes as it goes, saying the step it reached.
#[derive(Deserialize)]
struct Said {
    step: Option<String>,
    followed: Option<Followed>,
    why: Option<String>,
}

#[derive(Deserialize)]
struct Followed {
    core: Option<String>,
}

/// The follower's looks from `journalctl -u update-to-main-follows -o
/// short-unix`. An ask names the core main stood at, as `asking at
/// [@<zone commit>] core@<sha>`, and the step that ends it names the core it
/// built, which wins: main can move between the two.
#[must_use]
pub fn parse_follower(journal: &str) -> Follower {
    let mut follower = Follower::default();
    let mut asked: Option<(i64, String)> = None;
    for line in journal.lines() {
        let Some((stamp, rest)) = line.split_once(' ') else {
            continue;
        };
        let Ok(at) = stamp.parse::<f64>() else {
            continue;
        };
        #[expect(clippy::cast_possible_truncation, reason = "a time in seconds")]
        let at = at as i64;
        if let Some(core) = rest.split_once(" core@").map(|(_, sha)| sha.trim())
            && rest.contains("asking at")
        {
            asked = Some((at, core.to_owned()));
            follower.going = Some(at);
            continue;
        }
        let Some(brace) = rest.find('{') else {
            continue;
        };
        if rest.contains("a meeting or a call") {
            follower.meeting = true;
        } else if rest[brace..].contains("\"step\"") {
            follower.meeting = false;
        }
        let Ok(step) = serde_json::from_str::<Said>(&rest[brace..]) else {
            continue;
        };
        match step.step.as_deref() {
            Some("done" | "unchanged") => {
                follower.failed = None;
                follower.held = None;
            }
            Some("failed") => {
                let first = follower.failed.as_ref().map_or(at, |(first, _)| *first);
                follower.failed = Some((first, step.why.clone().unwrap_or_default()));
                follower.held = None;
            }
            Some("refused") => {
                if let Some(why) = step.why.as_ref().filter(|why| why.starts_with(NEEDS_HOLD)) {
                    follower.held = Some((at, why.clone()));
                }
            }
            _ => {}
        }
        let ended = matches!(
            step.step.as_deref(),
            Some("done" | "unchanged" | "failed" | "refused")
        );
        if let Some((_, core)) = asked.as_ref().filter(|_| ended) {
            let built = step
                .followed
                .and_then(|f| f.core)
                .unwrap_or_else(|| core.clone());
            match step.step.as_deref() {
                Some("done") => follower.looks.push(Look {
                    at,
                    core: built,
                    switched: true,
                }),
                Some("unchanged") => follower.looks.push(Look {
                    at,
                    core: built,
                    switched: false,
                }),
                _ => {}
            }
            asked = None;
            follower.going = None;
        }
    }
    follower
}

/// Every update's look at main from `journalctl -u update-to-main.service -o
/// short-unix`, whoever asked for it. The service ends each update with a
/// done or unchanged step naming the core it followed, the follower's and a
/// person's alike, while the follower's own journal holds only its asks.
/// On 9 November a switch by hand with a hold given put three merges on at
/// 19:59, and a card read from the follower alone said none had a switch.
#[must_use]
pub fn parse_looks(journal: &str) -> Vec<Look> {
    let mut looks = Vec::new();
    for line in journal.lines() {
        let Some((stamp, rest)) = line.split_once(' ') else {
            continue;
        };
        let (Ok(at), Some(brace)) = (stamp.parse::<f64>(), rest.find('{')) else {
            continue;
        };
        let Ok(step) = serde_json::from_str::<Said>(&rest[brace..]) else {
            continue;
        };
        let switched = match step.step.as_deref() {
            Some("done") => true,
            Some("unchanged") => false,
            _ => continue,
        };
        let Some(core) = step.followed.and_then(|f| f.core) else {
            continue;
        };
        #[expect(clippy::cast_possible_truncation, reason = "a time in seconds")]
        let at = at as i64;
        looks.push(Look { at, core, switched });
    }
    looks
}

impl Follower {
    /// Takes every update's looks, from the service's journal, over the
    /// follower's own, which are some of them. A hold the follower met
    /// before the newest look is over: only a person's ask with a hold sent
    /// moves the machine on after one, and that look is it. None at all
    /// keeps the follower's: an account outside the groups that read the
    /// system's journal is told nothing, and its follower still knows its own.
    pub fn saw(&mut self, looks: Vec<Look>) {
        if looks.is_empty() {
            return;
        }
        let newest = looks.iter().map(|look| look.at).max();
        if let (Some((held, _)), Some(newest)) = (&self.held, newest)
            && *held < newest
        {
            self.held = None;
        }
        self.looks = looks;
    }
}

/// One run nix-offload sent to a builder.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct Run {
    pub at: i64,
    #[serde(default)]
    pub on: String,
    #[serde(default)]
    pub seconds: f64,
    /// How long it waited for room on a builder, when recorded.
    pub waited: Option<f64>,
    #[serde(default)]
    pub status: i64,
    /// What it built from: `github:<owner>/<repo>/<commit>` for a pinned
    /// commit, `.` for a checkout as it stood.
    #[serde(default)]
    pub flake: String,
    /// The checks it built, as `checks.x86_64-linux.<name>`.
    #[serde(default)]
    pub attributes: Vec<String>,
}

impl Run {
    /// Whether it ran on a VM builder.
    #[must_use]
    pub fn vm(&self) -> bool {
        self.on.contains("vm")
    }
}

/// The runs in `there.jsonl` since `since`, a line that isn't one skipped.
#[must_use]
pub fn parse_runs(text: &str, since: i64) -> Vec<Run> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<Run>(line).ok())
        .filter(|run| run.at >= since)
        .collect()
}

/// A pressure file's `some` average over five minutes, in percent.
#[must_use]
pub fn avg300(pressure: &str) -> Option<f64> {
    pressure
        .lines()
        .find(|line| line.starts_with("some"))?
        .split_whitespace()
        .find_map(|field| field.strip_prefix("avg300="))?
        .parse()
        .ok()
}

/// The account's service manager's cgroup, from a process's own
/// `/proc/self/cgroup` line: the folder under `/sys/fs/cgroup` that holds
/// the slices Joel's apps and session run in, beside the agents' jobs.
#[must_use]
pub fn account(cgroup: &str) -> Option<String> {
    let path = cgroup.lines().find_map(|line| line.strip_prefix("0::"))?;
    let end = path.find(".service").filter(|_| path.contains("/user@"))?;
    Some(format!("/sys/fs/cgroup{}", &path[..end + ".service".len()]))
}

/// Memory available and swap in use, in GiB, from `/proc/meminfo`.
#[must_use]
pub fn memory(meminfo: &str) -> (f64, f64) {
    let kib = |name: &str| -> f64 {
        meminfo
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .and_then(|rest| rest.trim_start_matches(':').split_whitespace().next())
            .and_then(|n| n.parse::<f64>().ok())
            .unwrap_or(0.0)
    };
    let gib = 1024.0 * 1024.0;
    let available = kib("MemAvailable") / gib;
    let swap = (kib("SwapTotal") - kib("SwapFree")) / gib;
    (available, swap)
}

/// When the newest machine-wide out-of-memory kill was, from the kernel's
/// journal as `journalctl -k -o short-unix` writes it, leaving out a
/// cgroup's own limit, which kills inside it and restarts no week.
#[must_use]
pub fn last_machine_oom_kill(kernel: &str) -> Option<i64> {
    kernel
        .lines()
        .filter(|line| line.contains("oom-kill:") && !line.contains("CONSTRAINT_MEMCG"))
        .filter_map(|line| line.split_once(' ')?.0.parse::<f64>().ok())
        .map(|at| {
            #[expect(clippy::cast_possible_truncation, reason = "a time in seconds")]
            let at = at as i64;
            at
        })
        .max()
}

/// The net's place from its `state.json`: the head its round tests, and
/// true, or between rounds the last head it saw, and false.
#[must_use]
pub fn net_round(state: &str) -> Option<(String, bool)> {
    #[derive(Deserialize)]
    struct State {
        round: Option<String>,
        seen: Option<String>,
    }
    let state = serde_json::from_str::<State>(state).ok()?;
    match state.round {
        Some(round) => Some((round, true)),
        None => state.seen.map(|seen| (seen, false)),
    }
}

/// The net's hold on switching, from its `hold.json`: the test a merge
/// broke, the merge's pull request when it found one, whether its revert
/// merged, and whether its first try at one is still landing: the net
/// writes `revert_again` only once that try ends, merged or not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetHold {
    pub at: i64,
    pub test: String,
    pub bad_pr: Option<u64>,
    pub reverting: bool,
    pub landing: bool,
}

#[must_use]
pub fn net_hold(hold: &str) -> Option<NetHold> {
    #[derive(Deserialize)]
    struct Hold {
        at: i64,
        test: String,
        bad_pr: Option<u64>,
        revert_commit: Option<String>,
        revert_again: Option<bool>,
    }
    let hold = serde_json::from_str::<Hold>(hold).ok()?;
    Some(NetHold {
        at: hold.at,
        test: hold.test,
        bad_pr: hold.bad_pr,
        reverting: hold.revert_commit.is_some(),
        landing: hold.revert_commit.is_none() && hold.revert_again.is_none(),
    })
}

/// When the net last found a hold on the machine starting nothing, from
/// its `held.json`, which it writes each time it looks and leaves once the
/// hold ends, so its `at` is when the hold was last seen standing.
#[must_use]
pub fn net_held(held: &str) -> Option<i64> {
    serde_json::from_str::<serde_json::Value>(held).ok()?["at"].as_i64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nets_hold_says_its_test_its_merge_and_its_revert() {
        let failed = "{\"at\":1794024062,\"bad\":\"c210\",\"bad_pr\":2059,\
                      \"revert_commit\":null,\"revert_pr\":null,\"test\":\"vm-a\"}";
        assert_eq!(
            net_hold(failed),
            Some(NetHold {
                at: 1_794_024_062,
                test: "vm-a".into(),
                bad_pr: Some(2059),
                reverting: false,
                landing: true,
            })
        );
        let failed_to = "{\"at\":1,\"bad_pr\":7,\"revert_commit\":null,\"revert_again\":false,\
                         \"test\":\"vm-a\"}";
        assert!(
            net_hold(failed_to).is_some_and(|h| !h.landing && !h.reverting),
            "a revert that ended unmerged was read as still landing"
        );
        let reverting = "{\"at\":1,\"bad_pr\":7,\"revert_commit\":\"ab\",\"test\":\"vm-a\"}";
        assert!(net_hold(reverting).is_some_and(|h| h.reverting));
        assert_eq!(net_hold(""), None);
        assert_eq!(
            net_held("{\"at\":1794602830,\"why\":\"a reboot\"}"),
            Some(1_794_602_830),
            "when the net last found the machine held wasn't read"
        );
        assert_eq!(net_held(""), None, "no hold seen was read as one");
    }

    #[test]
    fn a_merge_reads_its_pull_request_and_parents() {
        let merges = parse_merges("abc 1700 p1 p2 Merge pull request #2066 from x/y\n");
        let (merge, main, branch) = &merges[0];
        assert_eq!((merge.pr, merge.at), (Some(2066), 1700));
        assert_eq!((main.as_str(), branch.as_str()), ("p1", "p2"));
    }

    #[test]
    fn a_merge_of_docs_alone_is_tests_only() {
        let merge = Merge {
            sha: "a".into(),
            at: 0,
            pr: None,
            last_commit: None,
            files: vec!["docs/delivery.md".into(), "modules/x/checks.nix".into()],
        };
        assert!(merge.tests_only());
        assert!(!merge.boot());
    }

    #[test]
    fn a_merge_to_the_kernel_or_the_lock_is_boot() {
        for path in [
            "modules/kernel/package.nix",
            "flake.lock",
            "hardware/x/default.nix",
        ] {
            let merge = Merge {
                sha: "a".into(),
                at: 0,
                pr: None,
                last_commit: None,
                files: vec![path.into()],
            };
            assert!(merge.boot(), "{path}");
        }
    }

    const JOURNAL: &str = "\
100.5 host update-to-main[1]: asking at @033663e08dd3 core@032dbeb6ecdb
101.0 host update-to-main[1]: {\"step\":\"fetching\"}
102.0 host update-to-main[1]: {\"step\":\"building\",\"commit\":\"ab\"}
200.0 host update-to-main[1]: {\"step\":\"done\",\"commit\":\"ab\",\"followed\":{\"core\":\"bebf965b8b6c\"}}
300.0 host update-to-main[1]: asking at core@be57b747568f
301.0 host update-to-main[1]: {\"step\":\"unchanged\",\"commit\":\"ab\"}
400.0 host update-to-main[1]: asking at core@ffff
";

    #[test]
    fn a_look_names_the_core_it_built_over_the_one_it_asked_at() {
        let follower = parse_follower(JOURNAL);
        assert_eq!(
            follower.looks,
            vec![
                Look {
                    at: 200,
                    core: "bebf965b8b6c".into(),
                    switched: true
                },
                Look {
                    at: 301,
                    core: "be57b747568f".into(),
                    switched: false
                },
            ]
        );
    }

    const SERVICE: &str = "\
100.0 host update-to-main[4]: uid 1000 asks for an update
200.0 host update-to-main[4]: {\"step\":\"done\",\"commit\":\"ab\",\"followed\":{\"core\":\"bebf965b8b6c\"}}
301.0 host update-to-main[4]: {\"step\":\"unchanged\",\"commit\":\"ab\",\"followed\":{\"core\":\"be57b747568f\"}}
450.0 host update-to-main[4]: {\"step\":\"building\",\"commit\":\"cd\"}
500.0 host update-to-main[4]: {\"step\":\"done\",\"commit\":\"cd\",\"restart\":\"its kernel changed\",\"followed\":{\"core\":\"b508914f6b44\"}}
600.0 host update-to-main[4]: {\"step\":\"done\",\"commit\":\"cd\"}
";

    #[test]
    fn a_switch_by_hand_is_a_look_as_the_followers_are() {
        let mut follower = parse_follower(JOURNAL);
        follower.held = Some((420, "switching would change its kernel".into()));
        follower.saw(parse_looks(SERVICE));
        let looks: Vec<_> = follower.looks.iter().map(|l| (l.at, l.switched)).collect();
        assert_eq!(
            looks,
            vec![(200, true), (301, false), (500, true)],
            "every ended update naming its core is a look, once, and only those"
        );
        assert_eq!(
            follower.looks[2].core, "b508914f6b44",
            "the switch by hand names the core it followed"
        );
        assert_eq!(follower.held, None, "a hold before the newest look stood");
    }

    #[test]
    fn a_hold_after_the_newest_look_stands_and_no_looks_keep_the_followers() {
        let mut follower = parse_follower(JOURNAL);
        follower.held = Some((700, "switching would change its kernel".into()));
        follower.saw(parse_looks(SERVICE));
        assert!(
            follower.held.is_some(),
            "a hold met after the newest look was dropped"
        );
        let mut follower = parse_follower(JOURNAL);
        follower.saw(Vec::new());
        assert_eq!(
            follower.looks,
            parse_follower(JOURNAL).looks,
            "a journal that says nothing took the follower's looks away"
        );
    }

    #[test]
    fn an_ask_with_no_end_yet_is_going() {
        assert_eq!(parse_follower(JOURNAL).going, Some(400));
    }

    #[test]
    fn failures_in_a_row_count_from_the_first_until_one_ends_well() {
        let failed = |at: u32, why: &str| {
            format!("{at}.0 h u[1]: {{\"step\":\"failed\",\"commit\":\"ab\",\"why\":\"{why}\"}}\n")
        };
        let mut journal = format!("{}{}", failed(10, "no builder"), failed(20, "dropped"));
        assert_eq!(
            parse_follower(&journal).failed,
            Some((10, "dropped".to_owned()))
        );
        journal.push_str("30.0 h u[1]: {\"step\":\"done\",\"commit\":\"ab\"}\n");
        assert_eq!(parse_follower(&journal).failed, None);
    }

    #[test]
    fn a_refusal_for_a_held_daemon_stands_until_an_update_ends() {
        let refused = |at: u32, why: &str| {
            format!("{at}.0 h u[1]: {{\"step\":\"refused\",\"why\":\"{why}\"}}\n")
        };
        let held = "switching would change jitsusama's sessions-kitty.service, which waits";
        let mut journal = refused(10, held);
        assert_eq!(parse_follower(&journal).held, Some((10, held.to_owned())));
        journal.push_str(&refused(20, "a call is on"));
        assert_eq!(
            parse_follower(&journal).held,
            Some((10, held.to_owned())),
            "a refusal for being busy leaves the hold standing"
        );
        journal.push_str("30.0 h u[1]: {\"step\":\"done\",\"commit\":\"ab\"}\n");
        assert_eq!(parse_follower(&journal).held, None);
    }

    #[test]
    fn a_switch_put_off_for_a_call_is_a_meeting() {
        let journal = "1.0 h u[1]: asking at core@aa\n2.0 h u[1]: {\"step\":\"waiting\",\"why\":\"a meeting or a call\"}\n";
        assert!(parse_follower(journal).meeting);
    }

    #[test]
    fn runs_before_the_window_and_lines_that_arent_runs_are_left_out() {
        let text = "{\"at\":5,\"on\":\"nix-offload-vm\",\"seconds\":60,\"waited\":30,\"status\":0}\n\
                    not json\n{\"at\":1,\"on\":\"nix-offload\",\"seconds\":9,\"status\":1}\n";
        let runs = parse_runs(text, 2);
        assert_eq!(runs.len(), 1);
        assert!(runs[0].vm());
        assert_eq!(runs[0].waited, Some(30.0));
    }

    #[test]
    fn cpu_pressure_reads_the_some_line() {
        let text = "some avg10=1.00 avg60=2.00 avg300=45.50 total=1\nfull avg10=0 avg60=0 avg300=9 total=0\n";
        assert_eq!(avg300(text), Some(45.5));
    }

    #[test]
    fn the_account_is_read_from_a_jobs_own_cgroup() {
        let line = "0::/user.slice/user-1000.slice/user@1000.service/jobs.slice/jobs.service\n";
        assert_eq!(
            account(line).as_deref(),
            Some("/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service")
        );
        assert_eq!(account("0::/system.slice/cadence.service\n"), None);
    }

    #[test]
    fn memory_reads_available_and_swap_used() {
        let text = "MemTotal: 100 kB\nMemAvailable: 2097152 kB\nSwapTotal: 4194304 kB\nSwapFree: 3145728 kB\n";
        assert_eq!(memory(text), (2.0, 1.0));
    }

    #[test]
    fn a_cgroups_own_kill_isnt_the_machines() {
        let text = "5.0 k: x invoked oom-killer\n9.0 k: oom-kill:constraint=CONSTRAINT_MEMCG,task=a\n\
                    7.5 k: oom-kill:constraint=CONSTRAINT_NONE,task=b\n";
        assert_eq!(last_machine_oom_kill(text), Some(7));
        assert_eq!(last_machine_oom_kill(""), None);
    }

    #[test]
    fn the_nets_round_is_read_when_testing_and_what_it_saw_between() {
        assert_eq!(
            net_round("{\"round\":\"cb5b\",\"seen\":\"61b0\"}"),
            Some(("cb5b".into(), true)),
            "a round's head wasn't read"
        );
        assert_eq!(
            net_round("{\"passed\":\"61b0\",\"seen\":\"7ee4\"}"),
            Some(("7ee4".into(), false)),
            "between rounds, what the net last saw wasn't read"
        );
        assert_eq!(
            net_round("{\"passed\":\"61b0\"}"),
            None,
            "a head came of nothing"
        );
    }
}
