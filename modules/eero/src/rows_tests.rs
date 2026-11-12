//! The Home Network's rows made from the model the contract draws
//! (home-network.json), in the launcher's own words for them
//! (tests/words.json, a slice of launcher/words.json that
//! `the-daemons-test-the-launchers-words` holds to the vocabulary). They're
//! the cases tests/home-network.qml holds HomeNetwork.js to, which stay
//! beside it until the place draws these rows.

use super::*;

/// The contract's network, which the place's drawer is tested on too.
pub(crate) fn model() -> HomeNetwork {
    serde_json::from_str(include_str!("../home-network.json")).expect("the model parses")
}

/// The launcher's words for the Home Network, which its drawer says too.
pub(crate) fn words() -> Words {
    let vocabulary: Value =
        serde_json::from_str(include_str!("../tests/words.json")).expect("the words parse");
    Words::read(&vocabulary).expect("the vocabulary has the Home Network's words")
}

fn made(model: &HomeNetwork) -> Vec<Value> {
    rows(model, &words(), &Numbers::default())
}

/// Items as the shell joins them, a time said by its form in brackets.
fn joined(items: &Value) -> String {
    items
        .as_array()
        .expect("items are a list")
        .iter()
        .map(|item| match item["as"].as_str() {
            Some(form) => format!("<{form}>"),
            None => item["said"].as_str().unwrap_or("").to_owned(),
        })
        .collect::<Vec<_>>()
        .join(DOT)
}

fn named<'a>(rows: &'a [Value], name: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("no row is named {name}"))
}

fn side_and_note(row: &Value) -> (String, String) {
    (joined(&row["side"]), joined(&row["note"]))
}

#[test]
fn the_internet_comes_first_then_each_node_with_its_devices_then_the_profiles() {
    let rows = made(&model());
    let names: Vec<&str> = rows.iter().filter_map(|row| row["name"].as_str()).collect();
    let at = |name: &str| names.iter().position(|one| *one == name).expect(name);
    assert_eq!(names[0], "Internet", "the internet heads the place");
    assert_eq!(
        &names[at("Hall")..at("Den")],
        ["Hall", "generic 3", "phone 1"],
        "the gateway node first, its devices under it"
    );
    assert_eq!(
        &names[at("Den")..at("Study")],
        [
            "Den",
            "generic 2",
            "laptop computer 2",
            "printer 1",
            "tablet 1"
        ],
        "then the nodes hanging from it, by location"
    );
    assert_eq!(
        &names[at("Study") + 1..at("Study") + 3],
        ["laptop computer 1", "generic 4"],
        "this machine first under its node"
    );
    assert_eq!(
        &names[names.len() - 3..],
        ["profile 1", "profile 2", "profile 3"],
        "the profiles last"
    );
    let devices = rows.iter().filter(|row| row["depth"] == 1).count();
    assert_eq!(devices, 15, "every device on now is a row inside its node");
}

#[test]
fn each_row_is_keyed_in_the_place_and_by_the_thing_it_is() {
    let rows = made(&model());
    let hall = named(&rows, "Hall");
    assert_eq!(hall["key"], "row:eero:node:2003");
    assert_eq!(hall["thing"], "eero:node:2003");
    assert_eq!(
        hall["glyph"], "accessPoint",
        "the gateway node wears eero:gateway's glyph"
    );
    assert_eq!(
        hall["keywords"][0], "eero Pro 6",
        "a node is found by its model"
    );
    assert_eq!(rows[0]["key"], "row:eero:internet");
    assert_eq!(rows[0]["glyph"], "web");
    let phone = named(&rows, "phone 1");
    assert_eq!(phone["glyph"], "phone", "a device wears its kind's mark");
    assert_eq!(phone["depth"], 1);
    assert!(
        rows.iter()
            .all(|row| row["place"] == PLACE && row.get("order").is_none()),
        "every row is the place's, its order Kept's to give"
    );
}

#[test]
fn the_internet_says_how_it_stands_and_its_last_speed_test() {
    let rows = made(&model());
    assert_eq!(
        side_and_note(&rows[0]),
        (
            "connected \u{b7} tested 370 Mb/s down".to_owned(),
            "11.9 Mb/s up \u{b7} <on> \u{b7} conferencing and gaming on \u{b7} 1 outage in 30 days \
             \u{b7} 3 nodes online \u{b7} 15 devices on now"
                .to_owned()
        )
    );
    assert_eq!(
        rows[0]["note"][1],
        json!({"at": 1_793_875_935_000_i64, "as": "on"}),
        "the test's day travels as a time, so the row never changes as the day does"
    );
}

