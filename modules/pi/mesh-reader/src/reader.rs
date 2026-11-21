//! The folders followed, each by one thread, and the clients each has.
//!
//! A folder's thread lists it, says the listing to every client that
//! has asked, then hears its watch, reading only the file each change
//! names and saying it only where it differs from what was last said. It
//! lists again when the watch missed changes or lost the folder, and
//! every backstop, saying only what the listing changed. Once no client
//! is left, the thread lets the folder go.
//!
//! Each client has a queue of its own with room for the folder's listing
//! and [`ROOM`] more lines. One whose queue is full is let go, never
//! waited for, so a stuck client never holds the folder's thread or the
//! other clients; it connects again and is given a fresh listing.

use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use crate::watch::{self, Heard, Watch};
use crate::wire::{self, Ask, LONGEST_ASK, Line};

/// The lines a client may fall behind by, past the folder's listing,
/// before it is let go.
pub const ROOM: usize = 1024;

/// How often a folder is listed whole while its watch says nothing, in
/// case the watch missed a change, as each pi process's own mirror did.
pub const BACKSTOP: Duration = Duration::from_secs(30);

/// How long a folder's thread waits before watching again a folder it
/// couldn't watch: one not made yet, or moved away and not back.
const AGAIN: Duration = Duration::from_secs(1);

/// How often a client's thread, with nothing to tell it, looks whether it went.
const CLIENT_LOOK: Duration = Duration::from_secs(1);

/// One client of a folder: its queue, and whether it is still there.
struct Client {
    queue: SyncSender<Line>,
    here: Arc<AtomicBool>,
}

/// A folder's files as last said, and who it says them to.
#[derive(Default)]
struct Said {
    files: BTreeMap<String, String>,
    ready: bool,
    clients: Vec<Client>,
}

impl Said {
    /// Say a line to every client, letting go of any whose queue is full
    /// or who has gone.
    fn tell(&mut self, line: &Line) {
        self.clients.retain(|client| {
            client.here.load(Ordering::SeqCst)
                && match client.queue.try_send(line.clone()) {
                    Ok(()) => true,
                    Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => false,
                }
        });
    }

    /// Say the whole listing, then `ready`, to one client.
    fn tell_all(&self, client: &Client) -> bool {
        self.files
            .iter()
            .map(|(name, text)| Line::Put {
                name: name.clone(),
                text: text.clone(),
            })
            .chain([Line::Ready])
            .all(|line| client.queue.try_send(line).is_ok())
    }

    /// Take in what a file holds now, saying it if it changed.
    fn take(&mut self, name: &str, text: Option<String>) {
        match text {
            Some(text) if self.files.get(name) != Some(&text) => {
                self.files.insert(name.to_string(), text.clone());
                self.tell(&Line::Put {
                    name: name.to_string(),
                    text,
                });
            }
            Some(_) => {}
            None => {
                if self.files.remove(name).is_some() {
                    self.tell(&Line::Gone {
                        name: name.to_string(),
                    });
                }
            }
        }
    }

    /// Take in a whole listing, saying what changed, and on the first
    /// one, the listing itself to everyone waiting for it.
    fn take_all(&mut self, listed: BTreeMap<String, String>) {
        // The first listing is said whole to each client waiting for it,
        // never file by file as well.
        if !self.ready {
            self.files = listed;
            self.ready = true;
            let clients = std::mem::take(&mut self.clients);
            self.clients = clients
                .into_iter()
                .filter(|client| self.tell_all(client))
                .collect();
            return;
        }
        let gone: Vec<String> = self
            .files
            .keys()
            .filter(|name| !listed.contains_key(*name))
            .cloned()
            .collect();
        for name in gone {
            self.take(&name, None);
        }
        for (name, text) in listed {
            self.take(&name, Some(text));
        }
    }
}

/// A folder followed and the suffix its clients asked for.
type Key = (PathBuf, String);

/// Every folder followed, under the roots it may follow.
pub struct Reader {
    roots: Vec<PathBuf>,
    backstop: Duration,
    folders: Mutex<HashMap<Key, Arc<Mutex<Said>>>>,
}

