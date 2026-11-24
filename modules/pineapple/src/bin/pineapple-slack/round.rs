//! One round of the Slack reader, whichever ears it hears through: its two
//! searches, each new message judged where it was said, and what follows
//! from a decision (a record, a stop, a reply, a resume).

use std::collections::HashSet;
use std::path::Path;

use pineapple::deaf::{Deafness, Round, Search};
use pineapple::ears::Ears;
use pineapple::gate::{Asked, gate};
use pineapple::judged::{self, Judged};
use pineapple::phone;
use pineapple::record::{self, Source};
use pineapple::reply::{self, Post};
use pineapple::resume;
use pineapple::slack::{Message, micros};
use pineapple::vocabulary::Words;
use pineapple::{Context, Heard, Joel, Platform, Target, Verdict, decide};
use slack_people::Account;

use super::{LOOK_BACK, MICROS, MILLIS, SLACK_RESUMES_LIVE, millis, seconds};

/// The safe word, as its search asks for it.
pub(super) const PHRASE: &str = "pineapple";

/// Joel's tag, as its search asks for it.
pub(super) fn tag(me: &str) -> String {
    format!("<@{me}>")
}

/// What the reader acts with: where replies go, the words they say, and
/// whether its stops, replies and resumes are only written down.
pub(super) struct Reader {
    pub(super) origin: String,
    pub(super) words: Words,
    pub(super) shadow: bool,
}

/// One search of each kind, and each new message judged and written down;
/// what it came to, a refused sign-in before a Slack that couldn't be asked.
/// `me` is Joel's Slack id, and his sign-in, where a reply could be said
/// with it.
///
/// A search whose answer can't be read is never partly trusted, and
/// nothing of it is judged; the other's messages are, and the round says
/// which search is deaf. Both deaf reads nothing. Either way the next
/// round asks both again, with no rest a poster could stretch.
pub(super) fn round(
    ears: &mut dyn Ears,
    reader: &Reader,
    me: (&str, Option<&Account>),
    state: &Path,
    judged: &mut Judged,
) -> Round {
    let (me, account) = me;
    let joel = Joel {
        slack_ids: vec![me.to_owned()],
        logins: Vec::new(),
        emails: Vec::new(),
    };
    // Checked each round, as a clock set back while it runs leaves a
    // since that would pass over everything said until it caught up.
    if judged.look_back_to(seconds().saturating_sub(LOOK_BACK)) {
        eprintln!(
            "<4>how far Slack was read starts later than ten minutes ago, as a clock set back \
             leaves it, so it reads from ten minutes back"
        );
    }
    let tag = tag(me);
    // A sign-in refused or missing for the first isn't asked for again; an
    // answer that couldn't be read is, so the other may still be heard.
    let now = seconds();
    let (phrase, tagged) = match ears.search(PHRASE, &judged.back_to(PHRASE, now)) {
        Err(deaf @ Round::Deaf(Deafness::Refused | Deafness::NoSignIn)) => return deaf,
        phrase => (phrase, ears.search(&tag, &judged.back_to(&tag, now))),
    };
    let unread = Round::Deaf(Deafness::Unreadable);
    let (phrase, tagged, deaf) = match (phrase, tagged) {
        (Ok(phrase), Ok(tagged)) => (phrase, tagged, None),
        (Ok(phrase), Err(tagged)) if tagged == unread => (phrase, Vec::new(), Some(Search::Tag)),
        (Err(phrase), Ok(tagged)) if phrase == unread => (Vec::new(), tagged, Some(Search::Phrase)),
        (phrase, tagged) => {
            let failed = [phrase.err(), tagged.err()];
            // A refusal is said first, then a sign-in none could give.
            // One search unasked, its network blinking, says nothing new
            // of the other, so only both unread are told as both.
            return [
                Round::Deaf(Deafness::Refused),
                Round::Deaf(Deafness::NoSignIn),
                Round::Unasked,
            ]
            .into_iter()
            .find(|round| failed.contains(&Some(*round)))
            .unwrap_or(unread);
        }
    };
    let mut seen = HashSet::new();
    let mut heard: Vec<&Message> = phrase
        .iter()
        .chain(&tagged)
        .filter(|message| judged.is_new(message) && seen.insert(message.key()))
        .collect();
    heard.sort_by_key(|message| micros(&message.ts));
    for message in heard {
        // The reader's own alert quotes others as Joel, so it's no saying.
        if message.author == me && phone::is_alert(&message.text) {
            judged.judged(message);
            continue;
        }
        judge(ears, reader, account, state, &joel, message);
        judged.judged(message);
    }
    // Only a search whose messages were all judged has read as far as
    // they reach; one deaf gave none, so stays where it was.
    judged.reached(PHRASE, phrase.last());
    judged.reached(&tag, tagged.last());
    // judged.json's read_at says when Slack was last read whole, so a
    // round with one search deaf leaves it as it was. Only a gateway's
    // answers are ever half read, and in shadow the gateway reader keeps
    // its own judged.json, apart from the one `heard` reads today. What it judged is
    // kept by key, so the other search's messages are judged once it reads.
    let read_at = match deaf {
        None => Some(millis()),
        Some(_) => judged::read_at(state),
    };
    if let Err(error) = judged::write(state, judged, read_at) {
        eprintln!("<3>couldn't keep how far it has read: {error}");
    }
    deaf.map_or(Round::Heard, |search| Round::Deaf(Deafness::Unread(search)))
}

