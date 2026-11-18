//! The files the fence's list empties only where they hold a credential,
//! such as Docker's settings, whose `auths` keep registries' logins and
//! whose `credsStore` names the helper answering every registry's from
//! Joel's keychain (ISXEG8, 16 November). The list names each file and
//! the settings that make it hold one, so the harness's fence and this
//! daemon's read the same words and hide the same files. The rules for
//! each setting follow `fence-docker.ts`'s, and go past them where
//! Docker's reader does: a setting under any spelling it folds to, a key
//! said twice, and a proxy's login with no scheme before it or a slash in
//! its password.

use std::fmt;

use serde::Deserialize;
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};

/// A file the list empties where it holds a credential: its path from
/// home, and the top-level settings that make it hold one.
#[derive(Deserialize)]
pub struct Emptied {
    pub path: String,
    pub when: Vec<String>,
}

/// What a login in `auths` carries when it carries one.
const LOGIN_FIELDS: [&str; 4] = ["auth", "identitytoken", "password", "registrytoken"];

/// Settings as they're written: every key kept, in its order, and a key
/// said twice kept twice, since cat reads every saying where a map of
/// `serde_json`'s keeps only the last.
enum Said {
    Nothing,
    Something,
    Text(String),
    Listed(Vec<Said>),
    Named(Vec<(String, Said)>),
}

impl<'de> Deserialize<'de> for Said {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(Reading)
    }
}

/// Reads JSON into what it says, keeping every key.
struct Reading;

impl<'de> Visitor<'de> for Reading {
    type Value = Said;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("settings in JSON")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Said, E> {
        Ok(Said::Nothing)
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<Said, E> {
        Ok(Said::Something)
    }

    fn visit_i64<E: de::Error>(self, _: i64) -> Result<Said, E> {
        Ok(Said::Something)
    }

    fn visit_u64<E: de::Error>(self, _: u64) -> Result<Said, E> {
        Ok(Said::Something)
    }

    fn visit_f64<E: de::Error>(self, _: f64) -> Result<Said, E> {
        Ok(Said::Something)
    }

    fn visit_str<E: de::Error>(self, said: &str) -> Result<Said, E> {
        Ok(Said::Text(said.to_owned()))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Said, A::Error> {
        let mut listed = Vec::new();
        while let Some(one) = seq.next_element()? {
            listed.push(one);
        }
        Ok(Said::Listed(listed))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Said, A::Error> {
        let mut named = Vec::new();
        while let Some(entry) = map.next_entry::<String, Said>()? {
            named.push(entry);
        }
        Ok(Said::Named(named))
    }
}

/// Whether settings reading `text` hold a credential by any setting
/// `when` names, under any spelling of it and any saying of it, since
/// Docker's own reader takes every spelling and merges every saying.
/// Settings that don't parse, or aren't an object, are held to hold one,
/// since what they hold can't be told.
pub fn empties(text: &str, when: &[String]) -> bool {
    let Ok(Said::Named(settings)) = serde_json::from_str::<Said>(text) else {
        return true;
    };
    when.iter()
        .any(|name| spelled(&settings, name).any(|value| holds(name, value)))
}

/// Whether one setting, by its name, holds a credential: a login in
/// `auths`, an address in `proxies` carrying a name and password, and
/// any other named setting by being there at all. What `auths` says in a
/// shape Docker can't read counts by being there, and a registry's name
/// or the address older logins wrote beside it carrying a login counts as
/// a proxy's does, since cat reads them all.
fn holds(name: &str, value: &Said) -> bool {
    match folded(name).as_str() {
        "auths" => {
            let Said::Named(logins) = value else {
                return !is_empty(value);
            };
            logins.iter().any(|(registry, login)| {
                if carries_login(registry) {
                    return true;
                }
                let Said::Named(login) = login else {
                    return !is_empty(login);
                };
                LOGIN_FIELDS
                    .iter()
                    .any(|field_name| spelled(login, field_name).any(|said| !is_empty(said)))
                    || spelled(login, "serveraddress")
                        .any(|said| matches!(said, Said::Text(address) if carries_login(address)))
            })
        }
        "proxies" => addresses(value).any(carries_login),
        _ => !is_empty(value),
    }
}

/// Every string a setting holds, however deep, its names among them,
/// since cat reads those too.
fn addresses(value: &Said) -> Box<dyn Iterator<Item = &str> + '_> {
    match value {
        Said::Text(said) => Box::new(std::iter::once(said.as_str())),
        Said::Listed(listed) => Box::new(listed.iter().flat_map(addresses)),
        Said::Named(named) => Box::new(
            named
                .iter()
                .flat_map(|(name, said)| std::iter::once(name.as_str()).chain(addresses(said))),
        ),
        Said::Nothing | Said::Something => Box::new(std::iter::empty()),
    }
}

/// Whether an address may carry a name and password of its own: an `@`
/// anywhere in it, as a proxy's reader takes `joel:pw@proxy:3128` and
/// `http://joel:pw@proxy:3128` alike. A password may hold a slash, and a
/// schemeless address a later `://`, so an `@` in what looks like a path
/// counts too, which only ever hides settings holding no login.
fn carries_login(address: &str) -> bool {
    address.contains('@')
}

/// Whether a setting says nothing: null, or an empty string, list or
/// object.
fn is_empty(value: &Said) -> bool {
    match value {
        Said::Nothing => true,
        Said::Text(said) => said.is_empty(),
        Said::Listed(listed) => listed.is_empty(),
        Said::Named(named) => named.is_empty(),
        Said::Something => false,
    }
}

/// A key as Docker's reader matches it, folded as Go folds: past case,
/// the long s is an s, the Kelvin sign a k and the dotless i an i. The
/// dotted capital I is folded to i too, by its one-letter lower case,
/// where Rust's full lowering makes it two; Go may not match it, and
/// folding more than Go does only ever hides more.
fn folded(key: &str) -> String {
    key.chars()
        .filter_map(|letter| letter.to_lowercase().next())
        .flat_map(char::to_uppercase)
        .flat_map(char::to_lowercase)
        .collect()
}

/// A setting under each of its spellings and each time it's said.
fn spelled<'a>(settings: &'a [(String, Said)], name: &str) -> impl Iterator<Item = &'a Said> {
    let name = folded(name);
    settings
        .iter()
        .filter(move |(key, _)| folded(key) == name)
        .map(|(_, value)| value)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Docker's settings, as the harness's list names them: not
    /// credHelpers, which only names a helper program.
    fn docker() -> Vec<String> {
        ["auths", "credsStore", "HttpHeaders", "proxies"]
            .map(String::from)
            .to_vec()
    }

