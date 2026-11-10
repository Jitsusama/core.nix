//! Mail's rows, ready for the launcher: each piece of mail that wants
//! Joel as the row Mail shows it, worded, ordered and with its acts, so
//! the shell lays it out and words nothing but the times (Joel,
//! 2026-11-10: "the daemons need to be the data layer, and they need to
//! serve the launcher"). The shape is the quest's google-rows.md: a
//! `row:mail:<id>` record whose `row` is the launcher's own key for it,
//! `mail:<id>`, and whose times travel as `{at, as}` for the shell to
//! word against its own clock, since a row worded here would change
//! every minute. A row's drawer, the mail's body with it, is a record of
//! its own, `drawer:mail:<id>`, which the row names and the shell watches
//! by key only while it shows. Mail.qml and Mail.js make the same row
//! today; these take their place as the shell reads them.

use std::fmt::Write;

use serde_json::{Value, json};

use crate::document_rows;
use crate::mail::Mail;

/// How many people a sent mail's row names before it counts the rest.
const NAMED: usize = 2;

/// Gmail, where a new message is written in a window of its own.
const COMPOSE: &str = "https://mail.google.com/mail/u/0/";

/// The words mail's rows are said in, from the launcher's vocabulary
/// (launcher/words.json), which the daemon is handed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Words {
    /// A row's name for mail with no subject.
    pub untitled: String,
    /// The glyph role every mail row shows.
    pub glyph: String,
    /// "to {who}", for mail Joel sent.
    pub to: String,
    /// "{one} and {other}".
    pub to_two: String,
    /// "{who} and {count} more".
    pub to_more: String,
    /// "{count} people".
    pub to_people: String,
}

/// Every place's words, from the launcher's vocabulary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Vocabulary {
    pub mail: Words,
    pub documents: document_rows::Words,
}

impl Vocabulary {
    /// Each place's words, or the first word one lacks.
    pub fn read(vocabulary: &Value) -> Result<Vocabulary, String> {
        Ok(Vocabulary {
            mail: Words::read(vocabulary)?,
            documents: document_rows::Words::read(vocabulary)?,
        })
    }
}

impl Words {
    /// Mail's words from the whole vocabulary, or the first one it lacks,
    /// so a vocabulary that lost one is said rather than shown as blanks.
    pub fn read(vocabulary: &Value) -> Result<Words, String> {
        let mail = vocabulary["things"]
            .as_array()
            .and_then(|things| things.iter().find(|thing| thing["id"] == "mail:message"))
            .ok_or("the vocabulary has no mail:message")?;
        let word = |at: &[&str]| -> Result<String, String> {
            at.iter()
                .fold(mail, |within, field| &within[field])
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("the vocabulary's mail:message has no {}", at.join(".")))
        };
        Ok(Words {
            untitled: word(&["states", "untitled", "name"])?,
            glyph: word(&["glyph"])?,
            to: word(&["said", "to"])?,
            to_two: word(&["said", "toTwo"])?,
            to_more: word(&["said", "toMore"])?,
            to_people: word(&["said", "toPeople"])?,
        })
    }
}

/// Each piece of mail that still wants Joel, as its row, newest first;
/// mail he dealt with is no row, since Mail shows it no more.
pub fn mail(held: &[Mail], words: &Words) -> Vec<Value> {
    let mut wanting: Vec<&Mail> = held.iter().filter(|mail| mail.done.is_none()).collect();
    wanting.sort_by(|one, other| other.at.cmp(&one.at).then_with(|| one.id.cmp(&other.id)));
    wanting
        .iter()
        .enumerate()
        .map(|(order, mail)| row(mail, order, words))
        .collect()
}

/// Each wanting piece's drawer, by the key its row's drawer is watched by.
pub fn mail_drawers(held: &[Mail], words: &Words) -> Vec<(String, Value)> {
    held.iter()
        .filter(|mail| mail.done.is_none())
        .map(|mail| {
            let key = format!("drawer:mail:{}", mail.id);
            (key, drawer(mail, titled(mail, words)))
        })
        .collect()
}

