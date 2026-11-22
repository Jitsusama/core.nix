//! The tests of samsung.rs: which boxes eero lists are asked, what a TV's
//! answer says, and how a TV heard once keeps its name through a minute
//! it can't be asked; and the TV and its listing the collector's tests
//! share.

use super::*;

use std::fs;

use serde_json::json;

/// The contract's TV: its hardware address and address, as the eero
/// record the collector's tests add it to gives them.
pub const MAC: &str = "02:00:00:00:00:90";
pub const ADDRESS: &str = "10.0.0.90";

/// Another box eero lists, by its hardware address.
const OTHER: &str = "02:00:00:00:00:92";

const NOW: u64 = 1_794_702_033;

/// What the contract's TV answered.
pub fn answer() -> Answer {
    let file = format!(
        "{}/contract/samsung/api-v2.json",
        env!("CARGO_MANIFEST_DIR")
    );
    fs::read_to_string(&file).expect("the contract has the TV's answer")
}

/// The contract's TV, asked once, as the daemon's first minute asks it.
pub fn asked() -> Asked {
    answered(answer())
}

/// The TV, asked once, giving `answer`.
pub fn answered(answer: Answer) -> Asked {
    remember(&mut Heard::new(), MAC.into(), Reply::Page(answer))
}

/// The TV as eero lists it: by the name it gives when it takes an
/// address, with no nickname, and as a box eero can't place.
pub fn listing() -> Value {
    json!({
        "mac": MAC, "name": "samsung", "nickname": null,
        "hostname": "samsung", "kind": "generic", "make": "Samsung",
        "manufacturer": "Samsung Electronics Co.,Ltd", "address": ADDRESS,
        "addresses": [ADDRESS], "node": "2001", "ethernet": false,
        "band": "5 GHz", "channel": 149,
    })
}

fn silent() -> Reply {
    Reply::Silent("it didn't answer: timed out".into())
}

fn one(answer: Answer) -> (Option<Source>, Vec<Found>) {
    read(
        vec![remember(&mut Heard::new(), MAC.into(), Reply::Page(answer))],
        NOW,
    )
}

#[test]
fn a_samsung_that_could_be_a_tv_is_listed_and_no_phone_or_other_maker() {
    let mut phone = listing();
    phone["mac"] = json!("02:00:00:00:00:91");
    phone["kind"] = json!("phone");
    let mut streamer = listing();
    streamer["mac"] = json!(OTHER);
    streamer["kind"] = json!("media_streamer");
    let mut lg = listing();
    lg["mac"] = json!("02:00:00:00:00:93");
    lg["make"] = json!("LG");
    let eero = json!({"devices": [listing(), phone, streamer, lg]});
    assert_eq!(
        listed(&eero),
        vec![
            (MAC.to_owned(), ADDRESS.to_owned()),
            (OTHER.to_owned(), ADDRESS.to_owned())
        ],
        "only the Samsung boxes that could be a TV are asked"
    );
}

#[test]
fn eeros_golden_record_lists_its_generic_samsung_and_no_other() {
    let eero: Value = serde_json::from_str(include_str!("../../../eero/home-network.json"))
        .expect("eero's golden record reads");
    let asked = listed(&eero);
    assert_eq!(
        asked,
        [("02:00:00:00:00:15".to_owned(), "10.0.0.25".to_owned())],
        "eero's one generic Samsung isn't asked alone, at its own address"
    );
}

#[test]
fn the_answer_says_its_name_model_and_power() {
    let (source, found) = one(answer());
    let found = found.first().expect("the TV is heard");
    assert_eq!(found.name.as_deref(), Some("Living Room"), "its own name");
    assert_eq!(found.model.as_deref(), Some("QN55Q70CAFXZC"), "its model");
    assert_eq!(found.television.power.as_deref(), Some("on"), "its power");
    let detail = found.detail.as_ref().expect("its answer is kept");
    assert_eq!(detail["device"]["OS"], json!("Tizen"), "its answer is kept");
    assert_eq!(detail["version"], json!("2.0.25"), "its answer is kept");
    assert_eq!(
        source.map(|source| source.state),
        Some(SourceState::Answering),
        "the source isn't answering"
    );
}

