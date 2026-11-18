//! An agent's window that takes Joel's focus as it opens is put behind
//! his work: moved, unfocused, to the empty workspace niri keeps last on
//! its screen, and the window he had focused is focused again. A
//! profiler's GUI an agent ran took his screen twice on 2026-11-14, and
//! a keyring's prompt his password on 2026-11-17 (ISXEG8); prompts are
//! closed instead (closing.rs). It's the daemon's way of doing what niri
//! does for a sandbox's windows (modules/niri/sandboxed-windows-stay-behind.patch),
//! for agents that run outside a sandbox, known by their cgroup
//! (unasked.rs), so it stops an agent's accident, not one set on it.

use std::path::Path;

use serde_json::{Value, json};

use crate::order;
use crate::unasked::Unasked;

/// The window niri's focus is on, and the one it was on before, as the
/// event stream says them.
#[derive(Default, Debug)]
pub struct Focus {
    now: Option<u64>,
    before: Option<u64>,
}

impl Focus {
    /// Hears one event of niri's stream. The list as the stream begins
    /// says where the focus is without a before, since nothing moved it.
    pub fn heard(&mut self, event: &Value) {
        let Some((name, said)) = event.as_object().and_then(|said| said.iter().next()) else {
            return;
        };
        let to = match name.as_str() {
            "WindowsChanged" => {
                self.now = said["windows"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|window| window["is_focused"].as_bool() == Some(true))
                    .and_then(|window| window["id"].as_u64());
                self.before = None;
                return;
            }
            "WindowOpenedOrChanged" if said["window"]["is_focused"].as_bool() == Some(true) => {
                said["window"]["id"].as_u64()
            }
            "WindowFocusChanged" => said["id"].as_u64(),
            _ => return,
        };
        if to != self.now {
            self.before = self.now;
            self.now = to;
        }
    }

    /// The window to give the focus back to, when `window` has just
    /// taken it, or none when it hasn't, or nothing had it before.
    #[must_use]
    pub fn taken_by(&self, window: u64) -> Option<u64> {
        if self.now == Some(window) {
            self.before
        } else {
            None
        }
    }
}

/// The workspace a window that just opened focused opened on, from the
/// event that says it, or none for any other event.
#[must_use]
pub fn opened_focused(event: &Value) -> Option<u64> {
    let window = &event.get("WindowOpenedOrChanged")?["window"];
    if window["is_focused"].as_bool() != Some(true) {
        return None;
    }
    window["workspace_id"].as_u64()
}

/// Where a window opened on workspace `on` goes behind: the last empty
/// workspace on the same screen, which niri always keeps, other than
/// `on` itself.
#[must_use]
pub fn emptiest(workspaces: &[Value], on: u64) -> Option<u64> {
    let screen = workspaces
        .iter()
        .find(|workspace| workspace["id"].as_u64() == Some(on))?["output"]
        .as_str()?;
    workspaces
        .iter()
        .filter(|workspace| {
            workspace["output"].as_str() == Some(screen)
                && workspace["id"].as_u64() != Some(on)
                && workspace["active_window_id"].is_null()
        })
        .max_by_key(|workspace| workspace["idx"].as_u64())
        .and_then(|workspace| workspace["id"].as_u64())
}

/// Puts an agent's window that took the focus behind Joel's work, off
/// the stream's thread so no event waits on niri: moved without focus to
/// the empty workspace last on its screen, then the window he had
/// focused, `back`, focused again. Each step niri refuses is said in the
/// journal and ends it.
pub fn put(niri: &Path, unasked: &Unasked, on: u64, back: Option<u64>) {
    let niri = niri.to_owned();
    let window = unasked.window;
    let from = unasked.unit.clone();
    std::thread::spawn(move || {
        let listed = order::answer(&niri, &json!("Workspaces"));
        let workspaces = listed
            .as_ref()
            .and_then(|listed| listed["Ok"]["Workspaces"].as_array())
            .map_or(&[][..], Vec::as_slice);
        let Some(behind) = emptiest(workspaces, on) else {
            eprintln!("<4>niri had no empty workspace to put an agent's window {window} behind");
            return;
        };
        let moving = json!({"Action": {"MoveWindowToWorkspace": {
            "window_id": window,
            "reference": {"Id": behind},
            "focus": false
        }}});
        if let Some(why) = order::refused(&niri, &moving) {
            eprintln!("<4>niri didn't put an agent's window {window} behind: {why}");
            return;
        }
        eprintln!("<4>put an agent's window behind Joel's work: window {window}, from {from}");
        let Some(back) = back else {
            return;
        };
        let again = json!({"Action": {"FocusWindow": {"id": back}}});
        if let Some(why) = order::refused(&niri, &again) {
            eprintln!("<4>niri didn't focus window {back} again after an agent's took it: {why}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(text: &str) -> Value {
        serde_json::from_str(text).expect("a test's event isn't JSON")
    }

    #[test]
    fn the_focus_is_given_back_to_the_window_an_agents_took_it_from() {
        let mut focus = Focus::default();
        focus.heard(&event(
            r#"{"WindowsChanged":{"windows":[
                {"id":1,"is_focused":false},{"id":2,"is_focused":true}]}}"#,
        ));
        assert_eq!(
            focus.taken_by(2),
            None,
            "the window focused as the stream began was taken to have taken it"
        );
        focus.heard(&event(
            r#"{"WindowOpenedOrChanged":{"window":{"id":7,"is_focused":true}}}"#,
        ));
        focus.heard(&event(r#"{"WindowFocusChanged":{"id":7}}"#));
        assert_eq!(
            focus.taken_by(7),
            Some(2),
            "an agent's window opening focused didn't give back Joel's"
        );
        assert_eq!(
            focus.taken_by(1),
            None,
            "a window without the focus was taken to have taken it"
        );
    }

    #[test]
    fn a_window_goes_behind_on_the_last_empty_workspace_of_its_screen() {
        let workspaces: Vec<Value> = serde_json::from_str(
            r#"[
                {"id":1,"idx":1,"output":"eDP-1","active_window_id":4},
                {"id":2,"idx":2,"output":"eDP-1","active_window_id":null},
                {"id":3,"idx":3,"output":"eDP-1","active_window_id":null},
                {"id":5,"idx":1,"output":"DP-4","active_window_id":null}
            ]"#,
        )
        .expect("the test's workspaces aren't JSON");
        assert_eq!(
            emptiest(&workspaces, 1),
            Some(3),
            "the window didn't go to the last empty workspace on its own screen"
        );
        assert_eq!(
            emptiest(&workspaces, 3),
            Some(2),
            "a window opened on the last empty workspace was put where it already is"
        );
        assert_eq!(
            emptiest(&workspaces, 9),
            None,
            "a workspace niri doesn't list was given a place"
        );
    }

    #[test]
    fn only_a_window_opening_focused_is_put_behind() {
        let opening = |focused: bool| {
            json!({"WindowOpenedOrChanged": {"window": {
                "id": 7, "is_focused": focused, "workspace_id": 1
            }}})
        };
        assert_eq!(
            opened_focused(&opening(true)),
            Some(1),
            "a window opening focused wasn't put behind"
        );
        assert_eq!(
            opened_focused(&opening(false)),
            None,
            "a window opening behind was moved"
        );
    }
}
