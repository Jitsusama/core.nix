use disk_guard::band::Band;
use disk_guard::cli::{Command, Env, parse, run};
use disk_guard::units::{Size, Span};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn words(line: &str) -> Vec<OsString> {
    line.split_whitespace().map(OsString::from).collect()
}

#[test]
fn the_daemons_are_told_their_config_and_state() {
    assert_eq!(
        parse(&words("user --config /c.json --state /s")).unwrap(),
        Command::User {
            config: Some(PathBuf::from("/c.json")),
            state: Some(PathBuf::from("/s")),
        }
    );
    assert_eq!(
        parse(&words("system")).unwrap(),
        Command::System {
            config: None,
            state: None
        }
    );
}

#[test]
fn the_commands_that_answer_read_their_options() {
    assert_eq!(
        parse(&words("plan --band urgent --pretend-free 20GiB")).unwrap(),
        Command::Plan {
            config: None,
            band: Some(Band::Urgent),
            pretend_free: Some(Size::Bytes(20 << 30)),
        }
    );
    assert_eq!(
        parse(&words("journal --since 1d --json")).unwrap(),
        Command::Journal {
            since: Some(Span(Duration::from_secs(86400))),
            json: true,
        }
    );
    assert_eq!(
        parse(&words("status --json")).unwrap(),
        Command::Status { json: true }
    );
    assert_eq!(
        parse(&words("check-config /c.json")).unwrap(),
        Command::CheckConfig {
            file: PathBuf::from("/c.json")
        }
    );
    assert_eq!(parse(&words("thaw")).unwrap(), Command::Thaw);
}

#[test]
fn what_wont_parse_says_why() {
    for (line, said) in [
        ("", "usage"),
        ("frobnicate", "frobnicate"),
        ("user --colour", "--colour"),
        ("user --config", "--config"),
        ("plan --band bogus", "bogus"),
        ("plan --pretend-free lots", "lots"),
        ("check-config", "check-config"),
        ("status --json --json", "--json"),
    ] {
        let refused = parse(&words(line)).unwrap_err();
        assert!(refused.contains(said), "{line:?} said {refused:?}");
    }
}

struct Machine {
    _dir: tempfile::TempDir,
    root: PathBuf,
    env: Env,
}

impl Machine {
    fn new() -> Machine {
        let dir = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let env = Env {
            config: root.join("config/disk-guard/config.json"),
            state: root.join("state/disk-guard"),
            status: root.join("run/disk-guard/status.json"),
            system_config: root.join("etc/disk-guard/config.json"),
            system_state: root.join("var/lib/disk-guard"),
            system_status: root.join("system/status.json"),
            system_socket: root.join("system/socket"),
            processes: root.join("proc"),
        };
        fs::create_dir_all(&env.state).unwrap();
        fs::create_dir_all(&env.processes).unwrap();
        Machine {
            _dir: dir,
            root,
            env,
        }
    }

    fn run(&self, line: &str) -> (i32, String, String) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(parse(&words(line)).unwrap(), &self.env, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    fn config(&self, path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
}

const GOOD: &str = r#"{ "version": 1, "scope": "user",
  "sources": { "trash": { "kind": "trash", "path": "/home/u/.local/share/Trash" } } }"#;

#[test]
fn a_config_is_checked_and_a_bad_one_refused_with_its_reason() {
    let machine = Machine::new();
    let good = machine.root.join("good.json");
    machine.config(&good, GOOD);
    let (code, _, err) = machine.run(&format!("check-config {}", good.display()));
    assert_eq!((code, err.as_str()), (0, ""));
    let bad = machine.root.join("bad.json");
    machine.config(
        &bad,
        r#"{ "version": 1, "scope": "user", "colour": "red", "sources": {} }"#,
    );
    let (code, _, err) = machine.run(&format!("check-config {}", bad.display()));
    assert_eq!(code, 1);
    assert!(err.contains("colour"), "{err}");
    let (code, _, err) = machine.run(&format!(
        "check-config {}",
        machine.root.join("none.json").display()
    ));
    assert_eq!(code, 1);
    assert!(err.contains("none.json"), "{err}");
}

fn wall() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

fn status(at: f64, band: &str) -> String {
    format!(
        r#"{{ "version": 1, "heartbeat": {at},
             "filesystems": [ {{ "path": "/home", "band": "{band}", "free": 161061273600,
                                 "total": 1073741824000, "forecast": 7200.0 }} ],
             "last": {{ "at": {at}, "path": "/home", "band": "pressure", "end": "reached",
                        "freed": 4509715660 }} }}"#
    )
}

