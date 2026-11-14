//! What the door says in the journal of each call it passes and each it
//! refuses, one line apiece in a shape that never changes.
//!
//! A reader of the journal keeps only fields, never a line's free text,
//! since free text could carry whatever a client sent. So each line opens
//! with fixed fields: what happened, why as one word from a fixed set, the
//! session the call came in on, the session its process claims
//! (src/peer.rs), the method, the host, the path or the GraphQL fields,
//! and the status. Only then, after ` -- `, comes the sentence a person
//! reads. No line ever holds a body or a header.
//!
//! Two, each one line though wrapped here:
//!
//! ```text
//! pi-github-door: passed session=01a1... claimed-session=01a1... method=GET
//!     host=api.github.com path=/user status=200
//! pi-github-door: refused why=merge session=01a1... claimed-session=01a1... method=PUT
//!     host=api.github.com path=/repos/o/r/pulls/1/merge status=403 -- ...
//! ```

use std::fmt::Write;

/// Why a call was refused, as the one word its line carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// What the client sent couldn't be read as a request the door judges.
    Unreadable,
    /// A GraphQL document that doesn't parse.
    Unparsable,
    /// A write the door doesn't let through.
    NotAllowed,
    /// A GraphQL mutation not on the list.
    Mutation,
    /// A GraphQL mutation sent over GET.
    GetMutation,
    /// Several GraphQL operations sent as one body.
    Batch,
    /// A GraphQL body naming a stored query rather than holding one.
    Persisted,
    /// A GraphQL subscription.
    Subscription,
    /// A push, through git's receive-pack or LFS's upload.
    ReceivePack,
    /// A pull request opened.
    PullCreate,
    /// A merge, or auto-merge turned on.
    Merge,
    /// Anything under tokens, keys, secrets, hooks, authorizations,
    /// applications, installations, or actions' secrets and variables.
    Credentials,
    /// A repository deleted or its settings changed.
    RepoSettings,
    /// A write to branch protection or rulesets.
    Protection,
    /// An Authorization other than the placeholder.
    Authorization,
    /// A header asking GitHub to read the method as another.
    MethodOverride,
    /// A body past the most the door reads.
    TooLarge,
    /// A session past its hourly cap.
    Cap,
    /// The person's token couldn't be read.
    NoToken,
    /// GitHub couldn't be reached.
    Upstream,
    /// A tunnel to one of GitHub's hosts on a port other than 443.
    Port,
}

impl Why {
    /// The word a line carries for this reason.
    pub fn word(self) -> &'static str {
        match self {
            Why::Unreadable => "unreadable",
            Why::Unparsable => "unparsable",
            Why::NotAllowed => "not-allowed",
            Why::Mutation => "mutation",
            Why::GetMutation => "get-mutation",
            Why::Batch => "batch",
            Why::Persisted => "persisted",
            Why::Subscription => "subscription",
            Why::ReceivePack => "receive-pack",
            Why::PullCreate => "pull-create",
            Why::Merge => "merge",
            Why::Credentials => "credentials",
            Why::RepoSettings => "repo-settings",
            Why::Protection => "protection",
            Why::Authorization => "authorization",
            Why::MethodOverride => "method-override",
            Why::TooLarge => "too-large",
            Why::Cap => "cap",
            Why::NoToken => "no-token",
            Why::Upstream => "upstream",
            Why::Port => "port",
        }
    }

    /// The HTTP status a refusal for this reason answers with.
    pub fn status(self) -> u16 {
        match self {
            Why::Unreadable | Why::Unparsable | Why::Batch | Why::Persisted => 400,
            Why::TooLarge => 413,
            Why::Cap => 429,
            Why::NoToken => 503,
            Why::Upstream => 502,
            _ => 403,
        }
    }
}

/// Something refused: why, and the sentence a person reads, which names
/// the route to take instead where there is one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub why: Why,
    pub detail: String,
    /// What was read of the call before it was refused, such as a
    /// GraphQL document's fields, for its line to say in place of the
    /// path.
    pub seen: Option<Seen>,
}

impl Refusal {
    /// A refusal for `why`, said to a person as `detail`.
    pub fn new(why: Why, detail: impl Into<String>) -> Self {
        Refusal {
            why,
            detail: detail.into(),
            seen: None,
        }
    }

    /// The same refusal, its line saying `seen` of the call.
    #[must_use]
    pub fn seeing(mut self, seen: Seen) -> Self {
        self.seen = Some(seen);
        self
    }
}

