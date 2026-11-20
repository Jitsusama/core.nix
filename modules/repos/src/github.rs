//! GitHub's part of the record: each repository's changes still open
//! there, with their checks and review. A refresh asks GitHub once, in one
//! GraphQL query per few dozen repositories, and keeps what it saw as
//! `repos.github/1` (`Seen`), which every reading merges into the record,
//! so a reading stays as quick as the disk and says how old GitHub's part
//! is rather than waiting on it.
//!
//! GitHub is asked through the machine's GitHub App, whose token can read
//! and do nothing else. A repository it isn't installed on answers as one
//! that isn't there, and is said to be not readable here rather than asked
//! again another way, unless the machine lists it to be read through gh's
//! sign-in (gh.rs). That token reads all its person can, so whichever way
//! is asked, only `GRAPHQL` is, with a query and nothing that writes,
//! following no redirect, and the token leaves only in a header.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::kept;
use crate::model::{Change, Checks, ChecksState, GitHubChange, Record, Review, Source, Through};
use crate::when;

/// The schema of what a refresh saw, which a reading checks before
/// trusting its shape.
pub const SCHEMA: &str = "repos.github/1";

/// How many repositories one query asks after, so each stays well inside
/// what GitHub lets one query cost and answers in a few seconds.
pub const PER_QUERY: usize = 25;

/// How many open changes are read for each repository: the ones updated
/// last, since those are the ones in play. Past it, a repository is busier
/// than a list of its changes helps with, and GitHub's own page is for it.
const CHANGES: usize = 50;

/// How many checks are read for each change.
const CHECKS: usize = 100;

/// How many of a change's asked reviews are read, more than a change
/// asks of people in practice.
const REVIEWERS: usize = 10;

/// What one refresh saw of GitHub.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seen {
    pub schema: String,
    /// When GitHub was asked.
    pub read_at: u64,
    /// Each repository's open changes, by the key the disk gives it, the
    /// one updated last first.
    pub changes: BTreeMap<String, Vec<Change>>,
    /// Why a repository, or GitHub itself, couldn't be read, for a person,
    /// each opening with the repository's key where it's one.
    #[serde(default)]
    pub unread: Vec<String>,
    #[serde(default)]
    pub through: BTreeMap<String, Through>,
}

/// A repository to ask GitHub after: its key on the disk, and the owner
/// and name GitHub knows it by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
    pub key: String,
    pub owner: String,
    pub name: String,
}

/// Each repository in the record that GitHub keeps, once.
pub fn asked(record: &Record) -> Vec<Asked> {
    let mut seen = BTreeSet::new();
    record
        .repositories
        .iter()
        .filter_map(|repository| {
            let path = repository.key.strip_prefix("github:")?;
            let (owner, name) = path.split_once('/')?;
            if name.contains('/') || !seen.insert(repository.key.clone()) {
                return None;
            }
            Some(Asked {
                key: repository.key.clone(),
                owner: owner.to_owned(),
                name: name.to_owned(),
            })
        })
        .collect()
}

/// The query asking after each of `asked`, each under an alias of its own,
/// `r0` on, so the answer says which is which however GitHub names them.
pub fn query(asked: &[Asked]) -> String {
    let aliases: Vec<String> = asked
        .iter()
        .enumerate()
        .map(|(at, one)| {
            let (owner, name) = (quoted(&one.owner), quoted(&one.name));
            format!("  r{at}: repository(owner: {owner}, name: {name}) {{ ...open }}\n")
        })
        .collect();
    let aliases = aliases.concat();
    format!(
        r"query {{
{aliases}}}
fragment open on Repository {{
  pullRequests(states: OPEN, first: {CHANGES},
      orderBy: {{field: UPDATED_AT, direction: DESC}}) {{
    nodes {{
      number title url isDraft headRefName baseRefName headRefOid updatedAt
      author {{ login }}
      reviewDecision
      reviewRequests(first: {REVIEWERS}) {{ nodes {{ requestedReviewer {{
        ... on User {{ login }} }} }} }}
      commits(last: 1) {{ nodes {{ commit {{ statusCheckRollup {{
        contexts(first: {CHECKS}) {{ nodes {{ __typename
          ... on CheckRun {{ name status conclusion }}
          ... on StatusContext {{ context state }} }} }} }} }} }} }}
    }}
  }}
}}
"
    )
}

/// A string as GraphQL writes one, which JSON's quoting is.
fn quoted(text: &str) -> String {
    Value::String(text.to_owned()).to_string()
}

