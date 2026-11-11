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

use std::cmp::Reverse;

use serde_json::{Value, json};

use crate::calendar_rows;
use crate::compose::{answering, writing};
use crate::document_rows;
use crate::mail::Mail;

/// How many people a sent mail's row names before it counts the rest.
const NAMED: usize = 2;

/// How many Mail's Recent lists, as every Recent does.
const RECENT: usize = 9;

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
    /// "{name} <{address}>", who sent it when mail gives both.
    pub sender: String,
    /// Its drawer's facts' names: From, To, Cc, Date, and Attachment and
    /// Attachments.
    pub from: String,
    pub to_fact: String,
    pub cc: String,
    pub date: String,
    pub attached: String,
    pub attached_several: String,
}

/// Every place's words, from the launcher's vocabulary, each read apart,
/// so one place's lost word costs the others nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Vocabulary {
    pub mail: Option<Words>,
    pub documents: Option<document_rows::Words>,
    pub calendar: Option<calendar_rows::Words>,
}

impl Vocabulary {
    /// Each place's words, and for each place that lacks one, the place
    /// and the first word it lacks, for the daemon to say once.
    pub fn read(vocabulary: &Value) -> (Vocabulary, Vec<String>) {
        fn kept<T>(place: &str, read: Result<T, String>, lacking: &mut Vec<String>) -> Option<T> {
            read.map_err(|error| lacking.push(format!("{place}: {error}")))
                .ok()
        }
        let mut lacking = Vec::new();
        let read = Vocabulary {
            mail: kept("mail", Words::read(vocabulary), &mut lacking),
            documents: kept(
                "documents",
                document_rows::Words::read(vocabulary),
                &mut lacking,
            ),
            calendar: kept(
                "calendar",
                calendar_rows::Words::read(vocabulary),
                &mut lacking,
            ),
        };
        (read, lacking)
    }
}

#[cfg(test)]
#[path = "marked_words.rs"]
pub(crate) mod marked;

/// The desktop entry every Google page opens through (`google-opens`,
/// opens.sh), in a window of Google's own rather than a browser tab, so
/// the launcher opens a link only where it is (ready-rows.md's `in`).
const OPENS_IN: &str = "google-opens";

/// An act opening `link` in Google's own window, its words `key`'s.
pub(crate) fn opens(key: &str, link: &str) -> Value {
    json!({"key": key, "open": link, "in": OPENS_IN})
}

/// A form with each `{name}` in it filled.
pub(crate) fn fill(form: &str, with: &[(&str, &str)]) -> String {
    with.iter().fold(form.to_owned(), |said, (name, value)| {
        said.replace(&format!("{{{name}}}"), value)
    })
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
        let fact = |fact: &str, at: &[&str]| -> Result<String, String> {
            let id = format!("mail:fact:{fact}");
            let thing = vocabulary["things"]
                .as_array()
                .and_then(|things| things.iter().find(|thing| thing["id"] == id.as_str()))
                .ok_or_else(|| format!("the vocabulary has no {id}"))?;
            at.iter()
                .fold(thing, |within, field| &within[field])
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("the vocabulary's {id} has no {}", at.join(".")))
        };
        Ok(Words {
            untitled: word(&["states", "untitled", "name"])?,
            glyph: word(&["glyph"])?,
            to: word(&["said", "to"])?,
            to_two: word(&["said", "toTwo"])?,
            to_more: word(&["said", "toMore"])?,
            to_people: word(&["said", "toPeople"])?,
            sender: word(&["said", "sender"])?,
            from: fact("from", &["name"])?,
            to_fact: fact("to", &["name"])?,
            cc: fact("cc", &["name"])?,
            date: fact("date", &["name"])?,
            attached: fact("attached", &["name"])?,
            attached_several: fact("attached", &["states", "several", "name"])?,
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
        .map(|mail| row(mail, "mail", words))
        .collect()
}

/// The mail Joel had lately, as Mail's Recent lists it (Mail.js's
/// `lately`): what he sent or read in the last month, newest first, a
/// conversation once by its newest, leaving out any conversation that
/// still wants him, which Mail lists above it and is never Recent. A
/// conversation is its page in Gmail, or the mail itself where it has
/// none.
fn had_lately<'a>(had: &'a [Mail], held: &[Mail]) -> Vec<&'a Mail> {
    let talk = |mail: &Mail| {
        if mail.page.is_empty() {
            format!("mail:{}", mail.id)
        } else {
            mail.page.clone()
        }
    };
    let mut seen: Vec<String> = held
        .iter()
        .filter(|mail| mail.done.is_none())
        .map(talk)
        .collect();
    let mut lately: Vec<&Mail> = had.iter().collect();
    lately.sort_by_key(|mail| Reverse(mail.at));
    lately.retain(|&mail| {
        let conversation = talk(mail);
        let new = !seen.contains(&conversation);
        seen.push(conversation);
        new
    });
    lately.truncate(RECENT);
    lately
}

