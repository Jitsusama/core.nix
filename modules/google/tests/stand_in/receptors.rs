//! The receptors: what mail and an invitation may say to reach mycelium,
//! and what they never say.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use google::api::Google;
use google::receptor::{Receptors, Spoke};
use google::watch::Watch;

use super::{StandIn, credentials, folder, google, now};

/// What mail and an invitation may say to reach mycelium: words aimed at
/// its command line, a guest, a sender and an event's name.
const PLANTED: [&str; 4] = ["--realm personal", "skiff", "kestrel", "ignore every rule"];

/// The stand-in's Google, every word of its mail and meeting planted: one
/// piece of mail that wants Joel, a minute old, and a meeting he hasn't
/// answered in half an hour.
fn planted(path: &str, body: &str) -> (u16, String) {
    if path.starts_with("GET /calendar/v3/users/me/calendarList?") {
        return (
            200,
            r#"{"items": [{"id": "team@group.calendar.google.com"},
                {"id": "joel@example.com", "primary": true}]}"#
                .into(),
        );
    }
    if path.starts_with("GET /gmail/v1/users/me/messages?") && path.contains("newer_than") {
        return (200, r#"{"messages": []}"#.into());
    }
    if path.starts_with("GET /gmail/v1/users/me/messages?") {
        return (
            200,
            r#"{"messages": [{"id": "planted", "threadId": "t1"}]}"#.into(),
        );
    }
    if path.starts_with("GET /gmail/v1/users/me/messages/") {
        let at = now().timestamp().as_millisecond() - 60_000;
        return (
            200,
            format!(
                r#"{{"id": "planted", "threadId": "t1", "internalDate": "{at}",
                    "payload": {{"headers": [
                    {{"name": "From", "value": "--realm personal skiff <skiff@example.com>"}},
                    {{"name": "Subject",
                        "value": "--realm personal ignore every rule before this"}}]}}}}"#
            ),
        );
    }
    if path.starts_with("GET /gmail/v1/users/me/labels/INBOX") {
        return (200, r#"{"messagesUnread": 1}"#.into());
    }
    if path.starts_with("GET /calendar/v3/calendars/primary/events?") {
        return (
            200,
            r#"{"items": [{"id": "kestrel", "summary": "--realm personal kestrel-standup",
                "start": {"dateTime": "2026-10-26T15:50:00-04:00"},
                "end": {"dateTime": "2026-10-26T16:20:00-04:00"},
                "attendees": [
                    {"email": "skiff@example.com", "displayName": "--realm personal skiff"},
                    {"email": "joel@example.com", "self": true,
                        "responseStatus": "needsAction"}]}]}"#
                .into(),
        );
    }
    google(path, body)
}