/// What GitHub's answer to `query(asked)` says: each repository's open
/// changes by its key on the disk, and why any couldn't be read.
pub fn parse(
    answer: &Value,
    asked: &[Asked],
    through: Through,
) -> (BTreeMap<String, Vec<Change>>, Vec<String>) {
    let mut changes = BTreeMap::new();
    let mut unread = Vec::new();
    let mut said = BTreeSet::new();
    for error in answer["errors"].as_array().into_iter().flatten() {
        if within_review_requests(error) {
            // A reviewer GitHub won't name, as a team may be to the App's
            // token, leaves its change and repository read; it's only a
            // reviewer with no login, which counts for nobody.
            continue;
        }
        let why = error["message"].as_str().unwrap_or("no reason given");
        let one = error["path"][0]
            .as_str()
            .and_then(|alias| alias.strip_prefix('r'))
            .and_then(|at| at.parse::<usize>().ok())
            .and_then(|at| asked.get(at));
        match one {
            Some(one) if said.insert(one.key.clone()) => {
                unread.push(format!("{}: {}", one.key, unseen(error, why, through)));
            }
            Some(_) => {}
            None => unread.push(format!("GitHub: {why}")),
        }
    }
    for (at, one) in asked.iter().enumerate() {
        let repository = &answer["data"][format!("r{at}")];
        if repository.is_null() {
            if said.insert(one.key.clone()) && answer["data"].is_object() {
                unread.push(format!("{}: GitHub didn't answer for it", one.key));
            }
            continue;
        }
        let open = &repository["pullRequests"];
        let mut read: Vec<Change> = open["nodes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(change)
            .collect();
        read.sort_by_key(|change| std::cmp::Reverse(change.updated_at));
        changes.insert(one.key.clone(), read);
    }
    (changes, unread)
}

/// Whether `error` is about one change's review requests alone, under a
/// repository GitHub otherwise answered.
fn within_review_requests(error: &Value) -> bool {
    error["path"].as_array().is_some_and(|path| {
        path.iter()
            .any(|step| step.as_str() == Some("reviewRequests"))
    })
}

/// Why GitHub couldn't see a repository, said for a person: one the App
/// isn't installed on, or a sign-in can't read, answers as one that isn't
/// there.
fn unseen(error: &Value, why: &str, through: Through) -> String {
    let asker = match through {
        Through::App => "GitHub's App",
        Through::Gh => "gh's sign-in",
    };
    if error["type"].as_str() == Some("NOT_FOUND") {
        format!("{UNREADABLE}: {asker} can't see it, or it's gone")
    } else {
        format!("{UNREADABLE}: {why}")
    }
}

/// One open change as GitHub says it, or none where it says too little.
fn change(node: &Value) -> Option<Change> {
    let rollup = &node["commits"]["nodes"][0]["commit"]["statusCheckRollup"];
    Some(Change {
        title: node["title"].as_str()?.to_owned(),
        author: node["author"]["login"].as_str().map(str::to_owned),
        draft: node["isDraft"].as_bool().unwrap_or(false),
        branch: node["headRefName"].as_str()?.to_owned(),
        base: node["baseRefName"].as_str()?.to_owned(),
        commit: node["headRefOid"].as_str().map(str::to_owned),
        updated_at: node["updatedAt"].as_str().and_then(when::seconds_of)?,
        checks: checks(rollup),
        review: review(&node["reviewDecision"]),
        github: Some(GitHubChange {
            number: node["number"].as_u64()?,
            url: node["url"].as_str()?.to_owned(),
        }),
        review_asked_of: asked_of(&node["reviewRequests"]),
        asks_your_review: false,
    })
}

/// Whose review a change asks, by login: people alone, since a team
/// asked answers as a reviewer with no login.
fn asked_of(requests: &Value) -> Vec<String> {
    requests["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|request| request["requestedReviewer"]["login"].as_str())
        .map(str::to_owned)
        .collect()
}

/// What a change's checks say, each named once: a check run by its name
/// and a commit status by its context, as GitHub shows them.
fn checks(rollup: &Value) -> Checks {
    let mut passed = 0;
    let mut failed = Vec::new();
    let mut pending = Vec::new();
    for one in rollup["contexts"]["nodes"].as_array().into_iter().flatten() {
        let (name, said) = match one["__typename"].as_str() {
            Some("CheckRun") => (
                one["name"].as_str(),
                if one["status"].as_str() == Some("COMPLETED") {
                    one["conclusion"].as_str()
                } else {
                    None
                },
            ),
            Some("StatusContext") => (one["context"].as_str(), one["state"].as_str()),
            _ => continue,
        };
        let name = name.unwrap_or("unnamed").to_owned();
        match said.map(outcome) {
            Some(Outcome::Passed) => passed += 1,
            Some(Outcome::Failed) => once(&mut failed, name),
            Some(Outcome::Pending) | None => once(&mut pending, name),
        }
    }
    let state = if !failed.is_empty() {
        ChecksState::Failing
    } else if !pending.is_empty() {
        ChecksState::Pending
    } else if passed > 0 {
        ChecksState::Passing
    } else {
        ChecksState::None
    };
    Checks {
        state,
        passed,
        failed,
        pending,
    }
}

/// `name` added to `names` unless it's there, since one workflow's job
/// may run several times under one name.
fn once(names: &mut Vec<String>, name: String) {
    if !names.contains(&name) {
        names.push(name);
    }
}

enum Outcome {
    Passed,
    Failed,
    Pending,
}

/// What a check's conclusion, or a status's state, comes to. A check
/// that was skipped or said nothing either way stands in no change's way.
fn outcome(said: &str) -> Outcome {
    match said {
        "SUCCESS" | "NEUTRAL" | "SKIPPED" => Outcome::Passed,
        "FAILURE" | "ERROR" | "CANCELLED" | "TIMED_OUT" | "ACTION_REQUIRED" | "STARTUP_FAILURE"
        | "STALE" => Outcome::Failed,
        _ => Outcome::Pending,
    }
}

/// Where GitHub says a change's review stands.
fn review(decision: &Value) -> Review {
    match decision.as_str() {
        Some("APPROVED") => Review::Approved,
        Some("CHANGES_REQUESTED") => Review::ChangesAsked,
        Some("REVIEW_REQUIRED") => Review::Waiting,
        _ => Review::None,
    }
}

/// What GitHub says of each of `asked`, `PER_QUERY` at a time, each query
/// put through `ask`, which answers GitHub's answer. An answer with no
/// data at all, as for a token GitHub refuses, ends the refresh, so what
/// the last one saw is kept rather than written over with nothing.
pub fn refresh(
    asked: &[Asked],
    now: u64,
    mut ask: impl FnMut(&str) -> Result<Value, String>,
) -> Result<Seen, String> {
    let mut seen = Seen::new(now);
    for some in asked.chunks(PER_QUERY) {
        let answer = data(ask(&query(some))?)?;
        seen.read(&answer, some, Through::App);
    }
    Ok(seen)
}

/// How many repositories one query through gh's sign-in asks after: one,
/// since those it's listed for are few and some are busy enough that
/// GitHub gives up on several of them in one query.
const GH_PER_QUERY: usize = 1;

/// What GitHub says of each of `asked` through gh's sign-in, which only a
/// machine's own list sends this way. A sign-in gh lacks, or one GitHub
/// refuses, is said for each of them as not readable here, and nothing
/// more is asked: not again, since a refusal stays one until a person
/// signs in, and not another way, since the App already can't see them.
/// Any other failure is said for the one it was asked after alone.
pub fn refresh_through_gh(
    asked: &[Asked],
    now: u64,
    token: impl FnOnce() -> Result<Token, String>,
    mut ask: impl FnMut(&Token, &str) -> Result<Value, Failed>,
) -> Seen {
    let mut seen = Seen::new(now);
    if asked.is_empty() {
        return seen;
    }
    let token = match token() {
        Ok(token) => token,
        Err(why) => {
            seen.unreadable(asked, &why);
            return seen;
        }
    };
    for (at, some) in asked.chunks(GH_PER_QUERY).enumerate() {
        match ask(&token, &query(some)).and_then(|answer| Ok(data(answer)?)) {
            Ok(answer) => seen.read(&answer, some, Through::Gh),
            Err(failed) if failed.refused => {
                seen.unreadable(&asked[at * GH_PER_QUERY..], &failed.why);
                break;
            }
            Err(failed) => seen.unreadable(some, &failed.why),
        }
    }
    seen
}

/// Why asking GitHub gave nothing: refused where the asking itself was
/// turned away, as a token GitHub won't take, which asking again can't
/// mend until a person does something.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failed {
    pub why: String,
    pub refused: bool,
}

impl Failed {
    pub fn refused(why: String) -> Self {
        Self { why, refused: true }
    }
}

impl From<String> for Failed {
    fn from(why: String) -> Self {
        Self {
            why,
            refused: false,
        }
    }
}

/// An answer holding data, or why GitHub gave none.
fn data(answer: Value) -> Result<Value, String> {
    if answer["data"].is_object() {
        return Ok(answer);
    }
    let why = answer["errors"][0]["message"].as_str();
    Err(format!(
        "GitHub answered no data: {}",
        why.unwrap_or("no reason given")
    ))
}

impl Seen {
    /// Says of each change whether it asks the review of `login`, as
    /// GitHub matches logins, ASCII case aside, so what's kept says so
    /// without holding a login (model.rs).
    pub fn mark_asked(&mut self, login: &str) {
        for change in self.changes.values_mut().flatten() {
            change.asks_your_review = change
                .review_asked_of
                .iter()
                .any(|one| one.eq_ignore_ascii_case(login));
        }
    }

    /// Nothing seen yet, by a refresh asking at `now`.
    pub fn new(now: u64) -> Self {
        Self {
            schema: SCHEMA.to_owned(),
            read_at: now,
            changes: BTreeMap::new(),
            unread: Vec::new(),
            through: BTreeMap::new(),
        }
    }

    /// What another way of asking saw, beside this one's.
    pub fn add(&mut self, other: Seen) {
        self.changes.extend(other.changes);
        self.unread.extend(other.unread);
        self.through.extend(other.through);
    }

    fn read(&mut self, answer: &Value, asked: &[Asked], through: Through) {
        let (changes, unread) = parse(answer, asked, through);
        self.through
            .extend(changes.keys().map(|key| (key.clone(), through)));
        self.changes.extend(changes);
        self.unread.extend(unread);
    }

    fn unreadable(&mut self, asked: &[Asked], why: &str) {
        self.unread.extend(
            asked
                .iter()
                .map(|one| format!("{}: {UNREADABLE}: {why}", one.key)),
        );
    }
}

/// How a repository no way could read is said, wherever it's shown.
pub const UNREADABLE: &str = "not readable here";

/// A token GitHub is asked with, from the broker or gh. It says nothing
/// of itself where it's printed, and leaves only as a request's header.
pub struct Token(String);

impl Token {
    pub(crate) fn new(secret: String) -> Self {
        Self(secret)
    }

    /// The header a request carries it in, built only where it's sent.
    pub fn header(&self) -> String {
        format!("bearer {}", self.0)
    }
}

impl std::fmt::Debug for Token {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str("Token(not shown)")
    }
}

