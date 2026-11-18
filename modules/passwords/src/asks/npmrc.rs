//! npm's settings, which the fence's list names and which hold a
//! registry's credential beside settings a command needs, so a released
//! command reads a copy with each credential's line held back, as
//! `fence-npmrc.ts` makes one for an agent.

use std::ffi::OsString;

use regex_lite::Regex;

use super::{Fence, Passwords, is_file, real};

/// A setting in npm's settings that is a credential, bare or for one
/// registry, as `fence-npmrc.ts` finds it.
const NPM_CREDENTIAL: &str = r"^\s*((?:[^=;#\s]*:)?_(?:authToken|auth|password))\s*=";

/// A setting whose value is an address carrying a name and password,
/// which npm sends as the registry's credential.
const NPM_USERINFO: &str = r"^\s*([^=;#\s]+)\s*=.*://[^/@\s]+@";

impl Passwords {
    /// bubblewrap's arguments for npm's settings: a copy without their
    /// credentials bound over them, or /dev/null where they hold one and
    /// no copy can be written, so a token is never left readable for want
    /// of a copy.
    pub(super) fn npmrc(&self, fence: &Fence) -> Vec<OsString> {
        let Some(named) = &fence.npmrc else {
            return Vec::new();
        };
        let Some(over) = real(&self.home.join(named)) else {
            return Vec::new();
        };
        if !is_file(&over) {
            return Vec::new();
        }
        let Ok(text) = std::fs::read_to_string(&over) else {
            return vec!["--ro-bind".into(), "/dev/null".into(), over.into()];
        };
        let Some(kept) = without_npm_credentials(&text) else {
            return Vec::new();
        };
        let source: OsString = self.written("npmrc", &kept).map_or_else(
            |why| {
                // npm then reads no settings at all, so the journal says why.
                eprintln!(
                    "<4>passwords: hiding {} with /dev/null, unwritten: {why}",
                    over.display()
                );
                "/dev/null".into()
            },
            Into::into,
        );
        vec!["--ro-bind".into(), source, over.into()]
    }
}

/// npm's settings with each credential's line replaced by a comment
/// naming its setting, never its value, as `fence-npmrc.ts` makes them;
/// or none where they hold no credential and need no copy.
fn without_npm_credentials(text: &str) -> Option<String> {
    let credential = Regex::new(NPM_CREDENTIAL).expect("npm's credential pattern makes");
    let userinfo = Regex::new(NPM_USERINFO).expect("npm's address pattern makes");
    let setting = |line: &str| {
        credential
            .captures(line)
            .or_else(|| userinfo.captures(line))
            .and_then(|found| found.get(1))
            .map(|found| found.as_str().to_owned())
    };
    let lines: Vec<&str> = text.split('\n').collect();
    if !lines.iter().any(|line| setting(line).is_some()) {
        return None;
    }
    let kept: Vec<String> = lines
        .iter()
        .map(|line| {
            setting(line).map_or_else(
                || (*line).to_owned(),
                |setting| format!("; {setting} is held back by the agent fence"),
            )
        })
        .collect();
    Some(kept.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npms_credentials_are_held_back_by_name_and_all_else_kept() {
        let kept = without_npm_credentials(
            "registry=https://registry.example/\n\
             //registry.example/:_authToken=made-up\n\
             _auth = made-up\n\
             @scope:registry=https://name:made-up@registry.example/\n\
             tokenHelper=/a/helper\n",
        )
        .expect("settings holding credentials need a copy");
        assert_eq!(
            kept,
            "registry=https://registry.example/\n\
             ; //registry.example/:_authToken is held back by the agent fence\n\
             ; _auth is held back by the agent fence\n\
             ; @scope:registry is held back by the agent fence\n\
             tokenHelper=/a/helper\n"
        );
    }

    #[test]
    fn npms_settings_holding_no_credential_need_no_copy() {
        assert_eq!(
            without_npm_credentials("registry=https://registry.example/\n; a comment\n"),
            None
        );
    }
}
