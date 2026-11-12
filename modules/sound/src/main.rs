//! sound SOCKET [--vocabulary WORDS] [--marks MARKS]
//!
//! The sound server's devices, held ready for the shell and anything else
//! that asks, over SOCKET in the shell's daemon interface (interface.rs):
//! each output and input as a `device` record, saying how loud it's set,
//! whether it's muted and whether it's the one chosen (devices.rs), and
//! as the launcher's Sound row for it, worded from WORDS, the launcher's
//! words.json (rows.rs), with its parts inside it (parts.rs), the
//! programs playing through it among them, each wearing its mark from
//! MARKS (programs.rs), and each program's drawer beside its row, as a
//! `drawer` record (drawer.rs); each program's stream as a `stream` record
//! (streams.rs); where each stream is sent, as one `targets` record; and,
//! while a watch wants them, how each running node keeps time, as a
//! `timing` record each, from PipeWire's profiler (timing.rs).
//!
//! It hears PipeWire through its own API for as long as it runs, rather
//! than a program's output read line by line: the registry says each
//! node and link as it comes and goes, each device's and stream's node
//! says its properties, its Props and its Format as they change, and the default
//! metadata says which output and input are chosen and where each stream
//! is sent. A program is named by its desktop entry, read again as the
//! entries change. A watch is told only the records that changed,
//! and those gone, so a volume moving tells its one device. Nothing runs
//! on a timer: with PipeWire gone, it tries again every two seconds until
//! it's back.
//!
//! An act (acts.rs) is worked out where it's asked, then done on
//! PipeWire's own loop, where the proxies live, through a channel the
//! loop hears; what it moved comes back to every watch as the server
//! says it. A card is heard for the route that plays each of its devices,
//! since a card's device takes its levels on that route. Each client of
//! the socket is answered in clients.rs.

mod clients;
mod profiler;

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::Shutdown;
use std::os::fd::FromRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clients::{Watch, serve};
use pipewire as pw;
use profiler::Profiler;
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
use sound::acts::Move;
use sound::devices::{Change, Devices, direction, said_state};
use sound::levels::{channels, format, levels, offered, profile, route};
use sound::naming::{Naming, followed, read_or_none};
use sound::parts::served;
use sound::rows::Words;
use sound::streams::playing;
use sound::timing::Timings;
use sound::{desk, drawer};
use windows::entries::Entries;
use windows::marks::Marks;

/// How long to wait after a connection couldn't be taken.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(100);
/// How long to wait before asking PipeWire again once it's gone.
const SERVER_AGAIN: Duration = Duration::from_secs(2);
/// The default metadata's key the session manager sends a stream by.
const TARGET: &str = "target.object";

/// What PipeWire's loop is handed to do: one act's move, and where to
/// say how it went; or whether any watch wants how nodes keep time.
enum Doing {
    Act {
        done: Move,
        answer: mpsc::Sender<Result<(), String>>,
    },
    Timing(bool),
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
    /// What each program is called and the mark it wears.
    naming: Naming,
    /// How PipeWire stands: "starting" until it first answers,
    /// "answering", or "not answering" while it's gone.
    server: &'static str,
    changes: u64,
    since: u64,
    /// Where an act goes to be done, while PipeWire answers.
    acting: Option<pw::channel::Sender<Doing>>,
    /// Whether any watch wants how nodes keep time, and what the
    /// profiler has said of it since.
    timed: bool,
    timings: Timings,
    /// What desk-facts says of each card, by its key.
    desk: BTreeMap<String, Value>,
}

const USAGE: &str = "usage: sound SOCKET [--vocabulary WORDS] [--marks MARKS]";

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(socket) = args.next().map(PathBuf::from) else {
        eprintln!("{USAGE}");
        std::process::exit(64);
    };
    // Without its words, a row says the vocabulary's ids for them, as
    // the launcher's Sound does, and without marks a program wears the
    // launcher's, rather than the daemon not starting.
    let (mut words, mut marks) = (Words::default(), Marks::default());
    while let Some(flag) = args.next() {
        match (flag.as_str(), args.next()) {
            ("--vocabulary", Some(path)) => words = read_or_none(&path, Words::from_vocabulary),
            ("--marks", Some(path)) => marks = read_or_none(&path, Marks::read),
            _ => {
                eprintln!("{USAGE}");
                std::process::exit(64);
            }
        }
    }
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
        naming: Naming {
            entries: Entries::default(),
            marks,
        },
        server: "starting",
        changes: 0,
        since: now_ms(),
        acting: None,
        timed: false,
        timings: Timings::default(),
        desk: BTreeMap::new(),
    }));
    // A program is named anew each time the desktop entries change.
    let renaming = Arc::clone(&shared);
    lock(&shared).naming.entries = followed(move |read| {
        let mut daemon = lock(&renaming);
        daemon.naming.entries = read;
        refresh(&mut daemon);
    });
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || follow(&shared));
    }
    // What desk-facts says of each card, read again each time it writes.
    if let Some(folder) =
        std::env::var_os("XDG_RUNTIME_DIR").map(|run| PathBuf::from(run).join("desk"))
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || {
            desk::follow(&folder, |said| {
                let mut daemon = lock(&shared);
                if daemon.desk != said {
                    daemon.desk = said;
                    refresh(&mut daemon);
                }
            });
        });
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
        daemon.timings.forget();
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
    /// Held only to be let go, which unbinds it.
    Profiler { _bound: Profiler },
}

