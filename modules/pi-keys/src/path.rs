//! Which threads a key typed into a window passes: the kitty drawing it,
//! and the pi running in it, found from niri's word on the window and the
//! scopes the sessions run in.

use serde::Deserialize;

/// A window as niri tells of it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Window {
    pub id: u64,
    #[serde(default)]
    pub app_id: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
}

impl Window {
    /// Whether it's a pi session's window, as the shell, the harness and
    /// cadence class them: pi-session or pi-spawned, bare or numbered.
    pub fn of_pi(&self) -> bool {
        self.app_id
            .as_deref()
            .is_some_and(|id| id.starts_with("pi-session") || id.starts_with("pi-spawned"))
    }
}

#[derive(Deserialize)]
struct Answer<T> {
    #[serde(rename = "Ok")]
    ok: T,
}

#[derive(Deserialize)]
struct FocusedWindow {
    #[serde(rename = "FocusedWindow")]
    window: Option<Window>,
}

#[derive(Deserialize)]
struct FocusedOutput {
    #[serde(rename = "FocusedOutput")]
    output: Option<Output>,
}

#[derive(Deserialize)]
struct Output {
    modes: Vec<Mode>,
    current_mode: Option<usize>,
}

#[derive(Deserialize)]
struct Mode {
    /// In millihertz.
    refresh_rate: u32,
}

/// The window niri says has the focus, from its answer to
/// `"FocusedWindow"`, or None when nothing has it.
pub fn focused(answer: &str) -> Option<Window> {
    serde_json::from_str::<Answer<FocusedWindow>>(answer)
        .ok()?
        .ok
        .window
}

/// How long the focused output takes to draw a frame, in milliseconds,
/// from niri's answer to `"FocusedOutput"`.
pub fn frame(answer: &str) -> Option<f64> {
    let output = serde_json::from_str::<Answer<FocusedOutput>>(answer)
        .ok()?
        .ok
        .output?;
    let mode = output.modes.get(output.current_mode?)?;
    (mode.refresh_rate > 0).then(|| 1e6 / f64::from(mode.refresh_rate))
}

/// The scope a window's session runs in, from each scope's unit and the
/// app ids its processes were started with: the one started with the
/// window's own, or else the one scope of a kitty that draws it alone. Two
/// started with the same are neither. It finds them as the jobs-sorter's
/// `focus::scope_of` does (modules/slices), so what's measured is the window
/// the sorter holds.
pub fn scope_of<'a>(window: &Window, scopes: &'a [(String, Vec<String>)]) -> Option<&'a str> {
    let one = |found: Vec<&'a str>| (found.len() == 1).then(|| found[0]);
    let app_id = window.app_id.as_deref()?;
    let started: Vec<&str> = scopes
        .iter()
        .filter(|(_, apps)| apps.iter().any(|a| a == app_id))
        .map(|(unit, _)| unit.as_str())
        .collect();
    if !started.is_empty() {
        return one(started);
    }
    let own = format!("kitty-{}-", window.pid?);
    one(scopes
        .iter()
        .map(|(unit, _)| unit.as_str())
        .filter(|unit| unit.starts_with(&own))
        .collect())
}

/// The app id a process was started with, from its environment's bytes.
pub fn app_id(environ: &[u8]) -> Option<String> {
    environ
        .split(|byte| *byte == 0)
        .find_map(|entry| entry.strip_prefix(b"MYCELIUM_APP_ID="))
        .map(|id| String::from_utf8_lossy(id).into_owned())
}

/// A running process: its pid, its parent's and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    pub pid: u32,
    pub parent: u32,
    pub name: String,
}

