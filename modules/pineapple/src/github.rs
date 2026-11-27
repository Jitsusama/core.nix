//! GitHub's answers, read for the safe word: the pull requests and issues
//! Joel takes part in that changed, as his notifications name them, and
//! what was said on each, its opening, comments, review comments and
//! reviews, with who said it, whether a bot did, and when. A tag is
//! `@login` there. Only reading; the program asks
//! (src/bin/pineapple-github/).
//!
//! What was said is judged once, by when it was first said: the word added
//! later by editing an old comment or a description isn't heard, as an
//! edited Slack message isn't. That is a limit of what the safe word
//! hears, not only of replies, and saying it again in a new comment is
//! heard. Whether one was edited isn't kept, since it would only hold a
//! public reply back, and none is said on GitHub.

use serde_json::Value;

use crate::Context;

/// How GitHub says a writer is the repository's own: its owner, a member
/// of the organization owning it, or a collaborator on it. Only they may
/// stop Joel's agents on GitHub (Joel's orchestrator, 16 November).
const ITS_OWN: [&str; 3] = ["OWNER", "MEMBER", "COLLABORATOR"];

/// What GitHub lets Joel's sign-in do in a repository he works in, any
/// one of which is enough: push to it, maintain it or run it.
const WORKS_IN: [&str; 3] = ["push", "maintain", "admin"];

/// Seconds in a day.
const DAY: u64 = 86_400;

/// The days from 1 March of year 0 to 1 January 1970, the epoch, in the
/// proleptic Gregorian calendar the conversions below count by.
const EPOCH_DAYS: i64 = 719_468;

/// The days in four hundred years, after which the calendar repeats.
const ERA_DAYS: i64 = 146_097;

/// A pull request or issue Joel takes part in, as a notification or a
/// search names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Thread {
    /// The site it's on, as its reader names it: `github`, or another
    /// speaking GitHub's API, so the same repository on two sites is two.
    pub site: String,
    /// Its repository, as `owner/name`.
    pub repo: String,
    /// Its number.
    pub number: u64,
    /// Whether it's a pull request rather than an issue.
    pub pull: bool,
}

impl Thread {
    /// The key naming it, `<site>:<owner>/<name>#<number>`.
    #[must_use]
    pub fn key(&self) -> String {
        format!("{}:{}#{}", self.site, self.repo, self.number)
    }

    /// What a stop said on it is about: a pull request is itself; an issue
    /// is what its opening names. Its repository goes with it, but alone
    /// isn't a place: an issue naming nothing narrower stops everything,
    /// and Joel is told it fell back (src/scope.rs).
    #[must_use]
    pub fn context(&self, opening: &str) -> Context {
        if self.pull {
            return Context {
                pr: Some(self.number),
                repo: Some(self.repo.clone()),
                ..Context::default()
            };
        }
        let mut context = Context::of_thread(opening);
        context.repo.get_or_insert_with(|| self.repo.clone());
        context
    }
}

/// Where on a pull request or issue something was said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Its title and description.
    Opening,
    /// A comment in its conversation.
    Comment,
    /// A comment on a line of a pull request's change.
    ReviewComment,
    /// A review's own words.
    Review,
}

impl Kind {
    /// Its name in the key of what was said (`Said`).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Opening => "opening",
            Self::Comment => "comment",
            Self::ReviewComment => "review-comment",
            Self::Review => "review",
        }
    }
}

/// One thing said on a pull request or issue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Said {
    /// The key naming it, `<site>:<owner>/<name>#<number>:<kind>:<id>`,
    /// which names its record.
    pub key: String,
    /// Where GitHub shows it.
    pub link: String,
    /// Who said it, by their login.
    pub author: String,
    /// Whether a bot or an app said it.
    pub by_bot: bool,
    /// Whether GitHub names its writer the repository's own; one it says
    /// nothing of isn't, so a missing field can't let a stranger in.
    pub own: bool,
    /// What it says, as GitHub keeps it.
    pub text: String,
    /// When it was said, in seconds since the epoch.
    pub at: u64,
}

