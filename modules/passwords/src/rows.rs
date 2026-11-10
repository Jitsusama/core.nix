//! Passwords' rows, ready for the launcher: each entry held as the row
//! the Passwords place shows for it, worded, ordered and with its acts,
//! so the shell makes no rows of its own (Joel, 2026-11-10: "the daemons
//! need to be the data layer, and they need to serve the launcher").
//! Each is a `row:<entry key>` record whose `row` is the entry's own key,
//! as every surface keys it. `order` places those listed with nothing
//! typed, the favourites by title then the twenty changed latest, and
//! `attention` those the last check found breached or reused, by title;
//! an entry with neither is found by typing alone. Times travel as `{at,
//! as}` for the shell to word against its own clock, so no row changes
//! each minute. Passwords.js makes the same rows today; these take its
//! place as the shell reads them. No secret is ever in one: an entry is
//! held without its secrets (lib.rs's `Entry`), and a copy is an ask.

use std::cmp::Ordering;
use std::collections::HashMap;

use serde::Deserialize;
use serde_json::{Map, Value, json};

/// How many entries changed lately are listed after the favourites.
const RECENT: usize = 20;

/// The kinds whose Return copies a password before they're read in full.
const PASSWORDED: [&str; 2] = ["login", "password"];

/// Each kind's mark, by the kind's word as the managers say it, as
/// Passwords.js's MARKS; a kind not named wears `ANY_KIND`'s.
const MARKS: [(&str, &str); 22] = [
    ("login", "login"),
    ("password", "storedPassword"),
    ("card", "paymentCard"),
    ("note", "secureNote"),
    ("identity", "identityCard"),
    ("key", "sshKey"),
    ("credential", "apiCredential"),
    ("database", "database"),
    ("server", "server"),
    ("wireless router", "router"),
    ("bank account", "bankAccount"),
    ("email account", "emailAccount"),
    ("driver's licence", "driversLicence"),
    ("outdoor licence", "outdoorLicence"),
    ("software licence", "softwareLicence"),
    ("membership", "membership"),
    ("passport", "passport"),
    ("reward program", "rewardProgram"),
    ("social security number", "socialSecurityNumber"),
    ("medical record", "medicalRecord"),
    ("crypto wallet", "cryptoWallet"),
    ("document", "keptFile"),
];
const ANY_KIND: &str = "entry";

/// The facts a drawer names, each `passwords:<id>` in the vocabulary.
const FACTS: [&str; 9] = [
    "health", "strength", "code", "changed", "websites", "tags", "created", "manager", "files",
];

/// The place's own words a row is said in.
const SAID: [&str; 9] = [
    "hidden",
    "set",
    "none",
    "unsecured",
    "reused",
    "reusedIn",
    "breached",
    "breaches",
    "within",
];

/// 1Password's ratings of a password too weak to trust.
const WEAK: [&str; 3] = ["terrible", "weak", "fair"];

/// A password manager as rows name it: its name in the daemon, its name as
/// Joel reads it, and the pattern of its own link to an entry.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Shown {
    pub name: String,
    #[serde(default)]
    pub called: String,
    #[serde(default)]
    pub shows: String,
}

/// The words rows are said in, from the launcher's vocabulary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Words {
    /// Each fact's name, by its id and its state ("" for none).
    names: HashMap<(String, String), String>,
    /// The place's own words.
    said: HashMap<String, String>,
}

impl Words {
    /// Passwords' words from the whole vocabulary, or the first it lacks.
    pub fn read(vocabulary: &Value) -> Result<Words, String> {
        let things = vocabulary["things"]
            .as_array()
            .ok_or("the vocabulary has no things")?;
        let thing = |id: &str| things.iter().find(|thing| thing["id"] == id);
        let mut words = Words::default();
        for fact in FACTS {
            let id = format!("passwords:{fact}");
            let named = thing(&id).ok_or_else(|| format!("the vocabulary has no {id}"))?;
            let name = named["name"]
                .as_str()
                .ok_or_else(|| format!("{id} has no name"))?;
            words
                .names
                .insert((fact.to_owned(), String::new()), name.to_owned());
            for (state, held) in named["states"].as_object().into_iter().flatten() {
                if let Some(name) = held["name"].as_str() {
                    words
                        .names
                        .insert((fact.to_owned(), state.clone()), name.to_owned());
                }
            }
        }
        let said = thing("passwords")
            .and_then(|place| place["said"].as_object())
            .ok_or("the vocabulary's passwords says nothing")?;
        for word in SAID {
            let text = said[word]
                .as_str()
                .ok_or_else(|| format!("the vocabulary's passwords has no {word}"))?;
            words.said.insert(word.to_owned(), text.to_owned());
        }
        Ok(words)
    }

