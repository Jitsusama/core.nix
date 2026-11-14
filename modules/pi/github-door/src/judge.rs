//! Which calls to GitHub the door carries, judged from the method, the
//! host, the path and, for GraphQL, the body; nothing here touches the
//! network, so every case is a unit test.
//!
//! Every read passes, but for credentials: anything under tokens, keys,
//! secrets, hooks, authorizations, applications, installations, or
//! actions' secrets and variables. Writes pass only to issues and their
//! comments, labels and reactions, pull requests other than their
//! opening, their pending or commenting reviews and review comments,
//! GraphQL mutations on the list (src/graphql.rs), and git's fetch.
//! Branch protection and rulesets may be read, never written. A
//! repository's own label is never deleted, since that takes it off every
//! issue. A push, a pull request opened, a merge and a review's verdict
//! (an approval, changes requested, a dismissal) are refused naming the
//! route that does them; every other write is refused as not allowed.

use crate::graphql::{self, Carried};
use crate::said::{Refusal, Seen, Why};

/// The route for a push.
const PUSH_ROUTE: &str = "push through the work tool (work push)";
/// The route for a pull request opened.
const PROPOSE_ROUTE: &str = "open a pull request with review_offer propose";
/// The route for a merge.
const MERGE_ROUTE: &str = "merge with review_offer merge";
/// The route for a review's verdict.
const VERDICT_ROUTE: &str = "review through the review tools (review_draft), outside the fence";
/// The one event a review sent from inside may carry, beside none at all,
/// which leaves it pending.
const COMMENTING: &str = "COMMENT";

/// The segments that mark credentials wherever a resource is named.
const CREDENTIALS: [&str; 19] = [
    "keys",
    "gpg_keys",
    "ssh_signing_keys",
    "secrets",
    "variables",
    "hooks",
    "authorizations",
    "applications",
    "installations",
    "installation",
    "access_tokens",
    "personal-access-tokens",
    "personal-access-token-requests",
    "registration-token",
    "remove-token",
    "generate-jitconfig",
    "credential-authorizations",
    "tokens",
    "token",
];

/// Top-level paths that are a GitHub App's own, or an OAuth app's.
const APP_PATHS: [&str; 5] = [
    "app",
    "app-manifests",
    "applications",
    "authorizations",
    "installation",
];

/// Areas of a repository or an organization whose every segment is read
/// for credentials, since secrets and variables sit inside them.
const CREDENTIAL_AREAS: [&str; 5] = ["actions", "dependabot", "codespaces", "agents", "copilot"];

/// The four hosts the door answers as GitHub; every other is tunnelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    Api,
    Web,
    Uploads,
    Codeload,
}

impl Host {
    /// Every host the door answers as.
    pub const ALL: [Host; 4] = [Host::Api, Host::Web, Host::Uploads, Host::Codeload];

    /// The host named `name`, when it's one of the four.
    pub fn named(name: &str) -> Option<Host> {
        let name = name.strip_suffix('.').unwrap_or(name);
        Host::ALL
            .into_iter()
            .find(|host| host.name().eq_ignore_ascii_case(name))
    }

    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Host::Api => "api.github.com",
            Host::Web => "github.com",
            Host::Uploads => "uploads.github.com",
            Host::Codeload => "codeload.github.com",
        }
    }
}

/// A call, as the judge reads it.
#[derive(Clone, Copy, Debug)]
pub struct Call<'a> {
    pub method: &'a str,
    pub host: Host,
    /// The request target: an absolute path and its query.
    pub target: &'a str,
    pub body: &'a [u8],
}

/// The verdict on `call`, with `allowed` the mutations let through: what
/// a line says of it, or why it's refused.
pub fn judge(call: &Call<'_>, allowed: &[&str]) -> Result<Seen, Refusal> {
    let (path, query) = call.target.split_once('?').unwrap_or((call.target, ""));
    let seen = Seen::Path(path.to_owned());
    let segments = segments(path)?;
    if query_param(query, "_method").is_some() {
        return Err(Refusal::new(
            Why::MethodOverride,
            "a _method parameter, asking GitHub to read the method as another",
        ));
    }
    let read = match call.method {
        "GET" | "HEAD" | "OPTIONS" => true,
        "POST" | "PUT" | "PATCH" | "DELETE" => false,
        _ => {
            return Err(Refusal::new(
                Why::NotAllowed,
                format!(
                    "the method {}, which the door doesn't carry",
                    crate::said::plain(call.method)
                ),
            ));
        }
    };
    match call.host {
        Host::Api => {
            api(call, &segments, query, read, allowed).map(|fields| fields.unwrap_or(seen))
        }
        Host::Web => web(call, &segments, query, read).map(|()| seen),
        Host::Uploads | Host::Codeload if read => Ok(seen),
        Host::Uploads | Host::Codeload => Err(not_allowed(call)),
    }
}

