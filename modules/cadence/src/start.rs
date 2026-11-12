//! Starting a died session again: what reviving reads (presences, quest
//! READMEs, processes, niri's windows) and what it does (a new terminal
//! on the session's own log, its window put back, a line to the orchestrator).
//! The decisions are revive.rs's.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use jiff::tz::TimeZone;
use serde::Deserialize;

use crate::mesh::{Mesh, Presence};
use crate::revive::{self, Gate, Held, Holder, Memory, Seen, Verdict};

/// What reviving is told: whom to watch and tell, and how a session is
/// started, as the words a login shell runs, before `--session <id>`.
pub struct Reviving {
    pub orchestrator: String,
    pub watcher: Option<String>,
    pub orchestrators: Vec<String>,
    pub skip: Vec<String>,
    pub restart: String,
    pub quests: PathBuf,
}

/// How long a started session has to say it is there.
const COMING_UP: u32 = 120;

/// Looks once, starting every watched session that died, one at a time,
/// and says what it did.
///
/// # Errors
///
/// When its memory can't be written, or a line can't be sent.
pub fn revive(
    reviving: &Reviving,
    mesh: &Mesh,
    folder: &Path,
    now: i64,
    send: &dyn Fn(&str, &str, String) -> io::Result<()>,
) -> io::Result<Vec<String>> {
    if !revive::settled(signed_in(), now) {
        return Ok(Vec::new());
    }
    let path = folder.join("revive.json");
    let mut memory: Memory = fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let seen = seen(mesh);
    let windows = windows();
    let node = node_holders(revive::realm_of(&reviving.restart));
    let mut said = Vec::new();
    let mut told = Vec::new();
    for s in revive::watched(
        &seen,
        &reviving.orchestrators,
        &reviving.orchestrator,
        &reviving.skip,
    ) {
        if s.running
            && let Some(ws) = windows.iter().find(|w| w.title == s.title).map(|w| w.ws)
        {
            memory.homes.insert(s.session.clone(), ws);
        }
        let quest = s
            .quest
            .as_ref()
            .and_then(|q| fs::read_to_string(reviving.quests.join(q).join("README.md")).ok())
            .map(|readme| revive::quest(&readme));
        let hold = node.as_deref().map_or_else(
            || {
                if held(&s.session) {
                    Held::Running
                } else {
                    Held::Not
                }
            },
            |all| revive::held(all, &s.session, lives),
        );
        let named = matches!(hold, Held::Running | Held::Parked);
        let Verdict::Died { cwd } = revive::judge(s, quest.as_ref(), is_pi, named) else {
            continue;
        };
        let line = match revive::gate(&memory, &s.session, now) {
            Gate::Go => {
                revive::started(&mut memory, &s.session, now);
                fs::write(&path, serde_json::to_vec_pretty(&memory)?)?;
                let closed = close_dead(&s.session, &hold, node.as_deref().unwrap_or_default());
                let mut line = start(
                    reviving,
                    mesh,
                    s,
                    &cwd,
                    memory.homes.get(&s.session).copied(),
                );
                if closed > 0 {
                    line = format!("{line} It closed {closed} window(s) left showing it dead.");
                }
                told.extend(tell(reviving, s, &line));
                line
            }
            Gate::TooSoon(_) => continue,
            Gate::Defect { starts, report } => {
                let line = format!(
                    "{} died again after {starts} starts this hour, so it is left down: \
                     something kills it as it starts.",
                    s.address
                );
                if report {
                    memory.defects.insert(s.session.clone(), now / 3600);
                    send(&reviving.orchestrator, "finding", line.clone())?;
                }
                line
            }
        };
        said.push(line);
    }
    fs::write(&path, serde_json::to_vec_pretty(&memory)?)?;
    for (to, text) in revive::gathered(&told) {
        let text = if reviving.watcher.as_ref() == Some(&to) {
            format!("Revived: {text}")
        } else {
            text
        };
        send(&to, "fyi", text)?;
    }
    Ok(said)
}

/// Whom `line`, of `s` started again, is for: the orchestrator, the session's
/// own orchestrator, and the watcher, each once, and never the session.
fn tell(reviving: &Reviving, s: &Seen, line: &str) -> Vec<(String, String)> {
    let watcher = reviving.watcher.as_ref().filter(|&w| w != &s.address);
    let mut told: Vec<(String, String)> = Vec::new();
    for to in [
        Some(&reviving.orchestrator),
        s.orchestrated_by.as_ref(),
        watcher,
    ]
    .into_iter()
    .flatten()
    {
        if !told.iter().any(|(peer, _)| peer == to) {
            told.push((to.clone(), line.to_owned()));
        }
    }
    told
}

