//! Reading one copy back: the log Drive holds under a name, downloaded and
//! opened to a path, as it was when it was sent.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

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
/// a file already at `to`, and whatever stops it leaves nothing there. The
/// sealed download goes beside `to` and is removed either way.
pub fn fetch(config: &Config, name: &str, to: &Path) -> Result<u64, String> {
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
    let sealed = beside(to, ".sealed");
    let downloaded = patiently(&config.retries, || {
        again(&mut login, &agent, |token| {
            drive.download(token, &id, name, &sealed)
        })
    });
    let opened = match downloaded {
        Err(Fault::Busy(why)) => Err(format!("Drive stayed busy, so try again later: {why}")),
        Err(fault) => Err(fault.to_string()),
        Ok(_) => seal::open(&sealed, to, &config.identities),
    };
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
