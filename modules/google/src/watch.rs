//! The account as it stands: one look reads who's signed in, gets a token,
//! asks for the meetings and the mail that wants Joel, and every few
//! minutes the documents he looked at lately and how each series of
//! meetings repeats, and says how it all stands; an answer to an
//! invitation, a meeting cancelled or moved, and mail marked read or
//! archived, are asked the same way, and change what's held at once. A
//! Drive folder is listed, and a file downloaded, only when the shell
//! asks.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use jiff::{Timestamp, Zoned};
use serde_json::Value;

use crate::account::{self, Missing};
use crate::api::{Google, MAKES_MEETINGS, Put, Refused};
use crate::calendar::{self, Answer};
use crate::drive::{self, Document, Downloaded, Folder, Kind, Listed, Unlisted, Unsaved, Uploaded};
use crate::files::{self, Failed, Fetched, Heard, How, Places};
use crate::mail::{self, Bytes, Mail, Tidy};
use crate::saves::{self, Staged};
use crate::state::{Made, Standing, State};
use crate::token::Keeper;

pub struct Watch {
    pub google: Google,
    pub credentials: PathBuf,
    /// The account to ask as, or the file's default.
    pub account: Option<String>,
    keeper: Keeper,
    state: State,
    /// The calendars Joel may change events on, as last heard.
    writable: Vec<String>,
    /// The mail marked read since it was last looked at, so marking one
    /// unread again brings it back at once, before Gmail lists it again.
    read: Vec<Mail>,
    /// When the documents were last heard, in milliseconds since the
    /// epoch, or none since the account was last gone.
    documented: Option<i64>,
    /// How each series of meetings repeats, by its id, as its rule's lines
    /// were last heard, and when.
    rules: HashMap<String, (Vec<String>, i64)>,
}

/// How long the documents are kept before Drive is asked again: what Joel
/// looked at lately moves when he opens one, seldom within minutes, and
/// it's three asks each time.
const DOCUMENTS_EVERY: i64 = 5 * 60 * 1000;

/// How long a series' rule is kept before Google is asked again: one
/// seldom changes, and it's an ask for each series.
const RULES_EVERY: i64 = DOCUMENTS_EVERY;

impl Watch {
    pub fn new(google: Google, credentials: PathBuf, account: Option<String>) -> Watch {
        Watch {
            google,
            credentials,
            account,
            keeper: Keeper::default(),
            state: State::default(),
            writable: Vec::new(),
            read: Vec::new(),
            documented: None,
            rules: HashMap::new(),
        }
    }

    /// How the account stands at `now`, asking Google if it's signed in.
    /// Unreachable, what was held is kept, with when it was heard. The
    /// mail is asked after the meetings, so Gmail failing alone keeps the
    /// mail held and costs the meetings nothing. Which calendars Joel may
    /// change events on is asked with the meetings, and kept as last heard
    /// when only that can't be.
    pub fn look(&mut self, now: &Zoned) -> &State {
        let (from, to) = calendar::window(now);
        match self.asking(now, |google, token| google.events(token, from, to)) {
            Ok(mut answer) => {
                match self.asking(now, Google::writable) {
                    Ok(writable) => self.writable = writable,
                    Err(Looked::Refused(Refused::Unreachable(why))) => eprintln!("<4>{why}"),
                    Err(looked) => {
                        self.fell(looked, Standing::Unreachable);
                        return &self.state;
                    }
                }
                answer["writable"] = Value::from(self.writable.clone());
                answer["series"] = self.ruled(&answer, now);
                self.state.account = Standing::Ready;
                self.state.meet.granted = self.makes_meetings();
                self.state.meetings = calendar::meetings(&answer, now.time_zone());
                self.state.heard = Some(now.timestamp().as_millisecond());
            }
            Err(looked) => {
                self.fell(looked, Standing::Unreachable);
                return &self.state;
            }
        }
        let held = std::mem::take(&mut self.state.mail);
        match self.asking(now, |google, token| wanted(google, token, &held)) {
            Ok((found, unread)) => {
                self.state.mail = found;
                self.state.unread = unread;
                // What Gmail lists now says what's unread, any marked
                // unread again from elsewhere included.
                self.read.clear();
            }
            Err(Looked::Refused(Refused::Unreachable(why))) => {
                eprintln!("<4>{why}");
                self.state.mail = held;
            }
            Err(looked) => {
                self.fell(looked, Standing::Unreachable);
                return &self.state;
            }
        }
        let at = now.timestamp().as_millisecond();
        if self
            .documented
            .is_none_or(|heard| at - heard >= DOCUMENTS_EVERY)
        {
            match self.asking(now, recently) {
                Ok(found) => {
                    self.state.documents = found;
                    self.documented = Some(at);
                }
                // Drive failing alone keeps the documents held, and costs
                // the meetings and the mail nothing.
                Err(Looked::Refused(Refused::Unreachable(why))) => eprintln!("<4>{why}"),
                Err(looked) => self.fell(looked, Standing::Unreachable),
            }
        }
        &self.state
    }

