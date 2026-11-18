//! Whether the firmware kept an error from before this machine's boot. A
//! machine reset from under the system, by its firmware or a power cut,
//! leaves no word in the journal, since nothing ran to write one; the
//! firmware keeps an error record across the reset instead, its boot error
//! region (ACPI BERT), and the kernel says at boot how many records it
//! found there. On the laptop this was written for, every abrupt end left
//! one and no clean boot did, but a record says only that the firmware
//! kept an error, not which, so the view says that much and no more.
//!
//! The kernel clears each record's block status as it reads it, so a copy
//! of the region taken later reads empty; its own line in this boot's
//! kernel log is the one word on it that holds. No owner's command says
//! it, so this asks the journal, which an account outside the journal's
//! readers can't read: for one, this says nothing.
//!
//! It is worth a look for a day after the boot: long enough to be seen the
//! morning after a cut in the night, and no longer, since one a week ago is
//! history rather than news.

use std::sync::OnceLock;

use serde_json::{Value, json};

use crate::parts::Answer;

/// The kernel's line, up to its count (drivers/acpi/apei/bert.c).
const LINE: &str = "BERT: Total records found: ";

/// The kernel's first line in every boot's log (init/version.c), which the
/// boot's records come after: the newest of the two is the count when
/// there is one, and this when the kernel found no records and said none.
const FIRST: &str = "Linux version ";

/// What asks this boot's kernel log for the kernel's count of the boot
/// error records, as one JSON document, the line's own, or the boot's
/// first line when it has none, so a clean boot answers too and is asked
/// no more. With `--grep`, `--lines` counts the newest matching entries
/// (journalctl(1): it implies `--reverse`), not the boot's last lines.
/// journalctl ends with 1 when no line matches, as when the journal can't
/// be read or has lost the boot's start, so those answer an error.
pub const ASK: &[&str] = &[
    "journalctl",
    "--dmesg",
    "--boot",
    "--output=json",
    "--output-fields=MESSAGE",
    "--lines=1",
    concat!(
        "--grep=^(?:",
        "BERT: Total records found: ",
        "|",
        "Linux version ",
        ")"
    ),
];

/// The count the kernel gave, once found, none being a count of 0. It
/// says it once, at boot, so a process that has read it needn't ask the
/// journal again, as `system serve` would every few seconds; the journal
/// takes one to two seconds to answer.
pub type Found = OnceLock<u64>;

/// How long after the boot an error kept before it is worth a look.
pub const NEWS_FOR: u64 = 24 * 60 * 60;

/// How long ago this machine booted after the firmware kept an error, in
/// seconds, from the journal's answer to [`ASK`] and the time since the
/// boot; none when no record was found, when the boot was more than a day
/// ago, or when the journal couldn't be read.
#[must_use]
pub fn kept_before_boot(said: &Answer, since_boot: u64) -> Option<u64> {
    let found = found_in(said)?;
    (found > 0 && since_boot <= NEWS_FOR).then_some(since_boot)
}

fn found_in(said: &Answer) -> Option<u64> {
    let line = said.as_ref().ok()?.get("MESSAGE")?.as_str()?;
    if line.starts_with(FIRST) {
        return Some(0);
    }
    line.strip_prefix(LINE)?.trim().parse().ok()
}

/// Whether the journal is still worth asking: only while the time since
/// the boot is known and within the day the news lasts, since an answer
/// can't be judged without it, and not once this process has the
/// kernel's count.
#[must_use]
pub fn worth_asking(found: &Found, since_boot: Option<u64>) -> bool {
    found.get().is_none() && since_boot.is_some_and(|since| since <= NEWS_FOR)
}

/// The journal's answer, or the count already found as one when it
/// wasn't asked, keeping a count found for the rest of the process.
pub fn remembered(found: &Found, said: Option<Answer>) -> Answer {
    if let Some(found) = found.get() {
        return Ok(json!({ "MESSAGE": format!("{LINE}{found}") }));
    }
    let said = said.unwrap_or_else(|| Err("not asked after the day".to_owned()));
    if let Some(count) = found_in(&said) {
        // Another reader setting it first set the same count.
        let _ = found.set(count);
    }
    said
}

/// The seconds since the boot, from /proc/uptime's first figure, which
/// counts time asleep, as the person does.
#[must_use]
pub fn since_boot(uptime: &str) -> Option<u64> {
    // Whole seconds are all the view says, so only the figure's whole part
    // is read, as digits: a negative or absurd figure isn't an uptime.
    let first = uptime.split_whitespace().next()?;
    let (whole, _) = first.split_once('.').unwrap_or((first, ""));
    whole.parse().ok()
}

