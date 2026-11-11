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

use signature::Verifier;
use ssh_key::public::KeyData;
use ssh_key::{HashAlg, PublicKey, Signature};

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
    pub fn bind(&mut self, hosts: &Hosts, fields: &mut Fields) -> Result<(), String> {
        let host = fields.string()?;
        let session = fields.string()?;
        let signed = fields.string()?;
        let forwarding = fields.boolean()?;
        fields.done()?;
        if forwarding {
            self.forwarded = true;
            return Err("a session reached through a forwarded agent".to_owned());
        }
        let host = PublicKey::from_bytes(host)
            .map_err(|error| format!("a host key it can't read: {error}"))?;
        if !hosts.allows(host.key_data()) {
            return Err(format!(
                "a session with a host it isn't told of ({})",
                host.fingerprint(HashAlg::Sha256)
            ));
        }
        let signed = Signature::try_from(signed)
            .map_err(|error| format!("a host signature it can't read: {error}"))?;
        host.key_data().verify(session, &signed).map_err(|_| {
            format!(
                "a session its host ({}) didn't sign",
                host.fingerprint(HashAlg::Sha256)
            )
        })?;
        self.bound.push(Bound {
            session: session.to_vec(),
            host: host.key_data().clone(),
        });
        Ok(())
    }

    /// Whether `data` is a login, with the key `key`, to a session bound
    /// with a host this agent is told of; why not, when it isn't.
    pub fn permits(&self, key: &[u8], data: &[u8]) -> Result<(), String> {
        if self.forwarded {
            return Err("its client reached it through a forwarded agent".to_owned());
        }
        let mut fields = Fields::new(data);
        let session = fields.string().map_err(|_| "it isn't a login".to_owned())?;
        let bound = self
            .bound
            .iter()
            .find(|one| one.session == session)
            .ok_or_else(|| {
                "it isn't a login to a session bound with a host it's told of".to_owned()
            })?;
        if fields.byte()? != USERAUTH_REQUEST {
            return Err("it isn't a login".to_owned());
        }
        let _user = fields.string()?;
        if fields.string()? != b"ssh-connection" {
            return Err("it logs in for something other than a connection".to_owned());
        }
        let hostbound = match fields.string()? {
            b"publickey" => false,
            b"publickey-hostbound-v00@openssh.com" => true,
            _ => return Err("it isn't a public key login".to_owned()),
        };
        if !fields.boolean()? {
            return Err("it asks for no signature".to_owned());
        }
        let _algorithm = fields.string()?;
        if fields.string()? != key {
            return Err("it names another key than the one asked to sign".to_owned());
        }
        if hostbound {
            let named = PublicKey::from_bytes(fields.string()?)
                .map_err(|error| format!("a host key it can't read: {error}"))?;
            if *named.key_data() != bound.host {
                return Err("it names another host than the session's".to_owned());
            }
        }
        fields.done()
    }
}
