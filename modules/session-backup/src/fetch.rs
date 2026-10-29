//! Reading one copy back: the log Drive holds under a name, downloaded and
//! opened to a path, as it was when it was sent.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::config::Config;
use crate::drive::{self, Drive, Fault};
use crate::ledger::Ledger;
use crate::retry::{again, patiently};
use crate::seal;
use crate::token::Login;

/// Write the log Drive holds as `name` to `to`, opened with a key the
/// settings name that this machine holds; answers how long the log is.
/// The ledger says where the copy is, and when it doesn't, as on a machine
/// rebuilt from nothing, Drive is asked for the name. It never writes over
/// a file already at `to`, and whatever stops it leaves nothing there.
pub fn fetch(config: &Config, name: &str, to: &Path) -> Result<u64, String> {
    if config.identities.is_empty() {
        return Err("the settings name no key to open a copy with; \
             list the machine's identity files in identities"
            .into());
    }
    if to.exists() {
        return Err(format!(
            "{} is already there, so nothing was fetched over it",
            to.display()
        ));
    }
    let ledger = Ledger::open(&config.ledger)?;
    let mut login = Login::read(&config.credentials, &config.token)?;
    let agent = drive::agent();
    let drive = Drive {
        agent: &agent,
        api: &config.api,
        folder: &config.folder,
    };
    let id = match ledger.id(name) {
        Some(id) => id.to_owned(),
        None => patiently(&config.retries, || {
            again(&mut login, &agent, |token| drive.find(token, name))
        })
        .map_err(|fault| fault.to_string())?
        .ok_or_else(|| format!("Drive holds no copy named {name}"))?,
    };
    let reading = Reading {
        drive: &drive,
        agent: &agent,
        retries: &config.retries,
        identities: &config.identities,
    };
    read_back(&reading, &mut login, &id, name, to).map_err(|fault| match fault {
        Fault::Busy(why) => format!("Drive stayed busy, so try again later: {why}"),
        fault => fault.to_string(),
    })
}

/// What reading a copy back takes.
#[derive(Clone, Copy)]
pub struct Reading<'a> {
    pub drive: &'a Drive<'a>,
    pub agent: &'a ureq::Agent,
    pub retries: &'a [Duration],
    pub identities: &'a [PathBuf],
}

/// Write the copy Drive holds as `id`, sent as `name`, opened to `to`;
/// answers how long the log is. Drive busy through every retry is
/// `Fault::Busy`; a copy no key here opens is `Fault::Other`. The sealed
/// download goes beside `to` and is removed either way, and whatever stops
/// it leaves nothing at `to`.
pub fn read_back(
    reading: &Reading<'_>,
    login: &mut Login,
    id: &str,
    name: &str,
    to: &Path,
) -> Result<u64, Fault> {
    let sealed = beside(to, ".sealed");
    let downloaded = patiently(reading.retries, || {
        again(login, reading.agent, |token| {
            reading.drive.download(token, id, name, &sealed)
        })
    });
    let opened =
        downloaded.and_then(|_| seal::open(&sealed, to, reading.identities).map_err(Fault::Other));
    // The sealed copy was only the way here.
    let _ = fs::remove_file(&sealed);
    opened
}

/// The path `path` names with `suffix` on the end of its file name.
fn beside(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().map(OsString::from).unwrap_or_default();
    name.push(suffix);
    path.with_file_name(name)
}
