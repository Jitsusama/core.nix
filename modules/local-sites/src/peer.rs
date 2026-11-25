//! Who is at the other end of a connection to this server: the person, one
//! of the machine's agents, or nothing that can be told. An answer pressed
//! on a page speaks as the person, and the node takes this server's steers
//! as the person's since it runs as the person's own unit, so the server
//! must not carry one an agent sent. Every header an agent could send is its
//! own to forge, and a fenced agent shares the loopback network, so the
//! kernel is asked instead: the connection's socket, by its two ends, in
//! `/proc/net/tcp`; the process holding that socket, by its inode, among
//! this account's open files; and that process told as the node tells who
//! asks it to steer (mycelium's asker), an agent's by the unit it runs in
//! or a pi among its forebears, the person's once its forebears end
//! without one. A socket of another account, or one nothing can be read
//! of, is told as nothing, and refused as an agent's is.
//!
//! As the node's, this is a fence against habit and mistakes, not a
//! boundary: within one account a process can move itself out of an
//! agent's unit or out from under pi. It keeps an agent from passing a
//! press off as the person's by the plain way, a request with forged
//! headers. Only Linux tells a socket's holder; elsewhere nothing here can
//! be asked, and the server takes no answers.

use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

/// How the names of the units the machine runs its agents in begin, below
/// the person's service manager's own unit, as mycelium's asker names them:
/// pi's hosts, each held session's scope, and the queues agents' commands
/// wait in.
const AGENTS_UNITS: [&str; 4] = [
    "agents.slice/pi-host-",
    "agents.slice/mycelium-pi-",
    "jobs.slice/",
    "offload.slice/",
];

/// The name pi's own process goes by.
const PI: &str = "pi";

/// The name a pi host's process goes by where no agent's unit tells it.
const PI_HOST: &str = "pi-host";

/// How many forebears are looked through at most, so a chain the kernel
/// tells wrong can't hold an answer up for long.
const FOREBEARS: usize = 64;

/// A connection that's up, as `/proc/net/tcp` says its state.
const ESTABLISHED: &str = "01";

/// The kernel's tables of TCP sockets, for each kind of address.
const TABLES: [&str; 2] = ["net/tcp", "net/tcp6"];

/// Who is at a connection's other end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Told {
    /// The person's own, which may speak as the person.
    Person,
    /// An agent's, named by its unit or the pi above it.
    Agent(String),
    /// Nothing could be told, for the reason given.
    Untold(&'static str),
}

/// Where the kernel tells of this machine's processes, and whose they are.
#[derive(Debug, Clone)]
pub struct Processes {
    /// `/proc`, or a stand-in for it.
    root: PathBuf,
    /// This server's account, the only one whose processes are read.
    uid: u32,
}

impl Processes {
    /// This machine's own, where its kernel tells a socket's holder: on
    /// Linux alone.
    #[must_use]
    pub fn here() -> Option<Self> {
        if !cfg!(target_os = "linux") {
            return None;
        }
        let uid = fs::metadata("/proc/self").ok()?.uid();
        Some(Self::at(PathBuf::from("/proc"), uid))
    }

    /// Those a folder laid out as `/proc` tells, of the account `uid`.
    #[must_use]
    pub const fn at(root: PathBuf, uid: u32) -> Self {
        Self { root, uid }
    }

    /// Who holds the end `peer` of a connection to this server at `server`.
    #[must_use]
    pub fn tell(&self, peer: SocketAddr, server: SocketAddr) -> Told {
        let Some(inode) = self.socket(peer, server) else {
            return Told::Untold("no socket of this account holds the connection");
        };
        let holders = self.holding(inode);
        if holders.is_empty() {
            return Told::Untold("no process of this account holds its socket");
        }
        // A socket a process handed its children is held by each; any one
        // of them an agent's could send the request.
        for pid in holders {
            let told = self.process(pid);
            if told != Told::Person {
                return told;
            }
        }
        Told::Person
    }

