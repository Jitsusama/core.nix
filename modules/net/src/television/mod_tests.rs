//! The TV's acts as net decides them before and after the TV is reached:
//! which device is a TV, where it's said to be away, and what each
//! failure says and offers.

use super::*;

/// A device called `name`, with `roles`, the hardware address
/// 02:00:00:00:00:90 and `addresses`.
fn device(name: &str, roles: &[&str], addresses: &[&str]) -> Device {
    serde_json::from_value(json!({
        "id": "mac:02:00:00:00:00:90",
        "name": name,
        "roles": roles,
        "source": "eero",
        "macs": ["02:00:00:00:00:90"],
        "addresses": addresses,
        "ports": [],
        "radios": [],
    }))
    .expect("a device")
}

fn never(_mac: &str) -> bool {
    false
}

#[test]
fn each_key_is_found_by_its_word_in_any_case() {
    for (word, said) in KEYS {
        assert_eq!(code(word), Some(said));
        assert_eq!(code(&word.to_uppercase()), Some(said));
    }
}

#[test]
fn a_word_no_key_goes_by_is_no_key() {
    for word in ["", "KEY_POWER", "factory-reset", "volume up", "power "] {
        assert_eq!(code(word), None, "{word:?}");
    }
}

#[test]
fn every_key_goes_by_one_word_and_one_code() {
    let mut words: Vec<&str> = KEYS.iter().map(|(word, _)| *word).collect();
    let mut codes: Vec<&str> = KEYS.iter().map(|(_, code)| *code).collect();
    words.sort_unstable();
    words.dedup();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!((words.len(), codes.len()), (KEYS.len(), KEYS.len()));
}

#[test]
fn a_key_is_said_in_plain_words() {
    assert_eq!(spoken("Volume-Up"), "volume up");
}

#[test]
fn a_tv_heard_on_its_page_or_one_that_allowed_this_machine_is_a_tv() {
    let heard = device("Living", &["television"], &[]);
    assert_eq!(
        television(&heard, never).map_err(|failure| failure.json(false)),
        Ok("02:00:00:00:00:90")
    );
    let silent = device("Living", &["client"], &[]);
    assert_eq!(
        television(&silent, |mac| mac == "02:00:00:00:00:90")
            .map_err(|failure| failure.json(false)),
        Ok("02:00:00:00:00:90"),
        "a TV in standby that allowed this machine isn't taken"
    );
    let refused = television(&silent, never).expect_err("a client is taken as a TV");
    assert_eq!(refused.code(), Exit::Usage.code());
}

#[test]
fn a_key_goes_only_to_an_address_eero_gives_now() {
    let tv = device("Living", &["television"], &["fe80::1", "10.0.0.90"]);
    assert_eq!(
        reached(&tv).map_err(|failure| failure.json(false)),
        Ok("10.0.0.90".parse::<IpAddr>().expect("an address"))
    );
    let gone = device("Living", &["television"], &[]);
    let failure = reached(&gone).expect_err("a TV with no address is reached");
    assert_eq!(failure.code(), Exit::Unavailable.code());
    assert_eq!(address(&gone), None, "a wake has an address to ask");
}

#[test]
fn each_reason_a_key_didnt_go_has_its_own_exit_and_way_out() {
    let tv = device("Living", &["television"], &["10.0.0.90"]);
    let cases = [
        (
            remote::Why::Unreachable("timed out".into()),
            Exit::Unavailable,
            Some("wake"),
        ),
        (remote::Why::NotTheTv, Exit::Refused, None),
        (remote::Why::Refused, Exit::Refused, Some("press mute")),
        (remote::Why::Forgotten, Exit::Refused, Some("press mute")),
        (remote::Why::Unanswered, Exit::Refused, Some("press mute")),
        (
            remote::Why::Broke("reset".into()),
            Exit::TryAgain,
            Some("press mute"),
        ),
        (remote::Why::Unlike("bad frame".into()), Exit::Failed, None),
    ];
    for (why, exit, then) in cases {
        let said = format!("{why:?}");
        let failure = refused(&tv, "Mute", why).json(false);
        assert_eq!(failure["exit"], exit.code(), "{said}");
        match then {
            Some(then) => assert!(
                failure["try"]
                    .as_str()
                    .is_some_and(|tried| tried.contains(then)),
                "{said} offers {}",
                failure["try"]
            ),
            None => assert!(failure["try"].is_null(), "{said} offers a retry"),
        }
    }
}

#[test]
fn a_try_line_names_the_tv_by_its_id_whatever_it_is_called() {
    let tv = device("Living Room", &["television"], &["10.0.0.90"]);
    let failure = refused(&tv, "mute", remote::Why::Refused).json(false);
    assert_eq!(
        failure["try"], "net devices mac:02:00:00:00:00:90 press mute",
        "the try line doesn't reach this TV alone"
    );
}

/// When the records below were collected, in seconds since the epoch.
const COLLECTED: u64 = 1_800_000_000;

