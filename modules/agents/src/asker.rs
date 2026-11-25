//! Who asked, for an ask only the shell may make: Start New Work, which
//! begins new work as Joel (sprout.rs), and an answer, which the node
//! takes as his (main.rs's answered). The agents daemon's socket takes
//! any process of Joel's account, an agent's command among them, and the
//! node judges the daemon as Joel, so a sprout an agent could ask here
//! would start work with more than the agent's own reach, and an answer
//! would answer its own ask as him. The kernel
//! names the process at the other end by pidfd (`SO_PEERPIDFD`), never a
//! pid an ask says, and its cgroup says which unit runs it: only the
//! shell's unit, under this daemon's own account, and only the unit's
//! main process, as its manager names it, running quickshell's own
//! program. Other programs run in that unit too (pi-ask while Super+S
//! talks, the shell's helpers, their children and orphans), and any of
//! them could run a program named quickshell, so the program alone is a
//! second line, as pineapple's peer.rs holds it (ISXEG8 on #5456).
//! The shell in turn starts new work and answers only from Joel's own
//! Return, never a walk from outside (Agents.qml's sprout and
//! answerKept, Now.qml's answerLook).
//!
//! What this holds to: a fenced command can't move itself out of its own
//! cgroup, but an unfenced process of Joel's could write itself into the
//! shell's, as any process of his may steer his sessions here already.
//! The read is jobs-broker's (../jobs-broker/src/peer.rs), which needs
//! the pidfd for more than this does.
//!
//! What Joel says or does to a session (main.rs's `AS_JOEL`) is held to
//! less: no agent may say it, as the node holds a steer that says it's
//! his ([`speaks_as_joel`]). The shell's own acts reach the node itself,
//! which takes them from anyone not an agent, so a stricter door here
//! would keep out only what walks in beside it (ISXEG8, 25 November).

use std::fs;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::Command;

/// The names quickshell's program goes by: Nix's wrapper runs the real
/// one under this name, and an unwrapped build runs as itself.
const SHELL_PROGRAMS: [&str; 2] = [".quickshell-wrapped", "quickshell"];

/// The shell's unit, whose main process alone may ask.
const SHELL_UNIT: &str = "quickshell.service";

/// What a refusal says, whichever check refused.
pub const REFUSED: &str = "only the launcher starts new work here";

/// `SO_PEERPIDFD` (Linux 6.5), the same on every architecture the
/// machines run, which the libc crate names only in its newer layout.
const SO_PEERPIDFD: libc::c_int = 77;

// The node's own names for who is an agent, as mycelium's
// src/node/asker.rs says them, which agents-tells-an-agent-as-the-node-does
// holds these to, so the two doors can't drift apart.

/// How the names of the units the machine runs its agents in begin, below
/// the person's service manager's own unit: pi's hosts, the scope each
/// held session runs in, and the queues agents' commands wait their turn
/// in, here or for a builder elsewhere. The rest of `agents.slice` is the
/// person's as much as theirs: their own terminals, the node and its
/// links run there.
const AGENTS_UNITS: [&str; 4] = [
    "agents.slice/pi-host-",
    "agents.slice/mycelium-pi-",
    "jobs.slice/",
    "offload.slice/",
];

/// The name pi's own process goes by.
const PI: &str = "pi";

/// The name a pi host's process goes by.
const PI_HOST: &str = "pi-host";

/// How many forebears are looked through at most, so a chain the kernel
/// tells wrong can't hold an ask up for long.
const FOREBEARS: usize = 64;

/// Why what an asker says can't be taken as Joel's.
#[derive(Debug, PartialEq, Eq)]
pub enum NotJoels {
    /// It runs as an agent: in this unit, or below this pi.
    Agent(String),
    /// Nothing could be told of it or of a forebear, for this reason.
    Untold(String),
}

/// How the daemon judges who says what Joel would: [`speaks_as_joel`],
/// or a stand-in for it in a test.
pub type Judge = fn(&UnixStream) -> Result<(), NotJoels>;

