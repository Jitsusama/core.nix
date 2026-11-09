//! whereabouts says where the laptop is, so the launcher's weather for here
//! follows it from town to town rather than staying in the time zone's city,
//! and the clock's zone can follow it too (clock-follows/).
//!
//! It answers over SOCKET, which systemd holds, in the shell's daemon
//! interface (serve.rs): one record, `here:laptop`, with the zone the clock
//! keeps, read from the link ROOT's `etc/localtime` is, and its last move,
//! as clock-follows writes it to MOVED. Both are read again as inotify says
//! they changed (notify.rs), and nothing is asked on a clock.
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
//! the laptop again (`again`), which takes the pin away. The daemon hears
//! either at once, by inotify.
//!
//!     whereabouts SOCKET ROOT [KEPT PIN MOVED]
//!     whereabouts set PIN WORDS
//!     whereabouts again PIN

mod clock;
mod fix;
mod geoclue;
mod interface;
mod named;
mod notify;
mod serve;

use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::StreamExt;
use serde_json::json;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use zbus::Connection;

use fix::{Fix, moved};
use notify::Notify;
use serve::{Known, Shared};

/// How long a here still unnamed waits before its name is asked again.
const AGAIN: Duration = Duration::from_mins(5);
/// How many times one here is asked again for what it lacks, an hour's
/// worth: a service that names a town but no country never will, and
/// isn't asked forever.
const TRIES: u32 = 12;
/// How long a GeoClue that refused or left waits before it's asked
/// again, so a machine without one isn't kept busy asking.
const GEOCLUE_AGAIN: Duration = Duration::from_mins(1);

/// How near, in metres, a town Joel names is taken to be: the width of a
/// town, which is as sure as its name makes it.
const BY_HAND: f64 = 1_000.0;

/// What `set` exits with when the place named couldn't be found, as
/// sysexits.h numbers it: no such place, and no network to ask.
const NOWHERE: u8 = 65;
const UNASKED: u8 = 69;
/// What the daemon exits with when it can't listen, as sysexits.h says
/// of a system error.
const UNLISTENED: u8 = 71;

/// Where a following daemon keeps what it says.
struct Files {
    kept: PathBuf,
    pin: PathBuf,
    moved: PathBuf,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let done = match args.as_slice() {
        ["set", pin, words @ ..] if !words.is_empty() => {
            return set(Path::new(pin), &words.join(" ")).await;
        }
        ["again", pin] => again(Path::new(pin)).map_err(Into::into),
        ["set" | "again", ..] => Err("set and again each need a pin".into()),
        [socket, root] => serve(Path::new(socket), Path::new(root), None).await,
        [socket, root, kept, pin, moved] => {
            let files = Files {
                kept: PathBuf::from(kept),
                pin: PathBuf::from(pin),
                moved: PathBuf::from(moved),
            };
            serve(Path::new(socket), Path::new(root), Some(files)).await
        }
        _ => Err("usage".into()),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.to_string() == "usage" => {
            eprintln!("<3>usage: whereabouts SOCKET ROOT [KEPT PIN MOVED]");
            eprintln!("<3>       whereabouts set PIN WORDS | again PIN");
            ExitCode::from(64)
        }
        Err(error) => {
            eprintln!("<3>{error}");
            ExitCode::FAILURE
        }
    }
}

/// Finds the place Joel named and pins it, which the daemon says at once.
async fn set(pin: &Path, words: &str) -> ExitCode {
    let found = match named::find(words.to_owned()).await {
        Ok(found) => found,
        Err(named::Unfound::Nowhere) => {
            eprintln!("<4>no place is called what was typed");
            return ExitCode::from(NOWHERE);
        }
        Err(named::Unfound::Unasked) => {
            eprintln!("<4>the place named couldn't be asked about");
            return ExitCode::from(UNASKED);
        }
    };
    let when = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let Some(fix) = Fix::new(found.latitude, found.longitude, BY_HAND, when) else {
        eprintln!("<3>the place found is nowhere on the Earth");
        return ExitCode::FAILURE;
    };
    let fix = fix
        .named(Some(found.place), found.zone)
        .countried(found.country)
        .by_hand();
    let pinned = pin
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fix.write(pin));
    match pinned {
        Ok(()) => {
            println!("<6>here set by hand");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("<3>{error}");
            ExitCode::FAILURE
        }
    }
}