/// The pull requests and issues a page of notifications names, each once,
/// in the order named; anything else it names (a release, a discussion)
/// is let go.
#[must_use]
pub fn notified(answer: &Value, site: &str) -> Vec<Thread> {
    let mut threads: Vec<Thread> = Vec::new();
    for one in answer.as_array().into_iter().flatten() {
        let pull = match one["subject"]["type"].as_str() {
            Some("PullRequest") => true,
            Some("Issue") => false,
            _ => continue,
        };
        let number = one["subject"]["url"]
            .as_str()
            .and_then(|url| url.rsplit('/').next())
            .and_then(|number| number.parse().ok());
        let (Some(repo), Some(number)) = (one["repository"]["full_name"].as_str(), number) else {
            continue;
        };
        let thread = Thread {
            site: site.to_owned(),
            repo: repo.to_owned(),
            number,
            pull,
        };
        if !threads.contains(&thread) {
            threads.push(thread);
        }
    }
    threads
}

/// The pull requests and issues a page of search results names, each once,
/// for a site with no notifications to ask, where the reader searches for
/// what Joel opened instead.
#[must_use]
pub fn found(answer: &Value, site: &str) -> Vec<Thread> {
    found_when(answer, site)
        .into_iter()
        .map(|(thread, _)| thread)
        .collect()
}

/// What [`found`] names, each with when the search says it last changed,
/// as GitHub writes it, or empty where it doesn't say.
#[must_use]
pub fn found_when(answer: &Value, site: &str) -> Vec<(Thread, String)> {
    let mut threads: Vec<(Thread, String)> = Vec::new();
    for one in answer["items"].as_array().into_iter().flatten() {
        let repo = one["repository_url"].as_str().and_then(|url| {
            let mut parts = url.rsplit('/');
            let name = parts.next()?;
            Some(format!("{}/{name}", parts.next()?))
        });
        let (Some(repo), Some(number)) = (repo, one["number"].as_u64()) else {
            continue;
        };
        let thread = Thread {
            site: site.to_owned(),
            repo,
            number,
            pull: one.get("pull_request").is_some(),
        };
        if !threads.iter().any(|(known, _)| *known == thread) {
            let at = one["updated_at"].as_str().unwrap_or_default();
            threads.push((thread, at.to_owned()));
        }
    }
    threads
}

/// Whether a bot or an app is who said something: GitHub's own mark, or a
/// cloud service account's login, which is how a site whose logins are
/// addresses names its checks and its reviewing apps.
fn by_bot(user: &Value) -> bool {
    user["type"] == "Bot"
        || user["login"]
            .as_str()
            .is_some_and(|login| login.ends_with(".gserviceaccount.com"))
}

/// Whether GitHub names `one`'s writer the repository's own.
fn own(one: &Value) -> bool {
    one["author_association"]
        .as_str()
        .is_some_and(|said| ITS_OWN.contains(&said))
}

/// Whether Joel works in a repository, as GitHub's answer for it says to
/// his sign-in. Anyone may make a repository, own it and mention him
/// there, so its own count only in one he works in.
#[must_use]
pub fn works_in(repo: &Value) -> bool {
    WORKS_IN.iter().any(|may| repo["permissions"][may] == true)
}

/// What an answer holds that was said on `thread`: for an opening, the
/// pull request or issue itself; else a list of what `kind` names. One
/// that can't be read whole, or a review with no words, is let go.
#[must_use]
pub fn said(answer: &Value, thread: &Thread, kind: Kind) -> Vec<Said> {
    match kind {
        Kind::Opening => opening(answer, thread).into_iter().collect(),
        _ => answer
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|one| one_said(one, thread, kind))
            .collect(),
    }
}

