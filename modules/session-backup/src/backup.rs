//! One run: every settled log not yet in the ledger, sealed and sent, then
//! the index's rows for the logs that have none, some of them read back
//! from Drive, then every old one Drive holds removed from the machine.

use std::fs::{self, File, TryLockError};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::config::Config;
use crate::drive::{self, Drive, Fault};
use crate::fetch::{self, Reading};
use crate::index::{self, Index};
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
    /// Logs given a row in the index, sent sealed as one object.
    pub indexed: usize,
    /// Why no copy could be read back for its row, when none could; the
    /// rest of the run goes on.
    pub unread: Option<String>,
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
/// Then, when the settings name an index and Drive wasn't busy, every log
/// the ledger names that has no row yet, those sent before there was an
/// index among them, gets one, and the rows go as one sealed object. So
/// do the rows of up to `backfill` logs gone from the machine that have
/// none, each read back from its copy and removed once read.
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
    let mut index = config.index.as_deref().map(Index::open).transpose()?;
    let mut done = Done {
        before: settled.len() - waiting.len(),
        ..Done::default()
    };
    let backlog = index.as_ref().is_some_and(|index| {
        !unindexed(&settled, &ledger, index).is_empty()
            || !gone(config, &settled, &ledger, index).is_empty()
    });
    if waiting.is_empty() && old.is_empty() && !backlog {
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
    let sending = Sending {
        drive: &drive,
        agent: &agent,
        retries: &config.retries,
        recipients: &recipients,
        scratch: &scratch,
    };
    for log in waiting.into_iter().take(most.unwrap_or(usize::MAX)) {
        send_log(log, &sending, &mut login, &mut ledger, &mut done)?;
        if done.busy.is_some() {
            break;
        }
    }
    if done.busy.is_none()
        && let Some(index) = index.as_mut()
    {
        let mut rows = rows(&unindexed(&settled, &ledger, index), config);
        let reading = Reading {
            drive: &drive,
            agent: &agent,
            retries: &config.retries,
            identities: &config.identities,
        };
        let gone = gone(config, &settled, &ledger, index);
        rows.extend(read_back(&gone, config, &reading, &mut login, &mut done));
        let name = batch_name(&config.host, seconds(now), &ledger);
        send_rows(
            &rows,
            &name,
            &sending,
            &mut login,
            &mut ledger,
            index,
            &mut done,
        )?;
    }
    if done.busy.is_none() {
        remove_held(&old, &drive, &mut login, &agent, &config.retries, &mut done)?;
    }
    Ok(done)
}

/// Seal and send one log, unless Drive holds it already from a run
/// stopped before the ledger said so. Drive busy through every retry says
/// so in `busy`, leaving the log for the next run.
fn send_log(
    log: &Settled,
    sending: &Sending<'_>,
    login: &mut Login,
    ledger: &mut Ledger,
    done: &mut Done,
) -> Result<(), String> {
    let Sending {
        drive,
        agent,
        retries,
        recipients,
        scratch,
    } = *sending;
    let at = seconds(SystemTime::now());
    let held = match patiently(retries, || {
        again(login, agent, |token| drive.find(token, &log.name))
    }) {
        Err(Fault::Busy(why)) => {
            done.busy = Some(why);
            return Ok(());
        }
        other => other.map_err(|fault| fault.to_string())?,
    };
    if let Some(id) = held {
        ledger.record(&log.name, &id, 0, at)?;
        done.found += 1;
        return Ok(());
    }
    let bytes = seal::seal(&log.path, scratch, recipients)?;
    let mut tried = false;
    let sent = patiently(retries, || {
        // A send Drive answered busy may still have landed, so a retry
        // looks before it sends again, and never makes a second copy.
        if std::mem::replace(&mut tried, true)
            && let Some(id) = again(login, agent, |token| drive.find(token, &log.name))?
        {
            return Ok((id, false));
        }
        again(login, agent, |token| {
            drive.create(token, &log.name, scratch, bytes)
        })
        .map(|id| (id, true))
    });
    // The sealed copy is scratch whether it went or not.
    let _ = fs::remove_file(scratch);
    match sent {
        Err(Fault::Busy(why)) => done.busy = Some(why),
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
    Ok(())
}

/// The name a run's rows go to Drive as, `<host>/index/<run>.jsonl.zst.age`,
/// or with `-2`, `-3` and on after the run's time when a run in the same
/// second sent rows already, since Drive would hold both under one name.
fn batch_name(host: &str, at: u64, ledger: &Ledger) -> String {
    let run = names::stamp(at);
    (1..)
        .map(|nth| match nth {
            1 => format!("{host}/index/{run}.jsonl.zst.age"),
            nth => format!("{host}/index/{run}-{nth}.jsonl.zst.age"),
        })
        .find(|name| !ledger.has(name))
        .unwrap_or_default()
}

/// The settled logs the ledger says were sent that have no row yet.
fn unindexed<'a>(settled: &'a [Settled], ledger: &Ledger, index: &Index) -> Vec<&'a Settled> {
    settled
        .iter()
        .filter(|log| ledger.has(&log.name) && !index.has(&log.name))
        .collect()
}

