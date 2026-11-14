//! What the proxy says in the journal of each login it signs and each
//! thing it refuses, one line apiece in a shape that never changes.
//!
//! A reader of the journal, such as a health daemon watching for an
//! agent probing the fence, keeps only fields, never a log's free text:
//! free text can carry whatever a client sent. So each line opens with
//! fixed fields, what happened, why as one word from a fixed set, the
//! host asked for as its key's fingerprint and the session the client
//! claims (src/peer.rs), and only then, after ` -- `, the sentence a
//! person reads. A login signed also says which key signed it, the
//! builders' or an agent's behind, and the account it logs in as:
//!
//! ```text
//! pi-agent-proxy: signed host=SHA256:... key=builders user=nix-offload claimed-session=01a1...
//! pi-agent-proxy: refused why=untold-host host=SHA256:... claimed-session=unknown -- a ...
//! pi-agent-proxy: many-signs claimed-session=01a1... signs=6000 -- signed 6000 logins this hour ...
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
    /// A login with the builders' key as another account than theirs.
    Account,
    /// A login with the builders' key to a host that isn't a lent builder.
    NotLent,
    /// A login to a lent builder with a key other than the builders'.
    LentHost,
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
            Why::Account => "account",
            Why::NotLent => "not-lent",
            Why::LentHost => "lent-host",
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

    /// Its line in the journal, for a client claiming the session
    /// `claimed`.
    pub fn line(&self, claimed: &str) -> String {
        format!(
            "pi-agent-proxy: refused why={} host={} claimed-session={claimed} -- {}",
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

/// Which key signed a login: the builders', which the proxy holds, or
/// one an agent behind it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signer {
    Builders,
    Agent,
}

/// What a line says of the account a login is as: itself when it's
/// plain, so a client can't write a field of its own into the line.
fn account(user: &[u8]) -> String {
    let plain = !user.is_empty()
        && user.len() <= 64
        && user
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(byte));
    if plain {
        String::from_utf8_lossy(user).into_owned()
    } else {
        "unreadable".to_owned()
    }
}

/// The line for a login it signed with `signer`'s key as `user`, to a
/// session with `host`, for a client claiming the session `claimed`.
pub fn signed_line(host: &KeyData, signer: Signer, user: &[u8], claimed: &str) -> String {
    let key = match signer {
        Signer::Builders => "builders",
        Signer::Agent => "agent",
    };
    format!(
        "pi-agent-proxy: signed host={} key={key} user={} claimed-session={claimed}",
        fingerprint(host),
        account(user)
    )
}

/// The line for a client claiming the session `claimed` that has had
/// `signs` logins signed this hour, past what any session's work needs.
/// It's a warning (`<4>`, as the journal reads stderr), and the proxy
/// signs on: a builder login refused would break a run, and the claim
/// is only a claim.
pub fn many_signs_line(claimed: &str, signs: u32) -> String {
    format!(
        "<4>pi-agent-proxy: many-signs claimed-session={claimed} signs={signs} -- signed {signs} logins this hour for one session, past what any session's work needs; it signs on"
    )
}

/// `host`'s fingerprint, as ssh shows it: `SHA256:` and its base64.
fn fingerprint(host: &KeyData) -> String {
    host.fingerprint(HashAlg::Sha256).to_string()
}