    #[test]
    fn a_login_in_auths_empties_the_file_by_each_field_it_may_carry() {
        for login in ["auth", "identitytoken", "password", "registrytoken", "Auth"] {
            let text = format!(r#"{{"auths": {{"registry.example": {{"{login}": "made-up"}}}}}}"#);
            assert!(
                empties(&text, &docker()),
                "a login by {login} was left: {text}"
            );
        }
    }

    #[test]
    fn logins_in_a_shape_docker_cant_read_empty_the_file() {
        // What Docker can't read, cat still can, so a login said in any
        // other shape than Docker's counts, as settings that don't parse do.
        for text in [
            r#"{"auths": ["made-up"]}"#,
            r#"{"auths": {"registry.example": "made-up"}}"#,
            r#"{"auths": {"registry.example": {"auth": {"value": "made-up"}}}}"#,
            r#"{"proxies": ["http://joel:made-up@proxy.example"]}"#,
        ] {
            assert!(
                empties(text, &docker()),
                "a login in a shape Docker can't read was left: {text}"
            );
        }
    }

    #[test]
    fn registries_named_without_a_login_are_left() {
        for text in [
            r#"{"auths": {"registry.example": {}}}"#,
            r#"{"auths": {"registry.example": {"auth": ""}}}"#,
            r#"{"auths": {"registry.example": {"email": "joel@example"}}}"#,
            r#"{"auths": {}, "credsStore": "", "HttpHeaders": {}, "detachKeys": "ctrl-p"}"#,
            r#"{"auths": null}"#,
            r#"{"auths": []}"#,
            r#"{"auths": {"registry.example": null}}"#,
            r#"{"auths": {"registry.example": ""}}"#,
            r#"{"auths": {"registry.example": {"auth": null}}}"#,
            r#"{"proxies": {"default": {"httpProxy": "http://proxy.example:3128"}}}"#,
            r#"{"credHelpers": {"registry.example": "ecr-login"}}"#,
        ] {
            assert!(
                !empties(text, &docker()),
                "settings with no credential were emptied: {text}"
            );
        }
    }

    #[test]
    fn a_helper_headers_or_a_proxys_login_empties_the_file() {
        for text in [
            r#"{"credsStore": "secretservice"}"#,
            r#"{"CREDSSTORE": "secretservice"}"#,
            r#"{"HttpHeaders": {"Authorization": "Bearer made-up"}}"#,
            r#"{"proxies": {"default": {"httpsProxy": "http://joel:made-up@proxy.example"}}}"#,
        ] {
            assert!(empties(text, &docker()), "a credential was left: {text}");
        }
    }

    #[test]
    fn a_credential_under_any_spelling_of_a_setting_empties_the_file() {
        // Docker's reader takes every spelling of a setting, so one empty
        // spelling beside another holding a login hides nothing. Each pair
        // has its credential under both the spelling read first and last.
        for text in [
            r#"{"auths": {}, "AUTHS": {"registry.example": {"auth": "made-up"}}}"#,
            r#"{"AUTHS": {}, "auths": {"registry.example": {"auth": "made-up"}}}"#,
            r#"{"credsStore": "", "CredsStore": "secretservice"}"#,
            r#"{"CredsStore": "", "credsStore": "secretservice"}"#,
            r#"{"auths": {"registry.example": {"auth": "", "Auth": "made-up"}}}"#,
            r#"{"auths": {"registry.example": {"Auth": "", "auth": "made-up"}}}"#,
        ] {
            assert!(
                empties(text, &docker()),
                "a second spelling's credential was left: {text}"
            );
        }
    }

    #[test]
    fn a_credential_under_a_key_said_twice_empties_the_file() {
        // cat reads every saying of a setting said twice, whichever one
        // Docker keeps, so a credential under either counts.
        for text in [
            r#"{"auths": {"registry.example": {"auth": "made-up"}}, "auths": {}}"#,
            r#"{"auths": {"registry.example": {"auth": "made-up"}, "registry.example": {}}}"#,
            r#"{"auths": {"registry.example": {"auth": "made-up", "auth": ""}}}"#,
            r#"{"credsStore": "secretservice", "credsStore": ""}"#,
        ] {
            assert!(
                empties(text, &docker()),
                "a credential said before its key's second saying was left: {text}"
            );
        }
    }

    #[test]
    fn a_credential_under_a_key_docker_folds_empties_the_file() {
        // Docker's reader folds the long s to s, the Kelvin sign to k and
        // the dotless i to i, by its upper case then lower; the dotted
        // capital I is folded to i too, which hides more if Go doesn't.
        for text in [
            r#"{"auths": {"registry.example": {"\u0131dentitytoken": "made-up"}}}"#,
            r#"{"auth\u017f": {"registry.example": {"auth": "made-up"}}}"#,
            r#"{"auths": {"registry.example": {"\u0130dentitytoken": "made-up"}}}"#,
            r#"{"prox\u0130es": {"default": {"httpProxy": "http://joel:made-up@p.example"}}}"#,
            r#"{"cred\u017fStore": "secretservice"}"#,
            r#"{"auths": {"registry.example": {"identityto\u212aen": "made-up"}}}"#,
        ] {
            assert!(
                empties(text, &docker()),
                "a credential under a folded key was left: {text}"
            );
        }
    }

    #[test]
    fn a_proxys_login_with_a_slash_in_its_password_empties_the_file() {
        // An @ anywhere counts as a login, so a password holding a slash
        // is never read as a path; an @ that is a path's only empties a
        // file holding no login, which fails safe.
        for text in [
            r#"{"proxies": {"default": {"httpProxy": "http://joel:made/up@proxy.example"}}}"#,
            r#"{"proxies": {"default": {"httpProxy": "joel:made/up@proxy.example:3128"}}}"#,
        ] {
            assert!(
                empties(text, &docker()),
                "a proxy's login holding a slash was left: {text}"
            );
        }
    }

    #[test]
    fn a_proxys_login_with_no_scheme_empties_the_file() {
        for text in [
            r#"{"proxies": {"default": {"httpProxy": "joel:made-up@proxy.example:3128"}}}"#,
            r#"{"proxies": {"default": {"httpProxy": "joel:made-up@proxy.example/a://b"}}}"#,
        ] {
            assert!(
                empties(text, &docker()),
                "a proxy's login with no scheme was left: {text}"
            );
        }
    }

    #[test]
    fn a_proxys_login_said_as_a_name_empties_the_file() {
        // Docker reads no login there, but cat reads every name as well.
        let text = r#"{"proxies": {"http://joel:made-up@proxy.example": {}}}"#;
        assert!(
            empties(text, &docker()),
            "a proxy's login said as a name was left: {text}"
        );
    }

    #[test]
    fn a_login_said_in_a_registrys_name_empties_the_file() {
        // Docker reads no login from a registry's name, or from the address
        // older logins wrote beside it, but cat reads both, as it reads a
        // proxy's.
        for text in [
            r#"{"auths": {"https://joel:made-up@registry.example": {}}}"#,
            r#"{"auths": {"joel:made-up@registry.example": {"email": "joel@example"}}}"#,
            r#"{"auths": {"registry.example": {"serveraddress": "https://joel:made-up@r.example"}}}"#,
            r#"{"auths": {"registry.example": {"ServerAddress": "joel:made-up@r.example"}}}"#,
        ] {
            assert!(
                empties(text, &docker()),
                "a login said in a registry's name was left: {text}"
            );
        }
    }

    #[test]
    fn a_proxy_with_no_at_is_no_login() {
        for text in [
            r#"{"proxies": {"default": {"httpProxy": "http://proxy.example:3128/path"}}}"#,
            r#"{"proxies": {"default": {"noProxy": "*.example,localhost"}}}"#,
        ] {
            assert!(
                !empties(text, &docker()),
                "a proxy with no login was emptied: {text}"
            );
        }
    }

    #[test]
    fn settings_that_cant_be_told_are_emptied() {
        for text in ["", "{", "[]", r#""auths""#, "null"] {
            assert!(
                empties(text, &docker()),
                "settings that can't be read were left: {text}"
            );
        }
    }

    #[test]
    fn only_the_settings_the_list_names_count() {
        let text = r#"{"credsStore": "secretservice"}"#;
        assert!(
            !empties(text, &["auths".to_owned()]),
            "an unnamed setting counted: {text}"
        );
    }
}
