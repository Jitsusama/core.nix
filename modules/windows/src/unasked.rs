//! A window an agent opened over Joel's work, said to the journal.
//!
//! Agents run in agents.slice and the slices polkit's rules name for
//! them, apart from Joel's programs in app.slice, so a window whose process sits there was opened
//! by an agent's command, as a profiler's GUI opened on Joel's screen
//! twice on 2026-11-14. The sessions' own windows sit there too, kitty's
//! one instance in agents.slice, and are Joel's to look at, so a pi
//! window is never said. What the launcher opens for him, a look in
//! Reader included, starts in app.slice and is never said either.
//!
//! Only a window opened while the stream runs is said: what's there as it
//! begins was said when it opened, or opened before the daemon ran.
//! The line names the unit and the folder it ran in, so the agent behind
//! it can be found.
//!
//! One that asks for a password is closed as well, and Joel told why
//! (`prompts`): an agent's test put a keyring's prompt on his screen twice
//! on 2026-11-17, and he typed his real password into it (ISXEG8). Unlike
//! the line, it's weighed whenever niri says a window, there as the stream
//! begins or changed since, as a prompt's app id may come late, and a
//! daemon restarted by a switch would otherwise leave one open. The
//! cgroup is a label an agent could change, not a boundary, so this stops
//! an agent's accident, not an agent set on getting past it.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::desktop::app_of;

/// An agent's window, as the journal says it.
#[derive(Debug, PartialEq, Eq)]
pub struct Unasked {
    /// The window's id, as niri says it.
    pub window: u64,
    /// The window's app id, as niri says it.
    pub app: String,
    /// The window's title.
    pub title: String,
    /// The unit its process runs in, the last part of its cgroup.
    pub unit: String,
    /// The folder its process runs in, when /proc says it.
    pub folder: Option<String>,
}