#[test]
fn a_tv_in_standby_says_so() {
    let standby = answer().replace("\"PowerState\": \"on\"", "\"PowerState\": \"standby\"");
    let (_, found) = one(standby);
    assert_eq!(
        found
            .first()
            .and_then(|found| found.television.power.as_deref()),
        Some("standby"),
        "its power isn't as it said"
    );
}

#[test]
fn its_ids_and_addresses_are_left_out() {
    let (_, found) = one(answer());
    let detail = found.first().and_then(|found| found.detail.clone());
    let kept = serde_json::to_string(&detail).expect("its detail writes");
    for said in [
        "00000000-0000-0000-0000-000000000090",
        ADDRESS,
        MAC,
        "02:00:00:00:20:01",
        "0.0.0.0",
    ] {
        assert!(!kept.contains(said), "{said} is kept: {kept}");
    }
    assert!(
        kept.contains("QN55Q70CAFXZC"),
        "the rest of its answer went too: {kept}"
    );
}

#[test]
fn a_samsung_that_isnt_a_tv_is_left_out_without_a_source() {
    let soundbar = answer().replace(A_TV, "Samsung Soundbar");
    let mut heard = Heard::new();
    let asked = remember(&mut heard, MAC.into(), Reply::Page(soundbar));
    assert!(heard.is_empty(), "a soundbar is remembered as a TV");
    assert_eq!(
        read(vec![asked], NOW),
        (None, Vec::new()),
        "a soundbar says something"
    );
}

#[test]
fn a_box_never_heard_as_a_tv_says_nothing_when_silent() {
    let asked = remember(&mut Heard::new(), OTHER.into(), silent());
    assert_eq!(
        read(vec![asked], NOW),
        (None, Vec::new()),
        "a silent phone says something"
    );
}

#[test]
fn a_tv_heard_once_keeps_its_name_while_silent() {
    let mut heard = Heard::new();
    remember(&mut heard, MAC.into(), Reply::Page(answer()));
    let asked = remember(&mut heard, MAC.into(), silent());
    let (source, found) = read(vec![asked], NOW);
    let [tv] = found.as_slice() else {
        panic!("the silent TV should be one: {found:?}");
    };
    assert_eq!(
        tv.name.as_deref(),
        Some("Living Room"),
        "its name went with its answer"
    );
    assert_eq!(
        tv.model.as_deref(),
        Some("QN55Q70CAFXZC"),
        "its model went with its answer"
    );
    assert_eq!(
        tv.television,
        Television {
            power: None,
            answering: false
        },
        "its power is said from an old answer"
    );
    assert_eq!(tv.detail, None, "an old answer is kept as this minute's");
    assert_eq!(
        source.map(|source| (source.state, source.read_at, source.reason)),
        Some((SourceState::Answering, None, None)),
        "a TV asleep reads as the source failing, or as read this minute"
    );
}

#[test]
fn a_tv_heard_once_that_refuses_is_said_as_answering_oddly() {
    let mut heard = Heard::new();
    remember(&mut heard, MAC.into(), Reply::Page(answer()));
    let asked = remember(&mut heard, MAC.into(), Reply::Refused("it says 403".into()));
    let (source, found) = read(vec![asked], NOW);
    let [tv] = found.as_slice() else {
        panic!("the TV should be kept as one: {found:?}");
    };
    assert_eq!(
        (tv.name.as_deref(), tv.television.answering),
        (Some("Living Room"), false),
        "a TV that refuses loses its name, or reads as answering"
    );
    assert_eq!(
        source.map(|source| (source.state, source.reason)),
        Some((
            SourceState::Unreachable,
            Some("Living Room: its answer isn't a TV's: it says 403".into())
        )),
        "a refusal reads as the TV asleep"
    );
}

#[test]
fn a_tv_heard_once_that_answers_as_something_else_keeps_its_name() {
    let mut heard = Heard::new();
    remember(&mut heard, MAC.into(), Reply::Page(answer()));
    let odd = answer().replace(A_TV, "Samsung Soundbar");
    let asked = remember(&mut heard, MAC.into(), Reply::Page(odd));
    let (source, found) = read(vec![asked], NOW);
    let [tv] = found.as_slice() else {
        panic!("the TV should be kept as one: {found:?}");
    };
    assert_eq!(
        tv.name.as_deref(),
        Some("Living Room"),
        "its name went with its answer"
    );
    assert_eq!(
        tv.model.as_deref(),
        Some("QN55Q70CAFXZC"),
        "its model went with its answer"
    );
    assert!(
        !tv.television.answering,
        "an answer that isn't a TV's reads as one"
    );
    assert_eq!(
        source.map(|source| (source.state, source.reason)),
        Some((
            SourceState::Unreachable,
            Some("Living Room: its answer isn't a TV's: it says it's Samsung Soundbar".into())
        )),
        "an odd answer isn't said as the source failing, with what it answered"
    );
}