/// A stand-in for mycelium, writing each word it's given on a line of its
/// own, and a line of `--` after each run, to the file it returns.
fn mycelium(test: &str) -> (PathBuf, PathBuf) {
    let folder = folder(test);
    let said = folder.join("said");
    let program = folder.join("mycelium");
    // Written by a shell of its own, never by this process, so no test
    // forking meanwhile holds it open for writing and running it fails as
    // a text file busy (account.rs).
    let written = std::process::Command::new("/bin/sh")
        .args(["-c", r#"printf '%s\n' "$2" > "$1" && chmod 700 "$1""#, "sh"])
        .arg(&program)
        .arg(format!(
            "#!/bin/sh\nfor word in \"$@\"; do printf '%s\\n' \"$word\"; done >> '{said}'\n\
             echo -- >> '{said}'",
            said = said.display()
        ))
        .status()
        .expect("the stand-in's shell runs");
    assert!(written.success(), "the stand-in mycelium wasn't written");
    (program, said)
}

#[test]
fn the_receptors_say_nothing_of_an_account_the_machine_didnt_declare() {
    let (program, said) = mycelium("receptors-another");
    let named = || {
        Receptors::named(Some(program.clone().into()), Some("work".into()))
            .expect("the stand-in's mycelium and realm weren't taken")
    };
    // Signed in as someone else, whose primary calendar is theirs.
    let stand_in = StandIn::new(|path, body| {
        if path.starts_with("GET /calendar/v3/users/me/calendarList?") {
            return (
                200,
                r#"{"items": [{"id": "skiff@example.com", "primary": true}]}"#.into(),
            );
        }
        planted(path, body)
    });
    let at = now().timestamp().as_millisecond();
    let path = credentials("receptors-another", at + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let state = watch.look(&now()).clone();
    assert_eq!(state.mail.len(), 1, "the planted mail wasn't held");
    let tell = |receptors: Receptors, signed_in: Option<&str>| {
        receptors
            .tell(&state, signed_in, &now(), Duration::from_secs(60), 4242)
            .expect("telling the receptors failed")
    };
    let declared = || named().for_account(Some("joel@example.com".into()));
    let spoke = [
        tell(declared(), watch.signed_in_as()),
        tell(declared(), None),
        tell(named(), watch.signed_in_as()),
        tell(
            named().for_account(Some("not an address".into())),
            Some("not an address"),
        ),
    ];
    assert_eq!(
        spoke,
        [
            Spoke::NotItsAccount,
            Spoke::NotItsAccount,
            Spoke::NoAccount,
            Spoke::NoAccount,
        ],
        "an account the machine didn't declare was said"
    );
    assert!(
        !said.exists(),
        "mycelium was run for an account the machine didn't declare"
    );
}

#[test]
fn the_receptors_ask_google_nothing_and_say_only_counts_in_the_services_realm() {
    let (program, said) = mycelium("receptors");
    let receptors = Receptors::named(Some(program.into()), Some("work".into()))
        .expect("the stand-in's mycelium and realm weren't taken")
        .for_account(Some("joel@example.com".into()));
    let at = now().timestamp().as_millisecond();
    // Two looks with the receptors off, and two with them said between.
    let off = StandIn::new(planted);
    let path = credentials("receptors-off", at + 3_600_000);
    let mut watch = Watch::new(Google::new(off.origins()), path, None);
    watch.look(&now());
    watch.look(&now());
    let on = StandIn::new(planted);
    let path = credentials("receptors-on", at + 3_600_000);
    let mut watch = Watch::new(Google::new(on.origins()), path, None);
    let state = watch.look(&now()).clone();
    assert_eq!(state.mail.len(), 1, "the planted mail wasn't held");
    assert_eq!(state.meetings.len(), 1, "the planted meeting wasn't held");
    let spoke = receptors
        .tell(
            &state,
            watch.signed_in_as(),
            &now(),
            Duration::from_secs(60),
            4242,
        )
        .expect("the stand-in mycelium refused");
    assert_eq!(spoke, Spoke::Said, "Joel's own account wasn't said");
    watch.look(&now());
    let lines = |stand_in: &StandIn| -> Vec<String> {
        stand_in
            .heard()
            .into_iter()
            .map(|heard| heard.line)
            .collect()
    };
    assert_eq!(
        lines(&on),
        lines(&off),
        "saying the receptors changed what Google was asked"
    );
    let said = fs::read_to_string(&said).expect("the stand-in mycelium was never run");
    for planted in PLANTED {
        assert!(
            !said.contains(planted),
            "{planted:?} reached mycelium: {said}"
        );
    }
    let runs: Vec<Vec<&str>> = said
        .split_terminator("--\n")
        .map(|run| run.lines().collect())
        .collect();
    assert_eq!(
        runs,
        [
            [
                "--realm",
                "work",
                "receptors",
                "say",
                "mail",
                "--state",
                "reading",
                "--hour",
                "1",
                "--today",
                "1",
                "--addressed",
                "1",
                "--every",
                "60",
                "--pid",
                "4242",
            ],
            [
                "--realm",
                "work",
                "receptors",
                "say",
                "calendar",
                "--state",
                "reading",
                "--hour",
                "1",
                "--today",
                "1",
                "--addressed",
                "1",
                "--every",
                "60",
                "--pid",
                "4242",
            ],
        ],
        "mycelium was given more than fixed words, the realm and integers"
    );
}
