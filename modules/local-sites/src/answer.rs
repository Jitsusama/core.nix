//! An answer Joel presses on one of the sites' own pages, carried to the
//! session that asked, as the launcher's For You carries one: the steer
//! `{act: "answer", look, value, from: "joel", at}`, sent over the node to
//! the session that left the look, or once that has ended to its quest's
//! own session (`answerers`), which checks it against the look again and
//! takes only the first answer (the harness's mesh steering).
//!
//! The session can't tell who wrote the steer, so the whole proof that Joel
//! pressed it rests here, and an answer is carried only when all of these
//! hold, each refused with a word the journal keeps:
//! - its `Origin` is this server's own, at the name it was asked by, and
//!   the browser says it came from this origin (`Sec-Fetch-Site`), so no
//!   other site's page, nor a document, which runs in the documents' own
//!   origin, same-site but never same-origin (http.rs), sent it;
//! - it says it's JSON (`Content-Type`), which no form can send, so an
//!   injected form can't press;
//! - it carries the cookie only a page of the sites' own is given, which no
//!   script reads and no other site's page sends, minted fresh each time
//!   the server starts and given only to the person's own process;
//! - the process at the connection's other end is the person's, as the
//!   kernel tells it (peer.rs), since every header above is an agent's to
//!   forge from the loopback network a fence shares;
//! - the look, read again from the looks' daemon as it stands now, waits on
//!   him and offers the value, as the kit's ask.ts and For You read one;
//! - no answer to that look was carried before, since the server started.
//!
//! Asking the kernel who pressed walks every descriptor the account holds,
//! and any process can ask it as fast as it connects, so it's paced
//! (`WALKS`), refused "busy" past that; so are the journal's refusals
//! (`REFUSALS`), each line let through saying how many weren't.
//!
//! The server asks no more of a page's answer than that one value: what it
//! sends is built here, never taken from the page, and says it came by the
//! sites (`via`).

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{self, Read};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use serde_json::{Value, json};

use crate::clock::{iso, now, when};
use crate::pace::{Pace, Turn};
use crate::page;
use crate::peer::Told;
use crate::posing;

/// Who holds the other end of a connection, by its two ends: the browser's,
/// then the server's.
pub type Teller = Box<dyn Fn(SocketAddr, SocketAddr) -> Told + Send + Sync>;

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

/// How often the kernel is asked who pressed: eight at once, far past a
/// page's one ask on loading and one on pressing, then four a second.
const WALKS: (u32, Duration) = (8, Duration::from_millis(250));

/// How often a refusal is journalled: sixteen at once, then one a second.
const REFUSALS: (u32, Duration) = (16, Duration::from_secs(1));

/// Where answers are checked and sent, and what was carried.
pub struct Answers {
    /// The looks' daemon (the to-do daemon), asked for a look as it stands.
    looks: PathBuf,
    /// The node, which carries a steer to the session it names.
    node: PathBuf,
    /// The cookie's value this run, which only a page of the sites' own is sent.
    secret: String,
    /// The looks answered since the server started, and those whose answer
    /// is on its way, each refused another.
    claims: Mutex<Claims>,
    /// Who pressed: the person, an agent, or nothing told.
    tell: Teller,
    /// How often `tell` may be asked.
    walks: Pace,
    /// How often a refusal is journalled.
    refusals: Pace,
}

/// The looks whose answers were carried, and those on their way.
#[derive(Default)]
struct Claims {
    answered: BTreeSet<String>,
    going: BTreeSet<String>,
}

/// The words the node refuses a steer with when no session goes by its
/// address now, the one refusal the next session is told in its place for.
/// The node's protocol names these words (mycelium's PROTOCOL.md, under a
/// steer: "as `no session goes by SAK9FZ now`"), so they're its to keep.
const GONE: &str = "no session goes by ";

