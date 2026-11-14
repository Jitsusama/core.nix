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
//! It is in shadow: every stop's record says so, it tells nobody and
//! answers nobody, and nothing acting on records takes a shadow one for a
//! stop. Each reply it would have sent is put through Joel's rules against
//! spam (src/gate.rs) all the same: the log names the rule that let it
//! through or held it, and replies.jsonl keeps those let through, which
//! the rules count. A line `off` in replies-switch beside them turns
//! replies off alone, read each time, so stops and alerts carry on.
//!
//! It never sends anything. It only ever reads Slack: Slack's own, unless a test names a
//! stand-in with `--origin` and a quicker pace with `--every`.
//!
//! Which messages it has judged is kept in judged.json beside the log, so a restart
//! neither judges a message twice nor misses one said while it was down,
//! as far back as a search's twenty newest reach. It goes by the messages
//! rather than by the newest one's time, since Slack's search can find a
//! message a while after a newer one. Starting with nothing kept, it
//! begins ten minutes back rather than judging Slack's whole history.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use pineapple::gate::{Asked, gate};
use pineapple::record::{self, Source};
use pineapple::resume;
use pineapple::slack::{Message, found, micros, root};
use pineapple::{Context, Heard, Joel, Platform, decide};
use serde_json::{Value, json};
use slack_people::{Account, Slack};
use terminal_kit::{Exit, Failure, Format, Help, flags};

/// How often Slack is searched, unless a test says otherwise: with the
/// enforcer's few seconds after, a stop lands well inside five minutes.
const EVERY: Duration = Duration::from_secs(30);

/// How far back a first start looks, in seconds.
const LOOK_BACK: u64 = 10 * 60;

/// How many of the newest matches each search asks for.
const COUNT: &str = "20";

/// How many of the newest decisions `heard` says.
const NEWEST: usize = 20;

/// Whether its stops are only written down: until Joel has read two days
/// of its log and said so, they are (the plan's item 10).
const SHADOW: bool = true;

/// Milliseconds in a second.
const MILLIS: u64 = 1000;

/// Microseconds in a second.
const MICROS: u128 = 1_000_000;

/// Slack's own origin.
const SLACK: &str = "https://slack.com";

/// How many judged messages are kept: far more than two searches' twenty
/// newest, so one of those is never forgotten while it's still found.
const KEPT: usize = 1000;

/// Which messages it has judged: those since `since`, by their keys,
/// newest last.
struct Judged {
    since: String,
    keys: Vec<String>,
}

impl Judged {
    /// Whether a message is yet to be judged.
    fn is_new(&self, message: &Message) -> bool {
        micros(&message.ts) > micros(&self.since) && !self.keys.contains(&message.key())
    }
}

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "pineapple-slack",
        "reads Slack for the safe word that stops Joel's agents, in shadow",
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
            .example("pineapple-slack run", "reads Slack until stopped")
            .option("--origin URL", "asks a stand-in for Slack there")
            .option(
                "--every MS",
                "how often it searches, half a minute unless given",
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
                let (origin, every) = run_options(rest)?;
                run(&origin, every)
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

/// Where run asks Slack, and how often, from its `--origin` and `--every`.
fn run_options(rest: &[String]) -> Result<(String, Duration), Failure> {
    let mut origin = SLACK.to_owned();
    let mut every = EVERY;
    let mut args = rest.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--verbose" | "-v" => {}
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
            other => return Err(flags::unknown("option", other, &["--origin", "--every"])),
        }
    }
    Ok((origin, every))
}

fn env_dir(var: &str, fallback: &str) -> PathBuf {
    std::env::var_os(var).map_or_else(
        || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(fallback),
        PathBuf::from,
    )
}

/// Where it keeps its log, its stops and what it has judged.
fn state() -> PathBuf {
    env_dir("XDG_STATE_HOME", ".local/state").join("pineapple")
}

