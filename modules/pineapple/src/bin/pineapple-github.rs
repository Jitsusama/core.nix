//! The safe word's reader for GitHub.
//!
//! `pineapple-github run`, as its service runs it, asks GitHub every
//! minute, with Joel's own sign-in (gh's, printed by the program
//! `SIGN_IN_KEYRING` names, only read), which pull requests and issues he
//! takes part in changed, and reads what was said on each since it last
//! asked: its opening, comments, review comments and reviews. Whatever
//! holds "pineapple" or tags him is judged with the pull request it was
//! said on as its place (src/github.rs), and the decision goes in
//! `$XDG_STATE_HOME/pineapple/decisions.jsonl` beside the Slack reader's,
//! a stop with a record of its own in stops/. The launcher's daemon, the
//! harness and Joel's phone take them from there as they take Slack's.
//! `pineapple-github` says when GitHub was last read.
//!
//! It only asks GitHub, never posts, and never marks a notification read:
//! a stop said there is answered nowhere in public, and a reply the rules
//! would let through on Slack is held, so only Joel hears. Joel's own
//! "pineapple resume" said there ends only stops written in shadow, since
//! his agents post as his GitHub account all day and one could forge it;
//! his press in the launcher resumes the rest.
//!
//! A round spends Joel's rate limit, which gh and his agents share: two
//! requests, and two to four more for each pull request or issue that
//! changed, a page more for each hundred of a long list. One his limit
//! refuses is tried again whole, so nothing it couldn't read is skipped.
//!
//! Unless `run` is given `--live`, it is in shadow: every stop's record
//! says so, and nothing acting on records takes a shadow one for a stop.
//! GitHub is GitHub's own, unless a test names a stand-in with `--origin`
//! and a quicker pace with `--every`.
//!
//! How far it has read is kept in judged-github.json: from when, and the
//! keys of what it judged, so a restart neither judges anything twice nor
//! misses what was said while it was down. Starting with nothing kept, it
//! begins ten minutes back rather than judging GitHub's whole history.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use pineapple::gate::Gate;
use pineapple::github::{self, Kind, Said, Thread, notified, worth_judging};
use pineapple::record::{self, Source};
use pineapple::resume;
use pineapple::{Context, Heard, Joel, Platform, Reply, Target, Verdict, decide};
use serde_json::{Value, json};
use terminal_kit::{Exit, Failure, Format, Help, flags};
use ureq::Agent;

/// How often GitHub is asked, unless a test says otherwise: with the
/// enforcer's few seconds after, a stop lands well inside five minutes.
const EVERY: Duration = Duration::from_secs(60);

/// How far back a first start looks, in seconds.
const LOOK_BACK: u64 = 10 * 60;

/// How far before the last round each round asks from, in seconds, since
/// GitHub can show a comment a little after the time it gives it. What
/// that finds twice is known by its key.
const OVERLAP: u64 = 60;

/// How long one request may take before the round gives up on it.
const PATIENCE: Duration = Duration::from_secs(20);

/// GitHub's own API.
const GITHUB: &str = "https://api.github.com";

/// Whether Joel's resume said on GitHub may end a live stop. Not while his
/// agents post as his GitHub account: a resume one forged would defeat
/// the safe word (WCJQSG, 2026-11-14, as for Slack's).
const GITHUB_RESUMES_LIVE: bool = false;

/// The rule a reply is held by on GitHub, where none is said in public.
const NO_REPLIES: &str = "no-replies-here";

/// How many of a list one request asks for, GitHub's most.
const PAGE: &str = "100";

/// The most pages one list is read to: three thousand of anything, far
/// past a night's notifications, so a list naming pages without end
/// can't hold a round.
const PAGES: usize = 30;

/// What it keeps of how far it has read, beside the Slack reader's.
const JUDGED: &str = "judged-github.json";

/// How many judged keys are kept: far more than a few rounds find.
const KEPT: usize = 1000;

