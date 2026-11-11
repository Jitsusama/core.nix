//! passwords-1password [--op PATH] [--open-unit UNIT [--opened FILE] [--systemctl PATH]]
//!
//! 1Password as a password manager of Passwords, on Passwords' line
//! protocol (modules/passwords/src/main.rs): an ask a line in, its answer
//! a line out by the same id, through 1Password's command, op, at PATH or
//! as found on the PATH. A line with no id says how it stands, by itself,
//! the moment that changes.
//!
//! op asks Joel, through 1Password, to let it in the first time, and again
//! once its leave lapses or 1Password locks. So op runs only for an ask
//! that comes from Joel's own key, `asked` or one that copies, shows or
//! lets in: never as this starts, and never for a list or a get the shell
//! makes on its own, which is answered as signed out until op has
//! answered here once. 1Password's prompt only ever answers what he just
//! did.
//!
//! op can't ask the app anything while 1Password's Integrate with
//! 1Password CLI is off, nor while the app isn't running, and no prompt
//! would ever come. So the manager watches the app's settings, and with
//! `--open-unit` the file the app's unit keeps while it runs (FILE, by
//! default `$XDG_RUNTIME_DIR/onepassword/open`), through inotify, so
//! nothing wakes while nothing changes, and says how it stands the moment
//! either changes: refused by the setting to turn on, or closed. Joel's
//! own ask while the app is closed starts its unit first, through
//! systemctl, and says so (2026-11-10: a refusal kept after Joel
//! turned the setting on, and an Unlock with the app closed that did
//! nothing).
//!
//! An audit (`audit`, asked by Joel alone) gives every login's password at
//! once, `{"passwords": [{"entry", "value"}]}`, from the cheapest op can
//! give them: its list of logins and passwords piped to one `op item get -
//! --reveal`, which reads every item named on its input in one run.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use inotify::{Inotify, WatchMask};
use onepassword_manager::{
    LET_IN, Leave, OPENING, Stands, UNASKED, account, entry, in_account, lets_its_command_in,
    passwords, period, reason, settings_file, value,
};
use serde_json::{Map, Value, json};

/// How long 1Password may take to start once its unit is asked to.
const OPENS_WITHIN: Duration = Duration::from_secs(20);
/// How long op may take to reach an app it just opened, since the app
/// answers op only once it has started in full.
const REACHES_WITHIN: Duration = Duration::from_secs(15);
/// How often op is run again while it can't yet reach that app.
const REACHING_AGAIN: Duration = Duration::from_secs(1);

fn main() {
    let given = given();
    let config_home = std::env::var("XDG_CONFIG_HOME").ok();
    let home = std::env::var("HOME").ok();
    let settings = settings_file(config_home.as_deref(), home.as_deref());
    let opened = given.opener.as_ref().map(|opener| opener.opened.clone());
    let open = opened.as_deref().map(Path::exists);
    let known = Arc::new(Known {
        open: Mutex::new(open),
        changed: Condvar::new(),
        leave: AtomicU8::new(Leave::Unasked.byte()),
    });
    // Watched before it first says how it stands, so nothing changing
    // between the two goes unsaid.
    let kept = kept_out(settings.as_deref());
    watch(settings.clone(), opened, &known, (kept, open));
    say(&Stands::of(kept, open, Leave::Unasked).said());
    let mut manager = Manager {
        op: given.op,
        known,
        account: None,
        settings,
        opener: given.opener,
        reaching: None,
    };
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let Ok(asked) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        say(&manager.answer(&asked));
    }
}

/// What the manager was started with.
struct Given {
    op: String,
    opener: Option<Opener>,
}

/// How the manager opens 1Password and knows it's open: the file its unit
/// keeps while the app runs, the unit, and systemctl to start it.
struct Opener {
    opened: PathBuf,
    unit: String,
    systemctl: String,
}

