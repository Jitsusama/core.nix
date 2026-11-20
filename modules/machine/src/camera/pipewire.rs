//! Who reads a camera through `PipeWire`, found in its registry. A
//! program reading a camera there holds no camera itself: `PipeWire`
//! does, for it. So where `PipeWire` holds one, the programs reading it
//! there are named in its place (`readers`), found by the node
//! `.devices/<name>` says the camera is, or failing that by the camera's
//! name. `PipeWire` stays named beside them wherever a reader can't be
//! named, and alone where none can, or where the registry can't be read,
//! so a camera on is never said to be off.
//!
//! The registry is listed (`pw-cli ls`) rather than dumped (pw-dump),
//! since a dump asks each camera node for its formats, and `PipeWire`
//! opens a camera nobody streams from to answer: a dump every few
//! seconds would open the camera as often, and its holders' file would
//! say `PipeWire` holds it. A reader is named by its process, by the pid
//! `PipeWire` takes from its socket, rather than by the `application.*`
//! it tells `PipeWire`, so a program can't name another for itself that
//! way. A program the camera portal let in reads on a socket the portal
//! connected, so that pid is the portal's, and the registry lists nothing
//! else of the program: it can't be named, so `PipeWire` stays named for
//! it. A process's name is still its own to set, as it is for a camera's
//! other holders, so the name says which program it says it is, and one
//! that gives none counts as a reader that can't be named.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard, PoisonError, mpsc};
use std::time::{Duration, Instant};

use super::{held_cameras, once};
use crate::Root;
use crate::using::program;

/// pw-cli, by its path, as the daemon's home configuration hands it; a
/// machine with none names `PipeWire` as each camera's holder it holds.
pub const PW_CLI: &str = "MACHINE_PW_CLI";

/// The programs that are `PipeWire` itself, holding a camera for whoever
/// reads it through them, by the name a process gives itself.
pub(super) const PIPEWIRE: [&str; 2] = ["pipewire", "wireplumber"];

/// The camera portal, by the start of the name its process gives itself,
/// which the kernel cuts to fifteen characters (`xdg-desktop-por`, or
/// `xdg-desktop-po` once a Nix wrapper's dot is taken off).
const PORTAL: &str = "xdg-desktop-po";

/// How long the registry's listing is waited for, so a `PipeWire` that's
/// stuck holds no read of the cameras back long; it answers in a few
/// milliseconds.
pub(super) const LIST_WAIT: Duration = Duration::from_secs(1);

/// How often the cameras are read again while `PipeWire` holds one, since
/// a program can start or stop reading it there while the camera's file
/// stays as it was.
const THROUGH_EVERY: Duration = Duration::from_secs(2);

/// How long a listing is kept through those that fail after it, so one
/// slow to come on a busy machine doesn't flick the mark to `PipeWire`
/// and back: three reads' worth.
const KEPT_FOR: Duration = Duration::from_secs(6);

/// The most of a listing read, far past any machine's, which is tens of
/// kilobytes: a client stuffing its properties gets a failed listing, not
/// a walk that holds every read up.
const LONGEST: u64 = 4 << 20;

/// One object in `PipeWire`'s registry, as `pw-cli ls` lists it: its id,
/// its kind (`Node`, `Link`, `Client`, `Device`) and its properties.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Global {
    pub id: u64,
    pub kind: String,
    pub props: BTreeMap<String, String>,
}

impl Global {
    /// A property that's a number, as the ids it names others by are.
    fn number(&self, key: &str) -> Option<u64> {
        self.props.get(key)?.parse().ok()
    }
}