/// Puts an error kept before this boot into this machine's record, which
/// the node gives first, as `firmware.error_before_boot`, the field the
/// machines part judges. A record the node gives with no `firmware`, or
/// with one that isn't a table, gets one of its own.
pub fn note(machines: &mut Answer, ago: u64) {
    let Some(here) = machines
        .as_mut()
        .ok()
        .and_then(|said| said.pointer_mut("/Ok/Machines/0"))
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    let firmware = here.entry("firmware").or_insert_with(|| json!({}));
    if !firmware.is_object() {
        *firmware = json!({});
    }
    firmware["error_before_boot"] = ago.into();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn said(message: &str) -> Value {
        json!({ "MESSAGE": message })
    }

    #[test]
    fn records_found_are_news_for_a_day_after_the_boot() {
        let one = Ok(said("BERT: Total records found: 1"));
        assert_eq!(kept_before_boot(&one, 3_600), Some(3_600));
        assert_eq!(kept_before_boot(&one, NEWS_FOR), Some(NEWS_FOR));
        assert_eq!(
            kept_before_boot(&one, NEWS_FOR + 1),
            None,
            "a day on it's history"
        );
    }

    #[test]
    fn no_record_or_no_journal_says_nothing() {
        assert_eq!(
            kept_before_boot(&Ok(said("BERT: Total records found: 0")), 60),
            None
        );
        assert_eq!(
            kept_before_boot(
                &Ok(said("BERT: [Hardware Error]: Skipped 1 error records")),
                60
            ),
            None,
            "only the count is read"
        );
        assert_eq!(
            kept_before_boot(&Ok(said("BERT: Total records found: x")), 60),
            None
        );
        assert_eq!(
            kept_before_boot(&Err("journalctl ended with 1".to_owned()), 60),
            None,
            "a boot with no line, or an account that can't read the journal"
        );
        assert_eq!(kept_before_boot(&Ok(json!({})), 60), None);
    }

    #[test]
    fn the_journal_is_asked_for_the_line_that_is_read() {
        assert_eq!(
            ASK.last(),
            Some(&format!("--grep=^(?:{LINE}|{FIRST})").as_str())
        );
        assert!(ASK.contains(&"--lines=1"), "one line, the newest matching");
    }

    #[test]
    fn a_clean_boot_is_a_count_of_none() {
        let first = Ok(said("Linux version 7.3.4 (nixbld@localhost) #1-Nix"));
        assert_eq!(kept_before_boot(&first, 60), None);
        let found = Found::new();
        let first = remembered(&found, Some(first));
        assert_eq!(kept_before_boot(&first, 60), None, "passed on as it was");
        assert_eq!(found.get(), Some(&0), "kept, so never asked again");
        assert!(
            !worth_asking(&found, Some(60)),
            "a clean boot is asked once"
        );
        assert_eq!(
            kept_before_boot(&remembered(&found, None), 60),
            None,
            "none found answers as none"
        );
    }

    #[test]
    fn a_count_found_is_kept_and_the_journal_asked_no_more() {
        let found = Found::new();
        assert!(worth_asking(&found, Some(60)), "nothing found yet, asked");
        assert!(
            !worth_asking(&found, None),
            "no uptime, nothing to judge it by"
        );
        assert!(!worth_asking(&found, Some(NEWS_FOR + 1)), "a day on");

        let none = remembered(&found, Some(Err("journalctl ended with 1".to_owned())));
        assert!(none.is_err(), "no line passes on as the journal's error");
        assert!(
            worth_asking(&found, Some(60)),
            "unread, or its start lost: asked again"
        );

        let one = remembered(&found, Some(Ok(said("BERT: Total records found: 1"))));
        assert_eq!(kept_before_boot(&one, 60), Some(60));
        assert!(!worth_asking(&found, Some(60)), "found, never asked again");
        assert_eq!(
            kept_before_boot(&remembered(&found, None), 120),
            Some(120),
            "the count found answers in the journal's place"
        );

        let unasked = Found::new();
        assert!(
            remembered(&unasked, None).is_err(),
            "nothing asked, nothing found"
        );
    }

    #[test]
    fn the_time_since_boot_is_uptimes_first_figure() {
        assert_eq!(since_boot("6683.39 21182.91\n"), Some(6_683));
        assert_eq!(since_boot(""), None);
        assert_eq!(since_boot("-1 0"), None);
        assert_eq!(since_boot("NaN 0"), None);
        assert_eq!(since_boot("12 0"), Some(12), "a figure with no fraction");
    }

    #[test]
    fn the_note_goes_on_this_machine_whatever_its_record_holds() {
        let mut machines: Answer = Ok(json!({ "Ok": { "Machines": [
            { "name": "here", "power": { "charge": 80 } },
            { "name": "there" },
        ] } }));
        note(&mut machines, 90);
        let said = machines.as_ref().unwrap();
        assert_eq!(
            said.pointer("/Ok/Machines/0/firmware/error_before_boot"),
            Some(&json!(90))
        );
        assert_eq!(
            said.pointer("/Ok/Machines/0/power/charge"),
            Some(&json!(80))
        );
        assert_eq!(
            said.pointer("/Ok/Machines/1/firmware"),
            None,
            "only this machine's"
        );

        let mut kept: Answer = Ok(json!({ "Ok": { "Machines": [
            { "firmware": { "version": "1.12.0" } },
        ] } }));
        note(&mut kept, 90);
        assert_eq!(
            kept.unwrap().pointer("/Ok/Machines/0/firmware"),
            Some(&json!({ "version": "1.12.0", "error_before_boot": 90 })),
            "a table the node gives keeps what it holds"
        );

        let mut null: Answer = Ok(json!({ "Ok": { "Machines": [{ "firmware": null }] } }));
        note(&mut null, 90);
        assert_eq!(
            null.unwrap()
                .pointer("/Ok/Machines/0/firmware/error_before_boot"),
            Some(&json!(90)),
            "a node saying none still gets the note"
        );

        let mut none: Answer = Ok(json!({ "Ok": { "Machines": [] } }));
        note(&mut none, 90);
        assert_eq!(
            none,
            Ok(json!({ "Ok": { "Machines": [] } })),
            "no machine, no note"
        );
    }
}