    fn name(&self, fact: &str, state: &str) -> String {
        self.names
            .get(&(fact.to_owned(), state.to_owned()))
            .or_else(|| self.names.get(&(fact.to_owned(), String::new())))
            .cloned()
            .unwrap_or_default()
    }

    fn said(&self, word: &str) -> &str {
        self.said.get(word).map_or("", String::as_str)
    }
}

/// Every entry's row, in the order held, from the records the daemon
/// serves (lib.rs's `Held::records`).
pub fn rows(records: &[Value], managers: &[Shown], words: &Words) -> Vec<Value> {
    let listed = places(&listed(records));
    let needing = places(&needing(records));
    let naming = managers.len() >= 2;
    records
        .iter()
        .map(|record| {
            let key = text(record, "key");
            let manager = managers
                .iter()
                .find(|manager| manager.name == text(record, "manager"));
            row(
                record,
                &Placed {
                    order: listed.get(key).copied(),
                    attention: needing.get(key).copied(),
                    called: manager
                        .filter(|_| naming)
                        .map_or("", |manager| &manager.called),
                    shows: manager.map_or("", |manager| &manager.shows),
                },
                words,
            )
        })
        .collect()
}

/// Where a row stands among the rest, and what its manager gives it.
struct Placed<'a> {
    order: Option<usize>,
    attention: Option<usize>,
    called: &'a str,
    shows: &'a str,
}

fn row(record: &Value, placed: &Placed, words: &Words) -> Value {
    let key = text(record, "key");
    let category = text(record, "category");
    let username = username(record);
    let link = shows(record, placed.shows);
    let address = website(record);
    let note = [placed.called, &host(&address)]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" \u{b7} ");
    let side = if username.is_empty() {
        json!([])
    } else {
        json!([{"said": username}])
    };
    json!({
        "key": format!("row:{key}"),
        "row": key,
        "place": "passwords",
        "kind": category,
        "order": placed.order,
        "attention": placed.attention,
        "name": title_of(record),
        "glyph": mark(category),
        "hue": "machine",
        "depth": 0,
        "side": side,
        "note": note,
        "favourite": record["favourite"] == true,
        "words": found_by(record),
        "passworded": passworded(record),
        "acts": acts(record, &link),
        "drawer": drawer(record, placed.called, words),
        "held": {"manager": text(record, "manager"), "id": text(record, "id"),
                 "website": address, "fills": browsed(&address)},
    })
}

fn text<'a>(record: &'a Value, field: &str) -> &'a str {
    record[field].as_str().unwrap_or_default()
}

fn title(record: &Value) -> &str {
    text(record, "title")
}

/// Each key's place in a list.
fn places(listed: &[&Value]) -> HashMap<String, usize> {
    listed
        .iter()
        .enumerate()
        .map(|(at, record)| (text(record, "key").to_owned(), at))
        .collect()
}

/// Titles in the order a person reads them: by letter, whatever the case.
fn by_title(one: &Value, other: &Value) -> Ordering {
    let (one, other) = (title(one), title(other));
    one.to_lowercase()
        .cmp(&other.to_lowercase())
        .then_with(|| one.cmp(other))
}

/// Those listed with nothing typed: the favourites by title, then the rest
/// by when each last changed, newest first, at most `RECENT`.
fn listed(records: &[Value]) -> Vec<&Value> {
    let mut favourites: Vec<&Value> = records
        .iter()
        .filter(|record| record["favourite"] == true)
        .collect();
    favourites.sort_by(|one, other| by_title(one, other));
    let mut lately: Vec<&Value> = records
        .iter()
        .filter(|record| record["favourite"] != true && record["changed"].is_number())
        .collect();
    let changed = |record: &Value| record["changed"].as_f64().unwrap_or_default();
    lately.sort_by(|one, other| {
        changed(other)
            .total_cmp(&changed(one))
            .then_with(|| by_title(one, other))
    });
    lately.truncate(RECENT);
    favourites.extend(lately);
    favourites
}

