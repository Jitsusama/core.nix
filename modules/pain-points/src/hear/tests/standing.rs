//! How the hour counts a pain someone took up or a landing closed, and a
//! run that finds both on the mesh and the machine.

use super::*;
use std::os::unix::fs::PermissionsExt;

fn pain(entry: &str, words: &str, hits: usize) -> Heard {
    serde_json::from_value(json!({
        "key": format!("s/{entry}"), "session": "s", "entry": entry, "at": AT - 60_000,
        "words": words, "kinds": ["corrected"], "hits": hits, "carried": false
    }))
    .expect("a pain")
}

#[test]
fn the_hour_leaves_out_of_open_what_was_said_to_the_void_taken_up_or_closed() {
    let mut void = pain("a", "the void heard this", 9);
    void.void = true;
    let mut taken = pain("b", "someone took this", 8);
    taken.taken_by = Some("ABC123".into());
    let mut closed = pain("c", "a landing closed this", 7);
    closed.taken_by = Some("ABC123".into());
    closed.closed_by = Some("255494bb414c".into());
    let open = pain("d", "nobody has this", 1);
    let hour = hour_of(&[void, taken, closed, open], AT);
    assert_eq!(
        (hour.open, hour.taken, hour.closed),
        (1, 1, 1),
        "one each: {hour:?}"
    );
    assert_eq!(
        hour.worst.as_deref(),
        Some("nobody has this"),
        "the worst is one nobody has taken up, never the void's: {hour:?}"
    );
}

#[test]
fn a_run_takes_up_and_closes_a_pain_from_its_thread_and_the_switch() {
    let place = Place::new("standing");
    place.routes();
    let messages = place.root.join("messages");
    fs::create_dir_all(messages.join("2026-11-16")).expect("a day folder");
    let said = |name: &str, text: serde_json::Value| {
        fs::write(messages.join("2026-11-16").join(name), text.to_string()).expect("a message");
    };
    let mut kept = pain("f7da717c", "i can't show the launcher in a call", 1);
    kept.to = Some("ABC123".into());
    kept.routed_at = Some(AT - 50_000);
    let mut other = pain("0badc0de", "the bar is the wrong colour", 1);
    other.to = Some("ABC123".into());
    other.routed_at = Some(AT - 50_000);
    let lines = [&kept, &other]
        .map(|h| serde_json::to_string(h).expect("a line") + "\n")
        .concat();
    fs::write(place.root.join("state/heard.jsonl"), lines).expect("kept");
    said(
        &format!("{}-EAR001-1-186eeb39.json", AT - 49_000),
        json!({"id": format!("{}-EAR001-1-186eeb39", AT - 49_000), "from": "EAR001",
               "to": ["@ABC123"], "text": "Joel hit something in mycelium:cell/ABC123#f7da717c"}),
    );
    said(
        &format!("{}-EAR001-1-286eeb39.json", AT - 48_000),
        json!({"id": format!("{}-EAR001-1-286eeb39", AT - 48_000), "from": "EAR001",
               "to": ["@ABC123"], "text": "Joel hit something in mycelium:cell/ABC123#0badc0de"}),
    );
    said(
        &format!("{}-ABC123-1-a85417d8.json", AT - 10_000),
        json!({"id": format!("{}-ABC123-1-a85417d8", AT - 10_000), "from": "ABC123",
               "to": ["#orchestration/x"], "re": "186eeb39",
               "text": "ABC123 landed core #5342, merged at 255494bb414c (12:38)"}),
    );
    said(
        &format!("{}-ABC123-1-b85417d8.json", AT - 9_000),
        json!({"id": format!("{}-ABC123-1-b85417d8", AT - 9_000), "from": "ABC123",
               "to": ["@EAR001"], "re": "286eeb39", "text": "on it, by noon"}),
    );
    fs::write(
        place.root.join("switches.jsonl"),
        r#"{"at":"x","generation":1,"revs":{"core":"cafe0123"}}"#,
    )
    .expect("a switch");
    // A git that says the landing's commit is in the core switched to,
    // and nothing else is.
    let git = place.root.join("git");
    fs::write(
        &git,
        format!(
            "#!{}\n[ \"$4 $5 $6\" = \"--is-ancestor 255494bb414c cafe0123\" ]\n",
            std::env::var("BASH").unwrap_or_else(|_| "/bin/sh".into())
        ),
    )
    .expect("a git");
    fs::set_permissions(&git, fs::Permissions::from_mode(0o755)).expect("made runnable");
    let mut ears = place.ears();
    ears.messages = messages;
    ears.switches = place.root.join("switches.jsonl");
    ears.core = place.root.join("core.git");
    ears.git = git;
    let hour = hear(&ears, CELLS, &mut |_, _| true).expect("a run");
    let heard = place.heard();
    let by = |entry: &str| heard.iter().find(|h| h.entry == entry).expect("kept");
    assert_eq!(
        by("f7da717c").closed_by.as_deref(),
        Some("255494bb414c"),
        "closed by the landing the switch carried"
    );
    assert_eq!(
        (
            by("0badc0de").taken_by.as_deref(),
            by("0badc0de").closed_by.as_deref()
        ),
        (Some("ABC123"), None),
        "taken up by its answer, and open still"
    );
    assert_eq!((hour.open, hour.taken, hour.closed), (0, 1, 1), "{hour:?}");
}
