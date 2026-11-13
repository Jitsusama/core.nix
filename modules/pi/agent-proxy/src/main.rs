//! The SSH agent a fenced pi command reaches, signing only logins to the
//! hosts it's told of.
//!
//! The agent harness runs an agent's shell commands in a fence that
//! keeps out the person's SSH agents, since a command holding one could
//! sign anything: log in anywhere as them, or sign a commit. Yet its
//! checks reach the person's other machines over SSH (check-there,
//! host-test-there). This stands between: the fence has its socket, and
//! it answers the agents behind it (`--upstream`, the TPM's and the
//! agents' own) only with their keys and with signatures for logins to
//! the hosts in `--hosts`, each proved by the session's own binding
//! (src/bind.rs). Anything else a client asks, adding a key, removing
//! one, locking, or a signature for anything else, is refused. Each
//! login signed and each refusal is said on stderr in one line of a
//! fixed shape (src/said.rs), so the journal holds the host and why, as
//! words a reader can keep without the line's free text.
//!
//! systemd hands it its listening socket (`LISTEN_FDS`); `--socket`
//! binds one itself, as the tests do.

mod bind;
mod said;
mod wire;

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::os::fd::FromRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use bind::{Bindings, Hosts};
use said::{Refusal, Why, signed_line};
use wire::{Fields, put_string, read_message, write_message};

/// What an agent answers when it won't, or can't, do what it was asked.
const FAILURE: u8 = 5;
/// What it answers when it did something that has no other answer.
const SUCCESS: u8 = 6;
/// A client asking which keys the agent holds, and its answer.
const REQUEST_IDENTITIES: u8 = 11;
const IDENTITIES_ANSWER: u8 = 12;
/// A client asking for a signature.
const SIGN_REQUEST: u8 = 13;
/// An agent's answer with the signature it was asked for.
const SIGN_RESPONSE: u8 = 14;
/// A client asking for something the protocol leaves to extensions.
const EXTENSION: u8 = 27;
/// The extension that binds a client's connection to a session.
const SESSION_BIND: &[u8] = b"session-bind@openssh.com";
/// The first socket systemd hands a service it starts.
const FIRST_PASSED_FD: i32 = 3;
/// The most clients answered at once. Each has a thread of its own, so
/// without a ceiling a fenced command opening connections without end
/// would run the proxy out of threads, and every fenced ssh with it.
const MOST_CLIENTS: usize = 64;

/// What every client is answered by: the hosts and the agents behind,
/// and how many clients are being answered now.
struct Shared {
    hosts: Hosts,
    upstreams: Vec<PathBuf>,
    clients: AtomicUsize,
}

/// A client's place among the [`MOST_CLIENTS`], given back as it goes.
struct Place(Arc<Shared>);

impl Place {
    /// A place for one more client, or none when every one is taken.
    fn take(shared: &Arc<Shared>) -> Option<Self> {
        if shared.clients.fetch_add(1, Ordering::SeqCst) >= MOST_CLIENTS {
            shared.clients.fetch_sub(1, Ordering::SeqCst);
            return None;
        }
        Some(Place(Arc::clone(shared)))
    }
}

impl Drop for Place {
    fn drop(&mut self) {
        self.0.clients.fetch_sub(1, Ordering::SeqCst);
    }
}

/// What the command line says.
struct Config {
    shared: Shared,
    socket: Option<PathBuf>,
}

fn main() {
    let config = match config(env::args().skip(1)) {
        Ok(config) => config,
        Err(why) => {
            eprintln!("pi-agent-proxy: {why}");
            eprintln!(
                "usage: pi-agent-proxy --hosts FILE --upstream SOCKET [--upstream SOCKET]... [--socket PATH]"
            );
            process::exit(2);
        }
    };
    let listener = match listener(config.socket.as_deref()) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("pi-agent-proxy: nothing to listen on: {error}");
            process::exit(1);
        }
    };
    let shared = Arc::new(config.shared);
    for client in listener.incoming() {
        match client {
            Ok(stream) => {
                // Dropped at once, which closes it, when there's no place.
                let Some(place) = Place::take(&shared) else {
                    let busy = format!("a client, since {MOST_CLIENTS} are answered already");
                    eprintln!("{}", Refusal::new(Why::Busy, busy).line());
                    continue;
                };
                thread::spawn(move || serve(&place.0, stream));
            }
            Err(error) => eprintln!("pi-agent-proxy: a client couldn't be taken: {error}"),
        }
    }
}

