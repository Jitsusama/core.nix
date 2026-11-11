//! The card: the last hour of the cycle in a few plain lines, written to a
//! file each tick so whoever steers the work can read it whether or not
//! the session keeping the cycle fast is there.

use std::fmt::Write;

use crate::stats::spread;
use crate::watch::Snapshot;

/// The card for the hour before `s.now`, with `clock` turning a time into
/// the hour and minute it reads as here.
#[must_use]
pub fn card(s: &Snapshot, clock: impl Fn(i64) -> String) -> String {
    let since = s.now - 3600;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "Card at {} (the hour since {})",
        clock(s.now),
        clock(since)
    );
    delivery(&mut out, s, &clock);
    machine(&mut out, s);
    net(&mut out, s, &clock);
    out
}

#[expect(clippy::cast_precision_loss, reason = "seconds within a lifetime")]
fn minutes(secs: i64) -> f64 {
    secs as f64 / 60.0
}

/// The hour's merges, how long each took to reach the machine, and its
/// switches.
fn delivery(out: &mut String, s: &Snapshot, clock: &impl Fn(i64) -> String) {
    let since = s.now - 3600;
    let merged: Vec<_> = s.landings.iter().filter(|l| l.merge.at >= since).collect();
    let tests_only = merged.iter().filter(|l| l.merge.tests_only()).count();
    let _ = writeln!(
        out,
        "Core: {} merged ({tests_only} tests or docs only).",
        merged.len()
    );
    let to_merge: Vec<f64> = merged
        .iter()
        .filter(|l| !l.merge.boot())
        .filter_map(|l| l.merge.last_commit.map(|c| minutes(l.merge.at - c)))
        .collect();
    line(
        out,
        "Last commit to merge, outside boot (target median 20)",
        &to_merge,
    );
    let mut on: Vec<(f64, String)> = merged
        .iter()
        .filter(|l| !l.merge.tests_only())
        .filter_map(|l| {
            l.on.as_ref()
                .map(|look| (minutes(look.at - l.merge.at), l.merge.name()))
        })
        .collect();
    let times: Vec<f64> = on.iter().map(|(m, _)| *m).collect();
    line(out, "Merge to on (target median 8)", &times);
    on.sort_by(|a, b| b.0.total_cmp(&a.0));
    if !on.is_empty() {
        let worst: Vec<String> = on
            .iter()
            .take(3)
            .map(|(m, n)| format!("{n} {m:.0}"))
            .collect();
        let _ = writeln!(out, "  slowest: {}", worst.join(", "));
    }
    let waiting: Vec<String> = s
        .landings
        .iter()
        .filter(|l| l.on.is_none() && !l.merge.tests_only() && s.now - l.merge.at < 6 * 3600)
        .map(|l| format!("{} {} min", l.merge.name(), (s.now - l.merge.at) / 60))
        .collect();
    if !waiting.is_empty() {
        let _ = writeln!(out, "  not on yet: {}", waiting.join(", "));
    }
    let switches: Vec<i64> = s
        .follower
        .looks
        .iter()
        .filter(|l| l.switched && l.at >= since)
        .map(|l| l.at)
        .collect();
    let last = switches
        .last()
        .map_or_else(|| "none".to_owned(), |&at| clock(at));
    let _ = writeln!(out, "  switches: {}, last at {last}", switches.len());
}

/// The hour's VM runs and the machine's pressure and memory.
fn machine(out: &mut String, s: &Snapshot) {
    let since = s.now - 3600;
    let vm: Vec<_> = s.runs.iter().filter(|r| r.vm() && r.at >= since).collect();
    let ok = vm.iter().filter(|r| r.status == 0).count();
    let waits: Vec<f64> = vm
        .iter()
        .filter_map(|r| r.waited.map(|w| w / 60.0))
        .collect();
    let _ = writeln!(out, "VM runs: {}, {ok} passed.", vm.len());
    line(out, "VM wait for a builder (target median 2)", &waits);

    let percent = |c: Option<f64>| c.map_or_else(|| "unknown".to_owned(), |c| format!("{c:.0}%"));
    let _ = writeln!(
        out,
        "CPU pressure over five minutes: {} (target 15 from agents' work), {} in what \
         Joel uses.",
        percent(s.cpu_avg300),
        percent(s.joel_cpu_avg300)
    );
    let _ = writeln!(
        out,
        "Memory: {:.0} GiB available, {:.0} GiB of swap in use.",
        s.available_gib, s.swap_gib
    );
    let _ = match s.last_oom_kill {
        Some(at) => writeln!(
            out,
            "  last machine-wide out-of-memory kill {:.1} h ago (target a week).",
            minutes(s.now - at) / 60.0
        ),
        None => writeln!(out, "  no machine-wide out-of-memory kill this week."),
    };
}