/// What a line says of a call: where it came in and what it asked.
#[derive(Clone, Debug, Default)]
pub struct Call {
    /// The session whose socket it came in on.
    pub session: String,
    /// The session its process claims.
    pub claimed: String,
    pub method: String,
    pub host: String,
    /// The path, without its query, or the GraphQL fields.
    pub seen: Seen,
}

/// What of a call's target a line says.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Seen {
    /// Nothing could be read.
    #[default]
    Nothing,
    /// A REST or git path, without its query.
    Path(String),
    /// A GraphQL operation's kind and its top-level fields.
    Fields {
        kind: &'static str,
        names: Vec<String>,
    },
}

impl Seen {
    /// The field a line carries: `path=...` or `fields=kind:a,b`.
    fn field(&self) -> String {
        match self {
            Seen::Nothing => "path=none".to_owned(),
            Seen::Path(path) => format!("path={}", plain(path)),
            Seen::Fields { kind, names } => {
                let names: Vec<String> = names.iter().map(|name| plain(name)).collect();
                format!("fields={kind}:{}", names.join(","))
            }
        }
    }
}

/// `text` as a field may carry it: every byte that could end the field or
/// forge another, a space, a control character, `%` or `=`, written as
/// `%XX`, so a client can't write a field of its own into a line.
pub fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_graphic() && byte != b'%' && byte != b'=' {
            out.push(char::from(byte));
        } else {
            // Writing to a String can't fail.
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

impl Call {
    /// The fields every line about this call carries.
    fn fields(&self, status: u16) -> String {
        format!(
            "session={} claimed-session={} method={} host={} {} status={status}",
            plain(&self.session),
            plain(&self.claimed),
            plain(&self.method),
            plain(&self.host),
            self.seen.field()
        )
    }

    /// The line for this call passed on, and GitHub answering `status`.
    pub fn passed_line(&self, status: u16) -> String {
        format!("pi-github-door: passed {}", self.fields(status))
    }

    /// The line for this call refused, saying what the refusal read of
    /// it when that's more than the call's own.
    pub fn refused_line(&self, refusal: &Refusal) -> String {
        let fields = match &refusal.seen {
            Some(seen) => Call {
                seen: seen.clone(),
                ..self.clone()
            }
            .fields(refusal.why.status()),
            None => self.fields(refusal.why.status()),
        };
        format!(
            "pi-github-door: refused why={} {fields} -- {}",
            refusal.why.word(),
            refusal.detail
        )
    }
}

/// The warning that a session has made `calls` calls this hour, past what
/// any session's work needs. It's a warning (`<4>`, as the journal reads
/// stderr), and the door carries on.
pub fn many_calls_line(session: &str, claimed: &str, calls: u32) -> String {
    format!(
        "<4>pi-github-door: many-calls session={} claimed-session={} calls={calls} -- \
         {calls} calls to GitHub this hour from one session, past what any session's work \
         needs; it carries on",
        plain(session),
        plain(claimed)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(seen: Seen) -> Call {
        Call {
            session: "s1".to_owned(),
            claimed: "s1".to_owned(),
            method: "GET".to_owned(),
            host: "api.github.com".to_owned(),
            seen,
        }
    }

    #[test]
    fn a_passed_call_says_its_fields_in_order() {
        let line = call(Seen::Path("/user".to_owned())).passed_line(200);
        assert_eq!(
            line,
            "pi-github-door: passed session=s1 claimed-session=s1 method=GET \
             host=api.github.com path=/user status=200"
        );
    }

    #[test]
    fn a_refusal_says_its_word_its_status_then_why() {
        let refusal = Refusal::new(Why::Merge, "merge with review_offer merge");
        let line = call(Seen::Fields {
            kind: "mutation",
            names: vec!["mergePullRequest".to_owned()],
        })
        .refused_line(&refusal);
        assert_eq!(
            line,
            "pi-github-door: refused why=merge session=s1 claimed-session=s1 method=GET \
             host=api.github.com fields=mutation:mergePullRequest status=403 -- \
             merge with review_offer merge"
        );
    }

    #[test]
    fn a_path_cannot_forge_a_field() {
        let line = call(Seen::Path("/a b status=200\nx".to_owned())).passed_line(404);
        assert!(
            line.contains("path=/a%20b%20status%3D200%0Ax status=404"),
            "{line}"
        );
        assert_eq!(line.matches("status=").count(), 1, "{line}");
    }
}
