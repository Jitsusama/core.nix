//! The two calls made to Drive, both scoped to the backup's one folder:
//! whether a name is in it, and making a file in it. Nothing here can
//! change, move or remove a file, and the tests fail on any other call.

use std::fs::File;
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};

/// How long a connection may take to open.
const CONNECT: Duration = Duration::from_secs(30);

/// Why a call failed.
#[derive(Debug)]
pub enum Fault {
    /// Drive no longer takes the token: trade for another and try again.
    Unauthorized,
    /// Drive is busy or out of reach for now, and asks for a retry later.
    Busy(String),
    Other(String),
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fault::Unauthorized => f.write_str("Drive no longer takes the login's token"),
            Fault::Busy(why) | Fault::Other(why) => f.write_str(why),
        }
    }
}

/// An agent that hands back every answer, so a refusal can be read.
pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_connect(Some(CONNECT))
        .build()
        .into()
}

/// The backup's folder on Drive.
pub struct Drive<'a> {
    pub agent: &'a ureq::Agent,
    pub api: &'a str,
    pub folder: &'a str,
}

impl Drive<'_> {
    /// The id of the file of this name in the folder, if there is one.
    pub fn find(&self, token: &str, name: &str) -> Result<Option<String>, Fault> {
        let query = format!(
            "name = '{}' and '{}' in parents and trashed = false",
            quoted(name),
            quoted(self.folder)
        );
        let url = format!(
            "{}/drive/v3/files?q={}&fields=files(id)&supportsAllDrives=true\
             &includeItemsFromAllDrives=true",
            self.api,
            encoded(&query)
        );
        let mut answer = self
            .agent
            .get(&url)
            .header("Authorization", format!("Bearer {token}"))
            .call()
            .map_err(|why| Fault::Busy(format!("can't reach Drive: {why}")))?;
        let body = read(&mut answer, "look for", name)?;
        let value: Value = serde_json::from_str(&body)
            .map_err(|why| Fault::Other(format!("Drive's list of {name} isn't JSON: {why}")))?;
        Ok(value
            .get("files")
            .and_then(Value::as_array)
            .and_then(|files| files.first())
            .and_then(|file| file.get("id"))
            .and_then(Value::as_str)
            .map(str::to_owned))
    }

    /// Make a file of this name in the folder from the sealed copy at
    /// `path`, `bytes` long, through a resumable upload, the kind Drive
    /// takes past 5 MB. Answers the new file's id.
    pub fn create(
        &self,
        token: &str,
        name: &str,
        path: &Path,
        bytes: u64,
    ) -> Result<String, Fault> {
        let metadata = json!({
            "name": name,
            "parents": [self.folder],
            "mimeType": "application/octet-stream",
        });
        let url = format!(
            "{}/upload/drive/v3/files?uploadType=resumable&supportsAllDrives=true&fields=id",
            self.api
        );
        let mut opened = self
            .agent
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("X-Upload-Content-Type", "application/octet-stream")
            .header("X-Upload-Content-Length", bytes.to_string())
            .content_type("application/json; charset=UTF-8")
            .send(metadata.to_string())
            .map_err(|why| Fault::Busy(format!("can't reach Drive: {why}")))?;
        let location = opened
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        read(&mut opened, "start sending", name)?;
        let location =
            location.ok_or_else(|| Fault::Other(format!("Drive gave nowhere to send {name}")))?;
        let mut file = File::open(path)
            .map_err(|why| Fault::Other(format!("can't read {}: {why}", path.display())))?;
        let mut sent = self
            .agent
            .put(&location)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Length", bytes.to_string())
            .content_type("application/octet-stream")
            .send(ureq::SendBody::from_reader(&mut file))
            .map_err(|why| Fault::Busy(format!("can't send {name} to Drive: {why}")))?;
        let body = read(&mut sent, "send", name)?;
        let value: Value = serde_json::from_str(&body)
            .map_err(|why| Fault::Other(format!("Drive's answer for {name} isn't JSON: {why}")))?;
        value
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| Fault::Other(format!("Drive took {name} but gave no id")))
    }
}

/// An answer's body, or why Drive refused.
fn read(
    answer: &mut ureq::http::Response<ureq::Body>,
    doing: &str,
    name: &str,
) -> Result<String, Fault> {
    let status = answer.status().as_u16();
    let body = answer.body_mut().read_to_string().unwrap_or_default();
    match status {
        200..=299 => Ok(body),
        401 => Err(Fault::Unauthorized),
        // Drive answers these when it is busy, and asks to be tried again.
        429 | 500..=599 => Err(Fault::Busy(format!(
            "Drive refused to {doing} {name} ({status}): {}",
            body.trim()
        ))),
        // A login made without a Drive scope is refused this way, which
        // signing in again with one fixes.
        403 if body.contains("insufficient authentication scopes")
            || body.contains("ACCESS_TOKEN_SCOPE_INSUFFICIENT") =>
        {
            Err(Fault::Other(format!(
                "Drive refused to {doing} {name}, since the login may not use Drive; \
                 sign in with {}",
                crate::token::SIGN_IN
            )))
        }
        _ => Err(Fault::Other(format!(
            "Drive refused to {doing} {name} ({status}): {}",
            body.trim()
        ))),
    }
}

/// A value inside a Drive query's single quotes.
fn quoted(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

/// A query string's value, percent-encoded.
fn encoded(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                char::from(byte).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}