/// Milliseconds in a second.
const MILLIS: u64 = 1000;

/// Microseconds in a second.
const MICROS: u128 = 1_000_000;

/// How far it has read: from `since`, in seconds, the keys of what it
/// judged, newest last.
struct Judged {
    since: u64,
    keys: Vec<String>,
}

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "pineapple-github",
        "reads GitHub for the safe word that stops Joel's agents",
    )
    .example("pineapple-github", "when GitHub was last read")
    .example("pineapple-github --json", "the same, for a script")
    .subcommand(
        Help::new("run", "the reader itself, as its service runs it")
            .example(
                "pineapple-github run",
                "reads GitHub until stopped, in shadow",
            )
            .example(
                "pineapple-github run --live",
                "the same, its stops standing",
            )
            .option(
                "--live",
                "acts on what it hears; only writes it down unless given",
            )
            .option("--origin URL", "asks a stand-in for GitHub there")
            .option("--every MS", "how often it asks, a minute unless given"),
    )
}

fn main() -> ExitCode {
    let help = help();
    let commands = help.command_names();
    terminal_kit::main(&help, env!("CARGO_PKG_VERSION"), |context| {
        match context.args.split_first() {
            None => last_read(context),
            Some((command, rest)) if command == "run" => run(&run_options(rest)?),
            Some((word, _)) if word == "--verbose" || word == "-v" => last_read(context),
            Some((word, _)) => Err(flags::unknown("command", word, &commands)),
        }
    })
}

/// How the reader runs: where it asks GitHub, how often, and whether it
/// acts on what it hears.
struct Run {
    origin: String,
    every: Duration,
    /// Whether its stops and resumes are only written down: unless its
    /// service says `--live`, they are, so nothing that runs it unasked
    /// acts.
    shadow: bool,
}

/// How run runs, from its `--live`, `--origin` and `--every`.
fn run_options(rest: &[String]) -> Result<Run, Failure> {
    let mut how = Run {
        origin: GITHUB.to_owned(),
        every: EVERY,
        shadow: true,
    };
    let mut args = rest.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--verbose" | "-v" => {}
            "--live" => how.shadow = false,
            "--origin" => {
                how.origin = args.next().cloned().ok_or_else(|| {
                    Failure::new(Exit::Usage, "--origin takes a URL")
                        .try_this("pineapple-github run --origin http://127.0.0.1:8080")
                })?;
            }
            "--every" => {
                how.every = args
                    .next()
                    .and_then(|given| given.parse().ok())
                    .map(Duration::from_millis)
                    .ok_or_else(|| {
                        Failure::new(Exit::Usage, "--every takes a whole number of milliseconds")
                            .try_this("pineapple-github run --every 500")
                    })?;
            }
            other => {
                let known = ["--live", "--origin", "--every"];
                return Err(flags::unknown("option", other, &known));
            }
        }
    }
    Ok(how)
}

/// Where it keeps what it judged, beside the Slack reader's log and stops.
fn state() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map_or_else(
            || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state"),
            PathBuf::from,
        )
        .join("pineapple")
}

/// When GitHub was last read whole, so a quiet reader is told from one
/// that isn't reading at all.
fn last_read(context: &terminal_kit::Context) -> Result<(), Failure> {
    let read_at = std::fs::read_to_string(state().join(JUDGED))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|kept| kept.get("read_at").and_then(Value::as_u64));
    let written = match (context.format(), read_at) {
        (Format::Text, Some(at)) => context.say(&format!(
            "GitHub was last read {} s ago\n",
            millis().saturating_sub(at) / MILLIS
        )),
        (Format::Text, None) => context.say("GitHub hasn't been read yet\n"),
        _ => context.answer(&json!({"schema": "pineapple-github.read/1", "read_at": read_at})),
    };
    written.map_err(|error| {
        Failure::new(Exit::Failed, "couldn't write the answer").because(error.to_string())
    })
}

