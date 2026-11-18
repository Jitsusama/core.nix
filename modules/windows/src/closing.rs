//! Closing an agent's password prompt and telling Joel of it: niri is
//! asked to close the window, the journal says so, and a notice names the
//! unit it came from, at most once a minute for a unit (unasked.rs says
//! which windows are prompts, and whose).

use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::order;
use crate::unasked::{self, Unasked};

/// When Joel was last told of each agent's unit's password prompt.
pub type Told = HashMap<String, Instant>;

/// How long a unit's prompts go untold after one was told.
const TOLD_AGAIN: Duration = Duration::from_mins(1);

/// The program a notice is put up through, libnotify's notify-send as the
/// package names it, so the daemon needs no bus of its own.
const NOTIFY_SEND: &str = match option_env!("WINDOWS_NOTIFY_SEND") {
    Some(program) => program,
    None => "notify-send",
};

/// Closes an agent's password prompt through niri and tells Joel which
/// unit it came from, off the stream's thread, so niri's next event isn't
/// held while the notice goes up.
pub fn shut(niri: &Path, unasked: &Unasked, told: &mut Told) {
    let close = json!({"Action": {"CloseWindow": {"id": unasked.window}}});
    let refused = order::refused(niri, &close);
    match &refused {
        Some(why) => eprintln!("<3>niri didn't close an agent's password prompt: {why}"),
        None => eprintln!("{}", unasked::closed(unasked)),
    }
    // An agent looping on its prompt is told once a minute for its unit,
    // each prompt still closed, so a critical notice never floods; one
    // niri refused is always told, since it's still open.
    let now = Instant::now();
    let lately = told
        .get(&unasked.unit)
        .is_some_and(|at| now.duration_since(*at) < TOLD_AGAIN);
    if lately && refused.is_none() {
        return;
    }
    told.insert(unasked.unit.clone(), now);
    let (summary, body) = unasked::notice(unasked, refused.is_none());
    std::thread::spawn(move || {
        let told = std::process::Command::new(NOTIFY_SEND)
            .args([
                "--app-name=Windows",
                "--urgency=critical",
                "--icon=dialog-password",
            ])
            .args([summary, body])
            .status();
        if !told.as_ref().is_ok_and(std::process::ExitStatus::success) {
            eprintln!("<3>Joel couldn't be told of an agent's password prompt: {told:?}");
        }
    });
}
