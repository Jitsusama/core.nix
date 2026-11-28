//! shell-preview run as a person runs it, with stand-ins first on its path:
//! git names a working copy and its tracked shell, nix describes a check's
//! machine as a built home in a folder the test writes, and
//! dbus-run-session stands in for the nested desktop, a shell and a sleep
//! it started, which ends at once when told to.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const GIT: &str = r#"#!/bin/sh
case "$1" in
  rev-parse) echo "$STAND_IN/repo" ;;
  -C) printf 'modules/quickshell/Launcher.qml\0' ;;
esac
"#;

const NIX: &str = r#"#!/bin/sh
case "$1 $2" in
  "eval --raw") echo x86_64-linux ;;
  "eval --json") cat "$STAND_IN/machine.json" ;;
esac
"#;

/// The nested desktop: it notes what it was given and its sleep's pid, and
/// passes no signal on to the sleep, as dbus-run-session passes none to
/// niri. Told to close at once, it ends with the code it's told, or 0.
const DBUS_RUN_SESSION: &str = r#"#!/bin/sh
echo "$@" > "$STAND_IN/started"
if [ -e "$STAND_IN/close-at-once" ]; then
  code=$(cat "$STAND_IN/close-at-once")
  exit "${code:-0}"
fi
sleep 300 &
echo $! > "$STAND_IN/desktop"
wait
"#;

/// A folder holding the stand-ins, the working copy and the built home.
struct StandIn {
    folder: PathBuf,
}

impl StandIn {
    fn new(test: &str) -> Self {
        let folder =
            std::env::temp_dir().join(format!("shell-preview-{}-{test}", std::process::id()));
        if let Err(error) = fs::remove_dir_all(&folder) {
            assert_eq!(
                error.kind(),
                std::io::ErrorKind::NotFound,
                "{} should be cleared: {error}",
                folder.display()
            );
        }
        // Made afresh, so a folder planted there fails the test rather
        // than hold the stand-ins it runs.
        fs::create_dir(&folder)
            .unwrap_or_else(|error| panic!("{} should be made afresh: {error}", folder.display()));
        let stand_in = Self { folder };
        for (name, script) in [
            ("git", GIT),
            ("nix", NIX),
            ("dbus-run-session", DBUS_RUN_SESSION),
        ] {
            let path = stand_in.write(&format!("bin/{name}"), script);
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("it runs");
        }
        stand_in.write("repo/modules/quickshell/Launcher.qml", "Item {}\n");
        stand_in.write("built/home/.config/quickshell/Launcher.qml", "Item {}\n");
        stand_in.write("built/home/.config/quickshell/Theme.qml", "QtObject {}\n");
        stand_in.write("built/home/.config/niri/config.kdl", "binds {}\n");
        fs::create_dir_all(stand_in.at("session")).expect("the session's runtime folder");
        let built = stand_in.at("built");
        stand_in.write(
            "machine.json",
            &serde_json::json!({
                "home": built.join("home"),
                "niri": built.join("niri"),
                "quickshell": built.join("quickshell/bin/quickshell"),
                "data": [built.join("system")],
            })
            .to_string(),
        );
        stand_in
    }

    fn at(&self, path: &str) -> PathBuf {
        self.folder.join(path)
    }

    fn write(&self, path: &str, text: &str) -> PathBuf {
        let path = self.at(path);
        fs::create_dir_all(path.parent().expect("a folder")).expect("its folder");
        fs::write(&path, text).expect("written");
        path
    }

