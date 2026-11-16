//! What Joel does in Drive through the watch: listing a folder,
//! downloading a file to open, pick or save, and sending a save up.

use std::path::{Path, PathBuf};

use jiff::Zoned;

use super::{Looked, Watch};
use crate::api::{Google, Put, Refused};
use crate::drive::{self, Downloaded, Folder, Kept, Kind, Listed, Unlisted, Unsaved, Uploaded};
use crate::files::{self, Failed, Heard, How, Places};
use crate::saves::{self, Staged};
use crate::state::{Standing, State};

impl Watch {
    /// Lists what the Drive folder `folder` holds, as the shell named it,
    /// or why it couldn't be, first among the `LISTINGS_KEPT` folders
    /// asked lately, its own last listing replaced. Drive failing leaves
    /// the rest held as it stood.
    pub fn list(&mut self, folder: &str, now: &Zoned) -> &State {
        let Some(named) = Folder::named(folder) else {
            return &self.state;
        };
        let found = self.asking(now, |google, token| match named.search() {
            Some(search) => Ok(google
                .folder(token, &search)?
                .map(|answer| drive::entries(&answer))),
            None => Ok(Some(drive::drives(&google.drives(token)?))),
        });
        let (files, more, failed) = match found {
            Ok(Some((files, more))) => (files, more, None),
            Ok(None) => (Vec::new(), false, Some(Unlisted::Gone)),
            Err(looked) => {
                self.fell(looked, self.state.account);
                if matches!(self.state.account, Standing::None | Standing::SignedOut) {
                    return &self.state;
                }
                (Vec::new(), false, Some(Unlisted::Unreachable))
            }
        };
        let listings = &mut self.state.drive.listings;
        listings.retain(|listed| listed.folder != folder);
        listings.insert(
            0,
            Listed {
                folder: folder.to_owned(),
                asked: now.timestamp().as_millisecond(),
                files,
                more,
                failed,
            },
        );
        listings.truncate(drive::LISTINGS_KEPT);
        &self.state
    }

    /// Says the Drive file `file` is being downloaded to open, pick or
    /// save, `to` a path Joel picked when he picked one, which `download`
    /// then does, so the shell can say so while it waits. The last
    /// `DOWNLOADS_KEPT` asked are told, newest first.
    pub fn downloading(&mut self, file: &str, how: How, to: Option<String>, now: &Zoned) -> &State {
        let downloads = &mut self.state.drive.downloads;
        downloads.insert(
            0,
            Downloaded {
                file: file.to_owned(),
                how,
                to,
                asked: now.timestamp().as_millisecond(),
                path: None,
                failed: None,
            },
        );
        downloads.truncate(drive::DOWNLOADS_KEPT);
        &self.state
    }

    /// Weighs the copies of Drive's files already kept, as the program
    /// starts, so Google Drive's drawer says what they take before any is
    /// handed out again.
    pub fn weigh(&mut self, places: &Places) {
        self.state.drive.kept = Kept {
            held: files::held(&places.drive),
            most: files::CACHED_MOST,
        };
    }

    /// Downloads the file `downloading` named last into `places`: where
    /// it is once written, or why it isn't. A program or a script is never
    /// downloaded to open, and one of Google's own kinds comes as the
    /// office program's file. One to open or hand on is fetched only when
    /// Drive has changed it since the copy kept, which Drive is asked
    /// first.
    pub fn download(&mut self, places: &Places, now: &Zoned) -> &State {
        let Some(asked) = self.state.drive.downloads.first().cloned() else {
            return &self.state;
        };
        let found = self.downloaded(&asked, places, now);
        if let Some(first) = self.state.drive.downloads.first_mut() {
            first.path = found.as_ref().ok().map(|path| path.display().to_string());
            first.failed = found.err();
        }
        &self.state
    }

