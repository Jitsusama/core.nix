//! What Joel does to mail through the watch: marking it read, archived or
//! unread again, asking how mail gone from what wants him left, and
//! fetching a file it carries to open or save.

use std::path::{Path, PathBuf};

use jiff::Zoned;

use super::{Looked, Watch};
use crate::api::Refused;
use crate::done::{self, Done, Went};
use crate::files::{self, Failed, Fetched, How, Places};
use crate::mail::{self, Bytes, Mail, Tidy};
use crate::state::{Standing, State};

impl Watch {
    /// Marks the mail `id` read or archives it, as `how` says, and lets it
    /// go from what's held so the shell hears it at once: it wants Joel no
    /// more either way. Held, it was unread in the inbox, so the inbox has
    /// one fewer unread. Mail Google didn't take changes nothing held.
    pub fn tidy(&mut self, id: &str, how: Tidy, now: &Zoned) -> &State {
        match self.asking(now, |google, token| google.unlabel(token, id, how.label())) {
            Ok(()) => {
                if let Some(at) = self.state.mail.iter().position(|mail| mail.id == id) {
                    let gone = self.state.mail.remove(at);
                    self.state.unread = self.state.unread.map(|unread| unread.saturating_sub(1));
                    let done = match how {
                        Tidy::Read => Went::ReadHere,
                        Tidy::Archive => Went::ArchivedHere,
                    };
                    self.finish(gone.clone(), done, now.timestamp().as_millisecond());
                    if how == Tidy::Read {
                        self.read.retain(|mail| mail.id != id);
                        self.read.push(gone);
                    }
                }
            }
            Err(looked) => self.fell(looked, self.state.account),
        }
        &self.state
    }

    /// Marks the mail `id` unread again, as Gmail's own button does, and,
    /// for one marked read here since it was last looked at, holds it as
    /// wanting Joel at once, in its place by when it came. Other mail
    /// comes back as the next look finds it. Mail Google didn't take
    /// changes nothing held.
    pub fn unread(&mut self, id: &str, now: &Zoned) -> &State {
        match self.asking(now, |google, token| google.label(token, id, "UNREAD")) {
            Ok(()) => {
                self.state.finished.retain(|mail| mail.id != id);
                self.departed.retain(|(mail, _)| mail.id != id);
                if let Some(at) = self.read.iter().position(|mail| mail.id == id) {
                    let back = self.read.remove(at);
                    if !self.state.mail.iter().any(|mail| mail.id == id) {
                        self.state.mail.push(back);
                        self.state
                            .mail
                            .sort_by_key(|mail| std::cmp::Reverse(mail.at));
                        self.state.unread = self.state.unread.map(|unread| unread + 1);
                    }
                }
            }
            Err(looked) => self.fell(looked, self.state.account),
        }
        &self.state
    }

    /// Asks Gmail once how each piece of mail gone from what wants Joel
    /// left it, and holds those done as done when they went, which a
    /// mail asked about again after an outage still says (done.rs). Once
    /// Gmail refuses, with a 429 or a 5xx, that mail and the rest aren't
    /// asked about this look but on the next, for as long as a thing done
    /// is told. False where the account fell while asking, which lets
    /// them go with the rest of what it held (`fell`).
    pub(super) fn left(&mut self, left: Vec<(Mail, i64)>, now: &Zoned) -> bool {
        let at = now.timestamp().as_millisecond();
        let mut left = left.into_iter();
        while let Some((mail, went)) = left.next() {
            match self.asking(now, |google, token| google.labels(token, &mail.id)) {
                Ok(labels) => {
                    if let Some(how) = done::left(labels.as_ref()) {
                        self.finish(mail, how, went);
                    }
                }
                Err(Looked::Refused(Refused::Unreachable(why))) => {
                    eprintln!("<4>{why}");
                    self.departed.extend(
                        std::iter::once((mail, went))
                            .chain(left)
                            .filter(|(_, went)| at - went < done::KEPT),
                    );
                    break;
                }
                Err(looked) => {
                    self.fell(looked, Standing::Unreachable);
                    return false;
                }
            }
        }
        true
    }

    /// Holds mail as done, `how` and `at`, newest first, once.
    fn finish(&mut self, mut mail: Mail, how: Went, at: i64) {
        mail.done = Some(Done { how, at });
        self.state.finished.retain(|was| was.id != mail.id);
        self.state.finished.insert(0, mail);
    }

    /// Says the mail `id`'s file `file` is being fetched to open or save,
    /// `to` a path Joel picked when he picked one, which `fetch` then does,
    /// so the shell can say so while it waits.
    pub fn fetching(
        &mut self,
        id: &str,
        file: usize,
        how: How,
        to: Option<String>,
        now: &Zoned,
    ) -> &State {
        self.state.fetched = Some(Fetched {
            mail: id.to_owned(),
            file,
            how,
            to,
            asked: now.timestamp().as_millisecond(),
            path: None,
            failed: None,
        });
        &self.state
    }

    /// Fetches the file `fetching` named, from the mail held, read here
    /// lately or wanting Joel, into `places`: where it is once written, or
    /// why it isn't. A program or a script is never fetched to open.
    pub fn fetch(&mut self, places: &Places, now: &Zoned) -> &State {
        let Some(asked) = self.state.fetched.clone() else {
            return &self.state;
        };
        let found = self.fetched(&asked, places, now);
        self.state.fetched = Some(Fetched {
            path: found.as_ref().ok().map(|path| path.display().to_string()),
            failed: found.err(),
            ..asked
        });
        &self.state
    }

    fn fetched(
        &mut self,
        asked: &Fetched,
        places: &Places,
        now: &Zoned,
    ) -> Result<PathBuf, Failed> {
        let file = self
            .state
            .mail
            .iter()
            .chain(&self.read)
            .find(|mail| mail.id == asked.mail)
            .and_then(|mail| mail.attachments.get(asked.file))
            .cloned()
            .ok_or(Failed::Gone)?;
        if file.runs && asked.how == How::Open {
            return Err(Failed::Runs);
        }
        let data = match &file.bytes {
            Bytes::Held(data) => data.clone(),
            Bytes::Asked(part) => {
                match self.asking(now, |google, token| {
                    google.attachment(token, &asked.mail, part)
                }) {
                    Ok(data) => data,
                    Err(looked) => {
                        self.fell(looked, self.state.account);
                        return Err(Failed::Unreachable);
                    }
                }
            }
        };
        let bytes = mail::bytes(&data);
        let from = &mut bytes.as_slice();
        match asked.how {
            How::Open | How::Pick => files::kept(&places.kept, &asked.mail, &file.name, from),
            How::Save => match &asked.to {
                Some(to) => files::put(Path::new(to), from),
                None => files::saved(&places.saves, &file.name, from),
            },
        }
        .map_err(|error| {
            eprintln!("<4>couldn't write a file mail carries: {error}");
            Failed::Unwritten
        })
    }
}