/// What a request to answer says of itself.
pub struct Asked<'a> {
    /// The name it reached the server by, which the door has checked.
    pub host: &'a str,
    pub origin: Option<&'a str>,
    /// `Sec-Fetch-Site`, as the browser writes it.
    pub fetch_site: Option<&'a str>,
    pub cookies: Option<&'a str>,
    /// What its body is, as `Content-Type` says.
    pub content_type: Option<&'a str>,
    /// The connection's ends, the browser's then the server's.
    pub ends: Option<(SocketAddr, SocketAddr)>,
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
    /// node at `node`, pressed by whom `tell` says, with a cookie drawn fresh.
    pub fn new(looks: PathBuf, node: PathBuf, tell: Teller) -> io::Result<Self> {
        let mut drawn = [0_u8; 16];
        File::open("/dev/urandom")?.read_exact(&mut drawn)?;
        Ok(Self::holding((looks, node), hex(&drawn), tell))
    }

    fn holding((looks, node): (PathBuf, PathBuf), secret: String, tell: Teller) -> Self {
        Self {
            looks,
            node,
            secret,
            claims: Mutex::default(),
            tell,
            walks: Pace::new(WALKS.0, WALKS.1),
            refusals: Pace::new(REFUSALS.0, REFUSALS.1),
        }
    }

    /// Journals `refusal`, unless refusals come too fast to journal each,
    /// saying how many went unjournalled before it.
    pub fn journal(&self, refusal: &str) {
        match self.refusals.take() {
            Turn::Go { turned: 0 } => eprintln!("{refusal}"),
            Turn::Go { turned } => {
                eprintln!("{refusal} ({turned} refused before it went unjournalled)");
            }
            Turn::Wait => {}
        }
    }

    /// The cookie a page of the sites' own asks for once a look waits, so its
    /// answers say where they came from: given only to the person's own
    /// process asking from this origin, sent back only to `/answer` by this
    /// site alone, and read by no script.
    pub fn cookie_for(&self, asked: &Asked) -> Result<String, Refused> {
        if asked.fetch_site != Some("same-origin") {
            return Err(refused("site", 403, "only this origin's pages answer"));
        }
        self.pressed_by_the_person(asked)?;
        Ok(format!(
            "{COOKIE}={}; Path=/answer; SameSite=Strict; HttpOnly",
            self.secret
        ))
    }

    /// Carries `asked` to the session that asked, answering what the node
    /// said, or why it wasn't carried.
    pub fn carry(&self, asked: &Asked) -> Result<Value, Refused> {
        if !is_json(asked.content_type) {
            return Err(refused("type", 415, "an answer is sent as JSON"));
        }
        self.pressed_here(asked)?;
        self.pressed_by_the_person(asked)?;
        let (id, value) = said(asked.body)?;
        self.claim(&id)?;
        let carried = self.steer(&id, &value);
        let mut claims = self.claims.lock().unwrap_or_else(PoisonError::into_inner);
        claims.going.remove(&id);
        if carried.is_ok() {
            claims.answered.insert(id);
        }
        carried
    }

    /// Claims look `id` for this press, so a second can't be carried beside
    /// it, held only as long as it takes to claim: what follows asks the
    /// daemons, and one slow to answer holds up no other look's press.
    fn claim(&self, id: &str) -> Result<(), Refused> {
        let mut claims = self.claims.lock().unwrap_or_else(PoisonError::into_inner);
        if claims.answered.contains(id) {
            return Err(refused("answered", 409, "this look was answered already"));
        }
        if !claims.going.insert(id.to_owned()) {
            return Err(refused(
                "answering",
                409,
                "an answer to this look is on its way",
            ));
        }
        Ok(())
    }

    /// Sends `value` as the answer to look `id`, once it's read again and
    /// offers it, to the first of its sessions the node takes it for.
    fn steer(&self, id: &str, value: &str) -> Result<Value, Refused> {
        let look = self.look(id)?;
        let now = now();
        offered(&look, value, now)?;
        let answerers = answerers(&look);
        if answerers.is_empty() {
            return Err(refused(
                "unaddressed",
                409,
                "the look names no session to answer",
            ));
        }
        let mut why = String::new();
        for address in answerers {
            let steer = json!({
                "ask": "steer",
                "address": address,
                "steer": {
                    "at": iso(now), "from": "joel", "act": "answer", "look": id,
                    "value": value, "via": "sites",
                },
            });
            let told = page::once(&self.node, &steer).map_err(|failed| {
                refused(
                    "unreached",
                    502,
                    &format!("the node doesn't answer: {failed}"),
                )
            })?;
            let Some(error) = told.get("error") else {
                let steered = told.get("steered").cloned().unwrap_or(Value::Null);
                return Ok(json!({"answered": id, "steered": steered}));
            };
            why.clear();
            why.push_str(error.get("message").and_then(Value::as_str).unwrap_or(""));
            // Only a session no one goes by now has the next told in its
            // place, as For You sends to the quest's once the one that
            // asked has ended; any other refusal stops here, since the
            // session that asked may still be there.
            if !why.starts_with(GONE) {
                break;
            }
        }
        Err(refused(
            "unsteered",
            502,
            &format!("the node refused it: {why}"),
        ))
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
            return Err(refused(
                "site",
                403,
                "an answer comes only from this origin",
            ));
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

    /// Whether the process at the other end of `asked`'s connection is the
    /// person's, as the kernel tells it.
    fn pressed_by_the_person(&self, asked: &Asked) -> Result<(), Refused> {
        if self.walks.take() == Turn::Wait {
            return Err(refused(
                "busy",
                429,
                "who pressed was asked too often just now: try again in a moment",
            ));
        }
        let told = asked.ends.map(|(peer, server)| (self.tell)(peer, server));
        match told {
            Some(Told::Person) => Ok(()),
            Some(Told::Agent(by)) => Err(refused(
                "agent",
                403,
                &format!("{by}, an agent's, pressed it, and an agent can't answer as the person"),
            )),
            Some(Told::Untold(why)) => Err(refused(
                "untold",
                403,
                &format!("who pressed it can't be told: {why}"),
            )),
            None => Err(refused(
                "untold",
                403,
                "who pressed it can't be told: the connection has no ends",
            )),
        }
    }

    /// The look `id` as the looks' daemon holds it now.
    fn look(&self, id: &str) -> Result<Value, Refused> {
        let key = format!("look:{id}");
        let asked = json!({"ask": "get", "keys": [key]});
        let told = page::once(&self.looks, &asked)
            .map_err(|why| refused("unread", 502, &format!("the looks don't answer: {why}")))?;
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
/// `value` one of its first four choices' values, exactly, of one whose
/// label doesn't pass for the launcher's own act (posing.rs). The table of
/// looks kit/tests/offered.json holds both readings to one.
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
        .filter_map(|choice| {
            let value = choice.get("value").and_then(Value::as_str)?;
            let label = choice.get("label").and_then(Value::as_str);
            Some((
                value,
                label.filter(|label| !label.is_empty()).unwrap_or(value),
            ))
        })
        .filter(|(offer, _)| !offer.is_empty())
        .take(OFFERED)
        // One whose label passes for the launcher's own act is offered
        // nowhere, as For You's `shown` drops it.
        .any(|(offer, label)| offer == value && !posing::poses(label));
    if !offers {
        return Err(refused("unoffered", 409, "the look doesn't offer that"));
    }
    Ok(())
}

/// The sessions an answer to `look` goes to, in turn, as the launcher's
/// For You sends one (Looks.js `answer`): the one that left it, by the
/// address it names, then its quest's own session, whose address is the
/// quest's last part, where the asking goes on once the first has ended.
/// None where the look names neither.
fn answerers(look: &Value) -> Vec<String> {
    let text = |field: &str| look.get(field).and_then(Value::as_str).unwrap_or("");
    let quests = text("quest").rsplit('-').next().unwrap_or("");
    let mut answerers: Vec<String> = Vec::new();
    for address in [text("address"), quests] {
        let address = address.to_uppercase();
        if !address.is_empty() && !answerers.contains(&address) {
            answerers.push(address);
        }
    }
    answerers
}

/// `bytes` written in hexadecimal.
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                DIGITS[usize::from(byte >> 4)],
                DIGITS[usize::from(byte & 0xf)],
            ]
        })
        .map(char::from)
        .collect()
}

