//! One entry of a Mac's unified log as `log show --style ndjson` and `log
//! stream --style ndjson` write it, read into the journal's entry, so a
//! Mac's day is tallied as a Linux machine's is; and what the follower asks
//! `log` for.

use std::path::Path;

use jiff::Timestamp;
use serde_json::Value;

use crate::journal::Entry;

/// Where macOS keeps `log`, named whole since a launchd agent's PATH is
/// short and zsh has a `log` of its own.
pub const LOG: &str = "/usr/bin/log";

/// The lines the day is tallied from: errors and faults, as the journal's
/// reader takes warnings and graver. A Mac's log has no warning level.
const PREDICATE: &str = "messageType == error OR messageType == fault";

/// How the unified log's levels read as syslog's priorities, so a Mac's
/// errors and faults are counted as a Linux machine's errors and critical
/// lines are.
const LEVELS: [(&str, u8); 5] = [
    ("Fault", 2),
    ("Error", 3),
    ("Default", 5),
    ("Info", 6),
    ("Debug", 7),
];

/// The subsystem the system's name lookup, libinfo, logs under from inside
/// whichever program asked it. sshd, sshd-session and sshd-auth each ask it
/// of every connection's address, and its probe failing as it sorts them,
/// "send failed: Invalid argument", would otherwise count as theirs, under
/// security, hundreds of times an hour on a Mac other machines reach.
const NAME_LOOKUP: &str = "com.apple.network.libinfo";

/// How `log` writes a moment: local time, to the microsecond, with its
/// offset.
const WRITTEN: &str = "%Y-%m-%d %H:%M:%S%.f%z";

/// The entry one line of `log --style ndjson` holds, or none when the line
/// isn't a line logged.
#[must_use]
pub fn parse(line: &str) -> Option<Entry> {
    let Ok(Value::Object(fields)) = serde_json::from_str::<Value>(line) else {
        return None;
    };
    let text = |name: &str| fields.get(name).and_then(Value::as_str);
    if text("eventType") != Some("logEvent") {
        return None;
    }
    let at = jiff::fmt::strtime::parse(WRITTEN, text("timestamp")?)
        .ok()?
        .to_timestamp()
        .ok()
        .map(Timestamp::as_microsecond)?;
    let level = text("messageType").unwrap_or_default();
    let priority = LEVELS
        .iter()
        .find(|(name, _)| *name == level)
        .map_or(6, |(_, priority)| *priority);
    let image = text("processImagePath").unwrap_or_default();
    let kernel = image == "/kernel";
    let lookup = text("subsystem") == Some(NAME_LOOKUP);
    let thing = if kernel {
        "kernel".to_owned()
    } else if lookup {
        "libinfo".to_owned()
    } else {
        Path::new(image).file_name().map_or_else(
            || "unnamed".to_owned(),
            |name| name.to_string_lossy().into_owned(),
        )
    };
    Some(Entry {
        at,
        priority,
        thing,
        message: text("eventMessage").unwrap_or_default().to_owned(),
        ran_in: if lookup { None } else { app_of(image) },
        crashed: None,
        whose: None,
        restart: false,
        kernel,
        boot: text("bootUUID").unwrap_or_default().to_owned(),
        // The log gives the kernel's clock in its own ticks, not
        // microseconds, and no line here marks a boot's stage to need it.
        since_boot: 0,
        mark: None,
        ask: None,
        job: None,
    })
}

/// The app a program ran from, when its image lies in a bundle: the
/// bundle's path, which is to a Mac what a program's scope is to systemd.
fn app_of(image: &str) -> Option<String> {
    let end = image.find(".app/")? + ".app".len();
    Some(image[..end].to_owned())
}

