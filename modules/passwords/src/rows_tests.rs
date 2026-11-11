//! Passwords' rows' cases, as Passwords.js's own in tests/passwords.qml
//! say the shell's rows today.

use super::*;

/// The account's own vocabulary, which passwords-holds-no-secret hands the
/// tests, so every word a row says is the shell's.
fn vocabulary() -> Value {
    let path = std::env::var("LAUNCHER_WORDS")
        .expect("LAUNCHER_WORDS names no vocabulary; passwords-holds-no-secret hands it one");
    let read = std::fs::read_to_string(&path).expect("the launcher's words didn't read");
    serde_json::from_str(&read).expect("the launcher's words aren't JSON")
}

fn words() -> Words {
    Words::read(&vocabulary()).expect("the vocabulary's words didn't read")
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

/// Rows made with nothing told before them.
fn made(records: &[Value]) -> Vec<Value> {
    rows(records, &one_manager(), &Kept::default())
}

/// Each row as `kept` tells it once it takes them, its order given, in
/// the order the records came.
fn told(records: &[Value], kept: &mut Kept) -> Vec<Value> {
    let made = rows(records, &one_manager(), kept);
    kept.take(made, Vec::new());
    records
        .iter()
        .map(|record| {
            let key = row_key(text(record, "key"));
            kept.get(&key)
                .map_or(Value::Null, |told| told["row"].clone())
        })
        .collect()
}

/// The drawer a row names, as the daemon serves it beside the rows.
fn drawer_of(row: &Value, records: &[Value], managers: &[Shown]) -> Value {
    drawers(records, managers, &words())
        .into_iter()
        .find(|(key, _)| row["drawer"] == *key)
        .map(|(_, drawer)| drawer)
        .unwrap_or_default()
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
fn an_entry_is_a_row_keyed_by_its_place_with_no_secret() {
    let mut entry = login("a", "Example", 5_000);
    entry["details"] = json!([
        {"id": "password", "purpose": "password", "hidden": true, "strength": "VERY_GOOD",
         "value": "s3cret"}
    ]);
    let records = [entry];
    let rows = made(&records);
    let row = &rows[0];
    assert_eq!(row["key"], "row:passwords:entry:onepassword:a");
    assert_eq!(row["thing"], "entry:onepassword:a");
    assert_eq!(
        row["drawer"], "drawer:passwords:entry:onepassword:a",
        "a row carried its drawer rather than naming it by its place"
    );
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
        row["order"].is_null(),
        "a row was ordered before it was kept"
    );
    let drawer = drawer_of(row, &records, &one_manager());
    for said in [row, &drawer] {
        assert!(
            !said.to_string().contains("s3cret"),
            "it held a secret: {said}"
        );
    }
}

#[test]
fn the_favourites_lead_by_title_then_the_latest_changed_then_the_rest_by_title() {
    let mut records: Vec<Value> = (0..25)
        .map(|at| login(&format!("e{at}"), &format!("Entry {at:02}"), 1_000 + at))
        .collect();
    records[3]["favourite"] = json!(true);
    records[1]["favourite"] = json!(true);
    records[1]["title"] = json!("banking");
    records[3]["title"] = json!("Alpha");
    let rows = told(&records, &mut Kept::default());
    let ordered = |at: usize| rows[at]["order"].as_i64();
    assert_eq!(ordered(3), Some(1024), "Alpha, a favourite, didn't lead");
    assert_eq!(
        ordered(1),
        Some(2048),
        "banking didn't follow Alpha by its letters"
    );
    assert_eq!(
        ordered(24),
        Some(3072),
        "the latest changed didn't follow the favourites"
    );
    assert_eq!(
        field(&rows, "listed")
            .iter()
            .filter(|listed| **listed == true)
            .count(),
        22
    );
    assert_eq!(
        rows[0]["listed"], false,
        "one changed earliest was listed past twenty"
    );
    assert_eq!(
        (ordered(0), ordered(2)),
        (Some(23 * 1024), Some(24 * 1024)),
        "those found only by typing didn't follow by title"
    );
    assert!(
        rows.iter().all(|row| row["order"].is_i64()),
        "a row had no order: {rows:?}"
    );
}

#[test]
fn a_row_that_moves_alone_is_the_one_row_told_and_the_rest_keep_their_order() {
    let mut records = vec![
        login("a", "Alpha", 3),
        login("b", "Bravo", 2),
        login("c", "Charlie", 1),
    ];
    let mut kept = Kept::default();
    let first = told(&records, &mut kept);
    assert_eq!(
        field(&first, "order"),
        [&json!(1024), &json!(2048), &json!(3072)]
    );
    records[2]["changed"] = json!(4);
    let moved = kept.take(rows(&records, &one_manager(), &kept), Vec::new());
    let keys: Vec<&Value> = moved.changed.iter().map(|record| &record["key"]).collect();
    assert_eq!(
        keys,
        [&json!("row:passwords:entry:onepassword:c")],
        "more than the row that moved was told: {moved:?}"
    );
    assert_eq!(moved.changed[0]["row"]["order"], 0);
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
    let mut kept = Kept::default();
    let rows = told(&records, &mut kept);
    assert_eq!(
        field(&rows, "attention"),
        [&json!(2048), &Value::Null, &json!(1024)]
    );
    let health = |at: usize| drawer_of(&rows[at], &records, &one_manager())["facts"][0].clone();
    assert_eq!(
        health(0),
        json!({"name": "Health", "said": "reused in 2 other entries", "hue": "warning"})
    );
    assert_eq!(health(2)["said"], "seen in 1,204 breaches");
    // Mike found troubled too comes between them, and they keep theirs.
    records[1]["health"] = json!({"reused": 1, "breached": 0});
    let again = told(&records, &mut kept);
    assert_eq!(
        field(&again, "attention"),
        [&json!(2048), &json!(1536), &json!(1024)],
        "those needing attention already moved for one more"
    );
}

#[test]
fn its_acts_come_in_the_design_order_each_with_what_it_asks() {
    let mut entry = login("a", "Example", 1);
    entry["details"] = json!([
        {"id": "pw", "purpose": "password", "hidden": true},
        {"id": "otp", "type": "code", "hidden": true}
    ]);
    let rows = made(&[entry]);
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
fn a_note_with_nothing_to_copy_or_open_reads_its_details_on_return() {
    let note = json!({"key": "entry:onepassword:n", "manager": "onepassword", "id": "n",
        "title": "", "category": "note"});
    let rows = made(&[note]);
    assert_eq!(rows[0]["passworded"], false);
    assert_eq!(rows[0]["name"], "Note", "a note without a title read blank");
    assert_eq!(rows[0]["glyph"], "secureNote");
    assert_eq!(
        rows[0]["acts"],
        json!([{"key": "verb:details", "step": "drawer"}]),
        "a row with nothing to copy or open had no Return"
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
    entry["own"]["files"] = json!(1_200);
    entry["details"] = json!([
        {"id": "pw", "purpose": "password", "hidden": true, "strength": "FAIR"},
        {"id": "notes", "purpose": "notes", "value": "Ask Ada."},
        {"id": "pin", "label": "pin", "section": "Extra", "hidden": true, "value": "s3cret"},
        {"id": "plan", "label": "plan", "value": "gold"}
    ]);
    let records = [entry];
    let rows = made(&records);
    assert_eq!(
        drawer_of(&rows[0], &records, &one_manager()),
        json!({
            "eyebrow": "in Keeping \u{b7} login", "title": "Example",
            "facts": [
                {"name": "Health", "said": "unsecured website", "hue": "warning"},
                {"name": "Strength", "said": "fair", "hue": "warning"},
                {"name": "One-Time Code", "said": "none"},
                {"name": "Last Changed", "at": 5_000.0, "as": "ago"},
                {"name": "Websites",
                 "said": "https://example.com/login, http://old.example.com"},
                {"name": "Tags", "said": "work, web"},
                {"name": "Created", "at": 1_000.0, "as": "ago"},
                {"name": "Files", "said": "1,200"},
                {"name": "Plan", "said": "gold"},
                {"heading": "Extra"},
                {"name": "Pin", "said": "hidden"}
            ],
            "body": "Ask Ada."
        })
    );
}

#[test]
fn a_strength_is_said_in_the_launchers_words() {
    let mut vocabulary = vocabulary();
    let things = vocabulary["things"].as_array_mut().unwrap();
    let place = things
        .iter_mut()
        .find(|thing| thing["id"] == "passwords")
        .unwrap();
    place["said"]["veryGood"] = json!("ever so good");
    let words = Words::read(&vocabulary).unwrap();
    let mut entry = login("a", "Example", 1);
    entry["details"] =
        json!([{"id": "pw", "purpose": "password", "hidden": true, "strength": "very_good"}]);
    let records = [entry];
    let (_, drawer) = drawers(&records, &one_manager(), &words).remove(0);
    assert_eq!(
        drawer["facts"][0],
        json!({"name": "Strength", "said": "ever so good"}),
        "a strength wasn't said in the vocabulary's word for it"
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
    let records = [login("a", "Example", 1)];
    let rows = rows(&records, &managers, &Kept::default());
    assert_eq!(rows[0]["note"], "1Password \u{b7} example.com");
    let facts = drawer_of(&rows[0], &records, &managers)["facts"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        facts.contains(&json!({"name": "Password Manager", "said": "1Password"})),
        "the drawer didn't name its manager among two: {facts:?}"
    );
}
