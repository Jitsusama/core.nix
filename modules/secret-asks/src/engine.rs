//! What every daemon that answers asks shares, whatever it may release
//! and wherever its yes comes from: the asks folder watched, each ask it
//! may serve held as a row, a yes and a no answered, the value read and
//! the command run on a yes, and each ask's answer written beside it.
//!
//! What may be released, how the value is read, and the command it runs
//! in are the [`Releaser`]'s, so the experiment's daemon (daemon.rs) and
//! the passwords daemon differ there alone.
//!
//! A yes is refused unless the ask still waits, the session is unlocked,
//! the ask and its tree read as the digest the yes carries says, and the
//! line was on screen long enough to read. A yes taken leaves a mark
//! beside the ask until it's answered, so an ask a restart cut off while
//! it ran is ended rather than released twice.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use daemon_serve::asks::error;
use inotify::{Inotify, WatchMask};
use serde_json::{Value, json};

use crate::ask::{self, Ask};
use crate::mask::{MOST_BYTES, masked};
use crate::reading::{self, Outcome};
use crate::row::{self, Held, STAYS_MS, State};
use crate::tree;
use crate::words::Words;

/// The most an ask's file may hold, so a huge one is no ask.
const MOST_ASK_BYTES: u64 = 64 * 1024;
/// How often a running command is looked at for having ended, since a
/// child's end can't be waited on with a time limit in std.
const LOOKS_EVERY: Duration = Duration::from_millis(20);
/// The least time a line must have been on screen for a yes to it to be
/// a yes to what it said.
pub const LEAST_SHOWN_MS: i64 = 500;
/// The most of a command's output kept to be masked: twice what an answer
/// carries, so a value printed where the answer is cut is still masked.
const MOST_READ_BYTES: usize = 2 * MOST_BYTES;
/// How long output is waited for once its command has ended, since
/// something it started may hold the pipe open long after.
const OUTPUT_LINGERS: Duration = Duration::from_secs(1);

/// Milliseconds since 1970.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_millis()).unwrap_or(i64::MAX)
        })
}

/// Writes a file whole, 0600, by a name of its own renamed into place, so
/// no reader ever sees half of it.
///
/// # Errors
///
/// Where the folder can't be made or the file written.
pub fn write_whole(path: &Path, said: &Value) -> std::io::Result<()> {
    let folder = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(folder)?;
    std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o700))?;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    let writing = folder.join(format!(".{}.writing", name.unwrap_or_default()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&writing)?;
    file.write_all(said.to_string().as_bytes())?;
    file.sync_all()?;
    std::fs::rename(&writing, path)
}

/// Whether the session in front, the account's display, is locked, as
/// logind says through `loginctl`; one logind can't say is taken as
/// locked, so no yes is taken where nobody can tell.
pub fn session_locked(loginctl: &Path) -> bool {
    let asked = |args: &[&str]| {
        Command::new(loginctl)
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()
            .filter(|ran| ran.status.success())
            .map(|ran| String::from_utf8_lossy(&ran.stdout).trim().to_owned())
    };
    let uid = std::fs::metadata("/proc/self").map(|meta| {
        use std::os::unix::fs::MetadataExt;
        meta.uid().to_string()
    });
    let Ok(uid) = uid else { return true };
    let Some(display) = asked(&["show-user", &uid, "--property=Display", "--value"]) else {
        return true;
    };
    let hint = asked(&["show-session", &display, "--property=LockedHint", "--value"]);
    hint.as_deref() != Some("no")
}

/// A value read to be released, its bytes zeroed once it's let go.
pub struct Secret(String);

impl Secret {
    pub fn new(value: String) -> Secret {
        Secret(value)
    }

    /// The value, for the command it's released to alone.
    pub fn reveal(&self) -> &str {
        &self.0
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        // SAFETY: zeroes are valid UTF-8, so the string stays a string.
        unsafe { self.0.as_bytes_mut() }.fill(0);
    }
}