/// Whether what the process at the other end of `stream` says may be
/// taken as Joel's: refused from an agent, as its cgroup or a forebear
/// tells it, and from one nothing can be told of.
///
/// # Errors
///
/// Why: it is an agent's, or its cgroup or a forebear's name couldn't
/// be read.
pub fn speaks_as_joel(stream: &UnixStream) -> Result<(), NotJoels> {
    let untold = |why: io::Error| NotJoels::Untold(format!("who asked can't be told: {why}"));
    let (pidfd, pid) = asker_of(stream).map_err(untold)?;
    let cgroup = fs::read_to_string(format!("/proc/{pid}/cgroup")).ok();
    tells(cgroup.as_deref(), pid, |process| {
        let status = fs::read_to_string(format!("/proc/{process}/status")).ok()?;
        named(&status).map(|(name, parent)| (name.to_owned(), parent))
    })?;
    // Read last, so a pid taken again since any read above can't answer.
    match pidfd {
        Some(pidfd) if !alive(&pidfd) => Err(untold(gone())),
        _ => Ok(()),
    }
}

/// What `cgroup`, process `pid`'s as read, and `process`, a lookup of a
/// process's name and parent, tell of it, as the node tells them: an
/// agent's in an agent's unit or below pi, the person's once its
/// forebears end without one, and untold where either can't be read.
///
/// # Errors
///
/// Why it isn't taken as Joel's.
pub fn tells(
    cgroup: Option<&str>,
    pid: u32,
    process: impl Fn(u32) -> Option<(String, u32)>,
) -> Result<(), NotJoels> {
    let Some(cgroup) = cgroup else {
        return Err(NotJoels::Untold(format!(
            "process {pid}'s cgroup can't be read"
        )));
    };
    if let Some(unit) = agents_unit(cgroup) {
        return Err(NotJoels::Agent(unit));
    }
    let mut at = pid;
    for _ in 0..FOREBEARS {
        let Some((name, parent)) = process(at) else {
            return Err(NotJoels::Untold(format!(
                "process {at}, of {pid}'s forebears, can't be read"
            )));
        };
        if name == PI {
            return Err(NotJoels::Agent(format!("pi's process {at}")));
        }
        if name == PI_HOST {
            return Err(NotJoels::Agent(format!("pi's host {at}")));
        }
        if parent <= 1 {
            return Ok(());
        }
        at = parent;
    }
    Err(NotJoels::Untold(format!(
        "process {pid} has more than {FOREBEARS} forebears"
    )))
}

