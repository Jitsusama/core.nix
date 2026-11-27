//! The safe word's alert on Joel's phone: the same alert the launcher
//! shows, as a message in his own note-to-self DM in Slack, which only he
//! sees.
//!
//! It is the one exception to never sending in Slack besides the replier,
//! and Joel's pick 5 (2026-11-13) holds it this narrow. It goes only to
//! his own IM: [`OwnIm`] is the one thing [`tell`] sends to, and nothing
//! makes one but [`own_im`], which asks Slack for the IM with Joel alone
//! and has Slack's conversations.info say it's an IM and his before it's
//! trusted, so no caller can name a channel. A reader over a tool gateway
//! sends by [`tell_self`], through the gateway's one tool that sends a
//! message from Joel to himself and can name nobody else. Either says only
//! the alert's fixed form, [`text`]: who stopped what, the words said, and
//! where. And it sends from the safe word's own reader, never from an
//! agent or a tool.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use serde_json::{Value, json};
use slack_people::{Account, Slack};
use ureq::Agent;

use crate::mcp::{Called, Gateway, never_connected};

/// How long Slack has to answer before it's given up on.
const PATIENCE: Duration = Duration::from_secs(10);

/// Slack's method that names the IM with the people given, here Joel
/// alone, which is his note-to-self. It's asked never to make one, so it
/// posts and makes nothing, but it is a write-scoped method all the same:
/// an IM Joel has closed it marks open again, back in his sidebar
/// (KP0L6A). It isn't one of slack-people's reads, so it's asked here.
/// Not users.conversations, which lists every IM: an Enterprise Grid
/// sign-in such as Joel's is refused it (`enterprise_is_restricted`, the
/// reader's first start against real Slack, 2026-11-14).
const OPEN: &str = "conversations.open";

/// Slack's method that posts a message.
const POST: &str = "chat.postMessage";

/// One alert, as the launcher titles it.
#[derive(Debug, Clone, Copy)]
pub struct Alert<'a> {
    /// What the launcher's row says, as "Someone stopped your agents on
    /// pull request example/shop#12".
    pub title: &'a str,
    /// The words that were said.
    pub words: &'a str,
    /// Where they were said.
    pub link: &'a str,
}

/// How many characters of the words said are shown, so a long message
/// doesn't flood Joel's phone; the link has the rest. A row's words are
/// already its gist, cut shorter in tell.rs, so this guards only words
/// that came some other way, such as a line written before rows were cut.
const WORDS_SHOWN: usize = 300;

/// How every alert begins, so the reader knows its own when a search
/// finds one in Joel's IM.
const OPENING: &str = "pineapple: ";

/// Whether `text`, written as Joel, is one of these alerts: it quotes what
/// others said, so the reader must never judge it as his own saying, or
/// words made to slip out of its quotation marks would resume as him.
#[must_use]
pub fn is_alert(text: &str) -> bool {
    text.starts_with(OPENING)
}

/// The alert's one form, the same the launcher's daemon writes down as
/// what it would send. What others wrote is escaped as Slack's formatting
/// asks (&, < and >), so none of it is drawn as a masked link, a mention
/// or `<!here>` in Joel's own IM (KP0L6A), and the words are cut short.
#[must_use]
pub fn text(alert: Alert) -> String {
    let mut words: String = alert.words.chars().take(WORDS_SHOWN).collect();
    if alert.words.chars().nth(WORDS_SHOWN).is_some() {
        words.push_str("...");
    }
    format!(
        "{OPENING}{}\nSaid: \"{}\"\nWhere: {}",
        escaped(alert.title),
        escaped(&words),
        escaped(alert.link)
    )
}

/// Text as Slack draws it plainly: its three control characters escaped.
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Joel's own note-to-self IM, as Slack confirmed it: made only by
/// [`own_im`], and the only place [`tell`] sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnIm(String);

impl OwnIm {
    /// The IM's id, to write down where an alert went.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.0
    }
}

