//! How the day reads a kernel's line: which driver, device or refuser it's
//! from, and whether what it said only as information is still a device's
//! event. Linux's kernel and a Mac's open their lines differently, and both
//! are read here.

use crate::day::Kind;
use crate::sign;

/// What the kernel says only as information that's still a device going
/// wrong, or going away: a USB device dropped or reset, a PCI Express link's
/// corrected errors, a processor throttled, firmware that wouldn't load.
/// Each is said in full in the kernel's own words, and matched as written.
const DEVICE_EVENTS: [&[&str]; 6] = [
    &["USB disconnect"],
    &["reset ", " USB device number "],
    &["AER: ", " error"],
    &["throttled"],
    &["Direct firmware load"],
    &["failed to load firmware"],
];

/// How closely a report's lines follow the line before them, in
/// microseconds: Linux's kernel writes a report all at once, its eighty
/// lines within a few milliseconds.
pub const REPORT_GAP: i64 = 1_000_000;

/// How the lines of a Linux kernel's report open after the line that
/// opened it: who was running, the call trace, the registers' preamble,
/// and the memory's state when an allocation failed.
const REPORT_LINES: [&str; 18] = [
    "CPU: ",
    "Tainted: ",
    "Hardware name: ",
    "Workqueue: ",
    "Call Trace:",
    "<TASK>",
    "</TASK>",
    "<IRQ>",
    "</IRQ>",
    "? ",
    "Code: ",
    "Modules linked in:",
    "---[ end trace",
    "Mem-Info:",
    "active_anon:",
    "lowmem_reserve[]:",
    "Free swap ",
    "Total swap ",
];

/// How a Mac's kernel says a line came again, before the line itself.
const FOLDED: [&str; 2] = [" duplicate report for ", " duplicate reports for "];

/// Whether a kernel line said as information is a device's event.
pub fn is_device_event(message: &str) -> bool {
    DEVICE_EVENTS
        .iter()
        .any(|words| words.iter().all(|word| message.contains(word)))
}

/// Whether a kernel line is one a report goes on with, once it opened:
/// one of its openings, a call's frame, `dump_stack_lvl+0x61/0x90`, a
/// register's value, `RIP: 0033:0x7977f7ca8472`, a memory node's state,
/// `Node 0 DMA free:11132kB`, or a count of pages, `0 pages hwpoisoned`.
pub fn continues(message: &str) -> bool {
    let line = message.trim_start();
    let mut words = line.split_whitespace();
    let first = words.next().unwrap_or_default();
    REPORT_LINES.iter().any(|opening| line.starts_with(opening))
        || (first.contains("+0x") && first.contains("/0x"))
        || is_register(first, words.next().unwrap_or_default())
        || line
            .strip_prefix("Node ")
            .is_some_and(|node| node.starts_with(|c: char| c.is_ascii_digit()))
        || (!first.is_empty()
            && first.chars().all(|c| c.is_ascii_digit())
            && line.contains(" pages"))
}

/// Whether a line's first two words are a register and its value, as a
/// report gives them: `RAX: ffffffffffffffda`, `CS:  0010`, `RIP: 0033:0x7977`.
/// A short capital name alone, `PM:`, isn't enough, since other lines open
/// with one; its value has to be in hexadecimal too.
fn is_register(name: &str, value: &str) -> bool {
    name.strip_suffix(':').is_some_and(|name| {
        (2..=4).contains(&name.len())
            && name
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    }) && !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c == ':' || c == 'x')
}

/// Who a kernel line is from, and its kind. Linux's kernel refuses through
/// the audit log, and a Mac's through its Sandbox and System Policy, which
/// Linux's never open with; every other speaker is a driver or a device.
pub fn named(message: &str) -> (String, Kind) {
    let speaker = speaker(message);
    let kind = if matches!(speaker.as_str(), "audit" | "Sandbox" | "System Policy") {
        Kind::Security
    } else {
        Kind::Devices
    };
    (speaker, kind)
}

/// Who a kernel line is from, by what it opens with once the openings that
/// say nothing of it are passed: a Mac's kernel folds repeats into
/// `3 duplicate reports for Sandbox: ...`, and its display drivers open
/// with their source file and line, `[AppleDCPDPTXController.cpp::371] ...`.
/// A speaker's instance, `SK[3]`, is dropped, so its instances are one.
fn speaker(message: &str) -> String {
    let message = FOLDED
        .iter()
        .find_map(|folded| message.split_once(folded))
        .filter(|(count, _)| count.chars().all(|c| c.is_ascii_digit()))
        .map_or(message, |(_, said)| said);
    // A Mac's System Policy, refusing a program, is the one speaker whose
    // name is two words.
    if message.starts_with("System Policy:") {
        return "System Policy".to_owned();
    }
    let opening = message
        .split_whitespace()
        .find(|word| !says_nothing_of_who(word))
        .unwrap_or("kernel");
    let driver = opening
        .strip_prefix('[')
        .and_then(|place| place.split_once("::"))
        .and_then(|(file, _)| file.rsplit_once('.'))
        .map(|(driver, _)| driver);
    let named = driver.unwrap_or_else(|| opening.trim_end_matches(':'));
    sign::signature(without_instance(named))
}

