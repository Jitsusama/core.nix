//! One run: every settled log not yet in the ledger, sealed and sent, then
//! every old one Drive holds removed from the machine.

use std::fs::{self, File, TryLockError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::config::Config;
use crate::drive::{self, Drive, Fault};
use crate::ledger::Ledger;
use crate::names::{self, Settled};
use crate::retry::{again, patiently};
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
    /// Why the run stopped early, when Drive stayed busy through every
    /// retry; the logs left wait for the next run.
    pub busy: Option<String>,
    /// Another run held the lock, so this one sent nothing.
    pub held: bool,
    /// Logs removed from the machine, since Drive holds them and they are
    /// older than the settings keep them for.
    pub removed: usize,
    /// The bytes those logs took.
    pub freed: u64,
}

/// Seal and send every settled log the ledger lacks, oldest first, up to
/// `most` of them when given. A call Drive is busy for is tried again
/// after each of the configured waits; when it stays busy, the run ends
/// there without failing, saying so in `busy`. Any other refusal fails
/// the run at that log, since what stops one, such as the login, stops
/// the rest. Either way the next run carries on from there.
///
/// Then, when the settings say how long to keep a log and Drive wasn't
/// busy, each log last written longer ago than that is removed from the
/// machine, once the ledger names its copy under its last write and Drive
/// still finds that copy by name. A log Drive no longer finds is kept, as
/// the only copy, and so is one written again since it was listed.
///
/// One run at a time: a run holds a lock beside the ledger from before it
/// reads the ledger until it is done, since two would share the scratch a
/// seal is written to and could each send the other's half-written copy.
/// A run that finds the lock held sends nothing and says so in `held`.
pub fn run(config: &Config, now: SystemTime, most: Option<usize>) -> Result<Done, String> {
    let Some(_lock) = lock(config)? else {
        return Ok(Done {
            held: true,
            ..Done::default()
        });
    };
    let mut ledger = Ledger::open(&config.ledger)?;
    let settled = names::settled(&config.sessions, &config.host, config.settled, now)?;
    let waiting: Vec<&Settled> = settled
        .iter()
        .filter(|log| !ledger.has(&log.name))
        .collect();
    let old: Vec<&Settled> = config.keep.map_or_else(Vec::new, |keep| {
        settled
            .iter()
            .filter(|log| {
                ledger.has(&log.name) && seconds(now).saturating_sub(log.written) >= keep.as_secs()
            })
            .collect()
    });
    let mut done = Done {
        before: settled.len() - waiting.len(),
        ..Done::default()
    };
    if waiting.is_empty() && old.is_empty() {
        return Ok(done);
    }
    let mut login = Login::read(&config.credentials, &config.token)?;
    let agent = drive::agent();
    let drive = Drive {
        agent: &agent,
        api: &config.api,
        folder: &config.folder,
    };
    let scratch = config.ledger.with_extension("sealing");
    let recipients = Recipients::parse(&config.recipients)?;
    for log in waiting.into_iter().take(most.unwrap_or(usize::MAX)) {
        let at = seconds(SystemTime::now());
        let held = patiently(&config.retries, || {
            again(&mut login, &agent, |token| drive.find(token, &log.name))
        });
        let held = match held {
            Err(Fault::Busy(why)) => {
                done.busy = Some(why);
                break;
            }
            other => other.map_err(|fault| fault.to_string())?,
        };
        if let Some(id) = held {
            ledger.record(&log.name, &id, 0, at)?;
            done.found += 1;
            continue;
        }
        let bytes = seal::seal(&log.path, &scratch, &recipients)?;
        let mut tried = false;
        let sent = patiently(&config.retries, || {
            // A send Drive answered busy may still have landed, so a retry
            // looks before it sends again, and never makes a second copy.
            if std::mem::replace(&mut tried, true)
                && let Some(id) = again(&mut login, &agent, |token| drive.find(token, &log.name))?
            {
                return Ok((id, false));
            }
            again(&mut login, &agent, |token| {
                drive.create(token, &log.name, &scratch, bytes)
            })
            .map(|id| (id, true))
        });
        // The sealed copy is scratch whether it went or not.
        let _ = fs::remove_file(&scratch);
        match sent {
            Err(Fault::Busy(why)) => {
                done.busy = Some(why);
                break;
            }
            Err(fault) => return Err(fault.to_string()),
            Ok((id, false)) => {
                ledger.record(&log.name, &id, 0, at)?;
                done.found += 1;
            }
            Ok((id, true)) => {
                ledger.record(&log.name, &id, bytes, at)?;
                done.sent += 1;
                done.bytes += bytes;
            }
        }
    }
    if done.busy.is_none() {
        remove_held(&old, &drive, &mut login, &agent, &config.retries, &mut done)?;
    }
    Ok(done)
}

/// Remove each of `old` from the machine that Drive still finds by name and
/// that wasn't written again since it was listed. Drive busy through every
/// retry ends the removing there, saying so in `busy`.
fn remove_held(
    old: &[&Settled],
    drive: &Drive<'_>,
    login: &mut Login,
    agent: &ureq::Agent,
    retries: &[Duration],
    done: &mut Done,
) -> Result<(), String> {
    for log in old {
        let held = match patiently(retries, || {
            again(login, agent, |token| drive.find(token, &log.name))
        }) {
            Err(Fault::Busy(why)) => {
                done.busy = Some(why);
                break;
            }
            other => other.map_err(|fault| fault.to_string())?,
        };
        // Drive no longer finds the copy, so this is the only one.
        if held.is_none() || !unchanged(log) {
            continue;
        }
        match fs::remove_file(&log.path) {
            Ok(()) => {
                done.removed += 1;
                done.freed += log.bytes;
            }
            // Gone since it was listed: nothing is left to remove.
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
            Err(why) => return Err(format!("can't remove {}: {why}", log.path.display())),
        }
    }
    Ok(())
}

/// Whether a log is as it was listed: its last write and length the same.
fn unchanged(log: &Settled) -> bool {
    fs::metadata(&log.path).is_ok_and(|meta| {
        meta.len() == log.bytes && meta.modified().is_ok_and(|at| seconds(at) == log.written)
    })
}

/// The run's lock, held until it is dropped, or `None` when another run
/// holds it.
fn lock(config: &Config) -> Result<Option<File>, String> {
    let path = config.ledger.with_extension("lock");
    if let Some(state) = path.parent() {
        fs::create_dir_all(state)
            .map_err(|why| format!("can't make {}: {why}", state.display()))?;
    }
    let file =
        File::create(&path).map_err(|why| format!("can't open {}: {why}", path.display()))?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(why)) => Err(format!("can't lock {}: {why}", path.display())),
    }
}

fn seconds(at: SystemTime) -> u64 {
    at.duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}
