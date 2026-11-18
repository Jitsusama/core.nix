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

/// Writes a script aside and copies it in, so no thread here holds it
/// open for writing when another test forks: a script run then is refused
/// as busy (ETXTBSY), as nix-offload's tests found on 31 October.
fn runnable(path: &Path, script: &str) {
    let draft = path.with_extension("draft");
    fs::write(&draft, script).expect("a draft");
    let copied = Command::new("cp")
        .arg(&draft)
        .arg(path)
        .status()
        .expect("cp ran");
    assert!(copied.success(), "copied {}", path.display());
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("made runnable");
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

/// A machine switched to core cafe0123, and a git that says only commit
/// 255494bb414c is in it, noting in the file it gives back each commit
/// it's asked of.
fn a_machine(place: &Place) -> (PathBuf, PathBuf) {
    fs::write(
        place.root.join("switches.jsonl"),
        r#"{"at":"x","generation":1,"revs":{"core":"cafe0123"}}"#,
    )
    .expect("a switch");
    fs::create_dir_all(place.root.join("core.git")).expect("a core");
    let git = place.root.join("git");
    let asks = place.root.join("asks");
    runnable(
        &git,
        &format!(
            "#!{}\necho \"$5\" >> {}\n[ \"$4 $5 $6\" = \"--is-ancestor 255494bb414c cafe0123\" ]\n",
            std::env::var("BASH").unwrap_or_else(|_| "/bin/sh".into()),
            asks.display()
        ),
    );
    (git, asks)
}

/// Three pains sent to ABC123 a little before the run, kept as heard.
fn three_pains(place: &Place) -> Vec<Heard> {
    let mut kept = Vec::new();
    for (n, words) in [
        "i can't show the launcher in a call",
        "the bar is the wrong colour",
        "the clock is an hour out",
    ]
    .iter()
    .enumerate()
    {
        let mut h = pain(&format!("e{}", n + 1), words, 1);
        h.session = "s-live".into();
        h.to = Some("ABC123".into());
        h.routed_at = Some(AT - 50_000);
        kept.push(h);
    }
    let lines = kept
        .iter()
        .map(|h| serde_json::to_string(h).expect("a line") + "\n")
        .collect::<String>();
    fs::write(place.root.join("state/heard.jsonl"), lines).expect("kept");
    kept
}

#[test]
fn a_run_takes_up_and_closes_a_pain_from_its_thread_and_the_switch() {
    let place = Place::new("standing");
    place.routes();
    let messages = place.root.join("messages");
    fs::create_dir_all(messages.join("2026-11-16")).expect("a day folder");
    let cells = route::cells(CELLS);
    let kept = three_pains(&place);
    // Each pain's question as the run sends it, and the thread after it:
    // the first landed on the machine; the other two landed on a commit
    // it doesn't run, which git is asked of once.
    let said = |ago: i64, from: &str, short: &str, to: &str, re: &str, text: &str| {
        let id = format!("{}-{from}-1-{short}", AT - ago);
        let message = json!({"id": id, "from": from, "to": [to], "re": re, "text": text});
        let at = messages.join("2026-11-16").join(format!("{id}.json"));
        fs::write(at, message.to_string()).expect("a message");
    };
    let ago = |n: usize| 49_000 - i64::try_from(n).expect("a few") * 1000;
    for (n, h) in kept.iter().enumerate() {
        let question = route::finding(&[h], &cells, 0);
        said(
            ago(n),
            "EAR001",
            &format!("{n}86eeb39"),
            "@ABC123",
            "",
            &question,
        );
    }
    let asked = |n: usize| format!("{}-EAR001-1-{n}86eeb39", AT - ago(n));
    let group = "#orchestration/x";
    for (ago, short, to, re, text) in [
        (
            10_000,
            "a85417d8",
            group,
            0,
            "landed core #5342, merged at 255494bb414c (12:38)",
        ),
        (9_000, "b85417d8", "@EAR001", 1, "on it, by noon"),
        (
            8_000,
            "c85417d8",
            group,
            1,
            "landed core #5343, merged at 1111111abcd",
        ),
        (
            7_000,
            "d85417d8",
            group,
            2,
            "landed core #5343, merged at 1111111abcd",
        ),
    ] {
        said(ago, "ABC123", short, to, &asked(re), text);
    }
    let (git, asks) = a_machine(&place);
    let mut ears = place.ears();
    ears.git = git;
    let hour = hear(&ears, CELLS, &mut |_, _| true).expect("a run");
    let heard = place.heard();
    let by = |entry: &str| {
        let h = heard.iter().find(|h| h.entry == entry).expect("kept");
        (h.taken_by.as_deref(), h.closed_by.as_deref())
    };
    assert_eq!(
        by("e1"),
        (Some("ABC123"), Some("255494bb414c")),
        "closed by the landing the switch carried"
    );
    assert_eq!(by("e2"), (Some("ABC123"), None), "taken, its fix not on");
    assert_eq!(by("e3"), (Some("ABC123"), None), "taken, its fix not on");
    assert_eq!((hour.open, hour.taken, hour.closed), (0, 2, 1), "{hour:?}");
    assert_eq!(hour.unclosed, None, "it could ask: {hour:?}");
    let asked_of = fs::read_to_string(&asks).expect("git was asked");
    let mut commits: Vec<&str> = asked_of.lines().collect();
    commits.sort_unstable();
    assert_eq!(
        commits,
        ["1111111abcd", "255494bb414c"],
        "each commit asked of once"
    );
    // With its core gone, nothing more closes, and the run says why; what
    // closed stays closed.
    fs::remove_dir_all(place.root.join("core.git")).expect("the core gone");
    let hour = hear(&ears, CELLS, &mut |_, _| true).expect("another run");
    let heard = place.heard();
    let first = heard.iter().find(|h| h.entry == "e1").expect("kept");
    assert_eq!(
        first.closed_by.as_deref(),
        Some("255494bb414c"),
        "still closed"
    );
    assert!(
        hour.unclosed
            .as_deref()
            .is_some_and(|why| why.starts_with("no core is at")),
        "{hour:?}"
    );
}
