//! Joel's own words, found as soon as GitHub lists them rather than once
//! its search has indexed them, which can take eight minutes and more.
//! Each round lists the newest comments in each repository his account
//! was last active in, as his events feed names them, and reads each pull
//! request or issue where a comment of his worth judging isn't judged
//! yet. His events feed itself lags by minutes, so it only names where to
//! look. Each list is asked with the tag GitHub last gave it, so one
//! unchanged is answered 304, which his rate limit doesn't count; while
//! his agents are busy most change, so a round costs up to seven of the
//! 5,000 requests an hour his sign-in is allowed, 840 an hour at most.
//! Where GitHub can't be asked, the round lists no more, so a slow GitHub
//! costs one wait, and his words wait on the search (own.rs). A
//! repository gone, or not his sign-in's to read, is passed by, so one
//! refused every round can't keep the others unlisted.

use std::collections::HashMap;

use pineapple::github::{self, Kind, Thread, worth_judging};
use serde_json::Value;

use super::{GitHub, Unread};

/// What a list that can't be read costs Joel, as the journal says it.
const WAITS: &str = "so Joel's newest words there wait on its search";

/// The most repositories watched: those his account was last active in.
/// Each is one request a round, answered 304 while nothing changed.
const WATCHED: usize = 6;

/// How a repository's newest comments are listed, a hundred of them, the
/// most one page holds: far more than are written there in a round, so
/// burying his stop takes a hundred comments in half a minute, and then
/// it waits on the search.
const NEWEST: &str = "sort=created&direction=desc&per_page=100";

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
        let newest = |repo: &str| format!("/repos/{repo}/issues/comments?{NEWEST}");
        let events = match self.ask(github, &feed) {
            Ok(events) => events,
            Err(Unread::Gone(why) | Unread::Failed(why) | Unread::Limited(why)) => {
                eprintln!("<4>{why}, {WAITS}");
                return Vec::new();
            }
        };
        self.repos = active_in(&events);
        // Only what's still watched is kept, so the lists kept stay few.
        let watched: Vec<String> = self.repos.iter().map(String::as_str).map(&newest).collect();
        self.kept
            .retain(|path, _| *path == feed || watched.contains(path));
        let mut threads = Vec::new();
        for repo in self.repos.clone() {
            let comments = match self.ask(github, &newest(&repo)) {
                Ok(comments) => comments,
                Err(Unread::Gone(why)) => {
                    eprintln!("<4>{why}; it's passed by, {WAITS}");
                    continue;
                }
                Err(Unread::Failed(why) | Unread::Limited(why)) => {
                    eprintln!("<4>{why}, {WAITS}");
                    break;
                }
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
    /// keeps when GitHub answers that nothing changed. One gone or not
    /// Joel's to read is told from GitHub failing, and both from his rate
    /// limit spent, as a round tells them.
    fn ask(&mut self, github: &GitHub<'_>, path: &str) -> Result<Value, Unread> {
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
        let mut answer = request
            .call()
            .map_err(|error| Unread::Failed(format!("couldn't list {path} on {name}: {error}")))?;
        let status = answer.status().as_u16();
        if status == 304 {
            let kept = self.kept.get(path).map(|(_, kept)| kept.clone());
            return kept.ok_or_else(|| {
                Unread::Failed(format!("{name} answered 304 for {path}, with nothing kept"))
            });
        }
        let header = |key: &str| {
            let value = answer.headers().get(key)?;
            value.to_str().ok().map(str::to_owned)
        };
        let tag = header("etag");
        let remaining = header("x-ratelimit-remaining");
        let retry_after = header("retry-after").is_some();
        if status != 200 {
            let said: Value = answer.body_mut().read_json().unwrap_or_default();
            let message = said["message"].as_str().unwrap_or("no reason");
            let why = format!("{name} answered {status} for {path}: {message}");
            let limited = github::rate_limited(status, remaining.as_deref(), retry_after, message);
            return Err(match status {
                403 | 404 | 410 if !limited => Unread::Gone(why),
                _ if limited => Unread::Limited(why),
                _ => Unread::Failed(why),
            });
        }
        let listed: Value = answer.body_mut().read_json().map_err(|error| {
            Unread::Failed(format!("{name}'s {path} couldn't be read: {error}"))
        })?;
        if let Some(tag) = tag {
            self.kept.insert(path.to_owned(), (tag, listed.clone()));
        }
        Ok(listed)
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{WATCHED, active_in};

    #[test]
    fn the_latest_repositories_are_watched_each_once_and_six_at_most() {
        let event = |at: &str, repo: &str| json!({"created_at": at, "repo": {"name": repo}});
        let events = json!([
            event("2026-11-25T10:00:00Z", "a/oldest"),
            event("2026-11-25T12:00:00Z", "a/one"),
            event("2026-11-25T11:59:00Z", "a/two"),
            event("2026-11-25T11:58:00Z", "a/one"),
            event("2026-11-25T11:57:00Z", "a/three"),
            event("2026-11-25T11:56:00Z", "a/four"),
            event("2026-11-25T11:55:00Z", "a/five"),
            event("2026-11-25T11:54:00Z", "a/six"),
        ]);
        assert_eq!(WATCHED, 6, "the count this case is written for");
        assert_eq!(
            active_in(&events),
            ["a/one", "a/two", "a/three", "a/four", "a/five", "a/six"],
            "the latest first, whatever order the feed gives, each once, the oldest left out"
        );
    }
}
