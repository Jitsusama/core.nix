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
        parse(&words("plan --band nearly-full --pretend-free 20GiB")).unwrap(),
        Command::Plan {
            config: None,
            band: Some(Band::NearlyFull),
            pretend_free: Some(Size::Bytes(20 << 30)),
            fresh: false,
        }
    );
    assert_eq!(
        parse(&words("plan --fresh")).unwrap(),
        Command::Plan {
            config: None,
            band: None,
            pretend_free: None,
            fresh: true,
        }
    );
    assert_eq!(
        parse(&words("journal --since 1d --json")).unwrap(),
        Command::Journal {
            since: Some(Span(Duration::from_hours(24))),
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
}

/// Writes `text` to `path`, making the directory it goes in.
fn write_file(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

const GOOD: &str = r#"{ "version": 1, "scope": "user",
  "sources": { "trash": { "kind": "trash", "path": "/home/u/.local/share/Trash" } } }"#;

#[test]
fn a_config_is_checked_and_a_bad_one_refused_with_its_reason() {
    let machine = Machine::new();
    let good = machine.root.join("good.json");
    write_file(&good, GOOD);
    let (code, _, err) = machine.run(&format!("check-config {}", good.display()));
    assert_eq!((code, err.as_str()), (0, ""));
    let bad = machine.root.join("bad.json");
    write_file(
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
             "last": {{ "at": {at}, "path": "/home", "band": "filling", "end": "reached",
                        "freed": 4509715660 }} }}"#
    )
}

#[test]
fn status_tells_of_each_guard_that_wrote_one() {
    let machine = Machine::new();
    let (code, _, err) = machine.run("status");
    assert_eq!(code, 3);
    assert!(err.contains("no guard"), "{err}");

    write_file(&machine.env.status, &status(wall() - 12.0, "filling"));
    let (code, out, _) = machine.run("status");
    assert_eq!(code, 0);
    assert!(out.contains("user guard"), "{out}");
    assert!(out.contains("/home"), "{out}");
    assert!(out.contains("filling"), "{out}");
    assert!(out.contains("150.0 GiB free of 1000.0 GiB"), "{out}");
    assert!(out.contains("nearly-full in 2.0 h"), "{out}");
    assert!(out.contains("freed 4.2 GiB"), "{out}");
    assert!(!out.contains("system guard"), "{out}");

    write_file(
        &machine.env.system_status,
        &status(wall() - 600.0, "healthy"),
    );
    let (_, out, _) = machine.run("status");
    assert!(out.contains("system guard"), "{out}");
    assert!(out.contains("not heard from for 10 min"), "{out}");

    let (code, out, _) = machine.run("status --json");
    assert_eq!(code, 0);
    let both: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(both["user"]["filesystems"][0]["band"], "filling");
    assert_eq!(both["system"]["filesystems"][0]["band"], "healthy");
}

#[test]
fn status_says_what_a_second_version_guard_says_in_words() {
    let machine = Machine::new();
    let at = wall() - 5.0;
    write_file(
        &machine.env.status,
        &format!(
            r#"{{ "version": 2, "heartbeat": {at},
                 "says": "The disk is filling, with 150.0 GiB free.",
                 "filesystems": [],
                 "cleared": [
                   {{ "at": {at}, "says": "Cleared a Rust build, ~/src/a/target (4.0 GiB)." }},
                   {{ "at": {at}, "says": "Cleared a cache, ~/.cache/b (1.0 GiB)." }},
                   {{ "at": {at}, "says": "Cleared a log, ~/c.log (2.0 MiB)." }},
                   {{ "at": {at}, "says": "Cleared a log, ~/d.log (2.0 MiB)." }}
                 ] }}"#
        ),
    );
    let (code, out, _) = machine.run("status");
    assert_eq!(code, 0);
    assert!(
        out.contains("  The disk is filling, with 150.0 GiB free.\n"),
        "{out}"
    );
    assert!(
        out.contains("  5 s ago: Cleared a Rust build, ~/src/a/target (4.0 GiB)."),
        "{out}"
    );
    assert!(out.contains("Cleared a log, ~/c.log"), "{out}");
    assert!(!out.contains("~/d.log"), "three at most: {out}");
}

