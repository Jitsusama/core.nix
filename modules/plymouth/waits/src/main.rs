//! plymouth-waits: tells Plymouth what the boot or shutdown is waiting on.
//!
//! systemd tells Plymouth only that a unit has finished starting, so a boot
//! or shutdown that stalled showed its words and nothing else. This follows
//! systemd's jobs over its private socket, which is there from the first
//! moment to the last, and once a job has run a few seconds (jobs.rs) sends
//! Plymouth `waiting:` and the unit's description, the words systemd's own
//! console uses for it; `waiting:` alone once nothing is slow. The theme
//! says it in its own words.
//!
//! It ends when Plymouth quits after a boot, and with everything else at a
//! shutdown. Started while the system is up with no splash to tell, as
//! switching to a new configuration starts what the boot wants, it ends at
//! once. Nothing here is asked of systemd but at a job's few moments:
//! it sleeps on the socket until a signal comes, or a deadline it armed on
//! one.

mod jobs;

use std::error::Error;
use std::io::{self, Write};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::{SocketAddr, UnixStream};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use dbus::arg::{PropMap, Variant, prop_cast};
use dbus::channel::Channel;
use dbus::message::MessageType;
use dbus::{Message, Path};

use jobs::{Jobs, Next};

const OWN: &str = "plymouth-waits.service";
const SYSTEMD: &str = "org.freedesktop.systemd1";
const ROOT: &str = "/org/freedesktop/systemd1";
const MANAGER: &str = "org.freedesktop.systemd1.Manager";
const JOB: &str = "org.freedesktop.systemd1.Job";
const UNIT: &str = "org.freedesktop.systemd1.Unit";
const PROPERTIES: &str = "org.freedesktop.DBus.Properties";
const LOCAL: &str = "org.freedesktop.DBus.Local";
/// Plymouth quitting after a boot, which ends the watch.
const QUITTING: &str = "plymouth-quit.service";
/// How long systemd has to answer a question, which it does at once
/// unless it's stuck itself.
const ANSWER: Duration = Duration::from_secs(5);
const NO_REPLY: &str = "org.freedesktop.DBus.Error.NoReply";

/// One of systemd's jobs as `ListJobs` gives it: its id, unit, kind, step,
/// and the job's and unit's paths.
type Listed = (u32, String, String, String, Path<'static>, Path<'static>);

fn main() -> ExitCode {
    match watch() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("<3>{error}");
            ExitCode::FAILURE
        }
    }
}

fn watch() -> Result<(), Box<dyn Error>> {
    let bus = Channel::open_private("unix:path=/run/systemd/private")?;
    let mut jobs = Jobs::default();

    // systemd sends every job's signals to a client on its own socket,
    // asked or not, so a job that starts while this lists them is heard of
    // as well.
    let now = Instant::now();
    let found = listed(&bus)?;
    // Asked only once systemd has answered, since its first question can go
    // unread (listed).
    let state: Option<String> = property(&bus, ROOT, MANAGER, "SystemState");
    if state.as_deref().is_some_and(up) && !listening() {
        return Ok(());
    }
    for (id, unit, _, step, _, _) in found {
        jobs.added(id, &unit, now, OWN);
        jobs.stepped(id, step == "running", now);
    }

    loop {
        while let Some(message) = bus.pop_message() {
            let now = Instant::now();
            match signal(&message) {
                Some(Signal::New(id, unit)) => jobs.added(id, &unit, now, OWN),
                Some(Signal::Stepped(id, running)) => jobs.stepped(id, running, now),
                Some(Signal::Removed(id, unit)) => {
                    jobs.removed(id);
                    if unit == QUITTING {
                        return Ok(());
                    }
                }
                Some(Signal::Gone) => return Ok(()),
                None => {}
            }
        }

        // Asking systemd holds back whatever it sends meanwhile, so having
        // asked, the queue is read again before sleeping.
        let mut asked = false;
        loop {
            let now = Instant::now();
            match jobs.next(now) {
                Next::Look(id) => {
                    asked = true;
                    match running(&bus, id) {
                        Some(running) => jobs.stepped(id, running, now),
                        None => jobs.removed(id),
                    }
                }
                Next::Name(unit) => {
                    asked = true;
                    tell(&format!("waiting:{}", described(&bus, &unit)));
                }
                Next::Clear => tell("waiting:"),
                Next::Nothing => break,
            }
        }
        if asked {
            continue;
        }

        let wait = jobs
            .due()
            .map(|due| due.saturating_duration_since(Instant::now()));
        if bus.read_write(wait).is_err() {
            return Ok(());
        }
    }
}

