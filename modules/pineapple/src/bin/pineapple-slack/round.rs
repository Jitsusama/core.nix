//! One round of the Slack reader, whichever ears it hears through: its two
//! searches, each new message judged where it was said, and what follows
//! from a decision (a record, a stop, a reply, a resume).

use std::collections::HashSet;
use std::path::Path;

use pineapple::deaf::{Deafness, Round, Search};
use pineapple::ears::{Ears, Unread};
use pineapple::gate::{Asked, Gate, Sent, gate};
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
/// A search whose answer can't be read is never read as far as it reaches,
/// and the round says which search is deaf; the other's messages are
/// judged. What it gave is judged as its ears say it may be: messages a
/// second ask agreed with in full, any other only for a stop, which may
/// come from anyone, never for a resume a writer could type. One search
/// unasked leaves the round read in part, counted neither way, though the
/// other's messages are judged, unless it waits on a later page while the
/// other's answer can't be read, which is said as that search deaf; only
/// both unasked, neither waiting, is a round unasked. Either way the next
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
    let answers = [(PHRASE, &phrase), (tag.as_str(), &tagged)];
    let partly = in_part(ears, reader, account, state, &joel, judged, answers);
    // One search unasked says nothing new of the other, so the round is
    // read in part and counts neither way, but the other's stops are
    // heard as it read them, as is what the first read in part, and it has
    // read as far as they reach; when Slack was read whole stays as it was.
    // The round names the search left unasked unless it waits on a later
    // page, which its ears count.
    let alone = match (&phrase, &tagged) {
        (Ok(read), Err(Round::Unasked)) => Some((PHRASE, read, Search::Tag, tag.as_str())),
        (Err(Round::Unasked), Ok(read)) => Some((tag.as_str(), read, Search::Phrase, PHRASE)),
        _ => None,
    };
    if let Some((query, read, unasked, its)) = alone {
        let heard = read.iter().chain(&partly);
        judge_each(ears, reader, account, state, &joel, judged, heard);
        judged.reached(query, read.last());
        if let Err(error) = judged::write(state, judged, judged::read_at(state)) {
            eprintln!("<3>couldn't keep how far it has read: {error}");
        }
        return Round::Partly((!ears.waiting(its)).then_some(unasked));
    }
    let (phrase, tagged, deaf) = match (phrase, tagged) {
        (Ok(phrase), Ok(tagged)) => (phrase, tagged, None),
        (Ok(phrase), Err(tagged)) if tagged == unread => (phrase, Vec::new(), Some(Search::Tag)),
        (Err(phrase), Ok(tagged)) if phrase == unread => (Vec::new(), tagged, Some(Search::Phrase)),
        // One waiting on a later page, as it may for rounds on end,
        // mustn't hold back Joel's row on the other, which can't be read;
        // it's counted as heard for that row, its first pages read. One
        // whose first page blinked says nothing, so it falls through.
        (Err(phrase), Err(Round::Unasked)) if phrase == unread && ears.waiting(&tag) => {
            (Vec::new(), Vec::new(), Some(Search::Phrase))
        }
        (Err(Round::Unasked), Err(tagged)) if tagged == unread && ears.waiting(PHRASE) => {
            (Vec::new(), Vec::new(), Some(Search::Tag))
        }
        (phrase, tagged) => {
            let failed = [phrase.err(), tagged.err()];
            // What was judged of an answer it couldn't read is kept, and
            // how far each was read and when Slack was read whole aren't.
            if !partly.is_empty() {
                judge_each(ears, reader, account, state, &joel, judged, partly.iter());
                if let Err(error) = judged::write(state, judged, judged::read_at(state)) {
                    eprintln!("<3>couldn't keep what it judged: {error}");
                }
            }
            // A refusal is said first, then a sign-in none could give.
            // One unasked beside the other unreadable, waiting on no later
            // page, heard nothing either, so the round is deaf of both;
            // each search is counted on its own, so one blink doesn't
            // widen a row, and one failing every round is told. Of the
            // rest, only both unasked, neither waiting, is a round no
            // search was answered in, which a gateway's reader counts.
            let unasked = Some(Round::Unasked);
            let unreadable = failed.contains(&Some(unread));
            let left = |search: Search, query: &str, answer: Option<Round>| {
                (answer == unasked && !ears.waiting(query)).then_some(search)
            };
            let blinked = match (
                left(Search::Phrase, PHRASE, failed[0]),
                left(Search::Tag, &tag, failed[1]),
            ) {
                (Some(_), Some(_)) => Round::Unasked,
                (one, other) => Round::Partly(one.or(other)),
            };
            return [
                Round::Deaf(Deafness::Refused),
                Round::Deaf(Deafness::NoSignIn),
            ]
            .into_iter()
            .find(|round| failed.contains(&Some(*round)))
            .or_else(|| (failed.contains(&unasked) && !unreadable).then_some(blinked))
            .unwrap_or(unread);
        }
    };
    let heard = phrase.iter().chain(&tagged).chain(&partly);
    judge_each(ears, reader, account, state, &joel, judged, heard);
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

