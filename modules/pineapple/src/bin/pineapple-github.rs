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
//! his press in the launcher resumes the rest. Since any account may
//! comment on GitHub, and own a repository to mention him in, only a
//! repository's owner, its organization's members and its collaborators
//! stop him there, and only on one he works in or opened something on;
//! anyone else's words go in the log alone, and an outsider's opening
//! names no place, so it can't steer a stop elsewhere.
//!
//! A round spends Joel's rate limit, which gh and his agents share: two
//! requests, two to four more for each pull request or issue that changed,
//! one more for each repository where something on one is to be judged,
//! and a page more for each hundred of a long list. One his limit
//! refuses is tried again whole, so nothing it couldn't read is skipped.
//!
//! Unless `run` is given `--live`, it is in shadow: every stop's record
//! says so, and nothing acting on records takes a shadow one for a stop.
//! GitHub is GitHub's own, unless a test names a stand-in with `--origin`
//! and a quicker pace with `--every`.
//!
//! Another site speaking GitHub's API is read the same way, named by
//! `--origin` and `--site` so its keys and what it keeps are its own. One
//! with no notifications to ask, nor a way to ask who Joel is, is given
//! his address with `--authored-by`, his login with `--login`, and each
//! repository to search with `--repo`, since such a site searches only
//! within one: each round then searches for the pull requests he opened
//! there that changed, and what he says is known by his address. A pull
//! request he only commented on isn't read there.
//!
//! How far it has read is kept as src/progress.rs says. Starting with
//! nothing kept, it begins ten minutes back rather than judging GitHub's
//! whole history.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use pineapple::gate::Gate;
use pineapple::github::{self, Kind, Said, Thread, add_page, found, notified, worth_judging};
use pineapple::progress::{self, Judged};
use pineapple::record::{self, Source};
use pineapple::resume;
use pineapple::sign_in::{self, SignIn};
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

/// How far before the last round a search asks from, in seconds: a
/// search's index can lag an update by more than a minute, and a pull
/// request it indexed late would otherwise never be read. What that
/// finds again is known by its key.
const INDEX_LAG: u64 = 10 * 60;

/// The most repositories one reader searches. A site allows a signed-in
/// reader 30 searches a minute, and each round spends one a repository,
/// and one more for each page past the first; past that, every round
/// would be refused and the reader would never move on.
const SEARCHED: usize = 20;

/// How long one request may take before the round gives up on it.
const PATIENCE: Duration = Duration::from_secs(20);

/// GitHub's own API.
const GITHUB: &str = "https://api.github.com";

/// How long a reader whose sign-in ran over waits before asking again,
/// beyond its round's pace: long enough that a keyring's unlock prompt
/// isn't raised again each minute, short against a stop going unheard.
const RAN_OVER_REST: Duration = Duration::from_mins(9);

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

/// The site it reads unless told another.
const SITE: &str = "github";

/// How many judged keys are kept: far more than a few rounds find.
const KEPT: usize = 1000;

/// Milliseconds in a second.
const MILLIS: u64 = 1000;

/// Microseconds in a second.
const MICROS: u128 = 1_000_000;

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
            .option(
                "--origin URL",
                "asks another site speaking GitHub's API, or a stand-in",
            )
            .option(
                "--site NAME",
                "names that site in its keys and what it keeps",
            )
            .option(
                "--authored-by ADDRESS",
                "searches for Joel's pull requests there, for a site with no notifications",
            )
            .option(
                "--login LOGIN",
                "Joel's login there, for a site that won't say",
            )
            .option(
                "--repo OWNER/NAME",
                "a repository searched by author, given once for each, at most 20",
            )
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
    /// The site's name in keys and in what it keeps, so a site speaking
    /// GitHub's API beside GitHub is read apart from it.
    site: String,
    /// For a site with no notifications, Joel's address, by which it
    /// searches for the pull requests he opened instead.
    authored_by: Option<String>,
    /// Joel's login, for a site that can't be asked who he is.
    login: Option<String>,
    /// The repositories searched by author, one search each, since such a
    /// site searches only within one.
    repos: Vec<String>,
    every: Duration,
    /// Whether its stops and resumes are only written down: unless its
    /// service says `--live`, they are, so nothing that runs it unasked
    /// acts.
    shadow: bool,
}

