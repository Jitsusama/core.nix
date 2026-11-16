//! Asking Drive as the account: the documents looked at lately, a place's
//! and a folder's files, the shared drives, one file and its content, and
//! putting a file there new or over one.

use std::fs::File;
use std::path::Path;

use serde_json::Value;
use ureq::RequestBuilder;
use ureq::typestate::WithBody;

use super::{DOWNLOAD_TAKES, ENTRY_FIELDS, Google, Put, Refused, refused, unput, unreachable};
use crate::drive;
use crate::saves;

impl Google {
    /// The documents of one kind Joel looked at lately, the last first,
    /// from his own drive, what's shared with him and shared drives alike,
    /// with what every Drive file's row says of each.
    pub fn documents(&self, token: &str, kind: drive::Kind) -> Result<Value, Refused> {
        let shown = drive::SHOWN.to_string();
        let fields = format!("files({})", drive::FILE_FIELDS);
        self.get(
            token,
            "/drive/v3/files",
            &[
                ("q", &kind.search()),
                ("orderBy", "viewedByMeTime desc"),
                ("pageSize", &shown),
                ("corpora", "allDrives"),
                ("includeItemsFromAllDrives", "true"),
                ("supportsAllDrives", "true"),
                ("fields", &fields),
            ],
            "the documents",
        )
    }

    /// What one of Drive's own places holds, in its order, from every
    /// drive, with what every Drive file's row says of each, and whether
    /// it holds more.
    pub fn place(&self, token: &str, place: drive::Place) -> Result<Value, Refused> {
        let most = place.most().to_string();
        let fields = format!("nextPageToken,files({})", drive::FILE_FIELDS);
        self.get(
            token,
            "/drive/v3/files",
            &[
                ("q", &place.search()),
                ("orderBy", place.order()),
                ("pageSize", &most),
                ("corpora", "allDrives"),
                ("includeItemsFromAllDrives", "true"),
                ("supportsAllDrives", "true"),
                ("fields", &fields),
            ],
            "one of Drive's places",
        )
    }

    /// What a Drive folder holds, found by `search`, folders first, then
    /// by name, from every drive; none when Drive has no such folder.
    pub fn folder(&self, token: &str, search: &str) -> Result<Option<Value>, Refused> {
        let most = drive::LISTED_MOST.to_string();
        let fields = format!("nextPageToken,files({ENTRY_FIELDS})");
        self.found(
            token,
            "/drive/v3/files",
            &[
                ("q", search),
                ("orderBy", "folder,name_natural"),
                ("pageSize", &most),
                ("corpora", "allDrives"),
                ("includeItemsFromAllDrives", "true"),
                ("supportsAllDrives", "true"),
                ("fields", &fields),
            ],
            "a folder",
        )
    }

    /// The shared drives Joel is on, each with when it was made and
    /// whether he hid it, which Drive's places say.
    pub fn drives(&self, token: &str) -> Result<Value, Refused> {
        let most = drive::PLACED_MOST.to_string();
        self.get(
            token,
            "/drive/v3/drives",
            &[
                ("pageSize", &most),
                ("fields", "nextPageToken,drives(id,name,createdTime,hidden)"),
            ],
            "the shared drives",
        )
    }

    /// What Drive says of the file `id`, on any drive; none when it has
    /// no such file.
    pub fn file(&self, token: &str, id: &str) -> Result<Option<Value>, Refused> {
        self.found(
            token,
            &format!("/drive/v3/files/{id}"),
            &[("supportsAllDrives", "true"), ("fields", ENTRY_FIELDS)],
            "a file",
        )
    }

    /// The bytes of the file `id` as they come, or, for one of Google's
    /// own kinds, of its copy exported `as` the kind named; none when
    /// Drive has no such file.
    pub fn content(
        &self,
        token: &str,
        id: &str,
        exported: Option<&str>,
    ) -> Result<Option<impl std::io::Read + use<>>, Refused> {
        let request = match exported {
            Some(kind) => self
                .agent
                .get(format!("{}/drive/v3/files/{id}/export", self.origins.api))
                .query("mimeType", kind),
            None => self
                .agent
                .get(format!("{}/drive/v3/files/{id}", self.origins.api))
                .query("alt", "media")
                .query("supportsAllDrives", "true"),
        };
        let answer = request
            .header("Authorization", &format!("Bearer {token}"))
            .config()
            .timeout_global(Some(DOWNLOAD_TAKES))
            .build()
            .call()
            .map_err(|error| unreachable(&error))?;
        match answer.status().as_u16() {
            200 => Ok(Some(answer.into_body().into_reader())),
            404 => Ok(None),
            status => Err(refused(status, "a file's bytes")),
        }
    }

    /// Sends the file at `path` into Drive as a new file named `name` in
    /// `folder`, `root` or a folder's or shared drive's id.
    pub fn create(
        &self,
        token: &str,
        folder: &str,
        name: &str,
        path: &Path,
    ) -> Result<Put, Refused> {
        let started = self
            .agent
            .post(format!("{}/upload/drive/v3/files", self.origins.api));
        let about = serde_json::json!({ "name": name, "parents": [folder] });
        self.put(token, started, &about, name, path)
    }

    /// Sends the file at `path` into Drive as a new version of the file
    /// `id`, its name and folder as they were.
    pub fn replace(&self, token: &str, id: &str, name: &str, path: &Path) -> Result<Put, Refused> {
        let started = self
            .agent
            .patch(format!("{}/upload/drive/v3/files/{id}", self.origins.api));
        self.put(token, started, &serde_json::json!({}), name, path)
    }

    /// Sends a file resumably: what it is first, then its bytes where
    /// Drive says, read from disk as they go, so one of any size is never
    /// held whole.
    fn put(
        &self,
        token: &str,
        started: RequestBuilder<WithBody>,
        about: &Value,
        name: &str,
        path: &Path,
    ) -> Result<Put, Refused> {
        let Ok(file) = File::open(path) else {
            return Ok(Put::Unread);
        };
        let mime = saves::mime(name);
        let mut answer = started
            .header("Authorization", &format!("Bearer {token}"))
            .header("X-Upload-Content-Type", mime)
            .query("uploadType", "resumable")
            .query("supportsAllDrives", "true")
            .query("fields", "id,webViewLink")
            .send_json(about)
            .map_err(|error| unreachable(&error))?;
        let status = answer.status().as_u16();
        if status != 200 {
            return unput(status, &mut answer);
        }
        let session = answer
            .headers()
            .get("location")
            .and_then(|at| at.to_str().ok())
            .map(str::to_owned)
            .ok_or_else(|| {
                Refused::Unreachable("Drive began a save without saying where".into())
            })?;
        let mut answer = self
            .agent
            .put(&session)
            .header("Content-Type", mime)
            .config()
            .timeout_global(Some(DOWNLOAD_TAKES))
            .build()
            .send(file)
            .map_err(|error| unreachable(&error))?;
        let status = answer.status().as_u16();
        if !matches!(status, 200 | 201) {
            return unput(status, &mut answer);
        }
        let body: Value = answer.body_mut().read_json().unwrap_or(Value::Null);
        let text = |field: &str| body.get(field).and_then(Value::as_str).map(str::to_owned);
        Ok(Put::Done {
            id: text("id")
                .ok_or_else(|| Refused::Unreachable("Drive saved without an id".into()))?,
            link: text("webViewLink"),
        })
    }
}