/// Finds Joel's own IM: the one Slack names for Joel alone, never made
/// if it isn't there, which conversations.info then confirms is an IM
/// and his.
///
/// # Errors
///
/// Why it wasn't found or wasn't confirmed.
pub fn own_im(origin: &str, slack: &Slack, account: &Account) -> Result<OwnIm, String> {
    if account.user.is_empty() {
        return Err("the sign-in doesn't say who Joel is".to_owned());
    }
    let mut answer = agent()
        .get(format!("{origin}/api/{OPEN}"))
        .query("users", &account.user)
        .query("prevent_creation", "true")
        .query("return_im", "true")
        .header("Authorization", &format!("Bearer {}", account.token))
        .header("Cookie", &format!("d={}", account.cookie))
        .call()
        .map_err(|error| format!("couldn't reach Slack: {error}"))?;
    let named: Value = answer
        .body_mut()
        .read_json()
        .map_err(|error| format!("Slack's answer for Joel's own IM couldn't be read: {error}"))?;
    refused(&named)?;
    let id = picked(&named, &account.user).ok_or("Slack named no IM of Joel's own")?;
    let info = slack
        .read(account, "conversations.info", &[("channel", &id)])
        .map_err(|why| format!("Slack wouldn't say what {id} is: {why:?}"))?;
    if confirmed(&info, &account.user) {
        Ok(OwnIm(id))
    } else {
        Err(format!("{id} isn't Joel's own IM, as Slack tells it"))
    }
}

/// Why an alert didn't go, as far as can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Missed {
    /// It never went, or Slack said no: trying again can't say it twice.
    NotSent(String),
    /// It may have gone, as when Slack's answer timed out or couldn't be
    /// read: trying again may say it twice.
    Unsure(String),
}

/// Sends one alert to Joel's own IM, unfurling nothing under it, and
/// gives the text that went.
///
/// # Errors
///
/// Why it didn't go, and whether it may have all the same.
pub fn tell(origin: &str, account: &Account, own: &OwnIm, alert: Alert) -> Result<String, Missed> {
    let said = text(alert);
    let mut answer = agent()
        .post(format!("{origin}/api/{POST}"))
        .header("Authorization", &format!("Bearer {}", account.token))
        .header("Cookie", &format!("d={}", account.cookie))
        .send_form([
            ("channel", own.id()),
            ("text", said.as_str()),
            ("unfurl_links", "false"),
            ("unfurl_media", "false"),
        ])
        .map_err(|error| {
            let why = format!("couldn't reach Slack: {error}");
            // Only a connection never made is sure not to have posted.
            if never_connected(&error) {
                Missed::NotSent(why)
            } else {
                Missed::Unsure(why)
            }
        })?;
    let status = answer.status().as_u16();
    let took: Value = answer
        .body_mut()
        .read_json()
        .map_err(|error| Missed::Unsure(format!("Slack answered {status}: {error}")))?;
    refused(&took).map_err(|why| Missed::NotSent(format!("Slack answered {status}: {why}")))?;
    Ok(said)
}

/// The gateway's tool that sends a message from Joel to himself, which
/// takes the message alone and can't be given anyone else to send to.
const TO_SELF: &str = "slack_send_message_to_self";

/// What's said in place of the words when the gateway won't take them.
const WORDS_NOT_TAKEN: &str = "(the words are at the link)";

