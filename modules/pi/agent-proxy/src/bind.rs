//! Which hosts a client's sessions are with, and whether a signature it
//! asks for is a login to one of the hosts this agent is told of.
//!
//! OpenSSH's ssh tells its agent, once each connection's key exchange is
//! done, which host it reached and that session's identifier, with the
//! host's own signature over the identifier (`session-bind@openssh.com`
//! in OpenSSH's PROTOCOL.agent). A login it then asks the agent to sign
//! begins with that identifier. So a signature is given only for a login
//! whose session a host this agent is told of signed: a command can't
//! name an allowed host for a session with another, since it can't make
//! that host's signature over the other's identifier.
//!
//! A host is one this agent is told of when `--hosts` names it, or when
//! a machines file names it as a builder lent now (src/lent.rs), read
//! afresh at each binding; a session with a lent builder is marked so,
//! since only the builders' key signs for one, and only as their account.

use signature::Verifier;
use ssh_key::public::KeyData;
use ssh_key::{PublicKey, Signature};

use crate::lent::Lent;
use crate::said::{Refusal, Why};
use crate::wire::Fields;

/// A login request, as SSH's user authentication numbers it.
const USERAUTH_REQUEST: u8 = 50;

/// The host keys this agent signs logins to.
pub struct Hosts(Vec<KeyData>);

impl Hosts {
    /// The host keys in `text`, one OpenSSH public key a line; blank
    /// lines and those starting with `#` say nothing.
    pub fn read(text: &str) -> Result<Self, String> {
        let mut keys = Vec::new();
        for (at, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let key = PublicKey::from_openssh(line)
                .map_err(|error| format!("line {} isn't a public key: {error}", at + 1))?;
            keys.push(key.key_data().clone());
        }
        Ok(Hosts(keys))
    }

    fn allows(&self, host: &KeyData) -> bool {
        self.0.contains(host)
    }
}

/// A session a client's ssh bound, with a host this agent is told of.
struct Bound {
    session: Vec<u8>,
    host: KeyData,
    /// Whether the host is a builder lent to the machine (src/lent.rs)
    /// rather than one of the hosts in `--hosts`.
    lent: bool,
}

/// A login the proxy may sign: the host of the session it's for, whether
/// that host is a lent builder, and the account it logs in as.
pub struct Permit<'a> {
    pub host: &'a KeyData,
    pub lent: bool,
    pub user: Vec<u8>,
}

/// What one client's ssh has said of the sessions it holds.
#[derive(Default)]
pub struct Bindings {
    bound: Vec<Bound>,
    /// Set once a binding says the client reached this agent through a
    /// forwarded one, which it never signs for.
    forwarded: bool,
}

impl Bindings {
    /// Take a `session-bind@openssh.com` extension's fields, keeping the
    /// session when its host is one in `hosts` and that host signed it.
    pub fn bind(&mut self, hosts: &Hosts, lent: &Lent, fields: &mut Fields) -> Result<(), Refusal> {
        let host = fields.string()?;
        let session = fields.string()?;
        let signed = fields.string()?;
        let forwarding = fields.boolean()?;
        fields.done()?;
        if forwarding {
            self.forwarded = true;
            return Err(Refusal::new(
                Why::Forwarded,
                "a session reached through a forwarded agent",
            ));
        }
        let host = PublicKey::from_bytes(host)
            .map_err(|error| format!("a host key it can't read: {error}"))?;
        let told = hosts.allows(host.key_data());
        let lends = !told && lent.lends(host.key_data());
        if !told && !lends {
            return Err(
                Refusal::new(Why::UntoldHost, "a session with a host it isn't told of")
                    .at(host.key_data()),
            );
        }
        let signed = Signature::try_from(signed).map_err(|error| {
            Refusal::from(format!("a host signature it can't read: {error}")).at(host.key_data())
        })?;
        host.key_data().verify(session, &signed).map_err(|_| {
            Refusal::new(Why::Unsigned, "a session its host didn't sign").at(host.key_data())
        })?;
        self.bound.push(Bound {
            session: session.to_vec(),
            host: host.key_data().clone(),
            lent: lends,
        });
        Ok(())
    }

    /// Whether `data` is a login, with the key `key`, to a session bound
    /// with a host this agent is told of: that session's host when it is,
    /// and why not when it isn't.
    pub fn permits(&self, key: &[u8], data: &[u8]) -> Result<Permit<'_>, Refusal> {
        if self.forwarded {
            return Err(Refusal::new(
                Why::Forwarded,
                "its client reached it through a forwarded agent",
            ));
        }
        let mut fields = Fields::new(data);
        let session = fields
            .string()
            .map_err(|_| Refusal::new(Why::NotLogin, "it isn't a login"))?;
        let bound = self
            .bound
            .iter()
            .find(|one| one.session == session)
            .ok_or_else(|| {
                Refusal::new(
                    Why::NoSession,
                    "it isn't a login to a session bound with a host it's told of",
                )
            })?;
        let user =
            login(&mut fields, key, &bound.host).map_err(|refusal| refusal.at(&bound.host))?;
        Ok(Permit {
            host: &bound.host,
            lent: bound.lent,
            user,
        })
    }
}

/// Whether the rest of `fields`, after a login's session, logs in with
/// `key` to `host`: the account it logs in as when it does, and why not
/// when it doesn't.
fn login(fields: &mut Fields, key: &[u8], host: &KeyData) -> Result<Vec<u8>, Refusal> {
    let not_login = |detail: &str| Refusal::new(Why::NotLogin, detail);
    if fields.byte()? != USERAUTH_REQUEST {
        return Err(not_login("it isn't a login"));
    }
    let user = fields.string()?.to_vec();
    if fields.string()? != b"ssh-connection" {
        return Err(not_login(
            "it logs in for something other than a connection",
        ));
    }
    let hostbound = match fields.string()? {
        b"publickey" => false,
        b"publickey-hostbound-v00@openssh.com" => true,
        _ => return Err(not_login("it isn't a public key login")),
    };
    if !fields.boolean()? {
        return Err(not_login("it asks for no signature"));
    }
    let _algorithm = fields.string()?;
    if fields.string()? != key {
        return Err(Refusal::new(
            Why::Mismatch,
            "it names another key than the one asked to sign",
        ));
    }
    if hostbound {
        let named = PublicKey::from_bytes(fields.string()?)
            .map_err(|error| format!("a host key it can't read: {error}"))?;
        if named.key_data() != host {
            return Err(Refusal::new(
                Why::Mismatch,
                "it names another host than the session's",
            ));
        }
    }
    fields.done()?;
    Ok(user)
}
