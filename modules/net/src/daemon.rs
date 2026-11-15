//! The collector: every minute it reads eero's record from the eero
//! daemon's file and the modem's status pages, and writes the home
//! network's record whole to `$XDG_RUNTIME_DIR/net/network.json`, so a view
//! never reads half of one. It listens on nothing and sends nothing but
//! plain reads of the modem's pages, each given two seconds. The switch
//! it reads from its reader's report, never from the switch itself, since
//! signing in to the switch takes a password this account never holds.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use serde_json::Value;
use ureq::Agent;

use crate::collect::{self, Said};
use crate::sources::{modem, switch};
use crate::time;

/// How often the sources are read: the eero daemon's own pace, so each
/// round sees its latest.
const EVERY: Duration = Duration::from_secs(60);

/// How long one modem page may take; a modem that answers at all answers
/// in a fraction of this.
const PAGE: Duration = Duration::from_secs(2);

pub fn run() -> Result<(), String> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or("XDG_RUNTIME_DIR isn't set")?;
    let record = runtime.join("net/network.json");
    let eero = runtime.join("eero/home-network.json");
    fs::create_dir_all(runtime.join("net"))
        .map_err(|error| format!("can't make net's folder: {error}"))?;
    let agent: Agent = Agent::config_builder()
        .timeout_global(Some(PAGE))
        .build()
        .into();
    loop {
        let said = Said {
            eero: read_eero(&eero),
            modem: read_modem(&agent),
            switch: read_switch(Path::new(switch::REPORT)),
        };
        let network = collect::network(said, time::now());
        let written = serde_json::to_string_pretty(&network).map_err(|error| error.to_string())?;
        write(&record, &(written + "\n"))
            .map_err(|error| format!("can't write the record: {error}"))?;
        thread::sleep(EVERY);
    }
}

fn read_eero(file: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(file).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => "the eero daemon's record isn't there".to_owned(),
        _ => format!("can't read the eero daemon's record: {error}"),
    })?;
    serde_json::from_str(&text)
        .map_err(|error| format!("the eero daemon's record doesn't read: {error}"))
}

fn read_switch(file: &Path) -> Result<switch::Report, String> {
    let text = fs::read_to_string(file).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => "its reader's report isn't there".to_owned(),
        _ => format!("can't read its reader's report: {error}"),
    })?;
    serde_json::from_str(&text)
        .map_err(|error| format!("its reader's report doesn't read: {error}"))
}

/// Each page, as far as the modem answers: a page that doesn't is left
/// out, and the system page's absence says the modem isn't answering.
fn read_modem(agent: &Agent) -> Result<modem::Pages, String> {
    let mut pages = modem::Pages::new();
    let mut failed = None;
    for page in modem::PAGES {
        let url = format!("http://{}/data/{page}.asp", modem::ADDRESS);
        match agent
            .get(&url)
            .call()
            .and_then(|mut answer| answer.body_mut().read_to_string())
        {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(answer) => {
                    pages.insert(page.to_owned(), answer);
                }
                Err(error) => {
                    failed = failed.or(Some(format!("its {page} page isn't JSON: {error}")));
                }
            },
            Err(error) => {
                failed = failed.or(Some(format!("its {page} page didn't answer: {error}")));
            }
        }
        if page == "getSysInfo" && !pages.contains_key(page) {
            break;
        }
    }
    if pages.is_empty() {
        return Err(failed.unwrap_or_else(|| "it didn't answer".into()));
    }
    Ok(pages)
}

/// Writes a file whole, so a view never reads half of one.
fn write(file: &Path, contents: &str) -> io::Result<()> {
    let fresh = file.with_extension("json.new");
    fs::write(&fresh, contents)?;
    fs::rename(fresh, file)
}
