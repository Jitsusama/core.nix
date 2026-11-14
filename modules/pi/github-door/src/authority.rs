//! Each session's own certificate authority, which the door answers as
//! GitHub with.
//!
//! A fenced command trusts it only because the fence's environment names
//! a bundle holding it, so nothing outside the fence ever trusts it, and
//! no session trusts another's. Its key is made in memory when the
//! session opens and is never written anywhere, so a door that restarts
//! makes each session a new one. Its name constraints permit only
//! GitHub's four hosts: a name under one of them is permitted too, since
//! that's how a DNS name constraint reads (RFC 5280, 4.2.1.10), but the
//! door only ever signs the four. IP addresses are excluded outright.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rcgen::{
    BasicConstraints, CertificateParams, CidrSubnet, DnType, ExtendedKeyUsagePurpose,
    GeneralSubtree, IsCa, Issuer, KeyPair, KeyUsagePurpose, NameConstraints,
};
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

use crate::judge::Host;

/// How long its certificates hold: under the 398 days clients take a
/// server's certificate for, and longer than a door runs between switches.
const VALID: Duration = Duration::from_hours(397 * 24);
/// How far back they hold from, so a clock a little behind still takes
/// them.
const SLACK: Duration = Duration::from_hours(1);

/// A session's authority: its key, its certificate, and the answer it
/// makes as each of GitHub's hosts, made once each is first asked for.
pub struct Authority {
    issuer: Issuer<'static, KeyPair>,
    pem: String,
    answers: Mutex<HashMap<&'static str, Arc<ServerConfig>>>,
}

/// When, as rcgen reads a time, `offset` from now.
fn at(now: SystemTime, later: bool, offset: Duration) -> Result<time::OffsetDateTime, String> {
    let when = if later { now + offset } else { now - offset };
    let seconds = when
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "the clock is before 1970")?
        .as_secs();
    time::OffsetDateTime::from_unix_timestamp(
        i64::try_from(seconds).map_err(|_| "a time too far off")?,
    )
    .map_err(|error| error.to_string())
}

impl Authority {
    /// A new authority, its key made now, for the session named `session`.
    pub fn new(session: &str) -> Result<Self, String> {
        let now = SystemTime::now();
        let key = KeyPair::generate().map_err(|error| error.to_string())?;
        let mut params =
            CertificateParams::new(Vec::<String>::new()).map_err(|error| error.to_string())?;
        params.distinguished_name.push(
            DnType::CommonName,
            format!("pi-github-door session {session}"),
        );
        params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        params.name_constraints = Some(NameConstraints {
            permitted_subtrees: Host::ALL
                .iter()
                .map(|host| GeneralSubtree::DnsName(host.name().to_owned()))
                .collect(),
            excluded_subtrees: vec![
                GeneralSubtree::IpAddress(CidrSubnet::V4([0; 4], [0; 4])),
                GeneralSubtree::IpAddress(CidrSubnet::V6([0; 16], [0; 16])),
            ],
        });
        params.not_before = at(now, false, SLACK)?;
        params.not_after = at(now, true, VALID)?;
        let certificate = params
            .self_signed(&key)
            .map_err(|error| error.to_string())?;
        Ok(Authority {
            pem: certificate.pem(),
            issuer: Issuer::new(params, key),
            answers: Mutex::new(HashMap::new()),
        })
    }

    /// Its certificate, as PEM, for the session's bundle.
    pub fn pem(&self) -> &str {
        &self.pem
    }

    /// How the door answers a TLS client as `host`: a certificate for it
    /// alone, signed by this authority, speaking HTTP/1.1 only, so every
    /// request comes one at a time in a form the door reads.
    pub fn answer_as(&self, host: Host) -> Result<Arc<ServerConfig>, String> {
        let mut answers = self
            .answers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(config) = answers.get(host.name()) {
            return Ok(Arc::clone(config));
        }
        let now = SystemTime::now();
        let key = KeyPair::generate().map_err(|error| error.to_string())?;
        let mut params = CertificateParams::new(vec![host.name().to_owned()])
            .map_err(|error| error.to_string())?;
        params
            .distinguished_name
            .push(DnType::CommonName, host.name());
        params.is_ca = IsCa::ExplicitNoCa;
        params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        params.not_before = at(now, false, SLACK)?;
        params.not_after = at(now, true, VALID)?;
        let certificate = params
            .signed_by(&key, &self.issuer)
            .map_err(|error| error.to_string())?;
        let chain: Vec<CertificateDer<'static>> = vec![certificate.der().clone()];
        let private = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der()));
        let mut config =
            ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .map_err(|error| error.to_string())?
                .with_no_client_auth()
                .with_single_cert(chain, private)
                .map_err(|error| error.to_string())?;
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        let config = Arc::new(config);
        answers.insert(host.name(), Arc::clone(&config));
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_session_has_an_authority_of_its_own() {
        let one = Authority::new("one").expect("made");
        let two = Authority::new("two").expect("made");
        assert!(
            one.pem().starts_with("-----BEGIN CERTIFICATE-----"),
            "the authority's certificate is said as PEM"
        );
        assert_ne!(one.pem(), two.pem());
    }

    #[test]
    fn an_answer_is_made_once_a_host() {
        let authority = Authority::new("one").expect("made");
        let first = authority.answer_as(Host::Api).expect("answer");
        let again = authority.answer_as(Host::Api).expect("answer");
        assert!(
            Arc::ptr_eq(&first, &again),
            "an answer for a host is made once and kept"
        );
        assert_eq!(first.alpn_protocols, vec![b"http/1.1".to_vec()]);
    }
}