/// Mail's Recent as rows, `row:lately:<id>`, each as Mail's row says it.
pub fn lately(had: &[Mail], held: &[Mail], words: &Words) -> Vec<Value> {
    had_lately(had, held)
        .iter()
        .map(|mail| row(mail, "lately", words))
        .collect()
}

/// Each wanting piece's drawer, and each of Mail's Recent's, by the key
/// its row's drawer is watched by.
pub fn mail_drawers(held: &[Mail], had: &[Mail], words: &Words) -> Vec<(String, Value)> {
    let wanting = held
        .iter()
        .filter(|mail| mail.done.is_none())
        .map(|mail| ("mail", mail));
    let recent = had_lately(had, held)
        .into_iter()
        .map(|mail| ("lately", mail));
    wanting
        .chain(recent)
        .map(|(place, mail)| {
            let key = format!("drawer:{place}:{}", mail.id);
            (key, drawer(mail, titled(mail, words), words))
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

/// A mail's row, in its place's order without an `order`, which the
/// daemon spaces as each place's rows were last told (the ready-rows
/// crate's `Kept`).
fn row(mail: &Mail, place: &str, words: &Words) -> Value {
    json!({
        "key": format!("row:{place}:{}", mail.id),
        "thing": format!("mail:{}", mail.id),
        "place": place,
        "kind": "mail",
        "name": titled(mail, words),
        "glyph": words.glyph,
        "hue": "program",
        "depth": 0,
        "side": beside(mail, words),
        "drawer": format!("drawer:{place}:{}", mail.id),
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
fn drawer(mail: &Mail, title: &str, words: &Words) -> Value {
    let files: Vec<&str> = mail
        .attachments
        .iter()
        .map(|file| file.name.as_str())
        .collect();
    let carried = if files.len() == 1 {
        &words.attached
    } else {
        &words.attached_several
    };
    let mut facts = Vec::new();
    for (name, said) in [
        (&words.from, sender(mail, words)),
        (&words.to_fact, mail.to.clone()),
        (&words.cc, mail.cc.clone()),
    ] {
        if !said.is_empty() {
            facts.push(json!({"name": name, "said": said}));
        }
    }
    facts.push(json!({"name": words.date, "at": mail.at, "as": "dated"}));
    if !files.is_empty() {
        facts.push(json!({"name": carried, "said": files.join(", ")}));
    }
    json!({"title": title, "facts": facts, "body": mail.body})
}

/// Who sent it, by name and address as mail writes them, or whichever of
/// the two it gives.
fn sender(mail: &Mail, words: &Words) -> String {
    if mail.address.is_empty() || mail.address == mail.from {
        mail.from.clone()
    } else if mail.from.is_empty() {
        mail.address.clone()
    } else {
        fill(
            &words.sender,
            &[("name", &mail.from), ("address", &mail.address)],
        )
    }
}

/// Its acts, in the one order every surface lists them, each with what it
/// asks of this daemon, or the step the launcher takes for it: its sender
/// is People's row for the address (`step: person`), since the launcher
/// holds People, and Attachments steps in to its files (`step:
/// attachments`), which `held.attachments` gives until they're rows of
/// their own.
fn acts(mail: &Mail) -> Vec<Value> {
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
    listed.push(json!({"key": "verb:mail:link", "copy": mail.page}));
    listed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::done::{Done, Went};
    use crate::mail::{Bytes, File};

    fn words() -> Words {
        Words::read(&marked::vocabulary()).expect("mail's words didn't read")
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
        assert_eq!(words().untitled, "Mail Without a Subject");
        let lacking = json!({"things": [{"id": "mail:message", "glyph": "mail"}]});
        assert!(
            Words::read(&lacking).is_err_and(|lack| lack.contains("untitled")),
            "a vocabulary without mail's untitled word read as whole"
        );
        let mut factless = marked::vocabulary();
        factless["things"]
            .as_array_mut()
            .expect("the vocabulary has no things")
            .retain(|thing| thing["id"] != "mail:fact:cc");
        assert_eq!(
            Words::read(&factless),
            Err("the vocabulary has no mail:fact:cc".to_owned()),
            "a vocabulary without the Cc fact read as whole"
        );
        let (read, lacking) = Vocabulary::read(&factless);
        assert!(
            read.mail.is_none() && read.documents.is_some() && read.calendar.is_some(),
            "mail's lost word cost another place its rows"
        );
        assert_eq!(lacking, ["mail: the vocabulary has no mail:fact:cc"]);
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
        assert_eq!(first["thing"], "mail:b");
        assert_eq!(first["place"], "mail");
        assert_eq!(first["kind"], "mail");
        assert_eq!(first["name"], "Engines");
        assert_eq!(first["glyph"], "mail");
        assert_eq!(first["hue"], "program");
        assert_eq!(first["depth"], 0);
    }

    #[test]
    fn mails_recent_is_what_joel_had_lately_a_conversation_once_none_still_wanting_him() {
        let in_talk = |id: &str, at: i64, talk: &str| {
            let mut mail = one(id, at);
            mail.page = format!("https://mail.google.com/mail/u/0/#inbox/{talk}");
            mail
        };
        let mut no_page = one("bare", 500);
        no_page.page = String::new();
        let had = [
            in_talk("older", 1_000, "plans"),
            in_talk("newer", 4_000, "plans"),
            in_talk("asking", 3_000, "wanting"),
            in_talk("alone", 2_000, "alone"),
            no_page,
        ];
        let held = [in_talk("still", 5_000, "wanting")];
        let rows = lately(&had, &held, &words());
        let keys: Vec<&str> = rows.iter().filter_map(|row| row["key"].as_str()).collect();
        assert_eq!(
            keys,
            ["row:lately:newer", "row:lately:alone", "row:lately:bare"],
            "a conversation once at its newest, none that still wants him"
        );
        assert_eq!(rows[0]["place"], "lately");
        assert_eq!(rows[0]["thing"], "mail:newer");
        assert_eq!(rows[0]["drawer"], "drawer:lately:newer");
        let many: Vec<Mail> = (0..12)
            .map(|at| in_talk(&format!("m{at}"), at, &format!("t{at}")))
            .collect();
        assert_eq!(lately(&many, &[], &words()).len(), 9, "a Recent lists nine");
        let drawers: Vec<String> = mail_drawers(&held, &had, &words())
            .into_iter()
            .map(|(key, _)| key)
            .collect();
        assert_eq!(
            drawers,
            [
                "drawer:mail:still",
                "drawer:lately:newer",
                "drawer:lately:alone",
                "drawer:lately:bare"
            ]
        );
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
        let drawers = mail_drawers(&both, &[], &words());
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
        let drawers = mail_drawers(&[carrying], &[], &words());
        assert_eq!(drawers[0].0, "drawer:mail:a");
        assert_eq!(
            drawers[0].1,
            json!({
                "title": "Engines",
                "facts": [
                    {"name": "From", "said": "Ada Lovelace <ada@example.com>"},
                    {"name": "To", "said": "Joel"},
                    {"name": "Cc", "said": "Grace Hopper"},
                    {"name": "Date", "at": 1_000, "as": "dated"},
                    {"name": "Attachments", "said": "plan.pdf, notes.txt"}
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
                {"key": "verb:mail:open", "open": page, "in": "google-opens"},
                {"key": "verb:mail:reply", "open": reply, "in": "google-opens"},
                {"key": "verb:mail:replyAll", "open": reply_all, "in": "google-opens"},
                {"key": "verb:mail:archive", "ask": {"archive": {"id": "a"}}},
                {"key": "verb:mail:read", "ask": {"read": {"id": "a"}}},
                {"key": "mail:sender", "step": "person", "address": "ada@example.com"},
                {"key": "verb:mail:link", "copy": page}
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
    fn every_word_a_mail_shows_is_the_vocabularys() {
        let words =
            Words::read(&marked::marked(&marked::vocabulary())).expect("marked words didn't read");
        let mut untitled = one("u", 1_000);
        untitled.subject = String::new();
        untitled.from = String::new();
        let mut carrying = one("c", 2_000);
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
        let mut held = vec![untitled, carrying.clone()];
        for (at, went) in [1, 2, 5].into_iter().enumerate() {
            let mut sent = one(&format!("s{at}"), 3_000);
            sent.sent = true;
            sent.went = ["Ada", "Grace", "Alan", "Edsger", "Barbara"][..went]
                .iter()
                .map(|name| (*name).to_owned())
                .collect();
            held.push(sent);
        }
        carrying.attachments.truncate(1);
        carrying.id = "one".into();
        held.push(carrying);
        let mut shown = Vec::new();
        for row in mail(&held, &words) {
            let key = row["key"].as_str().unwrap_or_default().to_owned();
            shown.push((
                format!("{key} name"),
                row["name"].as_str().unwrap_or_default().to_owned(),
            ));
            for side in row["side"].as_array().into_iter().flatten() {
                for way in side["ways"].as_array().into_iter().flatten() {
                    shown.push((
                        format!("{key} side"),
                        way.as_str().unwrap_or_default().to_owned(),
                    ));
                }
            }
        }
        for (key, drawer) in mail_drawers(&held, &[], &words) {
            let title = drawer["title"].as_str().unwrap_or_default().to_owned();
            shown.push((format!("{key} title"), title));
            for fact in drawer["facts"].as_array().into_iter().flatten() {
                for field in ["name", "said"] {
                    if let Some(said) = fact[field].as_str() {
                        shown.push((format!("{key} fact {field}"), said.to_owned()));
                    }
                }
            }
        }
        assert!(
            shown.len() > 30,
            "only {} words were shown, so the spread reaches too little",
            shown.len()
        );
        // What a mail says of itself, which no vocabulary words: who sent
        // it and from where, whom it went and was copied to, its subject
        // and the names of the files it carries.
        let own: Vec<String> = held
            .iter()
            .flat_map(|each| {
                [
                    vec![
                        each.from.clone(),
                        each.address.clone(),
                        each.to.clone(),
                        each.cc.clone(),
                        each.subject.clone(),
                    ],
                    each.went.clone(),
                    each.attachments
                        .iter()
                        .map(|file| file.name.clone())
                        .collect(),
                ]
            })
            .flatten()
            .collect();
        let written = marked::written_in(&shown, &own);
        assert!(
            written.is_empty(),
            "words written in rows.rs, not the vocabulary:\n{}",
            written.join("\n")
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
