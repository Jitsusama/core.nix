//! whereabouts says where the laptop is, so the launcher's weather for here
//! follows it from town to town rather than staying in the time zone's city,
//! and the clock's zone can follow it too (clock-follows/).
//!
//! It answers over SOCKET, which systemd holds, in the shell's daemon
//! interface, which daemon-serve speaks for it (serve.rs): one record,
//! `here:laptop`, with the zone the clock keeps, read from the link ROOT's
//! `etc/localtime` is, and its last move, as clock-follows writes it to
//! MOVED. Both are read again as inotify says they changed (notify.rs),
//! and nothing is asked on a clock.
//!
//! Given KEPT, PIN and MOVED, as a machine that follows the laptop is, it
//! asks GeoClue, the system's location service, over D-Bus (geoclue.rs),
//! as a client of its own, to a city's accuracy, which is all the weather
//! needs. GeoClue finds it from the Wi-Fi networks in reach, through the
//! geolocation service the machine names, and says each new position as a
//! signal. Each position far enough from the last one said (fix.rs) is
//! named, its town, its country and the zone it keeps asked of the web
//! (named.rs), and said to every watch. It's kept in KEPT, in the runtime
//! folder, and read back as it starts, so a restart says nothing new of a
//! laptop that hasn't moved; one whose name couldn't be asked, as when it
//! starts before a network does, is asked again whenever GeoClue next says
//! anything. Should GeoClue refuse or leave the bus, it's asked again a
//! minute on, and the socket goes on answering meanwhile.
//!
//! It says in the journal that here moved, and never where to, since the
//! journal is kept and the runtime folder isn't.
//!
//! Where GeoClue can't find the laptop, or finds it wrong, Joel says where
//! it is himself (`set`, from Now's Location): the town he names is found
//! by the web (named.rs) and kept as PIN in the state folder, which the
//! daemon says in place of anything GeoClue says until he asks it to find
//! the laptop again (`again`), which takes the pin away (pin.rs). Both are
//! asks on the socket, which the launcher makes, and commands run by
//! hand. The daemon hears either at once, by inotify.
//!
//! Given WORDS, the launcher's words.json, it serves Location's rows too,
//! worded and ready to draw (rows.rs), so the launcher holds none of it.
//!
//! Its program goes by its domain's name, location, as its socket and
//! units do (docs/daemons.md):
//!
//!     location SOCKET ROOT [KEPT PIN MOVED] [--words WORDS]
//!     location set PIN WORDS
//!     location again PIN

mod clock;
mod figures;
mod fix;
mod geoclue;
mod interface;
mod named;
mod notify;
mod pin;
mod rows;
mod serve;
mod words;

use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use serde_json::json;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use zbus::Connection;

use fix::{Fix, moved};
use notify::Notify;
use pin::Unpinned;
use serve::{Known, Shared};
use words::Words;

/// How long a here still unnamed waits before its name is asked again.
const AGAIN: Duration = Duration::from_mins(5);
/// How many times one here is asked again for what it lacks, an hour's
/// worth: a service that names a town but no country never will, and
/// isn't asked forever.
const TRIES: u32 = 12;
/// How long a GeoClue that refused or left waits before it's asked
/// again, so a machine without one isn't kept busy asking.
const GEOCLUE_AGAIN: Duration = Duration::from_mins(1);

/// What `set` exits with when the place named couldn't be found, as
/// sysexits.h numbers it: no such place, and no network to ask.
const NOWHERE: u8 = 65;
const UNASKED: u8 = 69;
/// What the daemon exits with when it can't listen, as sysexits.h says
/// of a system error.
const UNLISTENED: u8 = 71;

/// Arguments it doesn't know, told from every other failure by its type.
#[derive(Debug)]
struct Usage;

impl std::fmt::Display for Usage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("usage")
    }
}

impl Error for Usage {}

