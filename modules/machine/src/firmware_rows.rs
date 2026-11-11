//! Firmware's rows and headline, ready for the launcher, from the
//! daemon's firmware records, fwupd's own answers (`firmware.rs`): each
//! device fwupd lists, those with an update offered first, then in the
//! order held, each holding its Install and its Version a step in, its
//! drawer saying everything fwupd says of it and of what's offered; then
//! Security, fwupd's rating of the machine's own firmware, each attribute
//! the level is made of a step in, with what fixes it. The headline says
//! "up to date" or how many updates are ready. They take the launcher's
//! `Firmware.qml` and `Firmware.js` across whole (VKCF3Z's rulings,
//! 2026-10-29 to 2026-11-09), and every word is words.json's.
//!
//! An act that can't be taken back (Install, Secure It, Undo It, Reboot
//! Into Firmware Setup) asks once: its row holds one row inside, `sure`,
//! naming what it does, whose Return asks the daemon's act (Y2YBTS,
//! 2026-11-11). While an act goes its row says so and asks nothing, and
//! once it ends it says how until the place is no longer watched
//! ([`Acting`]), as the launcher kept it until it shut.
//!
//! Long because each of Firmware's four kinds of row, its drawers and its
//! acts are one place's, worded from one record, and its cases beside
//! them; split by kind, each would carry the same helpers again.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::rows::{Made, Ready, item};
use crate::words::Words;

/// The place the launcher shows these rows in.
pub const PLACE: &str = "firmware";
/// A device fwupd keeps from people, as it does each region of the BIOS's
/// flash chip and the Secure Boot keys, which it updates only as a whole.
const HIDDEN: &str = "updatable-hidden";
/// An update that finishes only at a restart, as the BIOS's does.
const AT_RESTART: &str = "needs-reboot";
/// What fwupd names a vendor it doesn't know.
const UNKNOWN: &str = "Unknown";
/// What fwupd's history says of an update that's in but finishes at a
/// restart: pending, as a BIOS's capsule is until the restart flashes it,
/// or needing a reboot.
const AT_THE_RESTART: [&str; 2] = ["pending", "needs-reboot"];
/// What it says of one that didn't go in.
const FAILED: [&str; 2] = ["failed", "failed-transient"];
/// The characters release notes escape, and what each is, the ampersand
/// last so nothing is unescaped twice.
const ESCAPED: [(&str, &str); 7] = [
    ("&lt;", "<"),
    ("&gt;", ">"),
    ("&quot;", "\""),
    ("&#39;", "'"),
    ("&apos;", "'"),
    ("&nbsp;", " "),
    ("&amp;", "&"),
];
/// The tags of release notes that break a line, opening or closing.
const BREAKS: [&str; 5] = ["li", "p", "ul", "ol", "br"];
/// fwupd's highest level, whose protections are each of those levels' and
/// none above.
const TOP_LEVEL: u64 = 5;
/// What fixes a failing attribute, by fwupd's flag for it: a setting in
/// the firmware's setup, one in the system, or an update from its maker.
const FIXES: [(&str, &str); 3] = [
    ("action-config-fw", "setup"),
    ("action-config-os", "system"),
    ("action-contact-oem", "maker"),
];
/// The least an install is waited for, in minutes: long enough for a
/// polkit prompt Joel is answering and an update fwupd says nothing of
/// the length of.
const INSTALL_PATIENCE_MIN: u64 = 30;
/// A security attribute's fix or undo is one call fwupd answers at once
/// (`fu_util_security_fix` in fwupd 2.1.9), so two minutes is a polkit
/// prompt left unanswered, not a change still going in; the restart into
/// setup is waited for as long.
const CHANGE_PATIENCE_MIN: u64 = 2;
/// The units bytes are said in, a thousand apart, as the files and
/// Programs say them (VKCF3Z, 2026-11-07).
const BYTES: [&str; 6] = ["B", "kB", "MB", "GB", "TB", "PB"];
/// Past this a figure rounds to four digits, so it takes the next unit.
const ROUNDS_UP: f64 = 999.5;
/// The parts of a side or note said in one.
const DOT: &str = " \u{b7} ";

/// How Firmware's acts stand, as the daemon runs them: the one going, and
/// how each since the place was watched ended, each by its act and what
/// it acts on, as "install:<device>", "fix:<attribute>",
/// "undo:<attribute>" or "setup".
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Acting {
    pub going: Option<String>,
    pub ended: BTreeMap<String, Ended>,
}

/// How an act ended.
#[derive(Clone, Debug, PartialEq)]
pub enum Ended {
    /// It was done.
    Done,
    /// It didn't go, with what fwupd, fwupdmgr or logind said of why.
    Refused(String),
    /// The daemon stopped waiting for it after so many minutes.
    Stopped(u64),
}

/// Firmware's rows and headline from the daemon's firmware records, as
/// its acts stand. A machine whose fwupd lists no device has none.
pub fn read(records: &[Value], acting: &Acting, words: &Words) -> Ready {
    let say = Say { words, acting };
    let devices = devices_of(records);
    if devices.is_empty() {
        return Ready::default();
    }
    let mut made: Vec<Made> = devices.iter().map(|device| say.device(device)).collect();
    let host = records
        .iter()
        .find(|record| record["key"] == "firmware:host");
    if let Some(security) = host.and_then(|host| say.security(host)) {
        made.push(security);
    }
    let mut rows = Vec::new();
    crate::rows::placed(PLACE, made, None, 0, &mut rows);
    let offered = devices
        .iter()
        .filter(|device| device.offered.is_some())
        .count();
    let mut head = json!({ "key": format!("head:{PLACE}"), "place": PLACE,
        "said": say.standing(offered) });
    if offered > 0 {
        head["hue"] = json!("warning");
    }
    Ready {
        rows,
        head: Some(head),
    }
}

/// One device as Firmware shows it, from its record.
struct Device<'a> {
    record: &'a Value,
    id: &'a str,
    name: String,
    version: &'a str,
    /// The version offered, or the one installed to finish at the next
    /// restart, which is offered until then.
    offered: Option<&'a str>,
    /// The version an update installed waits for the restart to flash.
    waits: Option<&'a str>,
    restarts: bool,
    /// fwupd's words for why the last install didn't go in.
    failed: Option<&'a str>,
}

/// A record's text field, or "".
fn text<'a>(record: &'a Value, field: &str) -> &'a str {
    record[field].as_str().unwrap_or("")
}

/// Whether a record's list of names holds one.
fn holds(record: &Value, field: &str, name: &str) -> bool {
    record[field]
        .as_array()
        .is_some_and(|names| names.iter().any(|each| each == name))
}

/// Each device worth listing: a named device with a version that fwupd
/// doesn't hide, those with an update first, two of one name numbered,
/// as two USB-C controllers of one dock are, so neither reads as the
/// other.
fn devices_of(records: &[Value]) -> Vec<Device<'_>> {
    let shown = records.iter().filter(|record| {
        record["kind"] == "firmware"
            && record["key"] != "firmware:host"
            && !text(record, "name").is_empty()
            && !text(record, "version").is_empty()
            && !holds(record, "flags", HIDDEN)
    });
    let (mut list, rest): (Vec<Device>, Vec<Device>) = shown
        .map(device_of)
        .partition(|device| device.offered.is_some());
    list.extend(rest);
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for device in &list {
        *counts.entry(device.name.clone()).or_default() += 1;
    }
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for device in &mut list {
        if counts[&device.name] > 1 {
            let n = seen.entry(device.name.clone()).or_default();
            *n += 1;
            device.name = format!("{} {n}", device.name);
        }
    }
    list
}