// The holders were only in the JSON, so asking the guard in words what
// held the disk answered with what it had cleared, never what it hadn't.
#[test]
fn status_says_what_holds_the_most_in_words() {
    let machine = Machine::new();
    let at = wall() - 5.0;
    let holder = |says: &str| format!(r#"{{ "bytes": 1, "says": "{says}" }}"#);
    write_file(
        &machine.env.status,
        &format!(
            r#"{{ "version": 2, "heartbeat": {at}, "filesystems": [],
                 "holders_at": {at}, "holders": [ {}, {}, {}, {} ] }}"#,
            holder("The worktree a holds 40.0 GiB; a build is using it (process 7)."),
            holder("~/b holds 20.0 GiB, of which 2.0 GiB can be cleared now."),
            holder("~/c holds 10.0 GiB."),
            holder("~/d holds 5.0 GiB."),
        ),
    );
    let (code, out, _) = machine.run("status");
    assert_eq!(code, 0);
    let said = [
        "  holds the most:",
        "    The worktree a holds 40.0 GiB; a build is using it (process 7).",
        "    ~/b holds 20.0 GiB, of which 2.0 GiB can be cleared now.",
        "    ~/c holds 10.0 GiB.",
        "",
    ]
    .join("\n");
    assert!(out.contains(&said), "{out}");
    assert!(!out.contains("~/d"), "three at most: {out}");
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
            r#"{{"event":"action","at":{at},"band":"filling","path":"{path}","kind":"trash","size":1048576,"outcome":"reclaimed"}}"#
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
        r#"{{"event":"pass","at":{},"band":"nearly-full","end":"reached","freed":1073741824,"seconds":60.0}}"#,
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
        r#"{{"event":"action","at":{},"band":"filling","path":"/src/app/target","kind":"target","size":1048576,"outcome":"reclaimed"}}"#,
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
fn the_journal_shows_the_machines_guard_beside_the_accounts_by_time() {
    let machine = Machine::new();
    let now = wall();
    let line = |at: f64, path: &str| {
        format!(
            r#"{{"event":"action","at":{at},"band":"filling","path":"{path}","kind":"garbage","size":1048576,"outcome":"reclaimed"}}"#
        )
    };
    fs::write(
        machine.env.state.join("journal.jsonl"),
        format!("{}\n", line(now - 120.0, "/home/u/app/target")),
    )
    .unwrap();
    write_file(
        &machine.env.system_state.join("journal.jsonl"),
        &format!(
            "{}\n{}\n",
            line(now - 180.0, "/tmp/left"),
            line(now - 60.0, "/nix/store/abc-thing")
        ),
    );
    let (code, out, _) = machine.run("journal");
    assert_eq!(code, 0);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3, "{out}");
    for (said, (path, whose)) in lines.iter().zip([
        ("/tmp/left", "machine"),
        ("/home/u/app/target", "account"),
        ("/nix/store/abc-thing", "machine"),
    ]) {
        assert!(
            said.contains(path) && said.contains(whose),
            "expected {path} from the {whose}'s guard in its place, the journal said:\n{out}"
        );
    }
    let (_, json, _) = machine.run("journal --json --since 90s");
    let entry: serde_json::Value = serde_json::from_str(json.trim()).unwrap();
    assert_eq!(entry["guard"], "machine", "{json}");
    assert_eq!(entry["path"], "/nix/store/abc-thing", "{json}");
}

#[test]
fn a_machines_journal_it_cant_read_is_said_and_the_accounts_still_shown() {
    use std::os::unix::fs::PermissionsExt;
    let machine = Machine::new();
    let now = wall();
    fs::write(
        machine.env.state.join("journal.jsonl"),
        format!(
            r#"{{"event":"pass","at":{},"band":"filling","end":"reached","freed":0,"seconds":1.0}}"#,
            now - 60.0
        ) + "\n",
    )
    .unwrap();
    write_file(&machine.env.system_state.join("journal.jsonl"), "");
    fs::set_permissions(&machine.env.system_state, fs::Permissions::from_mode(0o000)).unwrap();
    let (code, out, err) = machine.run("journal");
    fs::set_permissions(&machine.env.system_state, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("pass reached"), "{out}");
    assert!(
        err.contains("machine's guard") && err.contains("var/lib/disk-guard"),
        "an unreadable machine's journal should be said, not passed over: {err}"
    );
}

#[test]
fn a_daemon_refuses_another_scopes_config_and_a_second_of_itself() {
    let machine = Machine::new();
    let system = machine.root.join("system.json");
    write_file(
        &system,
        r#"{ "version": 1, "scope": "system", "sources": {} }"#,
    );
    let (code, _, err) = machine.run(&format!("user --config {}", system.display()));
    assert_eq!(code, 1);
    assert!(err.contains("system"), "{err}");

    write_file(&machine.env.config, GOOD);
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
    write_file(
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
    write_file(
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
            "  filling: after 7.0 d idle",
            "  nearly-full: after 24.0 h idle",
            "  critically-full: at once",
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
    write_file(
        &machine.env.config,
        &format!(
            r#"{{ "version": 1, "scope": "user",
                 "sources": {{ "trash": {{ "kind": "trash", "path": "{}" }} }} }}"#,
            trash.display()
        ),
    );
    let (code, out, err) = machine.run("plan --band filling");
    assert_eq!(code, 0, "{err}");
    assert!(
        out.starts_with("Surveying every source"),
        "with no survey kept it says it walks first: {out}"
    );
    assert!(out.contains("filling"), "{out}");
    assert!(out.contains("old.txt"), "{out}");
    assert!(trash.join("files/old.txt").exists());
    assert!(
        !machine.env.state.join("journal.jsonl").exists(),
        "a plan journals nothing"
    );
}

