//! eero: the home network's eero, read from this machine at home or away.
//!
//! The eero has no local API, so everything goes through eero's cloud
//! (cloud.rs), under a session Joel makes once by typing the code eero sends
//! him (session.rs). It reads, and changes one setting: "Optimize for
//! conferencing and gaming".
//!
//! Usage:
//!   eero sign-in <email or phone>   sign in once, typing the code eero sends
//!   eero status [--json]            each network: online, speed, setting, devices
//!   eero devices                    what's connected now
//!   eero speedtest                  start the eero's own speed test
//!   eero sqm on|off                 optimize for conferencing and gaming, or not
//!   eero daemon                     keep the home network for the shell

mod api;
mod cloud;
mod daemon;
#[cfg(test)]
mod golden;
mod listeners;
mod model;
mod read;
mod schedule;
mod session;
mod status;
mod time;

use std::io::{self, BufRead, Write};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use cloud::Cloud;

const USAGE: &str = "usage: eero sign-in <email or phone> | status [--json] | devices | speedtest | \
                     sqm on|off | daemon";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    let done = match words.as_slice() {
        ["sign-in", login] => sign_in(login),
        ["status"] => show(false),
        ["status", "--json"] => show(true),
        ["devices"] => devices(),
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

/// Every network on the account, with how many devices each has on it.
fn read(cloud: &mut Cloud) -> Result<Vec<(api::Network, Option<usize>)>, String> {
    let account = cloud.get(api::ACCOUNT)?;
    let mut read = Vec::new();
    for url in api::networks(&account) {
        let network = api::network(&url, &cloud.get(&url)?);
        let devices = match &network.devices_url {
            Some(list) => Some(api::devices(&cloud.get(list)?).len()),
            None => None,
        };
        read.push((network, devices));
    }
    Ok(read)
}

fn show(json: bool) -> Result<(), String> {
    let mut cloud = Cloud::signed_in()?;
    let networks = read(&mut cloud)?;
    if json {
        println!("{}", status::file(now(), &networks));
    } else {
        for (network, devices) in &networks {
            println!("{}", status::line(network, *devices));
        }
    }
    Ok(())
}

fn devices() -> Result<(), String> {
    let mut cloud = Cloud::signed_in()?;
    for (network, _) in read(&mut cloud)? {
        let Some(list) = &network.devices_url else {
            continue;
        };
        for device in api::devices(&cloud.get(list)?) {
            let ip = device.ip.as_deref().unwrap_or("-");
            let link = if device.wired { "wired" } else { "wireless" };
            println!("{}  {}  {ip}  {link}", network.name, device.name);
        }
    }
    Ok(())
}

fn speedtest() -> Result<(), String> {
    let mut cloud = Cloud::signed_in()?;
    for url in api::networks(&cloud.get(api::ACCOUNT)?) {
        cloud.post(&format!("{url}/speedtest"), None)?;
    }
    println!("The eero is testing its line; `eero status` shows the result in about a minute.");
    Ok(())
}

/// Turns "Optimize for conferencing and gaming" on or off on every
/// network, and says each one's line once eero reads the change back.
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

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}
