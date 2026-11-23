//! The safe word's reader for Slack.
//!
//! `pineapple-slack run`, as its service runs it, searches Slack every
//! half minute with Joel's own sign-in (pi's, from the keyring or its old
//! file, only read)
//! for "pineapple", and for his tag by id, which finds someone asking him
//! to stop his agents without the phrase. Each message it hasn't judged
//! yet is judged in the place it was said, its thread's first message
//! saying what the thread is about, and the decision goes in
//! `$XDG_STATE_HOME/pineapple/decisions.jsonl`, one line each; a stop also
//! gets a record of its own beside it in stops/, which stands until Joel
//! resumes it. `pineapple-slack`, or `pineapple-slack heard`, says the
//! newest decisions, in the form `--format` asks for (docs/commands.md).
//!
//! Unless `run` is given `--live`, it is in shadow: every stop's record
//! says so, it tells nobody and
//! answers nobody, and nothing acting on records takes a shadow one for a
//! stop. Each reply it would have sent is put through Joel's rules against
//! spam (src/gate.rs) all the same: the log names the rule that let it
//! through or held it, and replies.jsonl keeps those let through, which
//! the rules count, each with where it would go and what it would say
//! (src/reply.rs), or none where replies.json holds no words Joel said
//! yes to. A line `off` in replies-switch beside them turns replies off
//! alone, read each time, so stops and alerts carry on.
//!
//! It also tells Joel's phone: each alert the launcher's daemon writes to
//! would-tell.jsonl goes, in its one form, to his own note-to-self IM and
//! nowhere else (src/phone.rs), once, kept in phoned.jsonl. In shadow it
//! finds that IM, so the finding is tried against Slack as it will be,
//! and sends nothing.
//!
//! In shadow it sends nothing and only reads Slack. Out of it, its two
//! writes are such a reply, from Joel's own account, in the thread that
//! asked, and only with words he said yes to; and an alert in his own IM.
//! Slack is Slack's own, unless a test names a stand-in with `--origin`
//! and a quicker pace with `--every`.
//!
//! Given `--gateway URL` and `--gateway-sign-in PROGRAM`, it hears Slack
//! through that tool gateway instead (src/ears.rs), signed in by what
//! the program prints, and only in shadow: it tells no phone, as the
//! gateway's Slack is only read, so it can run beside the reader that
//! acts and its decisions be held against that one's.
//!
//! Which messages it has judged is kept in judged.json beside the log
//! (src/judged.rs), so a restart neither judges a message twice nor
//! misses one said while it was down. Starting with nothing kept, it
//! begins ten minutes back rather than judging Slack's whole history.

use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use pineapple::deaf::{self, Deaf, Deafness, Round, Waiting};
use pineapple::ears::{Api, Ears, Through};
use pineapple::gate::{Asked, gate};
use pineapple::judged::{self, Afresh, Judged};
use pineapple::mcp::Gateway;
use pineapple::phone;
use pineapple::record::{self, Source};
use pineapple::reply::{self, Post};
use pineapple::resume;
use pineapple::sign_in::{self, ran_over};
use pineapple::slack::{Message, micros};
use pineapple::vocabulary::Words;
use pineapple::{Context, Heard, Joel, Platform, Target, Verdict, decide};
use serde_json::{Value, json};
use slack_people::{Account, LastSignIn, Signed, Slack};
use terminal_kit::{Exit, Failure, Format, Help, flags};

mod alerts;
use alerts::Phone;

/// How often Slack is searched, unless a test says otherwise: with the
/// enforcer's few seconds after, a stop lands well inside five minutes.
const EVERY: Duration = Duration::from_secs(30);

/// How many rounds the keyring's sign-in may take, and the rest after one
/// that ran over may last, at most. At the service's pace that's ten
/// minutes, past both, so it bounds only a test's quicker pace.
const SIGN_IN_ROUNDS: u32 = 20;

/// How far back a first start looks, in seconds.
const LOOK_BACK: u64 = 10 * 60;

/// How many of the newest decisions `heard` says.
const NEWEST: usize = 20;

