//! Passwords' rows' cases, as Passwords.js's own in tests/passwords.qml
//! say the shell's rows today.

use super::*;

/// The words the daemon's own tests serve rows in too.
fn words() -> Words {
    let vocabulary: Value = serde_json::from_str(include_str!("../tests/words.json"))
        .expect("the tests' vocabulary isn't JSON");
    Words::read(&vocabulary).expect("the vocabulary's words didn't read")
}

fn login(id: &str, title: &str, changed: u64) -> Value {
    json!({
        "key": format!("entry:onepassword:{id}"), "kind": "entry", "manager": "onepassword",
        "id": id, "title": title, "category": "login", "username": "ada",
        "websites": [{"address": "https://example.com/login", "primary": true}],
        "collection": {"id": "v1", "name": "Keeping"}, "own": {"account": "acct"},
        "changed": changed, "created": 1_000, "heard": 1,
    })
}

fn one_manager() -> Vec<Shown> {
    vec![Shown {
        name: "onepassword".into(),
        called: "1Password".into(),
        shows: "onepassword://view-item?a={account}&v={collection}&i={id}".into(),
    }]
}

fn field<'a>(rows: &'a [Value], field: &str) -> Vec<&'a Value> {
    rows.iter().map(|row| &row[field]).collect()
}

#[test]
fn the_vocabulary_missing_a_word_is_said_rather_than_read_blank() {
    let lacking = json!({"things": [{"id": "passwords:health", "name": "Health"}]});
    assert!(
        Words::read(&lacking).is_err_and(|lack| lack.contains("passwords:strength")),
        "a vocabulary without Strength read as whole"
    );
}

#[test]
fn an_entry_is_a_row_keyed_as_the_launcher_keys_it_with_no_secret() {
    let mut entry = login("a", "Example", 5_000);
    entry["details"] = json!([
        {"id": "password", "purpose": "password", "hidden": true, "strength": "VERY_GOOD"}
    ]);
    let rows = rows(&[entry], &one_manager(), &words());
    let row = &rows[0];
    assert_eq!(row["key"], "row:entry:onepassword:a");
    assert_eq!(row["row"], "entry:onepassword:a");
    assert_eq!(row["place"], "passwords");
    assert_eq!(row["name"], "Example");
    assert_eq!(row["glyph"], "login");
    assert_eq!(row["hue"], "machine");
    assert_eq!(row["side"], json!([{"said": "ada"}]));
    assert_eq!(
        row["note"], "example.com",
        "one manager's name was said on its rows"
    );
    assert_eq!(row["passworded"], true);
    assert!(
        !row.to_string().contains("\"value\":\"s3cret"),
        "a row held a secret: {row}"
    );
}

#[test]
fn the_favourites_lead_by_title_then_the_latest_changed_and_the_rest_are_only_found() {
    let mut records: Vec<Value> = (0..25)
        .map(|at| login(&format!("e{at}"), &format!("Entry {at:02}"), 1_000 + at))
        .collect();
    records[3]["favourite"] = json!(true);
    records[1]["favourite"] = json!(true);
    records[1]["title"] = json!("banking");
    records[3]["title"] = json!("Alpha");
    let rows = rows(&records, &one_manager(), &words());
    let ordered = |at: usize| rows[at]["order"].as_u64();
    assert_eq!(ordered(3), Some(0), "Alpha, a favourite, didn't lead");
    assert_eq!(
        ordered(1),
        Some(1),
        "banking didn't follow Alpha by its letters"
    );
    assert_eq!(
        ordered(24),
        Some(2),
        "the latest changed didn't follow the favourites"
    );
    assert_eq!(
        field(&rows, "order")
            .iter()
            .filter(|order| order.is_u64())
            .count(),
        22
    );
    assert_eq!(
        ordered(0),
        None,
        "one changed earliest was listed past twenty"
    );
}

#[test]
fn those_the_check_found_breached_or_reused_need_attention_by_title() {
    let mut records = vec![
        login("a", "Zulu", 1),
        login("b", "Mike", 2),
        login("c", "Alpha", 3),
    ];
    records[0]["health"] = json!({"reused": 2, "breached": 0});
    records[1]["health"] = json!({"reused": 0, "breached": 0});
    records[2]["health"] = json!({"reused": 0, "breached": 1_204});
    let rows = rows(&records, &one_manager(), &words());
    assert_eq!(
        field(&rows, "attention"),
        [&json!(1), &Value::Null, &json!(0)]
    );
    let health = |at: usize| rows[at]["drawer"]["facts"][0].clone();
    assert_eq!(
        health(0),
        json!({"label": "Health", "value": "reused in 2 other entries", "warns": true})
    );
    assert_eq!(health(2)["value"], "seen in 1,204 breaches");
}

