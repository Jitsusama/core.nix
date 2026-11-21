//! The followers health ends as `follow` returns, stood in for by shells
//! that stop themselves, as a quiet journal's follower waits for a line
//! that never comes, so only health's end can end them.

use std::thread;

use super::*;
use crate::signals::WRITES_WITHIN;

/// Whether the process `pid` names is still running, or a zombie no one
/// waited on.
fn alive(pid: &str) -> bool {
    sh(&format!("kill -0 {pid}"))
        .status()
        .is_ok_and(|status| status.success())
}

/// The process each of `count` followers says it is, as its first line.
fn pids_of(heard: &mpsc::Receiver<Option<String>>, count: usize) -> Vec<String> {
    (0..count)
        .map(|_| {
            heard
                .recv_timeout(Duration::from_secs(10))
                .ok()
                .flatten()
                .expect("each follower says which process it is")
                .trim()
                .to_owned()
        })
        .collect()
}

#[test]
fn every_follower_still_running_ends_as_health_does() {
    // Started as the replay starts them, from the children `start` hands
    // it, so the ones health ends are the ones it follows.
    let followers = Followers::default();
    let (send, heard) = mpsc::channel();
    let count = replayed_by(
        writing(&[]),
        |_| writing(&[]),
        |_, _| sh("echo $$; kill -STOP $$"),
        &send,
        &followers.children,
    );
    drop(send);
    let pids = pids_of(&heard, count);
    let before: Vec<bool> = pids.iter().map(|pid| alive(pid)).collect();
    drop(followers);
    let after: Vec<bool> = pids.iter().map(|pid| alive(pid)).collect();
    assert_eq!(
        (before, after, all_of(&heard)),
        (vec![true; count], vec![false; count], vec![]),
        "a follower still running outlived health's end, or sent its end as it went"
    );
}

#[test]
fn a_follower_health_ended_says_nothing_of_its_end() {
    // Health ending its followers isn't a follower's end, so a warning
    // that one ended would be false.
    let followers = Followers::default();
    let (send, heard) = mpsc::channel();
    let tell = send.clone();
    spawn_saying(
        sh("echo $$; kill -STOP $$"),
        send,
        true,
        move |words| {
            let _ = tell.send(Some(format!("said: {words}")));
        },
        &followers.children,
    )
    .expect("the shell starts");
    let pids = pids_of(&heard, 1);
    drop(followers);
    assert_eq!(
        (alive(&pids[0]), all_of(&heard)),
        (false, vec![]),
        "a follower health ended outlived it, or said or sent its end"
    );
}

#[test]
fn followers_that_wont_end_hold_health_only_its_bound() {
    // As one stuck past its kill on a stalled disk holds their end: here
    // the followers' lock, held, keeps it from returning. On a thread of
    // its own, so a drop that holds for good fails here.
    let followers = Followers::default();
    let children = followers.children.clone();
    let held = children.running();
    let (dropped, waited) = mpsc::channel();
    thread::spawn(move || {
        let since = Instant::now();
        drop(followers);
        let _ = dropped.send(since.elapsed());
    });
    // Long for a busy builder, and short of a second bound.
    let took = waited.recv_timeout(WRITES_WITHIN + Duration::from_secs(2));
    drop(held);
    assert!(
        took.is_ok_and(|took| took >= WRITES_WITHIN),
        "followers that wouldn't end held health {took:?}, its bound {WRITES_WITHIN:?}"
    );
}

#[test]
fn no_follower_starts_once_health_has_ended() {
    // The replay starts its followers on a thread of its own, which may
    // still be starting them as health ends. One that can't start would
    // be an error, had it been started.
    let followers = Followers::default();
    let children = followers.children.clone();
    drop(followers);
    let (send, _heard) = mpsc::channel();
    let started = spawn_saying(
        Command::new("/nonexistent/journalctl"),
        send,
        true,
        |_| {},
        &children,
    );
    assert!(
        started.is_ok(),
        "a follower was started once health had ended: {started:?}"
    );
}