/// The configuration `args` give.
fn config(mut args: impl Iterator<Item = String>) -> Result<Config, String> {
    let mut hosts = None;
    let mut upstreams = Vec::new();
    let mut socket = None;
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{arg} takes a value"));
        match arg.as_str() {
            "--hosts" => {
                let file = value()?;
                let text = fs::read_to_string(&file)
                    .map_err(|error| format!("{file} can't be read: {error}"))?;
                hosts = Some(Hosts::read(&text).map_err(|why| format!("{file}: {why}"))?);
            }
            "--upstream" => upstreams.push(PathBuf::from(value()?)),
            "--socket" => socket = Some(PathBuf::from(value()?)),
            _ => return Err(format!("{arg} isn't an option it takes")),
        }
    }
    let hosts = hosts.ok_or("--hosts is needed")?;
    if upstreams.is_empty() {
        return Err("at least one --upstream is needed".to_owned());
    }
    Ok(Config {
        shared: Shared {
            hosts,
            upstreams,
            clients: AtomicUsize::new(0),
        },
        socket,
    })
}

/// The socket at `socket`, bound afresh, or else the one systemd passed.
fn listener(socket: Option<&Path>) -> io::Result<UnixListener> {
    if let Some(path) = socket {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        return UnixListener::bind(path);
    }
    let ours = env::var("LISTEN_PID").ok().and_then(|pid| pid.parse().ok()) == Some(process::id());
    if !ours || env::var("LISTEN_FDS").as_deref() != Ok("1") {
        return Err(io::Error::other(
            "systemd passed no socket, and no --socket was given",
        ));
    }
    // SAFETY: systemd passed this process one listening socket, at the
    // first descriptor it hands on, and nothing else here owns it.
    Ok(unsafe { UnixListener::from_raw_fd(FIRST_PASSED_FD) })
}

/// Answer one client until it goes.
fn serve(shared: &Shared, mut client: UnixStream) {
    let mut session = Session::new(shared);
    loop {
        let request = match read_message(&mut client) {
            Ok(Some(request)) => request,
            Ok(None) => return,
            Err(error) => {
                eprintln!("pi-agent-proxy: a client said something unreadable: {error}");
                return;
            }
        };
        let answer = session.answer(&request);
        if let Err(error) = write_message(&mut client, &answer) {
            eprintln!("pi-agent-proxy: a client couldn't be answered: {error}");
            return;
        }
    }
}

/// One client's connection: what it bound, and the agents it reached.
struct Session<'a> {
    shared: &'a Shared,
    /// A connection to each agent behind, made once it's first needed.
    upstreams: Vec<Option<UnixStream>>,
    /// Which agent behind holds each key it listed.
    holders: HashMap<Vec<u8>, usize>,
    bindings: Bindings,
}

impl<'a> Session<'a> {
    fn new(shared: &'a Shared) -> Self {
        Session {
            shared,
            upstreams: shared.upstreams.iter().map(|_| None).collect(),
            holders: HashMap::new(),
            bindings: Bindings::default(),
        }
    }

    /// What to answer `request`.
    fn answer(&mut self, request: &[u8]) -> Vec<u8> {
        let mut fields = Fields::new(request);
        match fields.byte() {
            Ok(REQUEST_IDENTITIES) => self.identities(),
            Ok(SIGN_REQUEST) => self.sign(request, &mut fields),
            Ok(EXTENSION) => self.extension(&mut fields),
            Ok(kind) => refused(&Refusal::new(
                Why::Request,
                format!("a request of kind {kind}, since it only lists keys and signs logins"),
            )),
            Err(why) => refused(&Refusal::from(why)),
        }
    }