/// The agent's unit a process runs in, as its `cgroup` file tells it, by
/// the unit's own name, or none for any other.
fn agents_unit(cgroup: &str) -> Option<String> {
    cgroup.lines().find_map(|line| {
        // Control groups v2 tell one line, its hierarchy 0 and no
        // controllers named: `0::<path>`.
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
fn named(status: &str) -> Option<(&str, u32)> {
    let field = |key: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .map(str::trim)
    };
    Some((field("Name:")?, field("PPid:")?.parse().ok()?))
}

/// The process at the other end of `stream`: its pidfd and pid, or where
/// the kernel names no peer by pidfd (before Linux 6.5, as the lent
/// builders run), the pid it named at connect, which is all the node
/// ever reads of who asks it (mycelium's src/node/mod.rs, `peer_cred`).
fn asker_of(stream: &UnixStream) -> io::Result<(Option<OwnedFd>, u32)> {
    match pidfd_of(stream) {
        Ok((pidfd, pid)) => Ok((Some(pidfd), pid)),
        Err(error) if error.kind() == io::ErrorKind::Unsupported => {
            let mut cred = libc::ucred {
                pid: 0,
                uid: 0,
                gid: 0,
            };
            let mut length = libc::socklen_t::try_from(size_of::<libc::ucred>())
                .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
            // SAFETY: SO_PEERCRED writes one ucred into a struct of the
            // size `length` gives, and the socket is open for the call.
            let read = unsafe {
                libc::getsockopt(
                    stream.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_PEERCRED,
                    (&raw mut cred).cast(),
                    &raw mut length,
                )
            };
            if read != 0 {
                return Err(io::Error::last_os_error());
            }
            let pid = u32::try_from(cred.pid)
                .ok()
                .filter(|pid| *pid > 0)
                .ok_or_else(gone)?;
            Ok((None, pid))
        }
        Err(error) => Err(error),
    }
}

/// Whether the process at the other end of `stream` is the shell's, or
/// why it can't be taken for it, in words the shell says.
///
/// # Errors
///
/// Why: it isn't the shell's, it ended before its cgroup could be known
/// to be its own, or the kernel names no peer by pidfd.
pub fn the_shell(stream: &UnixStream) -> Result<(), String> {
    let untold = |why: io::Error| format!("who asked can't be told: {why}");
    let (pidfd, seen) = peer_of(stream).map_err(untold)?;
    // SAFETY: getuid has no failure and touches no memory.
    let uid = unsafe { libc::getuid() };
    if !takes(&seen, uid, shells_main_pid).map_err(untold)? {
        return Err(REFUSED.to_owned());
    }
    // Read last, so a pid taken again since any read above can't answer.
    if alive(&pidfd) {
        Ok(())
    } else {
        Err(untold(gone()))
    }
}

/// What was read of the process at the other end of a socket.
#[derive(Debug)]
pub struct Seen {
    /// Its pid, as its pidfd names it.
    pub pid: u32,
    /// Its `/proc/<pid>/cgroup`.
    pub cgroup: String,
    /// Where its `/proc/<pid>/exe` points.
    pub exe: String,
}

/// Whether `seen` is the shell under account `uid`: in its unit, running
/// its program, and the unit's main process, which `main_pid` asks the
/// manager for only once the rest holds, so nothing else costs a call.
///
/// # Errors
///
/// What `main_pid` gave when the manager couldn't be asked.
pub fn takes(
    seen: &Seen,
    uid: u32,
    main_pid: impl FnOnce() -> io::Result<u32>,
) -> io::Result<bool> {
    if !(of_the_shell(&seen.cgroup, uid) && the_shells_program(&seen.exe)) {
        return Ok(false);
    }
    // 0 when the unit has no main process, which no peer's pid is.
    Ok(main_pid()? == seen.pid)
}

/// The shell's unit's main process, as this account's manager says it.
fn shells_main_pid() -> io::Result<u32> {
    let said = Command::new("systemctl")
        .args([
            "--user",
            "show",
            "--property=MainPID",
            "--value",
            SHELL_UNIT,
        ])
        .output()?;
    if !said.status.success() {
        return Err(io::Error::other(format!(
            "systemd couldn't say the shell's main process: {}",
            String::from_utf8_lossy(&said.stderr).trim()
        )));
    }
    String::from_utf8_lossy(&said.stdout)
        .trim()
        .parse()
        .map_err(|_| io::Error::other("systemd named no main process for the shell"))
}

/// Whether `exe`, as `/proc/<pid>/exe` points, is quickshell's own
/// program.
pub fn the_shells_program(exe: &str) -> bool {
    Path::new(exe)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| SHELL_PROGRAMS.contains(&name))
}

/// Whether `cgroup`, as `/proc/<pid>/cgroup` says it, is the shell's own
/// unit under account `uid`: its unified line, that account's manager's
/// session slice and the shell's unit in it, never a unit inside it, nor
/// one merely named like it in another slice, as `systemd-run --user
/// --unit quickshell` could start while the shell isn't running.
pub fn of_the_shell(cgroup: &str, uid: u32) -> bool {
    let Some(path) = cgroup.lines().find_map(|line| line.strip_prefix("0::")) else {
        return false;
    };
    let steps: Vec<&str> = path.split('/').collect();
    match steps.as_slice() {
        [
            "",
            "user.slice",
            account,
            manager,
            "session.slice",
            "quickshell.service",
        ] => *account == format!("user-{uid}.slice") && *manager == format!("user@{uid}.service"),
        _ => false,
    }
}

/// The pidfd of the process at the other end of `stream`, and its pid,
/// cgroup and program, read through that pid: `alive` on the pidfd after
/// they're judged says whether they were its own.
fn peer_of(stream: &UnixStream) -> io::Result<(OwnedFd, Seen)> {
    let (pidfd, pid) = pidfd_of(stream)?;
    let cgroup = fs::read_to_string(format!("/proc/{pid}/cgroup"))?;
    let exe = fs::read_link(format!("/proc/{pid}/exe"))?
        .to_string_lossy()
        .into_owned();
    Ok((pidfd, Seen { pid, cgroup, exe }))
}

/// The pidfd of the process at the other end of `stream`, and its pid.
fn pidfd_of(stream: &UnixStream) -> io::Result<(OwnedFd, u32)> {
    let mut fd: libc::c_int = -1;
    let mut length = libc::socklen_t::try_from(size_of::<libc::c_int>())
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    // SAFETY: SO_PEERPIDFD writes one descriptor into an int of the size
    // `length` gives, and the socket is open for the call.
    let read = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            SO_PEERPIDFD,
            (&raw mut fd).cast(),
            &raw mut length,
        )
    };
    if read != 0 || fd < 0 {
        let error = io::Error::last_os_error();
        // A pid alone could be another process's by the time it's read,
        // so a kernel before 6.5 is refused rather than trusted.
        return Err(if error.raw_os_error() == Some(libc::ENOPROTOOPT) {
            io::Error::new(
                io::ErrorKind::Unsupported,
                "this kernel names no peer by pidfd (Linux 6.5 and later do)",
            )
        } else {
            error
        });
    }
    // SAFETY: the kernel just handed this descriptor to this process, and
    // nothing else holds it.
    let pidfd = unsafe { OwnedFd::from_raw_fd(fd) };
    let info = fs::read_to_string(format!("/proc/self/fdinfo/{}", pidfd.as_raw_fd()))?;
    // `-1` once the process has ended, `0` for one in no namespace of
    // this daemon's.
    let pid = info
        .lines()
        .find_map(|line| line.strip_prefix("Pid:"))
        .and_then(|pid| pid.trim().parse::<u32>().ok())
        .filter(|pid| *pid > 0)
        .ok_or_else(gone)?;
    Ok((pidfd, pid))
}

