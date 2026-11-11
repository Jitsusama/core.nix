use super::*;
use crate::kernel::Thread;

fn group(cpu_some: u64, memory_full: u64, swap: u64) -> Group {
    Group {
        cpu: Stall {
            some: cpu_some,
            full: 0,
        },
        memory: Stall {
            some: memory_full,
            full: memory_full,
        },
        swap,
        ..Group::default()
    }
}

fn snapshot(at: f64, groups: &[(&str, Group)], niri_wait: u64, niri_slices: u64) -> Snapshot {
    let mut s = Snapshot {
        at,
        ..Snapshot::default()
    };
    for (name, g) in groups {
        s.groups.insert(name.to_string(), *g);
    }
    s.threads.insert(
        Thread {
            label: "niri",
            name: "niri".into(),
            schedstat: "/proc/10/schedstat".into(),
            call: false,
        },
        Sched {
            run_ns: 0,
            wait_ns: niri_wait,
            slices: niri_slices,
        },
    );
    s
}

fn quiet_niri() -> Answers {
    Answers::of(&[0.4, 0.5, 0.6, 0.5, 0.5], 0)
}

#[test]
fn work_waiting_on_work_is_no_trouble() {
    let then = snapshot(
        0.0,
        &[("session", group(0, 0, 0)), ("jobs", group(0, 0, 0))],
        0,
        0,
    );
    // jobs stalled 9 s of 10, the desktop 10 ms; niri waited 20 us a turn.
    let now = snapshot(
        10.0,
        &[
            ("session", group(10_000, 0, 0)),
            ("jobs", group(9_000_000, 0, 0)),
        ],
        20_000_000,
        1_000,
    );
    let w = between(&then, &now, quiet_niri(), 0, "gen");
    assert_eq!(w.band, Band::Loaded);
    assert!(w.trouble.is_empty(), "{:?}", w.trouble);
    assert_eq!((w.worst.as_str(), w.who.as_str()), ("", ""));
    let mean = w.waits["niri"].mean_us;
    assert!((mean - 20.0).abs() < 1e-9, "niri's mean wait is {mean} us");
}

fn waiting_on_io(us: u64) -> Group {
    Group {
        io: Stall { some: us, full: 0 },
        ..Group::default()
    }
}

#[test]
fn the_desktop_waiting_a_frame_on_a_page_or_the_disk_is_trouble() {
    let then = snapshot(0.0, &[("session", waiting_on_io(0))], 0, 0);
    // niri asleep 27 ms while a build moved a page of its memory.
    let now = snapshot(10.0, &[("session", waiting_on_io(27_000))], 0, 0);
    let w = between(&then, &now, quiet_niri(), 0, "gen");
    assert_eq!(
        w.trouble,
        ["the desktop waited on the disk or on memory being moved 0.27% of the time"]
    );
    assert_eq!((w.worst.as_str(), w.who.as_str()), ("disk", "desktop"));
}

#[test]
fn the_desktop_waiting_under_a_frame_on_the_disk_is_no_trouble() {
    let then = snapshot(0.0, &[("session", waiting_on_io(0))], 0, 0);
    let now = snapshot(10.0, &[("session", waiting_on_io(5_000))], 0, 0);
    let w = between(&then, &now, quiet_niri(), 0, "gen");
    assert!(w.trouble.is_empty(), "{:?}", w.trouble);
}

#[test]
fn niri_waiting_what_a_day_of_builds_costs_it_is_no_trouble() {
    let then = snapshot(0.0, &[("session", group(0, 0, 0))], 0, 0);
    // The worst of a morning's windows beside twenty builds and more.
    let now = snapshot(10.0, &[("session", group(0, 0, 0))], 94_000_000, 1_000);
    let w = between(&then, &now, quiet_niri(), 0, "gen");
    assert!(w.trouble.is_empty(), "{:?}", w.trouble);
}

#[test]
fn the_desktop_waiting_is_trouble() {
    let then = snapshot(0.0, &[("session", group(0, 0, 0))], 0, 0);
    let now = snapshot(
        10.0,
        &[("session", group(300_000, 0, 0))],
        300_000_000,
        1_000,
    );
    let w = between(&then, &now, quiet_niri(), 0, "gen");
    assert_eq!(w.band, Band::Quiet);
    assert_eq!(
        w.trouble,
        [
            "the desktop waited for a CPU 3.0% of the time",
            "niri waited 300 us a turn for a CPU"
        ]
    );
    assert_eq!(w.worst, "cpu");
}