    /// Every key the agents behind hold, noting which holds each.
    fn identities(&mut self) -> Vec<u8> {
        let mut listed = Vec::new();
        for at in 0..self.upstreams.len() {
            match self.ask(at, &[REQUEST_IDENTITIES]) {
                Ok(answer) => match keys_in(&answer) {
                    Ok(keys) => {
                        for (key, comment) in keys {
                            self.holders.insert(key.clone(), at);
                            listed.push((key, comment));
                        }
                    }
                    Err(why) => eprintln!(
                        "pi-agent-proxy: {} listed its keys unreadably: {why}",
                        self.shared.upstreams[at].display()
                    ),
                },
                Err(error) => eprintln!(
                    "pi-agent-proxy: {} couldn't be asked for its keys: {error}",
                    self.shared.upstreams[at].display()
                ),
            }
        }
        let mut answer = vec![IDENTITIES_ANSWER];
        answer.extend_from_slice(
            &u32::try_from(listed.len())
                .unwrap_or(u32::MAX)
                .to_be_bytes(),
        );
        for (key, comment) in listed {
            put_string(&mut answer, &key);
            put_string(&mut answer, &comment);
        }
        answer
    }

    /// The signature `request` asks for, from the agent that holds its
    /// key, when it's a login this agent may sign.
    fn sign(&mut self, request: &[u8], fields: &mut Fields) -> Vec<u8> {
        let asked = (|| {
            let key = fields.string()?;
            let data = fields.string()?;
            let _flags = fields.number()?;
            fields.done()?;
            let host = self.bindings.permits(key, data)?;
            Ok::<_, Refusal>((key.to_vec(), host.clone()))
        })();
        let (key, host) = match asked {
            Ok(asked) => asked,
            Err(refusal) => return refused(&refusal),
        };
        if !self.holders.contains_key(&key) {
            self.identities();
        }
        let Some(&at) = self.holders.get(&key) else {
            return refused(
                &Refusal::new(
                    Why::NoHolder,
                    "a signature with a key no agent behind holds",
                )
                .at(&host),
            );
        };
        let shared = self.shared;
        let behind = shared.upstreams[at].display();
        match self.ask(at, request) {
            Ok(answer) if answer.first() == Some(&SIGN_RESPONSE) => {
                eprintln!("{}", signed_line(&host));
                answer
            }
            Ok(answer) => {
                let declined = format!("a signature, since {behind} declined it");
                eprintln!("{}", Refusal::new(Why::Upstream, declined).at(&host).line());
                answer
            }
            Err(error) => refused(
                &Refusal::new(
                    Why::Upstream,
                    format!("a signature, since {behind} couldn't be asked: {error}"),
                )
                .at(&host),
            ),
        }
    }

    /// Take a session binding; refuse every other extension.
    fn extension(&mut self, fields: &mut Fields) -> Vec<u8> {
        match fields.string() {
            Ok(SESSION_BIND) => match self.bindings.bind(&self.shared.hosts, fields) {
                Ok(()) => vec![SUCCESS],
                Err(refusal) => refused(&refusal),
            },
            Ok(name) => refused(&Refusal::new(
                Why::Extension,
                format!(
                    "the extension {}, which it doesn't take",
                    String::from_utf8_lossy(name)
                ),
            )),
            Err(why) => refused(&Refusal::from(why)),
        }
    }

    /// The agent behind at `at`'s answer to `request`. A connection that
    /// fails is let go, so the next ask makes another.
    fn ask(&mut self, at: usize, request: &[u8]) -> io::Result<Vec<u8>> {
        let answer = (|| {
            let stream = match &mut self.upstreams[at] {
                Some(stream) => stream,
                slot => slot.insert(UnixStream::connect(&self.shared.upstreams[at])?),
            };
            write_message(stream, request)?;
            read_message(stream)?.ok_or_else(|| io::Error::from(io::ErrorKind::UnexpectedEof))
        })();
        if answer.is_err() {
            self.upstreams[at] = None;
        }
        answer
    }
}

/// Say on stderr what was refused, and answer that it failed.
fn refused(refusal: &Refusal) -> Vec<u8> {
    eprintln!("{}", refusal.line());
    vec![FAILURE]
}

/// A key an agent holds, as its wire blob, and the comment it gives it.
type Listed = (Vec<u8>, Vec<u8>);

/// Each key, with its comment, in an identities answer.
fn keys_in(answer: &[u8]) -> Result<Vec<Listed>, String> {
    let mut fields = Fields::new(answer);
    if fields.byte()? != IDENTITIES_ANSWER {
        return Err("it didn't answer with its keys".to_owned());
    }
    let count = fields.number()?;
    let mut keys = Vec::new();
    for _ in 0..count {
        let key = fields.string()?.to_vec();
        let comment = fields.string()?.to_vec();
        keys.push((key, comment));
    }
    fields.done()?;
    Ok(keys)
}
