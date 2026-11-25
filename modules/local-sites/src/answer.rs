//! An answer Joel presses on one of the sites' own pages, carried to the
//! session that asked, as the launcher's For You carries one: the steer
//! `{act: "answer", look, value, from: "joel", at}`, sent over the node to
//! the session that left the look, which checks it against the look again
//! and takes only the first answer (the harness's mesh steering).
//!
//! The session can't tell who wrote the steer, so the whole proof that Joel
//! pressed it rests here, and an answer is carried only when all of these
//! hold, each refused with a word the journal keeps:
//! - its `Origin` is this server's own, at the name it was asked by, and
//!   the browser says it came from this origin (`Sec-Fetch-Site`), so no
//!   other site's page, nor a document, which runs in an origin of no
//!   one's (http.rs), sent it;
//! - it carries the cookie only a page of the sites' own is sent, which no
//!   script reads and no other site's page sends, minted fresh each time
//!   the server starts;
//! - the look, read again from the looks' daemon as it stands now, waits on
//!   him and offers the value, as the kit's ask.ts and For You read one;
//! - no answer to that look was carried before, since the server started.
//!
//! The server asks no more of a page's answer than that one value: what it
//! sends is built here, never taken from the page.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::page;

/// The cookie a page of the sites' own carries its answers with.
const COOKIE: &str = "sites-answer";

/// How many of a look's choices are offered, as ask.ts's and Looks.js's `OFFERED`.
const OFFERED: usize = 4;

/// How long after it came a waiting look goes quiet, three days, as ask.ts's `QUIET_AFTER`.
const QUIET_AFTER: i64 = 259_200_000;

/// What a look can be about, as ask.ts's `KINDS`.
const KINDS: [&str; 5] = ["pr", "page", "file", "diff", "terminal"];

/// The most an answer's body may run to, far past any look's id and value.
pub const LONGEST: usize = 4096;

/// Where answers are checked and sent, and what was carried.
pub struct Answers {
    /// The looks' daemon (the to-do daemon), asked for a look as it stands.
    looks: PathBuf,
    /// The node, which carries a steer to the session it names.
    node: PathBuf,
    /// The cookie's value this run, which only a page of the sites' own is sent.
    secret: String,
    /// The looks answered since the server started, each refused a second.
    answered: Mutex<BTreeSet<String>>,
}

/// What a request to answer says of itself.
pub struct Asked<'a> {
    /// The name it reached the server by, which the door has checked.
    pub host: &'a str,
    pub origin: Option<&'a str>,
    /// `Sec-Fetch-Site`, as the browser writes it.
    pub fetch_site: Option<&'a str>,
    pub cookies: Option<&'a str>,
    pub body: &'a [u8],
}

/// Why an answer wasn't carried: a word for the journal and the page, the
/// status it's answered with, and a sentence.
#[derive(Debug, PartialEq, Eq)]
pub struct Refused {
    pub word: &'static str,
    pub status: u16,
    pub why: String,
}

impl Answers {
    /// Answers checked against the looks at `looks` and sent through the
    /// node at `node`, with a cookie drawn fresh.
    pub fn new(looks: PathBuf, node: PathBuf) -> io::Result<Self> {
        let mut drawn = [0_u8; 16];
        File::open("/dev/urandom")?.read_exact(&mut drawn)?;
        Ok(Self::holding(looks, node, hex(&drawn)))
    }

    fn holding(looks: PathBuf, node: PathBuf, secret: String) -> Self {
        Self {
            looks,
            node,
            secret,
            answered: Mutex::new(BTreeSet::new()),
        }
    }

    /// The cookie a page of the sites' own is sent with, so its answers say
    /// where they came from: sent back only to `/answer`, by this site
    /// alone, and read by no script.
    pub fn cookie(&self) -> String {
        format!(
            "{COOKIE}={}; Path=/answer; SameSite=Strict; HttpOnly",
            self.secret
        )
    }