/// Whether the process a pidfd stands for still runs: its pidfd reads
/// as ready once it has ended.
fn alive(pidfd: &OwnedFd) -> bool {
    let mut asked = libc::pollfd {
        fd: pidfd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: one pollfd, for a descriptor this process holds open.
    let ready = unsafe { libc::poll(&raw mut asked, 1, 0) };
    ready == 0
}

fn gone() -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, "the process asking ended")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SESSION: &str = "0::/user.slice/user-1000.slice/user@1000.service/session.slice";

    #[test]
    fn only_the_shells_own_unit_is_the_shell() {
        assert!(
            of_the_shell(&format!("{SESSION}/quickshell.service\n"), 1000),
            "the shell's unit"
        );
        assert!(
            !of_the_shell(&format!("{SESSION}/quickshell.service\n"), 1001),
            "the shell's unit under another account"
        );
        for other in [
            "0::/user.slice/user-1000.slice/user@1000.service/app.slice/pi-host.service\n"
                .to_owned(),
            format!("{SESSION}/quickshell.service/fenced.scope\n"),
            format!("{SESSION}/not-quickshell.service\n"),
            format!("{SESSION}/quickshell.service.d\n"),
            "0::/user.slice/user-1000.slice/user@1000.service/app.slice/quickshell.service\n"
                .to_owned(),
            "0::/user.slice/user-1000.slice/user@1001.service/session.slice/quickshell.service\n"
                .to_owned(),
            "0::/system.slice/session.slice/quickshell.service\n".to_owned(),
            "0::/x/user.slice/user-1000.slice/user@1000.service/session.slice/quickshell.service\n"
                .to_owned(),
            "0::/user.slice/user-someone.slice/user@someone.service/session.slice/quickshell.service\n"
                .to_owned(),
            "0::/user.slice/user-.slice/user@.service/session.slice/quickshell.service\n".to_owned(),
            "0::/\n".to_owned(),
            String::new(),
            "1:name=systemd:/quickshell.service\n".to_owned(),
        ] {
            assert!(!of_the_shell(&other, 1000), "{other:?} isn't the shell");
        }
    }

    #[test]
    fn only_quickshells_own_program_is_the_shell() {
        for shell in [
            "/nix/store/2mr2gyg6-quickshell-0.3.2/bin/.quickshell-wrapped",
            "/nix/store/2mr2gyg6-quickshell-0.3.2/bin/quickshell",
        ] {
            assert!(the_shells_program(shell), "{shell} is the shell's");
        }
        // What runs beside it in its unit: pi-ask's pi and bash, its helpers.
        for other in [
            "/nix/store/giny1jia-nodejs-22.23.5/bin/node",
            "/nix/store/hasnvc0j-bash-5.3p20/bin/bash",
            "/run/current-system/sw/bin/litra",
            "/nix/store/x/bin/quickshell-helper",
            "",
        ] {
            assert!(!the_shells_program(other), "{other:?} isn't the shell's");
        }
    }

    fn in_the_shells_unit(pid: u32, exe: &str) -> Seen {
        Seen {
            pid,
            cgroup: format!("{SESSION}/quickshell.service\n"),
            exe: exe.to_owned(),
        }
    }

    const WRAPPED: &str = "/nix/store/2mr2gyg6-quickshell-0.3.2/bin/.quickshell-wrapped";

    #[test]
    fn only_the_shells_main_process_is_the_shell() {
        let shell = in_the_shells_unit(4_106_561, WRAPPED);
        assert!(
            takes(&shell, 1000, || Ok(4_106_561)).expect("asked"),
            "the unit's main process"
        );
        // A helper in the unit, or its child or orphan, running a program
        // named quickshell: the store's own with its own QML, or a copy.
        assert!(
            !takes(&in_the_shells_unit(4_200_000, WRAPPED), 1000, || Ok(
                4_106_561
            ))
            .expect("asked"),
            "a process beside the main one isn't the shell"
        );
        assert!(
            !takes(&shell, 1000, || Ok(0)).expect("asked"),
            "no main process, as while the shell restarts, is no shell"
        );
        assert!(
            takes(&shell, 1000, || Err(io::Error::other("no bus"))).is_err(),
            "a manager that can't be asked refuses"
        );
    }

    #[test]
    fn the_manager_is_asked_only_once_the_rest_holds() {
        let asked = || -> io::Result<u32> { panic!("the manager was asked") };
        let node = in_the_shells_unit(4_106_561, "/nix/store/giny1jia-nodejs-22.23.5/bin/node");
        assert!(
            !takes(&node, 1000, asked).expect("not asked"),
            "pi-ask's pi"
        );
        let elsewhere = Seen {
            cgroup: format!("{SESSION}/not-quickshell.service\n"),
            ..in_the_shells_unit(4_106_561, WRAPPED)
        };
        assert!(
            !takes(&elsewhere, 1000, asked).expect("not asked"),
            "another unit"
        );
    }

    const MANAGER: &str = "0::/user.slice/user-1000.slice/user@1000.service";

    /// Processes by their ids, each with its name and parent's, as a
    /// lookup of them.
    fn lineage<'a>(chain: &'a [(u32, &'a str, u32)]) -> impl Fn(u32) -> Option<(String, u32)> + 'a {
        move |pid| {
            chain
                .iter()
                .find(|(at, _, _)| *at == pid)
                .map(|(_, name, parent)| ((*name).to_owned(), *parent))
        }
    }

    /// A command in one of Joel's own terminals.
    const TERMINAL: [(u32, &str, u32); 3] =
        [(50, "mycelium", 49), (49, "bash", 48), (48, "kitty", 1)];

    #[test]
    fn an_asker_in_an_agents_unit_is_an_agent() {
        for (unit, named) in [
            (
                "agents.slice/pi-host-work-background.service",
                "pi-host-work-background.service",
            ),
            (
                "agents.slice/mycelium-pi-f3d9.scope",
                "mycelium-pi-f3d9.scope",
            ),
            ("jobs.slice/run-r1.scope", "run-r1.scope"),
            ("offload.slice/nix-offload-3.scope", "nix-offload-3.scope"),
        ] {
            let cgroup = format!("{MANAGER}/{unit}\n");
            assert_eq!(
                tells(Some(&cgroup), 50, lineage(&TERMINAL)),
                Err(NotJoels::Agent(named.to_owned())),
                "{unit}, though no pi is among its forebears"
            );
        }
    }

    #[test]
    fn an_asker_below_pi_is_an_agent() {
        let terminal = format!("{MANAGER}/agents.slice/kitty-2830-4.scope\n");
        let below_pi = [
            (50, "mycelium", 49),
            (49, "bash", 48),
            (48, "pi", 47),
            (47, "kitty", 1),
        ];
        assert_eq!(
            tells(Some(&terminal), 50, lineage(&below_pi)),
            Err(NotJoels::Agent("pi's process 48".to_owned())),
            "a command a session in a terminal runs"
        );
        let below_host = [(50, "bash", 49), (49, "pi-host", 1)];
        assert_eq!(
            tells(Some(&terminal), 50, lineage(&below_host)),
            Err(NotJoels::Agent("pi's host 49".to_owned())),
            "a command a host runs"
        );
    }

    #[test]
    fn a_plain_asker_is_joels() {
        for cgroup in [
            format!("{MANAGER}/agents.slice/kitty-2830-4.scope\n"),
            format!("{SESSION}/quickshell.service\n"),
            "0::/system.slice/nix-daemon.service\n".to_owned(),
            "0::/\n".to_owned(),
        ] {
            assert_eq!(
                tells(Some(&cgroup), 50, lineage(&TERMINAL)),
                Ok(()),
                "{cgroup:?}"
            );
        }
    }

    #[test]
    fn an_asker_nothing_can_be_told_of_is_refused() {
        let untold = |told: Result<(), NotJoels>| matches!(told, Err(NotJoels::Untold(_)));
        assert!(
            untold(tells(None, 50, lineage(&TERMINAL))),
            "its cgroup can't be read"
        );
        let shell = format!("{SESSION}/quickshell.service\n");
        assert!(
            untold(tells(Some(&shell), 50, lineage(&[(50, "bash", 49)]))),
            "a forebear can't be read"
        );
        assert!(
            untold(tells(
                Some(&shell),
                50,
                lineage(&[(50, "bash", 49), (49, "bash", 50)])
            )),
            "its forebears never end"
        );
    }

    #[test]
    fn this_test_can_be_told_of() {
        let (ours, _theirs) = UnixStream::pair().expect("a socket pair");
        // Joel's or an agent's, as whoever runs the tests is, but told,
        // by pidfd or by the pid the kernel named at connect.
        let told = speaks_as_joel(&ours);
        assert!(
            !matches!(told, Err(NotJoels::Untold(_))),
            "who asked is told: {told:?}"
        );
    }

    #[test]
    fn a_process_not_the_shells_is_refused() {
        let (ours, _theirs) = UnixStream::pair().expect("a socket pair");
        let refused = the_shell(&ours).expect_err("this test isn't the shell");
        // The lent builders run Linux 6.1, where who asked can't be told;
        // there that refusal is what's held to.
        assert!(
            refused == REFUSED || refused.contains("names no peer by pidfd"),
            "it says why: {refused}"
        );
    }
}
