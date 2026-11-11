//! sound SOCKET [--vocabulary WORDS]
//!
//! The sound server's devices, held ready for the shell and anything else
//! that asks, over SOCKET in the shell's daemon interface (interface.rs):
//! each output and input as a `device` record, saying how loud it's set,
//! whether it's muted and whether it's the one chosen (devices.rs), and
//! as the launcher's Sound row for it, worded from WORDS, the launcher's
//! words.json (rows.rs), with its parts inside it (parts.rs).
//!
//! It hears PipeWire through its own API for as long as it runs, rather
//! than a program's output read line by line: the registry says each
//! node as it comes and goes, each device's node says its properties and
//! its Props as they change, and the default metadata says which output
//! and input are chosen. A watch is told only the records that changed,
//! and those gone, so a volume moving tells its one device. Nothing runs
//! on a timer: with PipeWire gone, it tries again every two seconds until
//! it's back.
//!
//! An act (acts.rs) is worked out where it's asked, then done on
//! PipeWire's own loop, where the proxies live, through a channel the
//! loop hears; what it moved comes back to every watch as the server
//! says it. A card is heard for the route that plays each of its devices,
//! since a card's device takes its levels on that route.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::fd::FromRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use pipewire as pw;
use pw::context::ContextRc;
use pw::device::{Device, DeviceChangeMask, DeviceInfoRef, DeviceListener};
use pw::main_loop::MainLoopRc;
use pw::metadata::{Metadata, MetadataListener};
use pw::node::{Node, NodeChangeMask, NodeInfoRef, NodeListener};
use pw::registry::{GlobalObject, RegistryRc};
use pw::spa::param::ParamType;
use pw::spa::pod::Pod;
use pw::spa::utils::dict::DictRef;
use pw::types::ObjectType;
use ready_rows::kept::Kept;
use serde_json::{Value, json};
use sound::acts::{Move, act, planned};
use sound::devices::{Change, Devices, direction};
use sound::interface::{Ask, VERSION, ask, error, kind_of};
use sound::levels::{levels, offered, profile, route};
use sound::parts::served;
use sound::rows::Words;

/// How many lines a connection may have waiting to be written before
/// it's taken as no longer reading and hung up on.
const QUEUED: usize = 256;
/// How long to wait after a connection couldn't be taken.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(100);
/// How long to wait before asking PipeWire again once it's gone.
const SERVER_AGAIN: Duration = Duration::from_secs(2);
/// How long an act waits for PipeWire's loop to take it.
const ACT_PATIENCE: Duration = Duration::from_secs(2);

/// One act's move, for PipeWire's loop to do, and where to say how it
/// went.
struct Doing {
    done: Move,
    answer: mpsc::Sender<Result<(), String>>,
}

/// One watch a client holds.
struct Watch {
    connection: u64,
    id: Value,
    kinds: Vec<String>,
    keys: Option<Vec<String>>,
    told: SyncSender<String>,
    stream: Arc<UnixStream>,
}

impl Watch {
    fn wants(&self, key: &str) -> bool {
        match &self.keys {
            Some(keys) => keys.iter().any(|k| k == key),
            None => kind_of(key).is_some_and(|kind| self.kinds.iter().any(|k| k == kind)),
        }
    }
}