/// Whether Joel's resume said in Slack may end a live stop. Not yet, even
/// once the reader leaves shadow: anything posting with his sign-in reads
/// as him, so a resume an agent forged would defeat the safe word. It
/// waits until the harness's Slack-send guard is proven to stop every
/// agent posting as him (WCJQSG, 2026-11-14); his press in the launcher
/// resumes all the while.
const SLACK_RESUMES_LIVE: bool = false;

/// The replies' words, as Joel said yes to them (2026-11-14); a reply
/// missing any of them has nothing to say.
const REPLIES: &str = include_str!("../../../replies.json");

/// Milliseconds in a second.
const MILLIS: u64 = 1000;

/// Microseconds in a second.
const MICROS: u128 = 1_000_000;

/// Slack's own origin.
const SLACK: &str = "https://slack.com";

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "pineapple-slack",
        "reads Slack for the safe word that stops Joel's agents",
    )
    .example("pineapple-slack", "the newest decisions, the same as heard")
    .example("pineapple-slack heard --json", "the same, for a script")
    .subcommand(
        Help::new("heard", "the newest decisions, with where each was said")
            .example("pineapple-slack heard", "the same as bare pineapple-slack")
            .example("pineapple-slack heard --json", "the same, for a script")
            .exit(Exit::BadInput, "the log can't be read"),
    )
    .subcommand(
        Help::new("run", "the reader itself, as its service runs it")
            .example(
                "pineapple-slack run",
                "reads Slack until stopped, in shadow",
            )
            .example(
                "pineapple-slack run --live",
                "the same, its stops standing, its replies and alerts said",
            )
            .option(
                "--live",
                "acts on what it hears; only writes it down unless given",
            )
            .option("--origin URL", "asks a stand-in for Slack there")
            .option(
                "--every MS",
                "how often it searches, half a minute unless given",
            )
            .option(
                "--gateway URL",
                "hears Slack through that tool gateway, in shadow only",
            )
            .option(
                "--gateway-sign-in PROGRAM",
                "prints the gateway's sign-in, as --gateway needs",
            ),
    )
    .exit(Exit::BadInput, "the log can't be read")
}

fn main() -> ExitCode {
    let help = help();
    let commands = help.command_names();
    terminal_kit::main(&help, env!("CARGO_PKG_VERSION"), |context| {
        let (command, rest) = match context.args.split_first() {
            Some((command, rest)) => (command.as_str(), rest),
            None => ("heard", &[][..]),
        };
        match command {
            "heard" => none_after(rest).and_then(|()| heard(context)),
            "run" => {
                let how = run_options(rest)?;
                match &how.gateway {
                    Some((url, program)) => hear_through(&how, url, program),
                    None => run(&how),
                }
            }
            word => Err(flags::unknown("command", word, &commands)),
        }
    })
}

/// Nothing more on the line, where a command takes nothing; `--verbose`
/// is the kit's.
fn none_after(rest: &[String]) -> Result<(), Failure> {
    match rest
        .iter()
        .find(|arg| !matches!(arg.as_str(), "--verbose" | "-v"))
    {
        Some(arg) => Err(Failure::new(Exit::Usage, format!("{arg} isn't taken here"))
            .because("heard takes nothing more")
            .try_this("pineapple-slack --help")),
        None => Ok(()),
    }
}

/// How the reader runs: where it asks Slack, how often, and whether it
/// acts on what it hears.
struct Run {
    origin: String,
    every: Duration,
    /// Whether its stops, replies, alerts and resumes are only written
    /// down: unless its service says `--live`, they are, so nothing that
    /// runs it unasked, a test or a machine not yet let go live, acts.
    shadow: bool,
    /// The tool gateway it hears Slack through, and the program printing
    /// its sign-in, when it isn't Slack's own API.
    gateway: Option<(String, OsString)>,
}