/// Where GitHub is asked, and the only place a token is sent.
pub const GRAPHQL: &str = "https://api.github.com/graphql";

/// Whether `document` only reads: one query, holding no mutation or
/// subscription anywhere outside its strings and comments, so a name a
/// repository goes by can't make it write.
pub fn reads_only(document: &str) -> Result<(), String> {
    let mut words: Vec<String> = vec![String::new()];
    let mut chars = document.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                let mut escaped = false;
                for c in chars.by_ref() {
                    if c == '"' && !escaped {
                        break;
                    }
                    escaped = c == '\\' && !escaped;
                }
                words.push(String::new());
            }
            '#' => {
                chars.by_ref().find(|&c| c == '\n');
                words.push(String::new());
            }
            c if c.is_alphanumeric() || c == '_' => words.last_mut().into_iter().for_each(|w| {
                w.push(c);
            }),
            _ => words.push(String::new()),
        }
    }
    let words: Vec<&str> = words
        .iter()
        .map(String::as_str)
        .filter(|word| !word.is_empty())
        .collect();
    let writes = |word: &&str| matches!(*word, "mutation" | "subscription");
    if words.first() == Some(&"query") && !words.iter().any(writes) {
        Ok(())
    } else {
        Err("repos sends GitHub one query, nothing that writes or listens".to_owned())
    }
}