    /// Carries `asked` to the session that asked, answering what the node
    /// said, or why it wasn't carried.
    pub fn carry(&self, asked: &Asked) -> Result<Value, Refused> {
        self.pressed_here(asked)?;
        let (id, value) = said(asked.body)?;
        // Held while the answer goes, so two presses can't both be carried.
        let mut answered = self
            .answered
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if answered.contains(&id) {
            return Err(refused("answered", 409, "this look was answered already"));
        }
        let look = self.look(&id)?;
        let now = now();
        offered(&look, &value, now)?;
        let address = look
            .get("address")
            .and_then(Value::as_str)
            .filter(|address| !address.is_empty())
            .ok_or_else(|| refused("unaddressed", 409, "the look names no session to answer"))?;
        let steer = json!({
            "ask": "steer",
            "address": address.to_uppercase(),
            "steer": {"at": iso(now), "from": "joel", "act": "answer", "look": id, "value": value},
        });
        let told = page::once(&self.node, &steer)
            .map_err(|why| refused("unreached", 502, &format!("the node doesn't answer: {why}")))?;
        if let Some(error) = told.get("error") {
            let why = error.get("message").and_then(Value::as_str).unwrap_or("");
            return Err(refused("unsteered", 502, &format!("the node refused it: {why}")));
        }
        answered.insert(id.clone());
        Ok(json!({"answered": id, "steered": told.get("steered").cloned().unwrap_or(Value::Null)}))
    }

    /// Whether `asked` came from a page of the sites' own, open now.
    fn pressed_here(&self, asked: &Asked) -> Result<(), Refused> {
        let ours = format!("http://{}", asked.host);
        if asked.origin != Some(ours.as_str()) {
            return Err(refused(
                "origin",
                403,
                "an answer comes only from a page of the sites' own",
            ));
        }
        if asked.fetch_site != Some("same-origin") {
            return Err(refused("site", 403, "an answer comes only from this origin"));
        }
        let held = asked
            .cookies
            .into_iter()
            .flat_map(|cookies| cookies.split(';'))
            .filter_map(|cookie| cookie.trim().strip_prefix(COOKIE)?.strip_prefix('='))
            .any(|value| same(value, &self.secret));
        if !held {
            return Err(refused(
                "cookie",
                403,
                "an answer comes only from a page this server sent since it started",
            ));
        }
        Ok(())
    }

    /// The look `id` as the looks' daemon holds it now.
    fn look(&self, id: &str) -> Result<Value, Refused> {
        let key = format!("look:{id}");
        let asked = json!({"ask": "get", "keys": [key]});
        let told = page::once(&self.looks, &asked)
            .map_err(|why| refused("unreached", 502, &format!("the looks don't answer: {why}")))?;
        told.get("records")
            .and_then(Value::as_array)
            .and_then(|records| {
                records
                    .iter()
                    .find(|record| record.get("key").and_then(Value::as_str) == Some(key.as_str()))
            })
            .and_then(|record| record.get("look"))
            .cloned()
            .ok_or_else(|| refused("unknown", 404, "no look of that id is held"))
    }
}

fn refused(word: &'static str, status: u16, why: &str) -> Refused {
    Refused {
        word,
        status,
        why: why.to_owned(),
    }
}

/// The look and the value an answer's body names, each a word or more.
fn said(body: &[u8]) -> Result<(String, String), Refused> {
    let unreadable = || refused("unreadable", 400, "an answer names its look and its value");
    let said: Value = serde_json::from_slice(body).map_err(|_| unreadable())?;
    let text = |field: &str| {
        said.get(field)
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    };
    Ok((
        text("look").ok_or_else(unreadable)?,
        text("value").ok_or_else(unreadable)?,
    ))
}