/// What a client is given as it asks: its lines, and the flag that says
/// it is still there.
pub struct Following {
    pub lines: Receiver<Line>,
    pub here: Arc<AtomicBool>,
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // A thread that panicked holding it left what it held whole enough
    // to read: every change to it is one insert, remove or retain.
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Reader {
    /// A reader of folders under `roots`, listing each whole every
    /// `backstop`.
    pub fn new(roots: Vec<PathBuf>, backstop: Duration) -> Arc<Self> {
        let roots = roots
            .into_iter()
            .map(|root| root.canonicalize().unwrap_or(root))
            .collect();
        Arc::new(Reader {
            roots,
            backstop,
            folders: Mutex::new(HashMap::new()),
        })
    }

    /// How many folders it follows now.
    pub fn following(&self) -> usize {
        locked(&self.folders).len()
    }

    /// Follow the folder an ask names for a new client, or say why not.
    pub fn follow(self: &Arc<Self>, ask: &Ask) -> Result<Following, String> {
        let folder = self.allowed(&ask.folder)?;
        let key = (folder, ask.suffix.clone());
        // Held until the client is in, so the folder's thread, which takes
        // this lock to let an unasked folder go, never sees it empty first.
        let mut folders = locked(&self.folders);
        let said = if let Some(said) = folders.get(&key) {
            Arc::clone(said)
        } else {
            let said = Arc::new(Mutex::new(Said::default()));
            let reader = Arc::clone(self);
            let theirs = Arc::clone(&said);
            let followed = key.clone();
            thread::Builder::new()
                .name("folder".into())
                .spawn(move || reader.keep(&followed, &theirs))
                .map_err(|error| format!("it couldn't follow the folder: {error}"))?;
            folders.insert(key, Arc::clone(&said));
            said
        };
        let mut said = locked(&said);
        let (queue, lines) = sync_channel(said.files.len() + ROOM);
        let here = Arc::new(AtomicBool::new(true));
        let client = Client {
            queue,
            here: Arc::clone(&here),
        };
        if said.ready && !said.tell_all(&client) {
            return Err("its listing didn't fit its queue".into());
        }
        said.clients.push(client);
        Ok(Following { lines, here })
    }

    /// The folder asked for, as it really is, when it lies under a root.
    fn allowed(&self, folder: &Path) -> Result<PathBuf, String> {
        let real = folder
            .canonicalize()
            .map_err(|_| "the folder isn't there".to_string())?;
        if !real.is_dir() {
            return Err("the folder isn't a folder".into());
        }
        if !self.roots.iter().any(|root| real.starts_with(root)) {
            return Err("the folder isn't under a root it reads".into());
        }
        Ok(real)
    }

    /// Follow one folder for as long as it has clients.
    fn keep(&self, (dir, suffix): &Key, said: &Arc<Mutex<Said>>) {
        let mut watched: Option<Watch> = None;
        let mut listed_at: Option<Instant> = None;
        loop {
            if self.let_go_if_unasked(dir, suffix, said) {
                return;
            }
            let Some(watch) = &watched else {
                // Watch first and list after, so a change between the two
                // is in the listing or heard.
                match Watch::new(dir) {
                    Ok(watch) => {
                        watched = Some(watch);
                        listed_at = None;
                    }
                    Err(error) => {
                        eprintln!(
                            "pi-mesh-reader: {} can't be watched: {error}",
                            dir.display()
                        );
                        thread::sleep(AGAIN.min(self.backstop));
                    }
                }
                continue;
            };
            let due = listed_at.is_none_or(|at| at.elapsed() >= self.backstop);
            if due {
                match watch::list(dir, suffix) {
                    Ok(listed) => locked(said).take_all(listed),
                    Err(error) => {
                        eprintln!("pi-mesh-reader: {} can't be listed: {error}", dir.display());
                    }
                }
                listed_at = Some(Instant::now());
            }
            let left = listed_at.map_or(self.backstop, |at| {
                self.backstop.saturating_sub(at.elapsed())
            });
            let heard = match watch.hear(left.min(self.backstop)) {
                Ok(heard) => heard,
                Err(error) => {
                    eprintln!("pi-mesh-reader: {}'s watch failed: {error}", dir.display());
                    vec![Heard::Lost]
                }
            };
            for one in heard {
                match one {
                    Heard::Named(name) if watch::wanted(&name, suffix) => {
                        let text = watch::read(dir, &name).unwrap_or_else(|error| {
                            eprintln!(
                                "pi-mesh-reader: {} can't be read: {error}",
                                dir.join(&name).display()
                            );
                            None
                        });
                        locked(said).take(&name, text);
                    }
                    Heard::Named(_) => {}
                    Heard::Missed => listed_at = None,
                    Heard::Lost => {
                        watched = None;
                        break;
                    }
                }
            }
        }
    }

    /// Let the folder go once nobody follows it, answering whether it was.
    fn let_go_if_unasked(&self, dir: &Path, suffix: &str, said: &Arc<Mutex<Said>>) -> bool {
        let mut folders = locked(&self.folders);
        let mut said = locked(said);
        said.clients
            .retain(|client| client.here.load(Ordering::SeqCst));
        if !said.clients.is_empty() {
            return false;
        }
        folders.remove(&(dir.to_path_buf(), suffix.to_string()));
        true
    }
}

/// Answer one client: read its ask, then send it what its folder holds
/// and each change, until it goes or is let go.
pub fn serve(reader: &Arc<Reader>, stream: &UnixStream) {
    let Ok(mut writer) = stream.try_clone() else {
        return;
    };
    let mut asked = String::new();
    let mut lines = BufReader::new(stream.take(LONGEST_ASK as u64));
    if lines.read_line(&mut asked).is_err() {
        return;
    }
    let following = match wire::ask(&asked).and_then(|ask| reader.follow(&ask)) {
        Ok(following) => following,
        Err(why) => {
            let _ = writeln!(writer, "{}", Line::No { why }.text());
            return;
        }
    };
    // Nothing more is read from a client but its end, which says it went.
    let here = Arc::clone(&following.here);
    let gone = stream.try_clone();
    let _ = thread::Builder::new()
        .name("client-end".into())
        .spawn(move || {
            if let Ok(mut gone) = gone {
                let mut sink = [0u8; 256];
                while matches!(gone.read(&mut sink), Ok(read) if read > 0) {}
                here.store(false, Ordering::SeqCst);
                let _ = gone.shutdown(std::net::Shutdown::Both);
            }
        });
    // Woken each second to see whether the client went, so a client that
    // leaves a quiet folder doesn't hold its thread until the next change.
    loop {
        match following.lines.recv_timeout(CLIENT_LOOK) {
            Ok(line) => {
                let sent = writer
                    .write_all(line.text().as_bytes())
                    .and_then(|()| writer.write_all(b"\n"));
                if sent.is_err() {
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) if following.here.load(Ordering::SeqCst) => {}
            Err(_) => break,
        }
    }
    following.here.store(false, Ordering::SeqCst);
    let _ = stream.shutdown(std::net::Shutdown::Both);
}
