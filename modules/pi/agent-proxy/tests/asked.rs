//! The proxy asked in ways its unit never asks it: for its help, with a
//! flag it doesn't know or short of one it needs, or with a hosts file
//! it can't read. Each ends at once in the terminal kit's shape, with
//! the code docs/commands.md gives it, before it reaches any agent.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

/// A folder of the test's own, holding `hosts` as its hosts file.
fn folder(name: &str, hosts: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "pi-agent-proxy-asked-{name}-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("a folder for the test");
    fs::write(dir.join("hosts"), hosts).expect("the hosts file is written");
    dir
}

/// What the proxy says and ends with, run with `args` from `dir`.
fn ran(dir: &PathBuf, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pi-agent-proxy"))
        .args(args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .env_remove("LISTEN_PID")
        .env_remove("LISTEN_FDS")
        .stdin(Stdio::null())
        .output()
        .expect("pi-agent-proxy ran")
}

fn said(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Ends `code`, saying `words` and `look`, and nothing on stdout.
fn ended(name: &str, args: &[&str], code: i32, words: &str, look: &str) {
    ended_with(name, "", args, code, words, look);
}

fn ended_with(name: &str, hosts: &str, args: &[&str], code: i32, words: &str, look: &str) {
    let output = ran(&folder(name, hosts), args);
    let why = said(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(code),
        "{args:?} ended {:?}: {why}",
        output.status
    );
    assert!(why.contains(words), "{args:?} never said {words:?}: {why}");
    assert!(why.contains(look), "{args:?} never said {look:?}: {why}");
    assert!(
        output.stdout.is_empty(),
        "{args:?} said {:?} on stdout",
        said(&output.stdout)
    );
}

#[test]
fn its_help_opens_with_its_name_and_names_its_flags() {
    let output = ran(&folder("help", ""), &["--help"]);
    let help = said(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "help ended {:?}",
        output.status
    );
    assert!(help.starts_with("pi-agent-proxy"), "help opened {help:?}");
    for word in ["--hosts", "--upstream", "--key", "--lent", "--socket"] {
        assert!(help.contains(word), "help never named {word}: {help}");
    }
}

#[test]
fn a_mistyped_flag_names_the_nearest_and_is_refused_as_asked_wrongly() {
    ended(
        "typo",
        &["--hosts", "hosts", "--upstrem", "agent.sock"],
        64,
        "--upstream",
        "pi-agent-proxy --help",
    );
}

#[test]
fn a_flag_short_of_its_value_is_refused_as_asked_wrongly() {
    ended(
        "no-value",
        &["--hosts", "hosts", "--upstream"],
        64,
        "--upstream",
        "pi-agent-proxy --help",
    );
}

#[test]
fn a_proxy_told_of_no_hosts_or_no_agent_is_refused_as_asked_wrongly() {
    ended(
        "no-hosts",
        &["--upstream", "agent.sock"],
        64,
        "--hosts",
        "pi-agent-proxy --help",
    );
    ended(
        "no-agent",
        &["--hosts", "hosts"],
        64,
        "--upstream",
        "pi-agent-proxy --help",
    );
    ended(
        "no-lent",
        &["--hosts", "hosts", "--key", "builders"],
        64,
        "--lent",
        "pi-agent-proxy --help",
    );
}

#[test]
fn a_hosts_file_it_cant_read_is_its_configuration_gone_wrong() {
    ended(
        "unreadable",
        &["--hosts", "missing", "--upstream", "agent.sock"],
        78,
        "missing",
        "--hosts",
    );
    ended_with(
        "malformed",
        "not a key\n",
        &["--hosts", "hosts", "--upstream", "agent.sock"],
        78,
        "hosts",
        "--hosts",
    );
}

#[test]
fn a_proxy_with_nowhere_to_listen_fails_saying_so() {
    ended(
        "no-socket",
        &["--hosts", "hosts", "--upstream", "agent.sock"],
        1,
        "listen",
        "--socket",
    );
}