/// The reader, which runs until its service stops it.
fn run(how: &Run) -> Result<(), Failure> {
    let state = state();
    let mut judged = read_judged(&state).unwrap_or_else(|| Judged {
        since: seconds().saturating_sub(LOOK_BACK),
        keys: Vec::new(),
    });
    let agent: Agent = Agent::config_builder()
        .timeout_global(Some(PATIENCE))
        .http_status_as_error(false)
        .build()
        .into();
    loop {
        match token() {
            Some(token) => {
                let github = GitHub {
                    origin: &how.origin,
                    token: &token,
                    agent: &agent,
                };
                round(&github, &state, &mut judged, how.shadow);
            }
            None => eprintln!("<4>no GitHub sign-in yet, so nothing is read"),
        }
        std::thread::sleep(how.every);
    }
}

/// Joel's GitHub sign-in, as the program `SIGN_IN_KEYRING` names prints
/// it, read each round so one renewed while it runs is read without a
/// restart. None when it can't be run, fails or prints nothing; what it
/// says otherwise is let go, so no part of a sign-in reaches the journal.
fn token() -> Option<String> {
    let keyring = std::env::var_os("SIGN_IN_KEYRING")?;
    let out = Command::new(keyring)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    let token = text.trim();
    (out.status.success() && !token.is_empty()).then(|| token.to_owned())
}

/// Why GitHub didn't answer, as far as a round cares.
enum Unread {
    /// What was asked for is gone, or not Joel's to read: the round goes
    /// on without it.
    Gone(String),
    /// GitHub couldn't be asked now, its rate limit included: the round
    /// is tried again whole.
    Failed(String),
}

/// GitHub, asked as Joel.
struct GitHub<'a> {
    origin: &'a str,
    token: &'a str,
    agent: &'a Agent,
}

impl GitHub<'_> {
    /// One GET of a path, and its answer.
    fn get(&self, path: &str, query: &[(&str, &str)]) -> Result<Value, Unread> {
        let url = format!("{}{path}", self.origin);
        self.ask(&url, query).map(|(said, _)| said)
    }

    /// A whole list, page after page as GitHub's `Link` header names them,
    /// so a long one isn't cut at its first hundred. A next page anywhere
    /// but GitHub's own origin isn't asked, so Joel's sign-in goes nowhere
    /// else, and the round is tried again.
    fn all(&self, path: &str, query: &[(&str, &str)]) -> Result<Value, Unread> {
        let (said, mut next) = self.ask(&format!("{}{path}", self.origin), query)?;
        let Value::Array(mut list) = said else {
            return Ok(said);
        };
        let own = format!("{}/", self.origin);
        let mut pages = 1;
        while let Some(url) = next {
            if !url.starts_with(&own) {
                let why = format!("GitHub named a next page elsewhere: {url}");
                return Err(Unread::Failed(why));
            }
            if pages == PAGES {
                eprintln!("<4>{path} runs past {PAGES} pages, and the rest isn't read");
                break;
            }
            let (more, after) = self.ask(&url, &[])?;
            list.extend(more.as_array().into_iter().flatten().cloned());
            next = after;
            pages += 1;
        }
        Ok(Value::Array(list))
    }

    /// One GET of a URL on GitHub's origin: its answer, and the next page
    /// when it's one of a list.
    fn ask(&self, url: &str, query: &[(&str, &str)]) -> Result<(Value, Option<String>), Unread> {
        let path = url.strip_prefix(self.origin).unwrap_or(url);
        let mut request = self.agent.get(url);
        for (name, value) in query {
            request = request.query(name, value);
        }
        let mut answer = request
            .header("Authorization", &format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "pineapple-github")
            .call()
            .map_err(|error| Unread::Failed(format!("couldn't reach GitHub: {error}")))?;
        let status = answer.status().as_u16();
        let header = |name: &str| {
            let value = answer.headers().get(name)?;
            value.to_str().ok().map(str::to_owned)
        };
        let next = header("link")
            .as_deref()
            .and_then(github::next_page)
            .map(str::to_owned);
        let remaining = header("x-ratelimit-remaining");
        let limited = github::rate_limited(
            status,
            remaining.as_deref(),
            header("retry-after").is_some(),
        );
        let said: Value = answer
            .body_mut()
            .read_json()
            .map_err(|error| Unread::Failed(format!("GitHub answered {status}: {error}")))?;
        let why = || {
            let message = said["message"].as_str().unwrap_or("no reason");
            format!("GitHub answered {status} for {path}: {message}")
        };
        match status {
            200 => Ok((said, next)),
            _ if limited => Err(Unread::Failed(format!("{}, its rate limit", why()))),
            403 | 404 | 410 => Err(Unread::Gone(why())),
            _ => Err(Unread::Failed(why())),
        }
    }
}

