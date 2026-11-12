//! eero: the home network's eero, read from this machine at home or away.
//!
//! The eero has no local API, so everything goes through eero's cloud
//! (cloud.rs), under a session Joel makes once by typing the code eero sends
//! him (session.rs). It reads, and changes one setting: "Optimize for
//! conferencing and gaming".
//!
//! The views read the record the daemon keeps of the home network
//! (live.rs, views/), and never eero's cloud themselves, so a view is
//! instant and says how old what it shows is.
//!
//! Usage:
//!   eero sign-in <email or phone>   sign in once, typing the code eero sends
//!   eero <view> [--json]            a view of the home network, below
//!   eero watch <view> [--json]      the view, drawn again as it changes
//!   eero speedtest                  start the eero's own speed test
//!   eero sqm on|off                 optimize for conferencing and gaming, or not
//!   eero daemon                     keep the home network for the shell
//!
//! The views: status, nodes, node <location|id>, mesh, devices,
//! device <name|mac>, internet, speeds, firmware, radio, traffic, profiles,
//! profile <name|id> and events.
//!
//! For the shell: `eero completion zsh` writes zsh's completion, and
//! `eero names views|nodes|devices|profiles` what it offers of each.

mod api;
mod called;
mod cloud;
mod daemon;
mod day;
mod desktop;
mod drawers;
#[cfg(test)]
mod golden;
mod history;
mod listeners;
mod live;
mod model;
mod notices;
mod place;
mod probe;
mod read;
mod rows;
mod schedule;
mod served;
mod session;
mod status;
mod time;
mod views;
mod waiting;
mod watch;

use std::io::{self, BufRead, IsTerminal, Write};
use std::process::ExitCode;

use cloud::Cloud;

const USAGE: &str = concat!(
    "usage: eero <view> [--json] | watch <view> [--json] | speedtest | sqm on|off |\n",
    "       sign-in <email or phone> | daemon\n\n",
    "views: status, nodes, node <location|id>, mesh, devices, device <name|mac>, internet,\n",
    "       speeds, firmware, radio, traffic, profiles, profile <name|id>, events",
);

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|arg| arg == "--json");
    let words: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|arg| *arg != "--json")
        .collect();
    let done = match words.as_slice() {
        [view, ..] if views::View::named(view) => views::View::parse(&words)
            .and_then(|view| {
                live::show(&view, json, &views::theme::ink(io::stdout().is_terminal()))
            })
            .and_then(|page| {
                io::stdout()
                    .write_all(page.as_bytes())
                    .map_err(|e| e.to_string())
            }),
        ["watch"] => Err("name a view to watch: eero watch <view>".into()),
        ["watch", view @ ..] => views::View::parse(view).and_then(|view| {
            let terminal = io::stdout().is_terminal();
            let how = watch::Watch {
                view: &view,
                json,
                ink: &views::theme::ink(terminal),
                terminal,
            };
            watch::watch(&how, &mut io::stdout().lock())
        }),
        ["completion", "zsh"] => io::stdout()
            .write_all(include_str!("completion.zsh").as_bytes())
            .map_err(|e| e.to_string()),
        ["names", kind] => names(kind),
        ["sign-in", login] => sign_in(login),
        ["speedtest"] => speedtest(),
        ["sqm", "on"] => conferencing(true),
        ["sqm", "off"] => conferencing(false),
        ["daemon"] => daemon::run(),
        _ => Err(USAGE.into()),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("eero: {reason}");
            ExitCode::FAILURE
        }
    }
}

/// What completion offers of a kind, as zsh reads it. The views need no
/// record; the rest are what the record holds now.
fn names(kind: &str) -> Result<(), String> {
    use views::names;
    let record = || live::read(&live::file()?);
    let named = match kind {
        "views" => names::views(),
        "nodes" => names::nodes(&record()?),
        "devices" => names::devices(&record()?),
        "profiles" => names::profiles(&record()?),
        _ => return Err("usage: eero names views|nodes|devices|profiles".into()),
    };
    io::stdout()
        .write_all(names::for_zsh(&named).as_bytes())
        .map_err(|e| e.to_string())
}

fn sign_in(login: &str) -> Result<(), String> {
    let mut cloud = Cloud::new();
    cloud.sign_in(login)?;
    eprint!("eero sent a code to {login}. Type it here: ");
    io::stderr().flush().map_err(|e| e.to_string())?;
    let mut code = String::new();
    io::stdin()
        .lock()
        .read_line(&mut code)
        .map_err(|e| e.to_string())?;
    cloud.verify(code.trim())?;
    eprintln!("Signed in; the session is sealed for this account on this machine.");
    Ok(())
}

fn speedtest() -> Result<(), String> {
    let mut cloud = Cloud::signed_in()?;
    for url in api::networks(&cloud.get(api::ACCOUNT)?) {
        cloud.post(&format!("{url}/speedtest"), None)?;
    }
    println!("The eero is testing its internet; `eero speeds` shows the result in about a minute.");
    Ok(())
}

/// Turns "Optimize for conferencing and gaming" on or off on every
/// network, and says each one's status once eero reads the change back.
fn conferencing(on: bool) -> Result<(), String> {
    let mut cloud = Cloud::signed_in()?;
    for url in api::networks(&cloud.get(api::ACCOUNT)?) {
        let network = api::network(&url, &cloud.get(&url)?);
        let settings = network
            .settings_url
            .ok_or_else(|| format!("eero names no settings for {}", network.name))?;
        cloud.put(&settings, &api::conferencing(on))?;
        let now = api::network(&url, &cloud.get(&url)?);
        api::taken(on, now.sqm)?;
        println!("{}", status::line(&now, None));
    }
    Ok(())
}