    /// The rule each series in the calendar's answer repeats by, by its
    /// id, asking Google only of one not heard in `RULES_EVERY`. One that
    /// can't be heard now keeps what was heard of it, and one heard of
    /// never says nothing, for now; neither costs the meetings anything.
    fn ruled(&mut self, answer: &Value, now: &Zoned) -> Value {
        let at = now.timestamp().as_millisecond();
        let series = calendar::repeating(answer);
        self.rules.retain(|id, _| series.contains(id));
        for id in series {
            if self
                .rules
                .get(&id)
                .is_some_and(|(_, heard)| at - heard < RULES_EVERY)
            {
                continue;
            }
            match self.asking(now, |google, token| google.recurrence(token, &id)) {
                Ok(lines) => {
                    self.rules.insert(id, (lines, at));
                }
                Err(Looked::Refused(Refused::Unreachable(why))) => eprintln!("<4>{why}"),
                // Signed out since the meetings were heard: the next look
                // says so.
                Err(_) => break,
            }
        }
        self.rules
            .iter()
            .map(|(id, (lines, _))| (id.clone(), Value::from(lines.clone())))
            .collect::<serde_json::Map<_, _>>()
            .into()
    }

    /// Answers the invitation to the meeting `id` as `with`, and holds the
    /// meeting as answered so the shell hears it at once, before the next
    /// look says it again. An answer Google didn't take changes nothing
    /// held, and Google still answering the looks is still ready.
    pub fn answer(&mut self, id: &str, with: Answer, now: &Zoned) -> &State {
        match self.asking(now, |google, token| google.answer(token, id, with)) {
            Ok(()) => {
                for meeting in &mut self.state.meetings {
                    if meeting.id == id {
                        meeting.answer = Some(with);
                    }
                }
            }
            Err(looked) => self.fell(looked, self.state.account),
        }
        &self.state
    }

    /// Cancels the meeting `id` and lets it go from what's held, so the
    /// shell hears it at once. Only a meeting held as Joel's is cancelled:
    /// deleting one he's only asked to would take it off his calendar alone
    /// and tell nobody, which isn't cancelling it. One Google didn't take
    /// changes nothing held.
    pub fn cancel(&mut self, id: &str, now: &Zoned) -> &State {
        let his = self
            .state
            .meetings
            .iter()
            .any(|meeting| meeting.id == id && meeting.holds);
        if !his {
            eprintln!("<4>a meeting asked to be cancelled isn't one Joel holds");
            return &self.state;
        }
        match self.asking(now, |google, token| google.cancel(token, id)) {
            Ok(()) => self.state.meetings.retain(|meeting| meeting.id != id),
            Err(looked) => self.fell(looked, self.state.account),
        }
        &self.state
    }

    /// Moves the meeting `id` to run from `start` to `end`, through the
    /// calendar Google lets Joel change it on, and holds it moved so the
    /// shell hears it at once. Only a meeting held as one he may change is
    /// moved, and never one lasting all day, whose times are days. One
    /// Google didn't take changes nothing held.
    pub fn reschedule(&mut self, id: &str, start: i64, end: i64, now: &Zoned) -> &State {
        let through = self
            .state
            .meetings
            .iter()
            .find(|meeting| meeting.id == id && !meeting.all_day)
            .and_then(|meeting| meeting.through.clone());
        let (Some(through), Ok(from), Ok(to)) = (
            through,
            Timestamp::from_millisecond(start),
            Timestamp::from_millisecond(end),
        ) else {
            eprintln!("<4>a meeting asked to be moved isn't one Joel may change");
            return &self.state;
        };
        let moved = self.asking(now, |google, token| {
            google.reschedule(token, &through, id, from, to)
        });
        match moved {
            Ok(()) => {
                for meeting in &mut self.state.meetings {
                    if meeting.id == id {
                        meeting.start = start;
                        meeting.end = end;
                    }
                }
                self.state
                    .meetings
                    .sort_by_key(|meeting| (meeting.start, meeting.end));
            }
            Err(looked) => self.fell(looked, self.state.account),
        }
        &self.state
    }

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

    /// Whether Joel's sign-in lets this program make a Meet meeting, read
    /// afresh, since signing in again from pi widens it.
    fn makes_meetings(&self) -> bool {
        account::read(&self.credentials, self.account.as_deref())
            .is_ok_and(|account| account.grants(MAKES_MEETINGS))
    }

