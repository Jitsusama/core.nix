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

/// How a Mac's kernel says a line came again, before the line itself.
const FOLDED: [&str; 2] = [" duplicate report for ", " duplicate reports for "];

/// Whether a kernel line said as information is a device's event.
pub fn is_device_event(message: &str) -> bool {
    DEVICE_EVENTS
        .iter()
        .any(|words| words.iter().all(|word| message.contains(word)))
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
    let opening = message.split_whitespace().next().unwrap_or("kernel");
    let driver = opening
        .strip_prefix('[')
        .and_then(|place| place.split_once("::"))
        .and_then(|(file, _)| file.rsplit_once('.'))
        .map(|(driver, _)| driver);
    sign::signature(driver.unwrap_or_else(|| opening.trim_end_matches(':')))
}