/// The harness's old Slack sign-in file, read only when the keyring
/// `SIGN_IN_KEYRING` names holds no sign-in (../slack's `Account::read`).
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
    let read_at = std::fs::read_to_string(state().join("judged.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|kept| kept.get("read_at").and_then(Value::as_u64));
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

/// The reader, which runs until its service stops it.
fn run(origin: &str, every: Duration) -> Result<(), Failure> {
    let credentials = credentials();
    let state = state();
    let slack = Slack::new(origin);
    let mut judged = read_judged(&state).unwrap_or_else(|| Judged {
        since: format!("{}.000000", seconds().saturating_sub(LOOK_BACK)),
        keys: Vec::new(),
    });
    loop {
        match Account::read(&credentials) {
            Some(account) if !account.user.is_empty() => {
                round(&slack, &account, &state, &mut judged);
            }
            Some(_) => eprintln!("<4>the Slack sign-in doesn't say who Joel is"),
            None => eprintln!("<4>no Slack sign-in yet, so nothing is read"),
        }
        std::thread::sleep(every);
    }
}

/// One search of each kind, and each new message judged and written down.
fn round(slack: &Slack, account: &Account, state: &Path, judged: &mut Judged) {
    let joel = Joel {
        slack_ids: vec![account.user.clone()],
        logins: Vec::new(),
        emails: Vec::new(),
    };
    let tag = format!("<@{}>", account.user);
    // A round counts as read only once both searches answered, so
    // judged.json's read_at never says Slack was read when it wasn't.
    let (Some(phrase), Some(tagged)) = (
        search(slack, account, "pineapple"),
        search(slack, account, &tag),
    ) else {
        return;
    };
    let mut seen = HashSet::new();
    let mut heard: Vec<&Message> = phrase
        .iter()
        .chain(&tagged)
        .filter(|message| judged.is_new(message) && seen.insert(message.key()))
        .collect();
    heard.sort_by_key(|message| micros(&message.ts));
    for message in heard {
        judge(slack, account, state, &joel, message);
        judged.keys.push(message.key());
    }
    let over = judged.keys.len().saturating_sub(KEPT);
    judged.keys.drain(..over);
    write_judged(state, judged);
}

/// The newest messages a search finds, oldest first; nothing when Slack
/// can't be asked now, which the next round tries again.
fn search(slack: &Slack, account: &Account, query: &str) -> Option<Vec<Message>> {
    let asked = [
        ("query", query),
        ("count", COUNT),
        ("sort", "timestamp"),
        ("sort_dir", "desc"),
        ("highlight", "false"),
    ];
    let said = match slack.read(account, "search.messages", &asked) {
        Ok(said) => said,
        Err(why) => {
            eprintln!("<4>couldn't search Slack: {why:?}");
            return None;
        }
    };
    match found(&said) {
        Ok(messages) => Some(messages),
        Err(why) => {
            eprintln!("<4>Slack refused a search: {why}");
            None
        }
    }
}

/// One message judged where it was said, and written down.
fn judge(slack: &Slack, account: &Account, state: &Path, joel: &Joel, message: &Message) {
    let context = message
        .thread
        .as_deref()
        .and_then(|thread| thread_root(slack, account, &message.channel, thread))
        .map(|root| Context::of_thread(&root))
        .unwrap_or_default();
    let decision = decide(
        &Heard {
            platform: Platform::Slack,
            author: message.author.clone(),
            by_bot: message.by_bot,
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
    let replies = state.join("replies.jsonl");
    // A stop Joel resumed no longer holds its thread's reply back.
    let sent = record::ledger(&resume::cleared(
        &std::fs::read_to_string(&replies).unwrap_or_default(),
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
    let line = record::line(&source, &decision, gated, now, SHADOW);
    if let Err(error) = record::append(&state.join("decisions.jsonl"), &line) {
        eprintln!("<3>couldn't write a decision down: {error}");
    }
    if gated.goes() {
        let one = record::sent(&source, decision.reply, read_at, SHADOW);
        match record::append(&replies, &one) {
            Ok(()) => eprintln!("<5>a reply, in shadow: {}", message.link),
            Err(error) => eprintln!("<3>couldn't keep a reply's line: {error}"),
        }
    }
    if decision.stops() {
        let stop = record::stop(&source, &decision, now, SHADOW);
        match record::keep(&state.join("stops"), &key, &stop) {
            Ok(true) => eprintln!("<5>a stop, in shadow: {}", message.link),
            Ok(false) => {}
            Err(error) => eprintln!("<3>couldn't keep a stop's record: {error}"),
        }
    }
}

/// What the message starting a thread says, or none when Slack won't say.
fn thread_root(slack: &Slack, account: &Account, channel: &str, thread: &str) -> Option<String> {
    let asked = [("channel", channel), ("ts", thread), ("limit", "1")];
    match slack.read(account, "conversations.replies", &asked) {
        Ok(said) => root(&said),
        Err(why) => {
            eprintln!("<4>couldn't read a thread's first message: {why:?}");
            None
        }
    }
}

/// Whether replies-switch says `off`, read afresh each time so a switch
/// acts on the next message.
fn replies_off(state: &Path) -> bool {
    std::fs::read_to_string(state.join("replies-switch"))
        .is_ok_and(|said| said.lines().any(|line| line.trim() == "off"))
}

fn read_judged(state: &Path) -> Option<Judged> {
    let text = std::fs::read_to_string(state.join("judged.json")).ok()?;
    let kept: Value = serde_json::from_str(&text).ok()?;
    let since = kept.get("since").and_then(Value::as_str)?.to_owned();
    let keys = kept
        .get("keys")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    Some(Judged { since, keys })
}

/// Keeps what was judged whole, and when Slack was read, through a file
/// renamed into place. Written every round, a few hundred bytes each 30 s.
fn write_judged(state: &Path, judged: &Judged) {
    let kept = json!({ "since": judged.since, "keys": judged.keys, "read_at": millis() });
    let path = state.join("judged.json");
    let written = path.with_extension("new");
    let wrote = std::fs::create_dir_all(state)
        .and_then(|()| std::fs::write(&written, kept.to_string()))
        .and_then(|()| std::fs::rename(&written, &path));
    if let Err(error) = wrote {
        eprintln!("<3>couldn't keep how far it has read: {error}");
    }
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
