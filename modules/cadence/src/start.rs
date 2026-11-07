//! Starting a died session again: what reviving reads (presences, quest
//! READMEs, processes, niri's windows) and what it does (a new terminal
//! on the session's own log, its window put back, a line to the steward).
//! The decisions are revive.rs's.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use serde::Deserialize;

use crate::mesh::{Mesh, Presence};
use crate::revive::{self, Gate, Memory, Seen, Verdict};

/// What reviving is told: whom to watch and tell, and how a session is
/// started, as the words a login shell runs, before `--session <id>`.
pub struct Reviving {
    pub steward: String,
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
    let path = folder.join("revive.json");
    let mut memory: Memory = fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let seen = seen(mesh);
    let windows = windows();
    let mut said = Vec::new();
    for s in revive::watched(
        &seen,
        &reviving.orchestrators,
        &reviving.steward,
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
        let Verdict::Died { cwd } = revive::judge(s, quest.as_ref(), is_pi, named(&s.session))
        else {
            continue;
        };
        let line = match revive::gate(&memory, &s.session, now) {
            Gate::Go => {
                revive::started(&mut memory, &s.session, now);
                fs::write(&path, serde_json::to_vec_pretty(&memory)?)?;
                let line = start(
                    reviving,
                    mesh,
                    s,
                    &cwd,
                    memory.homes.get(&s.session).copied(),
                );
                tell(reviving, s, &line, send)?;
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
                    send(&reviving.steward, "finding", line.clone())?;
                }
                line
            }
        };
        said.push(line);
    }
    fs::write(&path, serde_json::to_vec_pretty(&memory)?)?;
    Ok(said)
}

fn tell(
    reviving: &Reviving,
    s: &Seen,
    line: &str,
    send: &dyn Fn(&str, &str, String) -> io::Result<()>,
) -> io::Result<()> {
    send(&reviving.steward, "fyi", line.to_owned())?;
    if let Some(by) = &s.orchestrated_by
        && by != &reviving.steward
    {
        send(by, "fyi", line.to_owned())?;
    }
    if let Some(watcher) = &reviving.watcher
        && watcher != &s.address
    {
        send(watcher, "fyi", format!("Revived: {line}"))?;
    }
    Ok(())
}

/// Starts the session in a terminal of its own, held as the restart
/// words say, waits for it to say it is there, and puts its window back
/// on the workspace it was last seen on.
fn start(reviving: &Reviving, mesh: &Mesh, s: &Seen, cwd: &str, home: Option<u32>) -> String {
    let words = format!("exec {} --session {}", reviving.restart, s.session);
    // Its own unit under the agents' slice, and a terminal class niri
    // opens unfocused, so nothing typed lands in it.
    let started = Command::new("systemd-run")
        .args([
            "--user",
            "--slice=agents.slice",
            "--collect",
            "--quiet",
            "--",
        ])
        .args(["kitty", "-1", "--instance-group", "pi-sessions"])
        .args(["--class", "pi-spawned", "--directory", cwd])
        .args(["zsh", "-lic", &words])
        .status();
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

/// Every session's presence, with whether its process runs.
fn seen(mesh: &Mesh) -> Vec<Seen> {
    mesh.all()
        .into_iter()
        .filter_map(|p: Presence| {
            let address = p.addresses.first()?.clone();
            Some(Seen {
                running: p.owner.as_ref().and_then(|o| o.pid).is_some_and(is_pi),
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
fn is_pi(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/comm")).is_ok_and(|c| c.trim() == "pi")
}

/// Whether a pi runs whose parent names `session` on its command line,
/// as a holder does, so one started by hand isn't started twice.
fn named(session: &str) -> bool {
    let Ok(procs) = fs::read_dir("/proc") else {
        return false;
    };
    procs
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().to_str()?.parse::<u32>().ok())
        .filter(|&pid| is_pi(pid))
        .any(|pid| {
            parent(pid)
                .and_then(|pp| fs::read(format!("/proc/{pp}/cmdline")).ok())
                .is_some_and(|c| String::from_utf8_lossy(&c).contains(session))
        })
}

fn parent(pid: u32) -> Option<u32> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    stat.rsplit_once(')')?
        .1
        .split_whitespace()
        .nth(1)?
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