/// What a deaf search gave, one whose later page went unanswered, or a
/// page a search read on past, by each search's query and its `answers`:
/// only a stop is taken from what no second ask agreed with, judged now
/// and again once read; what one agreed with in a search read in part is
/// given back to judge with the rest, in the order it was said.
fn in_part(
    ears: &mut dyn Ears,
    reader: &Reader,
    account: Option<&Account>,
    state: &Path,
    joel: &Joel,
    judged: &Judged,
    answers: [(&str, &Result<Vec<Message>, Round>); 2],
) -> Vec<Message> {
    let unread = Round::Deaf(Deafness::Unreadable);
    let mut partly = Vec::new();
    for (query, answer) in answers {
        let part = matches!(answer, Err(Round::Unasked)) || answer.as_ref().err() == Some(&unread);
        if part || answer.is_ok() {
            let Unread { agreed, unagreed } = ears.unread(query);
            for message in &unagreed {
                if judged.is_new(message) && !alert(joel, message) {
                    judge(ears, reader, account, state, joel, message, false);
                }
            }
            if part {
                partly.extend(agreed);
            }
        }
    }
    partly
}

/// Each new message `heard` judged once, in the order they were said, and
/// kept as judged.
fn judge_each<'a>(
    ears: &mut dyn Ears,
    reader: &Reader,
    account: Option<&Account>,
    state: &Path,
    joel: &Joel,
    judged: &mut Judged,
    heard: impl Iterator<Item = &'a Message>,
) {
    let mut seen = HashSet::new();
    let mut heard: Vec<&Message> = heard
        .filter(|message| judged.is_new(message) && seen.insert(message.key()))
        .collect();
    heard.sort_by_key(|message| micros(&message.ts));
    for message in heard {
        if !alert(joel, message) {
            judge(ears, reader, account, state, joel, message, true);
        }
        judged.judged(message);
    }
}

/// Whether `message` is the reader's own alert, which quotes others as
/// Joel, so is no saying.
fn alert(joel: &Joel, message: &Message) -> bool {
    joel.slack_ids.contains(&message.author) && phone::is_alert(&message.text)
}

