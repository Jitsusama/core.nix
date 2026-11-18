//! The day's tally, case by case: what each kind of line counts toward.

use super::*;

/// A line of `thing`'s at `priority`, saying `message`.
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
        restart: false,
        kernel: false,
        boot: "b".to_owned(),
        since_boot: 1,
        mark: None,
        ask: None,
        job: None,
    }
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
fn an_agents_crash_and_refusal_are_not_left_out_as_agents_errors() {
    let mut day = Day::new("2026-11-16");
    day.take(&Entry {
        crashed: Some("tec".to_owned()),
        whose: Some(Whose::Agents),
        agents: true,
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
        (tec.crashes, tec.agents_errors, tec.figured_errors()),
        (1, 0, 1),
        "a crash is left out once, as a crash, not again as an agent's"
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
