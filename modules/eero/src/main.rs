//! eero: the home network's eero, read from this machine at home or away.
//!
//! The eero has no local API, so everything goes through eero's cloud
//! (cloud.rs), under a session Joel makes once by typing the code eero sends
//! him (session.rs). It reads; changing a setting comes later.
//!
//! Usage:
//!   eero sign-in <email or phone>   sign in once, typing the code eero sends
//!   eero status [--json]            each network: online, speed, setting, devices
//!   eero devices                    what's connected now
//!   eero speedtest                  start the eero's own speed test
//!   eero watch [SECONDS]            keep the shell's status file, every 300 s

mod api;
mod cloud;
mod session;
mod status;

use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cloud::Cloud;

const USAGE: &str = "usage: eero sign-in <email or phone> | status [--json] | devices | speedtest | \
                     watch [SECONDS]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    let done = match words.as_slice() {
        ["sign-in", login] => sign_in(login),
        ["status"] => show(false),
        ["status", "--json"] => show(true),
        ["devices"] => devices(),
        ["speedtest"] => speedtest(),
        ["watch"] => watch(Duration::from_secs(300)),
        ["watch", secs] => match secs.parse() {
            Ok(secs) => watch(Duration::from_secs(secs)),
            Err(_) => Err(USAGE.into()),
        },
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

/// Keeps the shell's status file current, every `every`. A read that fails
/// is written as the file's error rather than ending the watch, since the
/// line being down is what the file is for.
fn watch(every: Duration) -> Result<(), String> {
    let file = runtime()?.join("eero/status.json");
    let mut cloud = Cloud::signed_in()?;
    loop {
        let at = now();
        let contents = match read(&mut cloud) {
            Ok(networks) => status::file(at, &networks),
            Err(reason) => status::failed(at, &reason),
        };
        write(&file, &contents.to_string()).map_err(|e| e.to_string())?;
        std::thread::sleep(every);
    }
}

fn runtime() -> Result<PathBuf, String> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| "XDG_RUNTIME_DIR isn't set".into())
}

/// Writes a file whole, so the shell never reads half of one.
fn write(file: &std::path::Path, contents: &str) -> io::Result<()> {
    let folder = file.parent().ok_or_else(|| io::Error::other("no folder"))?;
    fs::create_dir_all(folder)?;
    let fresh = file.with_extension("json.new");
    fs::write(&fresh, contents)?;
    fs::rename(fresh, file)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}