/// How far the net trails main, and whether the watching session is there.
fn net(out: &mut String, s: &Snapshot, clock: &impl Fn(i64) -> String) {
    match &s.round {
        Some(round) => {
            let what = if round.testing {
                "tests"
            } else {
                "tests nothing now; it last saw"
            };
            let _ = writeln!(
                out,
                "The net {what} {} ({} merges behind main), merged {} min ago.",
                round.head.chars().take(9).collect::<String>(),
                round.behind,
                (s.now - round.merged) / 60
            );
        }
        None => {
            let _ = writeln!(out, "The net is caught up with main.");
        }
    }
    if let Some(hold) = &s.net_hold {
        let pr = hold.bad_pr.map_or_else(
            || "a merge it couldn't name".to_owned(),
            |pr| format!("#{pr}"),
        );
        let revert = if hold.reverting {
            "its revert is under way"
        } else {
            "nothing reverts it"
        };
        let _ = writeln!(
            out,
            "  it holds switches since {} on {}, broken by {pr}; {revert}.",
            clock(hold.at),
            hold.test
        );
    }
    if s.watched {
        let watcher = if s.watcher_alive { "there" } else { "gone" };
        let card_at = s.last_card.map_or_else(|| "none".to_owned(), clock);
        let _ = writeln!(
            out,
            "The session keeping the cycle fast is {watcher}; its last card went at {card_at}."
        );
    }
}

fn line(out: &mut String, what: &str, xs: &[f64]) {
    let _ = match spread(xs) {
        Some(s) => writeln!(out, "{what}: {s}."),
        None => writeln!(out, "{what}: none."),
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::{Look, Merge, Run};
    use crate::watch::Landing;

    #[test]
    fn a_card_says_the_hours_merges_switches_and_runs() {
        let now = 100_000;
        let merge = Merge {
            sha: "abc".into(),
            at: now - 1200,
            pr: Some(2066),
            last_commit: Some(now - 1500),
            files: vec!["tools/merge-net/src/watch.rs".into()],
        };
        let look = Look {
            at: now - 600,
            core: "abc".into(),
            switched: true,
        };
        let s = Snapshot {
            now,
            landings: vec![Landing {
                merge,
                on: Some(look.clone()),
            }],
            follower: crate::records::Follower {
                looks: vec![look],
                ..Default::default()
            },
            runs: vec![Run {
                at: now - 60,
                on: "nix-offload-vm".into(),
                seconds: 300.0,
                waited: Some(30.0),
                status: 0,
            }],
            cpu_avg300: Some(12.0),
            joel_cpu_avg300: Some(1.0),
            last_oom_kill: Some(now - 9 * 3600),
            watched: true,
            watcher_alive: true,
            ..Snapshot::default()
        };
        let text = card(&s, |t| format!("t{t}"));
        for want in [
            "Core: 1 merged (0 tests or docs only).",
            "Last commit to merge, outside boot (target median 20): median 5.0",
            "Merge to on (target median 8): median 10.0",
            "slowest: #2066 10",
            "switches: 1, last at t99400",
            "VM runs: 1, 1 passed.",
            "VM wait for a builder (target median 2): median 0.5",
            "CPU pressure over five minutes: 12% (target 15 from agents' work), 1% in what Joel",
            "The net is caught up with main.",
            "last machine-wide out-of-memory kill 9.0 h ago",
            "is there; its last card went at none.",
        ] {
            assert!(text.contains(want), "missing {want:?} in\n{text}");
        }
    }

    #[test]
    fn a_net_between_rounds_behind_main_is_not_said_caught_up() {
        let s = Snapshot {
            now: 100_000,
            round: Some(crate::net::Round {
                head: "7ee4bf22c8fa".into(),
                merged: 100_000 - 3 * 3600,
                behind: 40,
                testing: false,
            }),
            ..Snapshot::default()
        };
        let text = card(&s, |t| format!("t{t}"));
        let want = "The net tests nothing now; it last saw 7ee4bf22c (40 merges behind main)";
        assert!(text.contains(want), "missing {want:?} in\n{text}");
    }
}