/// `pw-cli ls`'s listing as objects: each opens on a line `id N, type
/// PipeWire:Interface:Kind/version`, with its properties below it one to
/// a line as `key = "value"`.
///
/// pw-cli prints a value as it stands, so a client that puts a line break
/// in a property of its own can write lines that read as more. The
/// server's own properties, its `pipewire.sec.pid` among them, are listed
/// before any a client sets, so the first a key is said is the one kept.
/// pw-cli keeps the objects at their ids and lists them in that order
/// (`pw_map_for_each` over its globals), rising, so one said under an id
/// no higher than the last was written into a property, to stand for an
/// object listed before it or after: the listing is refused, rather than
/// read with a reader shadowed or renamed. What's left to a client is an
/// object of its own making under an id nothing has, to name whoever it
/// likes as reading, much as a process can call itself anything.
pub fn registry(text: &str) -> Result<Vec<Global>, String> {
    let mut globals: Vec<Global> = Vec::new();
    for line in text.lines().map(str::trim) {
        if let Some((id, kind)) = line
            .strip_prefix("id ")
            .and_then(|rest| rest.split_once(", type "))
            && let Ok(id) = id.parse()
        {
            if let Some(last) = globals.last()
                && id <= last.id
            {
                // Said the same each time, whatever ids a client picks, so
                // it's said once.
                return Err(
                    "pw-cli's listing was refused, as its ids fall back where a \
                            client's property holds a line break"
                        .to_owned(),
                );
            }
            let kind = kind.strip_prefix("PipeWire:Interface:").unwrap_or(kind);
            let kind = kind.split_once('/').map_or(kind, |(kind, _)| kind);
            globals.push(Global {
                id,
                kind: kind.to_owned(),
                props: BTreeMap::new(),
            });
        } else if let Some(global) = globals.last_mut()
            && let Some((key, value)) = line.split_once(" = ")
        {
            let value = value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .unwrap_or(value);
            global
                .props
                .entry(key.to_owned())
                .or_insert_with(|| value.to_owned());
        }
    }
    Ok(globals)
}

/// Where `PipeWire`'s registry comes from, the last listing of it, and
/// whether the last read found `PipeWire` holding a camera, which has the
/// cameras read again on a clock. It's kept apart from the rest of what's
/// read, under a lock of its own, since it's listed before the reader's
/// lock is taken, so a `PipeWire` slow to answer holds no other read up.
pub struct PipeWire {
    root: Root,
    /// pw-cli, or none to list no registry.
    list: Option<PathBuf>,
    state: Mutex<State>,
}

#[derive(Default)]
pub(super) struct State {
    /// The registry as last listed, while a camera's file names `PipeWire`.
    pub(super) graph: Option<Vec<Global>>,
    /// When the last read found `PipeWire` holding a camera.
    pub(super) held_at: Option<Instant>,
    /// When the registry was last listed whole.
    listed_at: Option<Instant>,
    /// Why the last listing failed, said once until one succeeds or
    /// `PipeWire` lets go.
    pub(super) failing: Option<String>,
    /// The cameras `PipeWire` holds that the last listing showed nothing
    /// of, said once until it shows them.
    pub(super) unfound: HashSet<String>,
}

impl PipeWire {
    /// Its registry from this pw-cli, or none, read for the cameras under
    /// `root`.
    pub fn new(root: &Root, list: Option<PathBuf>) -> Self {
        Self {
            root: root.clone(),
            list,
            state: Mutex::default(),
        }
    }

    /// Its registry from the pw-cli the environment names, if it names one.
    pub fn from_environment(root: &Root) -> Self {
        let list = std::env::var_os(PW_CLI)
            .filter(|list| !list.is_empty())
            .map(PathBuf::from);
        Self::new(root, list)
    }

    pub(super) fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// When the cameras are next due a read though nothing was written:
    /// while `PipeWire` holds a camera, a little after the last read.
    pub fn due(&self) -> Option<Instant> {
        self.state().held_at.map(|at| at + THROUGH_EVERY)
    }

    /// Lists the registry afresh while a camera's file names `PipeWire`,
    /// and forgets it while none does; it's run before the cameras are
    /// read, outside the reader's lock. A listing that fails leaves the
    /// last for up to `KEPT_FOR`, then none.
    pub fn refresh(&self) {
        let held = held_cameras(&self.root)
            .iter()
            .any(|(_, programs)| through_pipewire(programs));
        if !held {
            let mut state = self.state();
            state.graph = None;
            state.listed_at = None;
            state.failing = None;
            state.unfound.clear();
            return;
        }
        let listed = self.listed().and_then(|text| registry(&text));
        let mut state = self.state();
        match listed {
            Ok(graph) => {
                state.graph = Some(graph);
                state.listed_at = Some(Instant::now());
                state.failing = None;
            }
            Err(why) => {
                if state.failing.as_deref() != Some(why.as_str()) {
                    eprintln!(
                        "<4>PipeWire's registry gave no listing to go by, so a camera it \
                         holds is said held by it once the last listing is {KEPT_FOR:?} old: \
                         {why}"
                    );
                }
                state.failing = Some(why);
                if state.listed_at.is_none_or(|at| at.elapsed() >= KEPT_FOR) {
                    state.graph = None;
                }
            }
        }
    }

