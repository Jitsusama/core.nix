//! The person's token, held in memory only, and the placeholder the fence
//! holds in its place.
//!
//! The door reads the token by running `gh auth token` outside the fence,
//! where the keyring is, the first time a call needs it, and again when
//! GitHub answers 401, since a token can be refreshed or replaced under
//! it. It's never written down, never said in a line, and never handed to
//! a client: a fenced command holds only [`PLACEHOLDER`], and a request
//! carrying any other credential is refused.

use std::fmt;
use std::process::{Command, Stdio};
use std::sync::Mutex;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::judge::Host;

/// What the fence holds as `GH_TOKEN`: a fixed word, worth nothing
/// anywhere but at the door.
pub const PLACEHOLDER: &str = "pi-github-door-placeholder";

/// The user git names when it sends a token as a password.
const GIT_USER: &str = "x-access-token";

/// A token, which never prints.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Secret(..)")
    }
}

/// The token as the door holds it, and how it reads it again.
pub struct Token {
    command: Vec<String>,
    held: Mutex<Option<Secret>>,
}

impl Token {
    /// A token read, when first needed, by running `command`.
    pub fn new(command: Vec<String>) -> Self {
        Token {
            command,
            held: Mutex::new(None),
        }
    }

    /// The token, read now if it isn't held.
    pub fn current(&self) -> Result<Secret, String> {
        let mut held = self
            .held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(secret) = held.as_ref() {
            return Ok(secret.clone());
        }
        let secret = self.read()?;
        *held = Some(secret.clone());
        Ok(secret)
    }

    /// The token read again after GitHub refused `refused`, or none when
    /// it reads the same, so a call is tried again only with a new one.
    /// Another call that already read a new one is taken at its word.
    pub fn renewed(&self, refused: &Secret) -> Option<Secret> {
        let mut held = self
            .held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(secret) = held.as_ref().filter(|secret| *secret != refused) {
            return Some(secret.clone());
        }
        let secret = self.read().ok()?;
        *held = Some(secret.clone());
        (secret != *refused).then_some(secret)
    }

    /// The token, as the command prints it.
    fn read(&self) -> Result<Secret, String> {
        let (program, args) = self
            .command
            .split_first()
            .ok_or("no command reads the token")?;
        let output = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .map_err(|error| format!("{program} couldn't run: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "{program} gave no token ({}); sign in with gh auth login outside the fence",
                output.status
            ));
        }
        let text = String::from_utf8(output.stdout).map_err(|_| "the token isn't UTF-8")?;
        let token = text.trim();
        if token.is_empty() || !token.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(format!("{program} printed no token"));
        }
        Ok(Secret(token.to_owned()))
    }
}

/// Whether an Authorization header's value carries the placeholder and
/// nothing else: as a token, a bearer, or the user or password of basic.
pub fn is_placeholder(value: &str) -> bool {
    let value = value.trim();
    let Some((scheme, rest)) = value.split_once(' ') else {
        return false;
    };
    let rest = rest.trim();
    match scheme.to_ascii_lowercase().as_str() {
        "token" | "bearer" => rest == PLACEHOLDER,
        "basic" => STANDARD
            .decode(rest)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|pair| {
                pair.split_once(':')
                    .map(|(user, password)| user == PLACEHOLDER || password == PLACEHOLDER)
            })
            .unwrap_or(false),
        _ => false,
    }
}

/// The Authorization header's value `host` is sent with `secret`, or none
/// for codeload, whose links carry their own.
pub fn authorization(host: Host, secret: &Secret) -> Option<String> {
    match host {
        Host::Api | Host::Uploads => Some(format!("token {}", secret.0)),
        Host::Web => Some(format!(
            "Basic {}",
            STANDARD.encode(format!("{GIT_USER}:{}", secret.0))
        )),
        Host::Codeload => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_placeholder_is_known_in_each_form_gh_and_git_send_it() {
        assert!(
            is_placeholder(&format!("token {PLACEHOLDER}")),
            "gh's form is the placeholder"
        );
        assert!(
            is_placeholder(&format!("Bearer {PLACEHOLDER}")),
            "a bearer's form is the placeholder"
        );
        let basic = STANDARD.encode(format!("x-access-token:{PLACEHOLDER}"));
        assert!(
            is_placeholder(&format!("Basic {basic}")),
            "git's form is the placeholder"
        );
    }

    #[test]
    fn any_other_credential_is_not() {
        assert!(
            !is_placeholder("token ghp_realtoken"),
            "a real token isn't the placeholder"
        );
        assert!(
            !is_placeholder(&format!("token {PLACEHOLDER}x")),
            "a longer token isn't the placeholder"
        );
        assert!(
            !is_placeholder(PLACEHOLDER),
            "the placeholder with no scheme isn't"
        );
        assert!(
            !is_placeholder(&format!("Basic {}", STANDARD.encode("me:hunter2"))),
            "a user and password neither the placeholder isn't"
        );
        assert!(
            !is_placeholder(&format!("Digest {PLACEHOLDER}")),
            "another scheme isn't the placeholder"
        );
    }

    #[test]
    fn a_secret_never_prints() {
        assert_eq!(format!("{:?}", Secret("ghp_x".to_owned())), "Secret(..)");
    }

    #[test]
    fn it_is_read_once_and_again_only_when_it_changed() {
        let folder = tempfile::tempdir().expect("a folder");
        let file = folder.path().join("token");
        std::fs::write(&file, "one\n").expect("written");
        let token = Token::new(vec!["cat".to_owned(), file.display().to_string()]);
        let first = token.current().expect("read");
        std::fs::write(&file, "two\n").expect("written");
        assert_eq!(
            token.current().expect("held"),
            first,
            "held, not read again"
        );
        let renewed = token.renewed(&first).expect("a new one");
        assert_eq!(token.current().expect("held"), renewed);
        assert_eq!(
            token.renewed(&renewed),
            None,
            "the same one isn't tried again"
        );
    }

    #[test]
    fn a_command_that_fails_gives_no_token() {
        let token = Token::new(vec!["false".to_owned()]);
        assert!(
            token.current().is_err(),
            "a command that fails gives no token"
        );
    }
}
