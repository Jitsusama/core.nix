//! One entry of a Mac's unified log as `log show --style ndjson` and `log
//! stream --style ndjson` write it, read into the journal's entry, so a
//! Mac's day is tallied as a Linux machine's is; and what the follower asks
//! `log` for.

use crate::journal::Entry;

#[must_use]
pub fn parse(_line: &str) -> Option<Entry> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