/// Where a following daemon keeps what it says.
struct Files {
    kept: PathBuf,
    pin: PathBuf,
    moved: PathBuf,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    // The launcher's words, given last, for Location's rows.
    let words = match args.iter().position(|arg| arg == "--words") {
        Some(at) if at + 2 == args.len() => {
            let path = args.pop().map(PathBuf::from);
            args.pop();
            path.as_deref().and_then(Words::in_file)
        }
        _ => None,
    };
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let done = match args.as_slice() {
        ["set", pin, words @ ..] if !words.is_empty() => {
            return set(Path::new(pin), &words.join(" "));
        }
        ["again", pin] => pin::again(Path::new(pin))
            .map(|()| println!("<6>here to be found again"))
            .map_err(Into::into),
        ["set" | "again", ..] => Err("set and again each need a pin".into()),
        [socket, root] => serve(Path::new(socket), Path::new(root), None, words).await,
        [socket, root, kept, pin, moved] => {
            let files = Files {
                kept: PathBuf::from(kept),
                pin: PathBuf::from(pin),
                moved: PathBuf::from(moved),
            };
            serve(Path::new(socket), Path::new(root), Some(files), words).await
        }
        _ => Err(Usage.into()),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.is::<Usage>() => {
            eprintln!("<3>usage: location SOCKET ROOT [KEPT PIN MOVED] [--words WORDS]");
            eprintln!("<3>       location set PIN WORDS | again PIN");
            ExitCode::from(64)
        }
        Err(error) => {
            eprintln!("<3>{error}");
            ExitCode::FAILURE
        }
    }
}

/// Finds the place Joel named and pins it, which the daemon says at once.
fn set(pin: &Path, words: &str) -> ExitCode {
    match pin::set(pin, words) {
        Ok(()) => {
            println!("<6>here set by hand");
            ExitCode::SUCCESS
        }
        Err(Unpinned::Nowhere) => {
            eprintln!("<4>no place is called what was typed");
            ExitCode::from(NOWHERE)
        }
        Err(Unpinned::Unasked) => {
            eprintln!("<4>the place named couldn't be asked about");
            ExitCode::from(UNASKED)
        }
        Err(Unpinned::Unkept(why)) => {
            eprintln!("<3>{why}");
            ExitCode::FAILURE
        }
    }
}

/// The pin Joel set, if any.
fn pinned(pin: &Path) -> Option<Fix> {
    fs::read_to_string(pin).ok().as_deref().and_then(Fix::read)
}

async fn serve(
    socket: &Path,
    root: &Path,
    files: Option<Files>,
    words: Option<Words>,
) -> Result<(), Box<dyn Error>> {
    let localtime = root.join("etc/localtime");
    // Heard before anything's first read and before the socket answers,
    // so nothing changed as it starts is missed between the two.
    let hearing = Hearing::new(Watched {
        localtime: localtime.clone(),
        pin: files.as_ref().map(|files| files.pin.clone()),
        moved: files.as_ref().map(|files| files.moved.clone()),
    });
    let listener = match daemon_serve::serve::listen(socket) {
        Ok(listener) => listener,
        Err(why) => {
            eprintln!("<3>couldn't listen on {}: {why}", socket.display());
            std::process::exit(i32::from(UNLISTENED));
        }
    };
    let following = files.is_some();
    let known = Known {
        clock_zone: clock::zone(&localtime),
        moved: files.as_ref().and_then(|files| clock::moved(&files.moved)),
        geoclue: following.then(|| json!({"state": "starting"})),
        names: following.then(|| json!({"state": "starting"})),
        here: None,
    };
    let pin = files.as_ref().map(|files| files.pin.clone());
    let shared = Shared::new(known, words, pin);
    {
        let served = shared.served();
        std::thread::spawn(move || daemon_serve::serve::accept(served, listener));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || serve::turn(&shared));
    }
    let (pins, pinned) = unbounded_channel();
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || {
            if let Err(why) = hearing.and_then(|hearing| hearing.hear(&shared, &pins)) {
                eprintln!("<4>changes to the clock and the pin can't be heard: {why}");
            }
        });
    }
    if let Some(files) = files {
        follow(&shared, &files, pinned).await
    } else {
        std::future::pending::<()>().await;
        Ok(())
    }
}

