//! Asking gcloud, with the person's own login, for one of the service
//! account's tokens.

use std::ffi::OsStr;
use std::io::Read;
use std::process::{Command, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::account::Account;

/// How long gcloud has to mint a token. It takes seconds, fifteen at
/// worst when we looked, so a minute is a hung gcloud rather than a slow
/// one, such as a login asking to be renewed with nobody to answer.
pub const PATIENCE: Duration = Duration::from_mins(1);

/// How often a running gcloud is looked at again; the standard library
/// can't wait on a child with a deadline.
const LOOK_AGAIN: Duration = Duration::from_millis(50);

/// Variables that would make gcloud mint with something other than the
/// person's login, or impersonate another account. The broker is asked
/// from programs that may hand these to the commands they start, and a
/// token of the service account's own must never be the one minting.
const NOT_HANDED_ON: [&str; 5] = [
    "CLOUDSDK_AUTH_ACCESS_TOKEN",
    "CLOUDSDK_AUTH_ACCESS_TOKEN_FILE",
    "CLOUDSDK_AUTH_CREDENTIAL_FILE_OVERRIDE",
    "CLOUDSDK_AUTH_IMPERSONATE_SERVICE_ACCOUNT",
    "GOOGLE_APPLICATION_CREDENTIALS",
];

/// What's asked for: a token for Google's APIs, or one saying who the
/// account is to a service, such as IAP or Cloud Run, named as its
/// audience.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask<'a> {
    Access,
    Identity(&'a str),
}

/// Whether `text` is a token as Google mints one: an access token or a
/// signed JSON web token, letters, digits, dots, dashes and underscores,
/// none of which a shell reads as anything but itself.
#[must_use]
pub fn is_token(text: &str) -> bool {
    (20..=4096).contains(&text.len())
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

/// Whether `audience` may be asked for: a service's address, so one line
/// with no spaces.
#[must_use]
pub fn is_audience(audience: &str) -> bool {
    (1..=512).contains(&audience.len()) && audience.bytes().all(|b| b.is_ascii_graphic())
}

/// What gcloud is run with to mint what's asked for the account.
#[must_use]
pub fn arguments(ask: Ask<'_>, account: &Account) -> Vec<String> {
    let mut said = vec!["auth".to_owned()];
    match ask {
        Ask::Access => said.push("print-access-token".to_owned()),
        Ask::Identity(audience) => {
            said.push("print-identity-token".to_owned());
            said.push(format!("--audiences={audience}"));
            said.push("--include-email".to_owned());
        }
    }
    said.push(format!("--impersonate-service-account={}", account.email()));
    // Only errors, since gcloud warns of the impersonation on every run,
    // and never a prompt, since nobody is there to answer one.
    said.push("--verbosity=error".to_owned());
    said.push("--quiet".to_owned());
    said
}

/// Runs `gcloud` to mint what's asked for the account, and gives the token.
///
/// # Errors
///
/// When gcloud can't be run, refuses, says something that isn't a token,
/// or takes longer than `patience`.
pub fn mint(
    gcloud: &OsStr,
    ask: Ask<'_>,
    account: &Account,
    patience: Duration,
) -> Result<String, String> {
    let mut command = Command::new(gcloud);
    command
        .args(arguments(ask, account))
        .env("CLOUDSDK_CORE_DISABLE_PROMPTS", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for name in NOT_HANDED_ON {
        command.env_remove(name);
    }
    let mut child = command
        .spawn()
        .map_err(|why| format!("can't run {}: {why}", gcloud.to_string_lossy()))?;
    // Both are read as gcloud writes, so a full pipe never stops it.
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < patience => std::thread::sleep(LOOK_AGAIN),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "gcloud didn't answer within {} s",
                    patience.as_secs()
                ));
            }
            Err(why) => return Err(format!("can't wait for gcloud: {why}")),
        }
    };
    let said = out.join().unwrap_or_default();
    let complaint = err.join().unwrap_or_default();
    if !status.success() {
        let why = complaint
            .lines()
            .map(str::trim)
            .rfind(|line| !line.is_empty())
            .unwrap_or("it said nothing");
        let why: String = why.chars().take(300).collect();
        return Err(format!("gcloud refused a token: {why}"));
    }
    let token = said.trim();
    if !is_token(token) {
        return Err("gcloud answered without a token".to_owned());
    }
    Ok(token.to_owned())
}

/// Reads all a pipe says, on a thread of its own.
fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> JoinHandle<String> {
    std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_string(&mut text);
        }
        text
    })
}

#[cfg(test)]
mod tests {
    use super::{Ask, arguments, is_audience, is_token};
    use crate::account::Account;

    #[test]
    fn gcloud_is_asked_as_the_account_and_never_to_prompt() {
        let account = Account::parse("agents@lab.iam.gserviceaccount.com").unwrap();
        assert_eq!(
            arguments(Ask::Access, &account),
            [
                "auth",
                "print-access-token",
                "--impersonate-service-account=agents@lab.iam.gserviceaccount.com",
                "--verbosity=error",
                "--quiet",
            ],
            "an access token is asked for as the account"
        );
        assert_eq!(
            arguments(Ask::Identity("https://run.example"), &account)[1..4],
            [
                "print-identity-token",
                "--audiences=https://run.example",
                "--include-email",
            ],
            "an identity token names its audience"
        );
    }

    #[test]
    fn only_a_token_is_a_token() {
        assert!(
            is_token(&format!("ya29.{}", "a-b_c".repeat(40))),
            "an access token"
        );
        assert!(!is_token("ya29.short"), "too short to be minted");
        assert!(!is_token(&"a".repeat(4097)), "too long to be minted");
        assert!(!is_token("ya29.aaaaaaaaaaaaaaaaaaaa bbb"), "two words");
        assert!(
            !is_token("ya29.aaaaaaaaaaaaaaaaaaaa\nexport X=1"),
            "a second line"
        );
    }

    #[test]
    fn an_audience_is_one_word() {
        assert!(
            is_audience("https://lab.example/run"),
            "a service's address"
        );
        assert!(!is_audience(""), "nothing");
        assert!(!is_audience("https://a.example --impersonate"), "two words");
    }
}
