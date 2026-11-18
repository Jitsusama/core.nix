//! Whether the firmware kept an error from before this machine's boot. A
//! laptop whose embedded controller cuts the power leaves no word in the
//! journal, since nothing ran to write one; the firmware keeps an error
//! record across the cut instead, its boot error region (ACPI BERT), and
//! the kernel says at boot how many records it found there. On the laptop
//! this was written for, every abrupt end left one and no clean boot did,
//! but a record says only that the firmware kept an error, not which, so
//! the view says that much and no more.
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

use serde_json::{Value, json};

use crate::parts::Answer;

/// What asks this boot's kernel log for the kernel's count of the boot
/// error records, as one JSON document, the line's own; journalctl ends
/// with 1 when no line matches, so a boot with none answers an error.
pub const ASK: &[&str] = &[
    "journalctl",
    "--dmesg",
    "--boot",
    "--output=json",
    "--output-fields=MESSAGE",
    "--lines=1",
    "--grep=^BERT: Total records found: ",
];

/// How long after the boot an error kept before it is worth a look.
pub const NEWS_FOR: u64 = 24 * 60 * 60;

/// How long ago this machine booted after the firmware kept an error, in
/// seconds, from the journal's answer to [`ASK`] and the time since the
/// boot; none when no record was found, when the boot was more than a day
/// ago, or when the journal couldn't be read.
#[must_use]
pub fn kept_before_boot(said: &Answer, since_boot: u64) -> Option<u64> {
    let line = said.as_ref().ok()?.get("MESSAGE")?.as_str()?;
    let found: u64 = line
        .strip_prefix("BERT: Total records found: ")?
        .trim()
        .parse()
        .ok()?;
    (found > 0 && since_boot <= NEWS_FOR).then_some(since_boot)
}

/// The seconds since the boot, from /proc/uptime's first figure, which
/// counts time asleep, as the person does.
#[must_use]
pub fn since_boot(uptime: &str) -> Option<u64> {
    let seconds: f64 = uptime.split_whitespace().next()?.parse().ok()?;
    // Whole seconds are all the view says; a negative or absurd figure
    // isn't an uptime.
    (seconds.is_finite() && seconds >= 0.0).then(|| seconds as u64)
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

    fn said(message: &str) -> Answer {
        Ok(json!({ "MESSAGE": message }))
    }

    #[test]
    fn records_found_are_news_for_a_day_after_the_boot() {
        let one = said("BERT: Total records found: 1");
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
            kept_before_boot(&said("BERT: Total records found: 0"), 60),
            None
        );
        assert_eq!(
            kept_before_boot(&said("BERT: [Hardware Error]: Skipped 1 error records"), 60),
            None,
            "only the count is read"
        );
        assert_eq!(
            kept_before_boot(&said("BERT: Total records found: x"), 60),
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
    fn the_time_since_boot_is_uptimes_first_figure() {
        assert_eq!(since_boot("6683.39 21182.91\n"), Some(6_683));
        assert_eq!(since_boot(""), None);
        assert_eq!(since_boot("-1 0"), None);
        assert_eq!(since_boot("NaN 0"), None);
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