/// What the follower asks `log` for: the lines from now on, and the day's
/// lines before `now`, from `midnight`, both as local time `log` reads. A
/// line said in the moment `log stream` takes to start may be missed, once
/// a run, rather than counted twice.
#[must_use]
pub fn followed(midnight: &str, now: &str) -> Vec<Vec<String>> {
    let asked = |words: &[&str]| words.iter().map(|word| (*word).to_owned()).collect();
    vec![
        asked(&[
            "stream",
            "--style",
            "ndjson",
            "--type",
            "log",
            "--predicate",
            PREDICATE,
        ]),
        asked(&[
            "show",
            "--style",
            "ndjson",
            "--start",
            midnight,
            "--end",
            now,
            "--predicate",
            PREDICATE,
        ]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::day::{Day, Kind};

    /// A line a Mac's runningboardd wrote at fault, cut to the fields the
    /// unified log always gives.
    const FAULT: &str = r#"{"timestamp":"2026-11-12 05:55:36.023558-0500","messageType":"Fault",
        "eventType":"logEvent","processImagePath":"/usr/libexec/runningboardd","processID":668,
        "subsystem":"com.apple.runningboard","category":"process",
        "bootUUID":"F9D442DF-CC61-4F75-952B-4A09E737DE13","machTimestamp":6906630924156,
        "userID":0,"eventMessage":"Two equal instances have unequal identities."}"#;

    /// A line the kernel wrote at error.
    const KERNEL: &str = r#"{"timestamp":"2026-11-12 05:55:36.278395-0500","messageType":"Error",
        "eventType":"logEvent","processImagePath":"/kernel","processID":0,"subsystem":"",
        "category":"","bootUUID":"F9D442DF-CC61-4F75-952B-4A09E737DE13",
        "machTimestamp":6906637040237,"userID":0,
        "eventMessage":"vm_reclaim: [80186] Failed to free(reusable) err=1"}"#;

    /// A line an app's own program wrote at error, from inside its bundle.
    const APP: &str = r#"{"timestamp":"2026-11-12 05:56:31.630011-0500","messageType":"Error",
        "eventType":"logEvent",
        "processImagePath":"/Applications/Xcode.app/Contents/Developer/usr/bin/xcodebuild",
        "processID":80324,"subsystem":"com.apple.foundation","category":"general",
        "bootUUID":"F9D442DF-CC61-4F75-952B-4A09E737DE13","machTimestamp":6907965479034,
        "userID":501,"eventMessage":"Error loading IDESimulatorFoundation (99)"}"#;

    /// A line the system's name lookup wrote at error from inside sshd,
    /// as it sorted a connection's addresses.
    const LOOKUP: &str = r#"{"timestamp":"2026-11-13 15:57:02.114210-0500","messageType":"Error",
        "eventType":"logEvent","processImagePath":"/usr/sbin/sshd","processID":18567,
        "senderImagePath":"/usr/lib/system/libsystem_info.dylib",
        "subsystem":"com.apple.network.libinfo","category":"si_destination_compare",
        "bootUUID":"F9D442DF-CC61-4F75-952B-4A09E737DE13","machTimestamp":6907965479034,
        "userID":0,"eventMessage":"send failed: Invalid argument"}"#;

    fn entry(line: &str) -> Entry {
        parse(&line.replace('\n', "")).expect("a unified log line is an entry")
    }

    #[test]
    fn a_fault_is_its_programs_and_as_grave_as_a_critical_line() {
        let entry = entry(FAULT);
        assert_eq!(entry.thing, "runningboardd", "the program names the thing");
        assert_eq!(entry.priority, 2, "a fault is critical");
        assert_eq!(
            entry.at, 1_794_480_936_023_558,
            "the time is read with its offset, to the microsecond"
        );
        assert_eq!(
            entry.message, "Two equal instances have unequal identities.",
            "what it said"
        );
        assert_eq!(
            entry.boot, "F9D442DF-CC61-4F75-952B-4A09E737DE13",
            "the boot it was said in"
        );
        assert_eq!(entry.ran_in, None, "a system program runs in no app");
        assert!(
            !entry.kernel && !entry.restart && entry.crashed.is_none() && entry.mark.is_none(),
            "it's only a line"
        );
    }

    #[test]
    fn an_error_is_an_error() {
        assert_eq!(entry(KERNEL).priority, 3, "an error is an error");
    }

    #[test]
    fn the_kernels_line_is_the_kernels() {
        let entry = entry(KERNEL);
        assert!(entry.kernel, "the kernel's image is the kernel");
        assert_eq!(entry.thing, "kernel", "the kernel names the thing");
    }

    #[test]
    fn an_apps_program_ran_in_its_app() {
        let entry = entry(APP);
        assert_eq!(entry.thing, "xcodebuild", "the program names the thing");
        assert_eq!(
            entry.ran_in.as_deref(),
            Some("/Applications/Xcode.app"),
            "the bundle it ran from"
        );
    }

    #[test]
    fn the_name_lookups_line_is_the_name_lookups_wherever_it_ran() {
        let own = LOOKUP
            .replace("/usr/lib/system/libsystem_info.dylib", "/usr/sbin/sshd")
            .replace("com.apple.network.libinfo", "")
            .replace("si_destination_compare", "")
            .replace(
                "send failed: Invalid argument",
                "error: kex_exchange_identification",
            );
        let mut day = Day::new("2026-11-13");
        for line in [LOOKUP, &own] {
            day.take(&entry(line));
        }
        let thing = |name: &str| day.things.get(name).map(|thing| (thing.kind, thing.errors));
        assert_eq!(
            thing("libinfo"),
            Some((Some(Kind::Services), 1)),
            "the name lookup failing a probe is a service's, not sshd's: {:?}",
            day.things.keys().collect::<Vec<_>>()
        );
        assert_eq!(
            thing("sshd"),
            Some((Some(Kind::Security), 1)),
            "sshd's own line is still security's"
        );
    }

    #[test]
    fn what_isnt_a_logged_line_is_none() {
        assert_eq!(
            parse(r#"{"count":3989,"finished":1}"#),
            None,
            "log's count at the end isn't an entry"
        );
        assert_eq!(
            parse("Filtering the log data using \"messageType == fault\""),
            None,
            "log's words aren't an entry"
        );
        let activity = FAULT.replace('\n', "").replace(
            r#""eventType":"logEvent""#,
            r#""eventType":"activityCreateEvent""#,
        );
        assert_eq!(
            parse(&activity),
            None,
            "an activity starting isn't a line said"
        );
    }

    #[test]
    fn a_macs_lines_are_filed_as_a_linux_machines_are() {
        let sandbox = KERNEL.replace(
            "vm_reclaim: [80186] Failed to free(reusable) err=1",
            "Sandbox: rapportd(1039) deny(1) mach-lookup com.apple.AirPlayXPCHelper",
        );
        let mut day = Day::new("2026-11-12");
        for line in [FAULT, KERNEL, APP, &sandbox] {
            day.take(&entry(line));
        }
        let kind = |name: &str| day.things.get(name).and_then(|thing| thing.kind);
        assert_eq!(
            kind("runningboardd"),
            Some(Kind::Services),
            "a system program is a service"
        );
        assert_eq!(
            kind("xcodebuild"),
            Some(Kind::Programs),
            "an app's program is a program"
        );
        assert_eq!(
            kind("vm_reclaim"),
            Some(Kind::Devices),
            "the kernel's line is a device's"
        );
        assert_eq!(
            kind("Sandbox"),
            Some(Kind::Security),
            "the sandbox refusing is security's"
        );
        assert_eq!(
            day.things.get("runningboardd").map(|thing| thing.errors),
            Some(1),
            "a fault counts as an error"
        );
    }

    #[test]
    fn a_macs_kernel_line_is_named_by_what_it_speaks_of() {
        let kernel = |message: &str| {
            entry(&KERNEL.replace(
                "vm_reclaim: [80186] Failed to free(reusable) err=1",
                message,
            ))
        };
        let mut day = Day::new("2026-11-12");
        for message in [
            "3 duplicate reports for Sandbox: ecosystemd(1997) deny(1) mach-lookup com.apple.bird",
            "1 duplicate report for Sandbox: contactsd(912) deny(1) file-read-data /usr/bin",
            "System Policy: disk-guard(54051) deny(1) file-read-metadata /Users",
            "[AppleDCPAgileCDIDPDisplay.cpp::1043] link training failed",
        ] {
            day.take(&kernel(message));
        }
        let thing = |name: &str| day.things.get(name).map(|thing| (thing.kind, thing.errors));
        assert_eq!(
            thing("Sandbox"),
            Some((Some(Kind::Security), 2)),
            "a folded repeat is the sandbox's, not a count's: {:?}",
            day.things.keys().collect::<Vec<_>>()
        );
        assert_eq!(
            thing("System Policy"),
            Some((Some(Kind::Security), 1)),
            "System Policy refusing is security's, under its whole name"
        );
        assert_eq!(
            thing("AppleDCPAgileCDIDPDisplay"),
            Some((Some(Kind::Devices), 1)),
            "a driver's line is named by its source file"
        );
    }

    #[test]
    fn a_macs_kernel_line_is_named_past_what_says_nothing_of_who() {
        let kernel = |message: &str| {
            entry(&KERNEL.replace(
                "vm_reclaim: [80186] Failed to free(reusable) err=1",
                message,
            ))
        };
        let mut day = Day::new("2026-11-13");
        for message in [
            "SK[3]: flow_req_validate              flow req failed MAC check",
            "[ERROR] ANE0: FreeSharedMemorySurface: Ignoring firmware free request",
            "[ERROR] : processDeviceEvent: Cannot find request for uuid: 0x1f2",
            "troller.cpp::371] DCPAV[0] AppleDCPDPTX::getByteClkSelValue No device",
            "1043] DCPAV[0] AppleDCPDPTX::updateLinkRateDependentShadowRegisters",
        ] {
            day.take(&kernel(message));
        }
        let names = day.things.keys().map(String::as_str).collect::<Vec<_>>();
        for (name, errors) in [
            ("SK", 1),
            ("ANE#", 1),
            ("processDeviceEvent", 1),
            ("DCPAV", 2),
        ] {
            assert_eq!(
                day.things.get(name).map(|thing| thing.errors),
                Some(errors),
                "{name} is who spoke, past a level, an index or a cut word: {names:?}"
            );
        }
    }

    #[test]
    fn the_day_is_read_from_midnight_and_followed_from_now() {
        let asked = followed("2026-11-12 00:00:00", "2026-11-12 06:10:00");
        assert_eq!(asked[0][0], "stream", "one follows the log from now");
        assert_eq!(
            asked[1][..7],
            [
                "show",
                "--style",
                "ndjson",
                "--start",
                "2026-11-12 00:00:00",
                "--end",
                "2026-11-12 06:10:00"
            ],
            "the other reads the day so far"
        );
        assert!(
            asked
                .iter()
                .all(|words| words.last().map(String::as_str) == Some(PREDICATE)),
            "both read only errors and faults"
        );
    }
}