/// Sends one alert to Joel from himself through `gateway`. It refuses a
/// mention, so `@here`, `@channel` and `@everyone` in what others wrote
/// are said as `(at)here` and so on; any failing of the tool, or a
/// refusal of it before it ran, is met by sending it once more with the
/// words left to the link, so the title and where still reach his phone.
/// Only that second answer is judged, as it holds none of the words, and
/// any line of it the failing quotes is left out first, since the title
/// and where are still from Slack: a rate spent there, Slack refusing
/// the gateway's sign-in, or a refusal before it ran did nothing, so the
/// alert is tried again whole the next round.
///
/// The tool's failing promises nothing of whether Slack took the message,
/// so a second failing is counted as may have gone: two such rounds and
/// it's given up on, rather than sent twice a round for ten minutes. One
/// that failed before the tool was asked, as a session opened or its
/// backend was turned on, went nowhere, so it's tried again each round.
/// It gives the text that went, its mentions said otherwise or its words
/// left to the link as they were.
///
/// # Errors
///
/// Why it didn't go, and whether it may have all the same.
pub fn tell_self(gateway: &mut Gateway, alert: Alert) -> Result<String, Missed> {
    let ask = |gateway: &mut Gateway, alert| {
        let said = unmentioned(&text(alert));
        let called = gateway.call(TO_SELF, &json!({ "message": &said }));
        (said, called)
    };
    let (said, sent) = match ask(gateway, alert) {
        (_, Err(called)) if !alert.words.is_empty() && plainly(gateway, &called) => {
            let plain = Alert {
                words: WORDS_NOT_TAKEN,
                ..alert
            };
            ask(gateway, plain)
        }
        asked => asked,
    };
    let put = gateway.put_to(TO_SELF);
    let Err(called) = sent else {
        return Ok(said);
    };
    let nothing_ran = |why: &str| says(why, &said, spent) || says(why, &said, refuses_the_sign_in);
    Err(match called {
        Called::Unreachable(why) | Called::Unreadable(why) | Called::Failed(why) if !put => {
            Missed::NotSent(why)
        }
        Called::Failed(why) if nothing_ran(&why) => Missed::NotSent(why),
        Called::Unreachable(why) | Called::Unreadable(why) | Called::Failed(why) => {
            Missed::Unsure(why)
        }
        Called::NotReached(why) | Called::Declined(why) => Missed::NotSent(why),
        Called::Refused => Missed::NotSent("the gateway refused Joel's sign-in".to_owned()),
        Called::NoSignIn(_) => Missed::NotSent("no sign-in to the gateway was given".to_owned()),
    })
}

/// How many times a failing's escapes are read back at most, for a writer
/// that quotes a body already quoted as JSON inside its own.
const ESCAPED_DEEPEST: usize = 3;

/// Whether a failing says what `word` looks for in every reading of it,
/// each without any line of `said` it quotes, as written or as JSON would
/// write it: the title and where are still from Slack, so a name in them
/// can't be read as the gateway's own word. A reading reads the failing's
/// escapes back up to [`ESCAPED_DEEPEST`] times, taking the lines out
/// only after or also between, so a name holding a backslash of its own,
/// or holding another line, is found whole in at least one, while the
/// gateway's own word, outside any quote, is in them all.
fn says(why: &str, said: &str, word: fn(&str) -> bool) -> bool {
    let lines: Vec<&str> = said
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let without = |mut left: String| {
        for line in &lines {
            let json = serde_json::to_string(line).unwrap_or_default();
            let inner = json.get(1..json.len().saturating_sub(1)).unwrap_or(line);
            left = left.replace(line, "").replace(inner, "");
        }
        left
    };
    let (mut read, mut between) = (why.to_owned(), without(why.to_owned()));
    for _ in 0..ESCAPED_DEEPEST {
        if !word(&without(read.clone())) || !word(&between) {
            return false;
        }
        read = unescaped(&read);
        between = without(unescaped(&between));
    }
    word(&without(read)) && word(&between)
}

/// `why` with each of JSON's escapes read back, from left to right: `\\`,
/// `\"`, `\/`, the control characters' and `\uXXXX`, a surrogate pair as
/// one, since JSON writers other than serde's escape so what isn't ASCII,
/// `&`, `<`, `>` or `/`. What names no character is left as it was.
fn unescaped(why: &str) -> String {
    let mut out = String::with_capacity(why.len());
    let mut rest = why;
    while let Some(at) = rest.find('\\') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let (read, taken) = match rest.as_bytes().get(1) {
            Some(b'u') => escape(rest),
            Some(&mark) => short(mark).map_or((None, 1), |read| (Some(read), 2)),
            None => (None, 1),
        };
        match read {
            Some(read) => out.push(read),
            None => out.push_str(&rest[..taken]),
        }
        rest = &rest[taken..];
    }
    out.push_str(rest);
    out
}

/// The character a backslash and `mark` name in JSON, where `mark` isn't
/// `u`; none where they name none.
fn short(mark: u8) -> Option<char> {
    Some(match mark {
        b'"' => '"',
        b'\\' => '\\',
        b'/' => '/',
        b'b' => '\u{8}',
        b'f' => '\u{c}',
        b'n' => '\n',
        b'r' => '\r',
        b't' => '\t',
        _ => return None,
    })
}

