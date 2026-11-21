//! The collector: every minute it reads eero's record from the eero
//! daemon's file, the modem's status pages and what each receiver and
//! printer eero lists says, and writes the home network's record whole to
//! `$XDG_RUNTIME_DIR/net/network.json`, so a view never reads half of one.
//! It listens only on its own socket and sends nothing but reads, one at a time and
//! each given two seconds: the modem's pages, and a receiver's description
//! and the SOAP actions that only ask (src/sources/sony.rs), each only at
//! the receiver's own address and never following a redirect, and a
//! printer's four status pages (src/sources/hp.rs). The switch
//! it reads from its reader's report, never from the switch itself, since
//! signing in to the switch takes a password this account never holds.
//! Each minute's swinging figures are kept for a day in
//! `$XDG_STATE_HOME/net/day.jsonl` (src/history.rs), so the record can say
//! what a modem channel lost lately and when each radio was busiest.
//!
//! It serves the same record, split by kind (src/serve.rs), on the socket
//! systemd holds for it at `$XDG_RUNTIME_DIR/daemons/net.sock`, through
//! daemon-serve, so a page or the launcher watches what moves rather
//! than reading the file each minute.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use daemon_serve::serve::{self, Shared};
use serde_json::Value;
use ureq::Agent;

use crate::collect::{self, Said};
use crate::history::{self, Moment};
use crate::model::{self, Kept, Network};
use crate::room;
use crate::serve as served;
use crate::sources::{airplay, hp, modem, samsung, sony, switch};
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
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .ok_or("neither XDG_STATE_HOME nor HOME is set")?
        .join("net");
    fs::create_dir_all(&state)
        .map_err(|error| format!("can't make net's state folder: {error}"))?;
    let day = state.join("day.jsonl");
    let socket = runtime.join("daemons/net.sock");
    let shared = Shared::new(served::DOMAIN, Box::new(|_| None));
    let held = served::Served::new(Arc::clone(&shared));
    // The uplink is read on a thread of its own, only while it's watched,
    // and only on Linux, whose sock_diag says who owns each socket.
    #[cfg(target_os = "linux")]
    {
        let (heard, woken) = std::sync::mpsc::channel();
        shared.on_watches(Box::new(move || {
            let _ = heard.send(());
        }));
        let held = Arc::clone(&held);
        thread::spawn(move || served::watch_uplink(&held, &woken));
    }
    let listener = serve::listen(&socket)
        .map_err(|error| format!("can't listen on {}: {error}", socket.display()))?;
    serve::accept(Arc::clone(&shared), listener);
    let agent: Agent = Agent::config_builder()
        .timeout_global(Some(PAGE))
        .build()
        .into();
    // A receiver's answers name where it's asked next; sony.rs keeps only
    // those on the receiver itself, and this agent never follows a
    // redirect, so no answer can send a request elsewhere. A printer's
    // pages and an Apple TV's and a TV's answers are asked with it too, so
    // a redirect can't lead one away.
    let at_home: Agent = Agent::config_builder()
        .timeout_global(Some(PAGE))
        .max_redirects(0)
        .build()
        .into();
    // Each TV's last answer, so a minute it's silent keeps its name.
    let mut tvs = samsung::Heard::new();
    // Each source as the last record said it.
    let mut before = last_sources(&record);
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
        let apple_tvs = heard
            .as_ref()
            .map(airplay::listed)
            .unwrap_or_default()
            .into_iter()
            .map(|(mac, address)| (mac, read_apple_tv(&at_home, &address)))
            .collect();
        let listed = heard.as_ref().map(samsung::listed);
        // Only a minute eero answered says which TVs have left.
        if let Ok(listed) = &listed {
            samsung::forget_unlisted(&mut tvs, listed);
        }
        let televisions = listed
            .unwrap_or_default()
            .into_iter()
            .map(|(mac, address)| {
                samsung::remember(&mut tvs, mac, read_television(&at_home, &address))
            })
            .collect();
        let said = Said {
            eero: heard,
            modem: read_modem(&agent),
            switch: read_switch(Path::new(switch::REPORT)),
            receivers,
            printers,
            apple_tvs,
            televisions,
        };
        let mut network = collect::network(said, time::now());
        model::carry_over(&mut network.sources, &before);
        model::keep(&mut before, &network.sources, network.collected_at);
        let earlier = history::read(&day, network.collected_at);
        history::recall(&mut network, &earlier);
        room::work_out(&mut network);
        // A day that can't be kept costs what net says of the day, never
        // the record itself.
        if let Err(error) = history::keep(&day, &Moment::of(&network)) {
            eprintln!("can't keep the minute's figures: {error}");
        }
        let written = serde_json::to_string_pretty(&network).map_err(|error| error.to_string())?;
        write(&record, &(written + "\n"))
            .map_err(|error| format!("can't write the record: {error}"))?;
        held.network(&network);
        thread::sleep(EVERY);
    }
}

/// Each source as the record a daemon before this one wrote said it, so a
/// restart doesn't make an outage begin again. A record older than
/// `LATELY` says nothing of what happened since it was written, so it
/// carries nothing over, nor does one dated ahead of the clock, one of
/// another schema, or one that can't be read.
fn last_sources(record: &Path) -> Vec<Kept> {
    let now = time::now();
    fs::read_to_string(record)
        .ok()
        .and_then(|text| serde_json::from_str::<Network>(&text).ok())
        .filter(|network| network.schema == model::SCHEMA)
        .filter(|network| {
            now.checked_sub(network.collected_at)
                .is_some_and(|age| age <= model::LATELY)
        })
        .map(|network| {
            let at = network.collected_at;
            network
                .sources
                .into_iter()
                .map(|source| (at, source))
                .collect()
        })
        .unwrap_or_default()
}

fn read_eero(file: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(file).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => {
            "eero hasn't written what it read; see systemctl --user status eero-daemon".to_owned()
        }
        _ => format!("what eero read can't be opened: {error}"),
    })?;
    serde_json::from_str(&text).map_err(|error| format!("what eero read doesn't parse: {error}"))
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

/// What an Apple TV says of itself, as its bytes: a property list, which
/// airplay.rs reads.
fn read_apple_tv(agent: &Agent, address: &str) -> Result<airplay::Answer, String> {
    agent
        .get(&airplay::url(address))
        .call()
        .and_then(|mut answer| answer.body_mut().read_to_vec())
        .map_err(|error| format!("it didn't answer: {error}"))
}

/// What a Samsung TV says of itself, as its text, which samsung.rs reads,
/// no longer than a TV's answer could be. An error status, a redirect or
/// a page past that is the box answering with something else, so a TV
/// that starts refusing isn't taken for one asleep.
fn read_television(agent: &Agent, address: &str) -> samsung::Reply {
    let read = agent
        .get(&samsung::url(address))
        .call()
        .and_then(|mut answer| {
            answer
                .body_mut()
                .with_config()
                .limit(samsung::LONGEST)
                .read_to_string()
        });
    match read {
        Ok(page) => samsung::Reply::Page(page),
        Err(ureq::Error::StatusCode(status)) => {
            samsung::Reply::Refused(format!("it says {status}"))
        }
        Err(ureq::Error::TooManyRedirects) => {
            samsung::Reply::Refused("it sends it elsewhere".into())
        }
        Err(ureq::Error::BodyExceedsLimit(_)) => {
            samsung::Reply::Refused("it says more than a TV does".into())
        }
        Err(error) => samsung::Reply::Silent(format!("it didn't answer: {error}")),
    }
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
