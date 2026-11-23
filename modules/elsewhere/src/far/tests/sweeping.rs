//! The far line clearing the copies of runs here that are gone: which it
//! clears, which it leaves, and that it never reaches through a link.

use super::{Left, far_line, kind, scratch, work};
use crate::far::steps::{RUN, sweeping};
use std::path::Path;

/// This machine and its process space, as a far line writes them down.
fn here() -> String {
    let said = std::process::Command::new(far_sh())
        .args([
            "-c",
            "printf '%s %s' \"$(uname -n)\" \"$(readlink /proc/self/ns/pid 2>/dev/null)\"",
        ])
        .output()
        .unwrap();
    String::from_utf8(said.stdout).unwrap()
}

/// The number of a process that has ended, as a killed run's has.
fn gone() -> u32 {
    let mut ended = std::process::Command::new("true").spawn().unwrap();
    ended.wait().unwrap();
    ended.id()
}

/// The sh a far line runs in: dash, as the gate names it, or the
/// machine's own.
fn far_sh() -> String {
    std::env::var("ELSEWHERE_FAR_SH").unwrap_or_else(|_| "sh".to_owned())
}

/// Runs the sweep alone for a run whose copy is `work`, answering what it
/// said, once it ended well.
fn sweep(work: &Path) -> String {
    sweep_with(work, &[])
}

/// Runs the sweep as `sweep` does, with each of `programs`, a name and
/// its script, found before the machine's own.
fn sweep_with(work: &Path, programs: &[(&str, &str)]) -> String {
    let bin = work.parent().unwrap().with_extension("bin");
    std::fs::create_dir_all(&bin).unwrap();
    for (name, text) in programs {
        std::fs::write(bin.join(name), format!("#!/bin/sh\n{text}\n")).unwrap();
        std::fs::set_permissions(
            bin.join(name),
            std::os::unix::fs::PermissionsExt::from_mode(0o755),
        )
        .unwrap();
    }
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let swept = std::process::Command::new(far_sh())
        .args(["-c", &format!("work='{}'\n{}", work.display(), sweeping())])
        .env("PATH", path)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&bin);
    let said = String::from_utf8_lossy(&swept.stderr).into_owned();
    assert!(swept.status.success(), "the sweep failed: {said}");
    said
}

/// A copy named `id` in `runs`, with a file in it, and its run's file
/// saying `run`.
fn copy(runs: &Path, id: &str, run: &str) {
    std::fs::create_dir_all(runs.join(id)).unwrap();
    std::fs::write(runs.join(id).join("f"), "f").unwrap();
    std::fs::write(runs.join(format!("{id}{RUN}")), format!("{run}\n")).unwrap();
}

/// What lies in `runs`, by name, in order.
fn left_in(runs: &Path) -> Vec<String> {
    let mut left: Vec<_> = std::fs::read_dir(runs)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    left.sort();
    left
}

