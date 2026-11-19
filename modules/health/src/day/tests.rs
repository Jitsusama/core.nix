//! The day's tally, case by case: what each kind of line counts toward.

use super::*;

/// A line of `thing`'s at `priority`, saying `message`, from its own unit:
/// his shell's own when it's the launcher's, elsewhere when not.
pub(crate) fn entry(thing: &str, priority: u8, message: &str) -> Entry {
    Entry {
        at: 1,
        priority,
        thing: thing.to_owned(),
        message: message.to_owned(),
        ran_in: Some(format!("{thing}.service")),
        crashed: None,
        whose: None,
        agents: false,
        origin: if thing == launcher::IDENTIFIER {
            Origin::Shell
        } else {
            Origin::Elsewhere
        },
        restart: false,
        kernel: false,
        boot: "b".to_owned(),
        account: None,
        since_boot: 1,
        mark: None,
        ask: None,
        job: None,
    }
}

#[test]
fn a_programs_ready_is_never_taken_from_the_jobs_slice_nor_from_nowhere() {
    let mut day = Day::new("2026-11-17");
    let ready = "notices: ready in 12 ms";
    for origin in [Origin::Jobs, Origin::Unknown] {
        day.take(&Entry {
            origin,
            ..entry("notices", 6, ready)
        });
    }
    assert!(
        day.launches.ready.is_empty(),
        "a ready a test or a lab said was taken as his program's"
    );
    day.take(&Entry {
        origin: Origin::Elsewhere,
        ..entry("notices", 6, ready)
    });
    assert_eq!(
        day.launches.ready.get("notices"),
        Some(&vec![12]),
        "his program's ready is taken"
    );
}

#[test]
fn a_windows_opening_is_taken_only_from_the_windows_daemon() {
    let mut day = Day::new("2026-11-17");
    let opened = "opened foot 212 ms after its process started";
    day.take(&Entry {
        ran_in: Some("run-u9.service".to_owned()),
        ..entry(launches::IDENTIFIER, 6, opened)
    });
    assert!(
        day.launches.opened.is_empty(),
        "a command writing itself as windows timed an opening"
    );
    day.take(&entry(launches::IDENTIFIER, 6, opened));
    assert_eq!(
        day.launches.opened.get("foot"),
        Some(&vec![212]),
        "the windows daemon's own opening is taken"
    );
}

#[test]
fn a_pis_start_is_taken_wherever_it_ran() {
    let mut day = Day::new("2026-11-17");
    for origin in [Origin::Jobs, Origin::Unknown, Origin::Elsewhere] {
        day.take(&Entry {
            origin,
            ..entry("pi", 6, "pi: ready in 889 ms, session 01a1d0f7")
        });
    }
    assert_eq!(
        day.launches.pi.get("pi"),
        Some(&vec![889, 889, 889]),
        "a council's pi, a busy session moved to the jobs slice and a quick one are all pi starts"
    );
}

#[test]
fn a_boot_is_timed_to_a_ready_launcher_without_the_password() {
    let mut day = Day::new("2026-11-11");
    let at = |since_boot, mark| Entry {
        since_boot,
        mark: Some(mark),
        ..entry("systemd", 6, "")
    };
    day.take(&at(2_900_000, Mark::Asked));
    day.take(&at(26_200_000, Mark::Asked));
    day.take(&at(23_900_000, Mark::Opened));
    day.take(&at(37_700_000, Mark::Ready));
    day.take(&at(48_900_000, Mark::Ready));
    let boot = &day.boots["b"];
    assert_eq!(boot.to_ready(), None, "not until systemd says it started");
    day.take(&at(
        55_700_000,
        Mark::Started {
            firmware: 9_700_000,
            loader: 500_000,
            kernel: 400_000,
            initrd: 24_400_000,
            userspace: 20_700_000,
        },
    ));
    let boot = &day.boots["b"];
    assert_eq!(
        boot.password(),
        21_000_000,
        "the first ask to the disk open"
    );
    assert_eq!(
        boot.to_ready(),
        Some(9_700_000 + 500_000 + 37_700_000 - 21_000_000),
        "firmware and loader, then to the first ready, less the password"
    );
    assert!(day.things.is_empty(), "a boot's moments aren't warnings");
}

#[test]
fn a_shutdown_runs_from_its_ask_to_the_boots_last_entry() {
    let mut day = Day::new("2026-11-11");
    day.ended("b", 9);
    assert!(day.boots.is_empty(), "a boot not seen stopping is left out");
    day.take(&Entry {
        at: 10_000_000,
        mark: Some(Mark::Stopping),
        ..entry("systemd-logind", 5, "System is rebooting.")
    });
    assert_eq!(day.boots["b"].shutdown(), None, "not until it ended");
    day.ended("b", 40_000_000);
    assert_eq!(
        day.boots["b"].shutdown(),
        Some(30_000_000),
        "from the ask to its last entry"
    );
}