/// The windows niri has said, by id, and the agents' password prompts
/// niri was asked to close.
#[derive(Default)]
pub struct Watched {
    windows: BTreeSet<u64>,
    closing: BTreeSet<u64>,
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
                let id = window["id"].as_u64()?;
                if !self.windows.insert(id) {
                    return None;
                }
                let app = window["app_id"].as_str()?;
                if app_of(app).1 {
                    return None;
                }
                let pid = u32::try_from(window["pid"].as_u64()?).ok()?;
                let unit = agents_unit(&cgroup(pid)?)?.to_owned();
                Some(Unasked {
                    window: id,
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

    /// The agents' password prompts in one event of niri's stream, each
    /// once until it closes: those listed as the stream begins, and one
    /// opened or changed since, so a prompter naming itself only after
    /// it maps is weighed again then.
    pub fn prompting(
        &mut self,
        event: &Value,
        cgroup: impl Fn(u32) -> Option<String>,
        folder: impl Fn(u32) -> Option<String>,
    ) -> Vec<Unasked> {
        let Some((name, said)) = event.as_object().and_then(|said| said.iter().next()) else {
            return Vec::new();
        };
        let windows: Vec<&Value> = match name.as_str() {
            "WindowsChanged" => said["windows"].as_array().into_iter().flatten().collect(),
            "WindowOpenedOrChanged" => vec![&said["window"]],
            "WindowClosed" => {
                if let Some(id) = said["id"].as_u64() {
                    self.closing.remove(&id);
                }
                Vec::new()
            }
            _ => Vec::new(),
        };
        let mut found = Vec::new();
        for window in windows {
            let Some(id) = window["id"].as_u64() else {
                continue;
            };
            let app = window["app_id"].as_str().unwrap_or_default();
            if self.closing.contains(&id) || !prompts(app) {
                continue;
            }
            let Some(pid) = window["pid"]
                .as_u64()
                .and_then(|pid| u32::try_from(pid).ok())
            else {
                continue;
            };
            let Some(unit) = cgroup(pid).and_then(|said| agents_unit(&said).map(str::to_owned))
            else {
                continue;
            };
            self.closing.insert(id);
            found.push(Unasked {
                window: id,
                app: app.to_owned(),
                title: window["title"].as_str().unwrap_or_default().to_owned(),
                unit,
                folder: folder(pid),
            });
        }
        found
    }

    /// Weighs the prompt `window` again when niri next says it, as niri
    /// refused to close it and it's still open.
    pub fn ask_again(&mut self, window: u64) {
        self.closing.remove(&window);
    }
}

/// The unit of a process in a place polkit's rules take for an agent's
/// (modules/slices/agents-ask-first.rules), from its cgroup file's unified
/// line, or none for any other: agents.slice, jobs.service, jobs-work.slice,
/// jobs-timed.slice, shared.slice or a pi-jobs scope. What Joel starts at a
/// terminal runs in jobs-awaited.slice, none of these, so it's his.
#[must_use]
pub fn agents_unit(cgroup: &str) -> Option<&str> {
    let path = cgroup.lines().find_map(|line| line.strip_prefix("0::"))?;
    let parts: Vec<&str> = path.split('/').collect();
    let within = parts.windows(2).any(|pair| {
        matches!(
            pair[0],
            "agents.slice" | "jobs-work.slice" | "jobs-timed.slice" | "shared.slice"
        ) || pair == ["jobs.slice", "jobs.service"]
    });
    let fenced = parts
        .iter()
        .any(|part| part.starts_with("pi-jobs-") && part.ends_with(".scope"));
    let agents = within || fenced;
    agents
        .then(|| path.rsplit('/').next())
        .flatten()
        .filter(|unit| !unit.is_empty())
}

/// The journal's line for an agent's window: a warning, so health keeps
/// it among the daemon's lines, though not as an error of its own. The
/// app id, title and folder are quoted, as an agent names each, so none
/// can start a journal line of its own.
#[must_use]
pub fn line(unasked: &Unasked) -> String {
    let folder = unasked.folder.as_deref().map_or_else(
        || "a folder /proc didn't say".to_owned(),
        |folder| format!("{folder:?}"),
    );
    format!(
        "<4>an agent opened a window over Joel's work: {:?} {:?}, from {} in {folder}",
        unasked.app, unasked.title, unasked.unit
    )
}

/// Whether an agent's window asks for a password, by the app id it names
/// itself with: a keyring's prompter (gcr-prompter, gnome-keyring's
/// `SystemPrompter`), gpg's pinentry, ssh's askpass in each of its kinds, or
/// a polkit agent, or `KWallet`'s. polkit already refuses an agent's asks
/// (ISXEG8, #3548), but an agent can start its own agent, as it started
/// its own keyring. An X11 prompter is missed: niri gives the pid of
/// xwayland-satellite, which runs in niri's own unit.
#[must_use]
pub fn prompts(app: &str) -> bool {
    let app = app.to_lowercase();
    PROMPTERS.iter().any(|prompter| app.contains(prompter))
}

const PROMPTERS: [&str; 7] = [
    "askpass",
    "gcr-prompter",
    "kwallet",
    "pinentry",
    "policykit",
    "polkit",
    "systemprompter",
];

/// The journal's line for an agent's password prompt niri took the ask to
/// close: a warning, as the window's own line is. niri asks the window to
/// close, as its own close key does, so a prompter that ignored it would
/// stay; gcr-prompter, pinentry and askpass close. The app id and title
/// are quoted, so no window's name can start a journal line of its own.
#[must_use]
pub fn closed(unasked: &Unasked) -> String {
    format!(
        "<4>asked niri to close a password prompt an agent opened: {:?} {:?}, from {}",
        unasked.app, unasked.title, unasked.unit
    )
}

/// What Joel is told of an agent's password prompt, its summary and its
/// body, naming where it came from: that it was asked to close, since
/// niri asks the window as its own close key does and a prompter could
/// ignore it, or, when niri refused, that it couldn't be. Either way
/// nothing should be typed there. It says
/// window, the glossary's word, since a prompt there is the shell's line
/// (H2VYOB). The unit and folder are said as they are, escaped, since the
/// notices draw markup and an agent names its own folder.
#[must_use]
pub fn notice(unasked: &Unasked, shut: bool) -> (String, String) {
    let folder = unasked
        .folder
        .as_deref()
        .map_or_else(String::new, |folder| format!(", in {}", escaped(folder)));
    let from = format!("It came from {}{folder}", escaped(&unasked.unit));
    let summary = "An agent opened a password window".to_owned();
    if shut {
        (
            summary,
            format!("{from}, and was asked to close. A password typed there reaches the agent."),
        )
    } else {
        (
            summary,
            format!("{from}, and couldn't be closed: type no password there."),
        )
    }
}

/// `text` with what notices read as markup written as text.
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Whether this build reads processes where `WINDOWS_PROC` says, which
/// only the check's own build does, so nothing in Joel's account can blind
/// the installed daemon by setting it in the user manager.
const READS_A_STAND_IN: bool = option_env!("WINDOWS_TEST_PROC").is_some();

/// Where processes are read: /proc, or, in the check's build, the folder
/// `WINDOWS_PROC` names, so a stand-in's process can sit in an agent's
/// scope the build sandbox has no cgroups for.
#[must_use]
pub fn processes() -> PathBuf {
    let stand_in = READS_A_STAND_IN
        .then(|| std::env::var_os("WINDOWS_PROC"))
        .flatten();
    stand_in.map_or_else(|| PathBuf::from("/proc"), PathBuf::from)
}

/// A process's cgroup file.
pub fn cgroup(processes: &Path, pid: u32) -> Option<String> {
    fs::read_to_string(processes.join(pid.to_string()).join("cgroup")).ok()
}

/// A process's working folder.
pub fn folder(processes: &Path, pid: u32) -> Option<String> {
    fs::read_link(processes.join(pid.to_string()).join("cwd"))
        .ok()
        .map(|path| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOST: &str = "0::/user.slice/user-1000.slice/user@1000.service/agents.slice/\
                        pi-host-work-background.service\n";
    const JOB: &str = "0::/user.slice/user-1000.slice/user@1000.service/jobs.slice/jobs-work.slice/\
                       jobs-work-cistern-qest20261017wjatin.slice/run-r1.scope\n";
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
                window: 166,
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
    fn a_password_prompt_is_known_by_the_app_id_it_names_itself_with() {
        for app in [
            "gcr-prompter",
            "org.gnome.keyring.SystemPrompter",
            "pinentry-qt",
            "org.gnupg.pinentry-gtk",
            "ssh-askpass",
            "lxqt-openssh-askpass",
            "org.kde.ksshaskpass",
            "polkit-gnome-authentication-agent-1",
            "lxqt-policykit-agent",
            "org.kde.kwalletd6",
        ] {
            assert!(prompts(app), "{app} wasn't known for a password prompt");
        }
        for app in [
            "org.kde.heaptrack",
            "kitty",
            "slack",
            "1Password",
            "org.gnome.Nautilus",
        ] {
            assert!(!prompts(app), "{app} was taken for a password prompt");
        }
    }

    #[test]
    fn a_prompt_closed_is_said_and_told_naming_its_unit() {
        let unasked = Unasked {
            window: 7,
            app: "gcr-prompter".to_owned(),
            title: "Unlock Keyring".to_owned(),
            unit: "run-r1.scope".to_owned(),
            folder: Some("/trees/keyring".to_owned()),
        };
        assert_eq!(
            closed(&unasked),
            "<4>asked niri to close a password prompt an agent opened: \"gcr-prompter\" \
             \"Unlock Keyring\", from run-r1.scope"
        );
        assert_eq!(
            notice(&unasked, true),
            (
                "An agent opened a password window".to_owned(),
                "It came from run-r1.scope, in /trees/keyring, and was asked to close. A \
                 password typed there reaches the agent."
                    .to_owned()
            )
        );
        assert_eq!(
            notice(&unasked, false),
            (
                "An agent opened a password window".to_owned(),
                "It came from run-r1.scope, in /trees/keyring, and couldn't be closed: type no \
                 password there."
                    .to_owned()
            ),
            "a prompt niri didn't close was told as closed"
        );
    }

    #[test]
    fn a_folder_an_agent_names_is_told_as_text_and_an_app_id_starts_no_line() {
        let unasked = Unasked {
            window: 7,
            app: "gcr-prompter\n<0>forged".to_owned(),
            title: String::new(),
            unit: "run-r1.scope".to_owned(),
            folder: Some("/trees/<a href=\"x\">a&b</a>".to_owned()),
        };
        assert!(
            notice(&unasked, true)
                .1
                .contains("/trees/&lt;a href=\"x\"&gt;a&amp;b&lt;/a&gt;"),
            "the folder went to the notice as markup: {}",
            notice(&unasked, true).1
        );
        assert!(
            !closed(&unasked).contains('\n'),
            "an app id started a journal line: {}",
            closed(&unasked)
        );
    }

    /// Weighs `event` for prompts with processes placed as `hear` does.
    fn prompting(watched: &mut Watched, event: &Value) -> Vec<u64> {
        watched
            .prompting(
                event,
                |pid| {
                    let cgroup = match pid {
                        1 => HOST,
                        2 => JOB,
                        _ => APP,
                    };
                    Some(cgroup.to_owned())
                },
                |_| None,
            )
            .into_iter()
            .map(|prompt| prompt.window)
            .collect()
    }

    #[test]
    fn an_agents_prompt_is_closed_once_whenever_niri_says_it() {
        let mut watched = Watched::default();
        let listed = event(
            r#"{"WindowsChanged":{"windows":[
                {"id":4,"app_id":"gcr-prompter","pid":2},
                {"id":5,"app_id":"gcr-prompter","pid":40},
                {"id":6,"app_id":"kitty","pid":2}]}}"#,
        );
        assert_eq!(
            prompting(&mut watched, &listed),
            vec![4],
            "an agent's prompt open as the stream began wasn't closed, or Joel's was"
        );
        assert_eq!(
            prompting(&mut watched, &opened(4, "gcr-prompter", 2)),
            Vec::<u64>::new(),
            "a prompt already asked to close was asked again"
        );
        assert_eq!(
            prompting(&mut watched, &opened(8, "", 1)),
            Vec::<u64>::new(),
            "a window with no app id yet was taken for a prompt"
        );
        assert_eq!(
            prompting(&mut watched, &opened(8, "pinentry-qt", 1)),
            vec![8],
            "a prompt naming itself after it mapped wasn't closed"
        );
        watched.ask_again(8);
        assert_eq!(
            prompting(&mut watched, &opened(8, "pinentry-qt", 1)),
            vec![8],
            "a prompt niri refused to close wasn't asked again"
        );
        prompting(&mut watched, &event(r#"{"WindowClosed":{"id":8}}"#));
        assert_eq!(
            prompting(&mut watched, &opened(8, "pinentry-qt", 1)),
            vec![8],
            "a prompt again under a closed one's id wasn't closed"
        );
    }

    #[test]
    fn the_line_names_the_window_its_unit_and_its_folder_as_a_warning() {
        let unasked = Unasked {
            window: 166,
            app: "org.kde.heaptrack".to_owned(),
            title: "Heaptrack".to_owned(),
            unit: "pi-host-work-background.service".to_owned(),
            folder: None,
        };
        assert_eq!(
            line(&unasked),
            "<4>an agent opened a window over Joel's work: \"org.kde.heaptrack\" \"Heaptrack\", \
             from pi-host-work-background.service in a folder /proc didn't say"
        );
    }

    #[test]
    fn what_joel_starts_at_a_terminal_is_his_though_it_runs_in_a_jobs_slice() {
        let awaited = "0::/user.slice/user-1000.slice/user@1000.service/jobs.slice/\
                       jobs-awaited.slice/work-40-5.scope\n";
        assert_eq!(
            agents_unit(awaited),
            None,
            "Joel's terminal work was taken for an agent's"
        );
        assert_eq!(
            agents_unit(JOB),
            Some("run-r1.scope"),
            "an agent's job wasn't known"
        );
    }

    #[test]
    fn an_agents_places_are_those_polkits_rules_name() {
        let account = "0::/user.slice/user-1000.slice/user@1000.service";
        for (place, unit) in [
            ("shared.slice/push-walk-3.scope", "push-walk-3.scope"),
            ("jobs.slice/jobs.service", "jobs.service"),
            ("jobs.slice/jobs-timed.slice/run-r9.scope", "run-r9.scope"),
            (
                "jobs.slice/jobs-awaited.slice/pi-jobs-7.scope",
                "pi-jobs-7.scope",
            ),
        ] {
            assert_eq!(
                agents_unit(&format!("{account}/{place}\n")),
                Some(unit),
                "an agent's process in {place} wasn't known as polkit knows it"
            );
        }
        assert_eq!(
            agents_unit(&format!("{account}/jobs.slice/jobs-sorter.service\n")),
            None,
            "a jobs slice polkit doesn't name was taken for an agent's"
        );
    }
}