/// The loop's context, taught the profiler's protocol before anything
/// binds; one without the profiler's module still hears everything else.
fn taught_context(main_loop: &MainLoopRc) -> Result<ContextRc, String> {
    let context = ContextRc::new(main_loop, None).map_err(|why| format!("its context: {why}"))?;
    if !profiler::taught(&context) {
        eprintln!("<4>PipeWire's profiler module isn't there, so nothing is timed");
    }
    Ok(context)
}

/// What binding the profiler takes, on PipeWire's loop: the daemon, what's
/// bound, the profiler's global once the registry says it, the registry
/// it's bound from, and the loop its second's timer is on.
#[derive(Clone)]
struct Binding {
    shared: Arc<Mutex<Daemon>>,
    bound: Rc<RefCell<HashMap<u32, Bound>>>,
    profiler: Rc<Cell<Option<u32>>>,
    registry: RegistryRc,
    main_loop: MainLoopRc,
}

/// Hears one connection to PipeWire until it ends.
fn hear(shared: &Arc<Mutex<Daemon>>) -> Result<(), String> {
    let main_loop = MainLoopRc::new(None).map_err(|why| format!("its loop: {why}"))?;
    let context = taught_context(&main_loop)?;
    let core = context
        .connect_rc(None)
        .map_err(|why| format!("reaching it: {why}"))?;
    let registry = core
        .get_registry_rc()
        .map_err(|why| format!("its registry: {why}"))?;
    let binding = Binding {
        shared: Arc::clone(shared),
        bound: Rc::default(),
        profiler: Rc::default(),
        registry: registry.clone(),
        main_loop: main_loop.clone(),
    };
    let (acting, acts) = pw::channel::channel::<Doing>();
    let _acts = acts.attach(main_loop.loop_(), {
        let binding = binding.clone();
        move |doing: Doing| match doing {
            Doing::Act {
                done: moved,
                answer,
            } => {
                let _ = answer.send(done(&binding.bound.borrow(), &moved));
            }
            Doing::Timing(wanted) => profiling(&binding, binding.profiler.get(), wanted),
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
    let _registry_listener = listening(binding);
    {
        let mut daemon = lock(shared);
        daemon.server = "answering";
        daemon.acting = Some(acting);
    }
    main_loop.run();
    Ok(())
}

/// What the registry says, as it says it: each node, card, metadata and
/// link taken in, and the profiler's global kept, bound while a watch
/// wants it; and each one gone let go.
fn listening(binding: Binding) -> pw::registry::Listener {
    let registry = binding.registry.clone();
    registry
        .add_listener_local()
        .global({
            let binding = binding.clone();
            move |global| {
                let Binding {
                    shared,
                    bound,
                    profiler,
                    registry,
                    ..
                } = &binding;
                let found = match global.type_ {
                    ObjectType::Node => node(shared, registry, global),
                    ObjectType::Device => card(shared, registry, global),
                    ObjectType::Metadata => defaults(shared, registry, global),
                    ObjectType::Link => linked(shared, global),
                    ObjectType::Profiler => {
                        profiler.set(Some(global.id));
                        let wanted = lock(shared).timed;
                        profiling(&binding, Some(global.id), wanted);
                        None
                    }
                    _ => None,
                };
                if let Some(found) = found {
                    bound.borrow_mut().insert(global.id, found);
                }
            }
        })
        .global_remove({
            let Binding {
                shared,
                bound,
                profiler,
                ..
            } = binding;
            // A node, a card, or a link, never bound, goes with whatever it
            // was sent to; the profiler with nothing.
            move |id| match bound.borrow_mut().remove(&id) {
                Some(Bound::Profiler { .. }) | None if profiler.get() == Some(id) => {
                    profiler.set(None);
                }
                Some(Bound::Defaults { .. }) => {
                    told(
                        &shared,
                        Change::Chose {
                            key: None,
                            value: None,
                        },
                    );
                    let (stream, target) = (None, None);
                    told(&shared, Change::Sent { stream, target });
                }
                _ => told(&shared, Change::Gone { id }),
            }
        })
        .register()
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
            let defaults = metadata(bound)?;
            defaults.set_property(
                pw::core::PW_ID_CORE,
                key,
                Some("Spa:String:JSON"),
                Some(value),
            );
            Ok(())
        }
        // Untyped, as pw-metadata sets it, which the session manager reads.
        Move::Send { stream, target } => {
            metadata(bound)?.set_property(*stream, TARGET, None, target.as_deref());
            Ok(())
        }
        Move::Each(moves) => moves.iter().try_for_each(|one| done(bound, one)),
    }
}

/// Hears a link, from the node it goes out of to the one it goes into,
/// as its global says them; it's never bound, since nothing of it moves.
fn linked(shared: &Mutex<Daemon>, global: &GlobalObject<&DictRef>) -> Option<Bound> {
    let props = global.props?;
    let end = |key: &str| props.get(key).and_then(|id| id.parse().ok());
    let (from, to) = (end("link.output.node")?, end("link.input.node")?);
    told(
        shared,
        Change::Link {
            id: global.id,
            from,
            to,
        },
    );
    None
}

/// The default metadata, while it's bound.
fn metadata(bound: &HashMap<u32, Bound>) -> Result<&Metadata, String> {
    bound
        .values()
        .find_map(|bound| match bound {
            Bound::Defaults { proxy, .. } => Some(proxy),
            _ => None,
        })
        .ok_or_else(|| "the default metadata is gone".to_owned())
}

/// Binds a node that's an output or an input, or a program's stream, by
/// the class the registry says of it, hearing its properties, its Props
/// and its Format.
fn node(
    shared: &Arc<Mutex<Daemon>>,
    registry: &RegistryRc,
    global: &GlobalObject<&DictRef>,
) -> Option<Bound> {
    let class = global.props?.get("media.class")?;
    if direction(class).is_none() && playing(class).is_none() {
        return None;
    }
    let id = global.id;
    let node: Node = registry.bind(global).ok()?;
    let listener = node
        .add_listener_local()
        .info({
            let shared = Arc::clone(shared);
            move |info: &NodeInfoRef| {
                // A device's state is its drawer's; a filter's half's is
                // whether sound passes through it.
                if info.change_mask().contains(NodeChangeMask::STATE) {
                    let state = said_state(&info.state());
                    told(&shared, Change::State { id, state });
                }
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
            move |_, kind, _, _, pod| match (kind, pod) {
                (ParamType::Props, Some(pod)) => {
                    let (volumes, muted) = levels(pod.as_bytes());
                    told(&shared, Change::Levels { id, volumes, muted });
                    if let Some(channels) = channels(pod.as_bytes()) {
                        told(&shared, Change::Channels { id, channels });
                    }
                }
                // A node said to have no Format isn't set up to move audio.
                (ParamType::Format, pod) => {
                    let format = pod.and_then(|pod| format(pod.as_bytes()));
                    told(&shared, Change::Format { id, format });
                }
                _ => {}
            }
        })
        .register();
    node.subscribe_params(&[ParamType::Props, ParamType::Format]);
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

/// Binds the default metadata, which names the output and input chosen,
/// and where each program's stream is sent, by the stream's id.
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
                let value = value.map(str::to_owned);
                if subject == pw::core::PW_ID_CORE {
                    let key = key.map(str::to_owned);
                    told(&shared, Change::Chose { key, value });
                } else if key.is_none_or(|key| key == TARGET) {
                    // No key lets go of every key the stream had.
                    let stream = Some(subject);
                    let target = value.filter(|_| key.is_some());
                    told(&shared, Change::Sent { stream, target });
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
    desk::told(&mut records, &daemon.desk);
    let naming = |given: &str| daemon.naming.named(given);
    let ready = served(&records, &daemon.words, &naming);
    let timings = daemon.timings.records();
    let drawers = drawer::served(&records, &ready, &timings, &daemon.words, &naming);
    daemon.served.take(ready, drawers);
    records.extend(daemon.served.all().cloned());
    records.push(daemon.devices.targets());
    records.extend(timings);
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
        timed(daemon);
    }
}

/// Has the profiler bound while some watch wants how nodes keep time, and
/// let go once none does, since the server measures every cycle for it
/// while it's bound; what it said goes with it.
fn timed(daemon: &mut Daemon) {
    let wanted = daemon
        .watches
        .iter()
        .any(|watch| watch.wants_kind("timing"));
    if wanted == daemon.timed {
        return;
    }
    daemon.timed = wanted;
    if let Some(acting) = &daemon.acting {
        let _ = acting.send(Doing::Timing(wanted));
    }
    if !wanted {
        daemon.timings.forget();
        refresh(daemon);
    }
}

/// Binds the profiler by its global's id, once it's known, while a watch
/// wants it, and lets it go once none does. Each profile it says is taken
/// in, and each second a frame stands, the records told again where it
/// changed them: so a graph gone idle, which the profiler says nothing
/// of, is told within a second that nothing runs.
fn profiling(binding: &Binding, id: Option<u32>, wanted: bool) {
    let Some(id) = id else { return };
    let bound = &mut binding.bound.borrow_mut();
    if !wanted {
        bound.remove(&id);
        return;
    }
    if bound.contains_key(&id) {
        return;
    }
    let (heard, ticked) = (Arc::clone(&binding.shared), Arc::clone(&binding.shared));
    let profiler = Profiler::bind(
        &binding.registry,
        &binding.main_loop,
        id,
        move |profile| {
            let mut daemon = lock(&heard);
            if daemon.timed {
                daemon.timings.hear(profile);
            }
        },
        move || {
            let mut daemon = lock(&ticked);
            if daemon.timed && daemon.timings.stand() {
                refresh(&mut daemon);
            }
        },
    );
    if let Some(profiler) = profiler {
        bound.insert(id, Bound::Profiler { _bound: profiler });
    }
}
