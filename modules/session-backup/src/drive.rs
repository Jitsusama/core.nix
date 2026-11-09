//! The three calls made to Drive, each about the backup's one folder:
//! whether a name is in it, making a file in it, and reading back a file
//! found there. A backup into a folder of its own makes one call more,
//! once a run: finding that folder, or making it when there is none.
//! Nothing here can change, move or remove a file, and the tests fail on
//! any other call.

use std::fs::File;
use std::io;
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use ureq::SendBody;
use ureq::http::Request;
use ureq::http::header::HeaderValue;
use ureq::middleware::MiddlewareNext;

use crate::config::Folder;
use crate::retry::{again, patiently};
use crate::token::{Login, SET_PROJECT};

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

/// An agent that hands back every answer, so a refusal can be read. With
/// a `project`, every call names it as the one Drive counts it against,
/// which Drive asks of a login made with gcloud's application-default
/// client and refuses without.
pub fn agent(project: Option<&str>) -> Result<ureq::Agent, String> {
    let config = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_connect(Some(CONNECT));
    let config = match project {
        Some(project) => {
            let value = HeaderValue::from_str(project)
                .map_err(|why| format!("the login's quota project {project:?}: {why}"))?;
            config.middleware(
                move |mut request: Request<SendBody>, next: MiddlewareNext| {
                    request.headers_mut().insert(QUOTA_PROJECT, value.clone());
                    next.handle(request)
                },
            )
        }
        None => config,
    };
    Ok(config.build().into())
}

/// The header naming the project a call to a Google API is counted against.
const QUOTA_PROJECT: &str = "x-goog-user-project";

/// The backup's folder on Drive.
pub struct Drive<'a> {
    pub agent: &'a ureq::Agent,
    pub api: &'a str,
    pub folder: &'a str,
    /// How to sign in with a login that may use the folder, said when
    /// Drive refuses one that may not.
    pub sign_in: &'a str,
}

/// What Drive calls a folder.
const FOLDER_TYPE: &str = "application/vnd.google-apps.folder";

/// The id of the backup's own folder, `name`, at the top of the account's
/// Drive: the one a run made before, or one made now. A login that may use
/// only the files the backup made lists only those, so a folder of the
/// same name someone made in the web page is never the one it adds to.
pub fn own_folder(
    agent: &ureq::Agent,
    api: &str,
    sign_in: &str,
    token: &str,
    name: &str,
) -> Result<String, Fault> {
    let query = format!(
        "name = '{}' and mimeType = '{FOLDER_TYPE}' and 'root' in parents and trashed = false",
        quoted(name)
    );
    let url = format!(
        "{api}/drive/v3/files?q={}&fields=files(id)&orderBy=createdTime",
        encoded(&query)
    );
    let mut answer = agent
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .call()
        .map_err(|why| Fault::Busy(format!("can't reach Drive: {why}")))?;
    let body = read(&mut answer, sign_in, "look for the folder", name)?;
    let value: Value = serde_json::from_str(&body)
        .map_err(|why| Fault::Other(format!("Drive's list of {name} isn't JSON: {why}")))?;
    if let Some(id) = first_id(&value) {
        return Ok(id);
    }
    let mut made = agent
        .post(&format!("{api}/drive/v3/files?fields=id"))
        .header("Authorization", format!("Bearer {token}"))
        .content_type("application/json; charset=UTF-8")
        .send(json!({ "name": name, "mimeType": FOLDER_TYPE }).to_string())
        .map_err(|why| Fault::Busy(format!("can't reach Drive: {why}")))?;
    let body = read(&mut made, sign_in, "make the folder", name)?;
    let value: Value = serde_json::from_str(&body)
        .map_err(|why| Fault::Other(format!("Drive's answer for {name} isn't JSON: {why}")))?;
    value
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| Fault::Other(format!("Drive made the folder {name} but gave no id")))
}

/// The id of the folder the settings name: the one given, or the backup's
/// own, found or made with the retries a busy Drive is given.
pub fn folder_id(
    folder: &Folder,
    agent: &ureq::Agent,
    api: &str,
    retries: &[Duration],
    login: &mut Login,
) -> Result<String, Fault> {
    match folder {
        Folder::Given(id) => Ok(id.clone()),
        Folder::Own(name) => patiently(retries, || {
            again(login, agent, |token| {
                own_folder(agent, api, crate::token::sign_in(folder), token, name)
            })
        }),
    }
}

/// The id of the first file in a list Drive answered.
fn first_id(value: &Value) -> Option<String> {
    value
        .get("files")
        .and_then(Value::as_array)
        .and_then(|files| files.first())
        .and_then(|file| file.get("id"))
        .and_then(Value::as_str)
        .map(str::to_owned)
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
        let body = read(&mut answer, self.sign_in, "look for", name)?;
        let value: Value = serde_json::from_str(&body)
            .map_err(|why| Fault::Other(format!("Drive's list of {name} isn't JSON: {why}")))?;
        Ok(first_id(&value))
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
        read(&mut opened, self.sign_in, "start sending", name)?;
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
        let body = read(&mut sent, self.sign_in, "send", name)?;
        let value: Value = serde_json::from_str(&body)
            .map_err(|why| Fault::Other(format!("Drive's answer for {name} isn't JSON: {why}")))?;
        value
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| Fault::Other(format!("Drive took {name} but gave no id")))
    }

    /// Write the file Drive holds as `id` to `path`, however long it is;
    /// answers how many bytes it wrote. `name` says which file in a fault.
    pub fn download(&self, token: &str, id: &str, name: &str, path: &Path) -> Result<u64, Fault> {
        let url = format!(
            "{}/drive/v3/files/{}?alt=media&supportsAllDrives=true",
            self.api,
            encoded(id)
        );
        let mut answer = self
            .agent
            .get(&url)
            .header("Authorization", format!("Bearer {token}"))
            .call()
            .map_err(|why| Fault::Busy(format!("can't reach Drive: {why}")))?;
        if !answer.status().is_success() {
            read(&mut answer, self.sign_in, "read back", name)?;
        }
        let mut file = File::create(path)
            .map_err(|why| Fault::Other(format!("can't write {}: {why}", path.display())))?;
        // A connection dropped part way is Drive out of reach for now; the
        // retry starts the file again from nothing.
        io::copy(&mut answer.body_mut().as_reader(), &mut file)
            .map_err(|why| Fault::Busy(format!("can't read {name} back from Drive: {why}")))
    }
}

/// An answer's body, or why Drive refused.
fn read(
    answer: &mut ureq::http::Response<ureq::Body>,
    sign_in: &str,
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
        // A login made with gcloud's application-default client names no
        // project to count its calls against until one is set.
        403 if body.contains("requires a quota project") => Err(Fault::Other(format!(
            "Drive refused to {doing} {name}, since the login names no quota project; \
             name one with {SET_PROJECT}"
        ))),
        403 if body.contains("insufficient authentication scopes")
            || body.contains("ACCESS_TOKEN_SCOPE_INSUFFICIENT") =>
        {
            Err(Fault::Other(format!(
                "Drive refused to {doing} {name}, since the login may not use Drive; \
                 sign in with {sign_in}"
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