fn given() -> Given {
    let mut args = std::env::args().skip(1);
    let mut op = "op".to_owned();
    let mut opened = None;
    let mut unit = None;
    let mut systemctl = "systemctl".to_owned();
    while let (Some(flag), Some(value)) = (args.next(), args.next()) {
        match flag.as_str() {
            "--op" => op = value,
            "--opened" => opened = Some(PathBuf::from(value)),
            "--open-unit" => unit = Some(value),
            "--systemctl" => systemctl = value,
            _ => eprintln!("<4>passwords-1password doesn't know {flag}"),
        }
    }
    // The unit's own runtime folder, %t to systemd, which a command given
    // to Passwords can't say.
    let opened = opened.or_else(|| {
        std::env::var_os("XDG_RUNTIME_DIR")
            .map(|run| PathBuf::from(run).join("onepassword").join("open"))
    });
    Given {
        op,
        opener: unit.zip(opened).map(|(unit, opened)| Opener {
            opened,
            unit,
            systemctl,
        }),
    }
}

/// What the watch and the asks both know of 1Password.
struct Known {
    /// Whether the app is open, `None` where nothing says.
    open: Mutex<Option<bool>>,
    /// Told each time `open` changes, for an ask waiting on the app.
    changed: Condvar,
    /// What op said of its leave since the app last opened, so whether
    /// it's live.
    leave: AtomicU8,
}