#[test]
fn a_resumed_day_keeps_only_the_faults_health_saw_of_a_thing_the_journals_say_too() {
    let mut kept = Day::new("2026-11-12");
    kept.take(&entry("disk-guard", 3, "can't read the status"));
    kept.take(&entry("disk-guard", 3, "can't read the status"));
    kept.fault("disk-guard", Kind::Services, "stopped saying it's there", 5);
    assert_eq!(kept.things["disk-guard"].errors, 3, "two heard, one seen");
    let mut day = Day::new("2026-11-12");
    day.resume(kept);
    let thing = &day.things["disk-guard"];
    assert_eq!(
        (thing.lines, thing.errors),
        (1, 1),
        "the journal's two lines, which its replay says again, were kept as well: {thing:?}"
    );
    let said: Vec<_> = thing
        .signatures
        .values()
        .map(|said| (said.example.as_str(), said.count, said.priority, said.first))
        .collect();
    assert_eq!(
        said,
        [("stopped saying it's there", 1, 3, 5)],
        "the fault, as it was seen"
    );
    day.take(&entry("disk-guard", 3, "can't read the status"));
    assert_eq!(
        day.things["disk-guard"].errors, 2,
        "the replay's line counts beside the kept fault"
    );
}

#[test]
fn a_day_resumed_across_midnight_keeps_no_faults_and_sees_nothing_wrong() {
    let mut kept = Day::new("2026-11-11");
    kept.fault("lid", Kind::Devices, "the lid reads shut", 5);
    kept.seeing = Seeing {
        lid: true,
        guards: vec![PathBuf::from("/run/user/1000/disk-guard/status.json")],
        unseen_calls: vec!["skiff".to_owned()],
    };
    let mut day = Day::new("2026-11-12");
    day.resume(kept);
    assert!(
        day.things.is_empty() && day.seen.is_empty(),
        "yesterday's fault was kept"
    );
    assert_eq!(
        day.seeing,
        Seeing::default(),
        "yesterday's lid, guard or unseen machine, still seen wrong, would go uncounted today"
    );
}

#[test]
fn every_fault_of_a_thing_is_kept_through_a_start() {
    let mut kept = Day::new("2026-11-12");
    kept.fault("lid", Kind::Devices, "the lid reads shut", 5);
    kept.fault("lid", Kind::Devices, "the lid reads shut", 9);
    let mut day = Day::new("2026-11-12");
    day.resume(kept);
    let mut again = Day::new("2026-11-12");
    again.resume(day);
    let thing = &again.things["lid"];
    let said: Vec<_> = thing
        .signatures
        .values()
        .map(|said| (said.count, said.first, said.last))
        .collect();
    assert_eq!(
        (thing.errors, said, again.seen.len()),
        (2, vec![(2, 5, 9)], 2),
        "a start kept fewer of the lid's faults than it had: {thing:?}"
    );
}

/// Joel's words, typed at 4 to a session that ran at `place`.
fn words(place: &str, day: &mut Day) {
    day.heard(
        place,
        &Heard {
            at: 4,
            said: Said::Words {
                text: "why is this so slow".to_owned(),
            },
        },
    );
}

#[test]
fn joels_words_where_a_fault_is_named_the_same_outlast_two_starts() {
    // A session's place may be named as a fault's thing is: a clone
    // called disk-guard, say. The fault seen first once made the thing a
    // service's, which a start doesn't keep, so his words were lost.
    let mut kept = Day::new("2026-11-12");
    kept.fault("disk-guard", Kind::Services, "stopped saying it's there", 3);
    words("disk-guard", &mut kept);
    let mut day = Day::new("2026-11-12");
    day.resume(kept);
    let mut again = Day::new("2026-11-12");
    again.resume(day);
    let counted = |name: &str| {
        again
            .things
            .get(name)
            .map(|thing| (thing.kind, thing.lines, thing.errors))
    };
    assert_eq!(
        (counted("pi in disk-guard"), counted("disk-guard")),
        (
            Some((Some(Kind::Words), 1, 1)),
            Some((Some(Kind::Services), 1, 1))
        ),
        "his words and the fault weren't each kept once: {:?}",
        again.things
    );
    assert_eq!(again.seen.len(), 1, "the fault was seen again");
}