    fn start(&self) -> Child {
        let path = format!(
            "{}:{}",
            self.at("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        Command::new(env!("CARGO_BIN_EXE_shell-preview"))
            .env("PATH", path)
            .env("STAND_IN", &self.folder)
            .env("XDG_RUNTIME_DIR", self.at("session"))
            .env("WAYLAND_DISPLAY", "wayland-1")
            .stderr(Stdio::piped())
            .spawn()
            .expect("shell-preview starts")
    }

    /// Wait until the stand-in desktop has started its sleep, and give the
    /// sleep's pid.
    fn desktop(&self) -> i32 {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Ok(pid) = fs::read_to_string(self.at("desktop"))
                && let Ok(pid) = pid.trim().parse()
            {
                return pid;
            }
            assert!(Instant::now() < deadline, "the desktop never started");
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// Give the folder the preview made for itself, from niri's config it
    /// started the desktop with.
    fn own(&self) -> PathBuf {
        let started = fs::read_to_string(self.at("started")).expect("the desktop was started");
        let config = started
            .split_whitespace()
            .last()
            .map(PathBuf::from)
            .expect("the config it was given");
        config
            .ancestors()
            .nth(2)
            .expect("the preview's folder")
            .to_path_buf()
    }
}

/// Say whether a process is still running.
fn running(pid: i32) -> bool {
    // SAFETY: signal 0 only asks whether the process can be signalled.
    unsafe { libc::kill(pid, 0) == 0 }
}

#[test]
fn a_stop_ends_the_preview_and_the_desktop_it_started() {
    let stand_in = StandIn::new("stopped");
    let mut preview = stand_in.start();
    let sleep = stand_in.desktop();
    let own = stand_in.own();
    let link = fs::read_link(own.join("quickshell/Launcher.qml")).expect("a link");
    assert_eq!(link, stand_in.at("repo/modules/quickshell/Launcher.qml"));
    assert!(
        !own.join("quickshell/Theme.qml").is_symlink(),
        "the built theme, which no tracked file holds, shouldn't be linked"
    );

    let pid = i32::try_from(preview.id()).expect("a pid");
    let stopped = Instant::now();
    // SAFETY: kill only signals the preview this test started.
    unsafe { libc::kill(pid, libc::SIGTERM) };
    let status = preview.wait().expect("the preview ends");

    assert_eq!(status.code(), Some(143));
    // Asked, the desktop ends at once; only one that ignores being asked
    // waits out the preview's five seconds' grace to be killed.
    assert!(
        stopped.elapsed() < Duration::from_secs(2),
        "the desktop was killed, not asked"
    );
    assert!(!running(sleep), "the desktop's sleep was left running");
    assert!(!own.exists(), "the preview's folder was left behind");
}

#[test]
fn the_window_closing_ends_the_preview_as_the_desktop_did() {
    let stand_in = StandIn::new("closed");
    stand_in.write("close-at-once", "");

    let ran = stand_in
        .start()
        .wait_with_output()
        .expect("the preview ends");

    assert_eq!(
        ran.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        said.contains("1 file follows the working copy, 1 was built"),
        "{said}"
    );
}

#[test]
fn the_desktop_ending_badly_passes_its_code_on() {
    let stand_in = StandIn::new("failed");
    stand_in.write("close-at-once", "3");

    let ran = stand_in
        .start()
        .wait_with_output()
        .expect("the preview ends");

    let said = String::from_utf8_lossy(&ran.stderr);
    assert_eq!(ran.status.code(), Some(3), "{said}");
    assert!(said.contains("the nested desktop ended with 3"), "{said}");
}

/// shell-preview run with only these words, and nothing to show.
fn asked(words: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_shell-preview"))
        .args(words)
        .output()
        .expect("shell-preview runs")
}

/// The error a run said on stderr, as JSON.
fn error_of(ran: &std::process::Output) -> serde_json::Value {
    serde_json::from_slice(&ran.stderr).expect("its error is JSON")
}

#[test]
fn an_option_it_doesnt_take_is_refused_naming_the_nearest_it_does() {
    let ran = asked(&["--tpye", "firefox", "--format", "json"]);

    assert_eq!(ran.status.code(), Some(64));
    let error = error_of(&ran);
    assert_eq!(error["what"], "no option called --tpye", "{error}");
    assert_eq!(error["why"], "did you mean --type?", "{error}");
}

#[test]
fn an_option_without_its_value_is_refused() {
    let ran = asked(&["--check", "--format", "json"]);

    assert_eq!(ran.status.code(), Some(64));
    let error = error_of(&ran);
    assert_eq!(error["what"], "--check needs a value", "{error}");
}

#[test]
fn its_help_names_each_option_and_the_codes_it_ends_with() {
    let ran = asked(&["--help", "--format", "json"]);

    assert_eq!(ran.status.code(), Some(0));
    let help: serde_json::Value = serde_json::from_slice(&ran.stdout).expect("help/1");
    assert_eq!(help["schema"], "help/1", "{help}");
    let named = |group: &str, field: &str| -> Vec<String> {
        help[group]
            .as_array()
            .into_iter()
            .flatten()
            .map(|entry| entry[field].to_string().trim_matches('"').to_owned())
            .collect()
    };
    let options = named("options", "name");
    for option in ["--type TEXT", "--check NAME", "--account NAME"] {
        assert!(options.iter().any(|named| named == option), "{options:?}");
    }
    let codes = named("exits", "code");
    assert_eq!(codes, ["0", "1", "64", "130"], "{help}");
    assert!(
        help["passes"]
            .as_str()
            .is_some_and(|words| words.contains("nested desktop")),
        "the desktop's own code isn't said as passed on: {help}"
    );
}