/// Whether `look` waits on him at `now`, in milliseconds since the epoch,
/// and offers `value`, as ask.ts's `answering` reads it: whole, waiting,
/// asking, not past its `expiresAt` nor quiet three days after it came, and
/// `value` one of its first four choices' values, exactly.
fn offered(look: &Value, value: &str, now: i64) -> Result<(), Refused> {
    let text = |field: &str| look.get(field).and_then(Value::as_str).unwrap_or("");
    let whole = look.get("v") == Some(&json!(1))
        && ["id", "title", "target"]
            .iter()
            .all(|field| !text(field).is_empty())
        && KINDS.contains(&text("kind"));
    if !whole {
        return Err(refused("unwhole", 409, "the look isn't one For You reads"));
    }
    if text("state") != "waiting" {
        return Err(refused("ended", 409, "the look waits on nothing now"));
    }
    if look.get("asks") == Some(&Value::Bool(false)) {
        return Err(refused("unasked", 409, "the look asks nothing"));
    }
    if when(look.get("expiresAt")).is_some_and(|ends| now >= ends) {
        return Err(refused("lapsed", 409, "the look stopped waiting"));
    }
    if when(look.get("at")).is_some_and(|came| now - came > QUIET_AFTER) {
        return Err(refused("quiet", 409, "the look went quiet"));
    }
    let offers = look
        .get("choices")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|choice| choice.get("value").and_then(Value::as_str))
        .filter(|offer| !offer.is_empty())
        .take(OFFERED)
        .any(|offer| offer == value);
    if !offers {
        return Err(refused("unoffered", 409, "the look doesn't offer that"));
    }
    Ok(())
}

/// When `value` falls, in milliseconds since the epoch: a whole number of
/// them, or an ISO time naming its zone, as a session writes one; else none.
fn when(value: Option<&Value>) -> Option<i64> {
    match value? {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => parsed(text),
        _ => None,
    }
}

/// An ISO time, `2026-11-24T23:10:00.123Z` or with an offset, in
/// milliseconds since the epoch; none where it names no zone, which a
/// browser reads as its own and this server can't know.
fn parsed(text: &str) -> Option<i64> {
    let (date, rest) = text.split_once('T')?;
    let mut parts = date.split('-').map(str::parse::<i64>);
    let (year, month, day) = (parts.next()?.ok()?, parts.next()?.ok()?, parts.next()?.ok()?);
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let (clock, offset) = match rest.strip_suffix(['Z', 'z']) {
        Some(clock) => (clock, 0),
        None => {
            let at = rest.rfind(['+', '-'])?;
            let (clock, zone) = rest.split_at(at);
            let sign = if zone.starts_with('-') { -1 } else { 1 };
            let (hours, minutes) = zone.get(1..)?.split_once(':')?;
            let minutes = hours.parse::<i64>().ok()? * 60 + minutes.parse::<i64>().ok()?;
            (clock, sign * minutes)
        }
    };
    let (whole, fraction) = clock.split_once('.').unwrap_or((clock, ""));
    let mut parts = whole.split(':').map(str::parse::<i64>);
    let (hour, minute, second) = (parts.next()?.ok()?, parts.next()?.ok()?, parts.next()?.ok()?);
    if parts.next().is_some() || !fraction.bytes().all(|digit| digit.is_ascii_digit()) {
        return None;
    }
    let millis: String = fraction.chars().chain(std::iter::repeat('0')).take(3).collect();
    let millis = millis.parse::<i64>().ok()?;
    let days = days_from_civil(year, month, day);
    let seconds = ((days * 24 + hour) * 60 + minute - offset) * 60 + second;
    Some(seconds * 1000 + millis)
}

/// The days from 1970-01-01 to a day of the proleptic Gregorian calendar,
/// by Howard Hinnant's `days_from_civil`.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let shifted = if month > 2 { month - 3 } else { month + 9 };
    let of_year = (153 * shifted + 2) / 5 + day - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * 146_097 + of_cycle - 719_468
}

/// The day `days` after 1970-01-01, as its year, month and day, by Howard
/// Hinnant's `civil_from_days`.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let of_era = days - era * 146_097;
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted = (5 * of_year + 2) / 153;
    let day = of_year - (153 * shifted + 2) / 5 + 1;
    let month = if shifted < 10 { shifted + 3 } else { shifted - 9 };
    (year_of_era + era * 400 + i64::from(month <= 2), month, day)
}