    /// The inode of the socket of this account whose own end is `peer` and
    /// whose other is `server`.
    fn socket(&self, peer: SocketAddr, server: SocketAddr) -> Option<u64> {
        for table in TABLES {
            let Ok(text) = fs::read_to_string(self.root.join(table)) else {
                continue;
            };
            for line in text.lines().skip(1) {
                let fields: Vec<&str> = line.split_whitespace().collect();
                let [_, local, remote, state, _, _, _, uid, _, inode, ..] = fields[..] else {
                    continue;
                };
                if state != ESTABLISHED
                    || address(local) != Some(peer)
                    || address(remote) != Some(server)
                {
                    continue;
                }
                // Another account's socket is never told as the person's.
                return (uid.parse::<u32>().ok()? == self.uid)
                    .then(|| inode.parse().ok())
                    .flatten();
            }
        }
        None
    }

    /// The processes of this account holding the socket `inode` open.
    fn holding(&self, inode: u64) -> Vec<i32> {
        let held = format!("socket:[{inode}]");
        let Ok(entries) = fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut holders = Vec::new();
        for entry in entries.flatten() {
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse().ok())
            else {
                continue;
            };
            if entry.metadata().map(|about| about.uid()).ok() != Some(self.uid) {
                continue;
            }
            let Ok(open) = fs::read_dir(entry.path().join("fd")) else {
                continue;
            };
            let holds = open.flatten().any(|file| {
                fs::read_link(file.path()).is_ok_and(|link| link.as_os_str() == held.as_str())
            });
            if holds {
                holders.push(pid);
            }
        }
        holders
    }

    /// What is told of process `pid`: an agent's by its unit, else by its
    /// forebears.
    fn process(&self, pid: i32) -> Told {
        let process = self.root.join(pid.to_string());
        if let Some(unit) = fs::read_to_string(process.join("cgroup"))
            .ok()
            .as_deref()
            .and_then(agents_unit)
        {
            return Told::Agent(unit);
        }
        let mut at = pid;
        for _ in 0..FOREBEARS {
            let status = fs::read_to_string(self.root.join(at.to_string()).join("status")).ok();
            let Some((name, parent)) = status.as_deref().and_then(named) else {
                return Told::Untold("a forebear of its process can't be read");
            };
            if name == PI || name == PI_HOST {
                return Told::Agent(format!("{name}'s process {at}"));
            }
            if parent <= 1 {
                return Told::Person;
            }
            at = parent;
        }
        Told::Untold("its forebears run past what's looked through")
    }
}

/// An address as `/proc/net/tcp` writes one, its address's bytes in the
/// machine's own order and its port in hexadecimal: `0100007F:1F90` is
/// 127.0.0.1:8080 here.
fn address(written: &str) -> Option<SocketAddr> {
    let (ip, port) = written.split_once(':')?;
    let port = u16::from_str_radix(port, 16).ok()?;
    let word = |at: usize| {
        ip.get(at..at + 8)
            .and_then(|hex| u32::from_str_radix(hex, 16).ok())
            .map(u32::to_ne_bytes)
    };
    let ip = match ip.len() {
        8 => IpAddr::V4(Ipv4Addr::from(word(0)?)),
        32 => {
            let words = [word(0)?, word(8)?, word(16)?, word(24)?];
            let bytes: [u8; 16] = words.concat().try_into().ok()?;
            IpAddr::V6(Ipv6Addr::from(bytes))
        }
        _ => return None,
    };
    Some(SocketAddr::new(ip, port))
}