/// Starts the session in a terminal of its own, held as the restart
/// words say, waits for it to say it is there, and puts its window back
/// on the workspace it was last seen on.
fn start(reviving: &Reviving, mesh: &Mesh, s: &Seen, cwd: &str, home: Option<u32>) -> String {
    let words = format!("exec {} --session {}", reviving.restart, s.session);
    let opened = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis());
    let started = terminal(cwd, &words, &format!("pi-spawned.{opened}")).status();
    if !started.is_ok_and(|s| s.success()) {
        return format!("{} died, and starting it again failed.", s.address);
    }
    let old = s.at.clone();
    let mut back = None;
    for _ in 0..COMING_UP {
        back = seen(mesh)
            .into_iter()
            .find(|n| n.session == s.session && n.running && n.at != old);
        if back.is_some() {
            break;
        }
        sleep(Duration::from_secs(1));
    }
    let Some(back) = back else {
        return format!(
            "{} died; started again, but it didn't say it was there within 2 min.",
            s.address
        );
    };
    let mut place = String::from("its window not found");
    for _ in 0..30 {
        if let Some(w) = windows().into_iter().find(|w| w.title == back.title) {
            let ws = home.unwrap_or(w.ws);
            if ws != w.ws {
                let _ = Command::new("niri")
                    .args(["msg", "action", "move-window-to-workspace", "--window-id"])
                    .args([&w.id.to_string(), "--focus", "false", &ws.to_string()])
                    .status();
            }
            place = format!("its window on workspace {ws}");
            break;
        }
        sleep(Duration::from_secs(1));
    }
    format!("{} died; started again on its own log, {place}.", s.address)
}

/// The command that opens a revived session's terminal, running `words`
/// in `cwd`, as a window of the sessions' own kitty with the app id
/// `class`. Its own unit under the agents' slice, and a class niri opens
/// unfocused, so nothing typed lands in it. Its process is told the same
/// class in `MYCELIUM_APP_ID`, as Sessions.qml's inSessions tells its
/// own: one kitty draws every session's window, so the jobs-sorter and
/// the swap-in can find a window's scope only by the class its pi says.
/// On 11 November 48 of 58 pi windows had been opened as the bare
/// pi-spawned with no such word, so neither helped them. `class` takes a
/// number, the time it opened in milliseconds, so no two windows share
/// one; Sessions.qml counts on from the time its shell started.
fn terminal(cwd: &str, words: &str, class: &str) -> Command {
    let mut command = Command::new("systemd-run");
    command
        .args([
            "--user",
            "--slice=agents.slice",
            "--collect",
            "--quiet",
            "--",
        ])
        .args(["env", &format!("MYCELIUM_APP_ID={class}")])
        .args(["kitty", "-1", "--instance-group", "pi-sessions"])
        .args(["--class", class, "--directory", cwd])
        .args(["zsh", "-lic", words]);
    command
}

/// Every session's presence, with whether its process runs.
fn seen(mesh: &Mesh) -> Vec<Seen> {
    // A boot that can't be read leaves every session ended with one, so
    // nothing is reopened that Joel didn't ask for.
    let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .ok()
        .map(|id| id.trim().to_owned());
    let zone = TimeZone::system();
    let records = mesh.root.join("quest-workflow/sessions");
    mesh.all()
        .into_iter()
        .filter_map(|p: Presence| {
            let address = p.addresses.first()?.clone();
            let orchestrates = p
                .groups
                .iter()
                .any(|g| g.eq_ignore_ascii_case(&format!("#orchestration/{address}")));
            let ended = fs::read_to_string(records.join(format!("{}.json", p.session)))
                .ok()
                .and_then(|record| revive::ended_on_purpose(&record, &zone));
            Some(Seen {
                orchestrates,
                ended,
                this_boot: boot.is_some()
                    && p.owner.as_ref().and_then(|o| o.boot_id.as_ref()) == boot.as_ref(),
                running: p.runs(is_pi, is_host),
                parked: p.state == "parked" || mesh.is_parked(&p.session),
                address,
                orchestrated_by: p.orchestrated_by,
                quest: p.quest,
                title: p.title,
                at: p.at.to_string(),
                session: p.session,
            })
        })
        .collect()
}

