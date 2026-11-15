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

/// How long one page may take.
const PAGE: Duration = Duration::from_secs(5);

/// How long the sign-in may take. The real switch answered its sign-in
/// as a failure in 13 of 25 rounds on 15 Nov, far more often than it drops
/// a page, so it may take longer over it than over a page; a slow answer
/// needn't end the round either, since the next read is what tells.
const SIGN_IN: Duration = Duration::from_secs(20);

/// How many times each request is made before the switch reads as
/// unreachable, and the pause between them. The switch's web server closes
/// about one connection in four without a reply, at times twice running
/// (5 of 20 asks on 15 Nov), so a round of nine requests asked twice each
/// mostly lost one; five tries leave each about a 1 in 1000 chance.
const TRIES: usize = 5;
const PAUSE: Duration = Duration::from_millis(300);

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
/// when they hold no status. A sign-in the switch answered and the pages
/// still hold no status was refused, so the password isn't sent again; one
/// it didn't answer may still have started the session, so only the read
/// after it says whether to send it again.
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
    let mut unanswered = "";
    for _ in 0..TRIES {
        let answered = sign_in(agent, address, password);
        match pages(agent, address).map(|pages| switch::parse(&pages)) {
            Ok(Ok(status)) => return report(SourceState::Answering, None, Some(status)),
            Ok(Err(why)) if answered.is_ok() => {
                return report(
                    SourceState::SignedOut,
                    Some(format!("its sign-in was refused: {why}")),
                    None,
                );
            }
            Ok(Err(_)) => unanswered = answered.err().unwrap_or_default(),
            Err(why) => return report(SourceState::Unreachable, Some(why), None),
        }
        thread::sleep(PAUSE);
    }
    report(
        SourceState::Unreachable,
        Some(format!("its sign-in didn't answer: {unanswered}")),
        None,
    )
}

/// Each status page, one at a time, since the switch drops a reply when
/// asked for two at once.
fn pages(agent: &Agent, address: &str) -> Result<Pages, String> {
    switch::PAGES
        .iter()
        .map(|page| {
            let url = format!("http://{address}/{page}.htm");
            again(|| {
                let text = agent
                    .get(&url)
                    .call()
                    .and_then(|mut answer| answer.body_mut().read_to_string())
                    .map_err(|error| kind(&error))?;
                if text.is_empty() {
                    Err("it sent an empty page")
                } else {
                    Ok(text)
                }
            })
            .map(|text| ((*page).to_owned(), text))
            .map_err(|why| format!("its {} page didn't answer: {why}", switch::plainly(page)))
        })
        .collect()
}

/// Signs in, once. The switch ties the session to the address asking, and
/// its answer says nothing reliable, so the next read is what tells.
fn sign_in(agent: &Agent, address: &str, password: &str) -> Result<(), &'static str> {
    agent
        .post(format!("http://{address}/logon.cgi"))
        .config()
        .timeout_global(Some(SIGN_IN))
        .build()
        .send_form([
            ("username", USER),
            ("password", password),
            ("logon", "Login"),
        ])
        .map(drop)
        .map_err(|error| kind(&error))
}

/// What kind of failure a request met, in net's words. Only the kind is
/// said: ureq's own words could carry what was sent, the password included.
fn kind(error: &ureq::Error) -> &'static str {
    use io::ErrorKind;
    match error {
        ureq::Error::Timeout(_) => "it took too long",
        ureq::Error::Io(error) => match error.kind() {
            ErrorKind::ConnectionRefused => "it refused the connection",
            ErrorKind::ConnectionReset
            | ErrorKind::ConnectionAborted
            | ErrorKind::UnexpectedEof => "it closed the connection without a reply",
            ErrorKind::TimedOut => "it took too long",
            _ => "the connection failed",
        },
        ureq::Error::Protocol(_)
        | ureq::Error::Http(_)
        | ureq::Error::LargeResponseHeader(..)
        | ureq::Error::BodyExceedsLimit(_) => "its answer couldn't be read",
        _ => "the connection failed",
    }
}

