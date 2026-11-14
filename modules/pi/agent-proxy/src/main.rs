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
//! (src/bind.rs). It also holds the builders' key nix-offload reaches
//! lent builders with (`--key`), hidden from every fenced command, and
//! signs with it only a login as `nix-offload` to a builder lent now
//! (`--lent`, src/lent.rs); an agent's key behind never signs for a lent
//! builder, nor the builders' key for any other host. Anything else a
//! client asks, adding a key, removing one, locking, or a signature for
//! anything else, is refused. Each login signed and each refusal is said
//! on stderr in one line of a fixed shape (src/said.rs), so the journal
//! holds the host and why, as words a reader can keep without the
//! line's free text.
//!
//! systemd hands it its listening socket (`LISTEN_FDS`); `--socket`
//! binds one itself, as the tests do.

mod bind;
mod lent;
mod peer;
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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bind::{Bindings, Hosts};
use lent::{ACCOUNT, Key, Lent};
use said::{Refusal, Signer, Why, many_signs_line, signed_line};
use signature::Signer as _;
use ssh_key::Signature;
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
/// How long an agent behind has to answer before the proxy gives up on
/// it, so one waiting on its person, as on 8 November, never stalls a
/// login it isn't needed for.
const UPSTREAM_PATIENCE: Duration = Duration::from_secs(5);
/// The logins signed for one claimed session in an hour past which the
/// proxy warns, well above the busiest hour seen: the whole machine's
/// builder runs peaked at 1,014 in an hour on 14 November, each one to
/// three logins.
const MANY_SIGNS: u32 = 6_000;
/// The claimed sessions counted before last hour's are let go.
const MOST_COUNTED: usize = 1_000;
/// The comment the builders' key is listed with.
const BUILDERS_COMMENT: &[u8] = b"nix-offload builders";

/// What every client is answered by: the hosts and the agents behind,
/// the builders' key and the builders lent, how many clients are being
/// answered now, and the logins signed this hour for each claimed
/// session.
struct Shared {
    hosts: Hosts,
    upstreams: Vec<PathBuf>,
    key: Key,
    lent: Lent,
    clients: AtomicUsize,
    signs: Mutex<HashMap<String, (u64, u32)>>,
}