/// Whether `pid` is a pi, rather than gone or reused by another program.
pub fn is_pi(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/comm")).is_ok_and(|c| c.trim() == "pi")
}

/// Whether `pid` is a shared pi host: node running the harness's
/// `scripts/pi-host.ts`, rather than gone or reused by another program.
pub fn is_host(pid: u32) -> bool {
    fs::read(format!("/proc/{pid}/cmdline")).is_ok_and(|line| {
        line.split(|&b| b == 0)
            .any(|arg| arg.ends_with(b"/scripts/pi-host.ts"))
    })
}

/// What the machine's node holds, by `mycelium held`, or none when it
/// doesn't answer.
fn node_holders(realm: Option<&str>) -> Option<Vec<Holder>> {
    let mut ask = Command::new("mycelium");
    if let Some(realm) = realm {
        ask.args(["--realm", realm]);
    }
    let out = ask
        .arg("held")
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    Some(revive::holders(&String::from_utf8_lossy(&out.stdout)))
}

/// Closes every kitty window left showing `session` dead before it is
/// started again, so a restart replaces its window rather than adding one
/// beside it, as the harness does when it reopens a session: the windows
/// of its holders whose pi is gone, and those the harness marked as the
/// session's with no pi left under them. Says how many it closed.
fn close_dead(session: &str, hold: &Held, holders: &[Holder]) -> usize {
    let kitty = |w: &&revive::HeldWindow| w.terminal == "kitty";
    let mut dead: Vec<(String, String)> = match hold {
        Held::Dead(windows) => windows
            .iter()
            .filter(kitty)
            .map(|w| (w.within.clone(), w.id.clone()))
            .collect(),
        _ => Vec::new(),
    };
    let sockets: std::collections::BTreeSet<&str> = holders
        .iter()
        .flat_map(|h| &h.windows)
        .filter(kitty)
        .map(|w| w.within.as_str())
        .collect();
    for within in sockets {
        let Some(listing) = Command::new("kitty")
            .args(["@", "--to", within, "ls"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| serde_json::from_slice(&o.stdout).ok())
        else {
            continue;
        };
        for (id, pid) in revive::marked(&listing, session) {
            if !pid.is_some_and(pi_under) {
                dead.push((within.to_owned(), id));
            }
        }
    }
    dead.sort();
    dead.dedup();
    dead.iter()
        .filter(|(within, id)| {
            Command::new("kitty")
                .args(["@", "--to", within, "close-window", "--match"])
                .arg(format!("id:{id}"))
                .status()
                .is_ok_and(|s| s.success())
        })
        .count()
}

/// Whether `pid` shows a session: a pi, or a viewer of one a host runs.
/// A window handed into a host keeps its session's mark, and its holder
/// runs the viewer, not pi, so counting only a pi would close the window
/// Joel reads a hosted session in whenever its host restarts.
fn lives(pid: u32) -> bool {
    is_pi(pid) || fs::read(format!("/proc/{pid}/cmdline")).is_ok_and(|line| views(&line))
}

/// Whether a command line, its words split by NUL, is a hosted session's
/// viewer: node running the harness's `scripts/host-join.ts`, or
/// `mycelium attach`.
fn views(line: &[u8]) -> bool {
    let words: Vec<&[u8]> = line.split(|&b| b == 0).collect();
    let mycelium = words
        .first()
        .is_some_and(|program| program.rsplit(|&b| b == b'/').next() == Some(b"mycelium"));
    words
        .iter()
        .any(|word| word.ends_with(b"/scripts/host-join.ts"))
        || (mycelium && words.contains(&b"attach".as_slice()))
}

/// Whether a pi, or a hosted session's viewer, runs at or below `pid`.
fn pi_under(pid: u32) -> bool {
    let mut queue = vec![pid];
    let mut seen = std::collections::BTreeSet::new();
    while let Some(at) = queue.pop() {
        if !seen.insert(at) {
            continue;
        }
        if lives(at) {
            return true;
        }
        let Ok(tasks) = fs::read_dir(format!("/proc/{at}/task")) else {
            continue;
        };
        for task in tasks.filter_map(Result::ok) {
            let children = fs::read_to_string(task.path().join("children")).unwrap_or_default();
            queue.extend(
                children
                    .split_whitespace()
                    .filter_map(|c| c.parse::<u32>().ok()),
            );
        }
    }
    false
}

/// Whether a process other than a pi holds `session`, naming it after
/// `--session` on its command line, as a holder does. A holder whose pi
/// parked has no pi under it, and waits for a key in its own window to
/// start it, so a second one started elsewhere would write the same log.
fn held(session: &str) -> bool {
    let Ok(procs) = fs::read_dir("/proc") else {
        return false;
    };
    procs
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().to_str()?.parse::<u32>().ok())
        .filter(|&pid| !is_pi(pid))
        .filter_map(|pid| fs::read(format!("/proc/{pid}/cmdline")).ok())
        .any(|line| {
            let words: Vec<&[u8]> = line.split(|&b| b == 0).collect();
            words.windows(2).any(|pair| {
                matches!(pair[0], b"--session" | b"--session-id")
                    && String::from_utf8_lossy(pair[1]).contains(session)
            })
        })
}