/// The character the `\uXXXX` escape opening `at` names, two escapes for a
/// surrogate pair, and how many bytes it took; none where it names none,
/// the `\u` alone taken where no four digits follow it.
fn escape(at: &str) -> (Option<char>, usize) {
    let unit = |from: usize| {
        at.get(from..from + 2).filter(|mark| *mark == "\\u")?;
        let digits = at.get(from + 2..from + 6)?;
        if !digits.bytes().all(|digit| digit.is_ascii_hexdigit()) {
            return None;
        }
        u16::from_str_radix(digits, 16).ok()
    };
    let Some(first) = unit(0) else {
        return (None, 2);
    };
    if let Some(second) = unit(6)
        && let Some(Ok(pair)) = char::decode_utf16([first, second]).next()
        && pair.len_utf16() == 2
    {
        return (Some(pair), 12);
    }
    (char::from_u32(u32::from(first)), 6)
}

/// Whether a send that came back `called` is sent once more without its
/// words: its tool's failing or refusal, or a failing after the tool was
/// asked, as its backend was turned on, when nothing had run.
fn plainly(gateway: &Gateway, called: &Called) -> bool {
    if gateway.put_to(TO_SELF) {
        matches!(called, Called::Failed(_) | Called::Declined(_))
    } else {
        gateway.asked_of(TO_SELF)
    }
}

/// Whether a tool's failing says Slack's rate was spent, which did nothing:
/// either of Slack's words for it, `ratelimited` and `rate_limited`, or
/// HTTP's or a person's. It's asked only of a failing to send the alert
/// without its words, every line sent left out of it first, so nobody can
/// hold an alert back by saying "rate limit" in words or a name it quotes.
fn spent(why: &str) -> bool {
    let why = why.to_ascii_lowercase();
    why.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(|word| word == "ratelimited" || word == "rate_limited")
        || why.contains("rate limit")
        || why.contains("too many requests")
}

/// Whether a tool's failing says Slack refused the gateway's sign-in, so
/// nothing was posted, as the reader's ears tell it.
fn refuses_the_sign_in(why: &str) -> bool {
    why.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(crate::deaf::refuses_the_sign_in)
}

/// The words that mention everyone said so they don't, as the gateway
/// refuses to send a mention: `@` before `here`, `channel` or `everyone`
/// as a whole word, in any case, is said as `(at)`, while an address such
/// as `ops@channel4.com` is left as written.
fn unmentioned(text: &str) -> String {
    let mut said = String::with_capacity(text.len());
    for (at, character) in text.char_indices() {
        let everyone = || {
            let rest = text[at + character.len_utf8()..].to_ascii_lowercase();
            ["here", "channel", "everyone"].iter().any(|word| {
                rest.strip_prefix(word)
                    .is_some_and(|after| !after.starts_with(|next: char| next.is_alphanumeric()))
            })
        };
        if character == '@' && everyone() {
            said.push_str("(at)");
        } else {
            said.push(character);
        }
    }
    said
}

fn agent() -> Agent {
    Agent::config_builder()
        .timeout_global(Some(PATIENCE))
        .http_status_as_error(false)
        .build()
        .into()
}

/// Why Slack said no, where it did.
fn refused(said: &Value) -> Result<(), String> {
    if said["ok"].as_bool() == Some(true) {
        Ok(())
    } else {
        Err(said["error"].as_str().unwrap_or("no reason").to_owned())
    }
}

/// The IM Slack named, if it says it's an IM whose user is `joel`; a
/// missing `is_im` is let through, since conversations.info confirms it.
fn picked(named: &Value, joel: &str) -> Option<String> {
    let channel = &named["channel"];
    let ours = channel["is_im"].as_bool() != Some(false) && channel["user"].as_str() == Some(joel);
    channel["id"].as_str().filter(|_| ours).map(str::to_owned)
}

/// Whether conversations.info says it's an IM, and Joel's own.
fn confirmed(info: &Value, joel: &str) -> bool {
    let channel = &info["channel"];
    channel["is_im"].as_bool() == Some(true) && channel["user"].as_str() == Some(joel)
}

