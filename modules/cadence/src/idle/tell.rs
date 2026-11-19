//! Reading the sessions for idle.rs and sending what it decides.

use std::fs;
use std::io;
use std::path::Path;

use serde::Deserialize;

use super::{Quiet, Told, due, unsay};
use crate::asking::To;
use crate::mesh::{self, Mesh, Presence};
use crate::{clock, message, start, write_whole};

/// What the harness keeps of a session's running background work: the
/// pi that runs it, by its pid and start in seconds, and the jobs.
#[derive(Deserialize)]
struct Running {
    owner: u32,
    started: i64,
    #[serde(default)]
    jobs: Vec<serde_json::Value>,
}

/// Whether `p` runs background work of its own now, as the harness's
/// `job-workflow/running/<session>.json` says. A file the pi it names no
/// longer runs is left from a pi before, so only one naming the
/// presence's own pi and start, to the second either way, counts. A
/// session id is read only when it is plain letters, digits and hyphens,
/// since it names the file.
fn working(root: &Path, p: &Presence) -> bool {
    let Some(owner) = p.owner.as_ref() else {
        return false;
    };
    let plain = |c: char| c.is_ascii_alphanumeric() || c == '-';
    if p.session.is_empty() || !p.session.chars().all(plain) {
        return false;
    }
    let path = root
        .join("job-workflow/running")
        .join(format!("{}.json", p.session));
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Running>(&bytes).ok())
        .is_some_and(|r| {
            !r.jobs.is_empty()
                && Some(r.owner) == owner.pid
                && owner
                    .started_at
                    .is_some_and(|ms| (r.started - ms / 1000).abs() <= 1)
        })
}

/// Wakes each session idle with a next step, or naming none on a quest,
/// and tells each orchestrator
/// of its sessions idle two hours, remembering in `idle.json` what was
/// said.
pub fn tell_idle(mesh: &Mesh, logs: &Path, folder: &Path, now: i64) -> io::Result<()> {
    let path = folder.join("idle.json");
    let mut told: Told = fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .ok()
        .map(|id| id.trim().to_owned());
    let logged = mesh::logs_written(logs);
    let sessions: Vec<Quiet> = mesh
        .all()
        .iter()
        .filter_map(|p| {
            Quiet::from(
                p,
                logged.get(&p.session).copied(),
                boot.as_deref(),
                (mesh.is_parked(&p.session), working(&mesh.root, p)),
                |p| p.runs(start::is_pi, start::is_host),
            )
        })
        .collect();
    let tells = due(&sessions, &mut told, now, clock);
    let mut failed = None;
    for tell in &tells {
        if failed.is_some() {
            unsay(&mut told, tell);
            continue;
        }
        // A wake is a question, since only a question starts a listening
        // session's turn: a finding waits for the turn it would start. A
        // line for an orchestrator is told as it is.
        let intent = if tell.wakes {
            "question"
        } else {
            To::Orchestrator.intent()
        };
        match mesh.send(&message(&tell.to, intent, tell.text.clone())) {
            Ok(()) => println!("{} (to {})", tell.text, tell.to),
            Err(err) => {
                unsay(&mut told, tell);
                failed = Some(err);
            }
        }
    }
    write_whole(&path, &serde_json::to_string_pretty(&told)?)?;
    failed.map_or(Ok(()), Err)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn presence(pid: u32, started_ms: i64) -> Presence {
        serde_json::from_value(serde_json::json!({
            "session": "s-1",
            "owner": {"pid": pid, "startedAt": started_ms, "bootId": "b"},
        }))
        .expect("a presence")
    }

    #[test]
    fn only_background_work_of_the_running_pi_counts() {
        let root = tempfile::tempdir().expect("a folder");
        let running = root.path().join("job-workflow/running");
        fs::create_dir_all(&running).expect("its folder");
        assert!(!working(root.path(), &presence(7, 1_000_000)), "no file");
        let file = running.join("s-1.json");
        fs::write(&file, r#"{"owner":7,"started":1000,"jobs":[{"id":"j1"}]}"#).expect("written");
        assert!(
            working(root.path(), &presence(7, 1_000_090)),
            "its own pi's job"
        );
        assert!(
            !working(root.path(), &presence(8, 1_000_090)),
            "another pi's"
        );
        assert!(
            !working(root.path(), &presence(7, 2_000_000)),
            "a pi before this one"
        );
        assert!(
            working(root.path(), &presence(7, 1_001_000)),
            "a start rounded the other way"
        );
        let mut odd = presence(7, 1_000_090);
        odd.session = "../s-1".into();
        assert!(!working(root.path(), &odd), "a session id that isn't plain");
        fs::write(&file, r#"{"owner":7,"started":1000,"jobs":[]}"#).expect("written");
        assert!(
            !working(root.path(), &presence(7, 1_000_090)),
            "no jobs left"
        );
    }
}