/// Everything the threads share.
struct Daemon {
    held: BTreeMap<String, Value>,
    watches: Vec<Watch>,
    devices: Devices,
    /// The launcher's words its rows are worded in, and the rows as last
    /// told, each keeping its place in their order.
    words: Words,
    served: Kept,
    /// How PipeWire stands: "starting" until it first answers,
    /// "answering", or "not answering" while it's gone.
    server: &'static str,
    changes: u64,
    since: u64,
    /// Where an act goes to be done, while PipeWire answers.
    acting: Option<pw::channel::Sender<Doing>>,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(socket), vocabulary) = (args.next(), args.next()) else {
        eprintln!("usage: sound SOCKET [--vocabulary WORDS]");
        std::process::exit(64);
    };
    let socket = PathBuf::from(socket);
    // Without its words, a row says the vocabulary's ids for them, as
    // the launcher's Sound does, rather than the daemon not starting.
    let words = match (vocabulary.as_deref(), args.next()) {
        (Some("--vocabulary"), Some(path)) => worded(Path::new(&path)),
        (None, None) => Words::default(),
        _ => {
            eprintln!("usage: sound SOCKET [--vocabulary WORDS]");
            std::process::exit(64);
        }
    };
    let listener = match listen(&socket) {
        Ok(listener) => listener,
        Err(why) => {
            eprintln!("<3>couldn't listen on {}: {why}", socket.display());
            std::process::exit(71);
        }
    };
    let shared = Arc::new(Mutex::new(Daemon {
        held: BTreeMap::new(),
        watches: Vec::new(),
        devices: Devices::default(),
        words,
        served: Kept::default(),
        server: "starting",
        changes: 0,
        since: now_ms(),
        acting: None,
    }));
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || follow(&shared));
    }
    for (connection, stream) in (1u64..).zip(listener.incoming()) {
        let Ok(stream) = stream else {
            std::thread::sleep(ACCEPT_BACKOFF);
            continue;
        };
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || serve(&shared, stream, connection));
    }
}