fn titled<'a>(mail: &'a Mail, words: &'a Words) -> &'a str {
    if mail.subject.is_empty() {
        &words.untitled
    } else {
        &mail.subject
    }
}

fn row(mail: &Mail, order: usize, words: &Words) -> Value {
    json!({
        "key": format!("row:mail:{}", mail.id),
        "row": format!("mail:{}", mail.id),
        "place": "mail",
        "kind": "mail",
        "order": order,
        "name": titled(mail, words),
        "glyph": words.glyph,
        "hue": "program",
        "depth": 0,
        "side": beside(mail, words),
        "drawer": format!("drawer:mail:{}", mail.id),
        "acts": acts(mail),
        "held": {
            "page": mail.page, "address": mail.address, "sent": mail.sent,
            "attachments": mail.attachments,
        },
    })
}

/// Who it's from, or for mail Joel sent whom it went to, then when it
/// came, as Mail.js's besideItems lays them: neither gives way whole, and
/// the time never gives way at all.
fn beside(mail: &Mail, words: &Words) -> Vec<Value> {
    let when = json!({"at": mail.at, "as": "came", "keep": 2, "stays": true});
    let ways = if mail.sent {
        went_ways(&mail.went, words)
    } else {
        vec![mail.from.clone()]
    };
    match ways.first() {
        Some(first) if !first.is_empty() => {
            vec![
                json!({"said": first, "ways": ways, "keep": 1, "stays": true}),
                when,
            ]
        }
        _ => vec![when],
    }
}

/// Every way a sent mail's row may say whom it went to, most names first,
/// or nothing for nobody named.
fn went_ways(went: &[String], words: &Words) -> Vec<String> {
    let to = |who: &str| words.to.replace("{who}", who);
    match went {
        [] => vec![String::new()],
        [only] => vec![to(only)],
        _ => vec![
            to(&went_to(went, words, NAMED)),
            to(&went_to(went, words, 1)),
            to(&words.to_people.replace("{count}", &went.len().to_string())),
        ],
    }
}

/// The first `most` named, and how many more past them.
fn went_to(went: &[String], words: &Words, most: usize) -> String {
    let named = &went[..most.min(went.len())];
    if went.len() > most {
        return words
            .to_more
            .replace("{who}", &named.join(", "))
            .replace("{count}", &(went.len() - most).to_string());
    }
    match named {
        [one, other] => words.to_two.replace("{one}", one).replace("{other}", other),
        _ => named.join(""),
    }
}

/// Its drawer: its subject, who it's from and to, when it came, the files
/// it carries by name, and what it says, each plain text.
fn drawer(mail: &Mail, title: &str) -> Value {
    let files: Vec<&str> = mail
        .attachments
        .iter()
        .map(|file| file.name.as_str())
        .collect();
    let carried = if files.len() == 1 {
        "Attachment"
    } else {
        "Attachments"
    };
    let mut facts = Vec::new();
    for (label, value) in [
        ("From", sender(mail)),
        ("To", mail.to.clone()),
        ("Cc", mail.cc.clone()),
    ] {
        if !value.is_empty() {
            facts.push(json!({"label": label, "value": value}));
        }
    }
    facts.push(json!({"label": "Date", "at": mail.at, "as": "dated"}));
    if !files.is_empty() {
        facts.push(json!({"label": carried, "value": files.join(", ")}));
    }
    json!({"title": title, "facts": facts, "body": mail.body})
}

/// Who sent it, by name and address as mail writes them, or whichever of
/// the two it gives.
fn sender(mail: &Mail) -> String {
    if mail.address.is_empty() || mail.address == mail.from {
        mail.from.clone()
    } else if mail.from.is_empty() {
        mail.address.clone()
    } else {
        format!("{} <{}>", mail.from, mail.address)
    }
}