#[test]
fn apps_stalling_on_memory_and_the_desktop_swapping_are_trouble() {
    let then = snapshot(
        0.0,
        &[("session", group(0, 0, 0)), ("app", group(0, 0, 0))],
        0,
        0,
    );
    let now = snapshot(
        10.0,
        &[
            ("session", group(0, 0, 64 << 20)),
            ("app", group(0, 50_000, 0)),
        ],
        0,
        0,
    );
    let w = between(&then, &now, quiet_niri(), 0, "gen");
    assert_eq!(
        w.trouble,
        [
            "apps stalled on memory 0.50% of the time",
            "the desktop went to swap, 64 MiB now"
        ]
    );
    assert_eq!(w.worst, "memory");
}

#[test]
fn the_desktop_stalling_on_memory_is_trouble_from_a_frame_on() {
    let then = snapshot(0.0, &[("session", group(0, 0, 0))], 0, 0);
    // 6 ms of 10 s, under one 120 Hz frame: most windows' stalls are
    // this size, and niri answered within a millisecond through them.
    let under = snapshot(10.0, &[("session", group(0, 6_000, 0))], 0, 0);
    let w = between(&then, &under, quiet_niri(), 0, "gen");
    assert!(w.trouble.is_empty(), "{:?}", w.trouble);
    let frame = snapshot(10.0, &[("session", group(0, 9_000, 0))], 0, 0);
    let w = between(&then, &frame, quiet_niri(), 0, "gen");
    assert_eq!(
        w.trouble,
        ["the desktop stalled on memory 0.09% of the time"]
    );
}

#[test]
fn a_program_the_kernel_ended_for_memory_is_trouble_and_named() {
    let mut then = snapshot(
        0.0,
        &[("app", group(0, 0, 0)), ("jobs", group(0, 0, 0))],
        0,
        0,
    );
    let mut now = then.clone();
    now.at = 10.0;
    let killed = |n| Group {
        killed: n,
        ..Group::default()
    };
    then.apps.insert("chrome.scope".to_string(), killed(0));
    now.apps.insert("chrome.scope".to_string(), killed(2));
    now.groups.insert("app".to_string(), killed(2));
    // Work's programs ended are no wait of Joel's.
    now.groups.insert("jobs".to_string(), killed(5));
    let w = between(&then, &now, quiet_niri(), 0, "gen");
    assert_eq!(
        w.trouble,
        ["apps lost 2 programs to the kernel's OOM killer, in chrome.scope"]
    );
    assert_eq!(w.worst, "memory");
}

#[test]
fn the_worst_is_what_joel_would_feel_most_not_what_came_first() {
    let then = snapshot(
        0.0,
        &[("session", group(0, 0, 0)), ("app", group(0, 0, 0))],
        0,
        0,
    );
    let mut now = snapshot(
        10.0,
        &[("session", group(300_000, 0, 0)), ("app", group(0, 0, 0))],
        0,
        0,
    );
    now.groups.get_mut("app").unwrap().io = Stall {
        some: 200_000,
        full: 200_000,
    };
    // The desktop's CPU wait comes first, but a disk stall of 2% is felt more.
    let w = between(&then, &now, quiet_niri(), 0, "gen");
    assert_eq!(w.trouble.len(), 2, "{:?}", w.trouble);
    assert_eq!(w.worst, "disk");
    // niri not answering is felt more than either.
    let w = between(&then, &now, Answers::of(&[0.5, 0.5], 1), 0, "gen");
    assert_eq!(w.worst, "niri");
    assert_eq!(w.who, "desktop");
    // And a program ended for memory more than anything.
    now.groups.get_mut("app").unwrap().killed = 1;
    let w = between(&then, &now, Answers::of(&[0.5, 0.5], 1), 0, "gen");
    assert_eq!((w.worst.as_str(), w.who.as_str()), ("memory", "apps"));
}

#[test]
fn trouble_that_lasts_says_since_when() {
    let then = snapshot(0.0, &[("session", group(0, 0, 0))], 0, 0);
    let busy = snapshot(10.0, &[("session", group(300_000, 0, 0))], 0, 0);
    let quiet = snapshot(10.0, &[("session", group(0, 0, 0))], 0, 0);
    assert_eq!(
        since_of(
            &then,
            &[(100, &quiet), (110, &busy), (120, &busy), (130, &quiet)]
        ),
        [None, Some(110), Some(110), None]
    );
}

#[test]
fn a_wait_does_not_last_through_the_machine_asleep() {
    let then = snapshot(0.0, &[("session", group(0, 0, 0))], 0, 0);
    let busy = snapshot(10.0, &[("session", group(300_000, 0, 0))], 0, 0);
    // The window's own clock stops while the machine sleeps, so the one
    // it wakes in reads ten seconds though it ends 160 after the last.
    assert_eq!(
        since_of(&then, &[(110, &busy), (270, &busy), (280, &busy)]),
        [Some(110), Some(270), Some(270)]
    );
}

