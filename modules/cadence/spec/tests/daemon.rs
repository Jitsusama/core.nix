//! cadence-spec over its own socket, as the launcher's Spec asks it, with
//! a stand-in for cadence: one printing a row and a headline, which a
//! watch of Spec is served, and one failing, which leaves Spec with none
//! and the daemon answering. Asked For is watched and served the same.

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use daemon_client::Daemon;
use serde_json::{Value, json};

/// cadence-spec started in a folder of its own, over a stand-in cadence
/// running `body`, which notes what it was asked in `asked`, with the
/// launcher's words as cadence's tests hold them, so a headline it makes
/// of them is served as the launcher's would be (round 6 on #6875, F108).
fn started(name: &str, body: &str) -> Daemon {
    let folder = std::env::temp_dir().join(format!("cadence-spec-{name}-{}", std::process::id()));
    fs::create_dir_all(&folder).expect("a folder");
    let cadence = folder.join("cadence");
    fs::write(
        &cadence,
        format!(
            "#!{}\necho \"$*\" >> {}\n{body}\n",
            stand_in_shell().display(),
            folder.join("asked").display()
        ),
    )
    .expect("a stand-in cadence");
    fs::set_permissions(&cadence, fs::Permissions::from_mode(0o755)).expect("it runs");
    fs::write(
        folder.join("words.json"),
        include_str!("../../tests/words.json"),
    )
    .expect("its words");
    let socket = folder.join("cadence-spec.sock");
    Daemon::start(
        Command::new(env!("CARGO_BIN_EXE_cadence-spec"))
            .current_dir(&folder)
            .arg(&socket)
            .arg(&cadence)
            .arg("words.json"),
        socket,
        folder,
    )
}

/// What a watch of `place` holds once its headline has come, by key.
fn watched(daemon: &Daemon, place: &str) -> BTreeMap<String, Value> {
    watched_all(daemon, &[place])
}

/// What one client watching each of `places` holds once every one's
/// headline has come, by key.
fn watched_all(daemon: &Daemon, places: &[&str]) -> BTreeMap<String, Value> {
    let mut client = daemon.client();
    client.ask(&json!({"id": 1, "ask": "hello"}));
    for (id, place) in (2..).zip(places) {
        client.send(&json!({"id": id, "ask": "watch", "kind": ["row", "head"], "place": place}));
    }
    let mut held = BTreeMap::new();
    while !places
        .iter()
        .all(|place| held.contains_key(&format!("head:{place}")))
    {
        let said = client.heard();
        for record in said["records"]
            .as_array()
            .into_iter()
            .chain(said["changed"].as_array())
            .flatten()
        {
            let key = record["key"].as_str().unwrap_or("").to_owned();
            held.insert(key, record.clone());
        }
    }
    held
}

/// The shell the stand-in runs in, from the test's PATH, since a build's
/// sandbox may have none at /bin.
fn stand_in_shell() -> PathBuf {
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .map(|folder| folder.join("sh"))
        .find(|found| found.is_file())
        .unwrap_or_else(|| PathBuf::from("/bin/sh"))
}

/// Everything the stand-in cadence was asked, a line each, or nothing yet.
fn asked(folder: &Path) -> String {
    fs::read_to_string(folder.join("asked")).unwrap_or_default()
}

#[test]
fn a_watch_of_spec_is_served_the_rows_cadence_makes() {
    let printed = json!({
        "rows": [{"key": "row:spec:scale", "place": "spec", "name": "Scale"}],
        "head": {"key": "head:spec", "place": "spec", "said": "1 reading out of spec"},
    });
    let daemon = started("served", &format!("echo '{printed}'"));
    let held = watched(&daemon, "spec");
    assert_eq!(held["head:spec"]["kind"], "head", "{held:?}");
    assert_eq!(
        held["head:spec"]["head"]["said"], "1 reading out of spec",
        "{held:?}"
    );
    assert_eq!(held["row:spec:scale"]["row"]["name"], "Scale", "{held:?}");
    assert_eq!(asked(daemon.folder()).trim(), "axes --served words.json");
}

