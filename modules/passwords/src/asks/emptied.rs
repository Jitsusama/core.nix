//! The files the fence's list empties only where they hold a credential,
//! such as Docker's settings, whose `auths` keep registries' logins and
//! whose `credsStore` names the helper answering every registry's from
//! Joel's keychain (ISXEG8, 16 November). The list names each file and
//! the settings that make it hold one, so the harness's fence and this
//! daemon's read the same words and hide the same files. The rules for
//! each setting follow `fence-docker.ts`'s, and go past them where
//! Docker's reader does: a setting under any spelling, and a proxy's
//! login with no scheme before it.

use serde::Deserialize;
use serde_json::{Map, Value};

/// A file the list empties where it holds a credential: its path from
/// home, and the top-level settings that make it hold one.
#[derive(Deserialize)]
pub struct Emptied {
    pub path: String,
    pub when: Vec<String>,
}

/// What a login in `auths` carries when it carries one.
const LOGIN_FIELDS: [&str; 4] = ["auth", "identitytoken", "password", "registrytoken"];

/// Whether settings reading `text` hold a credential by any setting
/// `when` names, under any spelling of it, since Docker's own reader
/// takes every spelling. Settings that don't parse, or aren't an
/// object, are held to hold one, since what they hold can't be told.
pub fn empties(text: &str, when: &[String]) -> bool {
    let Ok(Value::Object(settings)) = serde_json::from_str::<Value>(text) else {
        return true;
    };
    when.iter()
        .any(|name| spelled(&settings, name).any(|value| holds(name, value)))
}

/// Whether one setting, by its name, holds a credential: a login in
/// `auths`, an address in `proxies` carrying a name and password, and
/// any other named setting by being there at all.
fn holds(name: &str, value: &Value) -> bool {
    match name.to_lowercase().as_str() {
        "auths" => value.as_object().is_some_and(|logins| {
            logins.values().any(|login| {
                login.as_object().is_some_and(|login| {
                    LOGIN_FIELDS.iter().any(|field_name| {
                        spelled(login, field_name)
                            .any(|said| matches!(said, Value::String(said) if !said.is_empty()))
                    })
                })
            })
        }),
        "proxies" => addresses(value).any(carries_login),
        _ => !is_empty(value),
    }
}

/// Every string a setting holds, however deep.
fn addresses(value: &Value) -> Box<dyn Iterator<Item = &str> + '_> {
    match value {
        Value::String(said) => Box::new(std::iter::once(said.as_str())),
        Value::Array(listed) => Box::new(listed.iter().flat_map(addresses)),
        Value::Object(named) => Box::new(named.values().flat_map(addresses)),
        Value::Null | Value::Bool(_) | Value::Number(_) => Box::new(std::iter::empty()),
    }
}

/// Whether an address carries a name and password of its own: an `@`
/// before its path, with or without a scheme, as a proxy's reader takes
/// `joel:pw@proxy:3128` and `http://joel:pw@proxy:3128` alike.
fn carries_login(address: &str) -> bool {
    let past_scheme = address.split_once("://").map_or(address, |(_, rest)| rest);
    past_scheme
        .split('/')
        .next()
        .is_some_and(|host| host.contains('@'))
}

/// Whether a setting says nothing: null, or an empty string, list or
/// object.
fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(said) => said.is_empty(),
        Value::Array(listed) => listed.is_empty(),
        Value::Object(named) => named.is_empty(),
        Value::Bool(_) | Value::Number(_) => false,
    }
}

/// A setting under each of its spellings.
fn spelled<'a>(settings: &'a Map<String, Value>, name: &'a str) -> impl Iterator<Item = &'a Value> {
    settings
        .iter()
        .filter(move |(key, _)| key.eq_ignore_ascii_case(name))
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
    fn registries_named_without_a_login_are_left() {
        for text in [
            r#"{"auths": {"registry.example": {}}}"#,
            r#"{"auths": {"registry.example": {"auth": ""}}}"#,
            r#"{"auths": {"registry.example": {"email": "joel@example"}}}"#,
            r#"{"auths": {}, "credsStore": "", "HttpHeaders": {}, "detachKeys": "ctrl-p"}"#,
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
    fn a_proxys_login_with_no_scheme_empties_the_file() {
        let text = r#"{"proxies": {"default": {"httpProxy": "joel:made-up@proxy.example:3128"}}}"#;
        assert!(
            empties(text, &docker()),
            "a proxy's login with no scheme was left: {text}"
        );
    }

    #[test]
    fn an_at_past_a_proxys_address_is_no_login() {
        for text in [
            r#"{"proxies": {"default": {"httpProxy": "http://proxy.example:3128/at@path"}}}"#,
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
