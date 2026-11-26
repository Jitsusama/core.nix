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
            fair_ns: None,
            credited: None,
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
            fair_ns: None,
            credited: None,
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
                    wait_us: 0.5,
                    doubted: 0,
                }
            ),
            (
                "tuple".to_string(),
                Worst {
                    thread: "Signaling".into(),
                    wait_us: 200.0,
                    doubted: 0,
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
fn an_app_s_drawing_waits_are_its_own_and_its_hidden_tabs_counted() {
    let mut then = snapshot(0.0, &[], 0, 0);
    let mut now = snapshot(10.0, &[], 0, 0);
    ran(&mut then, "chrome-ui", "Compositor", 0, 0);
    ran(&mut then, "chrome-ui", "chrome", 0, 0);
    // 3 ms over ten runs, 300 us a run; 2 ms over one, the worst.
    ran(&mut now, "chrome-ui", "Compositor", 3_000_000, 10);
    ran(&mut now, "chrome-ui", "chrome", 2_000_000, 1);
    now.lowered.insert("chrome-ui".into(), 4);
    let w = between(&then, &now, quiet_niri(), 0, "gen");
    let chrome = &w.waits["chrome-ui"];
    // 5 ms over eleven runs in all.
    assert!((chrome.mean_us - 5_000.0 / 11.0).abs() < 1e-6, "{chrome:?}");
    assert!((chrome.worst_us - 2_000.0).abs() < 1e-9, "{chrome:?}");
    // Not a call's, which a window outside a call leaves out.
    assert!(w.call.is_empty(), "{:?}", w.call);
    assert_eq!(w.lowered, BTreeMap::from([("chrome-ui".to_string(), 4)]));
    let line = serde_json::to_string(&w).unwrap();
    let back: Window = serde_json::from_str(&line).unwrap();
    assert_eq!(back.lowered, w.lowered);
    // A window with no tab lowered writes no count, and one written before
    // they were counted reads as none.
    now.lowered.clear();
    let line = serde_json::to_string(&between(&then, &now, quiet_niri(), 0, "gen")).unwrap();
    assert!(!line.contains("lowered"), "{line}");
    let back: Window = serde_json::from_str(&line).unwrap();
    assert!(back.lowered.is_empty(), "{:?}", back.lowered);
}

#[test]
fn a_window_counts_the_lesser_wait_and_carries_what_one_count_had_early() {
    let mut then = snapshot(0.0, &[], 0, 0);
    let mut now = snapshot(10.0, &[], 0, 0);
    let mut next = snapshot(20.0, &[], 0, 0);
    let drawing = |s: &mut Snapshot, wait_ns: u64, fair: u64, slices: u64| {
        ran(s, "chrome-ui", "Compositor", wait_ns, slices);
        s.threads.values_mut().for_each(|t| t.fair_ns = Some(fair));
    };
    // The schedstat books a 900 ms sleep and 2 ms of a wait still going at
    // the read; the fair count has 1 ms of waits in all.
    drawing(&mut then, 0, 0, 0);
    drawing(&mut now, 902_000_000, 1_000_000, 10);
    let w = between(&then, &now, quiet_niri(), 0, "gen");
    assert!(
        (w.waits["chrome-ui"].mean_us - 100.0).abs() < 1e-6,
        "the schedstat's sleep passed as a wait: {:?}",
        w.waits
    );
    credit(&then, &mut now);
    // The fair count books the 2 ms when that wait ends, after the read.
    drawing(&mut next, 905_000_000, 6_000_000, 20);
    let w = between(&now, &next, quiet_niri(), 0, "gen");
    assert!(
        (w.waits["chrome-ui"].mean_us - 500.0).abs() < 1e-6,
        "the wait the schedstat had at the read was lost: {:?}",
        w.waits
    );
}

#[test]
fn a_tab_shown_in_a_window_is_counted_from_where_it_rested_hidden() {
    let mut then = snapshot(0.0, &[], 0, 0);
    let mut now = snapshot(10.0, &[], 0, 0);
    // Hidden at the window's start, so resting; shown by its end.
    ran(&mut then, "chrome-ui", "Compositor", 1_000_000, 5);
    let rested = then.threads.pop_first().unwrap();
    then.resting.insert(rested.0, rested.1);
    ran(&mut now, "chrome-ui", "Compositor", 9_000_000, 9);
    let w = between(&then, &now, quiet_niri(), 0, "gen");
    // 8 ms over four runs since it rested: 2 ms a run.
    let chrome = &w.waits["chrome-ui"];
    assert!((chrome.worst_us - 2_000.0).abs() < 1e-9, "{chrome:?}");
}

#[test]
fn an_app_s_threads_blocked_in_the_kernel_say_the_most_one_was() {
    let mut then = snapshot(0.0, &[], 0, 0);
    let mut now = snapshot(10.0, &[], 0, 0);
    for s in [&mut then, &mut now] {
        ran(s, "slack-ui", "slack", 0, 0);
        ran(s, "slack-ui", "Compositor", 0, 0);
    }
    let thread = |s: &Snapshot, name: &str| {
        s.threads
            .keys()
            .find(|t| t.name == name)
            .expect("the thread was made above")
            .clone()
    };
    then.blocked.insert(thread(&then, "slack"), 1_000_000_000);
    then.blocked.insert(thread(&then, "Compositor"), 5_000_000);
    // 350 ms blocked in the window, and 100 ms.
    now.blocked.insert(thread(&now, "slack"), 1_350_000_000);
    now.blocked.insert(thread(&now, "Compositor"), 105_000_000);
    let w = between(&then, &now, quiet_niri(), 0, "gen");
    assert_eq!(
        w.waits["slack-ui"].blocked_ms,
        Some(350.0),
        "the most a thread was blocked wasn't the window's"
    );
    // niri, whose time blocked isn't kept here, says none and writes none.
    assert_eq!(
        w.waits["niri"].blocked_ms, None,
        "a time blocked not kept was said"
    );
    let line: serde_json::Value = serde_json::to_value(&w).unwrap();
    assert!(
        line["waits"]["niri"].get("blocked_ms").is_none(),
        "a time blocked not kept was written: {line}"
    );
    // A row written before it was kept reads as none kept.
    let old: Wait = serde_json::from_str(r#"{"mean_us": 1.0, "worst_us": 2.0}"#).unwrap();
    assert_eq!(old.blocked_ms, None, "an old row read as blocked");
}

fn sched(wait_ns: u64, slices: u64) -> Sched {
    Sched {
        run_ns: 0,
        wait_ns,
        slices,
        fair_ns: None,
        credited: None,
    }
}

#[test]
fn a_timer_s_sleep_booked_as_one_read_s_wait_is_left_out_and_counted() {
    let mut then = snapshot(0.0, &[], 0, 0);
    let mut now = snapshot(10.0, &[], 0, 0);
    ran(&mut then, "chrome-ui", "Compositor", 0, 0);
    ran(&mut then, "chrome-ui", "chrome", 0, 0);
    ran(&mut now, "chrome-ui", "Compositor", 3_000_000, 10);
    // The main thread ran three times and waited 300 us in all, but one
    // 2 s read gained its timer's 4 s sleep, which unguarded reads as
    // 1,333 ms a run, as Chrome's did at 02:47 on 21 Nov.
    ran(&mut now, "chrome-ui", "chrome", 4_000_300_000, 3);
    // A call's audio encoder ran twice for 100 us of waiting, and one read
    // gained the same kind of sleep.
    ran(&mut then, "tuple", "AudioEncoder", 0, 0);
    ran(&mut now, "tuple", "AudioEncoder", 4_000_100_000, 3);
    let chrome = now.threads.keys().find(|t| t.name == "chrome").unwrap();
    let encoder = now
        .threads
        .keys()
        .find(|t| t.name == "AudioEncoder")
        .unwrap();
    let before = BTreeMap::from([
        (chrome.clone(), sched(100_000, 1)),
        (encoder.clone(), sched(50_000, 1)),
    ]);
    let after = BTreeMap::from([
        (chrome.clone(), sched(4_000_200_000, 2)),
        (encoder.clone(), sched(4_000_050_000, 2)),
    ]);
    let asleep = BTreeSet::from([chrome.clone(), encoder.clone()]);
    let mut booked = BTreeMap::new();
    book(&mut booked, &before, &asleep, &after, 2.0);
    now.booked = booked;
    let w = between(&then, &now, quiet_niri(), 0, "gen");
    let call = &w.call["tuple"];
    assert!((call.wait_us - 50.0).abs() < 1e-9, "{call:?}");
    assert_eq!(call.doubted, 1, "the call's booked read wasn't counted");
    let waits = &w.waits["chrome-ui"];
    // The main thread keeps 200 us over two runs; the compositor's
    // 300 us a run is the worst.
    assert!((waits.worst_us - 300.0).abs() < 1e-9, "{waits:?}");
    assert!((waits.mean_us - 3_200.0 / 12.0).abs() < 1e-6, "{waits:?}");
    assert_eq!(waits.doubted, 1, "the booked read wasn't counted");
    let line = serde_json::to_string(&w).unwrap();
    assert!(line.contains("\"doubted\":1"), "{line}");
    // A window with none doubted writes no count, and an older one reads
    // as none.
    now.booked.clear();
    let line = serde_json::to_string(&between(&then, &now, quiet_niri(), 0, "gen")).unwrap();
    assert!(!line.contains("doubted"), "{line}");
    let back: Window = serde_json::from_str(&line).unwrap();
    assert_eq!(back.waits["chrome-ui"].doubted, 0, "{line}");
}

#[test]
fn only_a_read_that_waited_longer_than_the_time_between_is_booked() {
    let thread = |name: &str| Thread {
        label: "slack-ui",
        name: name.into(),
        schedstat: format!("/proc/slack/{name}").into(),
        call: false,
    };
    let before = BTreeMap::from([
        (thread("starved"), sched(0, 0)),
        (thread("waited"), sched(0, 0)),
        (thread("drifted"), sched(0, 0)),
        (thread("slept"), sched(0, 0)),
        (thread("unread"), sched(0, 0)),
    ]);
    // Ready at the read before were starved and unread, whose state
    // wasn't read; the rest slept.
    let asleep = BTreeSet::from([thread("waited"), thread("drifted"), thread("slept")]);
    let after = BTreeMap::from([
        // Ready at the read before and given a CPU 3 s later, which the
        // kernel adds all at once: a real starvation, kept.
        (thread("starved"), sched(3_000_000_000, 1)),
        // Woke and waited nearly all of a 2 s read: kept.
        (thread("waited"), sched(1_990_000_000, 1)),
        // Past it by less than a read's drift: kept.
        (thread("drifted"), sched(2_040_000_000, 1)),
        // A 10 s timer's sleep: booked.
        (thread("slept"), sched(9_999_000_000, 1)),
        // With no state read before, nothing says it slept: kept.
        (thread("unread"), sched(9_999_000_000, 1)),
        // New since the read before: nothing to compare, so kept.
        (thread("new"), sched(9_999_000_000, 1)),
    ]);
    let mut booked = BTreeMap::new();
    book(&mut booked, &before, &asleep, &after, 2.0);
    let names: Vec<&str> = booked.keys().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["slept"], "{booked:?}");
    assert_eq!(booked[&thread("slept")], sched(9_999_000_000, 1));
    // A second booked read in the same window adds to the first.
    let later = BTreeMap::from([(thread("slept"), sched(13_999_000_000, 3))]);
    book(&mut booked, &after, &asleep, &later, 2.0);
    assert_eq!(
        booked[&thread("slept")],
        sched(13_999_000_000, 3),
        "{booked:?}"
    );
}

#[test]
fn an_app_s_processes_that_go_unwatched_are_counted_in_the_window() {
    let then = snapshot(0.0, &[], 0, 0);
    let mut now = snapshot(10.0, &[], 0, 0);
    now.unwatched.insert("chrome-ui".into(), 2);
    let w = between(&then, &now, quiet_niri(), 0, "gen");
    assert_eq!(w.unwatched, BTreeMap::from([("chrome-ui".to_string(), 2)]));
    let line = serde_json::to_string(&w).unwrap();
    let back: Window = serde_json::from_str(&line).unwrap();
    assert_eq!(back.unwatched, w.unwatched);
    // None unwatched writes no count, and an older window reads as none.
    now.unwatched.clear();
    let line = serde_json::to_string(&between(&then, &now, quiet_niri(), 0, "gen")).unwrap();
    assert!(!line.contains("unwatched"), "{line}");
    let back: Window = serde_json::from_str(&line).unwrap();
    assert!(back.unwatched.is_empty(), "{:?}", back.unwatched);
}

#[test]
fn answers_give_their_quantiles() {
    let a = Answers::of(&[3.0, 1.0, 2.0], 0);
    assert_eq!((a.p50_ms, a.max_ms, a.asked), (2.0, 3.0, 3));
    assert_eq!(Answers::of(&[], 2).asked, 2);
}
