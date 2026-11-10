//! Which core the machine the net guards runs: the one its newest switch
//! followed, as update-to-main says in its journal. A hold on switching
//! protects the machine only from a merge it doesn't run yet. On 10
//! November the net held main at 09:55 on #2798, which the machine had run
//! since 08:02, and so kept three fixes Joel was waiting for off it.

use serde_json::{Value, json};

use crate::state::{State, short};
use crate::watch::World;

/// What the net says when a hold lets switches through.
#[must_use]
pub fn lets_through(bad: &str, test: &str) -> String {
    format!(
        "the machine already runs {}, which broke {test}, so the hold lets switches through \
         while its revert is decided",
        short(bad)
    )
}

/// Let switches through a hold whose bad merge the machine has come to
/// run since it was written, as a switch past the hold by hand brings it.
pub fn let_through_once_it_runs(world: &dyn World, state: &State) -> Result<(), String> {
    let Some(mut hold) = state.hold() else {
        return Ok(());
    };
    let Some(bad) = hold["bad"].as_str().map(str::to_owned) else {
        return Ok(());
    };
    if hold["unsettled"] == true || hold["already_runs"] == true || !world.runs(&bad) {
        return Ok(());
    }
    hold["already_runs"] = json!(true);
    state.set_hold(&hold)?;
    let test = hold["test"].as_str().unwrap_or_default();
    state.say(&lets_through(&bad, test));
    Ok(())
}

/// The core the newest switch followed, from update-to-main's journal as
/// `journalctl --output=cat` gives it, oldest first. None when no switch
/// there followed one, so a hold stands as it would without this.
#[must_use]
pub fn core(journal: &str) -> Option<String> {
    let newest = journal
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<Value>(line.get(line.find('{')?..)?).ok())
        .find(|said| said["step"] == "done")?;
    newest["followed"]["core"].as_str().map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_newest_switch_names_the_core_that_runs() {
        let journal = [
            r#"{"step":"done","commit":"m1","applied":"now","followed":{"core":"c1"}}"#,
            "finished switching to system configuration /nix/store/x",
            r#"{"step":"done","commit":"m2","applied":"now","followed":{"core":"c2"}}"#,
            r#"{"step":"refused","commit":"m3","why":"a meeting or a call is within ten minutes"}"#,
        ]
        .join("\n");
        assert_eq!(core(&journal).as_deref(), Some("c2"));
    }

    #[test]
    fn a_journal_with_no_switch_that_followed_core_names_none() {
        assert_eq!(core(""), None);
        assert_eq!(
            core(r#"{"step":"done","commit":"m1","applied":"now"}"#),
            None,
            "a switch that followed no core says nothing of it"
        );
        assert_eq!(core("not json {"), None);
        let older = r#"{"step":"done","commit":"m1","applied":"now","followed":{"core":"c1"}}"#;
        let newer = r#"{"step":"done","commit":"m2","applied":"now"}"#;
        assert_eq!(
            core(&format!("{older}\n{newer}")),
            None,
            "an older switch's core no longer runs"
        );
    }
}