    /// Says a new Meet meeting is being made, which `make` then does, so
    /// the shell can say so while Google answers.
    pub fn making(&mut self, now: &Zoned) -> &State {
        self.state.meet.made = Some(Made {
            asked: now.timestamp().as_millisecond(),
            link: None,
            failed: false,
        });
        &self.state
    }

    /// Makes the meeting `making` asked for: its link once Google makes
    /// it, or that it couldn't be. A sign-in without Meet's scope isn't
    /// asked of Google at all. Signed out while asking, nothing is held,
    /// and nothing made.
    pub fn make(&mut self, now: &Zoned) -> &State {
        if self.state.meet.made.is_none() {
            return &self.state;
        }
        self.state.meet.granted = self.makes_meetings();
        let link = if self.state.meet.granted {
            match self.asking(now, Google::space) {
                Ok(link) => Some(link),
                Err(looked) => {
                    self.fell(looked, self.state.account);
                    None
                }
            }
        } else {
            None
        };
        if let Some(made) = &mut self.state.meet.made {
            made.failed = link.is_none();
            made.link = link;
        }
        &self.state
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

    /// Downloads the file `downloading` named last into `places`: where
    /// it is once written, or why it isn't. A program or a script is never
    /// downloaded to open, and one of Google's own kinds comes as the
    /// office program's file.
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
                files::kept(&places.kept.join("drive"), &file.id, &name, &mut heard)
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

    /// What's held once Google didn't answer: nothing, for an account gone
    /// or signed out, since it may not be Joel's to see any more; what was
    /// held, standing as `unreachable` says, when Google couldn't be
    /// reached.
    fn fell(&mut self, looked: Looked, unreachable: Standing) {
        // What Joel saved stays told, being his own files rather than the
        // account's, so the shell can say why one didn't go up.
        let uploads = std::mem::take(&mut self.state.drive.uploads);
        match looked {
            Looked::Missing => {
                self.state = State::without(Standing::None);
                self.documented = None;
            }
            Looked::Refused(Refused::SignedOut) => {
                self.state = State::without(Standing::SignedOut);
                self.documented = None;
            }
            Looked::Refused(Refused::Unreachable(why)) => {
                eprintln!("<4>{why}");
                self.state.account = unreachable;
            }
        }
        self.state.drive.uploads = uploads;
    }

    /// Asks Google as the account, with a token it'll take.
    fn asking<T>(
        &mut self,
        now: &Zoned,
        ask: impl Fn(&Google, &str) -> Result<T, Refused>,
    ) -> Result<T, Looked> {
        let account = account::read(&self.credentials, self.account.as_deref())
            .map_err(|_: Missing| Looked::Missing)?;
        let at = now.timestamp().as_millisecond();
        let google = &self.google;
        let token = self
            .keeper
            .token(&account, at, |account| google.refresh(account))?;
        match ask(google, &token) {
            // A token that looked fresh but wasn't, as one revoked is:
            // once more, with a new one.
            Err(Refused::SignedOut) => {
                self.keeper.spurn(&token);
                let token = self
                    .keeper
                    .token(&account, at, |account| google.refresh(account))?;
                let again = ask(google, &token);
                // Refused with a token just made, the sign-in itself is no
                // good for the calendar: it waits for a new one.
                if matches!(again, Err(Refused::SignedOut)) {
                    self.keeper.refuse(&account);
                }
                Ok(again?)
            }
            answer => Ok(answer?),
        }
    }
}

/// The documents of every kind Joel looked at lately, each kind's last
/// first.
fn recently(google: &Google, token: &str) -> Result<Vec<Document>, Refused> {
    let mut found = Vec::new();
    for kind in Kind::ALL {
        found.extend(drive::documents(&google.documents(token, kind)?, kind));
    }
    Ok(found)
}

/// The mail that wants Joel, newest first, and how many in the inbox are
/// unread. Mail already held isn't asked for again, since what Gmail
/// lists of it never changes.
fn wanted(
    google: &Google,
    token: &str,
    held: &[Mail],
) -> Result<(Vec<Mail>, Option<u64>), Refused> {
    let mut found = Vec::new();
    for id in mail::listed(&google.wanting(token)?) {
        if let Some(kept) = held.iter().find(|kept| kept.id == id) {
            found.push(kept.clone());
            continue;
        }
        match google.mail(token, &id) {
            Ok(answer) => found.extend(mail::read(&answer)),
            // Gone between the list and the ask, as mail archived
            // elsewhere is; the next look lists it again if it's not.
            Err(Refused::Unreachable(_)) => {}
            Err(refused) => return Err(refused),
        }
    }
    found.sort_by_key(|mail| std::cmp::Reverse(mail.at));
    Ok((found, mail::unread(&google.inbox(token)?)))
}

enum Looked {
    Missing,
    Refused(Refused),
}

impl From<Refused> for Looked {
    fn from(refused: Refused) -> Looked {
        Looked::Refused(refused)
    }
}