/// GitHub's answer to `query`, asked with `token`, once `endpoint` is
/// GitHub's and `query` only reads; anything else is refused before the
/// token's header is built. Only the command asks, so a crate reading
/// repositories through the library leaves the HTTP side out with the
/// `github` feature.
#[cfg(feature = "github")]
pub fn post(endpoint: &str, token: &Token, query: &str) -> Result<Value, Failed> {
    if endpoint != GRAPHQL {
        return Err(Failed::refused(format!(
            "repos asks only {GRAPHQL}, not {endpoint}"
        )));
    }
    reads_only(query).map_err(Failed::refused)?;
    send(endpoint, token, query)
}

/// The keeping of what a refresh saw at `file`, written whole through a
/// file beside it and renamed over it, so a reading never finds half of
/// one, and readable by its owner alone, since it names private changes.
/// Keeps what a refresh saw, as `keep` does, each change first marked by
/// whether it asks the review of `login`, the one the machine declares, so
/// the collection can lead with the reviews asked of him once the refresh
/// has gone. With no login, nothing is marked.
pub fn keep_marked(file: &Path, seen: &mut Seen, login: Option<&str>) -> io::Result<()> {
    if let Some(login) = login {
        seen.mark_asked(login);
    }
    keep(file, seen)
}

pub fn keep(file: &Path, seen: &Seen) -> io::Result<()> {
    kept::write(file, &serde_json::to_vec(seen).map_err(io::Error::other)?)
}