/// The refusal of a write the door doesn't let through.
fn not_allowed(call: &Call<'_>) -> Refusal {
    Refusal::new(
        Why::NotAllowed,
        format!(
            "a {} to {}, a write the door doesn't let through",
            call.method,
            call.host.name()
        ),
    )
}

/// A path's segments, each percent-decoded, or why it can't be read: a
/// path GitHub might read as another (a dot segment, an empty one, or an
/// encoded slash) is refused rather than judged.
fn segments(path: &str) -> Result<Vec<String>, Refusal> {
    let unreadable = |what: &str| Refusal::new(Why::Unreadable, format!("a path {what}"));
    let Some(rest) = path.strip_prefix('/') else {
        return Err(unreadable("that isn't absolute"));
    };
    if rest.is_empty() {
        return Ok(Vec::new());
    }
    let rest = rest.strip_suffix('/').unwrap_or(rest);
    rest.split('/')
        .map(|raw| {
            let decoded = decode(raw).ok_or_else(|| unreadable("with a broken escape"))?;
            if decoded.is_empty() {
                return Err(unreadable("with an empty segment"));
            }
            if decoded == "." || decoded == ".." {
                return Err(unreadable("with a dot segment"));
            }
            if decoded.contains(['/', '\\']) || decoded.chars().any(char::is_control) {
                return Err(unreadable(
                    "with a slash or a control character inside a segment",
                ));
            }
            Ok(decoded)
        })
        .collect()
}

/// `text` with its percent escapes decoded, or none when one is broken or
/// what it decodes to isn't UTF-8.
fn decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' {
            let hex = text.get(at + 1..at + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// The value of `name` in a query string, decoded, `+` as a space.
pub fn query_param(query: &str, name: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = decode(&key.replace('+', " "))?;
        (key == name).then(|| decode(&value.replace('+', " ")).unwrap_or_default())
    })
}

/// Whether a segment is `word`, as GitHub would read it whatever its case.
fn is(segment: Option<&String>, word: &str) -> bool {
    segment.is_some_and(|segment| segment.eq_ignore_ascii_case(word))
}

/// Whether a segment is any of `words`.
fn any_of(segment: Option<&String>, words: &[&str]) -> bool {
    words.iter().any(|word| is(segment, word))
}