#[test]
fn its_acts_come_in_the_design_order_each_with_what_it_asks() {
    let mut entry = login("a", "Example", 1);
    entry["details"] = json!([
        {"id": "pw", "purpose": "password", "hidden": true},
        {"id": "otp", "type": "code", "hidden": true}
    ]);
    let rows = rows(&[entry], &one_manager(), &words());
    let shown = "onepassword://view-item?a=acct&v=v1&i=a";
    assert_eq!(
        rows[0]["acts"],
        json!([
            {"key": "verb:passwords:password",
             "ask": {"ask": "copy", "key": "entry:onepassword:a", "detail": "pw"}},
            {"key": "verb:passwords:code",
             "ask": {"ask": "copy", "key": "entry:onepassword:a", "detail": "otp"}},
            {"key": "verb:passwords:username", "ask": {"copy": "ada"}},
            {"key": "verb:passwords:website",
             "ask": {"open": {"link": "https://example.com/login", "fills": true}}},
            {"key": "verb:passwords:show", "ask": {"open": {"link": shown}}},
            {"key": "verb:passwords:link", "ask": {"copy": shown}}
        ])
    );
}

#[test]
fn a_note_has_no_password_to_copy_and_no_link_without_its_parts() {
    let note = json!({"key": "entry:onepassword:n", "manager": "onepassword", "id": "n",
        "title": "", "category": "note"});
    let rows = rows(&[note], &one_manager(), &words());
    assert_eq!(rows[0]["passworded"], false);
    assert_eq!(rows[0]["name"], "Note", "a note without a title read blank");
    assert_eq!(rows[0]["glyph"], "secureNote");
    assert_eq!(
        rows[0]["acts"],
        json!([]),
        "a note without an account offered a link"
    );
}

#[test]
fn its_drawer_says_every_figure_but_the_secret_its_times_to_word() {
    let mut entry = login("a", "Example", 5_000);
    entry["websites"] = json!([
        {"address": "http://old.example.com"},
        {"address": "https://example.com/login", "primary": true}
    ]);
    entry["tags"] = json!(["work", "web"]);
    entry["details"] = json!([
        {"id": "pw", "purpose": "password", "hidden": true, "strength": "FAIR"},
        {"id": "notes", "purpose": "notes", "value": "Ask Ada."},
        {"id": "pin", "label": "pin", "section": "Extra", "hidden": true},
        {"id": "plan", "label": "plan", "value": "gold"}
    ]);
    let rows = rows(&[entry], &one_manager(), &words());
    assert_eq!(
        rows[0]["drawer"],
        json!({
            "eyebrow": "in Keeping \u{b7} login", "eyebrowNames": true, "title": "Example",
            "facts": [
                {"label": "Health", "value": "unsecured website", "warns": true},
                {"label": "Strength", "value": "fair", "warns": true},
                {"label": "One-Time Code", "value": "none"},
                {"label": "Last Changed", "at": 5_000.0, "as": "ago"},
                {"label": "Websites",
                 "value": "https://example.com/login, http://old.example.com"},
                {"label": "Tags", "value": "work, web"},
                {"label": "Created", "at": 1_000.0, "as": "ago"},
                {"label": "Plan", "value": "gold"},
                {"heading": "Extra"},
                {"label": "Pin", "value": "hidden"}
            ],
            "body": "Ask Ada."
        })
    );
}

#[test]
fn with_two_managers_each_row_names_its_own() {
    let mut managers = one_manager();
    managers.push(Shown {
        name: "other".into(),
        called: "Other".into(),
        shows: String::new(),
    });
    let rows = rows(&[login("a", "Example", 1)], &managers, &words());
    assert_eq!(rows[0]["note"], "1Password \u{b7} example.com");
    let facts = rows[0]["drawer"]["facts"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        facts.contains(&json!({"label": "Password Manager", "value": "1Password"})),
        "the drawer didn't name its manager among two: {facts:?}"
    );
}
