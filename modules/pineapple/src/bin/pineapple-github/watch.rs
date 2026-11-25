//! Joel's own words, found as soon as GitHub lists them rather than once
//! its search has indexed them, which can take eight minutes and more.
//! Each round lists the newest comments in each repository his account
//! was last active in, as his events feed names them, and reads each pull
//! request or issue where a comment of his worth judging isn't judged
//! yet. His events feed itself lags by minutes, so it only names where to
//! look. Each list is asked with the tag GitHub last gave it, so one
//! unchanged is answered 304, which his rate limit doesn't count. Where a
//! list can't be read, his words there wait on the search (own.rs).

use std::collections::HashMap;

use pineapple::github::{self, Kind, Thread, worth_judging};
use serde_json::Value;

use super::GitHub;

/// The most repositories watched: those his account was last active in.
/// Each is one request a round, answered 304 while nothing changed.
const WATCHED: usize = 6;

/// How a repository's newest comments are listed, thirty of them: far
/// more than are written there in a round.
const NEWEST: &str = "sort=created&direction=desc&per_page=30";

/// Where Joel's account was last active, and the answers GitHub gave each
/// list asked, by what was asked, with the tag it gave each.
#[derive(Default)]
pub(super) struct Watched {
    repos: Vec<String>,
    kept: HashMap<String, (String, Value)>,
}

impl Watched {
    /// The pull requests and issues where a comment of Joel's, said since
    /// `reach`, is worth judging and isn't among `judged`, as each watched
    /// repository lists its newest comments now.
    pub(super) fn listed(
        &mut self,
        github: &GitHub<'_>,
        (site, login): (&str, &str),
        reach: u64,
        judged: &[String],
    ) -> Vec<Thread> {
        let feed = format!("/users/{login}/events?per_page={}", super::PAGE);
        if let Some(events) = self.ask(github, &feed) {
            self.repos = active_in(&events);
        }
        let mut threads = Vec::new();
        for repo in self.repos.clone() {
            let newest = format!("/repos/{repo}/issues/comments?{NEWEST}");
            let Some(comments) = self.ask(github, &newest) else {
                continue;
            };
            for comment in comments.as_array().into_iter().flatten() {
                let Some((thread, key, at)) = his(comment, &repo, (site, login)) else {
                    continue;
                };
                let fresh = at >= reach && !judged.contains(&key);
                if fresh && !threads.contains(&thread) {
                    threads.push(thread);
                }
            }
        }
        threads
    }

    /// One list, asked with the tag GitHub last gave it: the list it
    /// keeps when GitHub answers that nothing changed. One that can't be
    /// read is said in the journal, and nothing is listed from it.
    fn ask(&mut self, github: &GitHub<'_>, path: &str) -> Option<Value> {
        let url = format!("{}{path}", github.origin);
        let mut request = github
            .agent
            .get(&url)
            .header("Authorization", &format!("Bearer {}", github.token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "pineapple-github");
        if let Some((tag, _)) = self.kept.get(path) {
            request = request.header("If-None-Match", tag.as_str());
        }
        let name = github.name;
        let waits = "so Joel's newest words there wait on its search";
        let mut answer = match request.call() {
            Ok(answer) => answer,
            Err(error) => {
                eprintln!("<4>couldn't list {path} on {name}, {waits}: {error}");
                return None;
            }
        };
        let status = answer.status().as_u16();
        if status == 304 {
            return self.kept.get(path).map(|(_, kept)| kept.clone());
        }
        if status != 200 {
            eprintln!("<4>{name} answered {status} for {path}, {waits}");
            return None;
        }
        let tag = answer
            .headers()
            .get("etag")
            .and_then(|tag| tag.to_str().ok())
            .map(str::to_owned);
        let listed: Value = match answer.body_mut().read_json() {
            Ok(listed) => listed,
            Err(error) => {
                eprintln!("<4>{name}'s {path} couldn't be read, {waits}: {error}");
                return None;
            }
        };
        if let Some(tag) = tag {
            self.kept.insert(path.to_owned(), (tag, listed.clone()));
        }
        Some(listed)
    }
}

/// The repositories an events feed names, most lately active first, at
/// most [`WATCHED`] of them.
fn active_in(events: &Value) -> Vec<String> {
    let mut named: Vec<(&str, &str)> = events
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|event| {
            Some((
                event["created_at"].as_str()?,
                event["repo"]["name"].as_str()?,
            ))
        })
        .collect();
    // Times of one form sort as the times they name.
    named.sort_by(|one, other| other.0.cmp(one.0));
    let mut repos: Vec<String> = Vec::new();
    for (_, repo) in named {
        if repos.len() == WATCHED {
            break;
        }
        if !repos.iter().any(|known| known == repo) {
            repos.push(repo.to_owned());
        }
    }
    repos
}

/// A comment of Joel's worth judging, as a repository lists it: the pull
/// request or issue it's on, the key its reading would give it, and when
/// it was said.
fn his(comment: &Value, repo: &str, (site, login): (&str, &str)) -> Option<(Thread, String, u64)> {
    let author = comment["user"]["login"].as_str()?;
    let text = comment["body"].as_str()?;
    if !author.eq_ignore_ascii_case(login) || !worth_judging(text, login) {
        return None;
    }
    let number = comment["issue_url"]
        .as_str()?
        .rsplit('/')
        .next()?
        .parse()
        .ok()?;
    let thread = Thread {
        site: site.to_owned(),
        repo: repo.to_owned(),
        number,
        pull: comment["html_url"].as_str()?.contains("/pull/"),
    };
    let id = comment["id"].as_u64()?;
    let key = format!("{}:{}:{id}", thread.key(), Kind::Comment.name());
    let at = github::seconds(comment["created_at"].as_str()?)?;
    Some((thread, key, at))
}