/// What a daemon decides of its own: what it may release, whether the
/// session is locked, how a value is read, and what command it runs in.
pub trait Releaser: Send + Sync {
    /// Why an ask may not be served at all, where it may not; it's then
    /// answered refused, unseen.
    fn refused(&self, ask: &Ask) -> Option<String>;
    /// Whether the session in front is locked, when no yes is taken.
    fn locked(&self) -> bool;
    /// The value an ask names, read once Joel has said yes.
    ///
    /// # Errors
    ///
    /// Where it can't be read, saying why.
    fn read_value(&self, ask: &Ask) -> Result<Secret, String>;
    /// The command an ask runs, given the value; its standard input,
    /// output and error are the engine's to set.
    ///
    /// # Errors
    ///
    /// Where it can't be made, saying why.
    fn command(&self, ask: &Ask, value: &Secret) -> Result<Command, String>;
}

/// Where the engine works, and what it says things with.
pub struct Setting {
    /// The folder asks are put up in.
    pub asks: PathBuf,
    pub git: PathBuf,
    /// Joel's home, which a row shortens a tree's path by.
    pub home: String,
    pub words: Words,
    /// Where journald takes the readings' lines.
    pub journal: PathBuf,
}

/// The asks held, by id, and what to tell when they change.
pub struct Engine {
    setting: Setting,
    releaser: Box<dyn Releaser>,
    asks: Mutex<BTreeMap<String, Held>>,
    changed: Mutex<Sender<()>>,
}

impl Engine {
    /// An engine holding no asks yet, which tells `changed` each time its
    /// rows may have moved.
    pub fn new(setting: Setting, releaser: Box<dyn Releaser>, changed: Sender<()>) -> Engine {
        Engine {
            setting,
            releaser,
            asks: Mutex::new(BTreeMap::new()),
            changed: Mutex::new(changed),
        }
    }