#[test]
fn a_watch_of_asked_for_is_served_the_rows_cadence_makes_for_it() {
    let printed = json!({
        "rows": [{"key": "row:asked:be66afc9", "place": "asked", "name": "the split"}],
        "head": {"key": "head:asked", "place": "asked", "said": "1 late of 3"},
    });
    let daemon = started("asked", &format!("echo '{printed}'"));
    let held = watched(&daemon, "asked");
    assert_eq!(
        held["head:asked"]["head"]["said"], "1 late of 3",
        "{held:?}"
    );
    assert_eq!(
        held["row:asked:be66afc9"]["row"]["name"], "the split",
        "{held:?}"
    );
    assert_eq!(
        asked(daemon.folder()).trim(),
        "axes --asked words.json",
        "only the place watched is asked for, and once"
    );
    assert!(
        !held.contains_key("head:spec"),
        "no Spec unwatched: {held:?}"
    );
}

#[test]
fn both_places_watched_at_once_are_each_served_their_own() {
    let spec = json!({
        "rows": [{"key": "row:spec:scale", "place": "spec", "name": "Scale"}],
        "head": {"key": "head:spec", "place": "spec", "said": "all in spec"},
    });
    let asked_for = json!({
        "rows": [{"key": "row:asked:be66afc9", "place": "asked", "name": "the split"}],
        "head": {"key": "head:asked", "place": "asked", "said": "1 open"},
    });
    let body = format!("if [ \"$2\" = --served ]; then echo '{spec}'; else echo '{asked_for}'; fi");
    let daemon = started("both", &body);
    let held = watched_all(&daemon, &["spec", "asked"]);
    assert_eq!(held["row:spec:scale"]["row"]["name"], "Scale", "{held:?}");
    assert_eq!(
        held["row:asked:be66afc9"]["row"]["name"], "the split",
        "{held:?}"
    );
    assert_eq!(held["head:spec"]["head"]["said"], "all in spec");
    assert_eq!(held["head:asked"]["head"]["said"], "1 open");
    // In either order, as each watch may wake it before the other comes.
    let mut asks: Vec<String> = asked(daemon.folder()).lines().map(str::to_owned).collect();
    asks.sort();
    assert_eq!(
        asks,
        ["axes --asked words.json", "axes --served words.json"],
        "each read once, neither's rows dropping the other's"
    );
}

#[test]
fn asked_for_read_wrong_is_headed_unread_in_its_words_and_spec_serves_nothing() {
    let daemon = started(
        "unread",
        "echo 'cadence: the asks list is unread' >&2; exit 1",
    );
    let held = watched(&daemon, "asked");
    assert_eq!(
        held["head:asked"]["head"],
        json!({"key": "head:asked", "place": "asked",
            "said": "can't read the list now", "glyph": "failed"}),
        "Asked For stops waiting, told in the launcher's words: {held:?}"
    );
    let mut client = daemon.client();
    client.ask(&json!({"id": 1, "ask": "hello"}));
    let watch =
        client.ask(&json!({"id": 2, "ask": "watch", "kind": ["row", "head"], "place": "spec"}));
    daemon_client::until("Spec was never read", || {
        asked(daemon.folder()).contains("--served")
    });
    let again =
        client.ask(&json!({"id": 3, "ask": "watch", "kind": ["row", "head"], "place": "spec"}));
    for said in [&watch, &again] {
        assert!(
            said["records"].as_array().is_none_or(Vec::is_empty),
            "Spec's words say no unread headline, so it serves none: {said}"
        );
    }
}

#[test]
fn a_failed_read_serves_nothing_and_the_daemon_keeps_answering() {
    let daemon = started(
        "failed",
        "echo 'cadence: the axes read nothing' >&2; exit 1",
    );
    let mut client = daemon.client();
    client.ask(&json!({"id": 1, "ask": "hello"}));
    let watch =
        client.ask(&json!({"id": 2, "ask": "watch", "kind": ["row", "head"], "place": "spec"}));
    assert!(
        watch["records"].as_array().is_none_or(Vec::is_empty),
        "nothing to serve yet: {watch}"
    );
    daemon_client::until("cadence was never asked", || {
        !asked(daemon.folder()).is_empty()
    });
    let again = client.ask(&json!({"id": 3, "ask": "hello"}));
    assert!(again.is_object(), "the daemon still answers: {again}");
}
