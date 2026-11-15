//! cadence-spec over its own socket, as the launcher's Spec asks it, with
//! a stand-in for cadence: one printing a row and a headline, which a
//! watch of Spec is served, and one failing, which leaves Spec with none
//! and the daemon answering.

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use daemon_client::Daemon;
use serde_json::{Value, json};

/// cadence-spec started in a folder of its own, over a stand-in cadence
/// running `body`, which notes what it was asked in `asked`.
fn started(name: &str, body: &str) -> Daemon {
    let folder = std::env::temp_dir().join(format!("cadence-spec-{name}-{}", std::process::id()));
    fs::create_dir_all(&folder).expect("a folder");
    let cadence = folder.join("cadence");
    fs::write(
        &cadence,
        format!(
            "#!{}\necho \"$*\" > {}\n{body}\n",
            stand_in_shell().display(),
            folder.join("asked").display()
        ),
    )
    .expect("a stand-in cadence");
    fs::set_permissions(&cadence, fs::Permissions::from_mode(0o755)).expect("it runs");
    let socket = folder.join("cadence-spec.sock");
    Daemon::start(
        Command::new(env!("CARGO_BIN_EXE_cadence-spec"))
            .arg(&socket)
            .arg(&cadence)
            .arg("words.json"),
        socket,
        folder,
    )
}

/// What a watch of Spec holds once its headline has come, by key.
fn watched(daemon: &Daemon) -> BTreeMap<String, Value> {
    let mut client = daemon.client();
    client.ask(&json!({"id": 1, "ask": "hello"}));
    client.send(&json!({"id": 2, "ask": "watch", "kind": ["row", "head"], "place": "spec"}));
    let mut held = BTreeMap::new();
    while !held.contains_key("head:spec") {
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

/// What the stand-in cadence was last asked, or nothing yet.
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
    let held = watched(&daemon);
    assert_eq!(held["head:spec"]["kind"], "head", "{held:?}");
    assert_eq!(
        held["head:spec"]["head"]["said"], "1 reading out of spec",
        "{held:?}"
    );
    assert_eq!(held["row:spec:scale"]["row"]["name"], "Scale", "{held:?}");
    assert_eq!(asked(daemon.folder()).trim(), "axes --served words.json");
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