#[test]
fn plan_reads_the_survey_the_daemon_kept() {
    use std::os::unix::fs::MetadataExt;
    let machine = Machine::new();
    let trash = machine.root.join("Trash");
    fs::create_dir_all(trash.join("files")).unwrap();
    write_file(
        &machine.env.config,
        &format!(
            r#"{{ "version": 1, "scope": "user",
                 "sources": {{ "trash": {{ "kind": "trash", "path": "{}" }} }} }}"#,
            trash.display()
        ),
    );
    let device = fs::metadata(&trash).unwrap().dev();
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
        - 600.0;
    let kept = serde_json::json!({
        "version": 1, "at": at,
        "offered": [{ "device": device, "candidate": {
            "path": trash.join("files/seen.txt"), "class": "Trash", "idle": 1e9,
            "size": 4096, "occupied": false, "spent": false, "checkout": null } }]
    });
    fs::create_dir_all(&machine.env.state).unwrap();
    write_file(&machine.env.state.join("survey.json"), &kept.to_string());
    let (code, out, err) = machine.run("plan --band filling");
    assert_eq!(code, 0, "{err}");
    assert!(
        out.starts_with("From the survey the daemon's last pass made, 10 min ago"),
        "{out}"
    );
    assert!(out.contains("seen.txt"), "what that survey saw: {out}");
    let (code, out, err) = machine.run("plan --band filling --fresh");
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("Surveying every source"), "{out}");
    assert!(
        !out.contains("seen.txt"),
        "walked afresh, it's not there: {out}"
    );
}

/// A status file with one holder, as the account's guard writes it.
fn holding(machine: &Machine, path: &Path) {
    let status = serde_json::json!({
        "version": 2, "heartbeat": 1.0, "says": "", "filesystems": [], "cleared": [],
        "last": null, "frozen": [], "writer": null, "notice": null, "holders_at": 1.0,
        "holders": [{
            "path": path, "bytes": 4096, "owner": { "kind": "other", "name": "photos" },
            "name": "~/photos", "clearable": true, "clears": 4096, "why": null, "kept": null,
            "says": "~/photos holds 4.0 KiB, of which 4.0 KiB can be cleared now."
        }]
    });
    write_file(&machine.env.status, &status.to_string());
}

fn holder(machine: &Machine) -> serde_json::Value {
    let status: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&machine.env.status).unwrap()).unwrap();
    status["holders"][0].clone()
}

#[test]
fn a_keep_holds_its_reason_and_shows_in_the_status_before_the_command_ends() {
    let machine = Machine::new();
    let photos = machine.root.join("photos");
    fs::create_dir_all(&photos).unwrap();
    holding(&machine, &photos);
    let (code, out, err) = machine.run(&format!(
        "keep {} the old camera's copies",
        photos.display()
    ));
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(
        out,
        format!("Kept {}: the old camera's copies.\n", photos.display())
    );
    let kept = holder(&machine);
    assert_eq!(kept["kept"]["reason"], "the old camera's copies", "{kept}");
    assert!(
        kept["says"]
            .as_str()
            .unwrap()
            .ends_with("; you keep it: the old camera's copies."),
        "{kept}"
    );
    let (code, out, _) = machine.run(&format!("unkeep {}", photos.display()));
    assert_eq!(
        (code, out),
        (0, format!("No longer kept: {}.\n", photos.display()))
    );
    let let_go = holder(&machine);
    assert!(let_go["kept"].is_null(), "{let_go}");
    assert!(
        let_go["says"]
            .as_str()
            .unwrap()
            .ends_with("of which 4.0 KiB can be cleared now."),
        "{let_go}"
    );
}