/// What phoned.jsonl keeps for an alert that may have gone, whose answer
/// was lost.
pub const UNSURE: &str = "unsure";

/// How many sends an alert gets whose answers were lost, so one that
/// landed unheard reaches Joel's phone twice at most (KP0L6A).
const UNSURE_TRIES: usize = 2;

/// The alerts done with, by [`told_as`], from phoned.jsonl's lines: told,
/// written down in shadow, or sent as often as an unsure one may be.
#[must_use]
pub fn done(phoned: &[Value]) -> HashSet<String> {
    let mut unsure: HashMap<String, usize> = HashMap::new();
    let mut done = HashSet::new();
    for line in phoned {
        if line["sent"] == UNSURE {
            *unsure.entry(told_as(line)).or_default() += 1;
        } else {
            done.insert(told_as(line));
        }
    }
    done.extend(
        unsure
            .into_iter()
            .filter(|(_, tries)| *tries >= UNSURE_TRIES)
            .map(|(told, _)| told),
    );
    done
}

/// What an alert is told as, its row and the saying it shows, as the
/// launcher's daemon tells it.
#[must_use]
pub fn told_as(line: &Value) -> String {
    format!(
        "{}@{}",
        line["key"].as_str().unwrap_or_default(),
        line["saying"].as_str().unwrap_or_default()
    )
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use serde_json::json;

    use super::*;

    const JOEL: &str = "U0JOEL0001";

    fn phoned(key: &str, sent: &Value) -> Value {
        json!({"key": key, "saying": "stopped", "sent": sent})
    }

    #[test]
    fn an_alert_whose_answer_was_lost_is_sent_once_more_and_no_more() {
        let once = [phoned("a", &json!(UNSURE))];
        assert!(done(&once).is_empty(), "one unsure send is tried again");
        let twice = [phoned("a", &json!(UNSURE)), phoned("a", &json!(UNSURE))];
        assert!(
            done(&twice).contains("a@stopped"),
            "a second unsure send is the last"
        );
        let told = [phoned("b", &json!(true)), phoned("c", &json!(false))];
        assert_eq!(
            done(&told).len(),
            2,
            "one told, and one written down in shadow, are done"
        );
    }

    #[test]
    fn the_alert_says_who_stopped_what_the_words_and_where() {
        let said = text(Alert {
            title: "Someone stopped your agents on pull request example/shop#12",
            words: "pineapple override",
            link: "https://example.slack.com/archives/C1/p1",
        });
        assert_eq!(
            said,
            "pineapple: Someone stopped your agents on pull request example/shop#12\n\
             Said: \"pineapple override\"\n\
             Where: https://example.slack.com/archives/C1/p1"
        );
    }

    #[test]
    fn what_others_wrote_is_never_drawn_as_slacks_markup() {
        let said = text(Alert {
            title: "Someone stopped your agents on <!here>",
            words: "pineapple override <https://evil.example|your PR> & <@U0OTHER001>",
            link: "https://example.slack.com/archives/C1/p1?thread_ts=1&cid=C1",
        });
        assert_eq!(
            said,
            "pineapple: Someone stopped your agents on &lt;!here&gt;\n\
             Said: \"pineapple override &lt;https://evil.example|your PR&gt; &amp; \
             &lt;@U0OTHER001&gt;\"\n\
             Where: https://example.slack.com/archives/C1/p1?thread_ts=1&amp;cid=C1",
            "a masked link, a mention and <!here> come out escaped"
        );
    }

    #[test]
    fn a_mention_of_everyone_is_said_as_at() {
        assert_eq!(
            unmentioned("pineapple override @here and @Channel, @everyone; mail joel@example.com"),
            "pineapple override (at)here and (at)Channel, (at)everyone; mail joel@example.com",
            "each mention of everyone, in any case, and nothing else"
        );
        assert_eq!(
            unmentioned("ends with @"),
            "ends with @",
            "an @ at the end is left"
        );
        assert_eq!(
            unmentioned("mail ops@channel4.com or @hereford, but @here."),
            "mail ops@channel4.com or @hereford, but (at)here.",
            "only the whole word, so an address or a longer name is left"
        );
    }

    #[test]
    fn only_a_rate_spent_is_said_as_spent() {
        assert!(spent("Slack said: ratelimited"), "Slack's own word");
        assert!(spent("Slack said: rate_limited"), "Slack's other word");
        assert!(spent("Rate limit exceeded"), "in other words");
        assert!(spent("429 Too Many Requests"), "in HTTP's");
        assert!(!spent("Slack said: not_allowed"), "a refusal is no rate");
        assert!(
            !spent("Slack said: ratelimitedness"),
            "Slack's words count only whole"
        );
    }

    #[test]
    fn a_name_from_slack_quoted_in_a_failing_is_no_rate_spent() {
        let said = text(Alert {
            title: "A stop in owner/rate_limited \"x\"",
            words: WORDS_NOT_TAKEN,
            link: "https://x.slack.com/archives/C1/p1",
        });
        let json = serde_json::to_string(&said).expect("text as JSON");
        for why in [
            format!("could not post {said}"),
            format!("could not post {json}"),
        ] {
            assert!(spent(&why), "quoted, the name reads as Slack's word: {why}");
            assert!(!says(&why, &said, spent), "left out, it doesn't: {why}");
        }
        assert!(
            says(&format!("rate_limited, for {json}"), &said, spent),
            "the gateway's own word still counts"
        );
    }

    #[test]
    fn a_name_from_slack_quoted_in_a_failing_is_no_sign_in_refused() {
        let said = text(Alert {
            title: "A stop in owner/invalid_auth",
            words: WORDS_NOT_TAKEN,
            link: "https://x.slack.com/archives/C1/p1",
        });
        let escaped = as_python_writes(&serde_json::to_string(&said).expect("text as JSON"));
        let why = format!("could not post {escaped}");
        assert!(
            refuses_the_sign_in(&why),
            "quoted, the name reads as a refusal"
        );
        assert!(
            !says(&why, &said, refuses_the_sign_in),
            "left out, escaped as it was, it doesn't: {why}"
        );
        assert!(
            says(
                &format!("invalid_auth, for {escaped}"),
                &said,
                refuses_the_sign_in
            ),
            "the gateway's own word still counts"
        );
    }

    /// JSON text as Python's writer gives it, and Go's: whatever isn't
    /// ASCII, and `&`, `<` and `>`, as `\uXXXX`; and `/` as PHP's does.
    fn as_python_writes(json: &str) -> String {
        let mut escaped = String::new();
        for each in json.chars() {
            if each == '/' {
                escaped.push_str("\\/");
            } else if each.is_ascii() && !matches!(each, '&' | '<' | '>') {
                escaped.push(each);
            } else {
                for unit in each.encode_utf16(&mut [0; 2]) {
                    write!(escaped, "\\u{unit:04x}").expect("a String takes any text");
                }
            }
        }
        escaped
    }

    #[test]
    fn a_name_quoted_with_its_characters_escaped_is_left_out_too() {
        let said = text(Alert {
            title: "A stop in owner/rate_limited & caf\u{e9} \u{1f34d}",
            words: WORDS_NOT_TAKEN,
            link: "https://x.slack.com/archives/C1/p1",
        });
        let escaped = as_python_writes(&serde_json::to_string(&said).expect("text as JSON"));
        let why = format!("could not post {escaped}");
        assert!(spent(&why), "quoted, the name reads as Slack's word");
        assert!(
            !says(&why, &said, spent),
            "left out, escaped as it was, it doesn't: {why}"
        );
        assert!(
            says(&format!("rate_limited, for {escaped}"), &said, spent),
            "the gateway's own word still counts"
        );
        assert_eq!(
            unescaped(r"\u00e9 \ud83c\udf4d \ud83c \u12 \uzzzz"),
            "\u{e9} \u{1f34d} \\ud83c \\u12 \\uzzzz",
            "what names no character is left as it was"
        );
        assert_eq!(
            unescaped(r#"\\u0072 \/ \" \n \q \"#),
            "\\u0072 / \" \n \\q \\",
            "read from left to right, an escaped backslash opens no escape"
        );
        assert_eq!(
            unescaped(r"\b\f\r\t"),
            "\u{8}\u{c}\r\t",
            "each of JSON's control characters is read back"
        );
    }

    #[test]
    fn a_name_holding_another_line_or_quoted_deep_is_left_out() {
        let quoting = |title: &'static str, deep: usize| {
            let said = text(Alert {
                title,
                words: WORDS_NOT_TAKEN,
                link: "https://x.slack.com/archives/C1/p1",
            });
            let json = serde_json::to_string(&said).expect("text as JSON");
            let mut quoted = as_python_writes(&json);
            for _ in 1..deep {
                quoted = serde_json::to_string(&quoted).expect("text as JSON");
            }
            (said, quoted)
        };
        let holding = "In owner/rate_Said: \"(the words are at the link)\"limited & caf\u{e9}";
        let named = "A stop in owner/rate_limited & caf\u{e9}";
        for (said, quoted) in [quoting(holding, 1), quoting(named, 2), quoting(named, 3)] {
            let why = format!("could not post {quoted}");
            assert!(
                !says(&why, &said, spent),
                "left out, quoted as {quoted}, it reads as no rate spent"
            );
            assert!(
                says(&format!("rate_limited, for {quoted}"), &said, spent),
                "the gateway's own word beside it, quoted as {quoted}, still counts"
            );
        }
    }

    #[test]
    fn a_name_holding_a_backslash_of_its_own_is_left_out_however_quoted() {
        let said = text(Alert {
            title: "A stop in x/\\u0072ate_limited#12 caf\u{e9}",
            words: WORDS_NOT_TAKEN,
            link: "https://x.slack.com/archives/C1/p1",
        });
        let json = serde_json::to_string(&said).expect("text as JSON");
        let ascii = json.replace('\u{e9}', r"\u00e9");
        assert!(
            spent(&unescaped(said.as_str())),
            "read back first, the name would read as Slack's word"
        );
        for quoted in [said.as_str(), json.as_str(), ascii.as_str()] {
            let why = format!("could not post {quoted}");
            assert!(
                !says(&why, &said, spent),
                "left out, quoted as {quoted}, it reads as no rate spent"
            );
            assert!(
                says(&format!("rate_limited, for {quoted}"), &said, spent),
                "the gateway's own word beside it, quoted as {quoted}, still counts"
            );
        }
    }

    #[test]
    fn long_words_are_cut_short() {
        let long = "a".repeat(WORDS_SHOWN + 50);
        let said = text(Alert {
            title: "t",
            words: &long,
            link: "l",
        });
        let shown = format!("Said: \"{}...\"", "a".repeat(WORDS_SHOWN));
        assert!(
            said.contains(&shown),
            "cut to {WORDS_SHOWN} characters: {said}"
        );
    }

    #[test]
    fn only_an_im_whose_user_is_joel_is_picked() {
        let own = json!({"ok": true, "channel": {"id": "D0SELF0001", "is_im": true, "user": JOEL}});
        assert_eq!(
            picked(&own, JOEL).as_deref(),
            Some("D0SELF0001"),
            "his own IM is picked"
        );
        let theirs = json!({"ok": true, "channel": {"id": "D0OTHER001", "is_im": true,
            "user": "U0OTHER001"}});
        assert_eq!(picked(&theirs, JOEL), None, "nobody else's IM");
        let channel =
            json!({"ok": true, "channel": {"id": "C0ONE00001", "is_im": false, "user": JOEL}});
        assert_eq!(picked(&channel, JOEL), None, "nor a channel");
    }

    #[test]
    fn slack_must_say_it_is_an_im_and_joels() {
        let own = json!({"ok": true, "channel": {"id": "D0SELF0001", "is_im": true, "user": JOEL}});
        assert!(confirmed(&own, JOEL), "Joel's own IM is confirmed");
        let theirs = json!({"ok": true, "channel": {"id": "D0OTHER001", "is_im": true,
            "user": "U0OTHER001"}});
        assert!(!confirmed(&theirs, JOEL), "someone else's IM isn't");
        let channel = json!({"ok": true, "channel": {"id": "C0ONE00001", "is_channel": true}});
        assert!(!confirmed(&channel, JOEL), "a channel isn't");
        let refused = json!({"ok": false, "error": "channel_not_found"});
        assert!(!confirmed(&refused, JOEL), "nor is what Slack won't say");
    }
}
