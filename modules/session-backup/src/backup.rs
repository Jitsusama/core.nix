//! One run: every settled log not yet in the ledger, sealed and sent.

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::Config;
use crate::drive::{self, Drive, Fault};
use crate::ledger::Ledger;
use crate::names::{self, Settled};
use crate::seal::{self, Recipients};
use crate::token::Login;

/// What a run did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Done {
    /// Logs sealed and sent.
    pub sent: usize,
    /// Logs Drive held already, from a run stopped before it said so.
    pub found: usize,
    /// Sealed bytes sent.
    pub bytes: u64,
    /// Settled logs sent before this run.
    pub before: usize,
}

/// Seal and send every settled log the ledger lacks, oldest first, up to
/// `most` of them when given. Stops at the first log that can't be sent,
/// since what stops one, such as the login, stops the rest; the next run
/// carries on from there.
pub fn run(config: &Config, now: SystemTime, most: Option<usize>) -> Result<Done, String> {
    let mut ledger = Ledger::open(&config.ledger)?;
    let settled = names::settled(&config.sessions, &config.host, config.settled, now)?;
    let waiting: Vec<&Settled> = settled
        .iter()
        .filter(|log| !ledger.has(&log.name))
        .collect();
    let mut done = Done {
        before: settled.len() - waiting.len(),
        ..Done::default()
    };
    if waiting.is_empty() {
        return Ok(done);
    }
    let recipients = Recipients::parse(&config.recipients)?;
    let mut login = Login::read(&config.credentials, &config.token)?;
    let agent = drive::agent();
    let drive = Drive {
        agent: &agent,
        api: &config.api,
        folder: &config.folder,
    };
    let scratch = config.ledger.with_extension("sealing");
    if let Some(state) = scratch.parent() {
        fs::create_dir_all(state)
            .map_err(|why| format!("can't make {}: {why}", state.display()))?;
    }
    for log in waiting.into_iter().take(most.unwrap_or(usize::MAX)) {
        let at = seconds(SystemTime::now());
        if let Some(id) = again(&mut login, &agent, |token| drive.find(token, &log.name))? {
            ledger.record(&log.name, &id, 0, at)?;
            done.found += 1;
            continue;
        }
        let bytes = seal::seal(&log.path, &scratch, &recipients)?;
        let sent = again(&mut login, &agent, |token| {
            drive.create(token, &log.name, &scratch, bytes)
        });
        // The sealed copy is scratch whether it went or not.
        let _ = fs::remove_file(&scratch);
        let id = sent?;
        ledger.record(&log.name, &id, bytes, at)?;
        done.sent += 1;
        done.bytes += bytes;
    }
    Ok(done)
}

/// A call to Drive with a token, made once more with a fresh token when
/// Drive says the one it had stopped counting.
fn again<T>(
    login: &mut Login,
    agent: &ureq::Agent,
    call: impl Fn(&str) -> Result<T, Fault>,
) -> Result<T, String> {
    match call(&login.token(agent)?) {
        Err(Fault::Unauthorized) => {
            login.forget();
            call(&login.token(agent)?).map_err(|fault| fault.to_string())
        }
        other => other.map_err(|fault| fault.to_string()),
    }
}

fn seconds(at: SystemTime) -> u64 {
    at.duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}