/// How run runs, from its `--live`, `--origin`, `--site`, `--authored-by`,
/// `--login`, `--repo` and `--every`.
fn run_options(rest: &[String]) -> Result<Run, Failure> {
    let mut how = Run {
        origin: GITHUB.to_owned(),
        site: SITE.to_owned(),
        authored_by: None,
        login: None,
        repos: Vec::new(),
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
            "--site" => {
                how.site = args
                    .next()
                    .filter(|name| a_site(name))
                    .cloned()
                    .ok_or_else(|| {
                        Failure::new(Exit::Usage, "--site takes a name in small letters")
                            .try_this("pineapple-github run --site code")
                    })?;
            }
            "--authored-by" => {
                how.authored_by = Some(args.next().cloned().ok_or_else(|| {
                    Failure::new(Exit::Usage, "--authored-by takes Joel's address there")
                        .try_this("pineapple-github run --authored-by joel@example.com")
                })?);
            }
            "--login" => {
                how.login = Some(args.next().cloned().ok_or_else(|| {
                    Failure::new(Exit::Usage, "--login takes Joel's login there")
                        .try_this("pineapple-github run --login joel.example")
                })?);
            }
            "--repo" => {
                let repo = args.next().filter(|repo| repo.split('/').count() == 2);
                how.repos.push(repo.cloned().ok_or_else(|| {
                    Failure::new(Exit::Usage, "--repo takes a repository as OWNER/NAME")
                        .try_this("pineapple-github run --repo example/shop")
                })?);
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
                let known = [
                    "--live",
                    "--origin",
                    "--site",
                    "--authored-by",
                    "--login",
                    "--repo",
                    "--every",
                ];
                return Err(flags::unknown("option", other, &known));
            }
        }
    }
    if how.authored_by.is_some() && (how.login.is_none() || how.repos.is_empty()) {
        return Err(
            Failure::new(Exit::Usage, "--authored-by needs --login and a --repo")
                .because("a site searched by author can't say who Joel is, nor search it all")
                .try_this(
                    "pineapple-github run --authored-by joel@example.com --login joel \
                     --repo example/shop",
                ),
        );
    }
    if how.repos.len() > SEARCHED {
        return Err(Failure::new(
            Exit::Usage,
            format!("--repo is given at most {SEARCHED} times"),
        )
        .because("a site allows 30 searches a minute, and each round spends one a repository"));
    }
    Ok(how)
}

/// Whether a site's name can name a file: small letters, digits and dashes.
fn a_site(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// A site's name as Joel reads it.
fn shown(site: &str) -> String {
    if site == SITE {
        return "GitHub".to_owned();
    }
    let mut letters = site.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(letters).collect()
    })
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

/// When each site was last read whole, so a quiet reader is told from one
/// that isn't reading at all.
fn last_read(context: &terminal_kit::Context) -> Result<(), Failure> {
    let sites = progress::sites(&state());
    let github = sites
        .iter()
        .find(|(site, _)| site == SITE)
        .map(|(_, at)| *at);
    let written = match context.format() {
        Format::Text if sites.is_empty() => context.say("GitHub hasn't been read yet\n"),
        Format::Text => {
            let said: String = sites
                .iter()
                .map(|(site, at)| {
                    let ago = millis().saturating_sub(*at) / MILLIS;
                    format!("{} was last read {ago} s ago\n", shown(site))
                })
                .collect::<Vec<_>>()
                .concat();
            context.say(&said)
        }
        _ => {
            let each: serde_json::Map<String, Value> = sites
                .iter()
                .map(|(site, at)| (site.clone(), json!(at)))
                .collect();
            context.answer(&json!({"schema": "pineapple-github.read/1",
                "read_at": github, "sites": each}))
        }
    };
    written.map_err(|error| {
        Failure::new(Exit::Failed, "couldn't write the answer").because(error.to_string())
    })
}

