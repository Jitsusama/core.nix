//! The daemon: watches the asks folder, holds each ask it may serve,
//! serves them as rows in Asks over the socket systemd holds for it, and
//! answers the launcher's release and refuse.
//!
//! It's an experiment, so it may release one item alone, the one it's
//! given, and only into its own checker, which says how long the value is
//! and nothing more. An ask for anything else is refused unseen, so no
//! real secret can come out of it and no agent's command can run with
//! one. A yes is refused while the session is locked, and when anything
//! the line showed has changed since, the row then served again to be
//! read again. What it doesn't hold yet is that only Joel's key says yes:
//! its socket is his account's, as every daemon's is, so another program
//! could say yes for this throwaway item; the passwords daemon's own
//! channel brings that.
//!
//! Each answer is written beside its ask as `<id>.answer.json`, and the
//! ask's file is taken away; the journal says each ask's id, item,
//! command, tree and outcome, never the value.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use daemon_serve::asks::{Domain, error};
use daemon_serve::serve::{self, Own, Shared};
use inotify::{Inotify, WatchMask};
use ready_rows::kept::Kept;
use serde_json::{Value, json};

use crate::ask::{self, Ask};
use crate::mask::masked;
use crate::row::{self, Held, STAYS_MS, State};
use crate::tree;
use crate::words::Words;

/// The daemon as the launcher knows it: Asks' rows, and how many wait.
const DOMAIN: Domain = Domain {
    name: "secret-asks",
    kinds: &["row", "summary"],
};

/// The one command an ask may name: this program's checker.
pub const CHECKER: &[&str] = &["secret-asks", "received"];
/// The most an ask's file may hold, so a huge one is no ask.
const MOST_ASK_BYTES: u64 = 64 * 1024;
/// How often a running checker is looked at for having ended, since a
/// child's end can't be waited on with a time limit in std.
const LOOKS_EVERY: Duration = Duration::from_millis(20);

/// What the daemon is given.
pub struct Config {
    pub socket: PathBuf,
    pub asks: PathBuf,
    /// The one item it may release.
    pub item: String,
    /// 1Password's account to read it from, where op's default isn't it.
    pub account: Option<String>,
    pub op: PathBuf,
    pub git: PathBuf,
    pub loginctl: PathBuf,
    /// The checker's program, this one.
    pub checker: PathBuf,
    pub home: String,
    pub words: Words,
}

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

/// What's held of every ask, by id.
#[derive(Default)]
struct Asks {
    held: BTreeMap<String, Held>,
}

/// What every part of the daemon shares.
struct Daemon {
    config: Config,
    asks: Mutex<Asks>,
    changed: Mutex<Sender<()>>,
}