    /// Makes the asks folder its owner's alone, ends every ask a restart
    /// cut off while it ran, and watches the folder.
    ///
    /// # Errors
    ///
    /// Where the folder can't be made or watched.
    pub fn start(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.setting.asks)?;
        std::fs::set_permissions(&self.setting.asks, std::fs::Permissions::from_mode(0o700))?;
        for id in self.marked() {
            if self.answer_path(&id).exists() {
                // Answered, but cut off before its mark was taken away.
                self.forget(&id);
            } else {
                self.answered(&id, "ended", "ended by a restart");
            }
        }
        let changed = self
            .changed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        watch_folder(&self.setting.asks, changed)
    }

    fn held(&self) -> MutexGuard<'_, BTreeMap<String, Held>> {
        self.asks.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn changed(&self) {
        let _ = self
            .changed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .send(());
    }

    fn ask_path(&self, id: &str) -> PathBuf {
        self.setting.asks.join(format!("{id}.ask.json"))
    }

    fn answer_path(&self, id: &str) -> PathBuf {
        self.setting.asks.join(format!("{id}.answer.json"))
    }

    /// The mark left beside an ask while it's released.
    fn mark_path(&self, id: &str) -> PathBuf {
        self.setting.asks.join(format!("{id}.releasing"))
    }

    /// The id of every ask marked as being released.
    fn marked(&self) -> Vec<String> {
        std::fs::read_dir(&self.setting.asks)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter_map(|entry| {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        let id = name.strip_suffix(".releasing")?;
                        ask::id_of(&format!("{id}.ask.json")).map(str::to_owned)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Takes an ask's file and its mark away.
    fn forget(&self, id: &str) {
        let _ = std::fs::remove_file(self.ask_path(id));
        let _ = std::fs::remove_file(self.mark_path(id));
    }

    /// Writes an ask's answer and takes its file and mark away.
    fn answer(&self, id: &str, said: &Value) {
        if let Err(why) = write_whole(&self.answer_path(id), said) {
            eprintln!("<3>secret-asks: can't answer {id}: {why}");
        }
        self.forget(id);
    }

    /// Tells the journal how an ask ended, for the machine's health.
    fn reading(&self, ask: &Ask, outcome: Outcome, said: &Value) {
        reading::send(
            &self.setting.journal,
            &reading::line(ask, outcome, said["shown"].as_i64()),
        );
    }

    /// Answers an ask that ended without running, saying why.
    fn answered(&self, id: &str, state: &str, why: &str) {
        eprintln!("<6>secret-asks: {id} {state}: {why}");
        self.answer(
            id,
            &json!({"version": ask::VERSION, "id": id, "state": state, "why": why}),
        );
    }

    /// The ask a file holds as it stands now, or why it's none.
    fn read(&self, id: &str) -> Result<Ask, String> {
        let mut text = String::new();
        std::fs::File::open(self.ask_path(id))
            .map_err(|why| why.to_string())?
            .take(MOST_ASK_BYTES + 1)
            .read_to_string(&mut text)
            .map_err(|why| why.to_string())?;
        if text.len() as u64 > MOST_ASK_BYTES {
            return Err("its file is too big".to_owned());
        }
        Ask::read(id, &text)
    }

    /// Takes in every ask file not yet held, and lets go of each waiting
    /// ask whose file was taken away.
    pub fn look(&self) {
        let ids: Vec<String> = std::fs::read_dir(&self.setting.asks)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter_map(|entry| {
                        ask::id_of(&entry.file_name().to_string_lossy()).map(str::to_owned)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let now = now_ms();
        let mut held = self.held();
        held.retain(|id, one| one.state != State::Waiting || ids.contains(id));
        for id in ids {
            if held.contains_key(&id) {
                continue;
            }
            match self.read(&id) {
                Err(why) => self.answered(&id, "refused", &format!("not an ask: {why}")),
                Ok(ask) => {
                    if let Some(why) = self.releaser.refused(&ask) {
                        self.reading(&ask, Outcome::Unseen, &Value::Null);
                        self.answered(&id, "refused", &why);
                    } else if !ask.waits_at(now) {
                        self.reading(&ask, Outcome::Expired, &Value::Null);
                        self.answered(&id, "ended", "ended unanswered");
                    } else {
                        let tree = tree::read(&self.setting.git, Path::new(&ask.tree));
                        eprintln!(
                            "<6>secret-asks: {id} asked by {} for {} to run {:?} in {}",
                            ask.session, ask.item, ask.argv, ask.tree
                        );
                        held.insert(
                            id,
                            Held {
                                ask,
                                tree,
                                state: State::Waiting,
                                since_ms: now,
                            },
                        );
                    }
                }
            }
        }
    }

    /// Ends each ask past its ten minutes, and lets go of each row
    /// answered long enough ago; answers when next to look again.
    pub fn tidy(&self) -> Option<i64> {
        let now = now_ms();
        let mut held = self.held();
        let ended: Vec<Ask> = held
            .values()
            .filter(|one| one.state == State::Waiting && !one.ask.waits_at(now))
            .map(|one| one.ask.clone())
            .collect();
        for ask in ended {
            if let Some(one) = held.get_mut(&ask.id) {
                one.state = State::Ended;
                one.since_ms = now;
            }
            self.reading(&ask, Outcome::Expired, &Value::Null);
            self.answered(&ask.id, "ended", "ended unanswered");
        }
        held.retain(|_, one| {
            matches!(one.state, State::Waiting | State::Releasing) || now < one.since_ms + STAYS_MS
        });
        held.values()
            .filter_map(|one| match one.state {
                State::Waiting => Some(one.ask.ends_ms()),
                State::Releasing => None,
                _ => Some(one.since_ms + STAYS_MS),
            })
            .min()
    }

    /// A refusal the launcher can say to Joel: its code and message for
    /// whoever reads the answer, and `said`, the asks thing's words by
    /// `word`, which the launcher shows in place of "couldn't".
    fn refusal(&self, code: &str, message: &str, word: &str) -> Value {
        let mut refused = error(&Value::Null, code, message);
        refused["error"]["said"] = Value::String(self.setting.words.fill(word, &[]));
        refused
    }

    /// The launcher's release: refused unless the ask still waits, the
    /// session is unlocked, the ask and its tree read as the digest said,
    /// and the line was on screen long enough to read; then the value is
    /// read and the command run on a thread of its own, so the answer
    /// comes at once.
    pub fn release(self: &Arc<Self>, said: &Value) -> Value {
        let Some(id) = said["key"]
            .as_str()
            .and_then(|key| key.strip_prefix("ask:"))
        else {
            return self.refusal("refused", "no such ask", "gone");
        };
        let id = id.to_owned();
        let mut held = self.held();
        let Some(one) = held.get_mut(&id) else {
            return self.refusal("refused", "no such ask", "gone");
        };
        if one.state != State::Waiting {
            return self.refusal("refused", "it's already answered", "answered");
        }
        if self.releaser.locked() {
            self.reading(&one.ask, Outcome::Locked, said);
            return self.refusal("refused", "the screen is locked", "locked");
        }
        let now = match self.read(&id) {
            Ok(ask) if self.releaser.refused(&ask).is_none() => ask,
            _ => return self.refusal("refused", "the ask is gone", "gone"),
        };
        let tree = tree::read(&self.setting.git, Path::new(&now.tree));
        if Some(row::digest(&now, &tree).as_str()) != said["digest"].as_str() {
            self.reading(&one.ask, Outcome::Changed, said);
            one.ask = now;
            one.tree = tree;
            drop(held);
            self.changed();
            return self.refusal(
                "changed",
                "the ask changed after it was drawn; read it again",
                "changed",
            );
        }
        // A yes before the line could be read is no yes to what it said,
        // and the ask waits on for one that is.
        if said["shown"]
            .as_i64()
            .is_none_or(|shown| shown < LEAST_SHOWN_MS)
        {
            return self.refusal("refused", "the line wasn't shown long enough", "unread");
        }
        if let Err(why) = std::fs::write(self.mark_path(&id), "") {
            eprintln!("<3>secret-asks: can't mark {id} as releasing: {why}");
            let mut refused = error(&Value::Null, "failed", "its mark can't be left");
            refused["error"]["said"] = Value::String(
                self.setting
                    .words
                    .fill("failed", &[("why", "its mark can't be left")]),
            );
            return refused;
        }
        one.state = State::Releasing;
        one.since_ms = now_ms();
        let ask = one.ask.clone();
        drop(held);
        self.reading(&ask, Outcome::Released, said);
        self.changed();
        let engine = Arc::clone(self);
        std::thread::spawn(move || engine.run_released(&ask));
        json!({"releasing": true})
    }

    /// Reads the value and runs the command with it, then says what came
    /// of it.
    fn run_released(&self, ask: &Ask) {
        let started = now_ms();
        let ran = self
            .releaser
            .read_value(ask)
            .and_then(|value| self.ran(ask, &value));
        let (state, mut said) = match ran {
            Err(why) => (
                State::Failed { why: why.clone() },
                json!({"state": "failed", "why": why}),
            ),
            Ok((exit, output, cut)) => (
                State::Ran { exit },
                json!({"state": "ran", "exit": exit, "output": output, "cut": cut,
                       "why": if exit.is_none() { "stopped at the time limit" } else { "" }}),
            ),
        };
        said["version"] = json!(ask::VERSION);
        said["id"] = json!(ask.id);
        said["started"] = json!(started);
        said["ended"] = json!(now_ms());
        eprintln!(
            "<6>secret-asks: {} {} {}",
            ask.id,
            state.name(),
            said["exit"]
        );
        self.answer(&ask.id, &said);
        if let Some(one) = self.held().get_mut(&ask.id) {
            one.state = state;
            one.since_ms = now_ms();
        }
        self.changed();
    }

    /// Runs an ask's command under its time limit, its output and errors
    /// on one pipe read as it runs, so a command that says much can't
    /// stall on a full pipe; answers its exit (none where it was stopped)
    /// and its output, masked.
    fn ran(&self, ask: &Ask, value: &Secret) -> Result<(Option<i32>, String, bool), String> {
        let mut command = self.releaser.command(ask, value)?;
        let (mut reader, writer) =
            std::io::pipe().map_err(|why| format!("couldn't make its pipe: {why}"))?;
        let errors = writer
            .try_clone()
            .map_err(|why| format!("couldn't make its pipe: {why}"))?;
        command.stdin(Stdio::null()).stdout(writer).stderr(errors);
        let mut child = command
            .spawn()
            .map_err(|why| format!("couldn't run its command: {why}"))?;
        // The command keeps its own ends of the pipe; these let go of ours,
        // so the pipe closes once the command and what it started end.
        drop(command);
        let read = Arc::new(Mutex::new((Vec::new(), false)));
        let (done, finished) = mpsc::channel();
        {
            let read = Arc::clone(&read);
            std::thread::spawn(move || {
                let mut chunk = [0_u8; 8192];
                while let Ok(length) = reader.read(&mut chunk) {
                    if length == 0 {
                        break;
                    }
                    let mut kept = read.lock().unwrap_or_else(PoisonError::into_inner);
                    let room = MOST_READ_BYTES.saturating_sub(kept.0.len());
                    kept.0.extend_from_slice(&chunk[..length.min(room)]);
                    kept.1 |= length > room;
                }
                let _ = done.send(());
            });
        }
        let until = Instant::now() + Duration::from_secs(ask.minutes * 60);
        let exit = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status.code(),
                Ok(None) if Instant::now() < until => std::thread::sleep(LOOKS_EVERY),
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
            }
        };
        let _ = finished.recv_timeout(OUTPUT_LINGERS);
        let kept = read.lock().unwrap_or_else(PoisonError::into_inner);
        let (said, cut) = masked(&String::from_utf8_lossy(&kept.0), value.reveal());
        Ok((exit, said, cut || kept.1))
    }

    /// The launcher's refuse: the ask answered refused, if it still waits.
    pub fn refuse(&self, said: &Value) -> Value {
        let Some(id) = said["key"]
            .as_str()
            .and_then(|key| key.strip_prefix("ask:"))
        else {
            return self.refusal("refused", "no such ask", "gone");
        };
        let mut held = self.held();
        let Some(one) = held.get_mut(id).filter(|one| one.state == State::Waiting) else {
            return self.refusal("refused", "it's already answered", "answered");
        };
        one.state = State::Refused;
        one.since_ms = now_ms();
        let id = id.to_owned();
        let ask = one.ask.clone();
        drop(held);
        self.reading(&ask, Outcome::Refused, said);
        self.answered(&id, "refused", "refused by Joel");
        self.changed();
        json!({"refused": true})
    }

    /// Every row as it stands, newest ask first, and the summary.
    pub fn rows(&self) -> (Vec<Value>, Value) {
        let held = self.held();
        let mut asks: Vec<&Held> = held.values().collect();
        asks.sort_by_key(|one| std::cmp::Reverse(one.ask.asked_ms));
        let rows = asks
            .iter()
            .map(|one| row::row(one, &self.setting.home, &self.setting.words))
            .collect();
        (rows, row::summary(&asks, &self.setting.words))
    }
}

/// Wakes whoever holds `changed` each time something moves in the asks
/// folder.
fn watch_folder(folder: &Path, changed: Sender<()>) -> std::io::Result<()> {
    let mut inotify = Inotify::init()?;
    inotify.watches().add(
        folder,
        WatchMask::CLOSE_WRITE | WatchMask::MOVED_TO | WatchMask::DELETE | WatchMask::MOVED_FROM,
    )?;
    std::thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        while inotify.read_events_blocking(&mut buffer).is_ok() {
            if changed.send(()).is_err() {
                break;
            }
        }
    });
    Ok(())
}