#[test]
fn the_internet_says_why_nothing_is_known_while_eero_is_out_of_reach() {
    let mut model = model();
    model.cloud.state = CloudState::Unreachable;
    assert_eq!(joined(&made(&model)[0]["side"]), "eero unreachable");
    model.cloud.state = CloudState::SignedOut;
    assert_eq!(joined(&made(&model)[0]["side"]), "signed out of eero");
}

#[test]
fn the_internet_says_the_home_network_and_the_provider_only_when_they_ail() {
    let mut model = model();
    model.internet.mesh = Some("degraded".into());
    model.internet.isp_up = Some(false);
    model.internet.history.outages.clear();
    let note = joined(&made(&model)[0]["note"]);
    assert!(
        note.starts_with("home network degraded \u{b7} provider down \u{b7} 11.9 Mb/s up"),
        "{note}"
    );
    assert!(note.contains("no outages in 30 days"), "{note}");
}

#[test]
fn a_node_says_its_devices_and_how_it_reaches_the_one_before() {
    let rows = made(&model());
    assert_eq!(
        side_and_note(named(&rows, "Hall")),
        (
            "2 devices".to_owned(),
            "gateway node \u{b7} Ethernet \u{b7} eero Pro 6 \u{b7} v7.17.4-5".to_owned()
        )
    );
    assert_eq!(
        side_and_note(named(&rows, "Study")),
        (
            "9 devices".to_owned(),
            "Wi-Fi from Hall over 5 GHz high \u{b7} 5 of 5 bars \u{b7} eero Pro 6 \u{b7} v7.17.4-5"
                .to_owned()
        )
    );
}

#[test]
fn a_node_off_line_says_so_first() {
    let mut model = model();
    model.nodes[1].state = Some("OFFLINE".into());
    let note = joined(&named(&made(&model), "Study")["note"]);
    assert!(
        note.starts_with("offline \u{b7} Wi-Fi"),
        "an offline node says so first: {note}"
    );
}

#[test]
fn a_device_says_its_signal_band_rates_speed_and_address() {
    let rows = made(&model());
    assert_eq!(
        joined(&named(&rows, "laptop computer 1")["side"]),
        "this machine \u{b7} \u{2212}75 dBm"
    );
    let generic = named(&rows, "generic 2");
    assert_eq!(
        side_and_note(generic),
        (
            "\u{2212}37 dBm".to_owned(),
            "5 GHz \u{b7} channel 52 \u{b7} 1.20 Gb/s down \u{b7} 1.08 Gb/s up \u{b7} idle \
             \u{b7} 10.0.0.45"
                .to_owned()
        )
    );
    assert_eq!(
        generic["glyph"], "wireless",
        "a device of no kind it knows wears Wi-Fi's"
    );
    assert_eq!(
        joined(&named(&rows, "unknown computer 1")["note"]),
        "5 GHz \u{b7} channel 149 \u{b7} 1.20 Gb/s down \u{b7} 1.20 Gb/s up \u{b7} 11.5 Mb/s \
         down \u{b7} 9.86 Mb/s up \u{b7} 10.0.0.20"
    );
    assert_eq!(
        joined(&named(&rows, "printer 1")["note"]),
        "2.4 GHz \u{b7} channel 6 \u{b7} 72.2 Mb/s down \u{b7} idle \u{b7} 10.0.0.31"
    );
    assert_eq!(
        joined(&named(&rows, "generic 3")["note"]),
        "2.4 GHz \u{b7} channel 6 \u{b7} 72.2 Mb/s down \u{b7} 98.2 b/s down \u{b7} 154 b/s up \
         \u{b7} 10.0.0.99"
    );
}

#[test]
fn a_device_says_the_profile_its_in_and_ethernet_where_its_cabled() {
    let mut model = model();
    let printer = model
        .devices
        .iter_mut()
        .find(|device| device.name.as_deref() == Some("printer 1"))
        .expect("the printer is on");
    printer.profile = Some("3002".into());
    let generic = model
        .devices
        .iter_mut()
        .find(|device| device.name.as_deref() == Some("generic 3"))
        .expect("generic 3 is on");
    generic.ethernet = true;
    let rows = made(&model);
    let note = joined(&named(&rows, "printer 1")["note"]);
    assert!(
        note.ends_with("10.0.0.31 \u{b7} profile 2"),
        "a device ends with where it is: {note}"
    );
    let cabled = named(&rows, "generic 3");
    assert_eq!(
        side_and_note(cabled),
        (
            "Ethernet".to_owned(),
            "Ethernet \u{b7} 72.2 Mb/s down \u{b7} 98.2 b/s down \u{b7} 154 b/s up \u{b7} \
             10.0.0.99"
                .to_owned()
        )
    );
    assert_eq!(cabled["glyph"], "wired");
}

