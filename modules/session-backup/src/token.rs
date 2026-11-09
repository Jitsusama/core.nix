//! The access token a run sends to Drive, from the account's gcloud
//! application-default login: its refresh token traded at Google's token
//! endpoint, as gcloud itself would, without running gcloud.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::config::Folder;

/// How to sign in so the login can reach a folder made in Drive's web page.
/// gcloud's own login asks for no Drive scope, and insists on
/// cloud-platform among any it is given. The whole of Drive, rather than
/// only the files this makes, because a folder made in the web page is one
/// this could not otherwise add to.
pub const SIGN_IN: &str = "gcloud auth application-default login \
     --scopes=openid,https://www.googleapis.com/auth/userinfo.email,\
     https://www.googleapis.com/auth/cloud-platform,https://www.googleapis.com/auth/drive";

/// How to sign in so the login can reach the backup's own folder: only the
/// files the backup made, so the login can read nothing else in Drive. It
/// goes in a gcloud folder of the backup's own, so the account's own login
/// needs no Drive scope at all.
pub const SIGN_IN_OWN: &str = "CLOUDSDK_CONFIG=<the backup's own folder> \
     gcloud auth application-default login \
     --scopes=openid,https://www.googleapis.com/auth/userinfo.email,\
     https://www.googleapis.com/auth/cloud-platform,https://www.googleapis.com/auth/drive.file";

/// How to name the project Drive counts a login's calls against, which a
/// login made with gcloud's application-default client needs, run with
/// the same `CLOUDSDK_CONFIG` the login was made with.
pub const SET_PROJECT: &str =
    "gcloud auth application-default set-quota-project <a project with Drive's API on>";

/// How to sign in for a backup into `folder`.
pub fn sign_in(folder: &Folder) -> &'static str {
    match folder {
        Folder::Given(_) => SIGN_IN,
        Folder::Own(_) => SIGN_IN_OWN,
    }
}

/// A token this close to its end is traded again before it is sent.
const MARGIN: Duration = Duration::from_mins(5);

/// The login a token is made from.
pub struct Login {
    client_id: String,
    client_secret: String,
    refresh_token: String,
    /// The project Drive counts the calls against, which gcloud's own
    /// application-default client must name; a login made with gcloud's
    /// older client works without one.
    project: Option<String>,
    endpoint: String,
    /// How to sign in again, said when Google refuses the login.
    sign_in: &'static str,
    held: Option<(String, Instant)>,
}

impl Login {
    /// The login in an application-default credentials file, as
    /// `gcloud auth application-default login` writes it. `sign_in` says
    /// how to make it again, when it is missing or Google refuses it.
    pub fn read(path: &Path, endpoint: &str, sign_in: &'static str) -> Result<Login, String> {
        let text = fs::read_to_string(path).map_err(|why| {
            format!(
                "can't read the Google login in {}: {why}; sign in once with {sign_in}",
                path.display()
            )
        })?;
        let value: Value = serde_json::from_str(&text)
            .map_err(|why| format!("the Google login in {} isn't JSON: {why}", path.display()))?;
        let field = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| format!("the Google login in {} has no {key}", path.display()))
        };
        if field("type")? != "authorized_user" {
            return Err(format!(
                "the Google login in {} isn't a person's login",
                path.display()
            ));
        }
        Ok(Login {
            client_id: field("client_id")?,
            client_secret: field("client_secret")?,
            refresh_token: field("refresh_token")?,
            project: value
                .get("quota_project_id")
                .and_then(Value::as_str)
                .map(str::to_owned),
            endpoint: endpoint.to_owned(),
            sign_in,
            held: None,
        })
    }

    /// A token good for a few minutes yet, traded for when the one held is
    /// about to end.
    pub fn token(&mut self, agent: &ureq::Agent) -> Result<String, String> {
        if let Some((token, until)) = &self.held
            && Instant::now() + MARGIN < *until
        {
            return Ok(token.clone());
        }
        let mut answer = agent
            .post(&self.endpoint)
            .send_form([
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("refresh_token", self.refresh_token.as_str()),
                ("grant_type", "refresh_token"),
            ])
            .map_err(|why| format!("can't reach Google to sign in: {why}"))?;
        let status = answer.status().as_u16();
        let body = answer.body_mut().read_to_string().unwrap_or_default();
        if status != 200 {
            return Err(format!(
                "Google refused the login ({status}): {}; sign in again with {}",
                body.trim(),
                self.sign_in
            ));
        }
        let value: Value = serde_json::from_str(&body)
            .map_err(|why| format!("Google's token answer isn't JSON: {why}"))?;
        let token = value
            .get("access_token")
            .and_then(Value::as_str)
            .ok_or("Google's token answer has no access token")?
            .to_owned();
        let lasts = value
            .get("expires_in")
            .and_then(Value::as_u64)
            .unwrap_or(3600);
        self.held = Some((token.clone(), Instant::now() + Duration::from_secs(lasts)));
        Ok(token)
    }

    /// The project Drive is to count the calls against, when the login
    /// names one, as `gcloud auth application-default set-quota-project`
    /// writes it.
    pub fn project(&self) -> Option<&str> {
        self.project.as_deref()
    }

    /// Forget the token held, as when Drive says it no longer counts.
    pub fn forget(&mut self) {
        self.held = None;
    }
}