impl Shared {
    /// Count a login signed for `claimed`, warning once an hour when it
    /// passes [`MANY_SIGNS`]. Nothing is refused on it: the session is
    /// only a client's claim (src/peer.rs).
    fn count(&self, claimed: &str) {
        let hour = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs() / 3600);
        let Ok(mut signs) = self.signs.lock() else {
            return;
        };
        if signs.len() > MOST_COUNTED {
            signs.retain(|_, (at, _)| *at == hour);
        }
        let entry = signs.entry(claimed.to_owned()).or_insert((hour, 0));
        if entry.0 != hour {
            *entry = (hour, 0);
        }
        entry.1 += 1;
        if entry.1 == MANY_SIGNS {
            eprintln!("{}", many_signs_line(claimed, entry.1));
        }
    }
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
                "usage: pi-agent-proxy --hosts FILE [--upstream SOCKET]... [--key FILE --lent FILE...] [--socket PATH]"
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
                    let claimed = peer::claimed_session(&stream);
                    eprintln!("{}", Refusal::new(Why::Busy, busy).line(&claimed));
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
    let mut key = None;
    let mut lent = Vec::new();
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
            "--key" => key = Some(PathBuf::from(value()?)),
            "--lent" => lent.push(PathBuf::from(value()?)),
            "--socket" => socket = Some(PathBuf::from(value()?)),
            _ => return Err(format!("{arg} isn't an option it takes")),
        }
    }
    let hosts = hosts.ok_or("--hosts is needed")?;
    if upstreams.is_empty() && key.is_none() {
        return Err("at least one --upstream, or a --key, is needed".to_owned());
    }
    if key.is_some() && lent.is_empty() {
        return Err("--key needs at least one --lent, the builders it signs for".to_owned());
    }
    Ok(Config {
        shared: Shared {
            hosts,
            upstreams,
            key: Key::new(key),
            lent: Lent::new(lent),
            clients: AtomicUsize::new(0),
            signs: Mutex::new(HashMap::new()),
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
    let mut session = Session::new(shared, peer::claimed_session(&client));
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
    /// The pi session the client claims, said in each line it's given.
    claimed: String,
}

impl<'a> Session<'a> {
    fn new(shared: &'a Shared, claimed: String) -> Self {
        Session {
            shared,
            upstreams: shared.upstreams.iter().map(|_| None).collect(),
            holders: HashMap::new(),
            bindings: Bindings::default(),
            claimed,
        }
    }

    /// Say on stderr what was refused, and answer that it failed.
    fn refused(&self, refusal: &Refusal) -> Vec<u8> {
        eprintln!("{}", refusal.line(&self.claimed));
        vec![FAILURE]
    }

    /// What to answer `request`.
    fn answer(&mut self, request: &[u8]) -> Vec<u8> {
        let mut fields = Fields::new(request);
        match fields.byte() {
            Ok(REQUEST_IDENTITIES) => self.identities(),
            Ok(SIGN_REQUEST) => self.sign(request, &mut fields),
            Ok(EXTENSION) => self.extension(&mut fields),
            Ok(kind) => self.refused(&Refusal::new(
                Why::Request,
                format!("a request of kind {kind}, since it only lists keys and signs logins"),
            )),
            Err(why) => self.refused(&Refusal::from(why)),
        }
    }

    /// Every key the agents behind hold, noting which holds each.
    fn identities(&mut self) -> Vec<u8> {
        let mut listed = Vec::new();
        if let Some(blob) = self.builders_blob() {
            listed.push((blob, BUILDERS_COMMENT.to_vec()));
        }
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
            let permit = self.bindings.permits(key, data)?;
            Ok::<_, Refusal>((
                key.to_vec(),
                data.to_vec(),
                permit.host.clone(),
                permit.lent,
                permit.user,
            ))
        })();
        let (key, data, host, lent, user) = match asked {
            Ok(asked) => asked,
            Err(refusal) => return self.refused(&refusal),
        };
        if self.builders_blob().as_ref() == Some(&key) {
            return self.sign_for_builder(&data, &host, lent, &user);
        }
        if lent {
            return self.refused(
                &Refusal::new(
                    Why::LentHost,
                    "a login to a lent builder with a key other than the builders'",
                )
                .at(&host),
            );
        }
        if !self.holders.contains_key(&key) {
            self.identities();
        }
        let Some(&at) = self.holders.get(&key) else {
            return self.refused(
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
                eprintln!(
                    "{}",
                    signed_line(&host, Signer::Agent, &user, &self.claimed)
                );
                shared.count(&self.claimed);
                answer
            }
            Ok(answer) => {
                let declined = format!("a signature, since {behind} declined it");
                let line = Refusal::new(Why::Upstream, declined)
                    .at(&host)
                    .line(&self.claimed);
                eprintln!("{line}");
                answer
            }
            Err(error) => self.refused(
                &Refusal::new(
                    Why::Upstream,
                    format!("a signature, since {behind} couldn't be asked: {error}"),
                )
                .at(&host),
            ),
        }
    }

    /// The builders' key's wire blob, when the proxy holds one.
    fn builders_blob(&self) -> Option<Vec<u8>> {
        self.shared.key.read()?.public_key().to_bytes().ok()
    }

    /// The builders' key's signature over `data`, a login as `user` to a
    /// session with `host`, when `host` is a lent builder and `user` the
    /// account lent builders take that key for.
    fn sign_for_builder(
        &self,
        data: &[u8],
        host: &ssh_key::public::KeyData,
        lent: bool,
        user: &[u8],
    ) -> Vec<u8> {
        if !lent {
            return self.refused(
                &Refusal::new(
                    Why::NotLent,
                    "a login with the builders' key to a host that isn't a lent builder",
                )
                .at(host),
            );
        }
        if user != ACCOUNT {
            return self.refused(
                &Refusal::new(
                    Why::Account,
                    "a login with the builders' key as another account than theirs",
                )
                .at(host),
            );
        }
        let Some(key) = self.shared.key.read() else {
            return self.refused(
                &Refusal::new(
                    Why::NoHolder,
                    "a builder's login, since the builders' key can't be read",
                )
                .at(host),
            );
        };
        let signed: Result<Signature, _> = key.try_sign(data);
        let Some(blob) = signed
            .ok()
            .and_then(|signature| Vec::try_from(signature).ok())
        else {
            return self.refused(
                &Refusal::new(
                    Why::NoHolder,
                    "a builder's login, since the builders' key couldn't sign it",
                )
                .at(host),
            );
        };
        eprintln!(
            "{}",
            signed_line(host, Signer::Builders, user, &self.claimed)
        );
        self.shared.count(&self.claimed);
        let mut answer = vec![SIGN_RESPONSE];
        put_string(&mut answer, &blob);
        answer
    }

    /// Take a session binding; refuse every other extension.
    fn extension(&mut self, fields: &mut Fields) -> Vec<u8> {
        match fields.string() {
            Ok(SESSION_BIND) => {
                match self
                    .bindings
                    .bind(&self.shared.hosts, &self.shared.lent, fields)
                {
                    Ok(()) => vec![SUCCESS],
                    Err(refusal) => self.refused(&refusal),
                }
            }
            Ok(name) => self.refused(&Refusal::new(
                Why::Extension,
                format!(
                    "the extension {}, which it doesn't take",
                    String::from_utf8_lossy(name)
                ),
            )),
            Err(why) => self.refused(&Refusal::from(why)),
        }
    }

    /// The agent behind at `at`'s answer to `request`. A connection that
    /// fails is let go, so the next ask makes another.
    fn ask(&mut self, at: usize, request: &[u8]) -> io::Result<Vec<u8>> {
        let answer = (|| {
            let stream = match &mut self.upstreams[at] {
                Some(stream) => stream,
                slot => {
                    let stream = UnixStream::connect(&self.shared.upstreams[at])?;
                    stream.set_read_timeout(Some(UPSTREAM_PATIENCE))?;
                    stream.set_write_timeout(Some(UPSTREAM_PATIENCE))?;
                    slot.insert(stream)
                }
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