#[test]
fn a_clear_shows_as_clearing_before_the_command_ends() {
    let machine = Machine::new();
    let photos = machine.root.join("photos");
    fs::create_dir_all(&photos).unwrap();
    holding(&machine, &photos);
    let (code, out, err) = machine.run(&format!("clear {}", photos.display()));
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(
        out,
        format!(
            "Clearing {}; the guard starts on its next step.\n",
            photos.display()
        )
    );
    assert_eq!(holder(&machine)["clearing"], true);
    assert_eq!(
        disk_guard::clears::clearing(&machine.env.state),
        [photos.canonicalize().unwrap()]
    );
}

#[test]
fn a_clear_refuses_what_is_kept_or_not_there() {
    let machine = Machine::new();
    let photos = machine.root.join("photos");
    fs::create_dir_all(photos.join("2019")).unwrap();
    let (code, _, _) = machine.run(&format!("keep {} the only copies", photos.display()));
    assert_eq!(code, 0);
    let (code, _, err) = machine.run(&format!("clear {}", photos.join("2019").display()));
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("you keep it: the only copies"), "{err}");
    let gone = machine.root.join("gone");
    let (code, _, err) = machine.run(&format!("clear {}", gone.display()));
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("isn't there"), "{err}");
    assert_eq!(
        disk_guard::clears::clearing(&machine.env.state),
        Vec::<PathBuf>::new(),
        "nothing refused is asked"
    );
}

#[test]
fn a_keep_needs_a_folder_there_and_a_reason() {
    let machine = Machine::new();
    let gone = machine.root.join("gone");
    let (code, _, err) = machine.run(&format!("keep {} a reason", gone.display()));
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("isn't there"), "{err}");
    let refused = parse(&words(&format!("keep {}", machine.root.display()))).unwrap_err();
    assert!(refused.contains("needs a reason"), "{refused}");
    let (code, out, _) = machine.run(&format!("unkeep {}", machine.root.display()));
    assert_eq!(
        (code, out),
        (0, format!("{} wasn't kept.\n", machine.root.display()))
    );
}

/// The machine with its guard's socket at a path short enough to bind,
/// which a socket's address allows only 108 bytes of, in a folder the
/// caller holds.
fn with_short_socket() -> (Machine, tempfile::TempDir) {
    let mut machine = Machine::new();
    let short = tempfile::tempdir().unwrap();
    machine.env.system_socket = short.path().join("s");
    (machine, short)
}

#[test]
fn a_footprint_without_the_machines_guard_says_none_is_there() {
    let (machine, _short) = with_short_socket();
    let (code, out, err) = machine.run(&format!("footprint {}", machine.root.display()));
    assert_eq!((code, out.as_str()), (3, ""));
    assert!(err.contains("asking the machine's guard"), "{err}");
}

#[test]
fn a_footprint_says_what_the_disk_holds_beside_what_the_files_do() {
    let (machine, _short) = with_short_socket();
    let files = machine.root.join("files");
    write_file(&files.join("big"), &"x".repeat(8192));
    let mut server =
        disk_guard::socket::Server::bind(&machine.env.system_socket, Duration::ZERO).unwrap();
    std::thread::spawn(move || {
        server
            .serve(&mut |_, _, responder| {
                responder.send(&disk_guard::socket::Reply::Footprints(vec![
                    Some(4096),
                    None,
                ]));
            })
            .unwrap();
    });
    let gone = machine.root.join("gone");
    let (code, out, _) = machine.run(&format!("footprint {} {}", files.display(), gone.display()));
    assert_eq!(code, 1, "a path it couldn't read is a refusal: {out}");
    assert_eq!(
        out,
        format!(
            "{}: 4.0 KiB on the disk, for 8.0 KiB of files.\n\
             {}: 0 B of files; what they take on the disk can't be read \
             (not yours, not on Btrfs, or gone).\n",
            files.display(),
            gone.display()
        )
    );
}

#[test]
fn a_footprint_asked_too_soon_is_asked_again() {
    use disk_guard::socket::{Reply, Server, TOO_SOON};
    let (machine, _short) = with_short_socket();
    let mut server = Server::bind(&machine.env.system_socket, Duration::ZERO).unwrap();
    std::thread::spawn(move || {
        // The account asked a moment ago, as a guard's plan beside its
        // daemon does, so the first ask is refused and the next answered.
        let mut replies = [
            Reply::Error(TOO_SOON.to_owned()),
            Reply::Footprints(vec![Some(4096)]),
        ]
        .into_iter();
        for _ in 0..2 {
            let reply = replies.next().unwrap();
            server
                .serve(&mut |_, _, responder| responder.send(&reply))
                .unwrap();
        }
    });
    let (code, out, err) = machine.run(&format!("footprint {}", machine.root.display()));
    assert_eq!((code, err.as_str()), (0, ""), "{out}");
    assert!(out.contains("4.0 KiB on the disk"), "{out}");
}
