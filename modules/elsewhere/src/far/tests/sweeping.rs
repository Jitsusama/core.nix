//! The far line clearing the copies of runs here that are gone: which it
//! clears, which it leaves, and that it never reaches through a link, nor
//! writes through one a job left among the copies.

use super::{Left, far_line, far_line_in, kind, scratch, work};
use crate::far::steps::{CLEARING, RUN, sweeping};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

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
pub(super) fn gone() -> u32 {
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
    sweep_in(&far_sh(), work, programs, "")
}

/// Runs the sweep as `sweep` does, after the shell has run `before`.
fn sweep_after(work: &Path, before: &str) -> String {
    sweep_in(&far_sh(), work, &[], before)
}

/// Runs the sweep in `sh` for a run whose copy is `work`, after `before`,
/// with each of `programs` found before the machine's own, answering what
/// it said, once it ended well.
fn sweep_in(sh: &str, work: &Path, programs: &[(&str, &str)], before: &str) -> String {
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
    // Under C, which every machine has, in place of any locale of the
    // runner's, so each test asks for its own, through LC_ALL alone, since
    // C there outweighs any LANG or LC_CTYPE.
    let swept = std::process::Command::new(sh)
        .env("LC_ALL", "C")
        .args([
            "-c",
            &format!(
                "{before}\nwork='{}'\n{CLEARING}\n{}",
                work.display(),
                sweeping()
            ),
        ])
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

/// A folder `closed` in `within`, holding a file, closed to its owner as
/// a job may leave one.
fn closed(within: &Path) -> std::path::PathBuf {
    let closed = within.join("closed");
    std::fs::create_dir_all(&closed).unwrap();
    std::fs::write(closed.join("f"), "f").unwrap();
    std::fs::set_permissions(&closed, PermissionsExt::from_mode(0o500)).unwrap();
    closed
}

/// Whether `folder` is still closed to its owner, opening it so its test
/// can clear it.
fn still_closed(folder: &Path) -> bool {
    let mode = std::fs::metadata(folder).unwrap().permissions().mode() & 0o777;
    std::fs::set_permissions(folder, PermissionsExt::from_mode(0o700)).unwrap();
    mode == 0o500
}

/// What lies in `runs`, by name, in order.
pub(super) fn left_in(runs: &Path) -> Vec<String> {
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
    // Gone, named as a job is, with all a run leaves beside its copy, a
    // folder its job closed to its owner, and a link to a closed folder
    // beyond it, which stays closed.
    let job = "4242-1763912345678901234";
    copy(&runs, job, &format!("{gone} {here}"));
    std::fs::write(runs.join(format!("{job}.names")), "n").unwrap();
    std::fs::write(runs.join(format!("{job}.tar")), "t").unwrap();
    std::fs::write(runs.join(format!("{job}.back")), "b").unwrap();
    std::fs::write(runs.join(format!("{job}.back.lost")), "l").unwrap();
    closed(&runs.join(job));
    let beyond = closed(&root.join("beyond"));
    std::os::unix::fs::symlink(root.join("beyond"), runs.join(job).join("inner")).unwrap();
    // Gone, its job having made folders where its names, its archive,
    // its brought-back list and a list's leftovers go, and closed one in
    // each.
    let made = "4243-1763912345678901235";
    copy(&runs, made, &format!("{gone} {here}"));
    for beside in [".names", ".tar", ".back", ".back.lost"] {
        closed(&runs.join(format!("{made}{beside}")));
    }
    // Gone, but its copy is a link, which goes alone.
    std::fs::write(runs.join(format!("11{RUN}")), format!("{gone} {here}\n")).unwrap();
    let past = closed(&root.join("past"));
    std::os::unix::fs::symlink(root.join("past"), runs.join("11")).unwrap();
    // Live: this test's own process, with a closed folder left closed.
    copy(&runs, "3", &format!("{} {here}", std::process::id()));
    let live = closed(&runs.join("3"));
    // Another machine's, whose numbers say nothing here.
    copy(&runs, "4", &format!("{gone} osprey"));
    // This machine's, but another process space's.
    let machine = here.split(' ').next().unwrap();
    copy(&runs, "7", &format!("{gone} {machine} pid:[1]"));
    // A run's file that names no number, and one that is empty.
    copy(&runs, "5", &format!("x {here}"));
    copy(&runs, "8", "");
    // One that says more than a run writes, which no run here wrote.
    copy(&runs, "12", &format!("{gone} {here} more"));
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
        "12",
        "12.run",
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
    assert!(still_closed(&live), "the sweep opened a live run's folder");
    assert!(
        still_closed(&beyond),
        "the sweep opened a folder through a link"
    );
    assert!(
        still_closed(&past),
        "the sweep opened a folder a copy's link led to"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_copy_that_wont_go_keeps_its_run_s_file_and_is_named_once() {
    let root = scratch("far-swept-stuck");
    let runs = root.join("s");
    let job = "4242-1763912345678901234";
    copy(&runs, job, &format!("{} {}", gone(), here()));
    // Only the copy won't go; its run's file would, were it asked to.
    let rm = format!(
        "case \"$*\" in *'/{job}') echo \"rm: cannot remove '$2/f': Permission denied\" >&2; exit 1 ;; esac
PATH=${{PATH#*:}}
exec rm \"$@\""
    );
    let said = sweep_with(&runs.join("9"), &[("rm", &rm)]);
    assert_eq!(
        said,
        format!(
            "elsewhere: couldn't clear {}, an earlier run's copy\n",
            runs.join(job).display()
        ),
        "the sweep didn't name the copy it couldn't clear, once and alone"
    );
    assert_eq!(
        left_in(&runs),
        [job.to_owned(), format!("{job}{RUN}")],
        "a copy that wouldn't go lost its run's file, so no sweep could retry it"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_link_at_the_copies_folder_that_wont_clear_stops_the_sweep_and_says_so() {
    let root = scratch("far-swept-linked");
    let runs = root.join("s");
    let aside = root.join("aside");
    // Two gone runs, so a sweep that went on past the link would say so
    // again for the second.
    let (job, next) = ("4242-1763912345678901234", "4243-1763912345678901234");
    for id in [job, next] {
        copy(&runs, id, &format!("{} {}", gone(), here()));
    }
    // Another job of the quest moving the copies' folder aside, with a
    // link to it at its name, as the sweep's first head reads a gone
    // run's file (the head after tr in its pipeline leaves it be), and an
    // rm that can't clear that link.
    let (r, a) = (runs.display(), aside.display());
    let head = format!(
        "PATH=${{PATH#*:}}
[ \"$1\" != -c ] || [ -L '{r}' ] || {{ mv '{r}' '{a}' && ln -s '{a}' '{r}'; }}
exec head \"$@\""
    );
    let rm = format!("case \"$*\" in *' {r}') exit 1 ;; esac\nPATH=${{PATH#*:}}\nexec rm \"$@\"");
    let said = sweep_with(&runs.join("9"), &[("head", &head), ("rm", &rm)]);
    assert_eq!(
        said,
        "elsewhere: a link at its copies' folder couldn't be cleared, or one stands at the \
         quest's scratch, so the sweep stops, and this run is untaken if it still stands once \
         its files cross\n",
        "the sweep didn't say why it stopped, once and alone"
    );
    assert_eq!(
        left_in(&aside),
        [
            job.to_owned(),
            format!("{job}{RUN}"),
            next.to_owned(),
            format!("{next}{RUN}")
        ],
        "the sweep cleared through the link it couldn't clear"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// An rm that can't clear the job's copy where an end frees it with the
/// tree it was made from, passing all else to the machine's own.
const STUCK_COPY: &str = "PATH=${PATH#*:}
case \" $* \" in *\"/1/tree \"*\"/.s/1 \"*) exit 1 ;; esac
exec rm \"$@\"";

#[test]
fn each_end_of_a_far_line_takes_its_run_s_file_only_with_its_copy() {
    // A job that ends, one refused for a folder that never crossed, and
    // one untaken for a tree that couldn't be made.
    for stuck in [false, true] {
        ends_with_its_run_s_file(("ended", 0), "", None, stuck);
        ends_with_its_run_s_file(("refused", 77), "drafts", None, stuck);
        ends_with_its_run_s_file(("untaken", 75), "", Some(("tar", "exit 1")), stuck);
    }
}

/// Runs a job that `end`s, with the code that end gives, in the tree's
/// `folder` and with `program` ahead of the machine's own, with a copy
/// that won't go when `stuck`, and checks it ended so, and that its copy
/// and its run's file both stay when stuck, and both go when not.
fn ends_with_its_run_s_file(
    (end, given): (&str, i32),
    folder: &str,
    program: Option<(&str, &str)>,
    stuck: bool,
) {
    let mut programs: Vec<_> = program.into_iter().collect();
    if stuck {
        programs.push(("rm", STUCK_COPY));
    }
    let (code, err, home) = far_line_in(
        Path::new(folder),
        Left::Whole,
        &[("a", "1")],
        "touch ../ran",
        &kind(""),
        None,
        &programs,
    );
    let runs = work(&home);
    let (copy, run) = (
        runs.join("1").exists(),
        runs.join(format!("1{RUN}")).exists(),
    );
    assert_eq!(code, Some(given), "a job meant to have {end} didn't: {err}");
    assert!(
        (copy, run) == (stuck, stuck),
        "a job {end} with a copy that {} left its copy {copy} and run's file {run} \
         ({code:?}): {err}",
        if stuck { "won't go" } else { "goes" }
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_run_the_sweep_may_not_signal_is_kept_as_though_alive() {
    let root = scratch("far-swept-unsignalled");
    let runs = root.join("s");
    copy(&runs, "2", &format!("{} {}", gone(), here()));
    // A kill refused as one of another account's or from inside a
    // fence is, which a Linux build's sandbox, all one account, can't
    // give for real. A function stands before the shell's own kill.
    let refused = "kill() { echo \"kill: ($2) - Operation not permitted\" >&2; return 1; }";
    sweep_after(&runs.join("9"), refused);
    assert_eq!(
        left_in(&runs),
        ["2", "2.run"],
        "a run the sweep wasn't let signal was taken as gone"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_link_at_a_run_s_file_s_name_is_never_written_through() {
    let (code, err, home) = with_a_link_at_the_run_s_file(&[]);
    assert!(
        code == Some(0) && work(&home).join("ran").exists(),
        "the job never ran past the link at its run's file ({code:?}): {err}"
    );
    assert_eq!(
        std::fs::read_to_string(home.join("beyond")).unwrap(),
        "kept\n",
        "the run's file was written through the link at its name"
    );
    let _ = std::fs::remove_dir_all(&home);
    // Nor when the link stays, as one put there after it went would:
    // the job isn't taken, rather than written through it.
    let rm = "case \"$*\" in *'/.s/1.run') exit 0 ;; esac
PATH=${PATH#*:}
exec rm \"$@\"";
    let (code, err, home) = with_a_link_at_the_run_s_file(&[("rm", rm)]);
    assert_eq!(
        std::fs::read_to_string(home.join("beyond")).unwrap(),
        "kept\n",
        "the run's file was written through a link that stayed ({code:?}): {err}"
    );
    assert!(
        code == Some(75) && !work(&home).join("ran").exists(),
        "a job whose run's file couldn't be written ran or wasn't untaken ({code:?}): {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

/// Runs a job with a uname that, the first time it's asked, leaves a link
/// where the job's run's file goes, to a file `beyond` the copies, and
/// with `programs` ahead of the machine's own, checking the link was put.
fn with_a_link_at_the_run_s_file(programs: &[(&str, &str)]) -> (Option<i32>, String, PathBuf) {
    let uname = "PATH=${PATH#*:}
runs=\"$HOME/.cache/pi/agentic-harness.pi/quest-workspace/QEST-20261115-ABC123/.s\"
if mkdir \"$HOME/planting\" 2>/dev/null; then
  mkdir -p \"$runs\" && echo kept > \"$HOME/beyond\" &&
    ln -s \"$HOME/beyond\" \"$runs/1.run\" && touch \"$HOME/planted\"
fi
exec uname \"$@\"";
    let programs: Vec<_> = std::iter::once(("uname", uname))
        .chain(programs.iter().copied())
        .collect();
    let (code, err, home) = far_line(
        Left::Whole,
        &[("a", "1")],
        "touch ../ran",
        &kind(""),
        None,
        &programs,
    );
    assert!(
        home.join("planted").exists(),
        "no link was put at the run's file ({code:?}): {err}"
    );
    (code, err, home)
}

#[test]
fn a_link_at_the_copies_folder_is_cleared_rather_than_written_through() {
    // A uname that, the first time it's asked, leaves the copies' folder
    // a link to a folder beyond them, holding a file of its own.
    let uname = "PATH=${PATH#*:}
runs=\"$HOME/.cache/pi/agentic-harness.pi/quest-workspace/QEST-20261115-ABC123/.s\"
if mkdir \"$HOME/planting\" 2>/dev/null; then
  mkdir \"$HOME/beyond\" && touch \"$HOME/beyond/kept\" &&
    ln -s \"$HOME/beyond\" \"$runs\" && touch \"$HOME/planted\"
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
        home.join("planted").exists(),
        "no link was put at the copies' folder ({code:?}): {err}"
    );
    let runs = work(&home);
    assert!(
        code == Some(0)
            && !std::fs::symlink_metadata(&runs).unwrap().is_symlink()
            && runs.join("ran").exists(),
        "the job didn't run in a copies' folder of its own ({code:?}): {err}"
    );
    assert_eq!(
        left_in(&home.join("beyond")),
        ["kept"],
        "the far line wrote or cleared through the link at the copies' folder"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_run_s_file_holding_a_nul_and_bytes_no_text_holds_is_read_without_a_word() {
    let root = scratch("far-swept-bytes");
    let runs = root.join("s");
    // The far machine's sh under a locale that reads text as UTF-8, as a
    // Mac's own does, where its tr refuses what isn't. And bash, a far
    // machine's sh on NixOS and the one sh that warns of a NUL it reads,
    // under C, since a locale a machine lacks is a warning of its own, and
    // its NUL warning comes under any.
    let utf8 = "export LC_ALL=en_US.UTF-8 2>/dev/null";
    // On a Mac, its own tr, which refuses such bytes, rather than the GNU
    // one a dev shell puts first, which never does.
    let own_tr: &[(&str, &str)] = if cfg!(target_os = "macos") {
        &[("tr", "exec /usr/bin/tr \"$@\"")]
    } else {
        &[]
    };
    for (sh, locale) in [(far_sh(), utf8), ("bash".to_owned(), "")] {
        copy(&runs, "2", "");
        let mut run = format!("{} {}", gone(), here()).into_bytes();
        run.extend_from_slice(b"\0\n\xff\xfe\0\n");
        std::fs::write(runs.join(format!("2{RUN}")), run).unwrap();
        let said = sweep_in(&sh, &runs.join("9"), own_tr, locale);
        assert!(
            said.is_empty(),
            "the sweep in {sh} said something of the bytes: {said}"
        );
        assert!(
            left_in(&runs).is_empty(),
            "a gone run's copy was kept in {sh} for what followed its line"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn only_a_run_s_file_s_first_512_bytes_are_read() {
    let root = scratch("far-swept-bound");
    let runs = root.join("s");
    let (here, gone) = (here(), gone());
    // Spaces that make `before` and them `at` bytes long, so what follows
    // starts at byte `at` + 1. Bytes here are counted from one.
    let to = |before: &str, at: usize| " ".repeat(at - before.len());
    // Gone runs here, each read only as far as its 512th byte: one with a
    // word from its 513th, which is never heard, and one whose naming of
    // this machine (and, on Linux, its process space) ends at its 512th,
    // no space, which is heard whole. So both copies go, and a bound a
    // byte either way keeps one.
    let named = format!("{gone} {here}");
    copy(&runs, "1", &format!("{named}{}more", to(&named, 512)));
    let named_here = here.trim_end();
    let number = gone.to_string();
    let past = to(&number, 512 - named_here.len());
    copy(&runs, "2", &format!("{number}{past}{named_here}"));
    let said = sweep(&runs.join("9"));
    assert!(said.is_empty(), "the sweep said something: {said}");
    assert!(
        left_in(&runs).is_empty(),
        "the sweep read a run's file to other than its 512th byte: {:?}",
        left_in(&runs)
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
runs=\"$HOME/.cache/pi/agentic-harness.pi/quest-workspace/QEST-20261115-ABC123/.s\"
if mkdir \"$HOME/leaving\" 2>/dev/null; then
  sh -c : & gone=$!
  wait \"$gone\"
  mkdir -p \"$runs/7\" && touch \"$runs/7/f\" &&
    printf '%s %s %s\\n' \"$gone\" \"$(uname -n)\" \"$(readlink /proc/self/ns/pid 2>/dev/null)\" > \"$runs/7.run\" &&
    printf '%s' \"$runs\" > \"$HOME/left\"
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
    // The stand-in's path is where the job ran, so its copy lay where
    // the far line sweeps, and its going is the sweep's doing.
    assert_eq!(
        std::fs::read_to_string(home.join("left")).unwrap_or_default(),
        work(&home).display().to_string(),
        "no gone run's copy was left where the far line sweeps"
    );
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
case \" $* \" in *\"/.s/1.names \"*\"/.s/1 \"*) exit 1 ;; esac
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
        number,
        std::fs::read_to_string(home.join("pid")).unwrap(),
        "the run's file doesn't name the far line's own process"
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