#[test]
fn a_device_nothing_names_is_called_by_its_maker_in_the_launchers_words() {
    let mut model = model();
    let device = &mut model.devices[0];
    device.name = None;
    device.nickname = None;
    device.hostname = None;
    device.make = None;
    device.manufacturer = Some("Intel Corporate".to_owned());
    let mac = device.mac.clone();

    let rows = made(&model);

    let row = rows
        .iter()
        .find(|row| row["thing"] == format!("eero:device:{mac}"))
        .expect("the device has a row");
    assert_eq!(row["name"], "Intel device");
}

#[test]
fn a_device_is_found_by_its_address_hostname_and_kind() {
    let rows = made(&model());
    let keywords = &named(&rows, "laptop computer 1")["keywords"];
    assert!(
        keywords
            .as_array()
            .is_some_and(|words| words.iter().any(|word| word == "laptop computer")),
        "its kind is worded: {keywords}"
    );
}

#[test]
fn a_profile_says_how_it_stands_and_how_many_of_its_devices_are_on() {
    let mut model = model();
    assert_eq!(
        side_and_note(named(&made(&model), "profile 1")),
        (
            "active".to_owned(),
            "10 devices \u{b7} none on now".to_owned()
        )
    );
    let on = model.devices[0].mac.clone();
    model.profiles[1].paused = Some(true);
    model.profiles[1].devices = vec![on, "02:00:00:00:00:35".into()];
    assert_eq!(
        side_and_note(named(&made(&model), "profile 2")),
        ("paused".to_owned(), "2 devices \u{b7} 1 on now".to_owned())
    );
}

#[test]
fn a_rate_is_said_in_whichever_unit_reads_it_in_three_digits() {
    let words = words();
    let numbers = Numbers::default();
    let said: Vec<String> = [1200.9, 369.86, 48.0, 999.96, 0.023_960_4, 0.000_098_2]
        .map(|mbps| words.rate(mbps, &numbers))
        .into();
    assert_eq!(
        said,
        [
            "1.20 Gb/s",
            "370 Mb/s",
            "48.0 Mb/s",
            "1.00 Gb/s",
            "24.0 kb/s",
            "98.2 b/s"
        ]
    );
}

#[test]
fn a_vocabulary_without_a_word_says_which() {
    let mut vocabulary: Value =
        serde_json::from_str(include_str!("../tests/words.json")).expect("the words parse");
    vocabulary["things"][1]["said"]
        .as_object_mut()
        .expect("the internet says words")
        .remove("sqmOn");
    assert_eq!(
        Words::read(&vocabulary),
        Err("the vocabulary's eero:internet has no said.sqmOn".into())
    );
}

/// The rows the desktop's tests serve as the daemon would
/// (tests/desktop/stand-in-eero.py), held to the records the daemon serves
/// of the contract's network, each ordered as Kept orders it and the
/// place's own drawer and each profile's beside them, so the stand-in
/// never says what the daemon wouldn't.
/// `EERO_GOLDEN=write cargo test` writes them afresh, to be read before
/// they're committed. They're compared as JSON, so `nix fmt` may lay the
/// file out as it likes.
#[test]
fn the_rows_the_desktops_tests_serve_are_the_daemons() {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/rows.json");
    let mut kept = ready_rows::kept::Kept::default();
    let model = model();
    let numbers = Numbers::default();
    let beside = std::iter::once(crate::place::record(&model, &words(), &numbers))
        .chain(crate::drawers::records(&model, &words(), &numbers))
        .collect();
    kept.take(made(&model), beside);
    let rows: Value = kept.all().cloned().collect();
    if std::env::var("EERO_GOLDEN").as_deref() == Ok("write") {
        let text = serde_json::to_string_pretty(&rows).expect("rows are JSON");
        std::fs::write(&file, text + "\n").expect("tests/rows.json can be written");
        return;
    }
    let golden: Value = std::fs::read_to_string(&file)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(|| {
            panic!(
                "no rows at {}; write them with EERO_GOLDEN=write",
                file.display()
            )
        });
    // Read back as the file holds them, since a share such as 0.38 may not
    // read back as the very float it was written from.
    let rows: Value = serde_json::to_string(&rows)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .expect("rows are JSON");
    assert!(
        golden == rows,
        "tests/rows.json isn't what the daemon makes of home-network.json; write it afresh with \
         EERO_GOLDEN=write"
    );
}