/// The reader, which runs until its service stops it.
fn run(how: &Run) -> Result<(), Failure> {
    let state = state();
    let mut judged = progress::read(&state, &how.site).unwrap_or_else(|| Judged {
        since: seconds().saturating_sub(LOOK_BACK),
        keys: Vec::new(),
    });
    let agent: Agent = Agent::config_builder()
        .timeout_global(Some(PATIENCE))
        .http_status_as_error(false)
        .build()
        .into();
    let name = shown(&how.site);
    loop {
        match token() {
            SignIn::Token(token) => {
                let github = GitHub {
                    origin: &how.origin,
                    name: &name,
                    token: &token,
                    agent: &agent,
                };
                round(&github, how, &state, &mut judged);
            }
            SignIn::Missing => eprintln!("<4>no {name} sign-in yet, so nothing is read"),
            SignIn::RanOver => {
                // A locked keyring prompts each time it's asked (RAN_OVER_REST).
                let rest = RAN_OVER_REST + how.every;
                eprintln!("<4>{}", sign_in::ran_over(&name, &how.site, rest));
                std::thread::sleep(RAN_OVER_REST);
            }
        }
        std::thread::sleep(how.every);
    }
}

/// Joel's sign-in to the site, as the program `SIGN_IN_KEYRING` names prints
/// it, each round, never waited on for good (pineapple's `sign_in`).
fn token() -> SignIn {
    std::env::var_os("SIGN_IN_KEYRING").map_or(SignIn::Missing, |program| {
        sign_in::token(&program, sign_in::PATIENCE)
    })
}

/// Why the site didn't answer, as far as a round cares.
enum Unread {
    /// What was asked for is gone, or not Joel's to read: the round goes
    /// on without it.
    Gone(String),
    /// The site couldn't be asked now, its rate limit included: the round
    /// is tried again whole.
    Failed(String),
}

/// GitHub, or a site speaking its API, asked as Joel.
struct GitHub<'a> {
    origin: &'a str,
    /// The site's name as Joel reads it, so the journal names what failed.
    name: &'a str,
    token: &'a str,
    agent: &'a Agent,
}

impl GitHub<'_> {
    /// One GET of a path, and its answer.
    fn get(&self, path: &str, query: &[(&str, &str)]) -> Result<Value, Unread> {
        let url = format!("{}{path}", self.origin);
        self.ask(&url, query).map(|(said, _)| said)
    }

    /// A whole list, page after page as the `Link` header names them, so a
    /// long one isn't cut at its first hundred. A next page anywhere but
    /// the site's own origin isn't asked, so Joel's sign-in goes nowhere
    /// else, and the round is tried again.
    fn all(&self, path: &str, query: &[(&str, &str)]) -> Result<Value, Unread> {
        let (mut list, mut next) = self.ask(&format!("{}{path}", self.origin), query)?;
        let own = format!("{}/", self.origin);
        let mut pages = 1;
        while let Some(url) = next {
            if !url.starts_with(&own) {
                let why = format!("{} named a next page elsewhere: {url}", self.name);
                return Err(Unread::Failed(why));
            }
            if pages == PAGES {
                let name = self.name;
                eprintln!("<4>{name}'s {path} runs past {PAGES} pages, and the rest isn't read");
                break;
            }
            let (more, after) = self.ask(&url, &[])?;
            add_page(&mut list, more);
            next = after;
            pages += 1;
        }
        Ok(list)
    }

    /// One GET of a URL on the site's origin: its answer, and the next page
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
            .map_err(|error| Unread::Failed(format!("couldn't reach {}: {error}", self.name)))?;
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
        let retry_after = header("retry-after").is_some();
        let name = self.name;
        let said: Value = answer
            .body_mut()
            .read_json()
            .map_err(|error| Unread::Failed(format!("{name} answered {status}: {error}")))?;
        let message = said["message"].as_str().unwrap_or("no reason");
        let limited = github::rate_limited(status, remaining.as_deref(), retry_after, message);
        let why = || format!("{name} answered {status} for {path}: {message}");
        match status {
            200 => Ok((said, next)),
            _ if limited => Err(Unread::Failed(format!("{}, its rate limit", why()))),
            403 | 404 | 410 => Err(Unread::Gone(why())),
            _ => Err(Unread::Failed(why())),
        }
    }
}