/// The files whose changes the daemon hears.
struct Watched {
    localtime: PathBuf,
    pin: Option<PathBuf>,
    moved: Option<PathBuf>,
}

/// A file's folder, made where it isn't there yet so it can be watched,
/// and the file's name within it.
fn beside(file: &Path) -> Option<(PathBuf, String)> {
    let folder = file.parent()?.to_path_buf();
    let _ = fs::create_dir_all(&folder);
    Some((folder, file.file_name()?.to_str()?.to_owned()))
}

/// The clock's zone, its move and the pin, each watched from the moment
/// hearing begins: `serve` begins it before it listens or first reads
/// any of them, so each change is either read or heard. Watched once its
/// loop ran, a pin set as the daemon started, after its first read, was
/// never heard, and Find It Again never offered.
struct Hearing {
    notify: Notify,
    watched: Watched,
    zone: Option<(i32, String)>,
    moved: Option<(i32, String)>,
    pin: Option<(i32, String)>,
}

impl Hearing {
    fn new(watched: Watched) -> io::Result<Hearing> {
        let notify = Notify::new()?;
        let watch = |file: &Path| {
            beside(file).and_then(|(folder, name)| Some((notify.watch(&folder)?, name)))
        };
        let zone = watch(&watched.localtime);
        let moved = watched.moved.as_deref().and_then(watch);
        let pin = watched.pin.as_deref().and_then(watch);
        Ok(Hearing {
            notify,
            watched,
            zone,
            moved,
            pin,
        })
    }

    /// Hears each change to the clock's zone, its move and the pin, for as
    /// long as the program runs: the first two are read again and said,
    /// and the pin's is passed to `follow`, which says what it means for
    /// here.
    fn hear(&self, shared: &Shared, pins: &UnboundedSender<()>) -> io::Result<()> {
        let watched = &self.watched;
        loop {
            for (said, name) in self.notify.changes()? {
                let is = |watch: &Option<(i32, String)>| {
                    watch
                        .as_ref()
                        .is_some_and(|(wd, file)| *wd == said && *file == name)
                };
                if is(&self.zone) {
                    shared.change(|known| known.clock_zone = clock::zone(&watched.localtime));
                }
                if is(&self.moved) {
                    let file = watched.moved.as_deref();
                    shared.change(|known| known.moved = file.and_then(clock::moved));
                }
                if is(&self.pin) {
                    let _ = pins.send(());
                }
            }
        }
    }
}

/// Says a new here: kept for the next start, and told every watch.
fn say(shared: &Shared, files: &Files, here: Option<Fix>) {
    if let Some(folder) = files.kept.parent() {
        let _ = fs::create_dir_all(folder);
    }
    let kept = match &here {
        Some(fix) => fix.write(&files.kept),
        None => match fs::remove_file(&files.kept) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        },
    };
    if let Err(why) = kept {
        eprintln!("<4>here couldn't be kept for the next start: {why}");
    }
    shared.change(|known| known.here = here);
}

/// Says the pin while there is one, and lets a here Joel set go once its
/// pin has; answers whether a pin holds.
fn pin_holds(shared: &Shared, files: &Files, last: &mut Option<Fix>) -> bool {
    if let Some(set) = pinned(&files.pin) {
        if last.as_ref() != Some(&set) {
            *last = Some(set.clone());
            say(shared, files, Some(set));
        }
        return true;
    }
    if last.as_ref().is_some_and(|fix| fix.hand) {
        *last = None;
        say(shared, files, None);
    }
    false
}