/// Its acts, in the one order every surface lists them, each with what it
/// asks of this daemon, or the step the launcher takes for it: its sender
/// is People's row for the address (`step: person`), since the launcher
/// holds People, and Attachments steps in to its files (`step:
/// attachments`), which `held.attachments` gives until they're rows of
/// their own.
fn acts(mail: &Mail) -> Vec<Value> {
    let opens = |key: &str, link: &str| json!({"key": key, "ask": {"open": {"link": link}}});
    let mut listed = vec![opens("verb:mail:open", &mail.page)];
    let subject = answering(&mail.subject);
    let reply = writing(&[&mail.address], &subject, &[]);
    if !reply.is_empty() {
        listed.push(opens("verb:mail:reply", &reply));
    }
    if !mail.others.is_empty() {
        let others: Vec<&str> = mail.others.iter().map(String::as_str).collect();
        let all = writing(&[&mail.address], &subject, &others);
        if !all.is_empty() {
            listed.push(opens("verb:mail:replyAll", &all));
        }
    }
    listed.push(json!({"key": "verb:mail:archive", "ask": {"archive": {"id": mail.id}}}));
    listed.push(json!({"key": "verb:mail:read", "ask": {"read": {"id": mail.id}}}));
    if !mail.address.is_empty() {
        listed.push(json!({"key": "mail:sender", "step": "person",
                           "address": mail.address}));
    }
    if !mail.attachments.is_empty() {
        listed.push(
            json!({"key": "verb:mail:attachments", "step": "attachments",
                           "count": mail.attachments.len()}),
        );
    }
    listed.push(json!({"key": "verb:mail:link", "ask": {"copy": mail.page}}));
    listed
}

/// A reply's subject, marked a reply once, as mail programs write it.
fn answering(subject: &str) -> String {
    let replied = subject
        .get(..3)
        .is_some_and(|start| start.eq_ignore_ascii_case("re:"));
    if replied {
        subject.to_owned()
    } else {
        format!("Re: {subject}")
    }
}

/// Where a new message to these addresses is written, about this subject
/// and copied to those, as Gmail's own link to compose takes them, each
/// address once; nothing with nobody to write to.
fn writing(to: &[&str], subject: &str, copied: &[&str]) -> String {
    let mut addresses: Vec<&str> = Vec::new();
    for address in to {
        if !address.is_empty() && !addresses.contains(address) {
            addresses.push(address);
        }
    }
    if addresses.is_empty() {
        return String::new();
    }
    let mut asked = vec![
        ("view", "cm".to_owned()),
        ("fs", "1".to_owned()),
        ("to", addresses.join(",")),
    ];
    let mut cc: Vec<&str> = Vec::new();
    for address in copied {
        if !address.is_empty() && !addresses.contains(address) && !cc.contains(address) {
            cc.push(address);
        }
    }
    if !cc.is_empty() {
        asked.push(("cc", cc.join(",")));
    }
    if !subject.is_empty() {
        asked.push(("su", subject.to_owned()));
    }
    let query: Vec<String> = asked
        .iter()
        .map(|(name, value)| format!("{name}={}", encoded(value)))
        .collect();
    format!("{COMPOSE}?{}", query.join("&"))
}

