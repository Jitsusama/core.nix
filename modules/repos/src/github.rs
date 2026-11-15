//! GitHub's part of the record: each repository's changes still open
//! there, with their checks and review. A refresh asks GitHub once, in one
//! GraphQL query per few dozen repositories, and keeps what it saw as
//! `repos.github/1` (`Seen`), which every reading merges into the record,
//! so a reading stays as quick as the disk and says how old GitHub's part
//! is rather than waiting on it.
//!
//! GitHub is asked only through the machine's GitHub App, whose token can
//! read and do nothing else. A repository it isn't installed on answers as
//! one that isn't there, and is said to be unread rather than asked again
//! another way.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::model::{Change, Checks, ChecksState, GitHubChange, Record, Review, Source};
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
pub fn parse(answer: &Value, asked: &[Asked]) -> (BTreeMap<String, Vec<Change>>, Vec<String>) {
    let mut changes = BTreeMap::new();
    let mut unread = Vec::new();
    let mut said = BTreeSet::new();
    for error in answer["errors"].as_array().into_iter().flatten() {
        let why = error["message"].as_str().unwrap_or("no reason given");
        let one = error["path"][0]
            .as_str()
            .and_then(|alias| alias.strip_prefix('r'))
            .and_then(|at| at.parse::<usize>().ok())
            .and_then(|at| asked.get(at));
        match one {
            Some(one) if said.insert(one.key.clone()) => {
                unread.push(format!("{}: {}", one.key, unseen(error, why)));
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

/// Why GitHub couldn't see a repository, said for a person: one the App
/// isn't installed on answers as one that isn't there.
fn unseen(error: &Value, why: &str) -> String {
    if error["type"].as_str() == Some("NOT_FOUND") {
        "GitHub's App can't see it, or it's gone".to_owned()
    } else {
        why.to_owned()
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
    })
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
    let mut seen = Seen {
        schema: SCHEMA.to_owned(),
        read_at: now,
        changes: BTreeMap::new(),
        unread: Vec::new(),
    };
    for some in asked.chunks(PER_QUERY) {
        let answer = ask(&query(some))?;
        if !answer["data"].is_object() {
            let why = answer["errors"][0]["message"].as_str();
            return Err(format!(
                "GitHub answered no data: {}",
                why.unwrap_or("no reason given")
            ));
        }
        let (changes, unread) = parse(&answer, some);
        seen.changes.extend(changes);
        seen.unread.extend(unread);
    }
    Ok(seen)
}

/// GitHub's answer to `query`, asked with `token`.
pub fn post(token: &str, query: &str) -> Result<Value, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(std::time::Duration::from_secs(60)))
        .user_agent("repos")
        .build()
        .into();
    let body = serde_json::json!({ "query": query }).to_string();
    let mut answer = agent
        .post("https://api.github.com/graphql")
        .header("Authorization", format!("bearer {token}"))
        .content_type("application/json")
        .send(body)
        .map_err(|why| format!("can't reach GitHub: {why}"))?;
    let status = answer.status();
    let text = answer
        .body_mut()
        .read_to_string()
        .map_err(|why| format!("GitHub's answer didn't arrive whole: {why}"))?;
    let said: Value = serde_json::from_str(&text).unwrap_or_default();
    if status.as_u16() != 200 {
        let why = said["message"].as_str().unwrap_or("no reason given");
        return Err(format!("GitHub refused the query ({status}): {why}"));
    }
    Ok(said)
}

/// The record with what a refresh saw of GitHub: each repository's open
/// changes, and GitHub as a source, saying when it was asked and what it
/// couldn't see. A repository the refresh didn't ask after, as one cloned
/// since, has no changes until the next.
pub fn merge(record: &mut Record, seen: &Seen) {
    for repository in &mut record.repositories {
        if let Some(changes) = seen.changes.get(&repository.key) {
            repository.changes.clone_from(changes);
        }
    }
    record.sources.push(Source {
        name: "github".to_owned(),
        what: "each repository's open changes and their checks, through GitHub's App".to_owned(),
        read_at: Some(seen.read_at),
        unread: seen.unread.clone(),
    });
}

#[cfg(test)]
mod tests;