/// Who Joel is on the site: as `--login` says, else as the site answers.
fn who(github: &GitHub<'_>, how: &Run) -> Option<String> {
    if let Some(login) = &how.login {
        return Some(login.clone());
    }
    match github.get("/user", &[]) {
        Ok(user) => {
            let login = user["login"].as_str().map(str::to_owned);
            if login.is_none() {
                eprintln!("<4>{} didn't say who Joel is", github.name);
            }
            login
        }
        Err(Unread::Gone(why) | Unread::Failed(why)) => {
            eprintln!("<4>couldn't ask {} who Joel is: {why}", github.name);
            None
        }
    }
}

/// The pull requests and issues that changed since `since`: those Joel
/// takes part in, as his notifications say, read ones too since he may
/// read one on his phone before a round; or, where a site has none, the
/// pull requests he opened, as a search finds them.
fn changed(github: &GitHub<'_>, how: &Run, since: &str) -> Result<Vec<Thread>, Unread> {
    if let Some(address) = &how.authored_by {
        let mut threads = Vec::new();
        for repo in &how.repos {
            let query = format!("is:pr repo:{repo} author:{address} updated:>={since}");
            let asked = [
                ("q", query.as_str()),
                ("sort", "updated"),
                ("order", "desc"),
                ("per_page", PAGE),
            ];
            let answer = github.all("/search/issues", &asked)?;
            threads.extend(found(&answer, &how.site));
        }
        return Ok(threads);
    }
    let asked = [
        ("participating", "true"),
        ("all", "true"),
        ("since", since),
        ("per_page", PAGE),
    ];
    let answer = github.all("/notifications", &asked)?;
    Ok(notified(&answer, &how.site))
}

/// One round: what changed since the last, judged oldest first, and how
/// far it has read moved on only once every pull request and issue it
/// named was read.
fn round(github: &GitHub<'_>, how: &Run, state: &Path, judged: &mut Judged) {
    let began = seconds();
    let Some(login) = who(github, how) else {
        return;
    };
    let back = if how.authored_by.is_some() {
        INDEX_LAG
    } else {
        OVERLAP
    };
    let from = judged.since.saturating_sub(back);
    let since = github::stamp(from);
    let threads = match changed(github, how, &since) {
        Ok(threads) => threads,
        Err(Unread::Gone(why) | Unread::Failed(why)) => {
            eprintln!("<4>couldn't ask {} what changed: {why}", github.name);
            return;
        }
    };
    // Where anyone may comment: GitHub's own, however its site is named.
    let open = how.authored_by.is_none()
        || how
            .origin
            .trim_end_matches('/')
            .eq_ignore_ascii_case(GITHUB);
    // What this round judges, which alone is worth asking about a repository.
    let keeps = |said: &Said| {
        said.at >= from && !judged.keys.contains(&said.key) && worth_judging(&said.text, &login)
    };
    let (mut heard, mut learned) = (Vec::new(), HashMap::new());
    for thread in &threads {
        match said_on(github, thread, &since, (open, &login), &keeps, &mut learned) {
            Ok(found) => heard.extend(found),
            Err(Unread::Gone(why)) => eprintln!("<4>{} can't be read: {why}", thread.key()),
            Err(Unread::Failed(why)) => {
                eprintln!("<4>couldn't read {}: {why}", thread.key());
                return;
            }
        }
    }
    heard.retain(|(said, _, _, _)| keeps(said));
    heard.sort_by_key(|(said, _, _, _)| said.at);
    // Where logins are addresses, what Joel says is by his address.
    let joel = Joel {
        slack_ids: Vec::new(),
        logins: vec![login],
        emails: how.authored_by.iter().cloned().collect(),
    };
    for (said, thread, context, outsider) in heard {
        let place = (&thread, context, outsider);
        judge(state, &joel, &said, place, how.shadow);
        judged.keys.push(said.key);
    }
    judged.since = began;
    let over = judged.keys.len().saturating_sub(KEPT);
    judged.keys.drain(..over);
    progress::write(state, &how.site, judged, millis());
}