impl Known {
    fn open(&self) -> Option<bool> {
        *self.open.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn leave(&self) -> Leave {
        Leave::of(self.leave.load(Ordering::SeqCst))
    }

    fn set_leave(&self, leave: Leave) {
        self.leave.store(leave.byte(), Ordering::SeqCst);
    }
}

/// Says a line on Passwords' protocol, whole, beside any other.
fn say(line: &Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

/// Whether 1Password's settings say its command is kept out, read
/// afresh. A file not read says nothing, and op's own words stand.
fn kept_out(settings: Option<&str>) -> bool {
    settings
        .and_then(|file| std::fs::read_to_string(file).ok())
        .and_then(|settings| lets_its_command_in(&settings))
        == Some(false)
}

/// Watches the folders holding 1Password's settings and the file its unit
/// keeps, and says how the manager stands each time what they say
/// changes. A folder not there isn't watched, which the manager says,
/// since 1Password makes its own the first time it runs.
fn watch(
    settings: Option<String>,
    opened: Option<PathBuf>,
    known: &Arc<Known>,
    mut was: (bool, Option<bool>),
) {
    let mut inotify = match Inotify::init() {
        Ok(inotify) => inotify,
        Err(why) => {
            eprintln!("<4>1Password's settings and app go unwatched: {why}");
            return;
        }
    };
    let changes = WatchMask::CLOSE_WRITE
        | WatchMask::CREATE
        | WatchMask::DELETE
        | WatchMask::MOVED_TO
        | WatchMask::MOVED_FROM;
    if let Some(folder) = settings.as_deref().map(Path::new).and_then(Path::parent)
        && let Err(why) = inotify.watches().add(folder, changes)
    {
        eprintln!("<4>1Password's settings go unwatched: {why}");
    }
    if let Some(folder) = opened.as_deref().and_then(Path::parent)
        && let Err(why) = std::fs::create_dir_all(folder)
            .and_then(|()| inotify.watches().add(folder, changes).map(|_| ()))
    {
        eprintln!("<4>whether 1Password is open goes unwatched: {why}");
    }
    let known = Arc::clone(known);
    std::thread::spawn(move || {
        let mut buffer = [0; 4096];
        while inotify.read_events_blocking(&mut buffer).is_ok() {
            let now = (
                kept_out(settings.as_deref()),
                opened.as_deref().map(Path::exists),
            );
            if now == was {
                continue;
            }
            was = now;
            *known.open.lock().unwrap_or_else(PoisonError::into_inner) = now.1;
            known.changed.notify_all();
            // A closed app's leave goes with it, and one opened again
            // hasn't been asked.
            if now.1 == Some(false) {
                known.set_leave(Leave::Unasked);
            }
            say(&Stands::of(now.0, now.1, known.leave()).said());
        }
        eprintln!("<4>stopped watching 1Password's settings and app");
    });
}

struct Manager {
    op: String,
    known: Arc<Known>,
    /// The account's id, once op has said it, for each entry's link.
    account: Option<String>,
    /// 1Password's settings file, read for whether it lets op in.
    settings: Option<String>,
    /// How it opens the app, where it was told.
    opener: Option<Opener>,
    /// Until when op may be run again for an ask whose app it just opened.
    reaching: Option<Instant>,
}

/// How an ask failed. One where op couldn't be asked (`stands`) says how
/// the manager stands with it: signed out for a leave not given or
/// lapsed, else by the setting, the app, or op's own words. One about an
/// entry, as one with no such detail, says only what was wrong.
struct Failed {
    signed_out: bool,
    stands: bool,
    reason: String,
}

impl Failed {
    /// One about an entry or an ask, saying nothing of the manager.
    fn only(reason: String) -> Self {
        Self {
            signed_out: false,
            stands: false,
            reason,
        }
    }
}

impl Manager {
    fn answer(&mut self, asked: &Value) -> Value {
        let id = asked.get("id").cloned().unwrap_or(Value::Null);
        let joels = asked.get("asked").and_then(Value::as_bool) == Some(true);
        let item = asked.get("entry").and_then(Value::as_str).unwrap_or("");
        let ask = asked.get("ask").and_then(Value::as_str);
        // Only Joel's own key opens the app, as only it has op prompt.
        let opens = joels || matches!(ask, Some("let-in" | "reveal" | "code"));
        let result = if opens && self.known.open() == Some(false) {
            self.open_the_app()
        } else {
            Ok(())
        }
        .and_then(|()| match ask {
            Some("list") => self.list(joels),
            Some("get") => self.get(item, joels),
            Some("reveal") => self.reveal(item, asked.get("detail").and_then(Value::as_str)),
            Some("code") => self.code(item),
            Some("audit") => self.audit(joels),
            Some("let-in") => self
                .run(&["vault", "list", "--format", "json"])
                .map(|_| json!({})),
            other => Err(Failed::only(format!(
                "1Password's manager can't answer {other:?}"
            ))),
        });
        self.reaching = None;
        let mut answer = match result {
            Ok(Value::Object(answer)) => answer,
            Ok(other) => {
                let mut wrapped = Map::new();
                wrapped.insert("answer".to_owned(), other);
                wrapped
            }
            Err(failed) => self.failed(&failed),
        };
        if self.known.leave() == Leave::In && !answer.contains_key("error") {
            answer.insert("state".to_owned(), json!("answering"));
        }
        answer.insert("id".to_owned(), id);
        Value::Object(answer)
    }

    /// A failure as an answer: its error, and how the manager stands
    /// where op couldn't be asked, the setting first, then the app, then
    /// op's leave or its own words, so Passwords always says why.
    fn failed(&self, failed: &Failed) -> Map<String, Value> {
        let mut answer = Map::new();
        if !failed.stands {
            answer.insert(
                "error".to_owned(),
                json!({"code": "refused", "message": failed.reason}),
            );
            return answer;
        }
        let kept = kept_out(self.settings.as_deref());
        let said = match Stands::of(kept, self.known.open(), Leave::In) {
            // Refused before op ran says that nothing asked it yet.
            Stands::Answering if failed.signed_out && self.known.leave() == Leave::Unasked => {
                Stands::Unasked.said()
            }
            Stands::Answering if failed.signed_out => Stands::LetIn.said(),
            Stands::Answering => json!({"state": "refused", "reason": failed.reason}),
            other => other.said(),
        };
        let code = if said["state"] == "refused" {
            "refused"
        } else {
            "signed-out"
        };
        answer.insert(
            "error".to_owned(),
            json!({"code": code, "message": said["reason"]}),
        );
        answer.insert("state".to_owned(), said["state"].clone());
        answer.insert("reason".to_owned(), said["reason"].clone());
        answer
    }

    /// Starts 1Password's unit for Joel's ask, saying so first, and waits
    /// for the app to be open, so op has it to ask him through.
    fn open_the_app(&mut self) -> Result<(), Failed> {
        let Some(opener) = &self.opener else {
            return Ok(());
        };
        say(&json!({"state": "signed-out", "reason": OPENING}));
        let unopened = |why: String| Failed::only(format!("1Password didn't open: {why}"));
        let started = Command::new(&opener.systemctl)
            .args(["--user", "start", &opener.unit])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .map_err(|why| unopened(why.to_string()))?;
        if !started.status.success() {
            return Err(unopened(
                String::from_utf8_lossy(&started.stderr).trim().to_owned(),
            ));
        }
        let open = self
            .known
            .open
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let (open, waited) = self
            .known
            .changed
            .wait_timeout_while(open, OPENS_WITHIN, |open| *open != Some(true))
            .unwrap_or_else(PoisonError::into_inner);
        drop(open);
        if waited.timed_out() {
            return Err(unopened(format!(
                "it wasn't running {} s after it was started",
                OPENS_WITHIN.as_secs()
            )));
        }
        self.reaching = Some(Instant::now() + REACHES_WITHIN);
        Ok(())
    }

    /// Whether op may run for this ask: Joel's own, or once op has let
    /// the launcher in and its leave is live.
    fn may(&self, joels: bool) -> Result<(), Failed> {
        let leave = self.known.leave();
        if joels || leave == Leave::In {
            Ok(())
        } else {
            Err(Failed {
                signed_out: true,
                stands: true,
                reason: (if leave == Leave::Unasked {
                    UNASKED
                } else {
                    LET_IN
                })
                .to_owned(),
            })
        }
    }

    fn list(&mut self, joels: bool) -> Result<Value, Failed> {
        self.may(joels)?;
        let items = self.json(&["item", "list", "--format", "json"])?;
        let account = self.account();
        let entries: Vec<Value> = items
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(entry)
            .map(|entry| in_account(entry, account.as_deref()))
            .collect();
        Ok(json!({"entries": entries}))
    }

    /// The account's id, asked of op once, only after it has answered for
    /// this ask, so its leave is live and asking prompts Joel for nothing.
    /// One op can't say leaves the entries without a link, and is asked
    /// again with the next.
    fn account(&mut self) -> Option<String> {
        if self.account.is_none() {
            self.account = self
                .json(&["account", "get", "--format", "json"])
                .ok()
                .as_ref()
                .and_then(account);
        }
        self.account.clone()
    }

    fn get(&mut self, item: &str, joels: bool) -> Result<Value, Failed> {
        self.may(joels)?;
        let got = self.json(&["item", "get", item, "--format", "json"])?;
        let entry = entry(&got)
            .ok_or_else(|| Failed::only(format!("1Password gave no item for {item}")))?;
        let account = self.account();
        Ok(json!({"entry": in_account(entry, account.as_deref())}))
    }

    /// One detail's value, read from the whole item op gives and the rest
    /// of it dropped as this returns.
    fn reveal(&mut self, item: &str, detail: Option<&str>) -> Result<Value, Failed> {
        let got = self.json(&["item", "get", item, "--format", "json"])?;
        let wanted = detail.unwrap_or("password");
        let value =
            value(&got, wanted).ok_or_else(|| Failed::only(format!("it has no {wanted}")))?;
        Ok(json!({"value": value}))
    }

    /// Every login's password, for an audit, only as Joel's own ask: the
    /// logins and passwords op lists, each read in full by one op run fed
    /// that list, and nothing kept of what it said but each id and its
    /// password, which go in the answer and are dropped with it.
    fn audit(&mut self, joels: bool) -> Result<Value, Failed> {
        if !joels {
            return Err(Failed::only(
                "passwords are audited only when you ask".to_owned(),
            ));
        }
        let listed = self.run(&[
            "item",
            "list",
            "--categories",
            "Login,Password",
            "--format",
            "json",
        ])?;
        // op refuses an empty list on its input, and there's nothing to read.
        if serde_json::from_str::<Value>(&listed)
            .is_ok_and(|items| items.as_array().is_some_and(Vec::is_empty))
        {
            return Ok(json!({"passwords": []}));
        }
        let said = self.run_with(
            &["item", "get", "-", "--reveal", "--format", "json"],
            Some(&listed),
        )?;
        let given: Vec<Value> = passwords(&said)
            .into_iter()
            .map(|(entry, value)| json!({"entry": entry, "value": value}))
            .collect();
        Ok(json!({"passwords": given}))
    }

    /// The one-time code now, as op works it out (`--otp`), with its
    /// period and the seconds left in it by this machine's clock.
    fn code(&mut self, item: &str) -> Result<Value, Failed> {
        let got = self.json(&["item", "get", item, "--format", "json"])?;
        let code = self.run(&["item", "get", item, "--otp"])?;
        let period = period(&got);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        Ok(json!({"code": code.trim(), "period": period, "left": period - now % period}))
    }

    fn json(&mut self, args: &[&str]) -> Result<Value, Failed> {
        let said = self.run(args)?;
        serde_json::from_str(&said)
            .map_err(|why| Failed::only(format!("1Password's answer couldn't be read: {why}")))
    }

    /// Runs op, its answer on success; what it says on failure tells a
    /// leave not given from anything else, and is never logged, since an
    /// answer may carry a secret.
    fn run(&mut self, args: &[&str]) -> Result<String, Failed> {
        self.run_with(args, None)
    }

    /// Runs op as [`Manager::run`] does, given `input` on its own input.
    /// Just after it opened the app for this ask, op is run again a while
    /// as it fails otherwise than for Joel's leave, since the app answers
    /// op only once it has started in full.
    fn run_with(&mut self, args: &[&str], input: Option<&str>) -> Result<String, Failed> {
        loop {
            let ran = self.ran(args, input);
            match (&ran, self.reaching) {
                (Err(failed), Some(until)) if !failed.signed_out && Instant::now() < until => {
                    std::thread::sleep(REACHING_AGAIN);
                }
                _ => return ran,
            }
        }
    }

    /// Runs op once, written `input` as it reads it so neither waits on
    /// the other.
    fn ran(&mut self, args: &[&str], input: Option<&str>) -> Result<String, Failed> {
        let unran = |why: std::io::Error| Failed {
            signed_out: false,
            stands: true,
            reason: format!("couldn't run 1Password's command: {why}"),
        };
        let mut child = Command::new(&self.op)
            .args(args)
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(unran)?;
        let writing = child.stdin.take().zip(input).map(|(mut stdin, input)| {
            let input = input.to_owned();
            std::thread::spawn(move || {
                let _ = stdin.write_all(input.as_bytes());
            })
        });
        let ran = child.wait_with_output().map_err(unran)?;
        if let Some(writing) = writing {
            let _ = writing.join();
        }
        if ran.status.success() {
            self.known.set_leave(Leave::In);
            return Ok(String::from_utf8_lossy(&ran.stdout).into_owned());
        }
        let said = reason(&String::from_utf8_lossy(&ran.stderr));
        let lowered = said.to_lowercase();
        let signed_out = [
            "not signed in",
            "authoriz",
            "locked",
            "sign in",
            "dismissed",
        ]
        .iter()
        .any(|word| lowered.contains(word));
        if signed_out {
            self.known.set_leave(Leave::Out);
        }
        Err(Failed {
            signed_out,
            stands: true,
            reason: if signed_out { LET_IN.to_owned() } else { said },
        })
    }
}
