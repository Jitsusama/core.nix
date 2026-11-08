//! The App: who it is, its key, and the one installation it mints for.

use std::fs;
use std::path::Path;
use std::process::Command;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rsa::RsaPrivateKey;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs1v15::SigningKey;
use rsa::sha2::Sha256;
use rsa::signature::{SignatureEncoding, Signer};

/// A GitHub App, opened, with the installation its tokens are for.
pub struct App {
    id: String,
    key: RsaPrivateKey,
    installation: String,
}

impl App {
    /// An App from its id, its key as GitHub hands it over (PKCS #1 PEM)
    /// and an installation's id.
    pub fn from_parts(id: &str, pem: &str, installation: &str) -> Result<App, String> {
        let key = RsaPrivateKey::from_pkcs1_pem(pem)
            .map_err(|why| format!("the App's key isn't one GitHub made: {why}"))?;
        let installation = installation.trim();
        if installation.is_empty() || !installation.bytes().all(|b| b.is_ascii_digit()) {
            return Err(format!("{installation:?} isn't an installation's id"));
        }
        Ok(App {
            id: id.to_owned(),
            key,
            installation: installation.to_owned(),
        })
    }

    /// The App sealed at `sealed`, as GitHub's manifest conversion answered
    /// (its `id` and `pem`), opened with age and the key at `identity`, for
    /// the installation whose id is in the file `installation`.
    pub fn open(sealed: &Path, installation: &Path, identity: &Path) -> Result<App, String> {
        for (path, what) in [(sealed, "the App"), (installation, "its installation")] {
            if !path.exists() {
                return Err(format!(
                    "{what} isn't made yet: {} is missing",
                    path.display()
                ));
            }
        }
        let opened = Command::new("age")
            .arg("-d")
            .arg("-i")
            .arg(identity)
            .arg(sealed)
            .output()
            .map_err(|why| format!("can't run age: {why}"))?;
        if !opened.status.success() {
            return Err(format!(
                "age couldn't open {}: {}",
                sealed.display(),
                String::from_utf8_lossy(&opened.stderr).trim()
            ));
        }
        let made: serde_json::Value = serde_json::from_slice(&opened.stdout)
            .map_err(|why| format!("{} doesn't hold GitHub's answer: {why}", sealed.display()))?;
        let id = match &made["id"] {
            serde_json::Value::Number(id) => id.to_string(),
            _ => return Err(format!("{} names no App id", sealed.display())),
        };
        let pem = made["pem"]
            .as_str()
            .ok_or_else(|| format!("{} holds no key", sealed.display()))?;
        let installation = fs::read_to_string(installation)
            .map_err(|why| format!("can't read {}: {why}", installation.display()))?;
        App::from_parts(&id, pem, &installation)
    }

    /// The installation its tokens are minted for.
    pub fn installation(&self) -> &str {
        &self.installation
    }

    /// The JSON web token GitHub asks an App to sign, at `now` in seconds
    /// since 1970. It starts a minute early, for a clock that runs behind
    /// GitHub's, and lasts nine minutes, under GitHub's ten.
    pub fn jwt(&self, now: u64) -> String {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","typ":"JWT"}"#);
        let claims = serde_json::json!({
            "iat": now.saturating_sub(60),
            "exp": now + 540,
            "iss": self.id,
        });
        let claims = URL_SAFE_NO_PAD.encode(claims.to_string());
        let signed = format!("{header}.{claims}");
        let signature = SigningKey::<Sha256>::new(self.key.clone()).sign(signed.as_bytes());
        format!("{signed}.{}", URL_SAFE_NO_PAD.encode(signature.to_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::pkcs1::{EncodeRsaPrivateKey, LineEnding};
    use rsa::pkcs1v15::{Signature, VerifyingKey};
    use rsa::signature::Verifier;

    fn key() -> RsaPrivateKey {
        RsaPrivateKey::new(&mut rand::thread_rng(), 2048).unwrap()
    }

    fn pem(key: &RsaPrivateKey) -> String {
        key.to_pkcs1_pem(LineEnding::LF).unwrap().to_string()
    }

    #[test]
    fn signs_a_token_github_can_check() {
        let key = key();
        let app = App::from_parts("42", &pem(&key), "7\n").unwrap();
        let jwt = app.jwt(1_000_000);
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3, "a JWT is three parts: {jwt}");
        let claims: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap();
        assert_eq!(
            claims,
            serde_json::json!({"iat": 999_940, "exp": 1_000_540, "iss": "42"})
        );
        let signature =
            Signature::try_from(URL_SAFE_NO_PAD.decode(parts[2]).unwrap().as_slice()).unwrap();
        VerifyingKey::<Sha256>::new(key.to_public_key())
            .verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &signature)
            .expect("the signature is the App key's");
        assert_eq!(app.installation(), "7");
    }

    #[test]
    fn refuses_what_isnt_an_installation() {
        let refused = App::from_parts("42", &pem(&key()), "7; rm -rf")
            .err()
            .unwrap();
        assert!(refused.contains("isn't an installation's id"), "{refused}");
    }

    #[test]
    fn says_which_file_is_missing_before_the_app_is_made() {
        let folder = tempfile::tempdir().unwrap();
        let sealed = folder.path().join("app.json.age");
        let refused = App::open(&sealed, &folder.path().join("id"), &folder.path().join("k"))
            .err()
            .unwrap();
        assert_eq!(
            refused,
            format!("the App isn't made yet: {} is missing", sealed.display())
        );
    }
}
