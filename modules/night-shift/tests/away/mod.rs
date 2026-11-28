//! night-shift run on the stand-in machine while Joel is away: a slice's
//! bar and the machine's ease, and hold again once he's back, at the
//! machine or on a call.

use super::{MANAGER, Machine, raise, said};
use std::fs::{self, File};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant, SystemTime};

/// The options easing app.slice's bar while Joel is away, with rhythm's
/// last minute seen `ago` seconds back, or no minutes at all with None,
/// and `more` after them.
fn eased(machine: &Machine, ago: Option<u64>, more: &str) -> String {
    let seen = machine.path("state/rhythm/seen");
    if let Some(ago) = ago {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        fs::create_dir_all(seen.parent().unwrap()).unwrap();
        fs::write(&seen, format!("{}\n{}\n", now - ago - 60, now - ago)).unwrap();
    }
    format!(
        "--stop app.slice=1 --away-mark {} --away-seen {} --away-after 60 \
         --away-stop app.slice=50 {more}",
        machine.path("runtime/someone-at-the-machine").display(),
        seen.display()
    )
}

/// A job stalling app.slice at 1.5% for a second, past its own bar and
/// under its eased one.
fn stalls_the_apps(machine: &Machine) -> String {
    let app = format!("cgroups/{MANAGER}/app.slice/memory.pressure");
    format!("{}; sleep 1", raise(machine, &app, 1.5))
}

#[test]
fn with_joel_away_an_eased_bar_lets_the_job_run_on() {
    let machine = Machine::new();
    let output = machine.run(&eased(&machine, Some(3600), ""), &stalls_the_apps(&machine));
    let said = said(&output);
    assert!(output.status.success(), "{said}");
    assert!(
        said.contains("Joel is away, so its eased bars hold"),
        "{said}"
    );
    assert!(said.contains("finished"), "it was stopped: {said}");
    let samples = fs::read_to_string(machine.state("test-job.samples")).unwrap();
    assert!(
        samples.contains("app.slice 1.50 own 0.00 away"),
        "{samples}"
    );
}

#[test]
fn with_joel_away_an_eased_bar_still_stops_the_job_past_it() {
    let machine = Machine::new();
    let app = format!("cgroups/{MANAGER}/app.slice/memory.pressure");
    let job = format!("{}; sleep 30", raise(&machine, &app, 60.0));
    let began = Instant::now();
    let output = machine.run(&eased(&machine, Some(3600), ""), &job);
    let said = said(&output);
    assert!(output.status.success(), "{said}");
    assert!(said.contains("Joel is away"), "{said}");
    assert!(
        said.contains("stopping: app.slice's memory full avg10 is 60.00%, past 50%"),
        "an eased bar let a stall past it go on: {said}"
    );
    assert!(
        began.elapsed() < Duration::from_secs(20),
        "it waited for the job"
    );
}

#[test]
fn with_joel_away_the_machines_eased_bar_holds_and_still_stops() {
    for (machine_at, stopped) in [(8.0, false), (15.0, true)] {
        let machine = Machine::new();
        let job = format!(
            "{}; sleep 1",
            raise(&machine, "proc/pressure/memory", machine_at)
        );
        let output = machine.run(
            &eased(
                &machine,
                Some(3600),
                "--stop-machine 5 --away-stop-machine 12",
            ),
            &job,
        );
        let said = said(&output);
        assert!(output.status.success(), "at {machine_at}: {said}");
        assert!(said.contains("Joel is away"), "at {machine_at}: {said}");
        assert_eq!(
            said.contains(&format!("memory full avg10 is {machine_at:.2}%, past 12%")),
            stopped,
            "at {machine_at}, the machine's eased bar of 12 wasn't the one held: {said}"
        );
        assert_eq!(
            said.contains("finished"),
            !stopped,
            "at {machine_at}: {said}"
        );
    }
}

#[test]
fn with_no_minutes_to_read_joel_counts_as_away_once_gone_long_enough() {
    let machine = Machine::new();
    let app = format!("cgroups/{MANAGER}/app.slice/memory.pressure");
    let job = format!("sleep 2; {}; sleep 1", raise(&machine, &app, 1.5));
    let output = machine.run(&eased(&machine, None, "--away-after 1"), &job);
    let said = said(&output);
    assert!(output.status.success(), "{said}");
    assert!(
        said.contains("Joel is away, so its eased bars hold"),
        "no minutes to read kept him here for good: {said}"
    );
    assert!(said.contains("finished"), "it was stopped: {said}");
}

#[test]
fn with_joel_here_or_lately_here_its_own_bar_holds() {
    for (why, ago, mark) in [
        ("seen half a minute ago", Some(30), false),
        ("marked at the machine", Some(3600), true),
        ("no minutes to read", None, false),
    ] {
        let machine = Machine::new();
        if mark {
            fs::create_dir_all(machine.path("runtime")).unwrap();
            File::create(machine.path("runtime/someone-at-the-machine")).unwrap();
        }
        let output = machine.run(&eased(&machine, ago, ""), &stalls_the_apps(&machine));
        let said = said(&output);
        assert!(output.status.success(), "{why}: {said}");
        assert!(
            said.contains("stopping: app.slice's memory full avg10 is 1.50%, past 1%"),
            "{why}: {said}"
        );
        assert!(!said.contains("Joel is away"), "{why}: {said}");
    }
}