#[test]
fn only_the_copies_of_runs_here_that_are_gone_are_cleared() {
    let root = scratch("far-swept");
    let runs = root.join("s");
    let (here, gone) = (here(), gone());
    // Gone, named as a job is, with all a run leaves beside its copy and
    // a folder its job closed to its owner.
    let job = "4242-1763912345678901234";
    copy(&runs, job, &format!("{gone} {here}"));
    std::fs::write(runs.join(format!("{job}.names")), "n").unwrap();
    std::fs::create_dir_all(runs.join(format!("{job}.back"))).unwrap();
    std::fs::write(runs.join(format!("{job}.back.lost")), "l").unwrap();
    let closed = runs.join(job).join("closed");
    std::fs::create_dir_all(&closed).unwrap();
    std::fs::write(closed.join("f"), "f").unwrap();
    std::fs::set_permissions(&closed, std::os::unix::fs::PermissionsExt::from_mode(0o500)).unwrap();
    // Live: this test's own process.
    copy(&runs, "3", &format!("{} {here}", std::process::id()));
    // Another machine's, whose numbers say nothing here.
    copy(&runs, "4", &format!("{gone} osprey"));
    // This machine's, but another process space's.
    let machine = here.split(' ').next().unwrap();
    copy(&runs, "7", &format!("{gone} {machine} pid:[1]"));
    // A run's file that names no number, and one that is empty.
    copy(&runs, "5", &format!("x {here}"));
    copy(&runs, "8", "");
    // A run this sweep may not signal, which may well be alive: launchd,
    // on a Mac. In a Linux build's sandbox, the first process is the
    // build's own, alive either way.
    copy(&runs, "10", &format!("1 {here}"));
    // A name no job has, which would name what lies beside copy 3.
    copy(&runs, "3.back", &format!("{gone} {here}"));
    // A run's file that is a link, which is never read.
    copy(&runs, "6", "");
    std::fs::remove_file(runs.join(format!("6{RUN}"))).unwrap();
    std::fs::write(root.join("elsewhere.run"), format!("{gone} {here}\n")).unwrap();
    std::os::unix::fs::symlink(root.join("elsewhere.run"), runs.join(format!("6{RUN}"))).unwrap();
    // A copy with no run's file, untouched for years, which no age clears.
    std::fs::create_dir_all(runs.join("old")).unwrap();
    let touched = std::process::Command::new("touch")
        .args(["-t", "202001010000"])
        .arg(runs.join("old"))
        .status()
        .unwrap();
    assert!(touched.success(), "touch couldn't age the old copy");
    let said = sweep(&runs.join("9"));
    assert!(said.is_empty(), "the sweep said something: {said}");
    let kept = [
        "10",
        "10.run",
        "3",
        "3.back",
        "3.back.run",
        "3.run",
        "4",
        "4.run",
        "5",
        "5.run",
        "6",
        "6.run",
        "7",
        "7.run",
        "8",
        "8.run",
        "old",
    ];
    assert_eq!(
        left_in(&runs),
        kept,
        "the sweep took a copy whose run isn't known gone, or left one whose is"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_linux_far_line_that_cant_read_its_process_space_clears_nothing() {
    let root = scratch("far-swept-unknown");
    let runs = root.join("s");
    let machine = here().split(' ').next().unwrap().to_owned();
    // Named as a run here that can't read its space writes it down.
    copy(&runs, "2", &format!("{} {machine}", gone()));
    let uname = "[ \"$1\" = -s ] && { echo Linux; exit 0; }
PATH=${PATH#*:}
exec uname \"$@\"";
    sweep_with(&runs.join("9"), &[("uname", uname), ("readlink", "exit 1")]);
    assert_eq!(
        left_in(&runs),
        ["2", "2.run"],
        "a sweep that couldn't tell process spaces apart judged a number"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_far_line_whose_copies_lie_through_a_link_clears_nothing_it_leads_to() {
    let root = scratch("far-swept-link");
    let real = root.join("real");
    copy(&real, "2", &format!("{} {}", gone(), here()));
    std::os::unix::fs::symlink(&real, root.join("s")).unwrap();
    sweep(&root.join("s/9"));
    assert_eq!(
        left_in(&real),
        ["2", "2.run"],
        "the sweep cleared what the copies' folder, a link, leads to"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_far_line_clears_a_gone_run_s_copy_and_leaves_nothing_of_its_own() {
    // A uname that, the first time it's asked, leaves the copy of a run
    // here that has gone, as the far line begins.
    let uname = "PATH=${PATH#*:}
runs=\"$HOME/.cache/pi/agentic-harness.pi/quest-workspace/QEST-20261115-ABC123/s\"
if mkdir \"$HOME/left\" 2>/dev/null; then
  sh -c : & gone=$!
  wait \"$gone\"
  mkdir -p \"$runs/7\" && touch \"$runs/7/f\" &&
    printf '%s %s %s\\n' \"$gone\" \"$(uname -n)\" \"$(readlink /proc/self/ns/pid 2>/dev/null)\" > \"$runs/7.run\"
fi
exec uname \"$@\"";
    let (code, err, home) = far_line(
        Left::Whole,
        &[("a", "1")],
        "touch ../ran",
        &kind(""),
        None,
        &[("uname", uname)],
    );
    assert!(
        code == Some(0) && work(&home).join("ran").exists(),
        "the job never ran ({code:?}): {err}"
    );
    assert!(home.join("left").exists(), "no gone run's copy was left");
    assert_eq!(
        left_in(&work(&home)),
        ["ran"],
        "the far line left a gone run's copy or something of its own: {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_stop_cut_short_leaves_its_run_s_file_for_the_next_sweep() {
    // An rm that can't clear a stopped copy, as one killed in mycelium's
    // grace doesn't, passing all else to the machine's own.
    let rm = "PATH=${PATH#*:}
case \" $* \" in *\"/s/1.names \"*\"/s/1 \"*) exit 1 ;; esac
exec rm \"$@\"";
    let (code, err, home) = far_line(
        Left::Whole,
        &[("a", "1")],
        "touch ../ran && kill -TERM $PPID",
        &kind(""),
        None,
        &[("rm", rm)],
    );
    let runs = work(&home);
    assert!(
        runs.join("ran").exists(),
        "the job never ran ({code:?}): {err}"
    );
    let run = std::fs::read_to_string(runs.join(format!("1{RUN}"))).unwrap_or_default();
    let number = run.split(' ').next().unwrap_or_default();
    assert!(
        runs.join("1/a").exists() && number.parse::<u32>().is_ok(),
        "a stop cut short left no copy or no run's file to clear it by: {run:?}"
    );
    assert_eq!(
        run.trim_end_matches('\n')
            .split_once(' ')
            .map(|(_, rest)| rest),
        Some(here().as_str()),
        "the run's file doesn't name this machine and its process space"
    );
    sweep(&runs.join("9"));
    assert_eq!(
        left_in(&runs),
        ["ran"],
        "the next sweep didn't clear what the stop left"
    );
    let _ = std::fs::remove_dir_all(&home);
}
