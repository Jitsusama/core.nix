//! The collector: every minute it reads eero's record from the eero
//! daemon's file, the modem's status pages and what each receiver and
//! printer eero lists says, and writes the home network's record whole to
//! `$XDG_RUNTIME_DIR/net/network.json`, so a view never reads half of one.
//! It listens on nothing and sends nothing but reads, one at a time and
//! each given two seconds: the modem's pages, and a receiver's description
//! and the SOAP actions that only ask (src/sources/sony.rs), each only at
//! the receiver's own address and never following a redirect, and a
//! printer's four status pages (src/sources/hp.rs). The switch
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
use crate::sources::{hp, modem, sony, switch};
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
    // A receiver's answers name where it's asked next; sony.rs keeps only
    // those on the receiver itself, and this agent never follows a
    // redirect, so no answer can send a request elsewhere. A printer's
    // pages are asked with it too, so a redirect can't lead one away.
    let at_home: Agent = Agent::config_builder()
        .timeout_global(Some(PAGE))
        .max_redirects(0)
        .build()
        .into();
    loop {
        let heard = read_eero(&eero);
        let receivers = heard
            .as_ref()
            .map(sony::listed)
            .unwrap_or_default()
            .into_iter()
            .map(|(mac, address)| (mac, read_receiver(&at_home, &address)))
            .collect();
        let printers = heard
            .as_ref()
            .map(hp::listed)
            .unwrap_or_default()
            .into_iter()
            .map(|(mac, address)| (mac, read_printer(&at_home, &address)))
            .collect();
        let said = Said {
            eero: heard,
            modem: read_modem(&agent),
            switch: read_switch(Path::new(switch::REPORT)),
            receivers,
            printers,
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

/// What a printer answers: each page as far as it does, a page that
/// doesn't left out, and the printer unreachable when none answers.
fn read_printer(agent: &Agent, address: &str) -> Result<hp::Answers, String> {
    let mut answers = hp::Answers::new();
    let mut failed = None;
    for page in hp::PAGES {
        match agent
            .get(&hp::url(address, page))
            .call()
            .and_then(|mut answer| answer.body_mut().read_to_string())
        {
            Ok(text) => {
                answers.insert(page.to_owned(), text);
            }
            Err(error) => {
                failed = failed.or(Some(format!("its {page} page didn't answer: {error}")));
            }
        }
    }
    if answers.is_empty() {
        return Err(failed.unwrap_or_else(|| "it didn't answer".into()));
    }
    Ok(answers)
}

/// What a receiver answers: its description first, since it names where
/// each service is asked, then each read, then Sony's system information
/// from where its action list says. A read that fails after the
/// description is left out, since the rest still say something.
fn read_receiver(agent: &Agent, address: &str) -> Result<sony::Answers, String> {
    let get = |url: &str| {
        agent
            .get(url)
            .call()
            .and_then(|mut answer| answer.body_mut().read_to_string())
    };
    let url = format!("http://{address}:{}/description.xml", sony::PORT);
    let description =
        get(&url).map_err(|error| format!("its description didn't answer: {error}"))?;
    let mut answers = sony::Answers::new();
    for ask in sony::asks(&description, address) {
        let answer = match &ask.soap {
            Some(soap) => agent
                .post(&ask.url)
                .header("Content-Type", "text/xml; charset=\"utf-8\"")
                .header("SOAPACTION", &soap.action)
                .send(&soap.body)
                .and_then(|mut answer| answer.body_mut().read_to_string()),
            None => get(&ask.url),
        };
        if let Ok(answer) = answer {
            answers.insert(ask.name, answer);
        }
    }
    if let Some(url) = answers
        .get("ActionList")
        .and_then(|list| sony::system(list, address))
        && let Ok(answer) = get(&url)
    {
        answers.insert("getSystemInformation".into(), answer);
    }
    answers.insert("description".into(), description);
    Ok(answers)
}

/// Writes a file whole, so a view never reads half of one.
fn write(file: &Path, contents: &str) -> io::Result<()> {
    let fresh = file.with_extension("json.new");
    fs::write(&fresh, contents)?;
    fs::rename(fresh, file)
}