enum Signal {
    New(u32, String),
    Stepped(u32, bool),
    Removed(u32, String),
    /// systemd has gone, as it does at the very end of a shutdown.
    Gone,
}

/// What a message from systemd says of its jobs, if anything.
fn signal(message: &Message) -> Option<Signal> {
    if message.msg_type() != MessageType::Signal {
        return None;
    }
    let interface = message.interface()?;
    let member = message.member()?;
    match (&*interface, &*member) {
        (MANAGER, "JobNew") => {
            let (id, _, unit): (u32, Path<'_>, String) = message.read3().ok()?;
            Some(Signal::New(id, unit))
        }
        (MANAGER, "JobRemoved") => {
            let (id, _, unit): (u32, Path<'_>, String) = message.read3().ok()?;
            Some(Signal::Removed(id, unit))
        }
        (PROPERTIES, "PropertiesChanged") => {
            let id = job_id(&message.path()?)?;
            let (changed, values): (String, PropMap) = message.read2().ok()?;
            if changed != JOB {
                return None;
            }
            let step: &String = prop_cast(&values, "State")?;
            Some(Signal::Stepped(id, step == "running"))
        }
        (LOCAL, "Disconnected") => Some(Signal::Gone),
        _ => None,
    }
}

/// The jobs systemd has now.
///
/// systemd can leave a client's first question on its socket unread: it
/// reads ahead while the client signs in, keeps what follows the sign-in,
/// and then waits on the socket, which has nothing more to give. Asking
/// again gives it something, so the list is asked for until it comes, each
/// wait twice the last; the answer to an earlier ask, if it comes too, is
/// passed over with everything else that isn't a signal.
fn listed(bus: &Channel) -> Result<Vec<Listed>, Box<dyn Error>> {
    let mut wait = Duration::from_millis(100);
    loop {
        let call = Message::new_method_call(SYSTEMD, ROOT, MANAGER, "ListJobs")?;
        match bus.send_with_reply_and_block(call, wait) {
            Ok(reply) => return Ok(reply.read1()?),
            Err(error) if error.name() == Some(NO_REPLY) && wait < ANSWER => wait *= 2,
            Err(error) => return Err(error.into()),
        }
    }
}

/// Whether systemd's `SystemState` says the system is up, neither booting
/// nor shutting down.
fn up(state: &str) -> bool {
    matches!(state, "running" | "degraded")
}

/// A job's id from its path, /org/freedesktop/systemd1/job/<id>.
fn job_id(path: &str) -> Option<u32> {
    path.strip_prefix("/org/freedesktop/systemd1/job/")?
        .parse()
        .ok()
}

/// Whether a job is running, or None once it has ended.
fn running(bus: &Channel, id: u32) -> Option<bool> {
    let path = format!("/org/freedesktop/systemd1/job/{id}");
    let step: String = property(bus, &path, JOB, "State")?;
    Some(step == "running")
}

/// A unit's description, or its name where it has none.
fn described(bus: &Channel, unit: &str) -> String {
    ask(bus, ROOT, MANAGER, "GetUnit", |call| call.append1(unit))
        .ok()
        .and_then(|reply| reply.read1::<Path<'_>>().ok().map(Path::into_static))
        .and_then(|path| property::<String>(bus, &path, UNIT, "Description"))
        .filter(|description| !description.is_empty())
        .unwrap_or_else(|| unit.to_owned())
}

fn property<T>(bus: &Channel, path: &str, interface: &str, name: &str) -> Option<T>
where
    T: for<'a> dbus::arg::Get<'a> + dbus::arg::Arg,
{
    let reply = ask(bus, path, PROPERTIES, "Get", |call| {
        call.append2(interface, name)
    })
    .ok()?;
    reply.read1::<Variant<T>>().ok().map(|Variant(value)| value)
}

/// A method of systemd's, asked over its own socket.
fn ask(
    bus: &Channel,
    path: &str,
    interface: &str,
    method: &str,
    arguments: impl FnOnce(Message) -> Message,
) -> Result<Message, dbus::Error> {
    let call = Message::new_method_call(SYSTEMD, path, interface, method)
        .map_err(|error| dbus::Error::new_failed(&error))?;
    bus.send_with_reply_and_block(arguments(call), ANSWER)
}

/// Sends Plymouth a status, as systemd sends it the units it starts. A
/// Plymouth that has gone, or isn't there yet, misses it.
fn tell(status: &str) {
    if let Err(error) = told(status) {
        eprintln!("<7>Plymouth didn't hear {status:?}: {error}");
    }
}

fn told(status: &str) -> io::Result<()> {
    plymouth()?.write_all(&update(status))
}

fn plymouth() -> io::Result<UnixStream> {
    let address = SocketAddr::from_abstract_name(b"/org/freedesktop/plymouthd")?;
    UnixStream::connect_addr(&address)
}

/// Whether Plymouth is there to be told anything.
fn listening() -> bool {
    plymouth().is_ok()
}

/// Plymouth's update request: U, a byte saying an argument follows, the
/// argument's length with its closing zero, then the argument, cut to the
/// most the length can say without splitting a character.
fn update(status: &str) -> Vec<u8> {
    let mut end = status.len().min(254);
    while !status.is_char_boundary(end) {
        end -= 1;
    }
    let status = &status.as_bytes()[..end];
    let mut message = vec![b'U', 2, u8::try_from(status.len() + 1).unwrap_or(u8::MAX)];
    message.extend_from_slice(status);
    message.push(0);
    message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_update_is_framed_as_plymouth_reads_it() {
        assert_eq!(
            update("waiting:Docker"),
            b"U\x02\x0fwaiting:Docker\0".to_vec()
        );
    }

    #[test]
    fn a_long_update_is_cut_whole_characters_short() {
        let status = format!("waiting:{}", "é".repeat(200));
        let message = update(&status);
        assert_eq!(message[2] as usize, message.len() - 3);
        assert!(message.len() - 3 <= 255);
        assert!(std::str::from_utf8(&message[3..message.len() - 1]).is_ok());
    }

    #[test]
    fn only_a_boot_or_a_shutdown_is_told_of() {
        assert!(up("running"));
        assert!(up("degraded"));
        for state in ["initializing", "starting", "stopping", "maintenance"] {
            assert!(!up(state), "{state}");
        }
    }

    #[test]
    fn a_job_path_gives_its_id() {
        assert_eq!(job_id("/org/freedesktop/systemd1/job/123"), Some(123));
        assert_eq!(job_id("/org/freedesktop/systemd1/unit/a"), None);
    }

    /// systemd signs what it sends on its own socket with its well-known
    /// name, where a bus would put a unique one; a job's signals read all
    /// the same.
    #[test]
    fn a_signal_signed_by_name_reads() {
        let mut new = Message::new_signal(ROOT, MANAGER, "JobNew")
            .unwrap()
            .append3(
                7_u32,
                Path::new("/org/freedesktop/systemd1/job/7").unwrap(),
                "a.service",
            );
        new.set_sender(Some(SYSTEMD.into()));
        assert!(matches!(signal(&new), Some(Signal::New(7, unit)) if unit == "a.service"));

        let removed = Message::new_signal(ROOT, MANAGER, "JobRemoved")
            .unwrap()
            .append3(
                7_u32,
                Path::new("/org/freedesktop/systemd1/job/7").unwrap(),
                QUITTING,
            )
            .append1("done");
        assert!(matches!(signal(&removed), Some(Signal::Removed(7, unit)) if unit == QUITTING));
    }

    #[test]
    fn a_job_stepping_reads_from_its_properties() {
        let mut values = PropMap::new();
        values.insert(
            "State".into(),
            Variant(Box::new("running".to_owned()) as Box<dyn dbus::arg::RefArg>),
        );
        let changed = Message::new_signal(
            "/org/freedesktop/systemd1/job/9",
            PROPERTIES,
            "PropertiesChanged",
        )
        .unwrap()
        .append3(JOB, values, Vec::<String>::new());
        assert!(matches!(signal(&changed), Some(Signal::Stepped(9, true))));
    }
}
