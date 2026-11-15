//! A window an agent opened over Joel's work, said to the journal.
//!
//! Agents run in agents.slice and the jobs slices, apart from Joel's
//! programs in app.slice, so a window whose process sits there was opened
//! by an agent's command, as a profiler's GUI opened on Joel's screen
//! twice on 2026-11-14. The sessions' own windows sit there too, kitty's
//! one instance in agents.slice, and are Joel's to look at, so a pi
//! window is never said. What the launcher opens for him, a look in
//! Reader included, starts in app.slice and is never said either.
//!
//! Only a window opened while the stream runs is said: what's there as it
//! begins was said when it opened, or opened before the daemon ran.
//! Nothing is closed; the line names the unit and the folder it ran in, so
//! the agent behind it can be found.

use std::collections::BTreeSet;
use std::fs;

use serde_json::Value;

use crate::desktop::app_of;

/// An agent's window, as the journal says it.
#[derive(Debug, PartialEq, Eq)]
pub struct Unasked {
    /// The window's app id, as niri says it.
    pub app: String,
    /// The window's title.
    pub title: String,
    /// The unit its process runs in, the last part of its cgroup.
    pub unit: String,
    /// The folder its process runs in, when /proc says it.
    pub folder: Option<String>,
}

/// The windows niri has said, by id.
#[derive(Default)]
pub struct Watched {
    windows: BTreeSet<u64>,
}

impl Watched {
    /// Hears one event of niri's stream. `cgroup` reads a process's
    /// cgroup file and `folder` its working folder.
    pub fn heard(
        &mut self,
        event: &Value,
        cgroup: impl Fn(u32) -> Option<String>,
        folder: impl Fn(u32) -> Option<String>,
    ) -> Option<Unasked> {
        let (name, said) = event.as_object()?.iter().next()?;
        match name.as_str() {
            "WindowsChanged" => {
                self.windows = said["windows"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|window| window["id"].as_u64())
                    .collect();
                None
            }
            "WindowOpenedOrChanged" => {
                let window = &said["window"];
                if !self.windows.insert(window["id"].as_u64()?) {
                    return None;
                }
                let app = window["app_id"].as_str()?;
                if app_of(app).1 {
                    return None;
                }
                let pid = u32::try_from(window["pid"].as_u64()?).ok()?;
                let unit = agents_unit(&cgroup(pid)?)?.to_owned();
                Some(Unasked {
                    app: app.to_owned(),
                    title: window["title"].as_str().unwrap_or_default().to_owned(),
                    unit,
                    folder: folder(pid),
                })
            }
            "WindowClosed" => {
                self.windows.remove(&said["id"].as_u64()?);
                None
            }
            _ => None,
        }
    }
}

/// The unit of a process in agents.slice or a jobs slice, from its cgroup
/// file's unified line, or none for any other.
#[must_use]
pub fn agents_unit(cgroup: &str) -> Option<&str> {
    let path = cgroup.lines().find_map(|line| line.strip_prefix("0::"))?;
    let agents = path.split('/').any(|part| {
        part == "agents.slice"
            || part
                .strip_suffix(".slice")
                .is_some_and(|slice| slice.starts_with("jobs"))
    });
    agents
        .then(|| path.rsplit('/').next())
        .flatten()
        .filter(|unit| !unit.is_empty())
}

/// The journal's line for an agent's window: a warning, so health keeps
/// it among the daemon's lines, though not as an error of its own.
#[must_use]
pub fn line(unasked: &Unasked) -> String {
    let folder = unasked
        .folder
        .as_deref()
        .unwrap_or("a folder /proc didn't say");
    format!(
        "<4>an agent opened a window over Joel's work: {} {:?}, from {} in {folder}",
        unasked.app, unasked.title, unasked.unit
    )
}

/// A process's cgroup file.
pub fn cgroup(pid: u32) -> Option<String> {
    fs::read_to_string(format!("/proc/{pid}/cgroup")).ok()
}

