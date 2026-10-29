//! whereabouts says where the laptop is, so the launcher's weather for here
//! follows it from town to town rather than staying in the time zone's city.
//!
//! It asks GeoClue, the system's location service, over D-Bus (geoclue.rs),
//! as a client of its own, to a city's accuracy, which is all the weather
//! needs. GeoClue finds it from the Wi-Fi networks in reach, through the
//! geolocation service the machine names, and says each new position as a
//! signal, so nothing here asks on a clock.
//!
//! Each position far enough from the last one said (fix.rs) is named, its
//! town and the zone it keeps asked of the web (named.rs), and written to
//! the file it's given, a small JSON object replaced whole, which the
//! launcher watches (Location.qml). The file is read back as it starts, so
//! a restart says nothing new of a laptop that hasn't moved; one whose
//! name couldn't be asked, as when it starts before a network does, is
//! asked again whenever GeoClue next says anything.
//! Should GeoClue leave the bus, it stops, for systemd to start it again.
//!
//! It says in the journal that here moved, and never where to, since the
//! journal is kept and the file, in the runtime directory, isn't.
//!
//! Where GeoClue can't find the laptop, or finds it wrong, Joel says where
//! it is himself (`set`, from Now's Location): the town he names is found
//! by the web (named.rs) and kept as a pin in the state folder, which the
//! resident writes in place of anything GeoClue says until he asks it to
//! find the laptop again (`again`), which takes the pin away.
//!
//!     whereabouts FILE PIN
//!     whereabouts set FILE PIN WORDS
//!     whereabouts again FILE PIN

mod fix;
mod geoclue;
mod named;

use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use futures_util::StreamExt;
use zbus::Connection;

use fix::{Fix, moved};

/// How long a here still unnamed waits before its name is asked again.
const AGAIN: std::time::Duration = std::time::Duration::from_mins(5);

/// How near, in metres, a town Joel names is taken to be: the width of a
/// town, which is as sure as its name makes it.
const BY_HAND: f64 = 1_000.0;

/// What `set` exits with when the place named couldn't be found, as
/// sysexits.h numbers it: no such place, and no network to ask.
const NOWHERE: u8 = 65;
const UNASKED: u8 = 69;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let done = match args.as_slice() {
        [path, pin] => follow(Path::new(path), Path::new(pin)).await,
        ["set", path, pin, words @ ..] if !words.is_empty() => {
            return set(Path::new(path), Path::new(pin), &words.join(" ")).await;
        }
        ["again", path, pin] => again(Path::new(path), Path::new(pin)).map_err(Into::into),
        _ => {
            eprintln!("<3>usage: whereabouts FILE PIN | set FILE PIN WORDS | again FILE PIN");
            return ExitCode::from(2);
        }
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("<3>{error}");
            ExitCode::FAILURE
        }
    }
}

/// Finds the place Joel named and pins it, writing it as here at once.
async fn set(path: &Path, pin: &Path, words: &str) -> ExitCode {
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
    let fix = fix.named(Some(found.place), found.zone).by_hand();
    let pinned = pin
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fix.write(pin))
        .and_then(|()| fix.write(path));
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

/// Takes the pin away, and here with it, so the resident, started again,
/// says only what GeoClue finds. Here is written as nowhere, `{}`, never
/// removed, since the launcher can't see a removed file come back.
fn again(path: &Path, pin: &Path) -> io::Result<()> {
    match fs::remove_file(pin) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
        _ => {}
    }
    let mut name = path.file_name().unwrap_or_default().to_owned();
    name.push(".new");
    let beside = path.with_file_name(name);
    fs::write(&beside, "{}")?;
    fs::rename(&beside, path)?;
    println!("<6>here to be found again");
    Ok(())
}

/// The pin Joel set, if any.
fn pinned(pin: &Path) -> Option<Fix> {
    fs::read_to_string(pin).ok().as_deref().and_then(Fix::read)
}

async fn follow(path: &Path, pin: &Path) -> Result<(), Box<dyn Error>> {
    // A here Joel set whose pin has gone is no here to keep, so GeoClue's
    // next is said wherever it is.
    let mut last = fs::read_to_string(path)
        .ok()
        .as_deref()
        .and_then(Fix::read)
        .filter(|fix| !fix.hand || pinned(pin).is_some());
    if let Some(set) = pinned(pin).filter(|set| last.as_ref() != Some(set)) {
        set.write(path)?;
        last = Some(set);
    }
    let bus = Connection::system().await?;
    // Heard before GeoClue is first asked, so its leaving is never missed,
    // and the client's signal before it starts, so no position is.
    let mut gone = geoclue::gone(&bus).await?;
    let client = geoclue::client(&bus).await?;
    let mut updates = client.receive_location_updated().await?;
    client.start().await?;
    // A position GeoClue already had may come before the signal is heard.
    let mut found = geoclue::fix(&bus, client.location().await?).await;
    loop {
        // While Joel's pin holds, GeoClue's positions are heard and left.
        let held = pinned(pin).is_some();
        if let Some(now) = found.take().filter(|_| !held) {
            // A new here, or the last one again while it still lacks its
            // name or its zone.
            let renamed = if moved(last.as_ref(), &now) {
                Some(now)
            } else {
                last.clone().filter(Fix::unnamed)
            };
            if let Some(fix) = renamed {
                let said = named::name(fix.latitude, fix.longitude).await;
                let fix = fix.named(said.place, said.zone);
                fix.write(path)?;
                if last.as_ref().is_some_and(|was| was.distance(&fix) < 1.0) {
                    println!("<6>here named");
                } else {
                    println!(
                        "<6>here moved, to within {} km",
                        (fix.accuracy / 1000.0).ceil()
                    );
                }
                last = Some(fix);
            }
        }
        // GeoClue says nothing more of a laptop sitting still, so one still
        // unnamed, as when the program starts before a network does, is
        // asked again a few minutes on, and never once it's named.
        let unnamed = !held && last.as_ref().is_some_and(Fix::unnamed);
        tokio::select! {
            update = updates.next() => match update {
                Some(update) => found = geoclue::fix(&bus, update.args()?.now().clone()).await,
                None => return Err("GeoClue's client stopped saying anything".into()),
            },
            () = tokio::time::sleep(AGAIN), if unnamed => found = last.clone(),
            _ = gone.next() => return Err("GeoClue left the bus".into()),
        }
    }
}