/// Makes one request until the switch answers it, at most `TRIES` times.
fn again<T>(mut ask: impl FnMut() -> Result<T, &'static str>) -> Result<T, &'static str> {
    for _ in 1..TRIES {
        if let Ok(answer) = ask() {
            return Ok(answer);
        }
        thread::sleep(PAUSE);
    }
    ask()
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
        stand_in(signed_in, Behaves::Well)
    }

    /// How the stand-in's web server behaves.
    #[derive(Clone, Copy, PartialEq)]
    enum Behaves {
        Well,
        /// It closes about one connection in three without a reply, at
        /// random but the same each run, before acting on what was asked, as
        /// the real switch's web server closes about one in four. A fixed
        /// rhythm would fall in step with the round's requests.
        Flakily,
        /// It starts the session, then answers the sign-in only after longer
        /// than a page may take.
        SlowlyOverTheSignIn,
        /// It starts the session, then closes the sign-in's connection
        /// without a reply.
        SilentlyOverTheSignIn,
    }

    fn stand_in(signed_in: bool, behaves: Behaves) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let address = listener.local_addr().expect("its address").to_string();
        let heard = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&heard);
        thread::spawn(move || {
            let mut session = signed_in;
            let mut dice: u32 = 0x9e37_79b9;
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
                dice ^= dice << 13;
                dice ^= dice >> 17;
                dice ^= dice << 5;
                if behaves == Behaves::Flakily && dice.is_multiple_of(3) {
                    kept.lock()
                        .expect("the log")
                        .push(format!("dropped {request}"));
                    continue;
                }
                let path = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
                kept.lock().expect("the log").push(request);
                if path == "/logon.cgi" {
                    session = body.contains("password=a+test+password+%26+more");
                    match behaves {
                        Behaves::SlowlyOverTheSignIn => thread::sleep(PAGE + PAUSE),
                        Behaves::SilentlyOverTheSignIn => continue,
                        _ => {}
                    }
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
    fn a_switch_that_drops_connections_is_asked_again_until_it_answers() {
        let (address, heard) = stand_in(false, Behaves::Flakily);
        let report = round(&agent(), &address, Some(PASSWORD), 10);
        assert_eq!(report.state, SourceState::Answering, "{:?}", report.reason);
        assert_eq!(report.status.map(|status| status.ports.len()), Some(8));
        assert_eq!(signed_in(&heard), 1, "a sign-in it answered was sent again");
        let dropped = |what: &str| {
            heard
                .lock()
                .expect("the log")
                .iter()
                .any(|request| request.starts_with(&format!("dropped {what}")))
        };
        assert!(dropped("GET"), "the stand-in dropped no page");
        assert!(dropped("POST"), "the stand-in dropped no sign-in");
    }

    #[test]
    fn a_sign_in_answered_slowly_still_reads_the_status() {
        let (address, heard) = stand_in(false, Behaves::SlowlyOverTheSignIn);
        let report = round(&agent(), &address, Some(PASSWORD), 10);
        assert_eq!(report.state, SourceState::Answering, "{:?}", report.reason);
        assert_eq!(
            signed_in(&heard),
            1,
            "it signed in again over a session it had"
        );
    }

    #[test]
    fn a_sign_in_closed_without_a_reply_is_judged_by_the_next_read() {
        let (address, heard) = stand_in(false, Behaves::SilentlyOverTheSignIn);
        let report = round(&agent(), &address, Some(PASSWORD), 10);
        assert_eq!(report.state, SourceState::Answering, "{:?}", report.reason);
        assert_eq!(
            signed_in(&heard),
            1,
            "it signed in again over a session it had"
        );
    }

    #[test]
    fn a_refused_password_reads_as_signed_out_and_is_sent_once() {
        let (address, heard) = switch_on_loopback(false);
        let report = round(&agent(), &address, Some("not it"), 10);
        assert_eq!(report.state, SourceState::SignedOut);
        assert_eq!(signed_in(&heard), 1, "a refused password was sent again");
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
        assert_eq!(
            report.reason.as_deref(),
            Some("its System Info page didn't answer: it refused the connection")
        );
        let written = serde_json::to_string(&report).expect("it writes");
        assert!(
            !written.contains(PASSWORD),
            "the password reached the report: {written}"
        );
    }
}