/// Each window's `since`, carried along the windows ending at each time.
fn since_of(then: &Snapshot, windows: &[(u64, &Snapshot)]) -> Vec<Option<u64>> {
    let mut last: Option<Window> = None;
    let mut seen = Vec::new();
    for &(ended, now) in windows {
        let mut w = between(then, now, quiet_niri(), ended, "gen");
        w.carry(last.as_ref());
        seen.push(w.since);
        last = Some(w);
    }
    seen
}

#[test]
fn an_app_s_stall_names_the_unit_that_stalled_most() {
    let mut then = snapshot(0.0, &[("app", group(0, 0, 0))], 0, 0);
    let mut now = snapshot(10.0, &[("app", group(0, 225_000, 0))], 0, 0);
    for (name, stalled) in [
        ("dev.up.backend-dev.service", 220_000),
        ("kitty-1.scope", 5_000),
    ] {
        then.apps.insert(name.to_string(), group(0, 0, 0));
        now.apps.insert(name.to_string(), group(0, stalled, 0));
    }
    // One that came in the window has nothing to compare with.
    now.apps
        .insert("new.scope".to_string(), group(0, 900_000, 0));
    let w = between(&then, &now, quiet_niri(), 0, "gen");
    assert_eq!(
        w.trouble,
        ["apps stalled on memory 2.25% of the time, most in dev.up.backend-dev.service (2.20%)"]
    );
}

#[test]
fn niri_slow_to_answer_is_trouble() {
    let then = snapshot(0.0, &[], 0, 0);
    let now = snapshot(10.0, &[], 0, 0);
    let slow = Answers::of(&[0.5, 0.5, 0.5, 0.5, 12.0], 1);
    let w = between(&then, &now, slow, 0, "gen");
    assert_eq!(
        w.trouble,
        [
            "niri took over a frame to answer, 12.0 ms",
            "niri didn't answer 1 of 6 times"
        ]
    );
    assert_eq!(w.worst, "niri");
}

fn ran(s: &mut Snapshot, label: &'static str, name: &str, wait_ns: u64, slices: u64) {
    s.threads.insert(
        Thread {
            label,
            name: name.into(),
            schedstat: format!("/proc/{label}/{name}").into(),
            call: true,
        },
        Sched {
            run_ns: 0,
            wait_ns,
            slices,
        },
    );
}

#[test]
fn a_call_s_worst_thread_is_named_with_its_wait_a_run_each_program_apart() {
    let mut then = snapshot(0.0, &[], 0, 0);
    let mut now = snapshot(10.0, &[], 0, 0);
    for s in [&mut then, &mut now] {
        ran(s, "tuple", "Metrics", 0, 0);
    }
    ran(&mut then, "tuple", "AudioEncoder", 0, 0);
    ran(&mut then, "tuple", "Signaling", 0, 0);
    ran(&mut then, "chrome", "AudioOutputDevi", 1_000, 1);
    // 1 ms over ten runs, 100 us a run; 400 us over two, 200 a run.
    ran(&mut now, "tuple", "AudioEncoder", 1_000_000, 10);
    ran(&mut now, "tuple", "Signaling", 400_000, 2);
    ran(&mut now, "chrome", "AudioOutputDevi", 51_000, 101);
    let w = between(&then, &now, quiet_niri(), 0, "gen");
    assert_eq!(
        w.call,
        BTreeMap::from([
            (
                "chrome".to_string(),
                Worst {
                    thread: "AudioOutputDevi".into(),
                    wait_us: 0.5
                }
            ),
            (
                "tuple".to_string(),
                Worst {
                    thread: "Signaling".into(),
                    wait_us: 200.0
                }
            ),
        ])
    );
    // A call's threads are their own record, not the desktop's waits.
    assert_eq!(w.waits.keys().collect::<Vec<_>>(), ["niri"]);
}

#[test]
fn a_window_without_a_call_carries_nothing_new() {
    let mut then = snapshot(0.0, &[], 0, 0);
    let mut now = snapshot(10.0, &[], 0, 0);
    // A thread that didn't run in the window has no wait a run.
    ran(&mut then, "chrome", "AudioThread", 5, 1);
    ran(&mut now, "chrome", "AudioThread", 5, 1);
    let w = between(&then, &now, quiet_niri(), 0, "gen");
    assert!(w.call.is_empty(), "{:?}", w.call);
    let line = serde_json::to_string(&w).unwrap();
    assert!(!line.contains("call"), "{line}");
    // And a window written before calls were watched reads as one
    // without a call.
    let back: Window = serde_json::from_str(&line).unwrap();
    assert!(back.call.is_empty(), "{:?}", back.call);
}

#[test]
fn answers_give_their_quantiles() {
    let a = Answers::of(&[3.0, 1.0, 2.0], 0);
    assert_eq!((a.p50_ms, a.max_ms, a.asked), (2.0, 3.0, 3));
    assert_eq!(Answers::of(&[], 2).asked, 2);
}