    fn downloaded(
        &mut self,
        asked: &Downloaded,
        places: &Places,
        now: &Zoned,
    ) -> Result<PathBuf, Failed> {
        let file = match self.asking(now, |google, token| google.file(token, &asked.file)) {
            Ok(file) => file.as_ref().and_then(drive::entry).ok_or(Failed::Gone)?,
            Err(looked) => {
                self.fell(looked, self.state.account);
                return Err(Failed::Unreachable);
            }
        };
        if file.runs && asked.how == How::Open {
            return Err(Failed::Runs);
        }
        let (exported, name) = match file.kind.google().map(Kind::export) {
            Some((kind, ending)) => (Some(kind), format!("{}{ending}", file.name)),
            None if file.kind == drive::Is::Folder => return Err(Failed::Gone),
            None => (None, file.name.clone()),
        };
        // A copy of the version Drive holds is handed out again as it is.
        if asked.how != How::Save
            && let Some(path) = files::fresh(&places.drive, &file.id, &name, file.changed)
        {
            return Ok(path);
        }
        let body = match self.asking(now, |google, token| {
            google.content(token, &file.id, exported)
        }) {
            Ok(body) => body.ok_or(Failed::Gone)?,
            Err(looked) => {
                self.fell(looked, self.state.account);
                return Err(Failed::Unreachable);
            }
        };
        let mut heard = Heard::new(body);
        match asked.how {
            How::Open | How::Pick => {
                let path = files::cached(&places.drive, &file.id, &name, file.changed, &mut heard);
                let held = files::pruned(&places.drive, files::CACHED_MOST, &file.id);
                self.state.drive.kept = Kept {
                    held,
                    most: files::CACHED_MOST,
                };
                path
            }
            How::Save => match &asked.to {
                Some(to) => files::put(Path::new(to), &mut heard),
                None => files::saved(&places.saves, &name, &mut heard),
            },
        }
        .map_err(|error| {
            if heard.fell {
                eprintln!("<4>Drive broke off a file: {error}");
                Failed::Unreachable
            } else {
                eprintln!("<4>couldn't write a Drive file: {error}");
                Failed::Unwritten
            }
        })
    }

    /// Says the file staged at `staged` is going up to Drive, which
    /// `upload` then does, so the shell can say so while it goes. The last
    /// `UPLOADS_KEPT` sent are told, newest first.
    pub fn uploading(&mut self, staged: &Staged, now: &Zoned) -> &State {
        let uploads = &mut self.state.drive.uploads;
        uploads.insert(
            0,
            Uploaded {
                path: staged.path.display().to_string(),
                folder: staged.folder.clone(),
                name: staged.name.clone(),
                asked: now.timestamp().as_millisecond(),
                id: None,
                link: None,
                failed: None,
                from: saves::from(&staged.path),
            },
        );
        uploads.truncate(drive::UPLOADS_KEPT);
        &self.state
    }

    /// Sends up the file `uploading` named last: a new version of the
    /// file it went up as before, or of the one of that name its folder
    /// held when last listed, as the picker's Replace It means, and else a
    /// new file. Once it's up, its id is kept beside it.
    pub fn upload(&mut self, now: &Zoned) -> &State {
        let Some(asked) = self.state.drive.uploads.first().cloned() else {
            return &self.state;
        };
        let path = PathBuf::from(&asked.path);
        let found = self.uploaded(&asked, &path, now);
        if let Ok((id, _)) = &found
            && let Err(error) = saves::keep(&path, id)
        {
            eprintln!("<4>couldn't mark a Drive save as sent: {error}");
        }
        if let Some(first) = self.state.drive.uploads.first_mut() {
            match found {
                Ok((id, link)) => {
                    first.id = Some(id);
                    first.link = link;
                }
                Err(failed) => first.failed = Some(failed),
            }
        }
        &self.state
    }

    fn uploaded(
        &mut self,
        asked: &Uploaded,
        path: &Path,
        now: &Zoned,
    ) -> Result<(String, Option<String>), Unsaved> {
        let listed = self
            .state
            .drive
            .listings
            .iter()
            .find(|listed| listed.folder == asked.folder)
            .and_then(|listed| {
                listed
                    .files
                    .iter()
                    .find(|file| file.name == asked.name && file.kind != drive::Is::Folder)
            })
            .map(|file| file.id.clone());
        let before = saves::sent(path).or(listed);
        let put = match before {
            Some(id) => match self.sent(now, |google, token| {
                google.replace(token, &id, &asked.name, path)
            })? {
                Put::NotGranted => return Err(Unsaved::ReplaceNotGranted),
                // Gone from Drive since: it goes up anew.
                Put::Gone => None,
                put => Some(put),
            },
            None => None,
        };
        let put = match put {
            Some(put) => put,
            None => self.sent(now, |google, token| {
                google.create(token, &asked.folder, &asked.name, path)
            })?,
        };
        match put {
            Put::Done { id, link } => Ok((id, link)),
            Put::Gone | Put::NotGranted => Err(Unsaved::Gone),
            Put::Unread => Err(Unsaved::Unread),
        }
    }

    /// Asks as `asking` does, saying why a save didn't go up as a save's
    /// failure.
    fn sent(
        &mut self,
        now: &Zoned,
        ask: impl Fn(&Google, &str) -> Result<Put, Refused>,
    ) -> Result<Put, Unsaved> {
        self.asking(now, ask).map_err(|looked| {
            let failed = match looked {
                Looked::Refused(Refused::Unreachable(_)) => Unsaved::Unreachable,
                _ => Unsaved::SignedOut,
            };
            self.fell(looked, self.state.account);
            failed
        })
    }
}