/// A pull request's or issue's opening: its title and description.
fn opening(answer: &Value, thread: &Thread) -> Option<Said> {
    let title = answer["title"].as_str()?;
    let body = answer["body"].as_str().unwrap_or_default();
    Some(Said {
        key: format!("{}:{}", thread.key(), Kind::Opening.name()),
        link: answer["html_url"].as_str()?.to_owned(),
        author: answer["user"]["login"].as_str()?.to_owned(),
        by_bot: by_bot(&answer["user"]),
        own: own(answer),
        text: format!("{title}\n\n{body}"),
        at: seconds(answer["created_at"].as_str()?)?,
    })
}

/// One comment, review comment or review.
fn one_said(one: &Value, thread: &Thread, kind: Kind) -> Option<Said> {
    let text = one["body"]
        .as_str()
        .filter(|text| !text.trim().is_empty())?;
    let when = if kind == Kind::Review {
        "submitted_at"
    } else {
        "created_at"
    };
    let at = seconds(one[when].as_str()?)?;
    Some(Said {
        key: format!("{}:{}:{}", thread.key(), kind.name(), one["id"].as_u64()?),
        link: one["html_url"].as_str()?.to_owned(),
        author: one["user"]["login"].as_str()?.to_owned(),
        by_bot: by_bot(&one["user"]),
        own: own(one),
        text: text.to_owned(),
        at,
    })
}

/// Whether something said is worth judging: it holds the phrase's first
/// word, or tags `login`, as the Slack reader's two searches find theirs.
#[must_use]
pub fn worth_judging(text: &str, login: &str) -> bool {
    let text = text.to_lowercase();
    text.contains("pineapple") || text.contains(&format!("@{}", login.to_lowercase()))
}

/// Seconds since the epoch of a time as GitHub writes it,
/// `2026-11-14T15:04:05Z`; none for one written otherwise.
#[must_use]
pub fn seconds(at: &str) -> Option<u64> {
    let (date, time) = at.strip_suffix('Z')?.split_once('T')?;
    let mut date = date.splitn(3, '-').map(str::parse::<i64>);
    let (year, month, day) = (date.next()?.ok()?, date.next()?.ok()?, date.next()?.ok()?);
    let mut time = time.splitn(3, ':').map(str::parse::<u64>);
    let (hour, minute, second) = (time.next()?.ok()?, time.next()?.ok()?, time.next()?.ok()?);
    // Sixty, for the leap second a clock may write.
    let out = hour > 23 || minute > 59 || second > 60;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || out {
        return None;
    }
    let days = u64::try_from(days_from_civil(year, month, day)).ok()?;
    Some(days * DAY + hour * 3600 + minute * 60 + second)
}

/// The next page a `Link` header names, as GitHub pages a long list.
#[must_use]
pub fn next_page(link: &str) -> Option<&str> {
    link.split(',').find_map(|part| {
        let (url, rel) = part.split_once(';')?;
        let names_next = rel.split(';').any(|one| one.trim() == "rel=\"next\"");
        let url = url.trim().strip_prefix('<')?.strip_suffix('>')?;
        names_next.then_some(url)
    })
}

/// A page added to a list read so far: a list's own, or a search's
/// items. A page of another shape adds nothing.
pub fn add_page(list: &mut Value, page: Value) {
    let (kept, more) = match (list, page) {
        (Value::Array(kept), Value::Array(more)) => (kept, more),
        (Value::Object(kept), Value::Object(mut more)) => {
            match (kept.get_mut("items"), more.remove("items")) {
                (Some(Value::Array(kept)), Some(Value::Array(more))) => (kept, more),
                _ => return,
            }
        }
        _ => return,
    };
    kept.extend(more);
}

/// Whether GitHub refused for its rate limit rather than because what
/// was asked is gone: a 429, or a 403 saying none is left, when to ask
/// again, or in its message that it is a rate limit (or, as GitHub once
/// worded one, abuse detection), since a secondary limit may come with
/// neither header. Joel's limit is shared with gh and
/// every agent signed in as him, so it runs out, and a round it stops has
/// to be tried again whole.
#[must_use]
pub fn rate_limited(
    status: u16,
    remaining: Option<&str>,
    retry_after: bool,
    message: &str,
) -> bool {
    let message = message.to_ascii_lowercase();
    let says_so = message.contains("rate limit") || message.contains("abuse detection");
    status == 429 || (status == 403 && (remaining == Some("0") || retry_after || says_so))
}