/// Its health as the last check found it, as (reused, breached).
fn health_of(record: &Value) -> Option<(u64, u64)> {
    let found = record["health"].as_object()?;
    let count = |field: &str| found.get(field).and_then(Value::as_u64).unwrap_or_default();
    Some((count("reused"), count("breached")))
}

/// Those the last check found shared or breached, by title.
fn needing(records: &[Value]) -> Vec<&Value> {
    let mut troubled: Vec<&Value> = records
        .iter()
        .filter(|record| health_of(record).is_some_and(|(reused, breached)| reused + breached > 0))
        .collect();
    troubled.sort_by(|one, other| by_title(one, other));
    troubled
}

fn mark(category: &str) -> &'static str {
    MARKS
        .iter()
        .find(|(kind, _)| *kind == category)
        .map_or(ANY_KIND, |(_, mark)| mark)
}

fn details(record: &Value) -> Option<&Vec<Value>> {
    record["details"].as_array()
}

fn detail<'a>(record: &'a Value, field: &str, is: &str) -> Option<&'a Map<String, Value>> {
    details(record)?
        .iter()
        .filter_map(Value::as_object)
        .find(|detail| detail.get(field).and_then(Value::as_str) == Some(is))
}

/// Its website: its primary one, else its first, or "".
fn website(record: &Value) -> String {
    websites(record).into_iter().next().unwrap_or_default()
}

/// Every website it has, the primary first.
fn websites(record: &Value) -> Vec<String> {
    let given: Vec<&Value> = record["websites"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|one| !text(one, "address").is_empty())
        .collect();
    let primary = given
        .iter()
        .position(|one| one["primary"] == true)
        .unwrap_or_default();
    let mut all: Vec<String> = Vec::new();
    if let Some(first) = given.get(primary) {
        all.push(text(first, "address").to_owned());
    }
    for one in &given {
        let address = text(one, "address");
        if !all.iter().any(|held| held == address) {
            all.push(address.to_owned());
        }
    }
    all
}

