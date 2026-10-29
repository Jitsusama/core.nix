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

mod fix;
mod geoclue;
mod named;

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use futures_util::StreamExt;
use zbus::Connection;

use fix::{Fix, moved};

/// How long a here still unnamed waits before its name is asked again.
const AGAIN: std::time::Duration = std::time::Duration::from_mins(5);

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let Some(path) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("<3>usage: whereabouts FILE");
        return ExitCode::from(2);
    };
    match follow(&path).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("<3>{error}");
            ExitCode::FAILURE
        }
    }
}

async fn follow(path: &Path) -> Result<(), Box<dyn Error>> {
    let mut last = fs::read_to_string(path).ok().as_deref().and_then(Fix::read);
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
        if let Some(now) = found.take() {
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
        let unnamed = last.as_ref().is_some_and(Fix::unnamed);
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