/// One message judged where it was said, and written down; not `agreed`
/// with by a second ask, it could be typed, so only a stop it says is
/// kept, with nothing written, answered or resumed. Its writer is taken as
/// it says, so Joel's own stop isn't waved for want of his tag, as a stop
/// may come from anyone, yet its record never says Joel said it and calls
/// its writer unconfirmed; it links nowhere, as its link could point
/// anywhere, and was said no later than it was heard, so Joel's resume
/// ends it whatever time it types. It's kept under a key of its own
/// ([`unconfirmed`]), so a typed copy of a stop's channel and time never
/// takes the stop's record; once read agreed, the stop is kept whole under
/// its own key and its unconfirmed copy is taken away. Nor is
/// the thread it names read, as that could be any, each round it's deaf:
/// with no thread, a stop reaches what its words name, else its channel's
/// lanes, else everything.
fn judge(
    ears: &mut dyn Ears,
    reader: &Reader,
    account: Option<&Account>,
    state: &Path,
    joel: &Joel,
    message: &Message,
    agreed: bool,
) {
    let context = message
        .thread
        .as_deref()
        .filter(|_| agreed)
        .and_then(|thread| ears.root(&message.channel, thread))
        .map(|root| Context::of_thread(&root))
        .unwrap_or_default();
    let now = millis();
    let read_at = now / MILLIS;
    let heard_at = format!("{read_at}.000000");
    let (link, said_at) = if agreed {
        (message.link.as_str(), message.ts.as_str())
    } else if micros(&message.ts).is_some_and(|said| Some(said) < micros(&heard_at)) {
        ("", message.ts.as_str())
    } else {
        ("", heard_at.as_str())
    };
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
    let key = if agreed {
        message.key()
    } else {
        unconfirmed(message)
    };
    let thread = message.thread_key();
    let source = Source {
        key: &key,
        thread: &thread,
        link,
        author: &message.author,
        text: &message.text,
        said_at,
    };
    let said_at = micros(said_at).map_or(0, |at| u64::try_from(at / MICROS).unwrap_or(u64::MAX));
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
    let mute = account.is_none() && !reader.shadow;
    let gated = reply_gate(&asked, &sent, (read_at, state), mute);
    let entry = record::line(&source, &decision, gated, now, reader.shadow);
    if agreed && let Err(error) = record::append(&state.join("decisions.jsonl"), &entry) {
        eprintln!("<3>couldn't write a decision down: {error}");
    }
    if agreed && gated.goes() {
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
        let mut stop = record::stop(&source, &decision, now, reader.shadow);
        keep_stop(
            &state.join("stops"),
            &mut stop,
            message,
            &key,
            agreed,
            reader.shadow,
        );
    }
    if agreed && decision.verdict == Verdict::Resume && decision.by_joel {
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

/// Keeps a stop's record, `stop`, under `key` in `records`, and says so.
/// Not `agreed`, it never says Joel said it and calls its writer
/// unconfirmed; agreed, its unconfirmed copy is taken away.
fn keep_stop(
    records: &Path,
    stop: &mut serde_json::Value,
    message: &Message,
    key: &str,
    agreed: bool,
    shadow: bool,
) {
    if !agreed {
        stop["by_joel"] = false.into();
        stop["said_by"] = format!("{} (unconfirmed)", message.author).into();
    }
    // Joel's own, read agreed and never edited, said just before a resume
    // of his naming it and heard only after, is kept as he left it,
    // resumed (src/resume.rs). An edited one's time is when it was first
    // said, not when its words were, so it stands.
    if agreed && !message.edited {
        *stop = resume::as_left(records, std::mem::take(stop));
    }
    let shadow = if shadow { ", in shadow" } else { "" };
    let unagreed = if agreed {
        ""
    } else {
        ", in an answer no second ask agreed with"
    };
    let kept = record::keep(records, key, stop);
    match &kept {
        Ok(true) if resume::ended(stop) => eprintln!(
            "<5>a stop{shadow}, said before Joel resumed it, kept as resumed: {}",
            message.link
        ),
        // An unagreed stop is named by the key it's kept under, as its
        // link could point anywhere.
        Ok(true) => eprintln!(
            "<5>a stop{shadow}{unagreed}: {}",
            if agreed { message.link.as_str() } else { key }
        ),
        Ok(false) => {}
        Err(error) => eprintln!("<3>couldn't keep a stop's record: {error}"),
    }
    if agreed
        && kept.is_ok()
        && let Err(error) = record::forget(records, &unconfirmed(message))
    {
        eprintln!("<3>couldn't take away a stop's unconfirmed copy: {error}");
    }
}

/// The key a stop from an answer no second ask agreed with is kept under:
/// its message's own, which could be typed, beside a hash of the writer it
/// names and its words. A typed copy saying other words of the same
/// channel and time is kept beside it rather than in its place, and the
/// same words read again are the same stop.
fn unconfirmed(message: &Message) -> String {
    let said = format!("{}\0{}", message.author, message.text)
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        });
    format!("{}:unconfirmed:{said:016x}", message.key())
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
/// out of it, only one Slack took. With no sign-in of Slack's own, as a
/// reader over a gateway has, none is said, since the gateway sends to
/// Joel alone.
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
        eprintln!(
            "<5>a reply isn't said, as the gateway sends to Joel alone: {}",
            message.link
        );
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

/// The rule held to tell no thread anything, live over a gateway, which
/// sends to Joel alone, as the GitHub reader holds its replies.
const NO_REPLIES: &str = "no-replies-here";

/// The rule `asked`'s reply is held by at `read_at`, with `state` saying
/// whether replies are switched off, as [`gate`] says; one it lets
/// through is held by [`NO_REPLIES`] when the reader is `mute`, as none
/// can be said, so what's written down never says a reply went.
fn reply_gate(asked: &Asked<'_>, sent: &[Sent], read: (u64, &Path), mute: bool) -> Gate {
    let (read_at, state) = read;
    match gate(asked, sent, read_at, replies_off(state)) {
        Gate::Through(_) if mute => Gate::Held(NO_REPLIES),
        gated => gated,
    }
}

/// Whether replies-switch says `off`, read afresh each time so a switch
/// acts on the next message.
fn replies_off(state: &Path) -> bool {
    std::fs::read_to_string(state.join("replies-switch"))
        .is_ok_and(|said| said.lines().any(|line| line.trim() == "off"))
}