    /// The registry as pw-cli lists it, or why it couldn't within
    /// `LIST_WAIT`.
    fn listed(&self) -> Result<String, String> {
        let Some(list) = self.list.as_ref() else {
            return Err(format!("no pw-cli was handed over in {PW_CLI}"));
        };
        let mut child = Command::new(list)
            .arg("ls")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("{}: {error}", list.display()))?;
        let Some(mut stdout) = child.stdout.take() else {
            return Err("pw-cli gave no output to read".to_owned());
        };
        let (said, heard) = mpsc::channel();
        // Its stdout closes as it ends, so what it said is the sign it's done.
        // A client's property may hold bytes that are no UTF-8, which are
        // read as the replacement character rather than losing the rest.
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let read = stdout
                .by_ref()
                .take(LONGEST)
                .read_to_end(&mut bytes)
                .and_then(|_| stdout.read(&mut [0]))
                .map_err(|error| format!("pw-cli's listing: {error}"))
                .and_then(|more| match more {
                    0 => Ok(String::from_utf8_lossy(&bytes).into_owned()),
                    _ => Err(format!("pw-cli listed more than {LONGEST} bytes")),
                });
            let _ = said.send(read);
        });
        let Ok(read) = heard.recv_timeout(LIST_WAIT) else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("pw-cli didn't answer within {LIST_WAIT:?}"));
        };
        let text = match read {
            Ok(text) => text,
            Err(why) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(why);
            }
        };
        let ended = child.wait().map_err(|error| format!("pw-cli: {error}"))?;
        if !ended.success() {
            return Err(format!("pw-cli ended {ended}"));
        }
        Ok(text)
    }
}

/// Whether one of a camera's holders is `PipeWire`.
pub(super) fn through_pipewire(programs: &[String]) -> bool {
    programs
        .iter()
        .any(|holder| PIPEWIRE.contains(&holder.as_str()))
}

