//! Closing an agent's password prompt and telling Joel of it: niri is
//! asked to close the window, the journal says so, and a notice names the
//! unit it came from, at most once a minute for a folder (unasked.rs says
//! which windows are prompts, and whose).

use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::order;
use crate::unasked::{self, Unasked};

/// When Joel was last told of a prompt, by what it's told for: an agent's
/// folder, or its unit where /proc didn't say one, for a prompt closed,
/// and the window for one niri refused to close.
pub type Told = HashMap<String, Instant>;

/// How long a prompt goes untold after one like it was told.
const TOLD_AGAIN: Duration = Duration::from_mins(1);

/// The program a notice is put up through, libnotify's notify-send as the
/// package names it, so the daemon needs no bus of its own.
const NOTIFY_SEND: &str = match option_env!("WINDOWS_NOTIFY_SEND") {
    Some(program) => program,
    None => "notify-send",
};

/// Asks niri to close an agent's password prompt and tells Joel which
/// unit it came from, and whether niri took the ask. The ask holds niri's
/// stream for as long as niri takes to answer it, a second at most; the
/// notice goes up off the stream's thread.
pub fn shut(niri: &Path, unasked: &Unasked, told: &mut Told) -> bool {
    let close = json!({"Action": {"CloseWindow": {"id": unasked.window}}});
    let refused = order::refused(niri, &close);
    match &refused {
        Some(why) => eprintln!("<3>niri didn't close an agent's password prompt: {why}"),
        None => eprintln!("{}", unasked::closed(unasked)),
    }
    let shut = refused.is_none();
    if telling(told, &told_for(unasked, shut), Instant::now()) {
        let (summary, body) = unasked::notice(unasked, shut);
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
    shut
}

/// What a prompt is told for. An agent looping on its test makes a fresh
/// scope each run but works in one folder, so a closed prompt is told
/// once a minute for its folder, each one still closed, and a critical
/// notice never floods. One niri refused is told for its own window, as
/// it's still open, so no other prompt's notice hides it.
fn told_for(unasked: &Unasked, shut: bool) -> String {
    if shut {
        let from = unasked.folder.as_deref().unwrap_or(&unasked.unit);
        format!("from {from}")
    } else {
        format!("window {}", unasked.window)
    }
}

/// Whether to tell Joel of what `key` names now, and so when it last was;
/// what was told over a minute ago is let go, so `told` stays as small as
/// a minute's prompts.
fn telling(told: &mut Told, key: &str, now: Instant) -> bool {
    told.retain(|_, at| now.duration_since(*at) < TOLD_AGAIN);
    if told.contains_key(key) {
        return false;
    }
    told.insert(key.to_owned(), now);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prompt(window: u64, unit: &str, folder: Option<&str>) -> Unasked {
        Unasked {
            window,
            app: "gcr-prompter".to_owned(),
            title: String::new(),
            unit: unit.to_owned(),
            folder: folder.map(str::to_owned),
        }
    }

    #[test]
    fn a_folders_closed_prompts_are_told_once_a_minute_and_a_refusal_each_time_its_window_asks() {
        let mut told = Told::default();
        let start = Instant::now();
        let first = prompt(4, "run-r1.scope", Some("/trees/keyring"));
        let rerun = prompt(5, "run-r2.scope", Some("/trees/keyring"));
        assert!(
            telling(&mut told, &told_for(&first, true), start),
            "a folder's first closed prompt wasn't told"
        );
        assert!(
            !telling(&mut told, &told_for(&rerun, true), start),
            "a rerun in the same folder, in a fresh scope, was told again within a minute"
        );
        assert!(
            telling(&mut told, &told_for(&rerun, false), start),
            "a refused close was hidden by the folder's notice"
        );
        assert!(
            !telling(&mut told, &told_for(&rerun, false), start),
            "a refused window was told twice within a minute"
        );
        let elsewhere = prompt(6, "run-r3.scope", None);
        assert!(
            telling(&mut told, &told_for(&elsewhere, true), start),
            "a prompt with no folder was hidden by another unit's notice"
        );
        let later = start + TOLD_AGAIN;
        assert!(
            telling(&mut told, &told_for(&rerun, true), later),
            "a folder's prompt a minute on wasn't told"
        );
        assert_eq!(
            told.len(),
            1,
            "what was told a minute ago was kept: {told:?}"
        );
    }
}