/// What a REST path is under: a repository, an organization, the signed-in
/// user, or none of those, and the segments after it.
enum Under<'a> {
    Repo(&'a [String]),
    Org(&'a [String]),
    User(&'a [String]),
    Top(&'a [String]),
}

fn under(segments: &[String]) -> Under<'_> {
    let first = segments.first();
    if is(first, "repos") && segments.len() >= 3 {
        Under::Repo(&segments[3..])
    } else if is(first, "repositories") && segments.len() >= 2 {
        Under::Repo(&segments[2..])
    } else if (is(first, "orgs") || is(first, "organizations")) && segments.len() >= 2 {
        Under::Org(&segments[2..])
    } else if is(first, "users") && segments.len() >= 2 {
        Under::User(&segments[2..])
    } else if is(first, "user") {
        Under::User(&segments[1..])
    } else {
        Under::Top(segments)
    }
}

/// Whether a resource's segments reach credentials: the resource itself
/// is one, or it's an area holding secrets and variables and any segment
/// in it is one, or an environment's secrets and variables.
fn credentials(rest: &[String]) -> bool {
    let first = rest.first();
    if any_of(first, &CREDENTIALS) {
        return true;
    }
    if any_of(first, &CREDENTIAL_AREAS) {
        return rest
            .iter()
            .any(|segment| any_of(Some(segment), &CREDENTIALS));
    }
    is(first, "environments") && any_of(rest.get(2), &["secrets", "variables"])
}

/// The verdict on a call to the API: the GraphQL fields when it's GraphQL,
/// none for REST, which says its path.
fn api(
    call: &Call<'_>,
    segments: &[String],
    query: &str,
    read: bool,
    allowed: &[&str],
) -> Result<Option<Seen>, Refusal> {
    if is(segments.first(), "graphql") {
        return match call.method {
            "POST" => graphql::judge(call.body, Carried::Body, allowed).map(Some),
            "GET" | "HEAD" => match query_param(query, "query") {
                Some(document) => {
                    graphql::judge(document.as_bytes(), Carried::Get, allowed).map(Some)
                }
                None => Ok(None),
            },
            _ => Err(not_allowed(call)),
        };
    }
    let place = under(segments);
    let credential = match &place {
        Under::Top(rest) => any_of(rest.first(), &APP_PATHS),
        Under::Repo(rest) | Under::Org(rest) | Under::User(rest) => credentials(rest),
    };
    if credential {
        return Err(Refusal::new(
            Why::Credentials,
            "credentials: tokens, keys, secrets, variables, hooks, authorizations, applications \
             and installations stay outside the fence",
        ));
    }
    if read {
        return Ok(None);
    }
    match place {
        Under::Repo(rest) => repo_write(call, rest).map(|()| None),
        Under::Org(rest) if any_of(rest.first(), &["rulesets"]) => Err(protection()),
        _ => Err(not_allowed(call)),
    }
}

/// The refusal of a write to branch protection or rulesets.
fn protection() -> Refusal {
    Refusal::new(
        Why::Protection,
        "a write to branch protection or rulesets, which may be read but not written",
    )
}

/// The verdict on a REST write under a repository.
fn repo_write(call: &Call<'_>, rest: &[String]) -> Result<(), Refusal> {
    let method = call.method;
    let first = rest.first();
    if rest.is_empty() {
        return if matches!(method, "DELETE" | "PATCH") {
            Err(Refusal::new(
                Why::RepoSettings,
                "a repository deleted or its settings changed, which an agent never does",
            ))
        } else {
            Err(not_allowed(call))
        };
    }
    if any_of(first, &["rulesets", "rules"])
        || (is(first, "branches") && rest.iter().any(|segment| is(Some(segment), "protection")))
    {
        return Err(protection());
    }
    if is(first, "merges") {
        return Err(Refusal::new(
            Why::Merge,
            format!("a branch merged; {MERGE_ROUTE}"),
        ));
    }
    if is(first, "labels") && method == "DELETE" {
        return Err(Refusal::new(
            Why::NotAllowed,
            "a repository's label deleted, which takes it off every issue; \
             an issue's own label may be taken off",
        ));
    }
    if any_of(first, &["issues", "labels"]) {
        return Ok(());
    }
    if is(first, "comments") || is(first, "releases") {
        // A reaction to a commit comment or a release, and taking it back.
        let reaction = rest.len() >= 3
            && is(rest.get(2), "reactions")
            && rest.len() <= 4
            && matches!(method, "POST" | "DELETE");
        return if reaction {
            Ok(())
        } else {
            Err(not_allowed(call))
        };
    }
    if is(first, "pulls") {
        return pull_write(call, &rest[1..]);
    }
    Err(not_allowed(call))
}

/// The verdict on a REST write under a repository's `pulls`, given the
/// segments after it.
fn pull_write(call: &Call<'_>, rest: &[String]) -> Result<(), Refusal> {
    let method = call.method;
    let at = |index: usize, word: &str| is(rest.get(index), word);
    let passes = match rest.len() {
        0 if method == "POST" => {
            return Err(Refusal::new(
                Why::PullCreate,
                format!("a pull request opened; {PROPOSE_ROUTE}"),
            ));
        }
        // Its title, body, base or state.
        1 => method == "PATCH" && !at(0, "comments"),
        _ if at(1, "merge") => {
            return Err(Refusal::new(
                Why::Merge,
                format!("a pull request merged; {MERGE_ROUTE}"),
            ));
        }
        // A review comment edited or deleted, and reactions to one.
        _ if at(0, "comments") => match rest.len() {
            2 => matches!(method, "PATCH" | "DELETE"),
            3 | 4 => at(2, "reactions") && matches!(method, "POST" | "DELETE"),
            _ => false,
        },
        _ if at(1, "reviews") => return review_write(call, &rest[2..]),
        _ => {
            // Reviewers asked, review comments, and replies.
            (at(1, "requested_reviewers") && rest.len() == 2)
                || (at(1, "comments") && method == "POST")
        }
    };
    if passes {
        Ok(())
    } else {
        Err(not_allowed(call))
    }
}

/// The verdict on a REST write under a pull request's `reviews`, given
/// the segments after it. A review may be started, edited, commented and
/// taken back while pending, but an approval, changes requested or a
/// dismissal would move what a merge waits on, in the person's name.
fn review_write(call: &Call<'_>, rest: &[String]) -> Result<(), Refusal> {
    let method = call.method;
    let at = |index: usize, word: &str| is(rest.get(index), word);
    let sent = match rest.len() {
        // A review started, pending or with its event.
        0 => method == "POST",
        // A pending review's text edited, or the pending review deleted.
        1 => return passing(call, matches!(method, "PUT" | "DELETE")),
        2 if at(1, "dismissals") => {
            return Err(Refusal::new(
                Why::Verdict,
                format!("a review dismissed; {VERDICT_ROUTE}"),
            ));
        }
        // A pending review submitted with its event.
        2 if at(1, "events") => method == "POST",
        _ => false,
    };
    if !sent {
        return Err(not_allowed(call));
    }
    let query = call.target.split_once('?').map_or("", |(_, query)| query);
    if query_param(query, "event").is_some() {
        return Err(Refusal::new(
            Why::Verdict,
            format!("a review's event sent in its query, which isn't read; {VERDICT_ROUTE}"),
        ));
    }
    let events = events_in(call.body)
        .ok_or_else(|| Refusal::new(Why::Unreadable, "a review whose body isn't a JSON object"))?;
    let commenting = events.iter().all(|event| {
        event
            .as_str()
            .is_some_and(|event| event.eq_ignore_ascii_case(COMMENTING))
    });
    if commenting {
        Ok(())
    } else {
        Err(Refusal::new(
            Why::Verdict,
            format!("a review approving or requesting changes; {VERDICT_ROUTE}"),
        ))
    }
}

/// Allowed, or refused as not allowed.
fn passing(call: &Call<'_>, passes: bool) -> Result<(), Refusal> {
    if passes {
        Ok(())
    } else {
        Err(not_allowed(call))
    }
}

/// Every `event` a JSON body's top level gives, each one where a key is
/// given twice, since which of two GitHub keeps isn't the door's to
/// guess. Nothing, for an empty body; `None`, for one that isn't an
/// object.
fn events_in(body: &[u8]) -> Option<Vec<serde_json::Value>> {
    use serde::de::{Deserializer, MapAccess, Visitor};

    struct Events;
    impl<'de> Visitor<'de> for Events {
        type Value = Vec<serde_json::Value>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a JSON object")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut events = Vec::new();
            while let Some(key) = map.next_key::<String>()? {
                let value = map.next_value::<serde_json::Value>()?;
                if key == "event" {
                    events.push(value);
                }
            }
            Ok(events)
        }
    }

    if body.iter().all(u8::is_ascii_whitespace) {
        return Some(Vec::new());
    }
    let mut reader = serde_json::Deserializer::from_slice(body);
    let events = (&mut reader).deserialize_map(Events).ok()?;
    reader.end().ok()?;
    Some(events)
}

/// The verdict on a call to github.com: git's fetch and every read pass,
/// a push is refused, and so is every other write.
fn web(call: &Call<'_>, segments: &[String], query: &str, read: bool) -> Result<(), Refusal> {
    let last = segments.last();
    let pushing = is(last, "git-receive-pack")
        || query_param(query, "service").is_some_and(|service| service == "git-receive-pack");
    if pushing {
        return Err(Refusal::new(
            Why::ReceivePack,
            format!("a push; {PUSH_ROUTE}"),
        ));
    }
    if read {
        return Ok(());
    }
    let repository = segments.len() >= 3;
    if call.method == "POST" && repository && is(last, "git-upload-pack") && segments.len() == 3 {
        return Ok(());
    }
    let lfs_batch = call.method == "POST"
        && segments.len() == 6
        && is(segments.get(2), "info")
        && is(segments.get(3), "lfs")
        && is(segments.get(4), "objects")
        && is(last, "batch");
    if lfs_batch {
        return match serde_json::from_slice::<serde_json::Value>(call.body)
            .ok()
            .and_then(|body| {
                body.get("operation")
                    .and_then(|op| op.as_str().map(str::to_owned))
            })
            .as_deref()
        {
            Some("download") => Ok(()),
            Some("upload") => Err(Refusal::new(
                Why::ReceivePack,
                format!("an LFS upload; {PUSH_ROUTE}"),
            )),
            _ => Err(Refusal::new(
                Why::Unreadable,
                "an LFS batch naming no operation",
            )),
        };
    }
    Err(not_allowed(call))
}

#[cfg(test)]
mod tests;