/// The agent's unit a process runs in, as its `cgroup` file tells it, or
/// none for any other, read as mycelium's asker reads it.
fn agents_unit(cgroup: &str) -> Option<String> {
    cgroup.lines().find_map(|line| {
        // Control groups v2 tell one line: `0::<path>`.
        let path = line.strip_prefix("0::")?;
        let (_, below) = path.split_once(".service/").filter(|(manager, _)| {
            manager
                .rsplit('/')
                .next()
                .is_some_and(|unit| unit.starts_with("user@"))
        })?;
        AGENTS_UNITS
            .iter()
            .any(|unit| below.starts_with(unit))
            .then(|| below.rsplit('/').next().unwrap_or(below).to_owned())
    })
}

/// A process's name and its parent's id, as its `status` file tells them.
fn named(status: &str) -> Option<(&str, i32)> {
    let field = |key: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .map(str::trim)
    };
    Some((field("Name:")?, field("PPid:")?.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::path::Path;

    /// The person's service manager's unit, as a cgroup path begins below it.
    const MANAGER: &str = "0::/user.slice/user-1000.slice/user@1000.service/";

    /// A folder laid out as `/proc`, taken away once the test ends.
    struct Proc {
        root: PathBuf,
    }

    impl Drop for Proc {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    impl Proc {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir()
                .join(format!("local-sites-peer-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(root.join("net")).expect("its net folder");
            Self { root }
        }

        /// Process `pid`, named `name`, child of `parent`, in `cgroup`.
        fn process(&self, pid: i32, name: &str, parent: i32, cgroup: &str) -> PathBuf {
            let at = self.root.join(pid.to_string());
            fs::create_dir_all(at.join("fd")).expect("its fd folder");
            fs::write(
                at.join("status"),
                format!("Name:\t{name}\nPPid:\t{parent}\n"),
            )
            .expect("its status");
            fs::write(at.join("cgroup"), cgroup).expect("its cgroup");
            at
        }

        /// Process `pid` holding socket `inode` open as its file 3.
        fn holds(&self, pid: i32, inode: u64) {
            symlink(
                format!("socket:[{inode}]"),
                self.root.join(format!("{pid}/fd/3")),
            )
            .expect("its socket");
        }

        /// The kernel's table, with the browser's end of a connection to
        /// 127.0.0.1:8484 from port 40000, as `uid`'s socket `inode`.
        fn connected(&self, uid: u32, inode: u64) {
            let head = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when \
                        retrnsmt   uid  timeout inode";
            let listening = "   0: 0100007F:2124 00000000:0000 0A 00000000:00000000 00:00000000 \
                             00000000  1000        0 11 1 0000000000000000 100 0 0 10 0";
            let browser = format!(
                "   1: 0100007F:9C40 0100007F:2124 01 00000000:00000000 00:00000000 \
                 00000000 {uid:>5}        0 {inode} 1 0000000000000000 20 4 30 10 -1"
            );
            fs::write(
                self.root.join("net/tcp"),
                format!("{head}\n{listening}\n{browser}\n"),
            )
            .expect("its table");
        }

        fn processes(&self) -> Processes {
            Processes::at(self.root.clone(), uid_of(&self.root))
        }
    }

    fn uid_of(path: &Path) -> u32 {
        fs::metadata(path).expect("it's there").uid()
    }

    fn ends() -> (SocketAddr, SocketAddr) {
        (
            "127.0.0.1:40000".parse().expect("the browser's end"),
            "127.0.0.1:8484".parse().expect("the server's end"),
        )
    }

    #[test]
    fn the_kernels_addresses_are_read_in_the_machines_own_order() {
        let here = |written| address(written).map(|at| at.to_string());
        let loopback = if cfg!(target_endian = "little") {
            "0100007F:2124"
        } else {
            "7F000001:2124"
        };
        assert_eq!(here(loopback).as_deref(), Some("127.0.0.1:8484"));
        let six = if cfg!(target_endian = "little") {
            "00000000000000000000000001000000:1F90"
        } else {
            "00000000000000000000000000000001:1F90"
        };
        assert_eq!(here(six).as_deref(), Some("[::1]:8080"));
        assert_eq!(here("0100007F"), None, "an address with no port was read");
        assert_eq!(
            here("01007F:2124"),
            None,
            "an address of no length was read"
        );
    }

    #[test]
    fn a_browser_the_person_started_is_the_persons() {
        if cfg!(target_endian = "big") {
            return;
        }
        let proc = Proc::new("person");
        let uid = uid_of(&proc.root);
        proc.connected(uid, 777);
        proc.process(900, "systemd", 1, &format!("{MANAGER}init.scope\n"));
        proc.process(
            910,
            "chromium",
            900,
            &format!("{MANAGER}app.slice/app-chromium.scope\n"),
        );
        proc.holds(910, 777);
        let (peer, server) = ends();
        assert_eq!(proc.processes().tell(peer, server), Told::Person);
    }

    #[test]
    fn a_press_from_an_agents_unit_or_below_pi_is_an_agents() {
        if cfg!(target_endian = "big") {
            return;
        }
        let (peer, server) = ends();
        let proc = Proc::new("agent");
        let uid = uid_of(&proc.root);
        proc.connected(uid, 777);
        let unit = format!("{MANAGER}agents.slice/pi-host-abc.service\n");
        proc.process(920, "curl", 1, &unit);
        proc.holds(920, 777);
        assert_eq!(
            proc.processes().tell(peer, server),
            Told::Agent("pi-host-abc.service".to_owned()),
            "a press from an agent's unit was the person's"
        );
        let proc = Proc::new("below-pi");
        proc.connected(uid, 777);
        proc.process(930, "pi", 1, &format!("{MANAGER}app.slice/kitty.scope\n"));
        proc.process(
            931,
            "curl",
            930,
            &format!("{MANAGER}app.slice/kitty.scope\n"),
        );
        proc.holds(931, 777);
        assert_eq!(
            proc.processes().tell(peer, server),
            Told::Agent("pi's process 930".to_owned()),
            "a press from below pi was the person's"
        );
    }

    #[test]
    fn a_press_nothing_can_be_told_of_is_untold() {
        if cfg!(target_endian = "big") {
            return;
        }
        let (peer, server) = ends();
        let proc = Proc::new("untold");
        let uid = uid_of(&proc.root);
        proc.connected(uid, 777);
        let told = proc.processes().tell(peer, server);
        assert!(
            matches!(told, Told::Untold(_)),
            "a socket nobody holds was told: {told:?}"
        );
        proc.connected(uid + 1, 777);
        proc.process(940, "chromium", 1, &format!("{MANAGER}app.slice/x.scope\n"));
        proc.holds(940, 777);
        let told = proc.processes().tell(peer, server);
        assert!(
            matches!(told, Told::Untold(_)),
            "another account's socket was told: {told:?}"
        );
        proc.connected(uid, 777);
        proc.process(
            941,
            "chromium",
            942,
            &format!("{MANAGER}app.slice/x.scope\n"),
        );
        proc.holds(941, 777);
        let told = proc.processes().tell(peer, server);
        assert!(
            matches!(told, Told::Untold(_)),
            "an unreadable forebear was told: {told:?}"
        );
    }

    #[test]
    fn only_the_units_mycelium_names_are_agents() {
        for (cgroup, unit) in [
            ("agents.slice/pi-host-a.service", Some("pi-host-a.service")),
            (
                "agents.slice/mycelium-pi-b.scope",
                Some("mycelium-pi-b.scope"),
            ),
            ("jobs.slice/run-1.scope", Some("run-1.scope")),
            ("offload.slice/o.scope", Some("o.scope")),
            ("agents.slice/kitty.scope", None),
            ("app.slice/chromium.scope", None),
        ] {
            assert_eq!(
                agents_unit(&format!("{MANAGER}{cgroup}\n")).as_deref(),
                unit,
                "{cgroup}"
            );
        }
        assert_eq!(
            agents_unit("0::/system.slice/jobs.slice/x.scope\n"),
            None,
            "a unit outside the person's service manager is no agent's"
        );
    }
}