/// Whether `said`, a `Content-Type`, names JSON, whatever parameters follow.
fn is_json(said: Option<&str>) -> bool {
    said.and_then(|said| said.split(';').next())
        .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("application/json"))
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

    #[test]
    fn the_server_takes_just_the_looks_the_table_says_a_page_answers() {
        let table: Value = serde_json::from_str(include_str!("../kit/tests/offered.json"))
            .expect("the table reads");
        let now = when(table.get("now")).expect("the table's now");
        let cases = table["cases"].as_array().expect("its cases");
        for case in cases {
            let mut look = table["look"].clone();
            for (field, value) in case["with"].as_object().expect("what it differs by") {
                look[field] = value.clone();
            }
            let value = case["value"].as_str().expect("what's pressed");
            assert_eq!(
                offered(&look, value, now).is_ok(),
                case["offered"] == true,
                "{}",
                case["why"]
            );
        }
        assert!(cases.len() > 20, "the table lost its cases");
    }

    /// Answers whose presses `told` says who pressed.
    fn answers_told(told: Told) -> Answers {
        Answers::holding(
            (
                Path::new("/nowhere/looks.sock").to_path_buf(),
                Path::new("/nowhere/node.sock").to_path_buf(),
            ),
            "c0ffee".to_owned(),
            Box::new(move |_, _| told.clone()),
        )
    }

    /// Answers pressed by the person.
    fn answers() -> Answers {
        answers_told(Told::Person)
    }

    fn ends() -> (SocketAddr, SocketAddr) {
        let peer = "127.0.0.1:40000".parse().expect("the browser's end");
        (peer, "127.0.0.1:8484".parse().expect("the server's end"))
    }

    fn pressed<'a>(
        origin: Option<&'a str>,
        site: Option<&'a str>,
        cookies: Option<&'a str>,
    ) -> Asked<'a> {
        Asked {
            host: "sites.localhost:8484",
            origin,
            fetch_site: site,
            cookies,
            content_type: Some("application/json"),
            ends: Some(ends()),
            body: br#"{"look": "l1", "value": "yes"}"#,
        }
    }

    /// A press from a page of the sites' own, but for what `change` alters.
    fn pressed_here_but(change: impl FnOnce(&mut Asked)) -> Asked<'static> {
        let mut asked = pressed(
            Some("http://sites.localhost:8484"),
            Some("same-origin"),
            Some("sites-answer=c0ffee"),
        );
        change(&mut asked);
        asked
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
            (
                pressed(Some("http://localhost:8484"), Some("same-origin"), cookie),
                "origin",
            ),
            (
                pressed(Some("http://evil.example"), Some("same-origin"), cookie),
                "origin",
            ),
            (
                pressed(
                    Some("http://documents.sites.localhost:8484"),
                    Some("same-site"),
                    cookie,
                ),
                "origin",
            ),
            (pressed(ours, Some("same-site"), cookie), "site"),
            (pressed(ours, Some("cross-site"), cookie), "site"),
            (pressed(ours, None, cookie), "site"),
            (pressed(ours, Some("same-origin"), None), "cookie"),
            (
                pressed(ours, Some("same-origin"), Some("sites-answer=c0ffef")),
                "cookie",
            ),
            (
                pressed(ours, Some("same-origin"), Some("sites-answer=c0ffee0")),
                "cookie",
            ),
            (
                pressed(ours, Some("same-origin"), Some("xsites-answer=c0ffee")),
                "cookie",
            ),
        ] {
            let why = answers.carry(&asked).expect_err("refused");
            assert_eq!(why.word, word, "{}", why.why);
            assert_eq!(why.status, 403, "{word}");
        }
        let pressed_here = pressed(ours, Some("same-origin"), cookie);
        let why = answers.carry(&pressed_here).expect_err("no daemon stands");
        assert_eq!(
            why.word, "unread",
            "a page's answer passed the door: {}",
            why.why
        );
    }

    #[test]
    fn an_answer_not_sent_as_json_is_refused_so_no_form_can_press() {
        for said in [
            None,
            Some("text/plain"),
            Some("application/x-www-form-urlencoded"),
        ] {
            let asked = pressed_here_but(|asked| asked.content_type = said);
            let why = answers().carry(&asked).expect_err("refused");
            assert_eq!((why.word, why.status), ("type", 415), "{said:?}");
        }
        let asked =
            pressed_here_but(|asked| asked.content_type = Some("Application/JSON; charset=utf-8"));
        let why = answers().carry(&asked).expect_err("no daemon stands");
        assert_eq!(
            why.word, "unread",
            "JSON with a charset was refused: {}",
            why.why
        );
    }

    #[test]
    fn an_answer_an_agent_pressed_or_nobody_can_be_told_of_is_refused() {
        let agents = answers_told(Told::Agent("pi-host-abc.service".to_owned()));
        let why = agents
            .carry(&pressed_here_but(|_| ()))
            .expect_err("refused");
        assert_eq!((why.word, why.status), ("agent", 403), "{}", why.why);
        assert!(why.why.contains("pi-host-abc.service"), "{}", why.why);
        let untold = answers_told(Told::Untold("no process holds it"));
        let why = untold
            .carry(&pressed_here_but(|_| ()))
            .expect_err("refused");
        assert_eq!((why.word, why.status), ("untold", 403), "{}", why.why);
        let endless = pressed_here_but(|asked| asked.ends = None);
        let why = answers().carry(&endless).expect_err("refused");
        assert_eq!(
            why.word, "untold",
            "a press with no ends was carried: {}",
            why.why
        );
    }

    #[test]
    fn who_pressed_is_asked_only_so_often_and_a_flood_is_refused_busy() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let walked = Arc::new(AtomicUsize::new(0));
        let counting = Arc::clone(&walked);
        let answers = Answers::holding(
            (PathBuf::new(), PathBuf::new()),
            "c0ffee".to_owned(),
            Box::new(move |_, _| {
                counting.fetch_add(1, Ordering::Relaxed);
                Told::Person
            }),
        );
        let asking = pressed_here_but(|asked| asked.origin = None);
        let words: Vec<_> = (0..40)
            .map(|_| answers.cookie_for(&asking).err().map(|why| why.word))
            .collect();
        let busy = words.iter().filter(|word| **word == Some("busy")).count();
        // Forty at once take a moment at most, in which a spell or two
        // may give one back.
        let walks = walked.load(Ordering::Relaxed);
        assert!((8..=10).contains(&walks), "forty asks walked {walks} times");
        assert_eq!(
            busy + walks,
            40,
            "an ask was neither walked nor refused busy"
        );
        assert_eq!(words[0], None, "the first ask wasn't given the cookie");
    }

    #[test]
    fn the_cookie_goes_only_to_the_persons_page_and_to_no_script() {
        let asking = pressed_here_but(|asked| asked.origin = None);
        let cookie = answers()
            .cookie_for(&asking)
            .expect("the person's page is given it");
        assert_eq!(
            cookie,
            "sites-answer=c0ffee; Path=/answer; SameSite=Strict; HttpOnly"
        );
        let elsewhere = pressed_here_but(|asked| asked.fetch_site = Some("cross-site"));
        assert_eq!(
            answers().cookie_for(&elsewhere).map_err(|why| why.word),
            Err("site")
        );
        let documents = pressed_here_but(|asked| {
            asked.origin = Some("http://documents.sites.localhost:8484");
            asked.fetch_site = Some("same-site");
        });
        assert_eq!(
            answers().cookie_for(&documents).map_err(|why| why.word),
            Err("site"),
            "a document was given the cookie"
        );
        let agents = answers_told(Told::Agent("jobs".to_owned()));
        assert_eq!(
            agents.cookie_for(&asking).map_err(|why| why.word),
            Err("agent")
        );
        let untold = answers_told(Told::Untold("nothing"));
        assert_eq!(
            untold.cookie_for(&asking).map_err(|why| why.word),
            Err("untold")
        );
        let draw = || {
            let tell: Teller = Box::new(|_, _| Told::Person);
            Answers::new(PathBuf::new(), PathBuf::new(), tell).expect("a cookie drawn")
        };
        let (drawn, again) = (draw(), draw());
        assert_eq!(drawn.secret.len(), 32, "the cookie is drawn from 16 bytes");
        assert_ne!(drawn.secret, again.secret, "the cookie wasn't drawn fresh");
    }

    #[test]
    fn a_body_names_its_look_and_value_or_is_refused() {
        assert_eq!(
            said(br#"{"look": "l1", "value": "yes"}"#),
            Ok(("l1".to_owned(), "yes".to_owned()))
        );
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
        assert_eq!(
            offered(&look(), "no", NOW),
            Ok(()),
            "a choice with no label is offered by its value"
        );
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
            (
                changed("at", json!(iso(NOW - QUIET_AFTER - 1))),
                "yes",
                "quiet",
            ),
        ] {
            let why = offered(&look, value, NOW).expect_err(word);
            assert_eq!(why.word, word, "{}", why.why);
        }
        let unknown_zone = changed("expiresAt", json!("2020-01-01T00:00:00"));
        assert_eq!(
            offered(&unknown_zone, "yes", NOW),
            Ok(()),
            "a time with no zone says nothing"
        );
    }

    #[test]
    fn an_answer_goes_to_the_session_that_left_the_look_then_its_quests() {
        let mut look = look();
        assert_eq!(
            answerers(&look),
            ["ABC123"],
            "an answer went past the session that left the look"
        );
        look["quest"] = json!("QEST-20261124-live12");
        assert_eq!(
            answerers(&look),
            ["ABC123", "LIVE12"],
            "an answer had no quest's session to go to once its own ended"
        );
        look["address"] = json!("live12");
        assert_eq!(answerers(&look), ["LIVE12"], "one session was told twice");
        look["address"] = json!("");
        assert_eq!(
            answerers(&look),
            ["LIVE12"],
            "a look naming no session wasn't answered at its quest's"
        );
        look["quest"] = json!("");
        assert!(
            answerers(&look).is_empty(),
            "a look naming neither found a session to answer"
        );
    }

    #[test]
    fn two_cookies_are_told_apart_whatever_they_share() {
        assert!(same("c0ffee", "c0ffee"), "a cookie wasn't taken for itself");
        assert!(!same("c0ffee", "c0ffef"), "a cookie a letter off passed");
        assert!(!same("c0ffee", "c0ffe"), "a cookie's start passed for it");
        assert!(!same("", "c0ffee"), "no cookie passed for one");
    }
}
