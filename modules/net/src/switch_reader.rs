//! The switch's reader: what the switch's system service runs, apart from
//! net-daemon, since signing in to the switch takes its admin password,
//! which lets its holder change the switch. Each minute it reads the four
//! status pages, and signs in only when they come back holding no status,
//! as they do once the switch's session has ended: the sign-in is plain
//! HTTP, so each one puts the password on the home network in the clear.
//! It writes only what the pages show, as a report (sources/switch.rs),
//! for net-daemon to read; never the password or the sign-in.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use ureq::Agent;

use crate::model::SourceState;
use crate::sources::switch::{self, Pages, Report};
use crate::time;

/// How often the switch is read, as often as net-daemon reads its report.
const EVERY: Duration = Duration::from_secs(60);

/// How long one page may take. The switch's web server is slow and at
/// times sends nothing, so each page is asked for again once.
const PAGE: Duration = Duration::from_secs(5);

/// The switch's one account; its name can't be changed on these switches'
/// status pages, only its password.
const USER: &str = "admin";

pub fn run(address: &str) -> Result<(), String> {
    let folder = std::env::var_os("RUNTIME_DIRECTORY").map_or_else(
        || {
            Path::new(switch::REPORT)
                .parent()
                .map_or_else(PathBuf::new, Path::to_path_buf)
        },
        PathBuf::from,
    );
    let report = folder.join("report.json");
    let agent = agent();
    loop {
        let said = round(&agent, address, password().as_deref(), time::now());
        let written = serde_json::to_string_pretty(&said).map_err(|error| error.to_string())?;
        write(&report, &(written + "\n"))
            .map_err(|error| format!("can't write the report: {error}"))?;
        thread::sleep(EVERY);
    }
}

fn agent() -> Agent {
    Agent::config_builder()
        .timeout_global(Some(PAGE))
        .user_agent("Mozilla/5.0")
        // Signed out, the switch answers 401 with its sign-in page, which
        // is an answer, not a failure: the parse tells it from a status.
        .http_status_as_error(false)
        .build()
        .into()
}

/// The password systemd hands the service, if it's been given one.
fn password() -> Option<String> {
    let folder = std::env::var_os("CREDENTIALS_DIRECTORY")?;
    let said = fs::read_to_string(Path::new(&folder).join("switch-password")).ok()?;
    let said = said.trim_end_matches(['\n', '\r']).to_owned();
    (!said.is_empty()).then_some(said)
}

/// One round: the pages as they come, and a sign-in and a second read only
/// when they hold no status.
fn round(agent: &Agent, address: &str, password: Option<&str>, now: u64) -> Report {
    let report = |state, reason: Option<String>, status| Report {
        schema: switch::SCHEMA.into(),
        read_at: now,
        state,
        reason,
        status,
    };
    let first = match pages(agent, address) {
        Ok(first) => first,
        Err(why) => return report(SourceState::Unreachable, Some(why), None),
    };
    if let Ok(status) = switch::parse(&first) {
        return report(SourceState::Answering, None, Some(status));
    }
    let Some(password) = password else {
        return report(
            SourceState::SignedOut,
            Some("no password is set for it".into()),
            None,
        );
    };
    if let Err(why) = sign_in(agent, address, password) {
        return report(SourceState::Unreachable, Some(why), None);
    }
    match pages(agent, address).map(|pages| switch::parse(&pages)) {
        Ok(Ok(status)) => report(SourceState::Answering, None, Some(status)),
        Ok(Err(why)) => report(
            SourceState::SignedOut,
            Some(format!("its sign-in was refused: {why}")),
            None,
        ),
        Err(why) => report(SourceState::Unreachable, Some(why), None),
    }
}

/// Each status page, one at a time, since the switch drops a reply when
/// asked for two at once.
fn pages(agent: &Agent, address: &str) -> Result<Pages, String> {
    switch::PAGES
        .iter()
        .map(|page| {
            let url = format!("http://{address}/{page}.htm");
            let read = || {
                agent
                    .get(&url)
                    .call()
                    .and_then(|mut answer| answer.body_mut().read_to_string())
                    .ok()
                    .filter(|text| !text.is_empty())
            };
            read()
                .or_else(read)
                .map(|text| ((*page).to_owned(), text))
                .ok_or_else(|| format!("its {page} page didn't answer"))
        })
        .collect()
}

