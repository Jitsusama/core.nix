//! What the proxy says in the journal of each login it signs and each
//! thing it refuses, one line apiece in a shape that never changes.
//!
//! A reader of the journal, such as a health daemon watching for an
//! agent probing the fence, keeps only fields, never a log's free text:
//! free text can carry whatever a client sent. So each line opens with
//! fixed fields, what happened, why as one word from a fixed set, and
//! the host asked for as its key's fingerprint, and only then, after
//! ` -- `, the sentence a person reads:
//!
//! ```text
//! pi-agent-proxy: signed host=SHA256:...
//! pi-agent-proxy: refused why=untold-host host=SHA256:... -- a session with a host ...
//! pi-agent-proxy: refused why=no-session host=none -- it isn't a login to a session ...
//! ```

use ssh_key::HashAlg;
use ssh_key::public::KeyData;

/// Why something was refused, as the one word its line carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// What the client sent couldn't be read.
    Unreadable,
    /// A kind of request it never answers: adding, removing or locking.
    Request,
    /// An extension other than a session binding.
    Extension,
    /// A session reached through a forwarded agent.
    Forwarded,
    /// A session with a host it isn't told of.
    UntoldHost,
    /// A session its host didn't sign.
    Unsigned,
    /// A signature for no session a host it's told of bound.
    NoSession,
    /// A signature for something other than a public key login.
    NotLogin,
    /// A login naming another key, or another host, than the one asked.
    Mismatch,
    /// A key no agent behind it holds.
    NoHolder,
    /// The agent behind couldn't be asked, or declined.
    Upstream,
    /// A client past the most it answers at once.
    Busy,
}

impl Why {
    /// The word a line carries for this reason.
    pub fn word(self) -> &'static str {
        match self {
            Why::Unreadable => "unreadable",
            Why::Request => "request",
            Why::Extension => "extension",
            Why::Forwarded => "forwarded",
            Why::UntoldHost => "untold-host",
            Why::Unsigned => "unsigned",
            Why::NoSession => "no-session",
            Why::NotLogin => "not-login",
            Why::Mismatch => "mismatch",
            Why::NoHolder => "no-holder",
            Why::Upstream => "upstream",
            Why::Busy => "busy",
        }
    }
}

/// Something refused: why, the host it was for when that's known, and
/// the sentence a person reads.
#[derive(Debug)]
pub struct Refusal {
    why: Why,
    host: Option<String>,
    detail: String,
}

impl Refusal {
    /// A refusal for `why`, said to a person as `detail`.
    pub fn new(why: Why, detail: impl Into<String>) -> Self {
        Refusal {
            why,
            host: None,
            detail: detail.into(),
        }
    }

    /// The same refusal, for a session with `host`.
    #[must_use]
    pub fn at(mut self, host: &KeyData) -> Self {
        self.host = Some(fingerprint(host));
        self
    }

    /// Its line in the journal.
    pub fn line(&self) -> String {
        format!(
            "pi-agent-proxy: refused why={} host={} -- {}",
            self.why.word(),
            self.host.as_deref().unwrap_or("none"),
            self.detail
        )
    }
}

/// What a message's fields failing to read says: that it's unreadable.
impl From<String> for Refusal {
    fn from(detail: String) -> Self {
        Refusal::new(Why::Unreadable, detail)
    }
}

/// The line for a login it signed, to a session with `host`.
pub fn signed_line(host: &KeyData) -> String {
    format!("pi-agent-proxy: signed host={}", fingerprint(host))
}

/// `host`'s fingerprint, as ssh shows it: `SHA256:` and its base64.
fn fingerprint(host: &KeyData) -> String {
    host.fingerprint(HashAlg::Sha256).to_string()
}
