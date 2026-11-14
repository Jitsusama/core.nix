//! The experiment's daemon: the engine (engine.rs) over the asks folder,
//! serving its rows in Asks over the socket systemd holds for it, and
//! answering the launcher's release and refuse.
//!
//! It's an experiment, so it may release one item alone, the one it's
//! given, and only into its own checker, which says how long the value is
//! and nothing more. An ask for anything else is refused unseen, so no
//! real secret can come out of it and no agent's command can run with
//! one. What it doesn't hold is that only Joel's key says yes: its socket
//! is his account's, as every daemon's is, so another program could say
//! yes for this throwaway item; the passwords daemon, which answers asks
//! from the launcher's own channel, holds that.
//!
//! Each answer is written beside its ask as `<id>.answer.json`, and the
//! ask's file is taken away; the journal says each ask's id, item,
//! command, tree and outcome, never the value, and each answered ask
//! leaves a line of fields alone for the machine's health to count
//! (reading.rs).

use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

use daemon_serve::asks::Domain;
use daemon_serve::serve::{self, Own, Shared};
use ready_rows::kept::Kept;
use serde_json::json;

use crate::ask::Ask;
use crate::engine::{self, Engine, Releaser, Secret, Setting};
use crate::words::Words;

pub use crate::engine::{now_ms, write_whole};

/// The daemon as the launcher knows it: Asks' rows, and how many wait.
const DOMAIN: Domain = Domain {
    name: "secret-asks",
    kinds: &["row", "summary"],
    sole: &[],
    named_only: &[],
};

/// The one command an ask may name: this program's checker.
pub const CHECKER: &[&str] = &["secret-asks", "received"];

/// What the daemon is given.
pub struct Config {
    pub socket: PathBuf,
    pub asks: PathBuf,
    /// The one item it may release.
    pub item: String,
    /// 1Password's account to read it from, where op's default isn't it.
    pub account: Option<String>,
    pub op: PathBuf,
    pub git: PathBuf,
    pub loginctl: PathBuf,
    /// The checker's program, this one.
    pub checker: PathBuf,
    pub home: String,
    pub words: Words,
    /// Where journald takes the readings' lines.
    pub journal: PathBuf,
}

/// What the experiment releases, and how: its one item, read by op, into
/// its checker alone.
struct Experiment {
    item: String,
    account: Option<String>,
    op: PathBuf,
    loginctl: PathBuf,
    checker: PathBuf,
}

impl Releaser for Experiment {
    /// An item other than the one, or a command other than the checker.
    fn refused(&self, ask: &Ask) -> Option<String> {
        if ask.item != self.item {
            return Some("not listed".to_owned());
        }
        if ask.argv != CHECKER {
            return Some("the experiment runs only its own checker".to_owned());
        }
        None
    }

    /// Whether the session in front, the account's display, is locked,
    /// as logind says; one logind can't say is taken as locked.
    fn locked(&self) -> bool {
        let asked = |args: &[&str]| {
            Command::new(&self.loginctl)
                .args(args)
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .output()
                .ok()
                .filter(|ran| ran.status.success())
                .map(|ran| String::from_utf8_lossy(&ran.stdout).trim().to_owned())
        };
        let uid = std::fs::metadata("/proc/self").map(|meta| {
            use std::os::unix::fs::MetadataExt;
            meta.uid().to_string()
        });
        let Ok(uid) = uid else { return true };
        let Some(display) = asked(&["show-user", &uid, "--property=Display", "--value"]) else {
            return true;
        };
        let hint = asked(&["show-session", &display, "--property=LockedHint", "--value"]);
        hint.as_deref() != Some("no")
    }

    /// The one item's value, read by op, which asks Joel through
    /// 1Password when it must.
    fn read_value(&self, _: &Ask) -> Result<Secret, String> {
        let mut op = Command::new(&self.op);
        op.arg("read").arg("--no-newline");
        if let Some(account) = &self.account {
            op.arg("--account").arg(account);
        }
        let ran = op
            .arg(&self.item)
            .stdin(Stdio::null())
            .output()
            .map_err(|why| format!("couldn't run 1Password's command: {why}"))?;
        if !ran.status.success() {
            let said = String::from_utf8_lossy(&ran.stderr);
            let line = said.lines().last().unwrap_or("").trim();
            return Err(format!("1Password said: {line}"));
        }
        String::from_utf8(ran.stdout)
            .map(Secret::new)
            .map_err(|_| "the value isn't text".to_owned())
    }

    /// The checker, with the value as its one variable.
    fn command(&self, ask: &Ask, value: &Secret) -> Result<Command, String> {
        let mut checker = Command::new(&self.checker);
        checker
            .arg(CHECKER[1])
            .env_clear()
            .env("PATH", "/run/current-system/sw/bin")
            .env(&ask.given, value.reveal());
        Ok(checker)
    }
}

/// Runs the daemon for as long as it may.
pub fn run(config: Config) -> ExitCode {
    let (to, heard) = mpsc::channel();
    let asks = config.asks.clone();
    let engine = Arc::new(Engine::new(
        Setting {
            asks: config.asks,
            git: config.git,
            home: config.home,
            words: config.words,
            journal: config.journal,
        },
        Box::new(Experiment {
            item: config.item,
            account: config.account,
            op: config.op,
            loginctl: config.loginctl,
            checker: config.checker,
        }),
        to,
    ));
    if let Err(why) = engine.start() {
        eprintln!("secret-asks: can't keep {}: {why}", asks.display());
        return ExitCode::FAILURE;
    }
    let own: Own = {
        let engine = Arc::clone(&engine);
        Box::new(move |said| match said["ask"].as_str() {
            Some("release") => Some(engine.release(said)),
            Some("refuse") => Some(engine.refuse(said)),
            _ => None,
        })
    };
    let shared: Arc<Shared> = Shared::new(DOMAIN, own);
    match serve::listen(&config.socket) {
        Ok(listener) => serve::accept(Arc::clone(&shared), listener),
        Err(why) => {
            eprintln!(
                "secret-asks: can't listen on {}: {why}",
                config.socket.display()
            );
            return ExitCode::FAILURE;
        }
    }
    let mut kept = Kept::default();
    loop {
        engine.look();
        let next = engine.tidy();
        let (rows, summary) = engine.rows();
        if !kept.take(rows, vec![summary]).is_empty() {
            shared.change(kept.all().cloned().collect(), json!({}));
        }
        let wait =
            next.map(|at| Duration::from_millis(u64::try_from(at - engine::now_ms()).unwrap_or(0)));
        let woke = match wait {
            Some(wait) => heard.recv_timeout(wait),
            None => heard.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        if woke == Err(RecvTimeoutError::Disconnected) {
            return ExitCode::FAILURE;
        }
        // Everything said meanwhile is taken before the rows are made again.
        while heard.try_recv().is_ok() {}
    }
}
