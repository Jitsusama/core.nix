//! whereabouts says where the laptop is, so the launcher's weather for here
//! follows it from town to town rather than staying in the time zone's city.
//!
//! It asks GeoClue, the system's location service, over D-Bus (geoclue.rs),
//! as a client of its own, to a city's accuracy, which is all the weather
//! needs. GeoClue finds it from the Wi-Fi networks in reach, through the
//! geolocation service the machine names, and says each new position as a
//! signal, so nothing here asks on a clock.
//!
//! Each position far enough from the last one said (fix.rs) is written to
//! the file it's given, a small JSON object replaced whole, which the
//! launcher watches (launcher/Weather.qml). The file is read back as it
//! starts, so a restart says nothing new of a laptop that hasn't moved.
//! Should GeoClue leave the bus, it stops, for systemd to start it again.
//!
//! It says in the journal that here moved, and never where to, since the
//! journal is kept and the file, in the runtime directory, isn't.

mod fix;
mod geoclue;

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use futures_util::StreamExt;
use zbus::Connection;

use fix::{Fix, moved};

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
        if let Some(now) = found.take()
            && moved(last.as_ref(), &now)
        {
            now.write(path)?;
            println!(
                "<6>here moved, to within {} km",
                (now.accuracy / 1000.0).ceil()
            );
            last = Some(now);
        }
        tokio::select! {
            update = updates.next() => match update {
                Some(update) => found = geoclue::fix(&bus, update.args()?.now().clone()).await,
                None => return Err("GeoClue's client stopped saying anything".into()),
            },
            _ = gone.next() => return Err("GeoClue left the bus".into()),
        }
    }
}