/// Whether a line's word says nothing of who spoke: a Mac's kernel opens
/// some lines with their level, `[ERROR]`, or a lone `:`, and its log cuts a
/// long line into pieces, so a piece can open in a word's last part, as
/// `troller.cpp::371]` does, a `]` with no `[` before it.
fn says_nothing_of_who(word: &str) -> bool {
    word == "[ERROR]" || word == ":" || (word.ends_with(']') && !word.contains('['))
}

/// A speaker's name without the instance a Mac's kernel numbers it by,
/// `SK[3]` being `SK`.
fn without_instance(name: &str) -> &str {
    name.strip_suffix(']')
        .and_then(|name| name.rsplit_once('['))
        .filter(|(_, index)| !index.is_empty() && index.chars().all(|c| c.is_ascii_digit()))
        .map_or(name, |(name, _)| name)
}

#[cfg(test)]
mod tests {
    use crate::day::Day;
    use crate::day::tests::entry;
    use crate::journal::Entry;

    /// A warning from the kernel, said at `at`.
    fn kernel(at: i64, message: &str) -> Entry {
        Entry {
            kernel: true,
            at,
            ..entry("kernel", 4, message)
        }
    }

    #[test]
    fn a_reports_lines_are_one_line_of_the_line_that_opened_it() {
        let mut day = Day::new("2026-11-13");
        let report = [
            "machine: page allocation failure: order:4, mode:0x40cc0(GFP_KERNEL|__GFP_COMP)",
            "CPU: 9 UID: 1000 PID: 1652571 Comm: machine Tainted: G     U  WC O 7.3.3 #1-NixOS",
            "Tainted: [U]=USER, [W]=WARN, [C]=CRAP, [O]=OOT_MODULE",
            "Hardware name: QEMU Standard PC (Q35 + ICH9, 2009), BIOS 1.16.3",
            "Call Trace:",
            " <TASK>",
            " dump_stack_lvl+0x61/0x90",
            " ? __alloc_pages_direct_compact+0x104/0x220",
            " wmidev_invoke_method+0x1cf/0x270 [wmi]",
            "RIP: 0033:0x7977f7ca8472",
            "RAX: ffffffffffffffda RBX: 00007977e800ee60 RCX: 00007977e800ee60",
            "R10: 0000000000000000 R11: 00007977b65f29c8 R12: 0000000000001000",
            " </TASK>",
            "Mem-Info:",
            "active_anon:4632126 inactive_anon:3176151 isolated_anon:112",
            "Node 0 DMA free:11132kB boost:0kB min:12kB low:48kB high:84kB",
            "lowmem_reserve[]: 0 1604 63784 63784 63784",
            "Node 0 DMA: 1*4kB (U) 1*8kB (U) 1*16kB (U) 1*32kB (U)",
            "8016927 total pagecache pages",
            "Free swap  = 99299892kB",
            "16648237 pages RAM",
            "0 pages hwpoisoned",
        ];
        for (at, line) in (1_000..).step_by(20).zip(report) {
            day.take(&kernel(at, line));
        }
        assert_eq!(
            day.things.keys().collect::<Vec<_>>(),
            ["machine"],
            "a report's later lines are its opener's, not devices of their own"
        );
        assert_eq!(day.things["machine"].lines, 1, "a report is counted once");
    }

    #[test]
    fn a_line_shaped_like_a_reports_with_none_before_it_is_its_own() {
        let mut day = Day::new("2026-11-13");
        day.take(&kernel(
            1_000,
            "htb: cake qdisc 8001: is non-work-conserving?",
        ));
        day.take(&kernel(
            9_000_000,
            "CPU: 3 PID: 12 at drivers/gpu/drm/drm_vblank.c:1307",
        ));
        day.take(&kernel(
            9_000_500,
            "htb: cake qdisc 8002: is non-work-conserving?",
        ));
        assert_eq!(
            day.things.keys().collect::<Vec<_>>(),
            ["CPU", "htb"],
            "a line far from any before it opens nothing, and a line a report \
             never says isn't folded into one"
        );
        assert_eq!(
            day.things["htb"].lines, 2,
            "both of htb's lines are counted"
        );
    }
}