/// What was said on a pull request or issue since `since`, each with it,
/// the place it's about and whether an outsider said it: where anyone may
/// comment (`open`), a writer not the repository's own, or anyone on a
/// repository Joel doesn't work in. Reviews can't be asked for by time,
/// so all are read and the round keeps those since. Whether he works in
/// it is asked only when something there is one the round `keeps`, which
/// most threads hold nothing of, and once a round however many threads it
/// has, so his rate limit isn't spent on the rest.
fn said_on(
    github: &GitHub<'_>,
    thread: &Thread,
    since: &str,
    who: (bool, &str),
    keeps: &dyn Fn(&Said) -> bool,
    learned: &mut HashMap<String, bool>,
) -> Result<Sayings, Unread> {
    let (open, login) = who;
    let base = format!("/repos/{}", thread.repo);
    let number = thread.number;
    let opened = github.get(&format!("{base}/issues/{number}"), &[])?;
    // Opening it, as a pull request from his fork, he chose to work there.
    let joel_opened = opened["user"]["login"]
        .as_str()
        .is_some_and(|by| by.eq_ignore_ascii_case(login));
    let mut found = github::said(&opened, thread, Kind::Opening);
    // Its writer's standing alone, before anything said after it joins it.
    let opened_by_own = found.first().is_some_and(|it| it.own);
    let recent = [("since", since), ("per_page", PAGE)];
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
    let works_in =
        !open || joel_opened || (found.iter().any(keeps) && joel_works_in(github, &base, learned)?);
    let trusted = !open || joel_opened || (works_in && opened_by_own);
    let opening = if trusted {
        format!(
            "{}\n\n{}",
            opened["title"].as_str().unwrap_or_default(),
            opened["body"].as_str().unwrap_or_default()
        )
    } else {
        String::new()
    };
    let context = thread.context(&opening);
    Ok(found
        .into_iter()
        .map(|said| {
            let outsider = open && !(works_in && said.own);
            (said, thread.clone(), context.clone(), outsider)
        })
        .collect())
}

/// Whether Joel works in the repository at `base`, asked of GitHub only
/// the first time a round wants to know, and asked again the next round,
/// so a change to what he may do there is heard within a minute.
fn joel_works_in(
    github: &GitHub<'_>,
    base: &str,
    learned: &mut HashMap<String, bool>,
) -> Result<bool, Unread> {
    if let Some(&known) = learned.get(base) {
        return Ok(known);
    }
    let known = match github.get(base, &[]) {
        Ok(repo) => github::works_in(&repo),
        // Unreadable, it's one he doesn't work in; his own words still count.
        Err(Unread::Gone(why)) => {
            eprintln!(
                "<4>{why}, so a stop there by anyone but Joel counts this round \
                 only on what he opened"
            );
            false
        }
        Err(failed) => return Err(failed),
    };
    learned.insert(base.to_owned(), known);
    Ok(known)
}

/// What a round hears: each saying, with where and whether by an outsider.
type Sayings = Vec<(Said, Thread, Context, bool)>;

/// One thing said, judged on the pull request or issue it was said on,
/// and written down.
fn judge(state: &Path, joel: &Joel, said: &Said, place: (&Thread, Context, bool), shadow: bool) {
    let (thread, context, outsider) = place;
    let decision = decide(
        &Heard {
            platform: Platform::Markdown,
            author: said.author.clone(),
            by_bot: said.by_bot,
            outsider,
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