#[test]
fn joels_words_where_a_unit_is_named_the_same_leave_its_lines_to_the_replay() {
    // His words heard first once made a unit's lines the session's, kept
    // whole through a start, so the replay counted them a second time.
    let mut kept = Day::new("2026-11-12");
    words("disk-guard", &mut kept);
    kept.take(&entry("disk-guard", 3, "can't read the status"));
    let mut day = Day::new("2026-11-12");
    day.resume(kept);
    day.take(&entry("disk-guard", 3, "can't read the status"));
    let errors = |name: &str| day.things.get(name).map(|thing| thing.errors);
    assert_eq!(
        (errors("disk-guard"), errors("pi in disk-guard")),
        (Some(1), Some(1)),
        "the unit's line was counted twice, or his words lost: {:?}",
        day.things
    );
}

#[test]
fn a_day_written_before_faults_were_kept_reads_as_one_keeping_none() {
    let day: Day = serde_json::from_str(r#"{"date": "2026-11-12", "to": 1, "things": {}}"#)
        .expect("an older day reads");
    assert!(
        day.seen.is_empty() && day.seeing == Seeing::default(),
        "an older day kept faults: {day:?}"
    );
}

#[test]
fn repeats_are_counted_under_one_signature() {
    let mut day = Day::new("2026-11-12");
    day.take(&entry("pebble-lights", 4, "light 3 answered in 812 ms"));
    day.take(&Entry {
        at: 9,
        ..entry("pebble-lights", 3, "light 4 answered in 90 ms")
    });
    let thing = &day.things["pebble-lights"];
    assert_eq!(
        thing.kind,
        Some(Kind::Services),
        "a service's lines are a service's"
    );
    assert_eq!(
        (thing.lines, thing.errors),
        (2, 1),
        "both lines, one an error"
    );
    let said = &thing.signatures["light # answered in # ms"];
    assert_eq!(said.count, 2, "one signature for both");
    assert_eq!(
        (said.priority, said.first, said.last),
        (3, 1, 9),
        "gravest, first, last"
    );
    assert_eq!(
        said.example, "light 3 answered in 812 ms",
        "the first said, as it was"
    );
    assert_eq!(day.to, 9, "the day runs to its last entry");
}

#[test]
fn a_crash_is_its_programs_and_says_whose() {
    let mut day = Day::new("2026-11-12");
    let crash = |whose| Entry {
        crashed: Some("chrome".to_owned()),
        whose: Some(whose),
        ..entry(
            "systemd-coredump",
            2,
            "Process 1 (chrome) of user 1000 dumped core.",
        )
    };
    day.take(&crash(Whose::Agents));
    day.take(&crash(Whose::His));
    let chrome = &day.things["chrome"];
    assert_eq!(chrome.kind, Some(Kind::Programs), "a crash is a program's");
    assert_eq!(
        (chrome.crashes, chrome.his_crashes),
        (2, 1),
        "two crashes, one his"
    );
    assert_eq!(chrome.traces, 0, "a crash isn't counted again as a trace");
    assert_eq!(
        chrome.figured_errors(),
        0,
        "a crash is counted once, as a crash, not again as an error"
    );
}

#[test]
fn what_an_agents_command_said_is_kept_but_left_out_of_the_figure() {
    let mut day = Day::new("2026-11-16");
    let scope = |agents, message| Entry {
        ran_in: Some("work-9-1.scope".to_owned()),
        agents,
        ..entry("mg-lab", 3, message)
    };
    day.take(&scope(true, "TMPDIR is not writable: /proc"));
    day.take(&scope(true, "awk: command not found"));
    day.take(&scope(false, "could not open the display"));
    let lab = &day.things["mg-lab"];
    assert_eq!(
        (lab.lines, lab.errors, lab.agents_errors),
        (3, 3, 2),
        "every line is kept, and the agents' are told apart"
    );
    assert_eq!(
        lab.figured_errors(),
        1,
        "only the one not an agent's counts"
    );
    assert_eq!(
        lab.signatures.len(),
        3,
        "what agents said is still there to read"
    );
}

#[test]
fn an_agents_abort_is_its_crash_once_and_a_refusal_counts() {
    let mut day = Day::new("2026-11-16");
    // How tec aborted on 16 November: its own panic from agents' work,
    // then systemd-coredump's report of the crash, from outside it.
    day.take(&Entry {
        ran_in: Some("work-9-1.scope".to_owned()),
        agents: true,
        ..entry("tec", 3, "thread 'main' panicked at src/version.rs:40:6")
    });
    day.take(&Entry {
        crashed: Some("tec".to_owned()),
        whose: Some(Whose::Agents),
        ..entry(
            "systemd-coredump",
            2,
            "Process 1 (tec) of user 1000 dumped core.",
        )
    });
    day.take(&entry("tec", 3, "could not read the config"));
    day.take(&Entry {
        agents: true,
        ..entry("sudo", 3, "pam_unix(sudo:auth): authentication failure")
    });
    let tec = &day.things["tec"];
    assert_eq!(
        (
            tec.crashes,
            tec.traces,
            tec.agents_errors,
            tec.figured_errors()
        ),
        (1, 0, 1, 1),
        "the crash and the agent's panic are each left out once; his own error counts"
    );
    assert_eq!(
        (tec.badness().traces, tec.badness().lines),
        (0, 2),
        "an agent's panic ranks as neither a trace nor a line of his"
    );
    assert_eq!(
        day.things["sudo"].figured_errors(),
        1,
        "a refusal counts whoever was refused"
    );
}

#[test]
fn a_trace_and_a_restart_are_counted() {
    let mut day = Day::new("2026-11-12");
    day.take(&entry(
        "quickshell",
        4,
        "qrc:/Now.qml:3: TypeError: x is null",
    ));
    day.take(&Entry {
        restart: true,
        ..entry(
            "quickshell.service",
            6,
            "quickshell.service: Scheduled restart job",
        )
    });
    assert_eq!(day.things["quickshell"].traces, 1, "the trace is counted");
    let service = &day.things["quickshell.service"];
    assert_eq!(
        (service.restarts, service.lines),
        (1, 0),
        "a restart isn't a warning"
    );
}

#[test]
fn the_kernel_speaks_of_devices_and_the_audit_log() {
    let mut day = Day::new("2026-11-12");
    let kernel = |message| Entry {
        kernel: true,
        ..entry("kernel", 3, message)
    };
    day.take(&kernel("iwlwifi 0000:00:14.3: Microcode SW error detected"));
    day.take(&kernel("nvme0n1: I/O error"));
    day.take(&kernel("audit: type=1400 apparmor=\"DENIED\""));
    assert_eq!(
        day.things["iwlwifi"].kind,
        Some(Kind::Devices),
        "a driver is a device"
    );
    assert_eq!(
        day.things["nvme#n#"].kind,
        Some(Kind::Devices),
        "a disk is a device"
    );
    assert_eq!(
        day.things["audit"].kind,
        Some(Kind::Security),
        "the audit log is security"
    );
}

#[test]
fn a_device_event_is_counted_and_other_information_left_out() {
    let mut day = Day::new("2026-11-12");
    let kernel = |message| Entry {
        kernel: true,
        ..entry("kernel", 6, message)
    };
    day.take(&kernel("usb 3-2.1.3: USB disconnect, device number 22"));
    day.take(&kernel(
        "usb 2-1: reset SuperSpeed USB device number 2 using xhci_hcd",
    ));
    day.take(&kernel(
        "pcieport 0000:00:1c.0: AER: Corrected error message received from 0000:01:00.0",
    ));
    day.take(&kernel("pcieport 0000:00:1c.0: AER: enabled with IRQ 122"));
    day.take(&kernel("Bluetooth: hci0: Firmware loaded in 1534 usecs"));
    day.take(&entry("pebble-lights", 6, "USB disconnect seen"));
    let usb = &day.things["usb"];
    assert_eq!(usb.kind, Some(Kind::Devices), "a USB event is a device's");
    assert_eq!(
        (usb.events, usb.lines),
        (2, 0),
        "a disconnect and a reset are events, not warnings"
    );
    assert_eq!(
        day.things["pcieport"].events, 1,
        "a corrected error is an event; turning AER on isn't"
    );
    assert!(
        !day.things.contains_key("Bluetooth") && !day.things.contains_key("pebble-lights"),
        "other information is left out: {:?}",
        day.things.keys().collect::<Vec<_>>()
    );
}

#[test]
fn security_and_programs_are_told_apart_from_services() {
    let mut day = Day::new("2026-11-12");
    day.take(&entry(
        "sudo",
        4,
        "pam_unix(sudo:auth): conversation failed",
    ));
    day.take(&Entry {
        ran_in: Some("app-niri-kitty-1.scope".to_owned()),
        ..entry("kitty", 4, "[0.1] Failed to set locale")
    });
    assert_eq!(
        day.things["sudo"].kind,
        Some(Kind::Security),
        "sudo is security"
    );
    assert_eq!(
        day.things["kitty"].kind,
        Some(Kind::Programs),
        "a scope's is a program's"
    );
}

#[test]
fn a_thing_saying_something_new_each_line_is_kept_to_its_most() {
    let mut day = Day::new("2026-11-12");
    for n in 0..=MOST_SIGNATURES {
        let word: String = std::iter::repeat_n('a', n + 1).collect();
        day.take(&entry("chatty", 4, &word));
    }
    let chatty = &day.things["chatty"];
    assert_eq!(
        chatty.signatures.len(),
        MOST_SIGNATURES + 1,
        "its most, and the rest"
    );
    assert_eq!(
        chatty.signatures[THE_REST].count, 1,
        "the one past its most"
    );
}