impl Daemon {
    fn asks(&self) -> std::sync::MutexGuard<'_, Asks> {
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
        self.config.asks.join(format!("{id}.ask.json"))
    }

    /// Writes an ask's answer and takes its file away.
    fn answer(&self, id: &str, said: &Value) {
        let path = self.config.asks.join(format!("{id}.answer.json"));
        if let Err(why) = write_whole(&path, said) {
            eprintln!("<3>secret-asks: can't answer {id}: {why}");
        }
        let _ = std::fs::remove_file(self.ask_path(id));
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

    /// Why an ask can't be served, where it can't: an item other than the
    /// one, or a command other than the checker.
    fn refused(&self, ask: &Ask) -> Option<&'static str> {
        if ask.item != self.config.item {
            return Some("not listed");
        }
        if ask.argv != CHECKER {
            return Some("the experiment runs only its own checker");
        }
        None
    }

    /// Takes in every ask file not yet held, and lets go of each waiting
    /// ask whose file was taken away.
    fn look(&self) {
        let names: Vec<String> = std::fs::read_dir(&self.config.asks)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        let ids: Vec<String> = names
            .iter()
            .filter_map(|name| ask::id_of(name))
            .map(str::to_owned)
            .collect();
        let now = now_ms();
        let mut asks = self.asks();
        asks.held
            .retain(|id, held| held.state != State::Waiting || ids.contains(id));
        for id in ids {
            if asks.held.contains_key(&id) {
                continue;
            }
            match self.read(&id) {
                Err(why) => self.answered(&id, "refused", &format!("not an ask: {why}")),
                Ok(ask) => {
                    if let Some(why) = self.refused(&ask) {
                        self.answered(&id, "refused", why);
                    } else if !ask.waits_at(now) {
                        self.answered(&id, "ended", "ended unanswered");
                    } else {
                        let tree = tree::read(&self.config.git, Path::new(&ask.tree));
                        eprintln!(
                            "<6>secret-asks: {id} asked by {} for {} to run {:?} in {}",
                            ask.session, ask.item, ask.argv, ask.tree
                        );
                        let since_ms = now;
                        asks.held.insert(
                            id,
                            Held {
                                ask,
                                tree,
                                state: State::Waiting,
                                since_ms,
                            },
                        );
                    }
                }
            }
        }
    }

    /// Ends each ask past its ten minutes, and lets go of each row
    /// answered long enough ago; answers when next to look again.
    fn tidy(&self) -> Option<i64> {
        let now = now_ms();
        let mut asks = self.asks();
        let ended: Vec<String> = asks
            .held
            .values()
            .filter(|held| held.state == State::Waiting && !held.ask.waits_at(now))
            .map(|held| held.ask.id.clone())
            .collect();
        for id in ended {
            if let Some(held) = asks.held.get_mut(&id) {
                held.state = State::Ended;
                held.since_ms = now;
            }
            self.answered(&id, "ended", "ended unanswered");
        }
        asks.held.retain(|_, held| {
            matches!(held.state, State::Waiting | State::Releasing)
                || now < held.since_ms + STAYS_MS
        });
        asks.held
            .values()
            .filter_map(|held| match held.state {
                State::Waiting => Some(held.ask.ends_ms()),
                State::Releasing => None,
                _ => Some(held.since_ms + STAYS_MS),
            })
            .min()
    }

    /// A refusal the launcher can say to Joel: its code and message for
    /// whoever reads the socket, and `said`, the asks thing's words by
    /// `word`, which the launcher shows in place of "couldn't".
    fn refusal(&self, code: &str, message: &str, word: &str) -> Value {
        let mut refused = error(&Value::Null, code, message);
        refused["error"]["said"] = Value::String(self.config.words.fill(word, &[]));
        refused
    }

    /// Whether the session in front, the account's display, is locked,
    /// as logind says; one logind can't say is taken as locked.
    fn locked(&self) -> bool {
        let asked = |args: &[&str]| {
            Command::new(&self.config.loginctl)
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

    /// The launcher's release: refused unless the ask still waits, the
    /// session is unlocked, and the ask and its tree read as the digest
    /// said; then the value is read and the checker run on a thread of
    /// its own, so the answer comes at once.
    fn release(self: &Arc<Self>, said: &Value) -> Value {
        let Some(id) = said["key"]
            .as_str()
            .and_then(|key| key.strip_prefix("row:asks:"))
        else {
            return self.refusal("refused", "no such ask", "gone");
        };
        let id = id.to_owned();
        let mut asks = self.asks();
        let Some(held) = asks.held.get_mut(&id) else {
            return self.refusal("refused", "no such ask", "gone");
        };
        if held.state != State::Waiting {
            return self.refusal("refused", "it's already answered", "answered");
        }
        if self.locked() {
            return self.refusal("refused", "the screen is locked", "locked");
        }
        let now = match self.read(&id) {
            Ok(ask) if self.refused(&ask).is_none() => ask,
            _ => return self.refusal("refused", "the ask is gone", "gone"),
        };
        let tree = tree::read(&self.config.git, Path::new(&now.tree));
        if Some(row::digest(&now, &tree).as_str()) != said["digest"].as_str() {
            held.ask = now;
            held.tree = tree;
            drop(asks);
            self.changed();
            return self.refusal(
                "changed",
                "the ask changed after it was drawn; read it again",
                "changed",
            );
        }
        held.state = State::Releasing;
        held.since_ms = now_ms();
        let ask = held.ask.clone();
        drop(asks);
        self.changed();
        let daemon = Arc::clone(self);
        std::thread::spawn(move || daemon.run_released(&ask));
        json!({"releasing": true})
    }

    /// Reads the one item and runs the checker with it, then says what
    /// came of it.
    fn run_released(&self, ask: &Ask) {
        let (state, answer) = match self.read_value() {
            Err(why) => (
                State::Failed { why: why.clone() },
                json!({"state": "failed", "why": why}),
            ),
            Ok(value) => {
                let ran = self.checked(ask, &value);
                drop(value);
                match ran {
                    Err(why) => (
                        State::Failed { why: why.clone() },
                        json!({"state": "failed", "why": why}),
                    ),
                    Ok((exit, output, cut)) => (
                        State::Ran { exit },
                        json!({"state": "ran", "exit": exit, "output": output, "cut": cut,
                               "why": if exit.is_none() { "stopped at the time limit" } else { "" }}),
                    ),
                }
            }
        };
        let mut said = answer;
        said["version"] = json!(ask::VERSION);
        said["id"] = json!(ask.id);
        eprintln!(
            "<6>secret-asks: {} {} {}",
            ask.id,
            state.name(),
            said["exit"]
        );
        self.answer(&ask.id, &said);
        if let Some(held) = self.asks().held.get_mut(&ask.id) {
            held.state = state;
            held.since_ms = now_ms();
        }
        self.changed();
    }

    /// The one item's value, read by op, which asks Joel through
    /// 1Password when it must.
    fn read_value(&self) -> Result<Secret, String> {
        let mut op = Command::new(&self.config.op);
        op.arg("read").arg("--no-newline");
        if let Some(account) = &self.config.account {
            op.arg("--account").arg(account);
        }
        let ran = op
            .arg(&self.config.item)
            .stdin(Stdio::null())
            .output()
            .map_err(|why| format!("couldn't run 1Password's command: {why}"))?;
        if !ran.status.success() {
            let said = String::from_utf8_lossy(&ran.stderr);
            let line = said.lines().last().unwrap_or("").trim();
            return Err(format!("1Password said: {line}"));
        }
        String::from_utf8(ran.stdout)
            .map(Secret)
            .map_err(|_| "the value isn't text".to_owned())
    }

    /// Runs the checker with the value as its one variable, under its
    /// time limit, and answers its exit (none where it was stopped) and
    /// its output, masked.
    fn checked(&self, ask: &Ask, value: &Secret) -> Result<(Option<i32>, String, bool), String> {
        let mut child = Command::new(&self.config.checker)
            .arg(CHECKER[1])
            .env_clear()
            .env("PATH", "/run/current-system/sw/bin")
            .env(&ask.given, &value.0)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|why| format!("couldn't run the checker: {why}"))?;
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
        let mut output = String::new();
        if let Some(mut out) = child.stdout.take() {
            let _ = out.read_to_string(&mut output);
        }
        if let Some(mut err) = child.stderr.take() {
            let _ = err.read_to_string(&mut output);
        }
        let (said, cut) = masked(&output, &value.0);
        Ok((exit, said, cut))
    }

    /// The launcher's refuse: the ask answered refused, if it still waits.
    fn refuse(&self, said: &Value) -> Value {
        let Some(id) = said["key"]
            .as_str()
            .and_then(|key| key.strip_prefix("row:asks:"))
        else {
            return self.refusal("refused", "no such ask", "gone");
        };
        let mut asks = self.asks();
        let Some(held) = asks
            .held
            .get_mut(id)
            .filter(|held| held.state == State::Waiting)
        else {
            return self.refusal("refused", "it's already answered", "answered");
        };
        held.state = State::Refused;
        held.since_ms = now_ms();
        let id = id.to_owned();
        drop(asks);
        self.answered(&id, "refused", "refused by Joel");
        self.changed();
        json!({"refused": true})
    }

    /// Every row as it stands, newest ask first, and the summary.
    fn rows(&self) -> (Vec<Value>, Value) {
        let asks = self.asks();
        let mut held: Vec<&Held> = asks.held.values().collect();
        held.sort_by_key(|one| std::cmp::Reverse(one.ask.asked_ms));
        let rows = held
            .iter()
            .map(|one| row::row(one, &self.config.home, &self.config.words))
            .collect();
        (rows, row::summary(&held, &self.config.words))
    }
}