/// How run runs, from its `--live`, `--origin`, `--every`, `--gateway`
/// and `--gateway-sign-in`.
fn run_options(rest: &[String]) -> Result<Run, Failure> {
    let mut origin = SLACK.to_owned();
    let mut every = EVERY;
    let mut shadow = true;
    let (mut gateway, mut sign_in) = (None, None);
    let mut args = rest.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--verbose" | "-v" => {}
            "--live" => shadow = false,
            "--origin" => {
                origin = args.next().cloned().ok_or_else(|| {
                    Failure::new(Exit::Usage, "--origin takes a URL")
                        .try_this("pineapple-slack run --origin http://127.0.0.1:8080")
                })?;
            }
            "--every" => {
                every = args
                    .next()
                    .and_then(|given| given.parse().ok())
                    .map(Duration::from_millis)
                    .ok_or_else(|| {
                        Failure::new(Exit::Usage, "--every takes a whole number of milliseconds")
                            .try_this("pineapple-slack run --every 500")
                    })?;
            }
            "--gateway" => gateway = Some(given(&mut args, "--gateway", "a URL")?),
            "--gateway-sign-in" => {
                sign_in = Some(given(&mut args, "--gateway-sign-in", "a program")?);
            }
            other => {
                let known = [
                    "--live",
                    "--origin",
                    "--every",
                    "--gateway",
                    "--gateway-sign-in",
                ];
                return Err(flags::unknown("option", other, &known));
            }
        }
    }
    let gateway = match (gateway, sign_in) {
        (None, None) => None,
        (Some(url), Some(program)) if shadow => Some((url, OsString::from(program))),
        (Some(_), Some(_)) => {
            return Err(
                Failure::new(Exit::Usage, "--live isn't taken with --gateway")
                    .because("a reader over a tool gateway only writes down what it hears"),
            );
        }
        _ => {
            return Err(
                Failure::new(Exit::Usage, "--gateway and --gateway-sign-in go together")
                    .try_this("pineapple-slack run --gateway URL --gateway-sign-in PROGRAM"),
            );
        }
    };
    Ok(Run {
        origin,
        every,
        shadow,
        gateway,
    })
}

/// The word after an option that takes one.
fn given<'a>(
    args: &mut impl Iterator<Item = &'a String>,
    option: &str,
    what: &str,
) -> Result<String, Failure> {
    args.next()
        .cloned()
        .ok_or_else(|| Failure::new(Exit::Usage, format!("{option} takes {what}")))
}

fn env_dir(var: &str, fallback: &str) -> PathBuf {
    std::env::var_os(var).map_or_else(
        || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(fallback),
        PathBuf::from,
    )
}

/// The folder, under the state home, where the reader over a gateway
/// keeps all it writes, beside the live reader's `pineapple`.
const GATEWAY_STATE: &str = "pineapple-gateway";

/// Where it keeps its log, its stops and what it has judged.
fn state() -> PathBuf {
    env_dir("XDG_STATE_HOME", ".local/state").join("pineapple")
}

/// The harness's old Slack sign-in file, read only when the keyring
/// `SIGN_IN_KEYRING` names holds no sign-in (../slack's `Account::signed`).
fn credentials() -> PathBuf {
    env_dir("XDG_CONFIG_HOME", ".config").join("agentic-harness/slack/credentials.json")
}

/// The newest decisions, in the form asked for.
fn heard(context: &terminal_kit::Context) -> Result<(), Failure> {
    let log = state().join("decisions.jsonl");
    let text = match std::fs::read_to_string(&log) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(
                Failure::new(Exit::BadInput, "the log can't be read").because(error.to_string())
            );
        }
    };
    let lines: Vec<Value> = text
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    let newest = &lines[lines.len().saturating_sub(NEWEST)..];
    // When Slack was last read whole, so a quiet reader is told from one
    // that isn't reading at all.
    let read_at = judged::read_at(&state());
    let written = match context.format() {
        Format::Text if newest.is_empty() => context.say(&match read_at {
            Some(at) => format!(
                "nothing heard yet; Slack was last read {} s ago\n",
                millis().saturating_sub(at) / MILLIS
            ),
            None => "nothing heard yet, and Slack hasn't been read yet\n".to_owned(),
        }),
        Format::Text => context.say(&said(newest)),
        _ => context.answer(&json!({
            "schema": "pineapple-slack.heard/1",
            "heard": newest,
            "read_at": read_at,
        })),
    };
    written.map_err(|error| {
        Failure::new(Exit::Failed, "couldn't write the answer").because(error.to_string())
    })
}