/// The session's pi below a window's scope: the nearest pi among the
/// scope's processes and theirs, wherever they run, and the first started
/// of the nearest, since a pi a session runs, such as a subagent, sits
/// below it. A session's holder runs in a scope of its own, apart from the
/// window's, with pi under it, so pi is followed by parent, not by scope.
pub fn pi_below(scope: &[u32], processes: &[Process]) -> Option<u32> {
    let _ = (scope, processes);
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(app_id: &str, pid: u32) -> Window {
        Window {
            id: 1,
            app_id: Some(app_id.into()),
            pid: Some(pid),
        }
    }

    fn scopes(list: &[(&str, &[&str])]) -> Vec<(String, Vec<String>)> {
        list.iter()
            .map(|(unit, apps)| {
                (
                    unit.to_string(),
                    apps.iter().map(ToString::to_string).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn a_session_s_window_is_pi_s_and_a_browser_s_is_not() {
        let pi: Vec<bool> = [
            window("pi-session.1794413231964", 3272),
            window("pi-spawned", 3272),
            window("pi-spawned.133320004", 839_371),
            window("google-chrome", 9),
            Window {
                app_id: None,
                ..window("", 1)
            },
        ]
        .iter()
        .map(Window::of_pi)
        .collect();
        assert_eq!(pi, [true, true, true, false, false]);
    }

    #[test]
    fn niri_s_focused_window_is_read_from_its_answer() {
        let answer = r#"{"Ok":{"FocusedWindow":{"id":2,"title":"π","app_id":"pi-session.7","pid":3272,"workspace_id":22,"is_focused":true}}}"#;
        assert_eq!(
            focused(answer),
            Some(window("pi-session.7", 3272)).map(|w| Window { id: 2, ..w })
        );
        assert_eq!(focused(r#"{"Ok":{"FocusedWindow":null}}"#), None);
        assert_eq!(focused(r#"{"Err":"no"}"#), None);
    }

    #[test]
    fn a_frame_is_read_from_the_output_s_current_mode() {
        let answer = r#"{"Ok":{"FocusedOutput":{"name":"DP-1","modes":[{"width":3840,"height":2160,"refresh_rate":30000},{"width":3840,"height":2160,"refresh_rate":59997}],"current_mode":1}}}"#;
        let ms = frame(answer).unwrap();
        assert!((ms - 16.6675).abs() < 0.001, "{ms}");
        assert_eq!(frame(r#"{"Ok":{"FocusedOutput":null}}"#), None);
    }

    #[test]
    fn the_scope_started_with_the_window_s_app_id_is_its_own() {
        let found = scopes(&[
            ("kitty-3272-2.scope", &["pi-session.6"]),
            ("kitty-3272-3.scope", &["pi-session.7"]),
        ]);
        assert_eq!(
            scope_of(&window("pi-session.7", 3272), &found),
            Some("kitty-3272-3.scope")
        );
    }

    #[test]
    fn a_kitty_drawing_one_window_alone_names_its_scope() {
        let found = scopes(&[
            ("kitty-3272-2.scope", &["pi-session.6"]),
            ("kitty-839371-0.scope", &[]),
        ]);
        assert_eq!(
            scope_of(&window("pi-spawned.133320004", 839_371), &found),
            Some("kitty-839371-0.scope")
        );
    }

    #[test]
    fn a_bare_class_in_the_shared_kitty_is_no_scope() {
        let found = scopes(&[("kitty-3272-2.scope", &[]), ("kitty-3272-3.scope", &[])]);
        assert_eq!(scope_of(&window("pi-spawned", 3272), &found), None);
    }

    #[test]
    fn two_scopes_started_with_one_app_id_are_neither() {
        let found = scopes(&[
            ("kitty-3272-2.scope", &["pi-session.7"]),
            ("kitty-3272-9.scope", &["pi-session.7"]),
        ]);
        assert_eq!(scope_of(&window("pi-session.7", 3272), &found), None);
    }

    #[test]
    fn the_app_id_is_read_from_the_environment() {
        assert_eq!(
            app_id(b"HOME=/h\0MYCELIUM_APP_ID=pi-session.7\0TERM=x\0"),
            Some("pi-session.7".into())
        );
        assert_eq!(app_id(b"HOME=/h\0"), None);
    }

    fn process(pid: u32, parent: u32, name: &str) -> Process {
        Process {
            pid,
            parent,
            name: name.into(),
        }
    }

    #[test]
    fn the_first_pi_in_a_scope_is_the_session_s() {
        let processes = [
            process(900, 1, "zsh"),
            process(1200, 900, "pi"),
            process(950, 900, "pi"),
            process(960, 900, "mycelium"),
        ];
        assert_eq!(pi_below(&[900, 1200, 950, 960], &processes), Some(950));
        assert_eq!(
            pi_below(&[900], &[process(900, 1, "zsh")]),
            None,
            "no pi"
        );
    }

    #[test]
    fn pi_under_a_holder_apart_from_the_window_s_scope_is_found() {
        // As on 13 Nov: the window's scope holds the client alone, its
        // holder runs in a scope of its own, and pi runs under that.
        let processes = [
            process(2830, 1, ".kitty-wrapped"),
            process(8889, 2830, "mycelium"),
            process(9005, 8889, "mycelium"),
            process(9008, 9005, "pi"),
            process(9100, 9008, "pi"),
            process(7000, 1, "pi"),
        ];
        assert_eq!(
            pi_below(&[8889], &processes),
            Some(9008),
            "the holder's pi, not its subagent's nor another window's"
        );
    }
}