/// One message judged where it was said, and written down.
fn judge(
    ears: &mut dyn Ears,
    reader: &Reader,
    account: Option<&Account>,
    state: &Path,
    joel: &Joel,
    message: &Message,
) {
    let context = message
        .thread
        .as_deref()
        .and_then(|thread| ears.root(&message.channel, thread))
        .map(|root| Context::of_thread(&root))
        .unwrap_or_default();
    let decision = decide(
        &Heard {
            platform: Platform::Slack,
            author: message.author.clone(),
            by_bot: message.by_bot,
            // Slack says no standing to weigh; a shared channel's guests
            // are judged as anyone there is.
            outsider: false,
            text: message.text.clone(),
            context,
        },
        joel,
    );
    let key = message.key();
    let thread = message.thread_key();
    let source = Source {
        key: &key,
        thread: &thread,
        link: &message.link,
        author: &message.author,
        text: &message.text,
        said_at: &message.ts,
    };
    let now = millis();
    let read_at = now / MILLIS;
    let said_at =
        micros(&message.ts).map_or(0, |at| u64::try_from(at / MICROS).unwrap_or(u64::MAX));
    let went_out = state.join("replies.jsonl");
    // A stop Joel resumed no longer holds its thread's reply back.
    let sent = record::ledger(&resume::cleared(
        &std::fs::read_to_string(&went_out).unwrap_or_default(),
        &state.join("stops"),
    ));
    let asked = Asked {
        reply: decision.reply,
        thread: &thread,
        by_joel: decision.by_joel,
        by_bot: message.by_bot,
        edited: message.edited,
        age: read_at.saturating_sub(said_at),
    };
    let gated = gate(&asked, &sent, read_at, replies_off(state));
    let line = record::line(&source, &decision, gated, now, reader.shadow);
    if let Err(error) = record::append(&state.join("decisions.jsonl"), &line) {
        eprintln!("<3>couldn't write a decision down: {error}");
    }
    if gated.goes() {
        let named: Vec<String> = decision.reaches.iter().map(ToString::to_string).collect();
        let post = reply::post(message, decision.reply, &named, &reader.words);
        if answer(reader, account, message, post.as_ref()) {
            let one = record::sent(
                &source,
                decision.reply,
                read_at,
                reader.shadow,
                &reply::kept(post.as_ref()),
            );
            if let Err(error) = record::append(&went_out, &one) {
                eprintln!("<3>couldn't keep a reply's line: {error}");
            }
        }
    }
    if decision.stops() {
        let stop = record::stop(&source, &decision, now, reader.shadow);
        match record::keep(&state.join("stops"), &key, &stop) {
            Ok(true) if reader.shadow => eprintln!("<5>a stop, in shadow: {}", message.link),
            Ok(true) => eprintln!("<5>a stop: {}", message.link),
            Ok(false) => {}
            Err(error) => eprintln!("<3>couldn't keep a stop's record: {error}"),
        }
    }
    if decision.verdict == Verdict::Resume && decision.by_joel {
        let said = micros(&message.ts).unwrap_or(0);
        resume_as_joel(
            &state.join("stops"),
            &decision.reaches,
            (now, said),
            &message.link,
            reader.shadow,
        );
    }
}

/// Ends the stops Joel's own "pineapple resume" names (src/resume.rs):
/// what it reaches, or every stop when it reaches everything. In shadow,
/// and until Slack resumes are let go live, only the stops written in
/// shadow. `at` is when it's heard, in milliseconds, and when it was
/// said, in microseconds.
fn resume_as_joel(stops: &Path, reaches: &[Target], at: (u64, u128), link: &str, shadow: bool) {
    let (now, said) = at;
    let shadow = shadow || !SLACK_RESUMES_LIVE;
    for reach in reaches {
        let scope = match reach {
            Target::Everything => "all".to_owned(),
            named => named.to_string(),
        };
        let ended = resume::as_joel(stops, &scope, now, said, shadow);
        match ended {
            Ok(ids) if ids.is_empty() => {}
            Ok(ids) if shadow => eprintln!("<5>a resume, in shadow, of {}: {link}", ids.join(", ")),
            Ok(ids) => eprintln!("<5>Joel resumed {}: {link}", ids.join(", ")),
            Err(error) => eprintln!("<3>couldn't resume what Joel named: {error}"),
        }
    }
}

/// Say a reply the rules let through, and whether it counts as said: in
/// shadow each does, said or not, so the rules are tried as they will be;
/// out of it, only one Slack took.
fn answer(
    reader: &Reader,
    account: Option<&Account>,
    message: &Message,
    post: Option<&Post>,
) -> bool {
    if reader.shadow {
        eprintln!("<5>a reply, in shadow: {}", message.link);
        return true;
    }
    let Some(account) = account else {
        eprintln!("<3>no sign-in to say a reply with: {}", message.link);
        return false;
    };
    let Some(post) = post else {
        eprintln!(
            "<4>a reply has no words Joel said yes to, so none was said: {}",
            message.link
        );
        return false;
    };
    match reply::send(&reader.origin, account, post) {
        Ok(()) => {
            eprintln!("<5>a reply, said: {}", message.link);
            true
        }
        Err(why) => {
            eprintln!("<3>Slack didn't take a reply: {why}");
            false
        }
    }
}

/// Whether replies-switch says `off`, read afresh each time so a switch
/// acts on the next message.
fn replies_off(state: &Path) -> bool {
    std::fs::read_to_string(state.join("replies-switch"))
        .is_ok_and(|said| said.lines().any(|line| line.trim() == "off"))
}