/// Takes the pin away, so the daemon says only what GeoClue finds.
fn again(pin: &Path) -> io::Result<()> {
    match fs::remove_file(pin) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
        _ => {}
    }
    println!("<6>here to be found again");
    Ok(())
}

/// The pin Joel set, if any.
fn pinned(pin: &Path) -> Option<Fix> {
    fs::read_to_string(pin).ok().as_deref().and_then(Fix::read)
}

async fn serve(socket: &Path, root: &Path, files: Option<Files>) -> Result<(), Box<dyn Error>> {
    let localtime = root.join("etc/localtime");
    let listener = match serve::listen(socket) {
        Ok(listener) => listener,
        Err(why) => {
            eprintln!("<3>couldn't listen on {}: {why}", socket.display());
            std::process::exit(i32::from(UNLISTENED));
        }
    };
    let following = files.is_some();
    let shared = Shared::new(Known {
        clock_zone: clock::zone(&localtime),
        moved: files.as_ref().and_then(|files| clock::moved(&files.moved)),
        geoclue: following.then(|| json!({"state": "starting"})),
        names: following.then(|| json!({"state": "starting"})),
        here: None,
    });
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || serve::accept(&shared, &listener));
    }
    let (pins, pinned) = unbounded_channel();
    {
        let shared = Arc::clone(&shared);
        let watched = Watched {
            localtime,
            pin: files.as_ref().map(|files| files.pin.clone()),
            moved: files.as_ref().map(|files| files.moved.clone()),
        };
        std::thread::spawn(move || {
            if let Err(why) = hear(&shared, &watched, &pins) {
                eprintln!("<4>changes to the clock and the pin can't be heard: {why}");
            }
        });
    }
    match files {
        Some(files) => follow(&shared, &files, pinned).await,
        None => {
            std::future::pending::<()>().await;
            Ok(())
        }
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

/// Hears each change to the clock's zone, its move and the pin, for as
/// long as the program runs: the first two are read again and said, and
/// the pin's is passed to `follow`, which says what it means for here.
fn hear(shared: &Shared, watched: &Watched, pins: &UnboundedSender<()>) -> io::Result<()> {
    let notify = Notify::new()?;
    let watch = |file: &Path| {
        beside(file).and_then(|(folder, name)| Some((notify.watch(&folder)?, name)))
    };
    let zone = watch(&watched.localtime);
    let moved = watched.moved.as_deref().and_then(watch);
    let pin = watched.pin.as_deref().and_then(watch);
    loop {
        for (said, name) in notify.changes()? {
            let is = |watch: &Option<(i32, String)>| {
                watch
                    .as_ref()
                    .is_some_and(|(wd, file)| *wd == said && *file == name)
            };
            if is(&zone) {
                shared.change(|known| known.clock_zone = clock::zone(&watched.localtime));
            }
            if is(&moved) {
                let file = watched.moved.as_deref();
                shared.change(|known| known.moved = file.and_then(clock::moved));
            }
            if is(&pin) {
                let _ = pins.send(());
            }
        }
    }
}

/// Says a new here: kept for the next start, and told every watch.
fn say(shared: &Shared, files: &Files, here: Option<Fix>) {
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
    match pinned(&files.pin) {
        Some(set) => {
            if last.as_ref() != Some(&set) {
                *last = Some(set.clone());
                say(shared, files, Some(set));
            }
            true
        }
        None => {
            if last.as_ref().is_some_and(|fix| fix.hand) {
                *last = None;
                say(shared, files, None);
            }
            false
        }
    }
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
                let said = named::name(fix.latitude, fix.longitude).await;
                let answered = said.place.is_some() || said.zone.is_some();
                let fix = fix.named(said.place, said.zone).countried(said.country);
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
                // A pin taken away leaves here to GeoClue, which is asked
                // where the laptop is now rather than waited on.
                if last.is_none() {
                    found = geoclue::fix(&bus, client.location().await?).await;
                }
            }
            _ = gone.next() => return Ok("GeoClue left the bus".into()),
        }
    }
}