/// A process's working folder.
pub fn folder(pid: u32) -> Option<String> {
    fs::read_link(format!("/proc/{pid}/cwd"))
        .ok()
        .map(|path| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOST: &str = "0::/user.slice/user-1000.slice/user@1000.service/agents.slice/\
                        pi-host-work-background.service\n";
    const JOB: &str = "0::/user.slice/user-1000.slice/user@1000.service/jobs.slice/jobs-work.slice/\
                       jobs-work-aquifer-qest20261017wjatin.slice/run-r1.scope\n";
    const SESSIONS: &str =
        "0::/user.slice/user-1000.slice/user@1000.service/agents.slice/sessions-kitty.service\n";
    const APP: &str =
        "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-slack-2223746.scope\n";

    fn event(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    fn opened(id: u64, app: &str, pid: u32) -> Value {
        let window = format!(r#"{{"id":{id},"title":"Heaptrack","app_id":"{app}","pid":{pid}}}"#);
        event(&format!(
            r#"{{"WindowOpenedOrChanged":{{"window":{window}}}}}"#
        ))
    }

    /// Hears `event` with process 1 in the agents' host, 2 in a job, 3 in
    /// kitty's sessions and every other in app.slice.
    fn hear(watched: &mut Watched, event: &Value) -> Option<Unasked> {
        watched.heard(
            event,
            |pid| {
                let cgroup = match pid {
                    1 => HOST,
                    2 => JOB,
                    3 => SESSIONS,
                    _ => APP,
                };
                Some(cgroup.to_owned())
            },
            |pid| (pid == 1).then(|| "/trees/heaptrack".to_owned()),
        )
    }

    #[test]
    fn a_window_an_agent_opens_is_said_once_with_its_unit_and_folder() {
        let mut watched = Watched::default();
        let said = hear(&mut watched, &opened(166, "org.kde.heaptrack", 1));
        assert_eq!(
            said,
            Some(Unasked {
                app: "org.kde.heaptrack".to_owned(),
                title: "Heaptrack".to_owned(),
                unit: "pi-host-work-background.service".to_owned(),
                folder: Some("/trees/heaptrack".to_owned()),
            }),
            "a window the agents' host opened wasn't said with its unit and folder"
        );
        assert_eq!(
            hear(&mut watched, &opened(166, "org.kde.heaptrack", 1)),
            None,
            "a window changing was said again"
        );
        assert_eq!(
            hear(&mut watched, &opened(167, "org.kde.heaptrack", 2)).map(|unasked| unasked.unit),
            Some("run-r1.scope".to_owned()),
            "a window a job opened wasn't said"
        );
    }

    #[test]
    fn joels_programs_the_sessions_and_windows_there_as_the_stream_begins_are_never_said() {
        let mut watched = Watched::default();
        let listed = event(r#"{"WindowsChanged":{"windows":[{"id":9,"app_id":"x","pid":1}]}}"#);
        assert_eq!(
            hear(&mut watched, &listed),
            None,
            "the listed desktop was said"
        );
        assert_eq!(
            hear(&mut watched, &opened(9, "org.kde.heaptrack", 1)),
            None,
            "a window there as the stream began was said"
        );
        assert_eq!(
            hear(&mut watched, &opened(10, "slack", 40)),
            None,
            "a program of Joel's was said"
        );
        assert_eq!(
            hear(&mut watched, &opened(11, "pi-session.17", 3)),
            None,
            "a session's own window was said"
        );
    }

    #[test]
    fn the_line_names_the_window_its_unit_and_its_folder_as_a_warning() {
        let unasked = Unasked {
            app: "org.kde.heaptrack".to_owned(),
            title: "Heaptrack".to_owned(),
            unit: "pi-host-work-background.service".to_owned(),
            folder: None,
        };
        assert_eq!(
            line(&unasked),
            "<4>an agent opened a window over Joel's work: org.kde.heaptrack \"Heaptrack\", \
             from pi-host-work-background.service in a folder /proc didn't say"
        );
    }
}