/// A time as GitHub reads one in `since`.
#[must_use]
pub fn stamp(seconds: u64) -> String {
    let days = i64::try_from(seconds / DAY).unwrap_or(i64::MAX);
    let (year, month, day) = civil_from_days(days);
    let within = seconds % DAY;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        within / 3600,
        within % 3600 / 60,
        within % 60
    )
}

/// The days from the epoch to a date (Howard Hinnant's algorithm, which
/// counts years from March so a leap day falls last).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let of_era_days = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * ERA_DAYS + of_era_days - EPOCH_DAYS
}

/// The date a count of days from the epoch falls on, the inverse of
/// `days_from_civil`.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + EPOCH_DAYS;
    let era = days.div_euclid(ERA_DAYS);
    let of_era = days - era * ERA_DAYS;
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted = (5 * of_year + 2) / 153;
    let day = of_year - (153 * shifted + 2) / 5 + 1;
    let month = if shifted < 10 {
        shifted + 3
    } else {
        shifted - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn pull() -> Thread {
        Thread {
            site: "github".to_owned(),
            repo: "example/shop".to_owned(),
            number: 12,
            pull: true,
        }
    }

    #[test]
    fn a_time_reads_as_github_writes_it_and_back() {
        assert_eq!(seconds("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(seconds("2023-11-14T22:13:20Z"), Some(1_700_000_000));
        assert_eq!(
            seconds("2024-02-29T00:00:00Z"),
            Some(1_709_164_800),
            "a leap day"
        );
        assert_eq!(stamp(1_700_000_000), "2023-11-14T22:13:20Z");
        assert_eq!(stamp(1_709_164_800), "2024-02-29T00:00:00Z");
        assert_eq!(seconds("2023-11-14 22:13:20"), None, "written otherwise");
        assert_eq!(seconds("2023-13-14T22:13:20Z"), None, "no thirteenth month");
    }

    #[test]
    fn notifications_name_each_pull_request_and_issue_once() {
        let one = |kind: &str, url: &str| {
            json!({"subject": {"type": kind, "url": url},
                "repository": {"full_name": "example/shop"}})
        };
        let answer = json!([
            one(
                "PullRequest",
                "https://api.example.com/repos/example/shop/pulls/12"
            ),
            one(
                "Issue",
                "https://api.example.com/repos/example/shop/issues/7"
            ),
            one(
                "PullRequest",
                "https://api.example.com/repos/example/shop/pulls/12"
            ),
            one(
                "Release",
                "https://api.example.com/repos/example/shop/releases/1"
            ),
        ]);
        let issue = Thread {
            number: 7,
            pull: false,
            ..pull()
        };
        assert_eq!(notified(&answer, "github"), [pull(), issue]);
    }

    #[test]
    fn a_search_names_each_pull_request_once_on_its_own_site() {
        let one = |number: u64, pull: bool| {
            let mut item = json!({"number": number,
                "repository_url": "https://code.example.com/api/v3/repos/example/shop"});
            if pull {
                item["pull_request"] = json!({});
            }
            item
        };
        let answer = json!({"items": [one(12, true), one(12, true), one(7, false)]});
        let on_site = |thread: Thread| Thread {
            site: "code".to_owned(),
            ..thread
        };
        let issue = Thread {
            number: 7,
            pull: false,
            ..pull()
        };
        assert_eq!(
            found(&answer, "code"),
            [on_site(pull()), on_site(issue)],
            "each once, in order, an issue told from a pull request"
        );
        assert_eq!(
            on_site(pull()).key(),
            "code:example/shop#12",
            "its site names it, so the same repository elsewhere is another"
        );
    }

    #[test]
    fn a_cloud_service_account_is_a_bot() {
        let by = |login: &str, kind: &str| {
            json!([{"id": 1, "body": "pineapple override", "html_url": "https://code.example.com/x",
                "user": {"login": login, "type": kind}, "created_at": "2023-11-14T22:13:20Z"}])
        };
        let bot = |answer: Value| said(&answer, &pull(), Kind::Comment)[0].by_bot;
        assert!(
            bot(by("checks@example.iam.gserviceaccount.com", "User")),
            "a service account, though marked a user"
        );
        assert!(!bot(by("joel@example.com", "User")), "a person by address");
    }

    #[test]
    fn what_was_said_carries_who_said_it_and_when() {
        let answer = json!([
            {"id": 5, "body": "@joel.example pineapple override",
                "html_url": "https://code.example.com/example/shop/pull/12#c5",
                "user": {"login": "colleague", "type": "User"},
                "created_at": "2023-11-14T22:13:20Z", "updated_at": "2023-11-14T22:15:00Z"},
            {"id": 6, "body": "  ", "html_url": "https://code.example.com/x",
                "user": {"login": "colleague", "type": "User"},
                "created_at": "2023-11-14T22:13:20Z"},
            {"id": 7, "body": "merged", "html_url": "https://code.example.com/y",
                "user": {"login": "merger[bot]", "type": "Bot"},
                "created_at": "2023-11-14T22:13:20Z", "updated_at": "2023-11-14T22:13:20Z"},
        ]);
        let said = said(&answer, &pull(), Kind::Comment);
        assert_eq!(said.len(), 2, "a comment with no words is let go");
        assert_eq!(said[0].key, "github:example/shop#12:comment:5");
        assert_eq!(said[0].author, "colleague");
        assert_eq!(said[0].at, 1_700_000_000);
        assert!(!said[0].by_bot && said[1].by_bot, "a bot is marked");
    }

    #[test]
    fn only_the_repositorys_own_are_its_own_and_joel_works_where_he_may_push() {
        let by = |association: Option<&str>| {
            let mut one = json!({"id": 1, "body": "pineapple override",
                "html_url": "https://code.example.com/x",
                "user": {"login": "colleague", "type": "User"},
                "created_at": "2023-11-14T22:13:20Z"});
            if let Some(association) = association {
                one["author_association"] = json!(association);
            }
            json!([one])
        };
        let own_one = |association| said(&by(association), &pull(), Kind::Comment)[0].own;
        for own in ["OWNER", "MEMBER", "COLLABORATOR"] {
            assert!(own_one(Some(own)), "{own} is the repository's own");
        }
        for stranger in [
            "CONTRIBUTOR",
            "FIRST_TIME_CONTRIBUTOR",
            "FIRST_TIMER",
            "NONE",
        ] {
            assert!(!own_one(Some(stranger)), "{stranger} isn't");
        }
        assert!(!own_one(None), "nor is a writer GitHub says nothing of");
        for may in ["push", "maintain", "admin"] {
            assert!(
                works_in(&json!({"permissions": {"pull": true, may: true}})),
                "Joel works where he may {may}"
            );
        }
        assert!(
            !works_in(&json!({"permissions": {"pull": true, "push": false}})),
            "not where he may only read, as anyone's own public repository"
        );
        assert!(!works_in(&json!({})), "nor where GitHub says nothing");
        let opening = json!({"title": "Stop", "body": "pineapple override",
            "html_url": "https://code.example.com/example/shop/pull/12",
            "user": {"login": "colleague", "type": "User"}, "author_association": "NONE",
            "created_at": "2023-11-14T22:13:20Z"});
        assert!(
            !said(&opening, &pull(), Kind::Opening)[0].own,
            "an opening is read for its writer's standing too"
        );
    }

    #[test]
    fn an_opening_is_its_title_and_description() {
        let answer = json!({"title": "Stop", "body": "pineapple override",
            "html_url": "https://code.example.com/example/shop/pull/12",
            "user": {"login": "colleague", "type": "User"},
            "created_at": "2023-11-14T22:13:20Z", "updated_at": "2023-11-15T00:00:00Z"});
        let said = said(&answer, &pull(), Kind::Opening);
        assert_eq!(said[0].key, "github:example/shop#12:opening");
        assert_eq!(said[0].text, "Stop\n\npineapple override");
        assert_eq!(said[0].at, 1_700_000_000, "when it was opened");
    }

    #[test]
    fn a_pull_request_is_its_own_place_and_an_issue_what_it_names() {
        let on_pull = pull().context("anything");
        assert_eq!(
            (on_pull.pr, on_pull.repo.as_deref()),
            (Some(12), Some("example/shop"))
        );
        let issue = Thread {
            pull: false,
            ..pull()
        };
        let named = issue.context("broken by example/other#3");
        assert_eq!(
            (named.pr, named.repo.as_deref()),
            (Some(3), Some("example/other"))
        );
        let plain = issue.context("it's broken");
        assert_eq!(
            (plain.pr, plain.repo.as_deref()),
            (None, Some("example/shop"))
        );
    }

    #[test]
    fn a_long_list_names_its_next_page_and_its_last_names_none() {
        let link = "<https://api.example.com/notifications?page=2>; rel=\"next\", \
                    <https://api.example.com/notifications?page=5>; rel=\"last\"";
        assert_eq!(
            next_page(link),
            Some("https://api.example.com/notifications?page=2")
        );
        let last = "<https://api.example.com/notifications?page=1>; rel=\"first\", \
                    <https://api.example.com/notifications?page=4>; rel=\"prev\"";
        assert_eq!(next_page(last), None);
        assert_eq!(next_page(""), None);
    }

    #[test]
    fn a_page_adds_to_a_list_or_a_searchs_items_and_nothing_else() {
        let mut list = json!([1]);
        add_page(&mut list, json!([2, 3]));
        assert_eq!(list, json!([1, 2, 3]), "a list's next page follows it");
        let mut search = json!({"total_count": 3, "items": [1]});
        add_page(&mut search, json!({"total_count": 3, "items": [2, 3]}));
        assert_eq!(
            search,
            json!({"total_count": 3, "items": [1, 2, 3]}),
            "a search's next items follow its first"
        );
        add_page(&mut list, json!({"items": [4]}));
        assert_eq!(
            list,
            json!([1, 2, 3]),
            "a page of another shape adds nothing"
        );
    }

    #[test]
    fn a_rate_limit_is_told_from_something_gone() {
        let secondary = "You have exceeded a secondary rate limit";
        assert!(rate_limited(429, None, false, ""), "too many requests");
        assert!(rate_limited(403, Some("0"), false, ""), "none left");
        assert!(rate_limited(403, Some("12"), true, ""), "a secondary limit");
        assert!(
            rate_limited(403, Some("12"), false, secondary),
            "one said only in words"
        );
        assert!(
            rate_limited(403, None, false, "API Rate Limit Exceeded"),
            "whatever its case"
        );
        let abuse = "You have triggered an abuse detection mechanism.";
        assert!(
            rate_limited(403, None, false, abuse),
            "in the words GitHub once used"
        );
        assert!(
            !rate_limited(403, Some("12"), false, "Must have push access"),
            "not his to read"
        );
        assert!(!rate_limited(404, Some("0"), false, "Not Found"), "gone");
    }

    #[test]
    fn a_second_past_sixty_is_no_time() {
        assert_eq!(seconds("2023-11-14T22:13:61Z"), None);
        assert!(seconds("2016-12-31T23:59:60Z").is_some(), "a leap second");
    }

    #[test]
    fn worth_judging_is_the_phrase_or_a_tag() {
        assert!(
            worth_judging("Pineapple override", "joel.example"),
            "the phrase, in any case"
        );
        assert!(
            worth_judging("@Joel.Example please stop", "joel.example"),
            "a tag of Joel, in any case"
        );
        assert!(!worth_judging("looks good", "joel.example"), "neither");
    }
}