/// Signs in. The switch ties the session to the address asking, and its
/// answer says nothing reliable, so the next read is what tells.
fn sign_in(agent: &Agent, address: &str, password: &str) -> Result<(), String> {
    agent
        .post(&format!("http://{address}/logon.cgi"))
        .send_form([
            ("username", USER),
            ("password", password),
            ("logon", "Login"),
        ])
        .map(drop)
        // Only the kind of failure is said: ureq's own words could carry
        // what was sent.
        .map_err(|_| "its sign-in page didn't answer".to_owned())
}

/// Writes a file whole, so net-daemon never reads half of one.
fn write(file: &Path, contents: &str) -> io::Result<()> {
    let fresh = file.with_extension("json.new");
    fs::write(&fresh, contents)?;
    fs::rename(fresh, file)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    const PASSWORD: &str = "a test password & more";

    /// A stand-in switch on a loopback port: it serves the contract's pages
    /// once signed in with `PASSWORD`, and the sign-in form until then, and
    /// keeps every request it was sent.
    fn switch_on_loopback(signed_in: bool) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let address = listener.local_addr().expect("its address").to_string();
        let heard = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&heard);
        thread::spawn(move || {
            let mut session = signed_in;
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                let mut reader = BufReader::new(stream.try_clone().expect("a second handle"));
                let mut request = String::new();
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        length = value.trim().parse().unwrap_or(0);
                    }
                    request.push_str(&line);
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).expect("the body");
                let body = String::from_utf8_lossy(&body).into_owned();
                request.push_str(&body);
                let path = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
                kept.lock().expect("the log").push(request);
                if path == "/logon.cgi" {
                    session = body.contains("password=a+test+password+%26+more");
                }
                let page = path.trim_start_matches('/').trim_end_matches(".htm");
                let text = if session && switch::PAGES.contains(&page) {
                    fs::read_to_string(format!(
                        "{}/contract/switch/{page}.htm",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                    .expect("the contract has the page")
                } else {
                    "<html><form action=logon.cgi></form></html>".to_owned()
                };
                let _ = write!(
                    stream,
                    "HTTP/1.0 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{text}",
                    text.len()
                );
            }
        });
        (address, heard)
    }

    fn signed_in(heard: &Arc<Mutex<Vec<String>>>) -> usize {
        heard
            .lock()
            .expect("the log")
            .iter()
            .filter(|request| request.starts_with("POST /logon.cgi"))
            .count()
    }

    #[test]
    fn with_no_password_it_says_signed_out_and_never_signs_in() {
        let (address, heard) = switch_on_loopback(false);
        let report = round(&agent(), &address, None, 10);
        assert_eq!(report.state, SourceState::SignedOut);
        assert_eq!(report.reason.as_deref(), Some("no password is set for it"));
        assert_eq!(signed_in(&heard), 0, "it signed in with nothing");
    }

    #[test]
    fn it_signs_in_once_when_the_session_has_ended_and_reads_the_status() {
        let (address, heard) = switch_on_loopback(false);
        let report = round(&agent(), &address, Some(PASSWORD), 10);
        assert_eq!(report.state, SourceState::Answering, "{:?}", report.reason);
        assert_eq!(report.status.map(|status| status.ports.len()), Some(8));
        assert_eq!(signed_in(&heard), 1);
    }

    #[test]
    fn while_the_session_lasts_it_doesnt_sign_in_again() {
        let (address, heard) = switch_on_loopback(true);
        let report = round(&agent(), &address, Some(PASSWORD), 10);
        assert_eq!(report.state, SourceState::Answering, "{:?}", report.reason);
        assert_eq!(signed_in(&heard), 0, "it signed in over a live session");
    }

    #[test]
    fn a_refused_password_reads_as_signed_out() {
        let (address, _) = switch_on_loopback(false);
        let report = round(&agent(), &address, Some("not it"), 10);
        assert_eq!(report.state, SourceState::SignedOut);
    }

    #[test]
    fn the_password_is_in_no_report() {
        let (address, _) = switch_on_loopback(false);
        for password in [PASSWORD, "not it"] {
            let report = round(&agent(), &address, Some(password), 10);
            let written = serde_json::to_string(&report).expect("it writes");
            assert!(!written.contains(password), "the report holds the password");
        }
    }

    #[test]
    fn a_switch_that_doesnt_answer_reads_as_unreachable() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let address = listener.local_addr().expect("its address").to_string();
        drop(listener);
        let report = round(&agent(), &address, Some(PASSWORD), 10);
        assert_eq!(report.state, SourceState::Unreachable);
        assert!(
            !serde_json::to_string(&report)
                .expect("it writes")
                .contains(PASSWORD)
        );
    }
}