/// Hears the pin change while GeoClue is followed, and answers whether
/// GeoClue is to be asked where the laptop is now rather than waited on, as
/// it is once Joel takes his pin away. The pin is read first, so the here it
/// held is let go before what's left is judged: judged first, a here set by
/// hand was still there, GeoClue was never asked, and here stayed unknown
/// until the laptop moved far enough for GeoClue to say so.
fn found_again(shared: &Shared, files: &Files, last: &mut Option<Fix>) -> bool {
    !pin_holds(shared, files, last) && last.is_none()
}

/// Follows GeoClue for as long as the program runs, asking it again a
/// minute after it refuses or leaves, and hearing the pin meanwhile.
async fn follow(
    shared: &Shared,
    files: &Files,
    mut pins: UnboundedReceiver<()>,
) -> Result<(), Box<dyn Error>> {
    // A here Joel set whose pin has gone is no here to keep, so GeoClue's
    // next is said wherever it is.
    let mut last = fs::read_to_string(&files.kept)
        .ok()
        .as_deref()
        .and_then(Fix::read)
        .filter(|fix| !fix.hand);
    shared.change(|known| known.here.clone_from(&last));
    pin_holds(shared, files, &mut last);
    loop {
        let why = match geoclue_followed(shared, files, &mut last, &mut pins).await {
            Ok(why) | Err(why) => why,
        };
        eprintln!("<4>{why}; GeoClue is asked again in a minute");
        let reason = why.to_string();
        shared.change(|known| {
            known.geoclue = Some(json!({"state": "refused", "reason": reason}));
        });
        let again = tokio::time::sleep(GEOCLUE_AGAIN);
        tokio::pin!(again);
        loop {
            tokio::select! {
                () = &mut again => break,
                heard = pins.recv() => {
                    if heard.is_none() {
                        return Err("the pin can no longer be heard".into());
                    }
                    pin_holds(shared, files, &mut last);
                }
            }
        }
    }
}

