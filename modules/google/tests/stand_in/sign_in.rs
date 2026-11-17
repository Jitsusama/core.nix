//! Signing in: a token refreshed, or used as pi left it, the keyring read
//! or not, a sign-in refused or revoked, Google out of reach, and no file.

use std::fs;
use std::path::PathBuf;

use google::api::{Google, Origins};
use google::state::Standing;
use google::watch::Watch;

use super::{EVENTS, StandIn, credentials, google, now};

#[test]
fn a_stale_token_is_refreshed_and_the_meetings_are_told() {
    let stand_in = StandIn::new(google);
    let path = credentials("refreshed", 0);
    let before = fs::read(&path).unwrap();
    let mut watch = Watch::new(Google::new(stand_in.origins()), path.clone(), None);
    let state = watch.look(&now()).clone();
    assert_eq!(state.account, Standing::Ready);
    assert_eq!(state.meetings.len(), 1);
    assert_eq!(state.meetings[0].title, "Standup");
    let heard = stand_in.heard();
    assert!(
        heard[0].line.starts_with("POST /token"),
        "the first ask was {}",
        heard[0].line
    );
    let asked = &heard[1];
    assert_eq!(asked.authorization, "Bearer mine");
    assert!(
        asked
            .line
            .starts_with("GET /calendar/v3/calendars/primary/events?"),
        "the meetings were asked as {}",
        asked.line
    );
    assert!(
        asked.line.contains("timeMin=2026-10-26T04%3A00%3A00Z"),
        "{}",
        asked.line
    );
    assert!(
        asked.line.contains("singleEvents=true"),
        "repeats weren't asked one by one: {}",
        asked.line
    );
    assert_eq!(
        fs::read(&path).unwrap(),
        before,
        "the harness's file was written"
    );
}