/// `millis` since the epoch as an ISO time in UTC, as the shell writes `at`.
fn iso(millis: i64) -> String {
    let (days, of_day) = (millis.div_euclid(86_400_000), millis.rem_euclid(86_400_000));
    let (year, month, day) = civil_from_days(days);
    let (hour, minute) = (of_day / 3_600_000, of_day / 60_000 % 60);
    let (second, milli) = (of_day / 1000 % 60, of_day % 1000);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{milli:03}Z")
}

/// Now, in milliseconds since the epoch.
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| i64::try_from(since.as_millis()).unwrap_or(i64::MAX))
}

/// `bytes` written in hexadecimal.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Whether `one` is `other`, in a time that doesn't say how much matched.
fn same(one: &str, other: &str) -> bool {
    one.len() == other.len()
        && one
            .bytes()
            .zip(other.bytes())
            .fold(0, |differ, (a, b)| differ | (a ^ b))
            == 0
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    const NOW: i64 = 1_764_025_800_000;

    fn answers() -> Answers {
        Answers::holding(
            Path::new("/nowhere/looks.sock").to_path_buf(),
            Path::new("/nowhere/node.sock").to_path_buf(),
            "c0ffee".to_owned(),
        )
    }

    fn pressed<'a>(origin: Option<&'a str>, site: Option<&'a str>, cookies: Option<&'a str>) -> Asked<'a> {
        Asked {
            host: "sites.localhost:8484",
            origin,
            fetch_site: site,
            cookies,
            body: br#"{"look": "l1", "value": "yes"}"#,
        }
    }

    fn look() -> Value {
        json!({
            "v": 1, "id": "l1", "title": "A PR", "target": "https://example.com/pr/1",
            "kind": "pr", "state": "waiting", "at": iso(NOW - 60_000), "address": "abc123",
            "choices": [{"value": "yes", "label": "Merge It"}, {"value": "no"}],
        })
    }

    #[test]
    fn an_answer_from_anywhere_but_a_page_of_the_sites_own_is_refused() {
        let answers = answers();
        let ours = Some("http://sites.localhost:8484");
        let cookie = Some("theme=dark; sites-answer=c0ffee");
        for (asked, word) in [
            (pressed(None, Some("same-origin"), cookie), "origin"),
            (pressed(Some("null"), Some("same-origin"), cookie), "origin"),
            (pressed(Some("http://localhost:8484"), Some("same-origin"), cookie), "origin"),
            (pressed(Some("http://evil.example"), Some("same-origin"), cookie), "origin"),
            (pressed(ours, Some("cross-site"), cookie), "site"),
            (pressed(ours, None, cookie), "site"),
            (pressed(ours, Some("same-origin"), None), "cookie"),
            (pressed(ours, Some("same-origin"), Some("sites-answer=c0ffef")), "cookie"),
            (pressed(ours, Some("same-origin"), Some("sites-answer=c0ffee0")), "cookie"),
            (pressed(ours, Some("same-origin"), Some("xsites-answer=c0ffee")), "cookie"),
        ] {
            let why = answers.carry(&asked).expect_err("refused");
            assert_eq!(why.word, word, "{}", why.why);
            assert_eq!(why.status, 403, "{word}");
        }
        let pressed_here = pressed(ours, Some("same-origin"), cookie);
        let why = answers.carry(&pressed_here).expect_err("no daemon stands");
        assert_eq!(why.word, "unreached", "a page's answer passed the door: {}", why.why);
    }

    #[test]
    fn the_cookie_goes_only_to_answers_and_to_no_script() {
        let cookie = answers().cookie();
        assert_eq!(cookie, "sites-answer=c0ffee; Path=/answer; SameSite=Strict; HttpOnly");
        let drawn = Answers::new(PathBuf::new(), PathBuf::new()).expect("a cookie drawn");
        assert_eq!(drawn.secret.len(), 32, "the cookie is drawn from 16 bytes");
        assert_ne!(drawn.secret, answers().secret);
    }

    #[test]
    fn a_body_names_its_look_and_value_or_is_refused() {
        assert_eq!(said(br#"{"look": "l1", "value": "yes"}"#), Ok(("l1".to_owned(), "yes".to_owned())));
        let bodies: [&[u8]; 4] = [
            b"yes",
            br#"{"look": "l1"}"#,
            br#"{"look": "", "value": "yes"}"#,
            br#"{"look": 1, "value": "yes"}"#,
        ];
        for body in bodies {
            let why = said(body).expect_err("refused");
            assert_eq!((why.word, why.status), ("unreadable", 400));
        }
    }

    #[test]
    fn a_look_that_waits_and_offers_the_value_takes_it() {
        assert_eq!(offered(&look(), "yes", NOW), Ok(()));
        assert_eq!(offered(&look(), "no", NOW), Ok(()), "a choice with no label is offered by its value");
    }

    #[test]
    fn a_look_that_doesnt_wait_or_offer_the_value_refuses_it() {
        let changed = |field: &str, to: Value| {
            let mut look = look();
            look[field] = to;
            look
        };
        let many = json!([{"value": "a"}, {"value": "b"}, {"value": "c"}, {"value": "d"}, {"value": "yes"}]);
        for (look, value, word) in [
            (look(), "maybe", "unoffered"),
            (look(), "Merge It", "unoffered"),
            (changed("choices", many), "yes", "unoffered"),
            (changed("v", json!(2)), "yes", "unwhole"),
            (changed("kind", json!("session")), "yes", "unwhole"),
            (changed("title", json!("")), "yes", "unwhole"),
            (changed("state", json!("opened")), "yes", "ended"),
            (changed("state", json!("resolved")), "yes", "ended"),
            (changed("asks", json!(false)), "yes", "unasked"),
            (changed("expiresAt", json!(iso(NOW))), "yes", "lapsed"),
            (changed("expiresAt", json!(NOW - 1)), "yes", "lapsed"),
            (changed("at", json!(iso(NOW - QUIET_AFTER - 1))), "yes", "quiet"),
        ] {
            let why = offered(&look, value, NOW).expect_err(word);
            assert_eq!(why.word, word, "{}", why.why);
        }
        let unknown_zone = changed("expiresAt", json!("2020-01-01T00:00:00"));
        assert_eq!(offered(&unknown_zone, "yes", NOW), Ok(()), "a time with no zone says nothing");
    }

    #[test]
    fn a_time_reads_as_a_browser_reads_it_and_is_written_as_the_shell_writes_it() {
        assert_eq!(parsed("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parsed("2000-03-01T00:00:00.5Z"), Some(951_868_800_500));
        assert_eq!(parsed("2025-11-24T23:10:00.000Z"), Some(NOW));
        assert_eq!(parsed("2025-11-24T18:10:00-05:00"), Some(NOW), "an offset is taken off");
        assert_eq!(parsed("2025-11-25T00:40:00+01:30"), Some(NOW));
        assert_eq!(iso(NOW), "2025-11-24T23:10:00.000Z");
        assert_eq!(iso(951_868_800_500), "2000-03-01T00:00:00.500Z");
        for day in [-800_000, -1, 0, 59, 11_016, 20_416, 2_000_000] {
            assert_eq!(days_from_civil_of(civil_from_days(day)), day, "day {day}");
        }
        for unread in ["2025-11-24", "2025-11-24T23:10:00", "2025-13-01T00:00:00Z", "x-y-zT00:00:00Z", "2025-11-24T23:10Z"] {
            assert_eq!(parsed(unread), None, "{unread}");
        }
    }

    fn days_from_civil_of((year, month, day): (i64, i64, i64)) -> i64 {
        days_from_civil(year, month, day)
    }

    #[test]
    fn two_cookies_are_told_apart_whatever_they_share() {
        assert!(same("c0ffee", "c0ffee"));
        assert!(!same("c0ffee", "c0ffef"));
        assert!(!same("c0ffee", "c0ffe"));
        assert!(!same("", "c0ffee"));
    }
}