/// Another TV, as eero lists it.
fn other() -> Value {
    let mut other = listing();
    other["mac"] = json!(OTHER);
    other["address"] = json!("10.0.0.92");
    other
}

/// A box eero lists that isn't a Samsung, so it's never asked.
fn phone() -> Value {
    let mut phone = listing();
    phone["mac"] = json!("02:00:00:00:00:94");
    phone["address"] = json!("10.0.0.94");
    phone["make"] = json!("Apple");
    phone
}

#[test]
fn a_tv_eero_no_longer_knows_is_forgotten() {
    let mut tvs = Heard::new();
    let both = Ok::<Value, String>(json!({"devices": [listing(), other()]}));
    ask(&mut tvs, &both, |_| Reply::Page(answer()));
    let one = Ok::<Value, String>(json!({"devices": [other()]}));
    ask(&mut tvs, &one, |_| silent());
    let kept = tvs.keys().collect::<Vec<_>>();
    assert_eq!(kept, [OTHER], "a TV that left is still remembered");
    let none = Ok::<Value, String>(json!({"devices": [phone()]}));
    let asked = ask(&mut tvs, &none, |_| silent());
    assert!(
        asked.is_empty() && tvs.is_empty(),
        "a record listing no TV kept the last TV or asked one: {:?}",
        tvs.keys().collect::<Vec<_>>()
    );
}

#[test]
fn a_tv_eero_still_knows_keeps_its_name_while_it_cant_be_asked() {
    let mut addressless = listing();
    addressless["address"] = Value::Null;
    let mut unplaced = listing();
    unplaced["make"] = Value::Null;
    let off = json!({"mac": MAC, "name": "samsung", "make": "Samsung"});
    for (case, eero) in [
        ("listed with no address", json!({"devices": [addressless]})),
        ("listed with no maker", json!({"devices": [unplaced]})),
        (
            "listed as off",
            json!({"devices": [phone()], "devices_off": [off]}),
        ),
    ] {
        let mut tvs = Heard::new();
        let on = Ok::<Value, String>(json!({"devices": [listing()]}));
        ask(&mut tvs, &on, |_| Reply::Page(answer()));
        let names = |asked| {
            let (_, found) = read(asked, NOW);
            found.into_iter().map(|tv| tv.name).collect::<Vec<_>>()
        };
        let mut reached = false;
        let asked = ask(&mut tvs, &Ok::<Value, String>(eero), |_| {
            reached = true;
            silent()
        });
        assert!(!reached, "a TV {case} was asked");
        // Net shows only what eero lists on, so that minute a TV listed
        // on keeps its name and role, and one listed off isn't shown.
        let that_minute: &[Option<String>] = if case == "listed as off" {
            &[]
        } else {
            &[Some("Living Room".to_owned())]
        };
        assert_eq!(
            names(asked),
            that_minute,
            "a TV {case} lost its name that minute"
        );
        assert_eq!(
            names(ask(&mut tvs, &on, |_| silent())),
            [Some("Living Room".to_owned())],
            "a TV {case} for a minute lost its name once back"
        );
    }
}

#[test]
fn a_record_listing_no_devices_forgets_no_tv_and_asks_none() {
    let mut tvs = Heard::new();
    let eero = Ok::<Value, String>(json!({"devices": [listing()]}));
    ask(&mut tvs, &eero, |_| Reply::Page(answer()));
    for empty in [json!({}), json!({"devices": []}), json!({"devices": null})] {
        let mut asked_anyway = false;
        let asked = ask(&mut tvs, &Ok::<Value, String>(empty.clone()), |_| {
            asked_anyway = true;
            silent()
        });
        assert!(
            asked.is_empty() && !asked_anyway,
            "a record listing no devices asked a TV: {empty}"
        );
        assert_eq!(
            tvs.keys().collect::<Vec<_>>(),
            [MAC],
            "a record listing no devices forgot the TV: {empty}"
        );
    }
}