/// Who reads a camera through `PipeWire`, as its registry shows them, to
/// stand in `PipeWire`'s place among its holders: the camera's device, by
/// its node or failing that by its name where no other camera has it,
/// then its nodes that `PipeWire`'s own programs made, since any client
/// can say a device of its node, then whoever's linked to them, passing
/// through any node `PipeWire`'s own programs run, as a converter of
/// theirs is, to the program it serves. They're sorted, and `PipeWire` is
/// kept after
/// them wherever a reader can't be named (a node with no client, a client
/// with no process, or a node of `PipeWire`'s with nobody after it), and
/// alone where nobody is, so a camera it holds never reads as off. None
/// when the registry shows no such camera, as one not offered there is.
pub fn readers(
    graph: &[Global],
    device: Option<&str>,
    card: &str,
    root: &Root,
) -> Option<Vec<String>> {
    let typed = |kind: &'static str| graph.iter().filter(move |global| global.kind == kind);
    let devices: Vec<u64> = {
        let path = device.map(|device| format!("v4l2:{device}"));
        let by_path: Vec<u64> = typed("Device")
            .filter(|global| path.is_some() && global.props.get("object.path") == path.as_ref())
            .map(|global| global.id)
            .collect();
        let by_card = || {
            let named: Vec<u64> = typed("Device")
                .filter(|global| {
                    global.props.get("device.description").map(String::as_str) == Some(card)
                })
                .map(|global| global.id)
                .collect();
            // Two cameras of one name can't be told apart by it.
            if named.len() == 1 { named } else { Vec::new() }
        };
        if by_path.is_empty() {
            by_card()
        } else {
            by_path
        }
    };
    if devices.is_empty() {
        return None;
    }
    let by_id = |kind: &'static str| -> HashMap<u64, &Global> {
        typed(kind).map(|global| (global.id, global)).collect()
    };
    let (nodes, clients) = (by_id("Node"), by_id("Client"));
    let mut links: HashMap<u64, Vec<&Global>> = HashMap::new();
    for link in typed("Link") {
        if let Some(output) = link.number("link.output.node") {
            links.entry(output).or_default().push(link);
        }
    }
    let process = |node: u64| {
        let client = nodes.get(&node)?.number("client.id")?;
        let pid = clients.get(&client)?.number("pipewire.sec.pid")?;
        let name = program(root.read(&format!("/proc/{pid}/comm"))?.trim());
        // The camera portal connects for the program it lets in, so the
        // socket's pid is the portal's, not the program's.
        (!name.is_empty() && !name.starts_with(PORTAL)).then_some(name)
    };
    let mut frontier: Vec<u64> = typed("Node")
        .filter(|global| {
            global
                .number("device.id")
                .is_some_and(|id| devices.contains(&id))
                && process(global.id).is_some_and(|name| PIPEWIRE.contains(&name.as_str()))
        })
        .map(|global| global.id)
        .collect();
    let mut seen: HashSet<u64> = frontier.iter().copied().collect();
    let mut named = Vec::new();
    let mut unknown = false;
    while let Some(node) = frontier.pop() {
        let onward = links.get(&node).map_or(&[][..], Vec::as_slice);
        for link in onward {
            let Some(input) = link.number("link.input.node") else {
                unknown = true;
                continue;
            };
            if !seen.insert(input) {
                continue;
            }
            match process(input) {
                Some(name) if PIPEWIRE.contains(&name.as_str()) => frontier.push(input),
                Some(name) => once(&mut named, &name),
                None => unknown = true,
            }
        }
        // A node of `PipeWire`'s own that passes the camera to nobody is
        // read by something no link names.
        let device_node = nodes
            .get(&node)
            .and_then(|global| global.number("device.id"))
            .is_some_and(|id| devices.contains(&id));
        unknown |= onward.is_empty() && !device_node;
    }
    named.sort();
    if named.is_empty() || unknown {
        named.push("pipewire".to_owned());
    }
    Some(named)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::fmt::Write as _;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    /// A machine of its own for each test, under the temporary folder.
    pub(crate) fn machine(name: &str) -> Root {
        let root =
            std::env::temp_dir().join(format!("machine-camera-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        Root::new(root)
    }

    /// Each process the graph's clients are, by its pid, as /proc says.
    pub(crate) fn processes(root: &Root) {
        for (pid, name) in [
            (100, "pipewire"),
            (101, "wireplumber"),
            (102, ".tuple-wrapped"),
            (103, "chrome"),
            (104, "zoom"),
        ] {
            let folder = root.path(&format!("/proc/{pid}"));
            fs::create_dir_all(&folder).unwrap();
            fs::write(folder.join("comm"), format!("{name}\n")).unwrap();
        }
    }

    /// A registry as `pw-cli ls` lists one: the camera's device and its
    /// node, and for each program reading it, its client, its stream's
    /// node and the link between, with `PipeWire`'s own converter between
    /// the camera and Tuple; Chrome calls itself Firefox, which nothing
    /// believes; a second node of the camera's that nobody reads; and a
    /// microphone only Zoom reads. Its ids start past `base`, so two
    /// listings can be one registry.
    pub(crate) fn listing(card: &str, path: &str, base: u64) -> String {
        let global = |id: u64, kind: &str, props: &[(&str, String)]| {
            let id = base + id;
            let mut text = format!("\tid {id}, type PipeWire:Interface:{kind}/3\n");
            for (key, value) in props {
                writeln!(text, " \t\t{key} = \"{value}\"").unwrap();
            }
            text
        };
        let client = |id: u64, pid: u64, name: &str| {
            global(
                id,
                "Client",
                &[
                    ("pipewire.sec.pid", pid.to_string()),
                    ("application.name", name.to_owned()),
                ],
            )
        };
        let node = |id: u64, client: u64, device: Option<u64>| {
            let mut props = vec![("client.id", (base + client).to_string())];
            props.extend(device.map(|device| ("device.id", (base + device).to_string())));
            global(id, "Node", &props)
        };
        let link = |id: u64, from: u64, to: u64| {
            global(
                id,
                "Link",
                &[
                    ("link.output.node", (base + from).to_string()),
                    ("link.input.node", (base + to).to_string()),
                ],
            )
        };
        [
            client(1, 100, "pipewire"),
            client(2, 101, "WirePlumber"),
            client(3, 102, "Tuple"),
            client(4, 103, "Firefox"),
            client(5, 104, "zoom"),
            global(
                10,
                "Device",
                &[
                    ("device.api", "v4l2".to_owned()),
                    ("device.description", card.to_owned()),
                    ("object.path", format!("v4l2:{path}")),
                ],
            ),
            node(11, 1, Some(10)),
            node(12, 2, None),
            node(13, 3, None),
            node(14, 4, None),
            node(15, 1, Some(10)),
            global(20, "Device", &[("object.path", "alsa:hw:0".to_owned())]),
            node(21, 1, Some(20)),
            node(22, 5, None),
            link(30, 11, 12),
            link(31, 12, 13),
            link(32, 11, 14),
            link(33, 21, 22),
        ]
        .concat()
    }

    fn graph(root: &Root, card: &str, path: &str) -> Vec<Global> {
        processes(root);
        registry(&listing(card, path, 0)).expect("the listing rises by id")
    }

    /// Lists the registry until a listing comes whole, as one forking a
    /// shell on a starved builder may miss `LIST_WAIT` a time or two, and
    /// says how many tries it took.
    pub(crate) fn listed_whole(pipewire: &PipeWire) -> usize {
        for tries in 1..=5 {
            pipewire.refresh();
            // A failed listing leaves the last good one's time as it was,
            // so it's the failure that's looked for.
            let state = pipewire.state();
            if state.listed_at.is_some() && state.failing.is_none() {
                return tries;
            }
        }
        panic!("no listing came whole in five tries");
    }

    /// Who a camera's read by, as `readers` says when it knows.
    pub(crate) fn named(names: &[&str]) -> Option<Vec<String>> {
        let names: Vec<String> = names.iter().map(|&name| name.to_owned()).collect();
        (!names.is_empty()).then_some(names)
    }

    /// A pw-cli written for the test, as the daemon would be handed one;
    /// the caller holds `alone()` while it's written and run.
    pub(crate) fn pw_cli(root: &Root, body: &str) -> PathBuf {
        let path = std::env::var_os("PATH").expect("the test has a PATH");
        let shell = std::env::split_paths(&path)
            .map(|folder| folder.join("sh"))
            .find(|sh| sh.exists())
            .expect("a shell is on PATH");
        let folder = root.path("/bin");
        fs::create_dir_all(&folder).unwrap();
        let program = folder.join("pw-cli");
        fs::write(&program, format!("#!{}\n{body}\n", shell.display())).unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        program
    }

    #[test]
    fn a_listing_as_pw_cli_gives_one_is_read_as_its_objects() {
        // A client, a link, the two nodes it joins, a stream's and a
        // device's, and the loopback camera, as this machine's pw-cli
        // listed them.
        let globals = registry(include_str!("listed.txt")).expect("it rises by id");
        let kinds: Vec<(u64, &str)> = globals
            .iter()
            .map(|global| (global.id, global.kind.as_str()))
            .collect();
        assert_eq!(
            kinds,
            [
                (33, "Client"),
                (34, "Link"),
                (46, "Node"),
                (124, "Device"),
                (223, "Node")
            ]
        );
        assert_eq!(globals[0].number("pipewire.sec.pid"), Some(2180));
        assert_eq!(globals[1].number("link.output.node"), Some(46));
        assert_eq!(globals[1].number("link.input.node"), Some(223));
        // The two keys a reader is found by: a node's device, and its client.
        assert_eq!(globals[2].number("device.id"), None);
        assert_eq!(globals[2].number("client.id"), Some(36));
        assert_eq!(globals[4].number("device.id"), Some(160));
        assert_eq!(globals[4].number("client.id"), Some(33));
        assert_eq!(globals[3].props["object.path"], "v4l2:/dev/video32");
        assert_eq!(globals[3].props["device.description"], "Built-in Camera");
    }

    #[test]
    fn the_listing_is_read_as_its_objects() {
        let text = "\tid 33, type PipeWire:Interface:Client/3\n \t\tobject.serial = \"34\"\n \
                    \t\tapplication.name = \"Wire = Plumber\"\n\tid 34, type \
                    PipeWire:Interface:Link/3\n \t\tlink.output.node = \"46\"\nremote 0 \
                    is named 'pipewire-0'\n";
        let globals = registry(text).expect("it rises by id");
        assert_eq!(globals.len(), 2);
        assert_eq!((globals[0].id, globals[0].kind.as_str()), (33, "Client"));
        assert_eq!(globals[0].props["application.name"], "Wire = Plumber");
        assert_eq!(globals[1].number("link.output.node"), Some(46));
        assert_eq!(
            globals[1].props.len(),
            1,
            "a line that's no property is none"
        );
    }

    #[test]
    fn a_client_writing_lines_into_its_own_property_shadows_nothing() {
        // Its name carries a break and a pid of its choosing.
        let client = "\tid 33, type PipeWire:Interface:Client/3\n \t\tpipewire.sec.pid = \"7\"\n \
                      \t\tapplication.name = \"x\n \t\tpipewire.sec.pid = \"102\"\n";
        let globals = registry(client).expect("it rises by id");
        assert_eq!(
            globals[0].number("pipewire.sec.pid"),
            Some(7),
            "the server's pid, said first"
        );
        // Then an object under an id said before it, or one said after it.
        let link = |id: u64| {
            format!("\tid {id}, type PipeWire:Interface:Link/3\n \t\tlink.input.node = \"9\"\n")
        };
        let after = format!("{}\tid 34, type PipeWire:Interface:Node/3\n", link(40));
        for (what, written) in [("its own", link(33)), ("one after", after)] {
            assert!(
                registry(&format!("{client}{written}")).is_err(),
                "a Link written under {what} id is no listing to read"
            );
        }
    }

    #[test]
    fn the_programs_reading_through_pipewire_are_found_by_the_node() {
        let root = machine("node");
        let graph = graph(&root, "Built-in Camera", "/dev/video32");
        assert_eq!(
            readers(&graph, Some("/dev/video32"), "camera", &root),
            named(&["chrome", "tuple"]),
            "by each process, past PipeWire's converter, with nobody on the camera's idle \
             node, and none from the microphone"
        );
    }

    #[test]
    fn a_reader_the_portal_let_in_or_with_no_name_leaves_pipewire_named() {
        let root = machine("portal");
        let mut graph = graph(&root, "Built-in Camera", "/dev/video32");
        graph.extend(
            registry(
                "\tid 60, type PipeWire:Interface:Client/3\n \t\tpipewire.sec.pid = \"105\"\n\tid \
                 61, type PipeWire:Interface:Node/3\n \t\tclient.id = \"60\"\n\tid 62, type \
                 PipeWire:Interface:Link/3\n \t\tlink.output.node = \"11\"\n \t\tlink.input.node \
                 = \"61\"\n",
            )
            .expect("it rises by id"),
        );
        fs::create_dir_all(root.path("/proc/105")).unwrap();
        for comm in ["xdg-desktop-por", ".xdg-desktop-po", "", " "] {
            fs::write(root.path("/proc/105/comm"), format!("{comm}\n")).unwrap();
            assert_eq!(
                readers(&graph, Some("/dev/video32"), "camera", &root),
                named(&["chrome", "tuple", "pipewire"]),
                "a reader whose process calls itself {comm:?}"
            );
        }
    }

    #[test]
    fn a_listing_is_kept_through_a_failed_one_only_a_while() {
        let root = machine("kept-through");
        processes(&root);
        let folder = root.path(super::super::FOLDER);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("Built-in Camera"), "pipewire\n").unwrap();
        let listed = root.path("/listed");
        fs::write(&listed, listing("Built-in Camera", "/dev/video32", 0)).unwrap();
        let failing = root.path("/failing");
        let _alone = crate::acts::tests::alone();
        let cli = pw_cli(
            &root,
            &format!(
                "[ -e '{}' ] && cat '{}'; cat '{}'",
                failing.display(),
                listed.display(),
                listed.display()
            ),
        );
        let pipewire = PipeWire::new(&root, Some(cli));
        let programs = || super::super::cameras(&root, &pipewire)[0]["programs"].clone();
        listed_whole(&pipewire);
        let readers = serde_json::json!(["chrome", "tuple"]);
        assert_eq!(programs(), readers, "listed");
        fs::write(&failing, "").unwrap();
        // Tried until one comes, as a starved builder may miss the wait.
        let refused = || {
            pipewire
                .state()
                .failing
                .as_deref()
                .is_some_and(|why| why.contains("refused"))
        };
        for _ in 0..5 {
            pipewire.refresh();
            if refused() {
                break;
            }
        }
        assert!(refused(), "a listing whose ids fall back read as one");
        assert_eq!(programs(), readers, "kept through one a client wrote into");
        pipewire.state().listed_at = Some(
            Instant::now()
                .checked_sub(KEPT_FOR)
                .expect("the machine has been up longer than a listing is kept"),
        );
        pipewire.refresh();
        assert_eq!(
            programs(),
            serde_json::json!(["pipewire"]),
            "not kept once it's old"
        );
    }

    #[test]
    fn a_camera_with_no_node_said_is_found_by_its_name_where_its_alone_in_it() {
        let root = machine("card");
        let graph = graph(&root, "MX Brio 705 for Business", "/dev/video40");
        let readers_of =
            |graph: &[Global], device| readers(graph, device, "MX Brio 705 for Business", &root);
        let all = named(&["chrome", "tuple"]);
        assert_eq!(readers_of(&graph, None), all);
        assert_eq!(
            readers_of(&graph, Some("/dev/video9")),
            all,
            "a node since renumbered"
        );
        let mut twins = graph.clone();
        let twin = listing("MX Brio 705 for Business", "/dev/video42", 100);
        // Only Tuple reads the twin.
        let twin: String = twin
            .split_inclusive("\tid ")
            .filter(|object| !object.starts_with("132,"))
            .collect();
        twins.extend(registry(&twin).expect("it rises by id"));
        assert_eq!(readers_of(&twins, None), None, "two cameras of one name");
        assert_eq!(
            readers_of(&twins, Some("/dev/video42")),
            named(&["tuple"]),
            "each found by its own node"
        );
        assert_eq!(readers_of(&twins, Some("/dev/video40")), all);
    }

    #[test]
    fn a_camera_pipewire_doesnt_offer_has_its_readers_unknown() {
        let root = machine("offered");
        let graph = graph(&root, "Built-in Camera", "/dev/video32");
        assert_eq!(
            readers(&graph, Some("/dev/video40"), "ir-camera", &root),
            None
        );
        assert_eq!(readers(&[], Some("/dev/video32"), "camera", &root), None);
    }

    #[test]
    fn a_reader_saying_its_node_is_the_cameras_is_named_all_the_same() {
        let root = machine("claimed");
        let mut graph = graph(&root, "Built-in Camera", "/dev/video32");
        // Firefox's node, as its client may say of it.
        for global in &mut graph {
            if global.id == 14 {
                global.props.insert("device.id".to_owned(), "10".to_owned());
            }
        }
        assert_eq!(
            readers(&graph, Some("/dev/video32"), "camera", &root),
            named(&["chrome", "tuple"])
        );
    }

    #[test]
    fn pipewire_is_kept_wherever_a_reader_cant_be_named() {
        let root = machine("kept");
        let graph = graph(&root, "Built-in Camera", "/dev/video32");
        let without = |drop: &dyn Fn(&Global) -> bool| {
            let graph: Vec<Global> = graph
                .iter()
                .filter(|global| !drop(global))
                .cloned()
                .collect();
            readers(&graph, Some("/dev/video32"), "camera", &root)
        };
        assert_eq!(
            without(&|global| global.kind == "Link"),
            named(&["pipewire"]),
            "nobody linked"
        );
        assert_eq!(
            without(&|global| global.id == 4),
            named(&["tuple", "pipewire"]),
            "a client not listed"
        );
        let lacking = |id: u64, key: &str| {
            let mut graph = graph.clone();
            for global in &mut graph {
                if global.id == id {
                    global.props.remove(key);
                }
            }
            readers(&graph, Some("/dev/video32"), "camera", &root)
        };
        assert_eq!(
            lacking(4, "pipewire.sec.pid"),
            named(&["tuple", "pipewire"]),
            "a client with no process said, though it names itself"
        );
        assert_eq!(
            lacking(32, "link.input.node"),
            named(&["tuple", "pipewire"]),
            "a link to nowhere said"
        );
        assert_eq!(
            without(&|global| global.id == 31),
            named(&["chrome", "pipewire"]),
            "PipeWire's converter passing the camera to nobody"
        );
        fs::remove_file(root.path("/proc/103/comm")).unwrap();
        assert_eq!(
            readers(&graph, Some("/dev/video32"), "camera", &root),
            named(&["tuple", "pipewire"]),
            "a process gone"
        );
    }
}