/// Decisions as lines: what each was, what it reaches, and where.
fn said(decisions: &[Value]) -> String {
    let field = |decision: &Value, key: &str| {
        decision
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let mut out = String::new();
    for decision in decisions {
        let reaches: Vec<&str> = decision
            .get("reaches")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        out.push_str(&field(decision, "verdict"));
        if !reaches.is_empty() {
            out.push_str("  ");
            out.push_str(&reaches.join(", "));
        }
        let gate = field(decision, "gate");
        if !gate.is_empty() && gate != "unasked" {
            out.push_str("  reply ");
            out.push_str(&gate);
        }
        out.push_str("\n  ");
        out.push_str(&field(decision, "link"));
        out.push('\n');
    }
    out
}

/// What either reader starts from: what it has judged, saying what of
/// that couldn't be read.
fn begin(state: &Path) -> Judged {
    let (judged, afresh) = judged::read(state, seconds().saturating_sub(LOOK_BACK));
    match afresh {
        Some(Afresh::Unreadable) => eprintln!(
            "<4>how far Slack was read can't all be read, so it reads from ten minutes back, \
             keeping what it can of what was judged"
        ),
        Some(Afresh::Unjudged) => eprintln!(
            "<4>which Slack messages were judged can't all be read, so any of the newest said \
             since it first started may be judged, and answered, again"
        ),
        Some(Afresh::Nothing) | None => {}
    }
    judged
}

/// Where the live reader tells For You it can't hear Slack, since the
/// phone's alert goes through Slack: `named`, or else where For You reads.
fn told_in(named: Option<&OsStr>) -> Waiting {
    let waiting = deaf::waiting(named, &env_dir("XDG_STATE_HOME", ".local/state"));
    if let Some(refused) = &waiting.refused {
        eprintln!(
            "<4>SAFE_WORD_WAITING names {:?}, which isn't a whole path, so For You is told in \
             {:?}, which the unit's stop won't take away",
            refused.to_string_lossy(),
            waiting.file.to_string_lossy()
        );
    }
    waiting
}

/// Tells For You whether this round heard Slack, as it changes.
fn tell_deaf(deaf: &mut Deaf, waiting: &Waiting, heard: Round) {
    let change = deaf.heard(heard, millis());
    if let Err(error) = deaf::make(&waiting.file, &change) {
        eprintln!("<3>couldn't tell Joel in For You whether Slack is heard: {error}");
        deaf.failed(&change);
    }
}

/// The reader, which runs until its service stops it.
fn run(how: &Run) -> Result<(), Failure> {
    let credentials = credentials();
    let state = state();
    let slack = Slack::new(&how.origin);
    let reader = Reader {
        origin: how.origin.clone(),
        words: Words::from_object(REPLIES).unwrap_or_default(),
        shadow: how.shadow,
    };
    let mut judged = begin(&state);
    let waiting = told_in(std::env::var_os("SAFE_WORD_WAITING").as_deref());
    let mut phone = Phone {
        origin: how.origin.clone(),
        own: None,
        looked: false,
        shadow: how.shadow,
    };
    let patience = sign_in::PATIENCE.min(how.every * SIGN_IN_ROUNDS);
    let rest = slack_people::REST.min(how.every * SIGN_IN_ROUNDS);
    // A locked keyring doesn't end a sign-in, so stops are still heard.
    let mut last = LastSignIn::new();
    let mut deaf = Deaf::new();
    loop {
        let now = Instant::now();
        let signed = Account::signed(&credentials, patience, rest);
        match &signed {
            Signed::Out => eprintln!("<4>no Slack sign-in yet, so nothing is read"),
            // Said once, as the sign-in last read is let go.
            Signed::RanOver | Signed::Locked | Signed::Resting if last.lapsed(now) => {
                eprintln!("<4>{}", slack_people::no_longer_stands());
            }
            // A locked keyring prompts each time it's asked, so it rests.
            Signed::RanOver if last.stands(now) => eprintln!(
                "<4>Slack's sign-in ran past {} seconds and was stopped, so the one last read \
                 is used till the keyring is asked again",
                patience.as_secs()
            ),
            Signed::RanOver => eprintln!(
                "<4>{}",
                ran_over("Slack", "slack", patience, rest + how.every)
            ),
            Signed::Locked => eprintln!(
                "<4>{}",
                slack_people::locked(last.stands(now), rest + how.every)
            ),
            // Said as it began, or by what follows.
            Signed::In(_) | Signed::Resting => {}
        }
        let heard = match last.heard(signed, now) {
            Some(account) if !account.user.is_empty() => {
                let mut api = Api {
                    slack: &slack,
                    account: &account,
                };
                let joel = (account.user.as_str(), Some(&account));
                let heard = round(&mut api, &reader, joel, &state, &mut judged);
                alerts::tell(&slack, &mut phone, &account, &state);
                heard
            }
            Some(_) => {
                eprintln!("<4>the Slack sign-in doesn't say who Joel is");
                Round::Deaf(Deafness::NoSignIn)
            }
            None => Round::Deaf(Deafness::NoSignIn),
        };
        tell_deaf(&mut deaf, &waiting, heard);
        std::thread::sleep(how.every);
    }
}

/// The reader over a tool gateway, in shadow, until its service stops it.
/// Whose Slack it reads, the gateway says once; until it does, nothing is.
/// It keeps to its own folder, its row of what waits included, however it
/// was started, so nothing the live reader reads is ever written; and it
/// leaves the environment as it was, for the sign-in program it runs.
fn hear_through(how: &Run, url: &str, program: &OsStr) -> Result<(), Failure> {
    let state = env_dir("XDG_STATE_HOME", ".local/state").join(GATEWAY_STATE);
    let reader = Reader {
        origin: how.origin.clone(),
        words: Words::from_object(REPLIES).unwrap_or_default(),
        shadow: true,
    };
    let mut judged = begin(&state);
    // Named here rather than as the live reader's is, whose fallback for a
    // name that isn't whole is For You's own row.
    let waiting = Waiting {
        file: state.join("waiting.json"),
        refused: None,
    };
    let mut through = Through {
        gateway: Gateway::new(url, program),
    };
    let mut me: Option<String> = None;
    let mut deaf = Deaf::new();
    loop {
        let heard = match me.clone().map_or_else(|| through.me(), Ok) {
            Ok(user) => {
                let heard = round(&mut through, &reader, (&user, None), &state, &mut judged);
                me = Some(user);
                heard
            }
            Err(round) => round,
        };
        tell_deaf(&mut deaf, &waiting, heard);
        std::thread::sleep(how.every);
    }
}

/// What the reader acts with: where replies go, the words they say, and
/// whether its stops, replies and resumes are only written down.
struct Reader {
    origin: String,
    words: Words,
    shadow: bool,
}

/// One search of each kind, and each new message judged and written down;
/// what it came to, a refused sign-in before a Slack that couldn't be asked.
/// `me` is Joel's Slack id, and his sign-in, where a reply could be said
/// with it.
fn round(
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
    let tag = format!("<@{me}>");
    // A round counts as read only once both searches answered, so
    // judged.json's read_at never says Slack was read when it wasn't. A
    // sign-in refused or missing for the first isn't asked for again; an
    // answer that couldn't be read is, so the journal says if the other
    // reads.
    let (phrase, tagged) = match ears.search("pineapple") {
        Err(deaf @ Round::Deaf(Deafness::Refused | Deafness::NoSignIn)) => return deaf,
        phrase => (phrase, ears.search(&tag)),
    };
    let (phrase, tagged) = match (phrase, tagged) {
        (Ok(phrase), Ok(tagged)) => (phrase, tagged),
        (phrase, tagged) => {
            let failed = [phrase.err(), tagged.err()];
            // A refusal is said first, then a sign-in none could give,
            // then an answer none could read.
            return [Deafness::Refused, Deafness::NoSignIn, Deafness::Unreadable]
                .into_iter()
                .map(Round::Deaf)
                .find(|deaf| failed.contains(&Some(*deaf)))
                .unwrap_or(Round::Unasked);
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
    if let Err(error) = judged::write(state, judged, millis()) {
        eprintln!("<3>couldn't keep how far it has read: {error}");
    }
    Round::Heard
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

fn since_epoch() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
}

fn seconds() -> u64 {
    since_epoch().as_secs()
}

fn millis() -> u64 {
    u64::try_from(since_epoch().as_millis()).unwrap_or(u64::MAX)
}