#[test]
fn status_tells_of_each_guard_that_wrote_one() {
    let machine = Machine::new();
    let (code, _, err) = machine.run("status");
    assert_eq!(code, 3);
    assert!(err.contains("no guard"), "{err}");

    machine.config(&machine.env.status, &status(wall() - 12.0, "pressure"));
    let (code, out, _) = machine.run("status");
    assert_eq!(code, 0);
    assert!(out.contains("user guard"), "{out}");
    assert!(out.contains("/home"), "{out}");
    assert!(out.contains("pressure"), "{out}");
    assert!(out.contains("150.0 GiB free of 1000.0 GiB"), "{out}");
    assert!(out.contains("urgent in 2.0 h"), "{out}");
    assert!(out.contains("freed 4.2 GiB"), "{out}");
    assert!(!out.contains("system guard"), "{out}");

    machine.config(
        &machine.env.system_status,
        &status(wall() - 600.0, "healthy"),
    );
    let (_, out, _) = machine.run("status");
    assert!(out.contains("system guard"), "{out}");
    assert!(out.contains("not heard from for 10 min"), "{out}");

    let (code, out, _) = machine.run("status --json");
    assert_eq!(code, 0);
    let both: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(both["user"]["filesystems"][0]["band"], "pressure");
    assert_eq!(both["system"]["filesystems"][0]["band"], "healthy");
}

#[test]
fn the_journal_is_read_oldest_first_and_since_when_asked() {
    let machine = Machine::new();
    let now = wall();
    let line = |at: f64, path: &str| {
        format!(
            r#"{{"event":"action","at":{at},"band":"pressure","path":"{path}","kind":"trash","size":1048576,"outcome":"reclaimed"}}"#
        )
    };
    fs::write(
        machine.env.state.join("journal.jsonl.1"),
        format!("{}\n", line(now - 3.0 * 86400.0, "/old")),
    )
    .unwrap();
    fs::write(
        machine.env.state.join("journal.jsonl"),
        format!(
            "{}\n{}\n",
            line(now - 7200.0, "/older"),
            line(now - 60.0, "/new")
        ),
    )
    .unwrap();
    let (code, out, _) = machine.run("journal");
    assert_eq!(code, 0);
    let order: Vec<_> = ["/old", "/older", "/new"]
        .iter()
        .map(|path| out.find(path).unwrap())
        .collect();
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{out}");
    assert!(out.contains("reclaimed"), "{out}");
    let (_, out, _) = machine.run("journal --since 1h --json");
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(out.contains("/new"));
    serde_json::from_str::<serde_json::Value>(out.trim()).unwrap();
}

#[test]
fn a_daemon_refuses_another_scopes_config_and_a_second_of_itself() {
    let machine = Machine::new();
    let system = machine.root.join("system.json");
    machine.config(
        &system,
        r#"{ "version": 1, "scope": "system", "sources": {} }"#,
    );
    let (code, _, err) = machine.run(&format!("user --config {}", system.display()));
    assert_eq!(code, 1);
    assert!(err.contains("system"), "{err}");

    machine.config(&machine.env.config, GOOD);
    let held = disk_guard::serve::hold(&machine.env.state).unwrap();
    assert!(held.is_some());
    let (code, _, err) = machine.run("user");
    assert_eq!(code, 2);
    assert!(err.contains("already"), "{err}");
}

#[test]
fn plan_shows_what_a_pass_would_take() {
    let machine = Machine::new();
    let trash = machine.root.join("Trash");
    fs::create_dir_all(trash.join("files")).unwrap();
    fs::create_dir_all(trash.join("info")).unwrap();
    fs::write(trash.join("files/old.txt"), "x".repeat(4096)).unwrap();
    fs::write(
        trash.join("info/old.txt.trashinfo"),
        "[Trash Info]\nPath=/x/old.txt\nDeletionDate=2001-09-09T01:46:40\n",
    )
    .unwrap();
    machine.config(
        &machine.env.config,
        &format!(
            r#"{{ "version": 1, "scope": "user",
                 "sources": {{ "trash": {{ "kind": "trash", "path": "{}" }} }} }}"#,
            trash.display()
        ),
    );
    let (code, out, err) = machine.run("plan --band pressure");
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("pressure"), "{out}");
    assert!(out.contains("old.txt"), "{out}");
    assert!(trash.join("files/old.txt").exists());
    assert!(
        !machine.env.state.join("journal.jsonl").exists(),
        "a plan journals nothing"
    );
}