fn device_of(record: &Value) -> Device<'_> {
    let last = &record["last_update"];
    let version = text(record, "version");
    let waited = last["version"].as_str().filter(|waited| !waited.is_empty());
    let waits =
        waited.filter(|waited| AT_THE_RESTART.contains(&text(last, "state")) && *waited != version);
    Device {
        record,
        id: text(record, "id"),
        name: text(record, "name").to_owned(),
        version,
        offered: record["update"]["version"].as_str().or(waits),
        waits,
        restarts: holds(record, "flags", AT_RESTART),
        failed: FAILED
            .contains(&text(last, "state"))
            .then_some(text(last, "error")),
    }
}

/// One attribute of the security level, as fwupd says it.
struct Attribute<'a> {
    said: &'a Value,
    id: &'a str,
    name: &'a str,
    /// None for one about what's running.
    level: Option<u64>,
    passed: bool,
    /// A FIXES name, or "".
    fix: &'static str,
}

impl Attribute<'_> {
    fn flagged(&self, flag: &str) -> bool {
        holds(self.said, "flags", flag)
    }
}

/// Each attribute the level is made of, one another obsoletes left out
/// as fwupd's own count leaves it, named by fwupd's summary.
fn attributes_of(host: &Value) -> Vec<Attribute<'_>> {
    let Some(said) = host["attributes"].as_array() else {
        return vec![];
    };
    said.iter()
        .filter(|one| !holds(one, "flags", "obsoleted"))
        .map(|one| {
            let id = text(one, "id");
            let name = [text(one, "summary"), text(one, "name"), id]
                .into_iter()
                .find(|name| !name.is_empty())
                .unwrap_or_default();
            Attribute {
                said: one,
                id,
                name,
                level: one["level"].as_u64(),
                passed: one["passed"] == true,
                fix: FIXES
                    .iter()
                    .find(|(flag, _)| holds(one, "flags", flag))
                    .map_or("", |(_, fix)| *fix),
            }
        })
        .collect()
}

/// fwupd's rating as it writes it, "HSI:1!", without the version it
/// appends, " (v2.1.9)": its code, its level, and whether something
/// running lowers it, the "!".
fn rating_of(security: &str) -> (String, Option<u64>, bool) {
    let trimmed = security.trim();
    let code = trimmed
        .strip_suffix(')')
        .and_then(|open| open.rfind("(v").map(|at| &open[..at]))
        .map_or(trimmed, str::trim_end);
    let mut rest = code.strip_prefix("HSI:").unwrap_or("").chars();
    let level = rest
        .next()
        .and_then(|digit| digit.to_digit(10))
        .map(u64::from);
    let lowered = level.is_some() && rest.next() == Some('!');
    (code.to_owned(), level, lowered)
}

/// Release notes as words: fwupd gives them as a little HTML, paragraphs
/// and lists, so each becomes a line of its own and nothing else is kept.
fn words_of(html: &str) -> String {
    let mut plain = String::new();
    let mut rest = html;
    while let Some(open) = rest.find('<') {
        plain.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('>') else {
            rest = "";
            break;
        };
        let tag = rest[open + 1..open + close]
            .trim_start_matches(|c: char| c == '/' || c.is_whitespace())
            .split(|c: char| c.is_whitespace() || c == '/')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if BREAKS.contains(&tag.as_str()) {
            plain.push('\n');
        }
        rest = &rest[open + close + 1..];
    }
    plain.push_str(rest);
    for (escaped, said) in ESCAPED {
        plain = plain.replace(escaped, said);
    }
    plain
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The last line fwupdmgr, fwupd or logind said of why what it was asked
/// didn't go, as a note says it after a dot: its first letter lowered,
/// unless it opens a word in capitals, and no closing full stop (VKCF3Z,
/// 2026-11-09).
fn reason(said: &str) -> String {
    let Some(last) = said.lines().map(str::trim).rfind(|line| !line.is_empty()) else {
        return String::new();
    };
    let last = last.strip_suffix('.').unwrap_or(last);
    let mut letters = last.chars();
    let capitals = last
        .chars()
        .take(2)
        .filter(char::is_ascii_uppercase)
        .count()
        == 2;
    match letters.next() {
        Some(first) if !capitals => first.to_lowercase().chain(letters).collect(),
        _ => last.to_owned(),
    }
}

/// How many minutes an install of a device's update is waited for: twice
/// what fwupd says it takes, and never under `INSTALL_PATIENCE_MIN`.
fn install_patience(device: &Device) -> u64 {
    let minutes = device.record["update"]["install_minutes"]
        .as_u64()
        .unwrap_or(0);
    INSTALL_PATIENCE_MIN.max(minutes.saturating_mul(2))
}

/// A figure a step in that Return copies.
fn copy(id: String, name: String, said: &str) -> Made {
    Made {
        id,
        kind: "figure",
        name,
        hue: "machine",
        side: vec![item(said)],
        copies: said.to_owned(),
        ..Made::default()
    }
}

/// Firmware's words, and how its acts stand.
struct Say<'a> {
    words: &'a Words,
    acting: &'a Acting,
}

