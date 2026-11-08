//! Installation tokens: minted from GitHub with the App's own token, and
//! held until nearly spent, so a sandbox that runs a hundred commands in an
//! hour asks GitHub once.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::app::App;

/// What every token may do, and nothing more: read code, and open and
/// comment on pull requests and issues. Contents write is what merging
/// takes, so a token can never merge, and none can push or change a
/// setting. GitHub refuses a token more than its App was granted, so this
/// narrows a broader App too.
const PERMISSIONS: &str =
    r#"{"metadata":"read","contents":"read","pull_requests":"write","issues":"write"}"#;

/// GitHub's tokens last an hour; one is handed out for fifty minutes of it,
/// so none reaches a command with less than ten left.
const KEEP: Duration = Duration::from_secs(50 * 60);

/// An installation token and the moment GitHub says it lapses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub value: String,
    pub expires_at: String,
}

type Opener = Box<dyn FnMut() -> Result<App, String> + Send>;

/// Mints tokens for the App its opener opens, opening it at the first ask
/// and again at each ask until it opens, so it can run before the App is
/// made.
pub struct Minter {
    api: String,
    agent: ureq::Agent,
    open: Opener,
    app: Option<App>,
    held: Option<(Token, Instant)>,
    keep: Duration,
}

impl Minter {
    /// A minter asking the GitHub API at `api` (`https://api.github.com`).
    pub fn new(api: &str, open: Opener) -> Minter {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(30)))
            .user_agent("github-broker")
            .build()
            .into();
        Minter {
            api: api.trim_end_matches('/').to_owned(),
            agent,
            open,
            app: None,
            held: None,
            keep: KEEP,
        }
    }

    /// The same, holding each token for `keep` rather than fifty minutes.
    pub fn keeping(mut self, keep: Duration) -> Minter {
        self.keep = keep;
        self
    }

    /// A token with at least ten minutes left, the one held or a new one.
    pub fn token(&mut self) -> Result<Token, String> {
        if let Some((token, minted)) = &self.held
            && minted.elapsed() < self.keep
        {
            return Ok(token.clone());
        }
        if self.app.is_none() {
            self.app = Some((self.open)()?);
        }
        let token = self.mint()?;
        self.held = Some((token.clone(), Instant::now()));
        Ok(token)
    }

    fn mint(&self) -> Result<Token, String> {
        let Some(app) = &self.app else {
            return Err("the App isn't open".to_owned());
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "the clock is before 1970".to_owned())?
            .as_secs();
        let url = format!(
            "{}/app/installations/{}/access_tokens",
            self.api,
            app.installation()
        );
        let mut answer = self
            .agent
            .post(&url)
            .header("Authorization", format!("Bearer {}", app.jwt(now)))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .content_type("application/json")
            .send(format!(r#"{{"permissions":{PERMISSIONS}}}"#))
            .map_err(|why| format!("can't reach GitHub: {why}"))?;
        let status = answer.status();
        let body = answer
            .body_mut()
            .read_to_string()
            .map_err(|why| format!("GitHub's answer didn't arrive whole: {why}"))?;
        let said: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
        if status.as_u16() != 201 {
            let why = said["message"].as_str().unwrap_or("no reason given");
            return Err(format!("GitHub refused a token ({status}): {why}"));
        }
        let value = said["token"].as_str().unwrap_or_default();
        let expires_at = said["expires_at"].as_str().unwrap_or_default();
        // Both travel as words on one line, so neither may hold a space.
        let word = |text: &str| {
            !text.is_empty()
                && text
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-:.+".contains(&b))
        };
        if !word(value) || !word(expires_at) {
            return Err("GitHub answered without a token".to_owned());
        }
        Ok(Token {
            value: value.to_owned(),
            expires_at: expires_at.to_owned(),
        })
    }
}