/// Follows one client of GeoClue's until GeoClue stops answering, saying
/// why.
async fn geoclue_followed(
    shared: &Shared,
    files: &Files,
    last: &mut Option<Fix>,
    pins: &mut UnboundedReceiver<()>,
) -> Result<Box<dyn Error>, Box<dyn Error>> {
    let bus = Connection::system().await?;
    // Heard before GeoClue is first asked, so its leaving is never missed,
    // and the client's signal before it starts, so no position is.
    let mut gone = geoclue::gone(&bus).await?;
    let client = geoclue::client(&bus).await?;
    let mut updates = client.receive_location_updated().await?;
    client.start().await?;
    shared.change(|known| known.geoclue = Some(json!({"state": "answering"})));
    // A position GeoClue already had may come before the signal is heard.
    let mut found = geoclue::fix(&bus, client.location().await?).await;
    // How many times this here has been asked again for what it lacks.
    let mut tried: u32 = 0;
    loop {
        // While Joel's pin holds, GeoClue's positions are heard and left.
        let held = pin_holds(shared, files, last);
        if let Some(now) = found.take().filter(|_| !held) {
            // A new here, or the last one again while it still lacks its
            // name or its zone.
            let renamed = if moved(last.as_ref(), &now) {
                tried = 0;
                Some(now)
            } else {
                last.clone().filter(Fix::unnamed)
            };
            if let Some(fix) = renamed {
                let said = named::name(fix.latitude, fix.longitude, fix.accuracy).await;
                let answered = said.place.is_some() || said.zone.is_some();
                let fix = fix
                    .named(said.place, said.zone)
                    .zone_holding(said.zone_holds)
                    .countried(said.country);
                if last.as_ref().is_some_and(|was| was.distance(&fix) < 1.0) {
                    println!("<6>here named");
                } else {
                    println!(
                        "<6>here moved, to within {} km",
                        (fix.accuracy / 1000.0).ceil()
                    );
                }
                *last = Some(fix.clone());
                say(shared, files, Some(fix));
                shared.change(|known| {
                    known.names = Some(if answered {
                        json!({"state": "answering"})
                    } else {
                        json!({"state": "refused", "reason": "neither service could be asked"})
                    });
                });
            }
        }
        // GeoClue says nothing more of a laptop sitting still, so one still
        // unnamed, as when the program starts before a network does, is
        // asked again a few minutes on, and never once it's named or it's
        // been asked an hour's worth.
        let unnamed = !held && tried < TRIES && last.as_ref().is_some_and(Fix::unnamed);
        tokio::select! {
            update = updates.next() => match update {
                Some(update) => found = geoclue::fix(&bus, update.args()?.now().clone()).await,
                None => return Ok("GeoClue's client stopped saying anything".into()),
            },
            // Only these retries count toward the hour's worth: a laptop
            // offline for longer, saying it's still, is named once a
            // network comes.
            () = tokio::time::sleep(AGAIN), if unnamed => {
                tried = tried.saturating_add(1);
                found = last.clone();
            }
            heard = pins.recv() => {
                if heard.is_none() {
                    return Err("the pin can no longer be heard".into());
                }
                if found_again(shared, files, last) {
                    found = geoclue::fix(&bus, client.location().await?).await;
                }
            }
            _ = gone.next() => return Ok("GeoClue left the bus".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A following daemon's files, in a folder of the test's own.
    fn files(name: &str) -> Files {
        let folder =
            std::env::temp_dir().join(format!("whereabouts-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        Files {
            kept: folder.join("kept.json"),
            pin: folder.join("pin.json"),
            moved: folder.join("moved.json"),
        }
    }

    fn town(by_hand: bool) -> Fix {
        let fix = Fix::new(12.3457, -98.7654, pin::BY_HAND, 1_790_000_000).unwrap();
        if by_hand { fix.by_hand() } else { fix }
    }

    #[test]
    fn a_pin_taken_away_lets_its_here_go_and_asks_geoclue() {
        let files = files("taken");
        let shared = Shared::new(Known::default(), None, None);
        town(true).write(&files.pin).unwrap();
        let mut last = None;
        assert!(
            !found_again(&shared, &files, &mut last),
            "GeoClue was asked while the pin held"
        );
        assert!(
            last.as_ref().is_some_and(|fix| fix.hand),
            "the pin wasn't said: {last:?}"
        );
        pin::again(&files.pin).unwrap();
        assert!(
            found_again(&shared, &files, &mut last),
            "GeoClue wasn't asked once the pin was taken away"
        );
        assert!(last.is_none(), "the here the pin held was kept: {last:?}");
        assert!(
            !files.kept.exists(),
            "the here the pin held was kept for the next start"
        );
    }

    #[test]
    fn a_pin_heard_without_one_leaves_geoclues_here_and_asks_nothing() {
        let files = files("unpinned");
        let shared = Shared::new(Known::default(), None, None);
        let mut last = Some(town(false));
        assert!(
            !found_again(&shared, &files, &mut last),
            "GeoClue was asked again of a here it had said"
        );
        assert_eq!(last, Some(town(false)), "GeoClue's here was let go");
    }

    /// A pin written once hearing has begun is heard, though hearing's
    /// loop hadn't yet run: as a pin set while the daemon starts is, once
    /// it listens and before it first waits for a change.
    #[test]
    fn a_pin_written_once_hearing_begins_is_heard() {
        let files = files("heard");
        let watched = Watched {
            localtime: files.kept.with_file_name("localtime"),
            pin: Some(files.pin.clone()),
            moved: Some(files.moved.clone()),
        };
        let hearing = Hearing::new(watched).unwrap();
        town(true).write(&files.pin).unwrap();
        let shared = Shared::new(Known::default(), None, None);
        let (pins, mut pinned) = unbounded_channel();
        let (heard, told) = std::sync::mpsc::channel();
        std::thread::spawn(move || hearing.hear(&shared, &pins));
        std::thread::spawn(move || heard.send(pinned.blocking_recv()));
        assert_eq!(
            told.recv_timeout(Duration::from_secs(10)),
            Ok(Some(())),
            "the pin written as hearing began was never heard"
        );
    }
}