#[test]
fn pis_fresh_token_is_used_without_a_refresh() {
    let stand_in = StandIn::new(google);
    let path = credentials("fresh", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    assert_eq!(watch.look(&now()).account, Standing::Ready);
    let heard = stand_in.heard();
    assert!(
        heard
            .iter()
            .all(|heard| !heard.line.starts_with("POST /token")),
        "a fresh token was refreshed"
    );
    assert!(
        heard
            .iter()
            .all(|heard| heard.authorization == "Bearer pis"),
        "an ask went without pi's token"
    );
}

/// A keyring reader for `path`'s sign-in, in its folder, as secret-tool
/// is one: it prints the sign-in, fails saying why once a file `failing`
/// is beside it, and fails saying nothing, as for an item gone, once a
/// file `gone` is. The file itself is moved for the reader to print, so
/// only the keyring holds the sign-in.
fn keyring(path: &std::path::Path) -> PathBuf {
    let folder = path.parent().expect("the sign-in is in a folder");
    let kept = folder.join("kept.json");
    fs::rename(path, &kept).expect("the sign-in is moved into the keyring");
    let reader = folder.join("keyring");
    let script = format!(
        "#!/bin/sh\ncd '{}'\n[ -e gone ] && exit 1\n[ -e failing ] && {{ \
         echo 'secret-tool: Could not connect: No such file or directory' >&2; exit 1; }}\n\
         cat kept.json",
        folder.display()
    );
    // Written by a shell of its own, never by this process, so no test
    // forking meanwhile holds it open for writing (account.rs).
    let written = std::process::Command::new("/bin/sh")
        .args(["-c", r#"printf '%s\n' "$2" > "$1" && chmod 755 "$1""#, "sh"])
        .arg(&reader)
        .arg(script)
        .status()
        .expect("the reader's shell runs");
    assert!(written.success(), "the keyring reader wasn't written");
    reader
}

#[test]
fn a_keyring_that_cant_be_read_keeps_the_sign_in_it_last_gave() {
    let stand_in = StandIn::new(google);
    let path = credentials("unreadable", 0);
    let folder = path
        .parent()
        .expect("the sign-in is in a folder")
        .to_path_buf();
    let mut watch = Watch::new(Google::new(stand_in.origins()), path.clone(), None);
    watch.keyring = Some(keyring(&path));
    assert_eq!(
        watch.look(&now()).account,
        Standing::Ready,
        "the keyring's sign-in wasn't read"
    );
    fs::write(folder.join("failing"), "").expect("the keyring is made to fail");
    let state = watch.look(&now()).clone();
    assert_eq!(
        state.account,
        Standing::Ready,
        "a keyring that couldn't be read signed Joel out"
    );
    assert_eq!(
        state.meetings.len(),
        1,
        "the meetings went with the keyring"
    );
    fs::write(folder.join("gone"), "").expect("the keyring's item is taken away");
    assert_eq!(
        watch.look(&now()).account,
        Standing::None,
        "a sign-in taken from the keyring was kept"
    );
}

#[test]
fn a_keyring_that_cant_be_read_from_the_start_is_unreachable_not_signed_out() {
    let stand_in = StandIn::new(google);
    let path = credentials("unreadable-at-start", 0);
    let folder = path
        .parent()
        .expect("the sign-in is in a folder")
        .to_path_buf();
    let mut watch = Watch::new(Google::new(stand_in.origins()), path.clone(), None);
    watch.keyring = Some(keyring(&path));
    fs::write(folder.join("failing"), "").expect("the keyring is made to fail");
    assert_eq!(watch.look(&now()).account, Standing::Unreachable);
    assert!(
        watch.waits_on_keyring(),
        "the keyring would be looked at less often"
    );
    assert!(
        stand_in.heard().is_empty(),
        "Google was asked with no sign-in"
    );
    fs::remove_file(folder.join("failing")).expect("the keyring is mended");
    assert_eq!(
        watch.look(&now()).account,
        Standing::Ready,
        "a mended keyring wasn't read"
    );
    assert!(
        !watch.waits_on_keyring(),
        "a mended keyring was still waited on"
    );
}

#[test]
fn a_refused_sign_in_is_signed_out_and_not_asked_again() {
    let stand_in = StandIn::new(|path, _| {
        if path.starts_with("POST /token") {
            (400, r#"{"error": "invalid_grant"}"#.into())
        } else {
            (200, EVENTS.into())
        }
    });
    let mut watch = Watch::new(
        Google::new(stand_in.origins()),
        credentials("refused", 0),
        None,
    );
    let state = watch.look(&now()).clone();
    assert_eq!(state.account, Standing::SignedOut);
    assert_eq!(state.meetings, [] as [google::calendar::Meeting; 0]);
    watch.look(&now());
    assert_eq!(
        stand_in.heard().len(),
        1,
        "a refused sign-in was offered again"
    );
}

#[test]
fn a_revoked_token_is_refreshed_past_once_then_waits_for_a_new_sign_in() {
    let stand_in = StandIn::new(|path, _| {
        if path.starts_with("POST /token") {
            (
                200,
                r#"{"access_token": "mine", "expires_in": 3599}"#.into(),
            )
        } else if path.starts_with("GET") && !path.contains("pageToken") {
            (401, "{}".into())
        } else {
            (200, EVENTS.into())
        }
    });
    let path = credentials("revoked", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    assert_eq!(watch.look(&now()).account, Standing::SignedOut);
    let lines: Vec<_> = stand_in
        .heard()
        .into_iter()
        .map(|heard| heard.authorization)
        .collect();
    assert_eq!(lines, vec!["Bearer pis", "", "Bearer mine"]);
    watch.look(&now());
    assert_eq!(
        stand_in.heard().len(),
        3,
        "a refused sign-in was offered again"
    );
}

#[test]
fn an_unreachable_google_keeps_what_was_heard() {
    let stand_in = StandIn::new(google);
    let path = credentials("unreachable", 0);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path.clone(), None);
    let heard = watch.look(&now()).heard;
    // A port nothing listens on, as Google is to a laptop offline. Port 1
    // rather than one bound and let go, which a stand-in beside this test
    // could be handed next: no test may bind one below 1024.
    let closed = "127.0.0.1:1";
    watch.google = Google::new(Origins {
        token: format!("http://{closed}/token"),
        api: format!("http://{closed}"),
        meet: format!("http://{closed}"),
    });
    let state = watch.look(&now()).clone();
    assert_eq!(state.account, Standing::Unreachable);
    assert_eq!(state.meetings.len(), 1);
    assert_eq!(state.heard, heard);
}

#[test]
fn no_file_is_no_account_and_google_is_never_asked() {
    let stand_in = StandIn::new(google);
    let mut watch = Watch::new(
        Google::new(stand_in.origins()),
        PathBuf::from("/nonexistent/credentials.json"),
        None,
    );
    assert_eq!(watch.look(&now()).account, Standing::None);
    assert!(
        stand_in.heard().is_empty(),
        "google was asked with no account"
    );
}
