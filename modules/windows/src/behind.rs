//! An agent's window that takes Joel's focus as it opens is put behind
//! his work: moved, unfocused, to the empty workspace niri keeps last on
//! its screen, and the window he had focused is focused again. A
//! profiler's GUI an agent ran took his screen twice on 2026-11-14, and
//! a keyring's prompt his password on 2026-11-17 (ISXEG8); prompts are
//! closed instead (closing.rs). It's the daemon's way of doing what niri
//! does for a sandbox's windows (modules/niri/sandboxed-windows-stay-behind.patch),
//! for agents that run outside a sandbox, known by their cgroup
//! (unasked.rs), so it stops an agent's accident, not one set on it.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::{Value, json};

use crate::order;
use crate::unasked::{self, Unasked};

/// The window niri's focus is on, and the last of Joel's it was on before,
/// as the event stream says them. An agent's window is never the one
/// before, so a second agent's window opening focused gives the focus
/// back to Joel's rather than to the first, which would bring Joel's view
/// to where that one was hidden.
#[derive(Default, Debug)]
pub struct Focus {
    now: Option<u64>,
    before: Option<u64>,
    agents: BTreeSet<u64>,
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
            "WindowClosed" => {
                let gone = said["id"].as_u64();
                if self.before == gone {
                    self.before = None;
                }
                if let Some(gone) = gone {
                    self.agents.remove(&gone);
                }
                return;
            }
            _ => return,
        };
        if to != self.now {
            if self.now.is_some_and(|now| !self.agents.contains(&now)) {
                self.before = self.now;
            }
            self.now = to;
        }
    }

    /// Marks `window` as an agent's, so it's never the one given the
    /// focus back.
    pub fn agents(&mut self, window: u64) {
        self.agents.insert(window);
        if self.before == Some(window) {
            self.before = None;
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

/// Whether an agent's window is put behind when it takes the focus: not a
/// prompt, which is closed instead, nor a window of the sessions' own
/// kitty, whose plain windows Joel opens from a pi window himself.
#[must_use]
pub fn moved(unasked: &Unasked) -> bool {
    !unasked::prompts(&unasked.app) && unasked.unit != "sessions-kitty.service"
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

/// Whether niri says `window` has the focus now; not when niri didn't
/// answer, or no window has it.
fn holds(niri: &Path, window: u64) -> bool {
    order::answer(niri, &json!("FocusedWindow"))
        .is_some_and(|answer| answer["Ok"]["FocusedWindow"]["id"].as_u64() == Some(window))
}

/// Puts an agent's window that took the focus behind Joel's work, off
/// the stream's thread so no event waits on niri. The window he had
/// focused, `back`, is focused again first, so his keys leave the
/// agent's window as soon as they can, and only while the agent's window
/// still has the focus, so a window he focused himself meanwhile keeps
/// it, as order.rs's putting back leaves his own press. Then the agent's
/// window moves without focus to the empty workspace last on its screen.
/// Each step niri refuses is said in the journal; a refused move ends it.
pub fn put(niri: &Path, unasked: &Unasked, on: u64, back: Option<u64>) {
    let niri = niri.to_owned();
    let window = unasked.window;
    let from = unasked.unit.clone();
    std::thread::spawn(move || {
        if let Some(back) = back
            && holds(&niri, window)
        {
            let again = json!({"Action": {"FocusWindow": {"id": back}}});
            if let Some(why) = order::refused(&niri, &again) {
                eprintln!(
                    "<4>niri didn't focus window {back} again after an agent's took it: {why}"
                );
            }
        }
        let Some(listed) = order::answer(&niri, &json!("Workspaces")) else {
            eprintln!("<4>niri didn't list its workspaces, so agent's window {window} stayed");
            return;
        };
        let workspaces = listed["Ok"]["Workspaces"]
            .as_array()
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
    fn a_second_agents_window_gives_the_focus_back_to_joels_not_the_firsts() {
        let mut focus = Focus::default();
        focus.heard(&event(
            r#"{"WindowsChanged":{"windows":[{"id":2,"is_focused":true}]}}"#,
        ));
        // Opening focused says the focus moved, with no WindowFocusChanged.
        focus.heard(&event(
            r#"{"WindowOpenedOrChanged":{"window":{"id":7,"is_focused":true}}}"#,
        ));
        focus.agents(7);
        assert_eq!(
            focus.taken_by(7),
            Some(2),
            "a window opening focused, with no focus change said, didn't give back Joel's"
        );
        focus.heard(&event(
            r#"{"WindowOpenedOrChanged":{"window":{"id":8,"is_focused":true}}}"#,
        ));
        focus.agents(8);
        assert_eq!(
            focus.taken_by(8),
            Some(2),
            "a second agent's window gave the focus back to the first agent's"
        );
        focus.heard(&event(r#"{"WindowClosed":{"id":2}}"#));
        focus.heard(&event(r#"{"WindowFocusChanged":{"id":9}}"#));
        assert_eq!(
            focus.taken_by(9),
            None,
            "a closed window, or an agent's, was kept as the one to give the focus back to"
        );
    }

    #[test]
    fn the_sessions_own_kitty_and_prompts_are_never_moved() {
        let opened = |app: &str, unit: &str| Unasked {
            window: 7,
            app: app.to_owned(),
            title: String::new(),
            unit: unit.to_owned(),
            folder: None,
        };
        assert!(
            moved(&opened("foot", "run-r1.scope")),
            "an agent's terminal wasn't moved"
        );
        assert!(
            !moved(&opened("kitty", "sessions-kitty.service")),
            "a kitty window Joel opened from a pi window was moved"
        );
        assert!(
            !moved(&opened("gcr-prompter", "run-r1.scope")),
            "a prompt was moved rather than left to be closed"
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