/// The shell on the path, which the build's sandbox has though it has no
/// /bin/sh, for a stand-in program's first line.
fn shell() -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|folder| folder.join("sh"))
        .find(|sh| sh.exists())
        .expect("no sh on the path for the stand-in call program")
}

/// A stand-in call program in the machine's folder running `body`.
fn call_program(machine: &Machine, body: &str) -> PathBuf {
    let program = machine.path("a-call-is-on");
    fs::write(&program, format!("#!{}\n{body}\n", shell().display())).unwrap();
    Command::new("chmod")
        .arg("+x")
        .arg(&program)
        .status()
        .unwrap();
    program
}

#[test]
fn a_call_on_or_one_it_cant_tell_about_keeps_its_own_bar() {
    for (why, body, stopped, says) in [
        (
            "a call",
            "exit 0",
            true,
            "a call is on, so Joel counts as here",
        ),
        ("no call", "exit 1", false, ""),
        (
            "can't tell",
            "exit 69",
            true,
            "the call program can't tell (it exited 69), so it counts as a call",
        ),
    ] {
        let machine = Machine::new();
        let program = call_program(&machine, body);
        let options = eased(
            &machine,
            Some(3600),
            &format!("--away-call {}", program.display()),
        );
        let output = machine.run(&options, &stalls_the_apps(&machine));
        let said = said(&output);
        assert!(output.status.success(), "{why}: {said}");
        assert_eq!(said.contains("past 1%"), stopped, "{why}: {said}");
        assert!(said.contains(says), "{why} wasn't said: {said}");
    }
}

#[test]
fn a_call_program_that_wont_start_counts_as_a_call() {
    let machine = Machine::new();
    let options = eased(
        &machine,
        Some(3600),
        &format!("--away-call {}", machine.path("no-such-program").display()),
    );
    let output = machine.run(&options, &stalls_the_apps(&machine));
    let said = said(&output);
    assert!(output.status.success(), "{said}");
    assert!(
        said.contains("past 1%"),
        "a missing program eased the bar: {said}"
    );
    assert!(
        said.contains("the call program won't start"),
        "it didn't say why he counts as here: {said}"
    );
}

#[test]
fn a_call_program_that_hangs_holds_no_stop_off() {
    let machine = Machine::new();
    let program = call_program(&machine, "exec sleep 60");
    let app = format!("cgroups/{MANAGER}/app.slice/memory.pressure");
    let job = format!("{}; sleep 30", raise(&machine, &app, 1.5));
    let options = eased(
        &machine,
        Some(3600),
        &format!("--away-call {}", program.display()),
    );
    let began = Instant::now();
    let output = machine.run(&options, &job);
    let said = said(&output);
    assert!(output.status.success(), "{said}");
    assert!(
        said.contains("stopping: app.slice's memory full avg10 is 1.50%, past 1%"),
        "a hung call program eased the bar: {said}"
    );
    assert!(
        began.elapsed() < Duration::from_secs(20),
        "a hung call program held the stop up for {:?}",
        began.elapsed()
    );
}

#[test]
fn joel_coming_back_brings_its_own_bar_back() {
    let machine = Machine::new();
    let app = format!("cgroups/{MANAGER}/app.slice/memory.pressure");
    fs::create_dir_all(machine.path("runtime")).unwrap();
    let job = format!(
        "{}; sleep 0.5; touch '{}'; sleep 30",
        raise(&machine, &app, 1.5),
        machine.path("runtime/someone-at-the-machine").display()
    );
    let began = Instant::now();
    let output = machine.run(&eased(&machine, Some(3600), ""), &job);
    let said = said(&output);
    assert!(output.status.success(), "{said}");
    assert!(
        began.elapsed() < Duration::from_secs(20),
        "it waited for the job"
    );
    let gone = said
        .find("Joel is away")
        .unwrap_or_else(|| panic!("it never said Joel went: {said}"));
    let back = said
        .find("Joel is back, so its own bars hold again")
        .unwrap_or_else(|| panic!("it never said Joel came back: {said}"));
    let stop = said.find("stopping: app.slice's memory full avg10 is 1.50%, past 1%");
    assert!(gone < back && stop.is_some_and(|at| back < at), "{said}");
}

#[test]
fn an_eased_bar_with_nothing_to_say_who_is_here_is_refused() {
    let machine = Machine::new();
    let output = machine.run("--stop app.slice=1 --away-stop app.slice=50", "true");
    assert!(!output.status.success(), "an eased bar ran with no mark");
    assert!(said(&output).contains("--away-mark"), "{}", said(&output));
}