/// A value read from 1Password, its bytes zeroed once it's let go.
struct Secret(String);

impl Drop for Secret {
    fn drop(&mut self) {
        // SAFETY: zeroes are valid UTF-8, so the string stays a string.
        unsafe { self.0.as_bytes_mut() }.fill(0);
    }
}

/// Wakes the loop each time something moves in the asks folder.
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

/// Runs the daemon for as long as it may.
pub fn run(config: Config) -> ExitCode {
    if let Err(why) = std::fs::create_dir_all(&config.asks).and_then(|()| {
        std::fs::set_permissions(&config.asks, std::fs::Permissions::from_mode(0o700))
    }) {
        eprintln!("secret-asks: can't make {}: {why}", config.asks.display());
        return ExitCode::FAILURE;
    }
    let (to, heard) = mpsc::channel();
    if let Err(why) = watch_folder(&config.asks, to.clone()) {
        eprintln!("secret-asks: can't watch {}: {why}", config.asks.display());
        return ExitCode::FAILURE;
    }
    let socket = config.socket.clone();
    let daemon = Arc::new(Daemon {
        config,
        asks: Mutex::new(Asks::default()),
        changed: Mutex::new(to),
    });
    let own: Own = {
        let daemon = Arc::clone(&daemon);
        Box::new(move |said| match said["ask"].as_str() {
            Some("release") => Some(daemon.release(said)),
            Some("refuse") => Some(daemon.refuse(said)),
            _ => None,
        })
    };
    let shared: Arc<Shared> = Shared::new(DOMAIN, own);
    match serve::listen(&socket) {
        Ok(listener) => serve::accept(Arc::clone(&shared), listener),
        Err(why) => {
            eprintln!("secret-asks: can't listen on {}: {why}", socket.display());
            return ExitCode::FAILURE;
        }
    }
    let mut kept = Kept::default();
    loop {
        daemon.look();
        let next = daemon.tidy();
        let (rows, summary) = daemon.rows();
        if !kept.take(rows, vec![summary]).is_empty() {
            shared.change(kept.all().cloned().collect(), json!({}));
        }
        let wait = next.map(|at| Duration::from_millis(u64::try_from(at - now_ms()).unwrap_or(0)));
        let woke = match wait {
            Some(wait) => heard.recv_timeout(wait),
            None => heard.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        if woke == Err(RecvTimeoutError::Disconnected) {
            return ExitCode::FAILURE;
        }
        // Everything said meanwhile is taken before the rows are made again.
        while heard.try_recv().is_ok() {}
    }
}
