//! What a finished pass says: its line on standard error, whether it reads
//! as failing, and the line for a system daemon that couldn't be asked.
//! Apart from `pass`'s scenarios, which run a pass over a fake disk, so
//! neither file passes 800 lines.

use disk_guard::band::Band;
use disk_guard::pass::{End, Failing, Outcome, Report, failed_line, failing, told, unheard_line};
use std::path::PathBuf;

const GIB: u64 = 1 << 30;

/// A finished pass's report, with what came of each path it tried.
fn report(freed: u64, outcomes: Vec<Outcome>) -> Report {
    Report {
        end: End::Reached,
        band: Band::Filling,
        freed,
        actions: outcomes
            .into_iter()
            .enumerate()
            .map(|(number, outcome)| (PathBuf::from(format!("/tmp/taken-{number}")), outcome))
            .collect(),
    }
}

#[test]
fn a_pass_that_took_something_says_when_how_many_and_how_much_it_freed() {
    let report = report(
        8 * GIB + GIB / 4,
        vec![
            Outcome::Reclaimed,
            Outcome::Kept("in use".to_string()),
            Outcome::Reclaimed,
            Outcome::Failed("refused".to_string()),
        ],
    );
    assert_eq!(
        told(&report, 12.34, "2026-11-09 10:33:05").as_deref(),
        Some(
            "disk-guard: at 2026-11-09 10:33:05 a pass while filling took 2 of the 3 it \
             tried, freeing 8.2 GiB in 12.3 s"
        )
    );
}

#[test]
fn a_pass_whose_tries_all_failed_says_so_too() {
    let report = report(0, vec![Outcome::Failed("refused".to_string())]);
    assert_eq!(
        told(&report, 0.5, "2026-11-09 10:33:05").as_deref(),
        Some(
            "disk-guard: at 2026-11-09 10:33:05 a pass while filling took 0 of the 1 it \
             tried, freeing 0 B in 0.5 s"
        )
    );
}

#[test]
fn a_pass_that_only_judged_says_nothing() {
    let kept = report(0, vec![Outcome::Kept("in use".to_string())]);
    assert_eq!(told(&kept, 3.0, "2026-11-09 10:33:05"), None);
    assert_eq!(
        told(&report(0, Vec::new()), 3.0, "2026-11-09 10:33:05"),
        None
    );
}

// On 11 November a laptop's system guard spent 90 minutes with every
// delete of the store's garbage refused, each pass saying only that it
// took 0 of the 6 it tried, and its notice that nothing was left to clear.
#[test]
fn a_filling_pass_that_freed_nothing_for_its_failures_is_an_error() {
    let refused = "warning: unable to download 'https://cache.example/x.narinfo'\n\
                   retrying in 250 ms\n\
                   error: unable to download 'https://cache.example/x.narinfo': \
                   Could not resolve hostname\n";
    let report = report(
        0,
        vec![
            Outcome::Failed(refused.to_string()),
            Outcome::Kept("in use".to_string()),
            Outcome::Failed(refused.to_string()),
        ],
    );
    let failing = failing(&report).expect("a filling pass that freed nothing is failing");
    assert_eq!(
        failing,
        Failing {
            tried: 2,
            failed: 2,
            timed_out: false,
            last: "error: unable to download 'https://cache.example/x.narinfo': Could not \
                   resolve hostname"
                .to_string(),
        }
    );
    assert_eq!(
        failed_line(&failing, &report, "2026-11-11 15:40:00"),
        "disk-guard: error: at 2026-11-11 15:40:00 a pass while filling freed nothing: 2 of \
         the 2 it tried failed; the last: error: unable to download \
         'https://cache.example/x.narinfo': Could not resolve hostname"
    );
}

#[test]
fn a_timeout_is_an_error_even_when_the_rest_freed_something() {
    let report = report(
        3 * GIB,
        vec![
            Outcome::Reclaimed,
            Outcome::Failed("nix ran past 600s".to_string()),
        ],
    );
    let failing = failing(&report).expect("a delete timing out is failing");
    assert!(failing.timed_out, "{failing:?}");
    assert_eq!(
        failed_line(&failing, &report, "2026-11-11 15:40:00"),
        "disk-guard: error: at 2026-11-11 15:40:00 a pass while filling freed only 3.0 GiB: 1 \
         of the 2 it tried failed, one timing out; the last: nix ran past 600s"
    );
}

#[test]
fn a_system_daemon_that_couldnt_be_asked_is_said_with_why() {
    assert_eq!(
        unheard_line(
            Some("Resource temporarily unavailable"),
            "2026-11-14 14:18:48"
        ),
        "disk-guard: at 2026-11-14 14:18:48 the system daemon couldn't be asked about the \
         processes this account can't read (Resource temporarily unavailable), so what they \
         might use was kept"
    );
    assert_eq!(
        unheard_line(None, "2026-11-14 14:18:48"),
        "disk-guard: at 2026-11-14 14:18:48 the system daemon couldn't be asked about the \
         processes this account can't read, so what they might use was kept"
    );
}

#[test]
fn what_freed_something_failed_at_nothing_or_ran_while_healthy_isnt_an_error() {
    let partly = report(
        GIB,
        vec![Outcome::Reclaimed, Outcome::Failed("refused".to_string())],
    );
    assert_eq!(failing(&partly), None, "a pass that freed something");
    let kept = report(0, vec![Outcome::Kept("in use".to_string())]);
    assert_eq!(failing(&kept), None, "a pass that only judged");
    let mut healthy = report(0, vec![Outcome::Failed("nix ran past 600s".to_string())]);
    healthy.band = Band::Healthy;
    assert_eq!(failing(&healthy), None, "upkeep on a healthy disk");
}

// On 14 November two switches each rooted paths a pass had listed as
// garbage, and each left an error line though the guard lists again.
#[test]
fn a_batch_gone_stale_since_the_listing_isnt_an_error() {
    let stale = || Outcome::Failed("3 paths came alive since the listing".to_string());
    let switched = report(0, vec![Outcome::Kept("in use".to_string()), stale()]);
    assert_eq!(
        failing(&switched),
        None,
        "a stale batch read as a failing guard"
    );
    let both = report(0, vec![stale(), Outcome::Failed("refused".to_string())]);
    let failing = failing(&both).expect("a refusal beside a stale batch is still failing");
    assert_eq!(
        (failing.failed, failing.last.as_str()),
        (1, "refused"),
        "a stale batch was counted among the failures"
    );
}