/// The objects the ledger names, by name and id, whose logs are no longer
/// on the machine and have no row, at most as many as the settings read
/// back a run, by name. The index's own objects are no logs.
fn gone<'a>(
    config: &Config,
    settled: &[Settled],
    ledger: &'a Ledger,
    index: &Index,
) -> Vec<(&'a str, &'a str)> {
    if config.identities.is_empty() {
        return Vec::new();
    }
    let rows = format!("{}/index/", config.host);
    let here: std::collections::HashSet<&str> =
        settled.iter().map(|log| log.name.as_str()).collect();
    let mut gone: Vec<(&str, &str)> = ledger
        .sent()
        .filter(|(name, id)| {
            !id.is_empty() && !name.starts_with(&rows) && !index.has(name) && !here.contains(name)
        })
        .collect();
    gone.sort_unstable();
    gone.truncate(config.backfill);
    gone
}

/// The row of each of `gone`, read from its copy, read back beside the
/// ledger and removed once read. Drive busy through every retry ends it
/// there, saying so in `busy`; a copy no key here opens ends it too,
/// saying so in `unread`, since what stops one stops the rest.
fn read_back(
    gone: &[(&str, &str)],
    config: &Config,
    reading: &Reading<'_>,
    login: &mut Login,
    done: &mut Done,
) -> Vec<Value> {
    let copy = config.ledger.with_extension("reading");
    let mut rows = Vec::new();
    for (name, id) in gone {
        // A copy left by a run that was stopped is no copy of anything.
        let _ = fs::remove_file(&copy);
        match fetch::read_back(reading, login, id, name, &copy) {
            Ok(bytes) => {
                if let Ok(row) = index::row(name, &copy, bytes, config.records.as_deref()) {
                    rows.push(row);
                }
            }
            Err(Fault::Busy(why)) => {
                done.busy = Some(why);
                break;
            }
            Err(fault) => {
                done.unread = Some(fault.to_string());
                break;
            }
        }
    }
    // The copy was only read for its row.
    let _ = fs::remove_file(&copy);
    rows
}

/// The row of each of `logs`. A log gone or unreadable since it was
/// listed has no row to give; a later run reads it again if it is back.
fn rows(logs: &[&Settled], config: &Config) -> Vec<Value> {
    logs.iter()
        .filter_map(|log| {
            index::row(&log.name, &log.path, log.bytes, config.records.as_deref()).ok()
        })
        .collect()
}

/// What sending one more object takes.
#[derive(Clone, Copy)]
struct Sending<'a> {
    drive: &'a Drive<'a>,
    agent: &'a ureq::Agent,
    retries: &'a [Duration],
    recipients: &'a Recipients,
    scratch: &'a Path,
}

/// Send `rows`, if there are any, sealed as `name`, then keep them in the
/// index and the object in the ledger. Drive busy through every retry
/// leaves the rows for the next run, saying so in `busy`; a copy that
/// landed even so is one more object holding the same rows, which mining
/// reads as one.
fn send_rows(
    rows: &[Value],
    name: &str,
    sending: &Sending<'_>,
    login: &mut Login,
    ledger: &mut Ledger,
    index: &mut Index,
    done: &mut Done,
) -> Result<(), String> {
    if rows.is_empty() {
        return Ok(());
    }
    let plain = sending.scratch.with_extension("rows");
    let text: String = rows.iter().fold(String::new(), |mut text, row| {
        text.push_str(&row.to_string());
        text.push('\n');
        text
    });
    fs::write(&plain, text).map_err(|why| format!("can't write {}: {why}", plain.display()))?;
    let sealed = seal::seal(&plain, sending.scratch, sending.recipients);
    let _ = fs::remove_file(&plain);
    let bytes = sealed?;
    let at = seconds(SystemTime::now());
    let sent = patiently(sending.retries, || {
        again(login, sending.agent, |token| {
            sending.drive.create(token, name, sending.scratch, bytes)
        })
    });
    // The sealed copy is scratch whether it went or not.
    let _ = fs::remove_file(sending.scratch);
    match sent {
        Err(Fault::Busy(why)) => {
            done.busy = Some(why);
            Ok(())
        }
        Err(fault) => Err(fault.to_string()),
        Ok(id) => {
            ledger.record(name, &id, bytes, at)?;
            index.keep(rows)?;
            done.indexed = rows.len();
            Ok(())
        }
    }
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