/// The launcher's words, read from its words.json, or none where it
/// can't be read, said once.
fn worded(path: &Path) -> Words {
    let read = std::fs::read_to_string(path).map_err(|why| why.to_string());
    match read.and_then(|text| Words::from_vocabulary(&text).map_err(|why| why.to_string())) {
        Ok(words) => words,
        Err(why) => {
            eprintln!("<4>couldn't read the words in {}: {why}", path.display());
            Words::default()
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// The socket systemd holds for it (sound.socket), or, run by hand, its
/// own, in a folder only Joel reads, replacing one left behind.
fn listen(socket: &Path) -> std::io::Result<UnixListener> {
    if let Some(listener) = handed() {
        return Ok(listener);
    }
    if let Some(folder) = socket.parent() {
        std::fs::create_dir_all(folder)?;
        std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o700))?;
    }
    let _ = std::fs::remove_file(socket);
    UnixListener::bind(socket)
}

/// The listening socket systemd hands this process (`sd_listen_fds`).
fn handed() -> Option<UnixListener> {
    let pid: u32 = std::env::var("LISTEN_PID").ok()?.parse().ok()?;
    let count: u32 = std::env::var("LISTEN_FDS").ok()?.parse().ok()?;
    if pid != std::process::id() || count < 1 {
        return None;
    }
    // SAFETY: systemd hands descriptor 3 to this process alone, named by
    // LISTEN_PID, and nothing here has opened or taken it before.
    Some(unsafe { UnixListener::from_raw_fd(3) })
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Hears PipeWire, and asks again every two seconds while it's gone, as
/// it is between one sound server and the next. What was heard of one
/// is let go of as it goes, so its devices are told gone.
fn follow(shared: &Arc<Mutex<Daemon>>) {
    pw::init();
    loop {
        if let Err(why) = hear(shared)
            && lock(shared).server != "not answering"
        {
            eprintln!("<4>PipeWire isn't answering: {why}");
        }
        let mut daemon = lock(shared);
        daemon.server = "not answering";
        daemon.acting = None;
        daemon.devices.forget();
        refresh(&mut daemon);
        drop(daemon);
        std::thread::sleep(SERVER_AGAIN);
    }
}

/// What's bound of the server, by its global's id, kept for as long as
/// the global is there, since dropping a proxy or its listener stops it
/// being heard.
enum Bound {
    Node {
        proxy: Node,
        _listener: NodeListener,
    },
    Card {
        proxy: Device,
        _listener: DeviceListener,
    },
    Defaults {
        proxy: Metadata,
        _listener: MetadataListener,
    },
}

/// Hears one connection to PipeWire until it ends.
fn hear(shared: &Arc<Mutex<Daemon>>) -> Result<(), String> {
    let main_loop = MainLoopRc::new(None).map_err(|why| format!("its loop: {why}"))?;
    let context = ContextRc::new(&main_loop, None).map_err(|why| format!("its context: {why}"))?;
    let core = context
        .connect_rc(None)
        .map_err(|why| format!("reaching it: {why}"))?;
    let registry = core
        .get_registry_rc()
        .map_err(|why| format!("its registry: {why}"))?;
    let bound: Rc<RefCell<HashMap<u32, Bound>>> = Rc::default();
    let (acting, acts) = pw::channel::channel::<Doing>();
    let _acts = acts.attach(main_loop.loop_(), {
        let bound = Rc::clone(&bound);
        move |doing: Doing| {
            let _ = doing.answer.send(done(&bound.borrow(), &doing.done));
        }
    });
    let quit = main_loop.downgrade();
    let _core_listener = core
        .add_listener_local()
        .error(move |id, _, _, message| {
            if id == pw::core::PW_ID_CORE {
                eprintln!("<4>PipeWire went away: {message}");
                if let Some(main_loop) = quit.upgrade() {
                    main_loop.quit();
                }
            }
        })
        .register();
    let _registry_listener = registry
        .add_listener_local()
        .global({
            let (shared, bound, registry) =
                (Arc::clone(shared), Rc::clone(&bound), registry.clone());
            move |global| {
                let found = match global.type_ {
                    ObjectType::Node => node(&shared, &registry, global),
                    ObjectType::Device => card(&shared, &registry, global),
                    ObjectType::Metadata => defaults(&shared, &registry, global),
                    _ => None,
                };
                if let Some(found) = found {
                    bound.borrow_mut().insert(global.id, found);
                }
            }
        })
        .global_remove({
            let shared = Arc::clone(shared);
            move |id| match bound.borrow_mut().remove(&id) {
                Some(Bound::Node { .. } | Bound::Card { .. }) => told(&shared, Change::Gone { id }),
                Some(Bound::Defaults { .. }) => told(
                    &shared,
                    Change::Chose {
                        key: None,
                        value: None,
                    },
                ),
                None => {}
            }
        })
        .register();
    {
        let mut daemon = lock(shared);
        daemon.server = "answering";
        daemon.acting = Some(acting);
    }
    main_loop.run();
    Ok(())
}

fn pod_of(bytes: &[u8]) -> Result<&Pod, String> {
    Pod::from_bytes(bytes).ok_or_else(|| "the act's pod didn't hold together".to_owned())
}

/// Does one act's move on PipeWire's loop, where its proxies are.
fn done(bound: &HashMap<u32, Bound>, doing: &Move) -> Result<(), String> {
    match doing {
        Move::Node { id, pod } => match bound.get(id) {
            Some(Bound::Node { proxy, .. }) => {
                proxy.set_param(ParamType::Props, 0, pod_of(pod)?);
                Ok(())
            }
            _ => Err(format!("node {id} is gone")),
        },
        Move::Card { id, pod } => match bound.get(id) {
            Some(Bound::Card { proxy, .. }) => {
                proxy.set_param(ParamType::Route, 0, pod_of(pod)?);
                Ok(())
            }
            _ => Err(format!("card {id} is gone")),
        },
        Move::Choose { key, value } => {
            let defaults = bound.values().find_map(|bound| match bound {
                Bound::Defaults { proxy, .. } => Some(proxy),
                _ => None,
            });
            let Some(defaults) = defaults else {
                return Err("the default metadata is gone".to_owned());
            };
            defaults.set_property(
                pw::core::PW_ID_CORE,
                key,
                Some("Spa:String:JSON"),
                Some(value),
            );
            Ok(())
        }
    }
}

/// Binds a node that's an output or an input, by the class the registry
/// says of it, hearing its properties and its Props.
fn node(
    shared: &Arc<Mutex<Daemon>>,
    registry: &RegistryRc,
    global: &GlobalObject<&DictRef>,
) -> Option<Bound> {
    direction(global.props?.get("media.class")?)?;
    let id = global.id;
    let node: Node = registry.bind(global).ok()?;
    let listener = node
        .add_listener_local()
        .info({
            let shared = Arc::clone(shared);
            move |info: &NodeInfoRef| {
                // An info says its properties only where they changed,
                // as its first does.
                if !info.change_mask().contains(NodeChangeMask::PROPS) {
                    return;
                }
                let Some(props) = info.props() else { return };
                let properties = props
                    .iter()
                    .map(|(key, value)| (key.to_owned(), value.to_owned()));
                told(
                    &shared,
                    Change::Node {
                        id,
                        properties: properties.collect(),
                    },
                );
            }
        })
        .param({
            let shared = Arc::clone(shared);
            move |_, kind, _, _, pod| {
                let (Some(pod), ParamType::Props) = (pod, kind) else {
                    return;
                };
                let (volumes, muted) = levels(pod.as_bytes());
                told(&shared, Change::Levels { id, volumes, muted });
            }
        })
        .register();
    node.subscribe_params(&[ParamType::Props]);
    Some(Bound::Node {
        proxy: node,
        _listener: listener,
    })
}

/// Binds a card, hearing what it says it is, its profile, which route
/// plays each of its profile's devices, and every port it has.
fn card(
    shared: &Arc<Mutex<Daemon>>,
    registry: &RegistryRc,
    global: &GlobalObject<&DictRef>,
) -> Option<Bound> {
    if global.props?.get("media.class")? != "Audio/Device" {
        return None;
    }
    let id = global.id;
    let card: Device = registry.bind(global).ok()?;
    let listener = card
        .add_listener_local()
        .info({
            let shared = Arc::clone(shared);
            move |info: &DeviceInfoRef| {
                if !info.change_mask().contains(DeviceChangeMask::PROPS) {
                    return;
                }
                let Some(props) = info.props() else { return };
                let properties = props
                    .iter()
                    .map(|(key, value)| (key.to_owned(), value.to_owned()));
                told(
                    &shared,
                    Change::Card {
                        id,
                        properties: properties.collect(),
                    },
                );
            }
        })
        .param({
            let shared = Arc::clone(shared);
            move |_, kind, _, _, pod| {
                let Some(pod) = pod else { return };
                let change = match kind {
                    ParamType::Route => {
                        route(pod.as_bytes()).map(|(index, device)| Change::Route {
                            card: id,
                            index,
                            device,
                        })
                    }
                    ParamType::EnumRoute => {
                        offered(pod.as_bytes()).map(|port| Change::Offer { card: id, port })
                    }
                    ParamType::Profile => {
                        profile(pod.as_bytes()).map(|index| Change::Profile { card: id, index })
                    }
                    _ => None,
                };
                if let Some(change) = change {
                    told(&shared, change);
                }
            }
        })
        .register();
    // Every route it has, said again as a jack or a display comes or
    // goes, says whether anything is plugged in where each device plays,
    // and its profile which of them its devices are offered.
    card.subscribe_params(&[ParamType::Route, ParamType::EnumRoute, ParamType::Profile]);
    Some(Bound::Card {
        proxy: card,
        _listener: listener,
    })
}

/// Binds the default metadata, which names the output and input chosen.
fn defaults(
    shared: &Arc<Mutex<Daemon>>,
    registry: &RegistryRc,
    global: &GlobalObject<&DictRef>,
) -> Option<Bound> {
    if global.props?.get("metadata.name")? != "default" {
        return None;
    }
    let metadata: Metadata = registry.bind(global).ok()?;
    let listener = metadata
        .add_listener_local()
        .property({
            let shared = Arc::clone(shared);
            move |subject, key, _, value| {
                if subject == pw::core::PW_ID_CORE {
                    told(
                        &shared,
                        Change::Chose {
                            key: key.map(str::to_owned),
                            value: value.map(str::to_owned),
                        },
                    );
                }
                0
            }
        })
        .register();
    Some(Bound::Defaults {
        proxy: metadata,
        _listener: listener,
    })
}

/// Takes in one change the server said, telling each watch what of it
/// changed a record.
fn told(shared: &Mutex<Daemon>, change: Change) {
    let mut daemon = lock(shared);
    daemon.changes += 1;
    if daemon.devices.hear(change) {
        refresh(&mut daemon);
    }
}

/// Works every record out again, keeps those that changed and lets go of
/// those gone, telling each watch what changed of what it wants.
fn refresh(daemon: &mut Daemon) {
    let mut records = daemon.devices.records();
    let ready = served(&records, &daemon.words);
    daemon.served.take(ready, Vec::new());
    records.extend(daemon.served.all().cloned());
    let said: HashSet<String> = records
        .iter()
        .filter_map(|record| record["key"].as_str().map(str::to_owned))
        .collect();
    let gone: Vec<String> = daemon
        .held
        .keys()
        .filter(|key| !said.contains(*key))
        .cloned()
        .collect();
    for key in &gone {
        daemon.held.remove(key);
    }
    let mut changed = Vec::new();
    for record in records {
        let Some(key) = record["key"].as_str().map(str::to_owned) else {
            continue;
        };
        if daemon.held.get(&key) != Some(&record) {
            daemon.held.insert(key, record.clone());
            changed.push(record);
        }
    }
    if changed.is_empty() && gone.is_empty() {
        return;
    }
    let mut stuck = HashSet::new();
    for watch in &daemon.watches {
        let records: Vec<&Value> = changed
            .iter()
            .filter(|record| record["key"].as_str().is_some_and(|key| watch.wants(key)))
            .collect();
        let went: Vec<&String> = gone.iter().filter(|key| watch.wants(key)).collect();
        if records.is_empty() && went.is_empty() {
            continue;
        }
        let mut told = json!({"id": watch.id, "changed": records});
        if !went.is_empty() {
            told["gone"] = json!(went);
        }
        if watch.told.try_send(told.to_string()).is_err() {
            stuck.insert(watch.connection);
            let _ = watch.stream.shutdown(Shutdown::Both);
        }
    }
    if !stuck.is_empty() {
        daemon
            .watches
            .retain(|watch| !stuck.contains(&watch.connection));
    }
}

fn serve(shared: &Mutex<Daemon>, stream: UnixStream, connection: u64) {
    let Ok(reading) = stream.try_clone() else {
        return;
    };
    let Ok(hangup) = stream.try_clone().map(Arc::new) else {
        return;
    };
    let (told, lines) = mpsc::sync_channel::<String>(QUEUED);
    let mut writing = stream;
    std::thread::spawn(move || {
        for line in lines {
            if writeln!(writing, "{line}").is_err() {
                break;
            }
        }
    });
    for line in BufReader::new(reading).lines() {
        let Ok(line) = line else { break };
        let Some(answer) = answer(shared, ask(&line), connection, &told, &hangup) else {
            continue;
        };
        if told.send(answer.to_string()).is_err() {
            break;
        }
    }
    lock(shared)
        .watches
        .retain(|watch| watch.connection != connection);
}

/// The records held of these kinds or keys: by key, as the server's own
/// names order them.
fn held(daemon: &Daemon, kinds: &[String], keys: Option<&[String]>) -> Vec<Value> {
    match keys {
        Some(keys) => keys
            .iter()
            .filter_map(|key| daemon.held.get(key))
            .cloned()
            .collect(),
        None => daemon
            .held
            .iter()
            .filter(|(key, _)| kind_of(key).is_some_and(|kind| kinds.iter().any(|k| k == kind)))
            .map(|(_, record)| record.clone())
            .collect(),
    }
}

/// Works out what an act asks and has PipeWire's loop do it, saying it's
/// done or why not. What it moved comes to every watch as the server says
/// it back, so a watch's records are always the server's own.
fn acted(shared: &Mutex<Daemon>, id: &Value, name: &str, said: &Value) -> Value {
    let asked = match act(name, said) {
        Ok(asked) => asked,
        Err(refused) => return error(id, refused.code, &refused.message),
    };
    let (done, acting) = {
        let daemon = lock(shared);
        let Some(device) = daemon.devices.device(asked.key()) else {
            return error(id, "no-device", &format!("no device {} here", asked.key()));
        };
        match planned(&asked, &device) {
            Ok(done) => (done, daemon.acting.clone()),
            Err(refused) => return error(id, refused.code, &refused.message),
        }
    };
    let not_answering = || error(id, "not-answering", "the sound server isn't answering");
    let Some(acting) = acting else {
        return not_answering();
    };
    let (answer, answered) = mpsc::channel();
    if acting.send(Doing { done, answer }).is_err() {
        return not_answering();
    }
    match answered.recv_timeout(ACT_PATIENCE) {
        Ok(Ok(())) => json!({"id": id, "done": true}),
        Ok(Err(why)) => error(id, "not-done", &why),
        Err(_) => not_answering(),
    }
}

/// How the daemon stands, as hello and health say it.
fn standing(daemon: &Daemon, id: &Value) -> Value {
    json!({
        "id": id, "version": VERSION, "domain": "sound", "state": "answering",
        "since": daemon.since,
        "sources": {"pipewire": {"state": daemon.server, "changes": daemon.changes}},
    })
}

/// What one ask is answered with, or none where it's been told already.
fn answer(
    shared: &Mutex<Daemon>,
    asked: Ask,
    connection: u64,
    told: &SyncSender<String>,
    hangup: &Arc<UnixStream>,
) -> Option<Value> {
    Some(match asked {
        Ask::Hello { id } => standing(&lock(shared), &id),
        Ask::Get { id, keys } => {
            json!({"id": id, "records": held(&lock(shared), &[], Some(&keys))})
        }
        Ask::List { id, kinds } => json!({"id": id, "records": held(&lock(shared), &kinds, None)}),
        Ask::Watch { id, kinds, keys } => {
            let mut daemon = lock(shared);
            // A watch asked again by its id replaces the one before.
            daemon
                .watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            daemon.watches.push(Watch {
                connection,
                id: id.clone(),
                kinds: kinds.clone(),
                keys: keys.clone(),
                told: told.clone(),
                stream: Arc::clone(hangup),
            });
            // Told under the same lock a change is told under, so no
            // change comes before the answer it changes.
            let whole = json!({"id": id, "records": held(&daemon, &kinds, keys.as_deref())});
            if told.try_send(whole.to_string()).is_err() {
                let _ = hangup.shutdown(Shutdown::Both);
            }
            return None;
        }
        Ask::Unwatch { id } => {
            let mut daemon = lock(shared);
            daemon
                .watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            json!({"id": id, "unwatched": true})
        }
        Ask::Health { id } => {
            let daemon = lock(shared);
            let mut health = standing(&daemon, &id);
            health["watches"] = json!(daemon.watches.len());
            health
        }
        Ask::Act { id, act, said } => acted(shared, &id, &act, &said),
        Ask::UnknownKind { id, kind } => {
            error(&id, "unknown-kind", &format!("no kind {kind} here"))
        }
        Ask::Unknown { id, ask } => error(&id, "unknown-ask", &format!("no ask {ask:?} here")),
    })
}