impl Say<'_> {
    fn name(&self, id: &str, fill: &[(&str, &str)]) -> String {
        self.words.name(id, fill)
    }

    fn said(&self, id: &str, phrase: &str, fill: &[(&str, &str)]) -> String {
        self.words.said(id, phrase, fill)
    }

    /// How many updates are offered: "up to date", or "1 update ready".
    fn standing(&self, offered: usize) -> String {
        match offered {
            0 => self.said(PLACE, "current", &[]),
            1 => self.said(PLACE, "one", &[]),
            count => self.said(PLACE, "many", &[("count", &count.to_string())]),
        }
    }

    /// A device's version as its row says it, how it stands first
    /// (VKCF3Z, 2026-11-08): "1.12.0", "1.13.0 ready · from 1.12.0", or
    /// "1.13.0 at the restart · from 1.12.0" once it's installed to
    /// finish there.
    fn version_of(&self, device: &Device) -> String {
        let from = |way: &str, version: &str| {
            self.said(PLACE, way, &[("version", version), ("was", device.version)])
        };
        match (device.waits, device.offered) {
            (Some(waits), _) => from("restart", waits),
            (None, Some(offered)) => from("ready", offered),
            (None, None) => device.version.to_owned(),
        }
    }

    /// What holds an offered update back, in fwupd's own words, or "":
    /// the first problem fwupd says once nothing waits for the restart,
    /// since an update waiting there is held by nothing.
    fn held(&self, device: &Device) -> String {
        if device.offered.is_none() || device.waits.is_some() {
            return String::new();
        }
        let problems = device.record["problems"].as_array();
        problems
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .find(|problem| *problem != "update-pending")
            .map_or_else(String::new, |problem| self.problem(problem))
    }

    fn problem(&self, problem: &str) -> String {
        let said = self.said("firmware:held", problem, &[]);
        if said.is_empty() {
            problem.to_owned()
        } else {
            said
        }
    }

    /// A device's row: its version and what holds an update back on the
    /// side, after a dot, the latter going first where the row is too
    /// narrow (VKCF3Z, 2026-11-08), warning while an update is offered.
    fn device(&self, device: &Device) -> Made {
        let version = self.version_of(device);
        let held = self.held(device);
        let side = if held.is_empty() {
            vec![item(&version)]
        } else {
            vec![
                json!({ "said": version, "keep": 2 }),
                json!({ "said": held, "keep": 1 }),
            ]
        };
        let mut inside = Vec::new();
        let key = format!("install:{}", device.id);
        if device.offered.is_some()
            || self.acting.ended.contains_key(&key)
            || self.acting.going.as_ref() == Some(&key)
        {
            inside.push(self.install(device, &key));
        }
        inside.push(copy(
            format!("{}:version", device.id),
            self.name("firmware:version", &[]),
            device.version,
        ));
        Made {
            id: device.id.to_owned(),
            kind: "firmware",
            name: device.name.clone(),
            glyph: self.words.glyph(PLACE),
            hue: if device.offered.is_some() {
                "warning"
            } else {
                ""
            },
            side,
            drawer: Some(self.device_drawer(device, &key)),
            inside,
            ..Made::default()
        }
    }

    /// A drawer's fact, drawn in the Name face where it identifies.
    fn fact(&self, id: &str, said: &str, identifies: bool) -> Value {
        let mut fact = json!({ "name": self.name(id, &[]), "said": said });
        if identifies {
            fact["identifies"] = json!(true);
        }
        fact
    }

    /// An act's row, saying `note`.
    fn act(&self, id: String, verb: &str, name: String, note: String) -> Made {
        Made {
            id,
            kind: "act",
            name,
            glyph: self.words.glyph(verb),
            hue: "action",
            note,
            ..Made::default()
        }
    }

    /// The one row inside an act that can't be taken back, asking Joel to
    /// be sure, its Return asking the daemon's act.
    fn sure(&self, id: String, verb: &str, name: String, note: String, ask: &Value) -> Made {
        Made {
            id,
            kind: "act",
            name,
            glyph: self.words.glyph(verb),
            hue: "alert",
            note,
            acts: vec![json!({ "key": verb, "ask": ask })],
            sure: true,
            ..Made::default()
        }
    }

    /// What an act that ended without being done says after its verb's
    /// "didn't", or "" for one that was done or hasn't ended.
    fn failed(&self, verb: &str, ended: Option<&Ended>) -> String {
        let why = match ended {
            Some(Ended::Refused(said)) => reason(said),
            Some(Ended::Stopped(minutes)) => {
                self.said(verb, "stopped", &[("count", &minutes.to_string())])
            }
            Some(Ended::Done) | None => return String::new(),
        };
        [self.said(verb, "failed", &[]), why]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(DOT)
    }

    /// Install, as it stands: ready, holding the step that asks once,
    /// saying it can't be undone; installing, asking nothing more; or done
    /// with, once it waits for the restart or has gone in. One fwupd won't
    /// put in now says why in fwupd's words, and one that didn't install
    /// says so and can be tried again (VKCF3Z, 2026-10-30 and 2026-11-08).
    fn install(&self, device: &Device, key: &str) -> Made {
        const VERB: &str = "verb:firmware:install";
        let version = device.offered.unwrap_or(device.version);
        let ended = self.acting.ended.get(key);
        let said = |phrase: &str| self.said(VERB, phrase, &[]);
        let id = format!("{}:install", device.id);
        let name = self.name(VERB, &[("version", version)]);
        let held = self.held(device);
        let done = |note: String| Made {
            gone: true,
            ..self.act(id.clone(), VERB, name.clone(), note)
        };
        let note = if self.acting.going.as_deref() == Some(key) {
            said("installing")
        } else if device.waits.is_some() {
            return done(said("waits"));
        } else if device.offered.is_none() || ended == Some(&Ended::Done) {
            return done(said("installed"));
        } else if !held.is_empty() {
            held
        } else {
            let mut row = self.act(id.clone(), VERB, name, self.failed(VERB, ended));
            let fill = [("version", version), ("device", device.name.as_str())];
            let last = if device.restarts {
                "final-at-restart"
            } else {
                "final"
            };
            let ask = json!({ "ask": "act", "act": "firmware-install", "device": device.id,
                "patience_s": install_patience(device) * 60 });
            row.inside = vec![self.sure(
                format!("{id}:sure"),
                VERB,
                self.words.sure(VERB, &fill),
                said(last),
                &ask,
            )];
            return row;
        };
        self.act(id, VERB, name, note)
    }

    /// A device's drawer: who made it in its eyebrow, "from Dell ·
    /// firmware"; what it is and where it stands, what's offered and what
    /// holds it back, leaving the versions to the row's side (VKCF3Z,
    /// 2026-11-08); its identifiers; then why the last install didn't go,
    /// and the notes of what's offered. A fact fwupd doesn't know is left
    /// out.
    fn device_drawer(&self, device: &Device, key: &str) -> Value {
        let record = device.record;
        let update = &record["update"];
        let owned = |field: &Value| field.as_str().unwrap_or("").to_owned();
        let vendor = text(record, "vendor");
        let eyebrow = if vendor.is_empty() || vendor == UNKNOWN {
            self.said("firmware:vendor", "alone", &[])
        } else {
            self.said("firmware:vendor", "from", &[("vendor", vendor)])
        };
        let failed = match self.acting.ended.get(key) {
            ended @ Some(Ended::Refused(said)) if !reason(said).is_empty() => {
                self.failed("verb:firmware:install", ended)
            }
            _ => String::new(),
        };
        let notes = words_of(text(update, "description"));
        let body: Vec<String> = [failed, owned(&update["summary"]), notes]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect();
        json!({ "eyebrow": eyebrow, "title": device.name,
            "facts": self.device_facts(device), "body": body.join("\n\n") })
    }

    /// What a device's drawer says of it, in order, each fwupd doesn't
    /// know left out.
    fn device_facts(&self, device: &Device) -> Vec<Value> {
        let record = device.record;
        let update = &record["update"];
        let said = |id: &str, said: &str| (!said.is_empty()).then(|| self.fact(id, said, false));
        let names = |id: &str| {
            (!text(record, id).is_empty())
                .then(|| self.fact(&format!("firmware:{id}"), text(record, id), true))
        };
        let listed = |field: &str| {
            let names = record[field].as_array().into_iter().flatten();
            names.filter_map(Value::as_str).collect::<Vec<_>>()
        };
        // A day reads the clock, so it travels as a time the shell words
        // (ready-rows.md's Times).
        let day = |id: &str, at: &Value| {
            let at = at.as_u64()?;
            Some(json!({ "name": self.name(id, &[]), "at": at * 1000, "as": "day" }))
        };
        let count = |field: &Value| field.as_u64().map_or_else(String::new, |n| n.to_string());
        let urgency = text(update, "urgency");
        let size = update["size_b"].as_u64().filter(|b| *b > 0);
        let takes = update["install_minutes"].as_u64().filter(|m| *m > 0);
        let takes = takes.map_or_else(String::new, |m| {
            self.said("firmware:takes", "minutes", &[("count", &m.to_string())])
        });
        let held: Vec<String> = listed("problems")
            .into_iter()
            .map(|problem| self.problem(problem))
            .collect();
        let none = if device.offered.is_none() {
            text(record, "no_update")
        } else {
            ""
        };
        let battery = record["battery_pct"].as_u64();
        [
            said("firmware:lowest", text(record, "version_lowest")),
            said("firmware:bootloader", text(record, "version_bootloader")),
            said("firmware:branch", text(record, "branch")),
            day(
                "firmware:last-updated",
                &record["last_update"]["updated_at_s"],
            ),
            said(
                "firmware:urgency",
                if urgency == "unknown" { "" } else { urgency },
            ),
            said(
                "firmware:size",
                &size.map_or_else(String::new, |b| self.bytes(b)),
            ),
            day("firmware:released", &update["released_at_s"]),
            said("firmware:takes", &takes),
            said("firmware:held", &held.join(DOT)),
            said("firmware:none", none),
            said("firmware:failed", device.failed.unwrap_or("")),
            said("firmware:flashes", &count(&record["flashes_left"])),
            said(
                "firmware:battery",
                &battery.map_or_else(String::new, |pct| format!("{pct}%")),
            ),
            said("firmware:protocols", &listed("protocols").join(DOT)),
            names("plugin"),
            names("serial"),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// A count of bytes in the unit it reads best in, a thousand apart:
    /// "915 B", "1.52 MB".
    fn bytes(&self, count: u64) -> String {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a release's size is said to three digits"
        )]
        let mut scaled = count as f64;
        let mut at = 0;
        while scaled.abs() >= ROUNDS_UP && at < BYTES.len() - 1 {
            scaled /= 1000.0;
            at += 1;
        }
        let said = if at == 0 {
            self.words.number(scaled, 0)
        } else {
            self.words.significant(scaled)
        };
        format!("{said} {}", BYTES[at])
    }

    /// What fwupd found of an attribute in our words, "not enabled", and
    /// that it couldn't read what it needed where it says so.
    fn result_of(&self, attribute: &Attribute) -> String {
        let result = attribute.said["result"].as_str().unwrap_or("unknown");
        let found = self.found(result);
        if attribute.flagged("missing-data") {
            format!(
                "{found}{DOT}{}",
                self.said("firmware:result", "missing", &[])
            )
        } else {
            found
        }
    }

    fn found(&self, result: &str) -> String {
        let said = self.said("firmware:result", result, &[]);
        if said.is_empty() {
            result.to_owned()
        } else {
            said
        }
    }

    /// fwupd's rating of the machine's own firmware, "HSI:1!", without
    /// the version it appends; its note says the level in words, and that
    /// what's running lowers it where there's a "!" (VKCF3Z, 2026-11-09).
    /// The drawer says what the next level needs and what's running that
    /// lowers it, then what it rated and which fwupd, its body the level
    /// once as a sentence. A step in lists every attribute, each failing
    /// one first, then the rating, which Return copies, wearing Security
    /// Level's mark so it doesn't read as a heading.
    fn security(&self, host: &Value) -> Option<Made> {
        const ID: &str = "firmware:security";
        let (code, level, lowered) = rating_of(host["security"].as_str()?);
        let attributes = attributes_of(host);
        let meaning =
            level.map_or_else(String::new, |level| self.said(ID, &level.to_string(), &[]));
        let failing = |list: Vec<&Attribute>| {
            list.iter()
                .map(|one| format!("{} {}", one.name, self.result_of(one)))
                .collect::<Vec<_>>()
                .join(DOT)
        };
        let mut facts = Vec::new();
        if let Some(level) = level.filter(|level| *level < TOP_LEVEL) {
            let needed: Vec<&Attribute> = attributes
                .iter()
                .filter(|one| one.level == Some(level + 1) && !one.passed)
                .collect();
            if !needed.is_empty() {
                let next = (level + 1).to_string();
                facts.push(
                    json!({ "name": self.name("firmware:needed", &[("level", &next)]),
                    "said": failing(needed) }),
                );
            }
        }
        let lowering: Vec<&Attribute> = attributes
            .iter()
            .filter(|one| lowered && one.flagged("runtime-issue") && !one.passed)
            .collect();
        if !lowering.is_empty() {
            facts.push(json!({ "name": self.name("firmware:lowered", &[]),
                "said": failing(lowering) }));
        }
        if let Some(product) = host["product"].as_str().filter(|said| !said.is_empty()) {
            facts.push(json!({ "name": self.name("firmware:product", &[]), "said": product }));
        }
        if let Some(fwupd) = host["fwupd"].as_str().filter(|said| !said.is_empty()) {
            facts.push(json!({ "name": self.name("firmware:rated", &[]),
                "said": format!("fwupd {fwupd}") }));
        }
        let form = if lowered {
            "sentence-lowered"
        } else {
            "sentence"
        };
        let sentence = match level {
            Some(level) if !meaning.is_empty() => self.said(
                ID,
                form,
                &[("level", &level.to_string()), ("meaning", &meaning)],
            ),
            _ => String::new(),
        };
        let note = [
            meaning,
            if lowered {
                self.said(ID, "lowered", &[])
            } else {
                String::new()
            },
        ]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(DOT);
        let mut listed: Vec<&Attribute> = attributes.iter().collect();
        listed.sort_by_key(|one| (one.passed, one.level.unwrap_or(TOP_LEVEL + 1)));
        let mut inside: Vec<Made> = listed.into_iter().map(|one| self.attribute(one)).collect();
        let mut rating = copy("security:level".to_owned(), self.name(ID, &[]), &code);
        rating.glyph = self.words.glyph(PLACE);
        inside.push(rating);
        Some(Made {
            id: "security".to_owned(),
            kind: "rating",
            name: self.name(ID, &[]),
            glyph: self.words.glyph(PLACE),
            side: vec![item(&code)],
            note,
            drawer: Some(json!({ "eyebrow": self.said(ID, "eyebrow", &[]),
                "title": self.name(ID, &[]), "facts": facts, "body": sentence })),
            inside,
            ..Made::default()
        })
    }

    /// One attribute of the security level: its side its group, the
    /// level it counts at or what's running, muted, then what was found;
    /// the theme's mark for a pass, or its warning mark in warning ink for
    /// a shortfall (VKCF3Z, 2026-11-09). Its drawer says what passing
    /// would find, what fixes it, the setting it reads, what that's set to
    /// and what passing needs, and fwupd's description. A step in holds
    /// what takes Joel to its fix, then fwupd's id for it, which Return
    /// copies.
    fn attribute(&self, attribute: &Attribute) -> Made {
        let said = attribute.said;
        let group = attribute.level.map_or_else(
            || self.said("firmware:group", "running", &[]),
            |level| self.said("firmware:group", "level", &[("level", &level.to_string())]),
        );
        let result = self.result_of(attribute);
        let mut facts = Vec::new();
        let mut fact = |id: &str, value: &str, identifies: bool| {
            if !value.is_empty() {
                facts.push(self.fact(id, value, identifies));
            }
        };
        let failing = !attribute.passed;
        let setting = &said["bios_setting"];
        let kernel = &said["kernel"];
        if failing {
            let passes = said["passes_as"].as_str().map(|result| self.found(result));
            fact("firmware:passes-as", passes.as_deref().unwrap_or(""), false);
            fact(
                "firmware:fixed-in",
                &self.said("firmware:fixed-in", attribute.fix, &[]),
                false,
            );
        }
        fact("firmware:setting", text(setting, "id"), true);
        fact("firmware:set-to", text(setting, "current"), false);
        if failing {
            fact("firmware:should-be", text(setting, "target"), false);
        }
        fact("firmware:kernel-set-to", text(kernel, "current"), false);
        if failing {
            fact("firmware:kernel-should-be", text(kernel, "target"), false);
        }
        fact("firmware:plugin", text(said, "plugin"), true);
        fact("firmware:attribute", attribute.id, true);
        let kind = self.said("firmware:group", "kind", &[]);
        let eyebrow = [group.clone(), kind]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(DOT);
        let mut inside = Vec::new();
        if failing && attribute.flagged("can-fix") {
            inside.push(self.change(attribute, "fix"));
        }
        if attribute.passed && attribute.flagged("can-undo") {
            inside.push(self.change(attribute, "undo"));
        }
        if failing && attribute.fix == "setup" {
            inside.push(self.setup(attribute));
        }
        inside.push(copy(
            format!("security:{}:id", attribute.id),
            self.name("firmware:attribute", &[]),
            attribute.id,
        ));
        Made {
            id: format!("security:{}", attribute.id),
            kind: "attribute",
            name: attribute.name.to_owned(),
            glyph: if failing { "caution" } else { "complete" }.to_owned(),
            hue: if failing { "warning" } else { "machine" },
            side: vec![
                json!({ "said": group, "keep": 1, "hue": "muted" }),
                json!({ "said": result, "keep": 2 }),
            ],
            drawer: Some(
                json!({ "eyebrow": eyebrow, "title": attribute.name, "facts": facts,
                "body": text(said, "description") }),
            ),
            inside,
            ..Made::default()
        }
    }

    /// fwupd fixing an attribute itself, or putting a fix back, as
    /// fwupdmgr's security-fix and security-undo do: asked once, saying
    /// what it changes; saying so while it goes, and how it ended.
    fn change(&self, attribute: &Attribute, how: &str) -> Made {
        let verb = format!("verb:firmware:{how}");
        let key = format!("{how}:{}", attribute.id);
        let id = format!("security:{}:{how}", attribute.id);
        let name = self.name(&verb, &[]);
        let ended = self.acting.ended.get(&key);
        if self.acting.going.as_ref() == Some(&key) {
            return self.act(id, &verb, name, self.said(&verb, "changing", &[]));
        }
        if ended == Some(&Ended::Done) {
            return Made {
                gone: true,
                ..self.act(id, &verb, name, self.said(&verb, "changed", &[]))
            };
        }
        let failed = self.failed(&verb, ended);
        let note = if failed.is_empty() {
            self.said(&verb, "what", &[])
        } else {
            failed
        };
        let mut row = self.act(id.clone(), &verb, name, note);
        let ask = json!({ "ask": "act", "act": format!("firmware-{how}"),
            "attribute": attribute.id, "patience_s": CHANGE_PATIENCE_MIN * 60 });
        row.inside = vec![self.sure(
            format!("{id}:sure"),
            &verb,
            self.words.sure(&verb, &[("attribute", attribute.name)]),
            self.said(&verb, "final", &[]),
            &ask,
        )];
        row
    }

    /// Rebooting into the firmware's own setup, where the setting that
    /// fixes an attribute is changed, saying which and to what where
    /// fwupd says: asked once, since it closes every window. logind
    /// refuses it where the firmware can't be asked to open its setup, and
    /// the row then says logind's words for why.
    fn setup(&self, attribute: &Attribute) -> Made {
        const VERB: &str = "verb:firmware:setup";
        let setting = &attribute.said["bios_setting"];
        let target = text(setting, "target");
        let asked = if target.is_empty() {
            self.said(VERB, "find", &[])
        } else {
            self.said(
                VERB,
                "set",
                &[("setting", text(setting, "id")), ("target", target)],
            )
        };
        let failed = self.failed(VERB, self.acting.ended.get("setup"));
        let id = format!("security:{}:setup", attribute.id);
        let note = if failed.is_empty() { asked } else { failed };
        let mut row = self.act(id.clone(), VERB, self.name(VERB, &[]), note);
        let ask = json!({ "ask": "act", "act": "firmware-setup",
            "patience_s": CHANGE_PATIENCE_MIN * 60 });
        row.inside = vec![self.sure(
            format!("{id}:sure"),
            VERB,
            self.words.sure(VERB, &[]),
            self.said(VERB, "final", &[]),
            &ask,
        )];
        row
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Words {
        let vocabulary = json!({ "things": [
            { "id": "firmware", "name": "Firmware", "glyph": "firmware", "said": {
                "current": "up to date", "one": "1 update ready", "many": "{count} updates ready",
                "ready": "{version} ready \u{b7} from {was}",
                "restart": "{version} at the restart \u{b7} from {was}" } },
            { "id": "firmware:version", "name": "Version" },
            { "id": "firmware:vendor", "said": { "alone": "firmware",
                "from": "from {vendor} \u{b7} firmware" } },
            { "id": "firmware:last-updated", "name": "Last Updated" },
            { "id": "firmware:size", "name": "Size" },
            { "id": "firmware:released", "name": "Released" },
            { "id": "firmware:takes", "name": "Takes", "said": { "minutes": "{count} min" } },
            { "id": "firmware:held", "name": "Blocked By", "said": {
                "require-ac-power": "no charger" } },
            { "id": "firmware:serial", "name": "Serial" },
            { "id": "firmware:security", "name": "Security Level", "said": {
                "1": "only the most basic protections", "eyebrow": "on this machine \u{b7} rating",
                "lowered": "lowered by what's running",
                "sentence": "level {level} holds {meaning}.",
                "sentence-lowered":
                    "level {level} holds {meaning}, and what's running lowers it." } },
            { "id": "firmware:needed", "name": "Needed for Level {level}" },
            { "id": "firmware:lowered", "name": "Lowered By" },
            { "id": "firmware:result", "said": { "enabled": "enabled",
                "not-enabled": "not enabled", "missing": "unread" } },
            { "id": "firmware:group", "said": { "level": "level {level}", "running": "running",
                "kind": "check" } },
            { "id": "firmware:fixed-in", "said": { "setup": "firmware setup" } },
            { "id": "firmware:setting", "name": "Firmware Setting" },
            { "id": "firmware:attribute", "name": "Attribute" },
            { "id": "verb:firmware:install", "name": "Install {version}", "glyph": "firmware",
                "sure": "Install {version} on {device} Now", "said": {
                "final": "it can't be undone",
                "final-at-restart": "it can't be undone, and finishes at a restart",
                "installing": "installing", "waits": "restart to finish",
                "installed": "installed", "failed": "didn't install",
                "stopped": "stopped waiting after {count} min" } },
            { "id": "verb:firmware:fix", "name": "Secure It", "glyph": "firmware",
                "sure": "Secure {attribute} Now", "said": { "what": "fwupd changes the setting",
                "final": "it may need a restart", "changing": "securing",
                "changed": "secured", "failed": "didn't secure",
                "stopped": "stopped waiting after {count} min" } },
            { "id": "verb:firmware:undo", "name": "Undo It", "glyph": "firmware",
                "sure": "Undo {attribute} Now", "said": { "what": "fwupd puts it back",
                "final": "it may need a restart" } },
            { "id": "verb:firmware:setup", "name": "Reboot Into Firmware Setup",
                "glyph": "firmware", "sure": "Reboot Into Firmware Setup Now", "said": {
                "set": "set {setting} to {target} there", "find": "set it there",
                "final": "it closes every window", "failed": "didn't reboot" } }
        ] });
        Words::new(&vocabulary, "", "")
    }

    fn bios() -> Value {
        json!({ "key": "firmware:b10", "kind": "firmware", "id": "b10",
            "name": "System Firmware", "vendor": "Dell", "version": "1.12.0",
            "flags": ["internal", "updatable", "needs-reboot"], "problems": [],
            "serial": "ABC123",
            "update": { "version": "1.13.0", "summary": "A fix.",
                "description": "<p>Fixes &amp; more:</p><ul><li>one</li><li>two</li></ul>",
                "urgency": "unknown", "size_b": 1_520_000, "released_at_s": 1_761_000_000,
                "install_minutes": 20 },
            "last_update": { "state": "success", "updated_at_s": 1_750_000_000 } })
    }

    fn dock(n: u32) -> Value {
        json!({ "key": format!("firmware:d{n}"), "kind": "firmware", "id": format!("d{n}"),
            "name": "USB-C Controller", "vendor": "Unknown", "version": "2.0",
            "flags": ["updatable"], "problems": [] })
    }

    fn host() -> Value {
        json!({ "key": "firmware:host", "kind": "firmware", "security": "HSI:1! (v2.1.9)",
            "product": "XPS", "fwupd": "2.1.9", "attributes": [
                { "id": "org.fwupd.hsi.Tpm", "summary": "TPM v2.0", "level": 1,
                    "result": "enabled", "passed": true, "flags": ["success"] },
                { "id": "org.fwupd.hsi.Iommu", "summary": "IOMMU", "level": 2,
                    "result": "not-enabled", "passes_as": "enabled", "passed": false,
                    "flags": ["action-config-fw", "can-fix"],
                    "bios_setting": { "id": "iommu", "current": "Off", "target": "On" } },
                { "id": "org.fwupd.hsi.Lockdown", "summary": "Lockdown", "result": "not-enabled",
                    "passed": false, "flags": ["runtime-issue"] },
                { "id": "org.fwupd.hsi.Old", "summary": "Old", "level": 1, "passed": false,
                    "flags": ["obsoleted"] }] })
    }

    fn row<'a>(ready: &'a Ready, key: &str) -> &'a Value {
        ready
            .rows
            .iter()
            .find(|row| row["key"] == key)
            .unwrap_or_else(|| panic!("no {key} among {:?}", ready.rows))
    }

    fn read_of(records: &[Value]) -> Ready {
        read(records, &Acting::default(), &words())
    }

    #[test]
    fn each_device_is_listed_with_an_update_first_and_two_of_a_name_numbered() {
        let hidden = json!({ "key": "firmware:h", "kind": "firmware", "id": "h",
            "name": "Flash Region", "version": "1", "flags": ["updatable-hidden"] });
        let ready = read_of(&[dock(1), hidden, bios(), dock(2)]);
        let names: Vec<&Value> = ready
            .rows
            .iter()
            .filter(|row| row["depth"] == 0)
            .map(|row| &row["name"])
            .collect();
        assert_eq!(
            names,
            [
                "System Firmware",
                "USB-C Controller 1",
                "USB-C Controller 2"
            ]
        );
        let head = ready.head.as_ref().unwrap();
        assert_eq!(
            (head["said"].clone(), head["hue"].clone()),
            (json!("1 update ready"), json!("warning"))
        );
        let system = row(&ready, "row:firmware:b10");
        assert_eq!(
            system["side"],
            json!([{ "said": "1.13.0 ready \u{b7} from 1.12.0" }])
        );
        assert_eq!(system["hue"], "warning");
        assert!(
            read_of(&[host()]).head.is_none(),
            "no device lists no Firmware"
        );
    }

    #[test]
    fn install_asks_once_and_says_how_it_stands() {
        let ready = read_of(&[bios()]);
        let install = row(&ready, "row:firmware:b10:install");
        assert_eq!(install["name"], "Install 1.13.0");
        assert_eq!(
            install["acts"],
            json!([{ "key": "verb:more", "step": "rows" }])
        );
        let sure = row(&ready, "row:firmware:b10:install:sure");
        assert_eq!(sure["sure"], true);
        assert_eq!(sure["name"], "Install 1.13.0 on System Firmware Now");
        assert_eq!(
            sure["note"],
            "it can't be undone, and finishes at a restart"
        );
        assert_eq!(
            sure["acts"][0]["ask"],
            json!({ "ask": "act", "act": "firmware-install",
            "device": "b10", "patience_s": 2400 }),
            "twice its 20 min, past 30"
        );
        let mut acting = Acting {
            going: Some("install:b10".to_owned()),
            ..Acting::default()
        };
        let going = read(&[bios()], &acting, &words());
        assert_eq!(
            row(&going, "row:firmware:b10:install")["note"],
            "installing"
        );
        assert!(
            going.rows.iter().all(|row| row["sure"] != true),
            "nothing more is asked"
        );
        acting.going = None;
        acting.ended.insert(
            "install:b10".to_owned(),
            Ended::Refused("\nNo AC power.\n".into()),
        );
        let refused = read(&[bios()], &acting, &words());
        assert_eq!(
            row(&refused, "row:firmware:b10:install")["note"],
            "didn't install \u{b7} no AC power"
        );
        let mut waiting = bios();
        waiting["last_update"] = json!({ "state": "needs-reboot", "version": "1.13.0" });
        waiting["update"] = Value::Null;
        let waits = read_of(&[waiting]);
        assert_eq!(
            row(&waits, "row:firmware:b10")["side"][0]["said"],
            "1.13.0 at the restart \u{b7} from 1.12.0"
        );
        assert_eq!(
            row(&waits, "row:firmware:b10:install")["note"],
            "restart to finish"
        );
    }

    #[test]
    fn an_update_held_back_says_why_beside_its_version() {
        let mut held = bios();
        held["problems"] = json!(["update-pending", "require-ac-power"]);
        let ready = read_of(&[held]);
        assert_eq!(
            row(&ready, "row:firmware:b10")["side"],
            json!([
            { "said": "1.13.0 ready \u{b7} from 1.12.0", "keep": 2 },
            { "said": "no charger", "keep": 1 }])
        );
        let install = row(&ready, "row:firmware:b10:install");
        assert_eq!(
            (install["note"].clone(), install["acts"].clone()),
            (json!("no charger"), json!([]))
        );
    }

    #[test]
    fn a_devices_drawer_says_what_fwupd_does() {
        let ready = read_of(&[bios()]);
        let drawer = &row(&ready, "row:firmware:b10")["drawer"];
        assert_eq!(drawer["eyebrow"], "from Dell \u{b7} firmware");
        assert_eq!(
            drawer["facts"],
            json!([
            { "name": "Last Updated", "at": 1_750_000_000_000_u64, "as": "day" },
            { "name": "Size", "said": "1.52 MB" },
            { "name": "Released", "at": 1_761_000_000_000_u64, "as": "day" },
            { "name": "Takes", "said": "20 min" },
            { "name": "Serial", "said": "ABC123", "identifies": true }])
        );
        assert_eq!(drawer["body"], "A fix.\n\nFixes & more:\none\ntwo");
        let unknown = read_of(&[dock(1)]);
        assert_eq!(
            row(&unknown, "row:firmware:d1")["drawer"]["eyebrow"],
            "firmware"
        );
    }

    #[test]
    fn security_says_its_rating_and_lists_each_failing_attribute_first() {
        let ready = read_of(&[bios(), host()]);
        let security = row(&ready, "row:firmware:security");
        assert_eq!(security["side"], json!([{ "said": "HSI:1!" }]));
        assert_eq!(
            security["note"],
            "only the most basic protections \u{b7} lowered by what's running"
        );
        assert_eq!(
            security["drawer"]["facts"][0],
            json!({ "name": "Needed for Level 2", "said": "IOMMU not enabled" })
        );
        assert_eq!(
            security["drawer"]["facts"][1],
            json!({ "name": "Lowered By", "said": "Lockdown not enabled" })
        );
        let inside: Vec<&Value> = ready
            .rows
            .iter()
            .filter(|row| row["parent"] == "row:firmware:security")
            .map(|row| &row["name"])
            .collect();
        assert_eq!(
            inside,
            ["IOMMU", "Lockdown", "TPM v2.0", "Security Level"],
            "failing first, by level, what's running last, the obsoleted left out"
        );
        let iommu = row(&ready, "row:firmware:security:org.fwupd.hsi.Iommu");
        assert_eq!(
            iommu["side"],
            json!([{ "said": "level 2", "keep": 1, "hue": "muted" },
            { "said": "not enabled", "keep": 2 }])
        );
        assert_eq!(
            (iommu["glyph"].clone(), iommu["hue"].clone()),
            (json!("caution"), json!("warning"))
        );
        let fix = row(&ready, "row:firmware:security:org.fwupd.hsi.Iommu:fix:sure");
        assert_eq!(fix["name"], "Secure IOMMU Now");
        assert_eq!(
            fix["acts"][0]["ask"],
            json!({ "ask": "act", "act": "firmware-fix",
            "attribute": "org.fwupd.hsi.Iommu", "patience_s": 120 })
        );
        let setup = row(&ready, "row:firmware:security:org.fwupd.hsi.Iommu:setup");
        assert_eq!(setup["note"], "set iommu to On there");
        let level = row(&ready, "row:firmware:security:level");
        assert_eq!(
            level["acts"][0],
            json!({ "key": "verb:copy", "copy": "HSI:1!" })
        );
    }

    #[test]
    fn a_reason_and_a_rating_read_as_fwupd_wrote_them() {
        assert_eq!(
            reason("Failed\nUEFI capsule refused."),
            "UEFI capsule refused"
        );
        assert_eq!(reason("  \n"), "");
        assert_eq!(
            rating_of("HSI:3 (v2.1.9)"),
            ("HSI:3".to_owned(), Some(3), false)
        );
        assert_eq!(
            rating_of("HSI:INVALID"),
            ("HSI:INVALID".to_owned(), None, false)
        );
        assert_eq!(words_of("a<br/>b &lt;c&gt;"), "a\nb <c>");
        assert_eq!(
            reason("Decompressing\u{2026}\nSystem Firmware cannot be updated: on battery\n\n"),
            "system Firmware cannot be updated: on battery",
            "the last line, its first letter lowered"
        );
        let setup = "Cannot indicate to EFI to boot into setup mode: \
            Firmware does not support it.\n";
        assert_eq!(
            reason(setup),
            "cannot indicate to EFI to boot into setup mode: Firmware does not support it",
            "without its full stop"
        );
        assert_eq!(
            reason("UEFI capsule updates not available"),
            "UEFI capsule updates not available",
            "a word in capitals kept"
        );
        assert_eq!(reason(""), "");
        assert_eq!(rating_of("HSI:4"), ("HSI:4".to_owned(), Some(4), false));
        assert_eq!(
            rating_of("HSI:INVALID:missing-data"),
            ("HSI:INVALID:missing-data".to_owned(), None, false)
        );
    }

    /// The machine's SSD, whose last install failed and which fwupd
    /// offers nothing, saying why.
    fn ssd() -> Value {
        json!({ "key": "firmware:ssd", "kind": "firmware", "id": "ssd",
            "name": "PVC10 SK hynix 1024GB", "vendor": "SK hynix", "version": "61001171",
            "flags": ["internal", "updatable", "needs-reboot"],
            "no_update": "current version is 61001171: 61001171=same",
            "last_update": { "state": "failed", "version": "61001172",
                "error": "device was not in supported mode", "updated_at_s": null } })
    }

    #[test]
    fn a_device_without_a_name_or_a_version_is_left_out_and_its_history_says_why() {
        let spi = json!({ "key": "firmware:spi", "kind": "firmware", "id": "spi",
            "name": "Internal SPI Controller", "version": null });
        let nameless = json!({ "key": "firmware:n", "kind": "firmware", "id": "n",
            "name": null, "version": "1" });
        let pk = json!({ "key": "firmware:pk", "kind": "firmware", "id": "pk",
            "name": "Platform Key", "vendor": "Unknown", "version": "2026" });
        let ready = read_of(&[ssd(), spi, bios(), nameless, pk.clone()]);
        let names: Vec<&Value> = ready
            .rows
            .iter()
            .filter(|row| row["depth"] == 0)
            .map(|row| &row["name"])
            .collect();
        assert_eq!(
            names,
            ["System Firmware", "PVC10 SK hynix 1024GB", "Platform Key"],
            "the one offered first, then as fwupd lists them"
        );
        let drive = row(&ready, "row:firmware:ssd");
        assert_eq!(
            drive["side"],
            json!([{ "said": "61001171" }]),
            "its version alone"
        );
        assert_eq!(
            drive["drawer"]["facts"],
            json!([
                { "name": "firmware:none", "said": "current version is 61001171: 61001171=same" },
                { "name": "firmware:failed", "said": "device was not in supported mode" }]),
            "why nothing's offered and why its last install failed, and no day it never had"
        );
        assert_eq!(
            row(&ready, "row:firmware:pk")["drawer"]["eyebrow"],
            "firmware",
            "a vendor fwupd doesn't know is none"
        );
        let head = |records: &[Value]| read_of(records).head.unwrap();
        let current = head(&[ssd(), pk]);
        assert_eq!(
            (current["said"].clone(), current.get("hue").cloned()),
            (json!("up to date"), None)
        );
        let mut other = bios();
        other["id"] = json!("b11");
        other["key"] = json!("firmware:b11");
        assert_eq!(head(&[bios(), other])["said"], "2 updates ready");
        assert!(
            ready
                .rows
                .iter()
                .all(|row| row["key"] != "row:firmware:security"),
            "no rating before fwupd gives one"
        );
    }

    #[test]
    fn a_devices_drawer_says_how_it_is_flashed_and_how_urgent_its_update_is() {
        let facts = |record: Value| {
            row(&read_of(&[record]), "row:firmware:b10")["drawer"]["facts"]
                .as_array()
                .unwrap()
                .clone()
        };
        let mut flashed = bios();
        flashed["version_lowest"] = json!("1.2.0");
        flashed["plugin"] = json!("uefi_capsule");
        flashed["protocols"] = json!(["org.uefi.capsule", "com.dell.dock"]);
        flashed["update"]["urgency"] = json!("high");
        let said = facts(flashed);
        let fact = |name: &str, said: &str| json!({ "name": name, "said": said });
        assert_eq!(
            said[0],
            fact("firmware:lowest", "1.2.0"),
            "the lowest it takes first"
        );
        assert!(
            said.contains(&fact("firmware:urgency", "high")),
            "how urgent fwupd says it is"
        );
        assert!(
            said.contains(&fact(
                "firmware:protocols",
                "org.uefi.capsule \u{b7} com.dell.dock"
            )),
            "each way it's flashed"
        );
        assert!(
            said.contains(&json!({ "name": "firmware:plugin", "said": "uefi_capsule",
                "identifies": true })),
            "the plugin, as a name"
        );
        assert!(
            facts(bios())
                .iter()
                .all(|fact| fact["name"] != "firmware:urgency"),
            "an urgency fwupd doesn't know is none"
        );
    }

    #[test]
    fn an_update_waits_for_the_restart_until_it_is_flashed() {
        let waiting = |state: &str, version: &str| {
            let mut record = bios();
            record["version"] = json!(version);
            record["update"] = Value::Null;
            record["problems"] = json!(["update-pending"]);
            record["last_update"] = json!({ "state": state, "version": "1.13.0" });
            read_of(&[record])
        };
        let side = |ready: &Ready| row(ready, "row:firmware:b10")["side"].clone();
        assert_eq!(
            side(&waiting("pending", "1.12.0")),
            json!([{ "said": "1.13.0 at the restart \u{b7} from 1.12.0" }]),
            "pending, and held back by nothing"
        );
        let pending = waiting("pending", "1.12.0");
        let install = row(&pending, "row:firmware:b10:install");
        assert_eq!(
            (install["gone"].clone(), install["acts"].clone()),
            (json!(true), json!([])),
            "an install waiting for the restart is done with, asking nothing"
        );
        assert_eq!(
            side(&waiting("failed", "1.12.0")),
            json!([{ "said": "1.12.0" }]),
            "one that failed doesn't wait"
        );
        assert_eq!(
            side(&waiting("pending", "1.13.0")),
            json!([{ "said": "1.13.0" }]),
            "flashed at the restart, it waits no more"
        );
        let locked = json!({ "key": "firmware:pk", "kind": "firmware", "id": "pk",
            "name": "Platform Key", "version": "1", "problems": ["firmware-locked"] });
        assert_eq!(
            row(&read_of(&[locked]), "row:firmware:pk")["side"],
            json!([{ "said": "1" }]),
            "nothing held back with nothing offered"
        );
    }

    /// An attribute named for its id, passing or not, with `fields` over
    /// what fwupd says of one.
    fn attribute(id: &str, level: Option<u64>, passed: bool, fields: Value) -> Value {
        let mut one = json!({ "id": id, "summary": format!("{id} summary"), "level": level,
            "result": if passed { "enabled" } else { "not-enabled" },
            "passes_as": "enabled", "passed": passed,
            "flags": if passed { json!(["success"]) } else { json!([]) } });
        if let Value::Object(fields) = fields {
            for (field, value) in fields {
                one[field] = value;
            }
        }
        one
    }

    fn rated(security: &str, attributes: Vec<Value>) -> Value {
        json!({ "key": "firmware:host", "kind": "firmware", "security": security,
            "attributes": Value::Array(attributes) })
    }

    /// A machine at level 1, lowered, with an attribute of each kind.
    fn counted() -> Value {
        rated(
            "HSI:1! (v2.1.9)",
            vec![
                attribute("tpm", Some(1), true, json!({})),
                attribute(
                    "lockdown",
                    None,
                    false,
                    json!({ "flags": ["runtime-issue", "action-config-os"] }),
                ),
                attribute(
                    "rollback",
                    Some(2),
                    false,
                    json!({ "flags": ["action-config-fw"],
                "bios_setting": { "id": "AllowBiosDowngrade", "current": "Enabled" } }),
                ),
                attribute(
                    "mtd",
                    Some(2),
                    false,
                    json!({ "result": "not-supported", "flags": ["missing-data"] }),
                ),
                attribute("iommu", Some(2), true, json!({})),
                attribute(
                    "cet",
                    None,
                    true,
                    json!({ "flags": ["success", "runtime-issue"] }),
                ),
                attribute("smap", Some(4), false, json!({ "summary": "" })),
                attribute(
                    "swap",
                    None,
                    false,
                    json!({
                "flags": ["runtime-issue", "can-fix", "can-undo"],
                "kernel": { "current": "none", "target": "integrity" } }),
                ),
                attribute(
                    "hidden",
                    Some(1),
                    true,
                    json!({ "flags": ["success", "can-undo"] }),
                ),
            ],
        )
    }

    #[test]
    fn security_lists_each_failing_attribute_first_and_what_lowers_it() {
        let ready = read_of(&[bios(), counted()]);
        let inside: Vec<&Value> = ready
            .rows
            .iter()
            .filter(|row| row["parent"] == "row:firmware:security")
            .map(|row| &row["name"])
            .collect();
        assert_eq!(
            inside,
            [
                "rollback summary",
                "mtd summary",
                "smap",
                "lockdown summary",
                "swap summary",
                "tpm summary",
                "hidden summary",
                "iommu summary",
                "cet summary",
                "Security Level"
            ],
            "failing first, each part by level, what's running last, a name without a summary"
        );
        let facts = &row(&ready, "row:firmware:security")["drawer"]["facts"];
        let needed = "rollback summary not enabled \u{b7} mtd summary not-supported \u{b7} unread";
        assert_eq!(
            facts[0],
            json!({ "name": "Needed for Level 2", "said": needed }),
            "the next level's failing attributes, one fwupd couldn't read included"
        );
        assert_eq!(
            facts[1],
            json!({ "name": "Lowered By",
                "said": "lockdown summary not enabled \u{b7} swap summary not enabled" }),
            "what's running that fails, and not what passes"
        );
    }

    #[test]
    fn an_attributes_drawer_and_steps_say_what_changes_it() {
        let ready = read_of(&[bios(), counted()]);
        let drawer = |id: &str| {
            row(&ready, &format!("row:firmware:security:{id}"))["drawer"]["facts"].clone()
        };
        assert_eq!(
            drawer("rollback")[2],
            json!({ "name": "Firmware Setting", "said": "AllowBiosDowngrade", "identifies": true }),
            "its firmware setting, after what passing finds and where it's fixed"
        );
        let kernel = json!({ "name": "firmware:kernel-should-be", "said": "integrity" });
        assert!(
            drawer("swap").as_array().unwrap().contains(&kernel),
            "the kernel's setting to pass"
        );
        let keys: Vec<&str> = ready
            .rows
            .iter()
            .filter_map(|row| row["key"].as_str())
            .collect();
        assert!(
            keys.contains(&"row:firmware:security:swap:fix")
                && !keys.contains(&"row:firmware:security:swap:undo"),
            "a failing attribute fwupd can change is secured, not undone"
        );
        assert!(
            keys.contains(&"row:firmware:security:hidden:undo"),
            "a passing one fwupd can put back is undone"
        );
    }

    #[test]
    fn only_a_runtime_issue_lowers_and_nothing_is_needed_past_the_top() {
        let quiet = read_of(&[
            bios(),
            rated(
                "HSI:2! (v2.1.9)",
                vec![
                    attribute(
                        "updates",
                        None,
                        false,
                        json!({ "flags": ["runtime-updates"] }),
                    ),
                    attribute(
                        "attestation",
                        None,
                        false,
                        json!({ "flags": ["runtime-attestation"] }),
                    ),
                ],
            ),
        ]);
        assert_eq!(
            row(&quiet, "row:firmware:security")["drawer"]["facts"],
            json!([]),
            "only a runtime issue lowers the level"
        );
        let top = read_of(&[
            bios(),
            rated("HSI:5", vec![attribute("later", Some(6), false, json!({}))]),
        ]);
        assert_eq!(
            row(&top, "row:firmware:security")["drawer"]["facts"],
            json!([]),
            "nothing is needed past the top level"
        );
    }

    #[test]
    fn an_act_waited_for_too_long_says_how_long() {
        let device = |minutes: Value| {
            let mut record = bios();
            record["update"]["install_minutes"] = minutes;
            install_patience(&device_of(&record))
        };
        assert_eq!(device(json!(25)), 50, "twice what fwupd says it takes");
        assert_eq!(device(json!(3)), 30, "never under the least");
        assert_eq!(
            device(Value::Null),
            30,
            "the least, where fwupd gives no length"
        );
        let mut acting = Acting::default();
        acting
            .ended
            .insert("install:b10".to_owned(), Ended::Stopped(50));
        let stopped = read(&[bios()], &acting, &words());
        assert_eq!(
            row(&stopped, "row:firmware:b10:install")["note"],
            "didn't install \u{b7} stopped waiting after 50 min",
            "how long it was waited for, not fwupdmgr's words"
        );
        let failing = json!({ "key": "firmware:host", "kind": "firmware",
            "security": "HSI:1", "attributes": [{ "id": "a", "summary": "A", "level": 2,
                "result": "not-enabled", "passed": false, "flags": ["can-fix"] }] });
        acting.ended.clear();
        acting.ended.insert("fix:a".to_owned(), Ended::Stopped(2));
        let fix = read(&[bios(), failing.clone()], &acting, &words());
        assert_eq!(
            row(&fix, "row:firmware:security:a:fix")["note"],
            "didn't secure \u{b7} stopped waiting after 2 min"
        );
        acting.ended.insert("fix:a".to_owned(), Ended::Done);
        let done = read(&[bios(), failing], &acting, &words());
        let secured = row(&done, "row:firmware:security:a:fix");
        assert_eq!(
            (
                secured["note"].clone(),
                secured["gone"].clone(),
                secured["acts"].clone()
            ),
            (json!("secured"), json!(true), json!([])),
            "a change that's done is done with, asking nothing"
        );
        assert_eq!(
            row(&fix, "row:firmware:security:a:fix").get("gone"),
            None,
            "one that stopped may be tried again"
        );
    }
}
