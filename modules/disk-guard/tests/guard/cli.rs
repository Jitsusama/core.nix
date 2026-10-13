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
    assert_eq!(
        parse(&words("explain /home/u/src/app/target")).unwrap(),
        Command::Explain {
            path: PathBuf::from("/home/u/src/app/target"),
            config: None
        }
    );
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
fn each_platform_keeps_the_guard_where_it_keeps_such_things() {
    use std::path::PathBuf;
    let environment = |pairs: &'static [(&str, &str)]| {
        move |name: &str| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| PathBuf::from(value))
        }
    };
    let linux = Env::for_os("linux", environment(&[("HOME", "/home/u")]), 1000);
    assert_eq!(
        linux.config,
        Path::new("/home/u/.config/disk-guard/config.json")
    );
    assert_eq!(linux.state, Path::new("/home/u/.local/state/disk-guard"));
    assert_eq!(
        linux.status,
        Path::new("/run/user/1000/disk-guard/status.json")
    );
    assert_eq!(linux.system_state, Path::new("/var/lib/disk-guard"));
    assert_eq!(linux.system_socket, Path::new("/run/disk-guard/socket"));
    assert_eq!(linux.processes, Path::new("/proc"));

    let mac = Env::for_os(
        "macos",
        environment(&[("HOME", "/Users/u"), ("TMPDIR", "/var/folders/xy/T/")]),
        501,
    );
    assert_eq!(
        mac.config,
        Path::new("/Users/u/.config/disk-guard/config.json")
    );
    assert_eq!(
        mac.status,
        Path::new("/var/folders/xy/T/disk-guard/status.json")
    );
    assert_eq!(mac.system_config, Path::new("/etc/disk-guard/config.json"));
    assert_eq!(mac.system_state, Path::new("/var/db/disk-guard"));
    assert_eq!(
        mac.system_status,
        Path::new("/var/run/disk-guard/status.json")
    );
    assert_eq!(mac.system_socket, Path::new("/var/run/disk-guard/socket"));
    let bare = Env::for_os("macos", environment(&[("HOME", "/Users/u")]), 501);
    assert_eq!(
        bare.status,
        Path::new("/Users/u/Library/Caches/disk-guard/status.json")
    );
    let set = Env::for_os(
        "macos",
        environment(&[("HOME", "/Users/u"), ("XDG_RUNTIME_DIR", "/run/u")]),
        501,
    );
    assert_eq!(set.status, Path::new("/run/u/disk-guard/status.json"));
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
    // A pass says how it ended and how long it took.
    let pass = format!(
        r#"{{"event":"pass","at":{},"band":"urgent","end":"reached","freed":1073741824,"seconds":60.0}}"#,
        now - 30.0
    );
    fs::write(machine.env.state.join("journal.jsonl"), format!("{pass}\n")).unwrap();
    let (_, said, _) = machine.run("journal");
    assert!(
        said.contains("pass reached, freed 1.0 GiB in 60 s"),
        "{said}"
    );
    assert!(out.contains("/new"));
    serde_json::from_str::<serde_json::Value>(out.trim()).unwrap();
}

#[test]
fn the_journal_leaves_out_what_a_reclaim_began_with_but_keeps_it_as_json() {
    let machine = Machine::new();
    let now = wall();
    let begin = format!(
        r#"{{"event":"begin","at":{},"path":"/src/app/target","root":"/src"}}"#,
        now - 60.0
    );
    let action = format!(
        r#"{{"event":"action","at":{},"band":"pressure","path":"/src/app/target","kind":"target","size":1048576,"outcome":"reclaimed"}}"#,
        now - 60.0
    );
    fs::write(
        machine.env.state.join("journal.jsonl"),
        format!("{begin}\n{action}\n"),
    )
    .unwrap();
    let (code, out, _) = machine.run("journal");
    assert_eq!(code, 0);
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(out.contains("reclaimed") && !out.contains("begin"), "{out}");
    let (_, json, _) = machine.run("journal --json");
    assert_eq!(json.lines().count(), 2, "{json}");
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
fn explain_says_why_the_system_daemon_could_not_vouch_for_a_process() {
    let machine = Machine::new();
    let scratch = machine.root.join("scratch");
    fs::create_dir_all(scratch.join("run-1")).unwrap();
    machine.config(
        &machine.env.config,
        &format!(
            r#"{{ "version": 1, "scope": "user",
                 "sources": {{ "scratch": {{ "kind": "scratch", "path": "{}" }} }} }}"#,
            scratch.display()
        ),
    );
    // Root's, so the account can't read it, with no system daemon to ask.
    let proc = &machine.env.processes;
    crate::procs::text(proc, "300/stat", &crate::procs::stat(300, "rustc", 0, 77));
    crate::procs::text(proc, "300/status", "Name:\trustc\nUid:\t0\t0\t0\t0\n");
    crate::procs::text(proc, "300/cmdline", "");
    let run = scratch.join("run-1");
    let (code, out, err) = machine.run(&format!("explain {}", run.display()));
    assert_eq!(code, 0, "{err}");
    let said = out
        .lines()
        .find(|line| line.starts_with("  in use: "))
        .unwrap_or_default();
    assert!(
        said.starts_with(
            "  in use: maybe: process 300 can't be read, and the system daemon couldn't be asked ("
        ),
        "{out}"
    );
}

#[test]
fn explain_says_how_each_band_would_treat_a_path() {
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
    let old = trash.join("files/old.txt");
    let (code, out, err) = machine.run(&format!("explain {}", old.display()));
    assert_eq!(code, 0, "{err}");
    let lines: Vec<&str> = out.lines().collect();
    assert!(
        lines[0].starts_with(&format!("{}: trash, 4.0 KiB, idle ", old.display())),
        "{out}"
    );
    assert_eq!(
        &lines[1..],
        [
            "  in use: by nothing",
            "  healthy: after 30.0 d idle",
            "  pressure: after 7.0 d idle",
            "  urgent: after 24.0 h idle",
            "  critical: at once",
        ],
        "{out}"
    );
    // A file inside what's offered is told of by what holds it.
    fs::create_dir_all(trash.join("files/old-dir")).unwrap();
    fs::write(trash.join("files/old-dir/inside.txt"), "x").unwrap();
    fs::write(
        trash.join("info/old-dir.trashinfo"),
        "[Trash Info]\nPath=/x/old-dir\nDeletionDate=2001-09-09T01:46:40\n",
    )
    .unwrap();
    let inside = trash.join("files/old-dir/inside.txt");
    let (_, out, _) = machine.run(&format!("explain {}", inside.display()));
    let held = format!("{}: trash, ", trash.join("files/old-dir").display());
    assert!(out.starts_with(&held), "{out}");
    let (code, out, _) = machine.run("explain /nowhere/at/all");
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "No source offers /nowhere/at/all or anything holding it, so the guard leaves it be.\n"
    );
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