/// One round: what changed since the last, judged oldest first, and how
/// far it has read moved on only once every pull request and issue it
/// named was read.
fn round(github: &GitHub<'_>, state: &Path, judged: &mut Judged, shadow: bool) {
    let began = seconds();
    let login = match github.get("/user", &[]) {
        Ok(user) => user["login"].as_str().map(str::to_owned),
        Err(Unread::Gone(why) | Unread::Failed(why)) => {
            eprintln!("<4>couldn't ask GitHub who Joel is: {why}");
            return;
        }
    };
    let Some(login) = login else {
        eprintln!("<4>GitHub didn't say who Joel is");
        return;
    };
    let from = judged.since.saturating_sub(OVERLAP);
    let since = github::stamp(from);
    // Read ones too, since Joel may read one on his phone before a round.
    let asked = [
        ("participating", "true"),
        ("all", "true"),
        ("since", since.as_str()),
        ("per_page", PAGE),
    ];
    let threads = match github.all("/notifications", &asked) {
        Ok(answer) => notified(&answer),
        Err(Unread::Gone(why) | Unread::Failed(why)) => {
            eprintln!("<4>couldn't ask GitHub what changed: {why}");
            return;
        }
    };
    let mut heard = Vec::new();
    for thread in &threads {
        match said_on(github, thread, &since) {
            Ok(found) => heard.extend(found),
            Err(Unread::Gone(why)) => eprintln!("<4>{} can't be read: {why}", thread.key()),
            Err(Unread::Failed(why)) => {
                eprintln!("<4>couldn't read {}: {why}", thread.key());
                return;
            }
        }
    }
    heard.retain(|(said, ..)| {
        said.at >= from && !judged.keys.contains(&said.key) && worth_judging(&said.text, &login)
    });
    heard.sort_by_key(|(said, ..)| said.at);
    let joel = Joel {
        slack_ids: Vec::new(),
        logins: vec![login],
        emails: Vec::new(),
    };
    for (said, thread, context) in heard {
        judge(state, &joel, &said, &thread, context, shadow);
        judged.keys.push(said.key);
    }
    judged.since = began;
    let over = judged.keys.len().saturating_sub(KEPT);
    judged.keys.drain(..over);
    write_judged(state, judged);
}

/// What was said on a pull request or issue since `since`, each with it
/// and the place it's about. Reviews can't be asked for by time, so all
/// are read and the round keeps those since.
fn said_on(
    github: &GitHub<'_>,
    thread: &Thread,
    since: &str,
) -> Result<Vec<(Said, Thread, Context)>, Unread> {
    let base = format!("/repos/{}", thread.repo);
    let number = thread.number;
    let opened = github.get(&format!("{base}/issues/{number}"), &[])?;
    let opening = format!(
        "{}\n\n{}",
        opened["title"].as_str().unwrap_or_default(),
        opened["body"].as_str().unwrap_or_default()
    );
    let context = thread.context(&opening);
    let recent = [("since", since), ("per_page", PAGE)];
    let mut found = github::said(&opened, thread, Kind::Opening);
    let comments = github.all(&format!("{base}/issues/{number}/comments"), &recent)?;
    found.extend(github::said(&comments, thread, Kind::Comment));
    if thread.pull {
        let on_lines = github.all(&format!("{base}/pulls/{number}/comments"), &recent)?;
        found.extend(github::said(&on_lines, thread, Kind::ReviewComment));
        let reviews = github.all(
            &format!("{base}/pulls/{number}/reviews"),
            &[("per_page", PAGE)],
        )?;
        found.extend(github::said(&reviews, thread, Kind::Review));
    }
    Ok(found
        .into_iter()
        .map(|said| (said, thread.clone(), context.clone()))
        .collect())
}