/// Whether a browser opens it, so its manager's extension can fill it.
fn browsed(address: &str) -> bool {
    let lower = address.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// An address's host: `github.com` of `https://github.com/login`.
fn host(address: &str) -> String {
    let rest = address.split_once("://").map_or(address, |(_, rest)| rest);
    rest.split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .to_owned()
}

/// Its username: the one it was listed with, else the detail whose
/// purpose is one, or "".
fn username(record: &Value) -> String {
    let given = text(record, "username");
    if !given.is_empty() {
        return given.to_owned();
    }
    detail(record, "purpose", "username")
        .filter(|one| one.get("hidden") != Some(&Value::Bool(true)))
        .and_then(|one| one.get("value"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// A word as a name, its first letter a capital.
fn labelled(label: &str) -> String {
    let mut letters = label.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(letters).collect()
    })
}

/// What it's called: its title, else where it's for, else whose it is,
/// else its kind as a name, so no row reads blank.
fn title_of(record: &Value) -> String {
    let title = title(record).trim();
    if !title.is_empty() {
        return title.to_owned();
    }
    let host = host(&website(record));
    if !host.is_empty() {
        return host;
    }
    let username = username(record);
    if !username.is_empty() {
        return username;
    }
    let category = text(record, "category");
    labelled(if category.is_empty() {
        text(record, "id")
    } else {
        category
    })
}

/// The words besides its title it's found by: its username, every
/// website's address and host, and its tags.
fn found_by(record: &Value) -> Vec<String> {
    let addresses = websites(record);
    let mut words = vec![username(record)];
    words.extend(addresses.iter().cloned());
    words.extend(addresses.iter().map(|address| host(address)));
    for tag in record["tags"].as_array().into_iter().flatten() {
        words.push(tag.as_str().unwrap_or_default().to_owned());
    }
    words.retain(|word| !word.is_empty());
    words
}

/// Whether Return copies its password: by its kind until it's read in
/// full, then by its details.
fn passworded(record: &Value) -> bool {
    if details(record).is_some() {
        return detail(record, "purpose", "password").is_some();
    }
    PASSWORDED.contains(&text(record, "category"))
}

/// Its link in its manager's own program, from the manager's pattern, or
/// "" where it has none or the entry doesn't say a part of it.
fn shows(record: &Value, pattern: &str) -> String {
    if pattern.is_empty() {
        return String::new();
    }
    let parts = [
        ("{account}", record["own"]["account"].as_str()),
        ("{collection}", record["collection"]["id"].as_str()),
        ("{id}", record["id"].as_str()),
    ];
    let mut link = pattern.to_owned();
    for (part, value) in parts {
        if link.contains(part) {
            match value {
                Some(value) if !value.is_empty() => link = link.replace(part, &encoded(value)),
                _ => return String::new(),
            }
        }
    }
    link
}

/// A link's part as JavaScript's encodeURIComponent writes it.
fn encoded(text: &str) -> String {
    text.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

/// The acts inside it, in the one order the design gives them, each with
/// what it asks: a secret is copied by the daemon, the username and the
/// link through the clipboard's daemon, and the website and the entry
/// shown in its manager opened.
fn acts(record: &Value, link: &str) -> Vec<Value> {
    let key = text(record, "key");
    let copies = |detail: Option<&str>| json!({"ask": "copy", "key": key, "detail": detail});
    let verb = |id: &str| format!("verb:passwords:{id}");
    let mut listed = Vec::new();
    if passworded(record) {
        let id = detail(record, "purpose", "password")
            .and_then(|one| one.get("id"))
            .and_then(Value::as_str);
        listed.push(json!({"key": verb("password"), "ask": copies(id)}));
    }
    if let Some(code) = detail(record, "type", "code") {
        let id = code.get("id").and_then(Value::as_str);
        listed.push(json!({"key": verb("code"), "ask": copies(id)}));
    }
    let username = username(record);
    if !username.is_empty() {
        listed.push(json!({"key": verb("username"), "ask": {"copy": username}}));
    }
    let address = website(record);
    if !address.is_empty() {
        listed.push(json!({"key": verb("website"),
            "ask": {"open": {"link": address, "fills": browsed(&address)}}}));
    }
    if !link.is_empty() {
        listed.push(json!({"key": verb("show"), "ask": {"open": {"link": link}}}));
        listed.push(json!({"key": verb("link"), "ask": {"copy": link}}));
    }
    listed
}

/// A count with its thousands grouped, as the launcher's numbers are.
fn grouped(count: u64) -> String {
    let digits = count.to_string();
    let mut out = String::new();
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// What's wrong with it that its Strength doesn't say, or "".
fn health(record: &Value, words: &Words) -> String {
    let (reused, breached) = health_of(record).unwrap_or_default();
    let counted = |one: &str, many: &str, count: u64| {
        if count == 1 {
            words.said(one).to_owned()
        } else {
            words.said(many).replace("{count}", &grouped(count))
        }
    };
    let mut wrong = Vec::new();
    if reused > 0 {
        wrong.push(counted("reused", "reusedIn", reused));
    }
    if breached > 0 {
        wrong.push(counted("breached", "breaches", breached));
    }
    if websites(record)
        .iter()
        .any(|address| address.to_ascii_lowercase().starts_with("http://"))
    {
        wrong.push(words.said("unsecured").to_owned());
    }
    wrong.retain(|part| !part.is_empty());
    wrong.join(", ")
}

/// Its drawer: every figure its manager gives but the secret, in the
/// design's order, as Passwords.js's details; its times as `{at, as}`.
fn drawer(record: &Value, manager: &str, words: &Words) -> Value {
    let mut facts = Vec::new();
    let mut fact = |id: &str, state: &str, value: String, warns: bool| {
        if !value.is_empty() {
            let mut one = json!({"label": words.name(id, state), "value": value});
            if warns {
                one["warns"] = json!(true);
            }
            facts.push(one);
        }
    };
    fact("health", "", health(record, words), true);
    let read = details(record).is_some();
    if read {
        let rated = detail(record, "purpose", "password")
            .and_then(|one| one.get("strength"))
            .and_then(Value::as_str)
            .or_else(|| {
                details(record)?
                    .iter()
                    .filter_map(|one| one["strength"].as_str())
                    .find(|strength| !strength.is_empty())
            })
            .unwrap_or_default()
            .to_lowercase()
            .replace('_', " ");
        let weak = WEAK.contains(&rated.as_str());
        fact("strength", "", rated, weak);
        let code = detail(record, "type", "code").is_some();
        fact(
            "code",
            "",
            words.said(if code { "set" } else { "none" }).to_owned(),
            false,
        );
    }
    if let Some(at) = record["changed"].as_f64() {
        facts.push(json!({"label": words.name("changed", ""), "at": at, "as": "ago"}));
    }
    let addresses = websites(record);
    if !addresses.is_empty() {
        let state = if addresses.len() > 1 { "several" } else { "" };
        facts.push(json!({"label": words.name("websites", state),
                          "value": addresses.join(", ")}));
    }
    drawer_rest(record, manager, words, facts, read)
}

/// The drawer's facts after its websites, its eyebrow, title and words.
fn drawer_rest(
    record: &Value,
    manager: &str,
    words: &Words,
    mut facts: Vec<Value>,
    read: bool,
) -> Value {
    let tags: Vec<&str> = record["tags"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if !tags.is_empty() {
        facts.push(json!({"label": words.name("tags", ""), "value": tags.join(", ")}));
    }
    if let Some(at) = record["created"].as_f64() {
        facts.push(json!({"label": words.name("created", ""), "at": at, "as": "ago"}));
    }
    if !manager.is_empty() {
        facts.push(json!({"label": words.name("manager", ""), "value": manager}));
    }
    if let Some(files) = record["own"]["files"].as_u64().filter(|files| *files > 0) {
        facts.push(json!({"label": words.name("files", ""), "value": files.to_string()}));
    }
    if read {
        facts.extend(detail_facts(record, words.said("hidden")));
    }
    let kind = text(record, "category");
    let vault = record["collection"]["name"].as_str().unwrap_or_default();
    let eyebrow = if vault.is_empty() {
        kind.to_owned()
    } else {
        words
            .said("within")
            .replace("{vault}", vault)
            .replace("{kind}", kind)
    };
    let body = detail(record, "purpose", "notes")
        .filter(|notes| notes.get("hidden") != Some(&Value::Bool(true)))
        .and_then(|notes| notes.get("value"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    json!({"eyebrow": eyebrow, "eyebrowNames": true, "title": title(record), "facts": facts,
           "body": body})
}

/// Each detail read in full that isn't said elsewhere, those in no
/// section first, then each section under its name as a heading; a hidden
/// one says `hidden`.
fn detail_facts(record: &Value, hidden: &str) -> Vec<Value> {
    let elsewhere = |one: &Value| {
        one["empty"] == true
            || ["username", "password", "notes"].contains(&text(one, "purpose"))
            || text(one, "type") == "code"
    };
    let shown: Vec<&Value> = details(record)
        .into_iter()
        .flatten()
        .filter(|one| !elsewhere(one))
        .collect();
    let mut sections: Vec<&str> = Vec::new();
    for one in &shown {
        let section = text(one, "section");
        if !sections.contains(&section) {
            sections.push(section);
        }
    }
    sections.sort_by_key(|section| !section.is_empty());
    let mut out = Vec::new();
    for section in sections {
        if !section.is_empty() {
            out.push(json!({"heading": section}));
        }
        for one in shown.iter().filter(|one| text(one, "section") == section) {
            let label = labelled(one["label"].as_str().unwrap_or_else(|| text(one, "id")));
            let value = if one["hidden"] == true {
                hidden.to_owned()
            } else {
                text(one, "value").to_owned()
            };
            out.push(json!({"label": label, "value": value}));
        }
    }
    out
}

#[cfg(test)]
#[path = "rows_tests.rs"]
mod tests;