/// A link's part as JavaScript's encodeURIComponent writes it, which
/// Mail.js's links were.
fn encoded(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            out.push(char::from(byte));
        } else {
            // Writing to a String never fails.
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::done::{Done, Went};
    use crate::mail::{Bytes, File};

    fn words() -> Words {
        Words {
            untitled: "Mail Without a Subject".into(),
            glyph: "mail".into(),
            to: "to {who}".into(),
            to_two: "{one} and {other}".into(),
            to_more: "{who} and {count} more".into(),
            to_people: "{count} people".into(),
        }
    }

    fn one(id: &str, at: i64) -> Mail {
        Mail {
            id: id.into(),
            from: "Ada Lovelace".into(),
            address: "ada@example.com".into(),
            to: "Joel".into(),
            cc: String::new(),
            others: Vec::new(),
            subject: "Engines".into(),
            sent: false,
            went: Vec::new(),
            at,
            page: format!("https://mail.google.com/mail/u/0/#inbox/{id}"),
            body: "On the engine.".into(),
            attachments: Vec::new(),
            done: None,
        }
    }

    #[test]
    fn mails_words_are_read_from_the_vocabulary_or_the_lack_is_said() {
        let vocabulary = json!({"things": [
            {"id": "agents", "name": "Agents"},
            {"id": "mail:message", "glyph": "mail",
             "states": {"untitled": {"name": "Mail Without a Subject"}},
             "said": {"to": "to {who}", "toTwo": "{one} and {other}",
                      "toMore": "{who} and {count} more", "toPeople": "{count} people"}}
        ]});
        assert_eq!(Words::read(&vocabulary), Ok(words()));
        let lacking = json!({"things": [{"id": "mail:message", "glyph": "mail"}]});
        assert!(
            Words::read(&lacking).is_err_and(|lack| lack.contains("untitled")),
            "a vocabulary without mail's untitled word read as whole"
        );
    }

    #[test]
    fn mail_is_a_row_keyed_as_the_launcher_keys_it_newest_first() {
        let rows = mail(
            &[one("a", 1_000), one("b", 3_000), one("c", 2_000)],
            &words(),
        );
        let keys: Vec<&str> = rows.iter().filter_map(|row| row["key"].as_str()).collect();
        assert_eq!(keys, ["row:mail:b", "row:mail:c", "row:mail:a"]);
        let first = &rows[0];
        assert_eq!(first["row"], "mail:b");
        assert_eq!(first["place"], "mail");
        assert_eq!(first["kind"], "mail");
        assert_eq!(first["order"], 0);
        assert_eq!(first["name"], "Engines");
        assert_eq!(first["glyph"], "mail");
        assert_eq!(first["hue"], "program");
        assert_eq!(first["depth"], 0);
        assert_eq!(rows[2]["order"], 2);
    }

    #[test]
    fn mail_dealt_with_is_no_row_and_no_subject_is_said_untitled() {
        let mut done = one("done", 5_000);
        done.done = Some(Done {
            how: Went::ReadHere,
            at: 5_000,
        });
        let mut bare = one("bare", 1_000);
        bare.subject = String::new();
        let both = [done, bare];
        let rows = mail(&both, &words());
        assert_eq!(rows.len(), 1, "mail dealt with was kept a row: {rows:?}");
        assert_eq!(rows[0]["name"], "Mail Without a Subject");
        let drawers = mail_drawers(&both, &words());
        assert_eq!(
            drawers.len(),
            1,
            "mail dealt with kept a drawer: {drawers:?}"
        );
        assert_eq!(drawers[0].1["title"], "Mail Without a Subject");
    }

    #[test]
    fn beside_it_is_who_sent_it_then_when_it_came_as_a_time_to_word() {
        let rows = mail(&[one("a", 1_000)], &words());
        assert_eq!(
            rows[0]["side"],
            json!([
                {"said": "Ada Lovelace", "ways": ["Ada Lovelace"], "keep": 1, "stays": true},
                {"at": 1_000, "as": "came", "keep": 2, "stays": true}
            ])
        );
        let mut nameless = one("b", 1_000);
        nameless.from = String::new();
        let rows = mail(&[nameless], &words());
        assert_eq!(
            rows[0]["side"],
            json!([{"at": 1_000, "as": "came", "keep": 2, "stays": true}])
        );
    }

    #[test]
    fn mail_joel_sent_names_who_it_went_to_giving_way_by_whole_names() {
        let mut sent = one("a", 1_000);
        sent.sent = true;
        sent.went = ["Ada", "Grace", "Alan", "Edsger", "Barbara"]
            .map(str::to_owned)
            .to_vec();
        let rows = mail(&[sent.clone()], &words());
        assert_eq!(
            rows[0]["side"][0]["ways"],
            json!([
                "to Ada, Grace and 3 more",
                "to Ada and 4 more",
                "to 5 people"
            ])
        );
        sent.went.truncate(2);
        let rows = mail(&[sent.clone()], &words());
        assert_eq!(
            rows[0]["side"][0]["ways"],
            json!(["to Ada and Grace", "to Ada and 1 more", "to 2 people"])
        );
        sent.went.truncate(1);
        let rows = mail(&[sent], &words());
        assert_eq!(rows[0]["side"][0]["ways"], json!(["to Ada"]));
    }

    #[test]
    fn its_drawer_says_whom_it_came_from_and_went_to_when_and_what_it_carries() {
        let mut carrying = one("a", 1_000);
        carrying.cc = "Grace Hopper".into();
        carrying.attachments = ["plan.pdf", "notes.txt"]
            .map(|name| File {
                name: name.into(),
                kind: "application/pdf".into(),
                size: 2_048,
                runs: false,
                bytes: Bytes::Asked("x".into()),
            })
            .to_vec();
        let rows = mail(std::slice::from_ref(&carrying), &words());
        assert_eq!(
            rows[0]["drawer"], "drawer:mail:a",
            "the row didn't name its drawer"
        );
        let drawers = mail_drawers(&[carrying], &words());
        assert_eq!(drawers[0].0, "drawer:mail:a");
        assert_eq!(
            drawers[0].1,
            json!({
                "title": "Engines",
                "facts": [
                    {"label": "From", "value": "Ada Lovelace <ada@example.com>"},
                    {"label": "To", "value": "Joel"},
                    {"label": "Cc", "value": "Grace Hopper"},
                    {"label": "Date", "at": 1_000, "as": "dated"},
                    {"label": "Attachments", "value": "plan.pdf, notes.txt"}
                ],
                "body": "On the engine."
            })
        );
        let acts = rows[0]["acts"].as_array().cloned().unwrap_or_default();
        assert_eq!(
            acts[acts.len() - 2],
            json!({"key": "verb:mail:attachments", "step": "attachments", "count": 2}),
            "Attachments wasn't offered just before Copy the Link"
        );
        assert_eq!(rows[0]["held"]["attachments"][1]["name"], "notes.txt");
    }

    #[test]
    fn its_acts_come_in_the_one_order_each_with_what_it_asks() {
        let mut all = one("a", 1_000);
        all.subject = "Re: Engines".into();
        all.others = vec!["grace@example.com".into()];
        let rows = mail(&[all], &words());
        let page = "https://mail.google.com/mail/u/0/#inbox/a";
        let reply = "https://mail.google.com/mail/u/0/?view=cm&fs=1&to=ada%40example.com\
                     &su=Re%3A%20Engines";
        let reply_all = "https://mail.google.com/mail/u/0/?view=cm&fs=1&to=ada%40example.com\
                         &cc=grace%40example.com&su=Re%3A%20Engines";
        assert_eq!(
            rows[0]["acts"],
            json!([
                {"key": "verb:mail:open", "ask": {"open": {"link": page}}},
                {"key": "verb:mail:reply", "ask": {"open": {"link": reply}}},
                {"key": "verb:mail:replyAll", "ask": {"open": {"link": reply_all}}},
                {"key": "verb:mail:archive", "ask": {"archive": {"id": "a"}}},
                {"key": "verb:mail:read", "ask": {"read": {"id": "a"}}},
                {"key": "mail:sender", "step": "person", "address": "ada@example.com"},
                {"key": "verb:mail:link", "ask": {"copy": page}}
            ])
        );
        let mut alone = one("b", 1_000);
        alone.address = String::new();
        let rows = mail(&[alone], &words());
        let keys: Vec<&str> = rows[0]["acts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|act| act["key"].as_str())
            .collect();
        assert_eq!(
            keys,
            [
                "verb:mail:open",
                "verb:mail:archive",
                "verb:mail:read",
                "verb:mail:link"
            ],
            "mail with no address to answer still offered a reply"
        );
    }

    #[test]
    fn what_a_client_may_need_of_googles_own_is_held_beside_it() {
        let rows = mail(&[one("a", 1_000)], &words());
        assert_eq!(
            rows[0]["held"],
            json!({"page": "https://mail.google.com/mail/u/0/#inbox/a",
                   "address": "ada@example.com", "sent": false, "attachments": []})
        );
    }
}