/// A record collected at `COLLECTED` whose modem and switch are in
/// `states`, by name, each in its state since `since` seconds before.
fn network(states: &[(&str, &str)], since: u64) -> Network {
    let sources: Vec<Value> = states
        .iter()
        .map(|(name, state)| {
            json!({"name": name, "what": "", "state": state, "since": COLLECTED - since})
        })
        .collect();
    serde_json::from_value(json!({
        "schema": crate::model::SCHEMA,
        "collected_at": COLLECTED,
        "sources": sources,
        "devices": [],
        "links": [],
    }))
    .expect("a record")
}

#[test]
fn this_machine_is_off_home_only_while_neither_the_modem_nor_the_switch_answers() {
    for (states, off) in [
        (
            vec![("modem", "unreachable"), ("switch", "unreachable")],
            true,
        ),
        (vec![("modem", "unreachable")], true),
        (
            vec![("modem", "unreachable"), ("switch", "answering")],
            false,
        ),
        (vec![("modem", "answering"), ("eero", "unreachable")], false),
        (
            vec![("modem", "unreachable"), ("switch", "signed-out")],
            false,
        ),
        (vec![("switch", "signed-out")], false),
        (vec![("eero", "unreachable")], false),
        (vec![], false),
    ] {
        assert_eq!(
            off_home(&network(&states, UNHEARD), COLLECTED + 30),
            off,
            "{states:?}"
        );
    }
}

#[test]
fn only_a_fresh_record_long_unheard_says_this_machine_is_away() {
    let unheard = [("modem", "unreachable"), ("switch", "unreachable")];
    for (since, now, off, said) in [
        (
            UNHEARD,
            COLLECTED + FRESH,
            true,
            "a fresh record unheard long enough",
        ),
        (
            UNHEARD - 60,
            COLLECTED,
            false,
            "a modem missed for one minute's read",
        ),
        (
            UNHEARD,
            COLLECTED + FRESH + 1,
            false,
            "a record from before a suspend",
        ),
        (
            UNHEARD,
            COLLECTED - 5,
            true,
            "a clock a little behind the record",
        ),
    ] {
        assert_eq!(off_home(&network(&unheard, since), now), off, "{said}");
    }
}

#[test]
fn only_a_file_that_wont_open_as_a_tvs_is_offered_forgetting() {
    let tv = device("Living", &["television"], &[]);
    let cases = [
        (
            kept::Unopened::Unseen(std::io::ErrorKind::PermissionDenied.into()),
            false,
        ),
        (
            kept::Unopened::Unrun(std::io::ErrorKind::NotFound.into()),
            false,
        ),
        (kept::Unopened::Unserved, false),
        (kept::Unopened::Unsealed, true),
        (kept::Unopened::Unread, true),
    ];
    for (unopened, forgets) in cases {
        let said = format!("{unopened:?}");
        let failure = unopened_as(&tv, "mute", unopened).json(false);
        let tried = failure["try"].as_str().unwrap_or_default();
        if forgets {
            assert!(
                kept_file(&tv).is_some(),
                "no file is known to offer removing"
            );
            assert!(tried.starts_with("rm "), "{said} offers {tried}");
        } else {
            assert!(failure["try"].is_null(), "{said} offers {tried}");
        }
    }
}

#[test]
fn a_refused_token_that_couldnt_be_forgotten_is_said_so_and_keeps_the_certificate() {
    let tv = device("Living", &["television"], &[]);
    let failure = unforgotten(&tv, "mute").json(false);
    let because = failure["why"].as_str().unwrap_or_default();
    assert!(
        because.contains("couldn't forget"),
        "the failure says the token was forgotten: {because}"
    );
    assert_eq!(
        failure["try"], "net devices mac:02:00:00:00:00:90 press mute",
        "the try line doesn't press again, keeping the certificate"
    );
}

#[test]
fn only_a_tv_that_refused_or_stopped_asking_is_said_not_to_allow() {
    let tv = device("Living", &["television"], &[]);
    for why in [remote::Why::Refused, remote::Why::Unanswered] {
        assert_eq!(unallowed(&tv, &why), "Living didn't allow this machine");
    }
    assert_eq!(
        unallowed(&tv, &remote::Why::Forgotten),
        "Living no longer allows this machine"
    );
    for why in [
        remote::Why::Unreachable("timed out".into()),
        remote::Why::NotTheTv,
        remote::Why::Broke("reset".into()),
        remote::Why::Unlike("bad frame".into()),
    ] {
        let said = unallowed(&tv, &why);
        assert!(!said.contains("allow"), "{why:?} blames Allow: {said}");
    }
}

#[test]
fn the_answers_say_the_device_the_key_and_what_was_heard() {
    let tv = device("Living", &["television"], &[]);
    assert_eq!(
        pressed_answer(&tv, "Volume-Up", true),
        json!({
            "schema": "net.pressed/1",
            "device": "mac:02:00:00:00:00:90",
            "key": "volume-up",
            "allowed_now": true,
        })
    );
    assert_eq!(
        woken_answer(&tv, false),
        json!({"schema": "net.woken/1", "device": "mac:02:00:00:00:00:90", "heard": false})
    );
}
