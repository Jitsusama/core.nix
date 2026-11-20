use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use super::*;
use crate::source::Followers;

/// Whether the process `id` is gone and waited on, so no signal reaches it.
fn gone(id: u32) -> bool {
    let id = libc::pid_t::try_from(id).expect("a process's id");
    // SAFETY: signal 0 only asks whether the process is there.
    let asked = unsafe { libc::kill(id, 0) };
    asked == -1 && io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}

/// The signals the process `id` blocks and catches, as its status says.
#[cfg(target_os = "linux")]
fn blocking_and_catching(id: u32) -> (u64, u64) {
    let status = std::fs::read_to_string(format!("/proc/{id}/status")).expect("its status");
    let mask = |name: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .and_then(|mask| u64::from_str_radix(mask.trim(), 16).ok())
            .expect("its mask")
    };
    (mask("SigBlk:"), mask("SigCgt:"))
}

#[test]
fn a_signal_that_stops_health_ends_its_followers_first() {
    // SIGUSR2 stands in for the signals that stop health, which a test
    // can't catch without ending the others' runs at their stop. A catch
    // is the process's, so this is the one test that catches.
    let caught = Caught::catch(&[libc::SIGUSR2]).expect("the signal caught");
    let followers = Followers::default();
    let mut sleep = Command::new("sleep");
    sleep.arg("600");
    let follower = followers.following(sleep);
    // Were the signal blocked rather than caught, every follower would
    // block it, and a service manager's stop would wait out its timeout
    // to kill each; a catch is undone as a follower starts its program.
    #[cfg(target_os = "linux")]
    assert_eq!(
        blocking_and_catching(follower),
        (0, 0),
        "a follower started while health catches its stops blocks or catches a signal"
    );
    let (ended, heard) = mpsc::channel();
    let waiting = caught.on_one(followers.ender(), move |signal| {
        let _ = ended.send((signal, gone(follower)));
    });
    // SAFETY: the signal is caught, so raising it only writes it down.
    unsafe { libc::raise(libc::SIGUSR2) };
    let heard = heard.recv_timeout(Duration::from_secs(10));
    waiting.join().expect("the waiting thread ends");
    assert_eq!(
        heard,
        Ok((libc::SIGUSR2, true)),
        "a signal that stops health went unheard, or left its follower running"
    );
}