/// When the graphical session began, if it has: until then there is no
/// desktop to open a window on.
fn signed_in() -> Option<i64> {
    let out = Command::new("systemctl")
        .args([
            "--user",
            "show",
            "--timestamp=unix",
            "graphical-session.target",
        ])
        .args(["-p", "ActiveState", "-p", "ActiveEnterTimestamp"])
        .output()
        .ok()?;
    let shown = String::from_utf8_lossy(&out.stdout).into_owned();
    if !shown.lines().any(|l| l == "ActiveState=active") {
        return None;
    }
    shown
        .lines()
        .find_map(|l| l.strip_prefix("ActiveEnterTimestamp=@"))?
        .trim()
        .parse()
        .ok()
}

struct Window {
    id: u64,
    title: String,
    ws: u32,
}

#[derive(Deserialize)]
struct NiriWindow {
    id: u64,
    #[serde(default)]
    title: Option<String>,
    workspace_id: Option<u64>,
}

#[derive(Deserialize)]
struct NiriWorkspace {
    id: u64,
    idx: u32,
}

/// niri's windows, with the index of the workspace each is on, which is
/// what moving a window takes.
fn windows() -> Vec<Window> {
    let ask = |what: &str| {
        Command::new("niri")
            .args(["msg", "-j", what])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| o.stdout)
    };
    let workspaces: Vec<NiriWorkspace> = ask("workspaces")
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let windows: Vec<NiriWindow> = ask("windows")
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    windows
        .into_iter()
        .filter_map(|w| {
            let ws = workspaces.iter().find(|s| Some(s.id) == w.workspace_id)?;
            Some(Window {
                id: w.id,
                title: w.title?,
                ws: ws.idx,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hosted_sessions_viewer_counts_as_showing_it() {
        for viewer in [
            b"node\0/h/agentic-harness.pi/scripts/host-join.ts\0--session\0s\0".as_slice(),
            b"/nix/store/x-mycelium/bin/mycelium\0attach\0s\0",
        ] {
            assert!(
                views(viewer),
                "a viewer counted as dead: {}",
                String::from_utf8_lossy(viewer)
            );
        }
        for other in [
            b"node\0/h/agentic-harness.pi/scripts/pi-host.ts\0".as_slice(),
            b"zsh\0-c\0mycelium attach s\0",
            b"/bin/mycelium\0held\0",
        ] {
            assert!(
                !views(other),
                "not a viewer, yet counted as one: {}",
                String::from_utf8_lossy(other)
            );
        }
    }

    #[test]
    fn a_revived_window_and_its_pi_say_the_same_numbered_class() {
        let command = terminal("/home/j", "exec pi --session s", "pi-spawned.42");
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        let class = args.iter().position(|arg| arg == "--class");
        assert_eq!(
            class.and_then(|at| args.get(at + 1)).map(String::as_str),
            Some("pi-spawned.42"),
            "the window isn't opened with its own class: {args:?}"
        );
        let env = args.iter().position(|arg| arg == "env");
        let kitty = args.iter().position(|arg| arg == "kitty");
        assert!(
            env.is_some_and(|at| args.get(at + 1).map(String::as_str)
                == Some("MYCELIUM_APP_ID=pi-spawned.42")
                && kitty == Some(at + 2)),
            "its kitty isn't told the class in MYCELIUM_APP_ID: {args:?}"
        );
    }
}