/// One thing said, judged on the pull request or issue it was said on,
/// and written down.
fn judge(state: &Path, joel: &Joel, said: &Said, thread: &Thread, context: Context, shadow: bool) {
    let decision = decide(
        &Heard {
            platform: Platform::Markdown,
            author: said.author.clone(),
            by_bot: said.by_bot,
            text: said.text.clone(),
            context,
        },
        joel,
    );
    let thread_key = thread.key();
    // As Slack writes its times, so a resume compares them alike.
    let said_at = format!("{}.000000", said.at);
    let source = Source {
        key: &said.key,
        thread: &thread_key,
        link: &said.link,
        author: &said.author,
        text: &said.text,
        said_at: &said_at,
    };
    let now = millis();
    let gate = if decision.reply == Reply::Nothing {
        Gate::Unasked
    } else {
        Gate::Held(NO_REPLIES)
    };
    let line = record::line(&source, &decision, gate, now, shadow);
    if let Err(error) = record::append(&state.join("decisions.jsonl"), &line) {
        eprintln!("<3>couldn't write a decision down: {error}");
    }
    if decision.stops() {
        let stop = record::stop(&source, &decision, now, shadow);
        match record::keep(&state.join("stops"), &said.key, &stop) {
            Ok(true) if shadow => eprintln!("<5>a stop, in shadow: {}", said.link),
            Ok(true) => eprintln!("<5>a stop: {}", said.link),
            Ok(false) => {}
            Err(error) => eprintln!("<3>couldn't keep a stop's record: {error}"),
        }
    }
    if decision.verdict == Verdict::Resume && decision.by_joel {
        let at = (now, u128::from(said.at) * MICROS);
        resume_as_joel(&state.join("stops"), &decision.reaches, at, &said.link);
    }
}

/// Ends the stops Joel's own "pineapple resume" names (src/resume.rs),
/// only those written in shadow until GitHub resumes are let go live.
/// `at` is when it's heard, in milliseconds, and when it was said, in
/// microseconds.
fn resume_as_joel(stops: &Path, reaches: &[Target], at: (u64, u128), link: &str) {
    let (now, said) = at;
    let shadow = !GITHUB_RESUMES_LIVE;
    for reach in reaches {
        let scope = match reach {
            Target::Everything => "all".to_owned(),
            named => named.to_string(),
        };
        match resume::as_joel(stops, &scope, now, said, shadow) {
            Ok(ids) if ids.is_empty() => {}
            Ok(ids) if shadow => eprintln!("<5>a resume, in shadow, of {}: {link}", ids.join(", ")),
            Ok(ids) => eprintln!("<5>Joel resumed {}: {link}", ids.join(", ")),
            Err(error) => eprintln!("<3>couldn't resume what Joel named: {error}"),
        }
    }
}

fn read_judged(state: &Path) -> Option<Judged> {
    let text = std::fs::read_to_string(state.join(JUDGED)).ok()?;
    let kept: Value = serde_json::from_str(&text).ok()?;
    let since = kept.get("since").and_then(Value::as_u64)?;
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

/// Keeps how far it has read, and when GitHub was read, through a file
/// renamed into place.
fn write_judged(state: &Path, judged: &Judged) {
    let kept = json!({ "since": judged.since, "keys": judged.keys, "read_at": millis() });
    let path = state.join(JUDGED);
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