#[test]
fn a_tv_keeps_its_name_through_a_minute_net_couldnt_read_eero() {
    let mut heard = Heard::new();
    let eero = Ok::<Value, String>(json!({"devices": [listing()]}));
    let mut addresses = Vec::new();
    ask(&mut heard, &eero, |address| {
        addresses.push(address.to_owned());
        Reply::Page(answer())
    });
    assert_eq!(
        addresses,
        [ADDRESS],
        "the TV isn't asked at eero's address for it"
    );
    let mut asked_anyway = false;
    let unread = Err::<Value, _>(
        "eero hasn't written what it read; see systemctl --user status eero-daemon".to_owned(),
    );
    let none = ask(&mut heard, &unread, |_| {
        asked_anyway = true;
        silent()
    });
    assert!(
        none.is_empty() && !asked_anyway,
        "a TV is asked in a minute net couldn't read eero's record"
    );
    let (source, found) = read(ask(&mut heard, &eero, |_| silent()), NOW);
    let said: Vec<_> = found
        .iter()
        .map(|tv| (tv.name.as_deref(), tv.television.answering))
        .collect();
    assert_eq!(
        said,
        [(Some("Living Room"), false)],
        "a minute net couldn't read eero's record forgot the TV"
    );
    assert_eq!(
        source.map(|source| source.state),
        Some(SourceState::Answering),
        "a TV asleep after an unread minute reads as the source failing"
    );
}

#[test]
fn a_tv_answering_without_its_power_says_none() {
    let quiet = answer().replace("\"PowerState\": \"on\",", "");
    assert_ne!(
        quiet,
        answer(),
        "the contract's answer no longer says its power"
    );
    let (source, found) = one(quiet);
    assert_eq!(
        found.first().map(|tv| (tv.name.as_deref(), &tv.television)),
        Some((
            Some("Living Room"),
            &Television {
                power: None,
                answering: true
            }
        )),
        "a TV that doesn't say its power is given one, or isn't heard as answering"
    );
    assert_eq!(
        source.map(|source| source.state),
        Some(SourceState::Answering),
        "a TV without its power reads as the source failing"
    );
}

#[test]
fn a_word_the_tv_leaves_blank_is_unsaid() {
    let blank = answer()
        .replace("\"PowerState\": \"on\"", "\"PowerState\": \" \"")
        .replace("\"name\": \"Living Room\"", "\"name\": \"\"");
    assert!(
        !blank.contains("\"on\"") && !blank.contains("Living Room"),
        "the contract's answer no longer says its power and name as this test expects"
    );
    let (_, found) = one(blank);
    assert_eq!(
        found
            .first()
            .map(|tv| (tv.name.as_deref(), tv.television.power.as_deref())),
        Some((None, None)),
        "a blank name or power is said as a word"
    );
}

#[test]
fn an_answer_that_isnt_json_is_never_remembered() {
    let mut heard = Heard::new();
    remember(&mut heard, MAC.into(), Reply::Page("<html>".into()));
    assert!(heard.is_empty(), "a page is remembered as a TV");
}

#[test]
fn an_odd_tv_beside_one_answering_leaves_the_source_answering() {
    let mut heard = Heard::new();
    remember(&mut heard, OTHER.into(), Reply::Page(answer()));
    let odd = answer().replace(A_TV, "Samsung Soundbar");
    let (source, found) = read(
        vec![
            remember(&mut heard, OTHER.into(), Reply::Page(odd)),
            remember(&mut heard, MAC.into(), Reply::Page(answer())),
        ],
        NOW,
    );
    assert_eq!(found.len(), 2, "both TVs aren't said");
    assert_eq!(
        source.map(|source| (source.state, source.read_at, source.reason)),
        Some((SourceState::Answering, Some(NOW), None)),
        "one odd TV makes the source unreachable while another answers"
    );
}

#[test]
fn a_home_with_no_samsung_asks_nothing_and_says_no_source() {
    let mut lg = listing();
    lg["make"] = json!("LG");
    let asked = listed(&json!({"devices": [lg]}));
    assert!(asked.is_empty(), "another maker's box is asked: {asked:?}");
    assert_eq!(
        read(Vec::new(), NOW),
        (None, Vec::new()),
        "a source is said for nothing"
    );
}