/// GitHub's answer to `query` at `endpoint`, asked with `token`. A
/// redirect is answered, never followed, so the token goes nowhere else.
#[cfg(feature = "github")]
fn send(endpoint: &str, token: &Token, query: &str) -> Result<Value, Failed> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .max_redirects(0)
        .timeout_global(Some(std::time::Duration::from_secs(60)))
        .user_agent("repos")
        .build()
        .into();
    let body = serde_json::json!({ "query": query }).to_string();
    let mut answer = agent
        .post(endpoint)
        .header("Authorization", token.header())
        .content_type("application/json")
        .send(body)
        .map_err(|why| format!("can't reach GitHub: {why}"))?;
    let status = answer.status();
    if status.is_redirection() {
        return Err(Failed::refused(format!(
            "GitHub answered with a redirect ({}), which repos doesn't follow",
            status.as_u16()
        )));
    }
    let text = answer
        .body_mut()
        .read_to_string()
        .map_err(|why| format!("GitHub's answer didn't arrive whole: {why}"))?;
    let said: Value = serde_json::from_str(&text).unwrap_or_default();
    let why = said["message"]
        .as_str()
        .map_or_else(String::new, |why| format!(": {why}"));
    match status.as_u16() {
        200 => Ok(said),
        401 | 403 => Err(Failed::refused(format!(
            "GitHub refused the sign-in ({status}){why}"
        ))),
        _ if status.is_server_error() => {
            Err(format!("GitHub gave up on the query ({status}){why}").into())
        }
        _ => Err(format!("GitHub refused the query ({status}){why}").into()),
    }
}

/// The record with what a refresh saw of GitHub: each repository's open
/// changes, and GitHub as a source, saying when it was asked and what it
/// couldn't see. A repository the refresh didn't ask after, as one cloned
/// since, has no changes until the next.
pub fn merge(record: &mut Record, seen: &Seen) {
    for repository in &mut record.repositories {
        if let Some(changes) = seen.changes.get(&repository.key) {
            repository.changes.clone_from(changes);
            // A refresh from before gh's sign-in was a way read through
            // the App alone.
            repository.read_through = Some(
                seen.through
                    .get(&repository.key)
                    .copied()
                    .unwrap_or(Through::App),
            );
        }
    }
    let by_gh = seen
        .through
        .values()
        .filter(|&&way| way == Through::Gh)
        .count();
    let what = "each repository's open changes and their checks, through GitHub's App";
    record.sources.push(Source {
        name: "github".to_owned(),
        what: match by_gh {
            0 => what.to_owned(),
            1 => format!("{what}, and gh's sign-in for one"),
            many => format!("{what}, and gh's sign-in for {many}"),
        },
        read_at: Some(seen.read_at),
        unread: seen.unread.clone(),
    });
}

/// Where a refresh keeps what it saw: `$XDG_STATE_HOME/repos/github.json`,
/// else under `home`'s ~/.local/state; none when neither is set.
pub fn seen_file(home: Option<&Path>) -> Option<PathBuf> {
    kept::file(home, "github.json")
}

/// The record with what a refresh kept at `file` merged in (`merge`), as
/// every reader of the record takes it. No file means no refresh has run
/// here, as on a machine with no GitHub broker, and the record is the
/// disk's alone; a file no refresh wrote, or one that can't be read, is
/// said unread, with GitHub never having answered.
pub fn merge_kept(record: &mut Record, file: &Path) {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return,
        Err(why) => {
            return unread(
                record,
                format!("{} couldn't be read: {why}", file.display()),
            );
        }
    };
    match serde_json::from_str::<Seen>(&text) {
        Ok(seen) if seen.schema == SCHEMA => merge(record, &seen),
        _ => unread(
            record,
            format!("{} isn't a refresh's record", file.display()),
        ),
    }
}

/// GitHub as a source no refresh's answer reached, for `why`.
fn unread(record: &mut Record, why: String) {
    record.sources.push(Source {
        name: "github".to_owned(),
        what: "each repository's open changes and their checks".to_owned(),
        read_at: None,
        unread: vec![why],
    });
}

#[cfg(test)]
mod gh_tests;
#[cfg(test)]
mod tests;
