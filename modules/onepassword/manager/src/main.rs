//! passwords-1password [--op PATH]
//!
//! 1Password as a password manager of Passwords, on Passwords' line
//! protocol (modules/passwords/src/main.rs): an ask a line in, its answer
//! a line out by the same id, through 1Password's command, op, at PATH or
//! as found on the PATH.
//!
//! op asks Joel, through 1Password, to let it in the first time, and again
//! once its leave lapses or 1Password locks. So op runs only for an ask
//! that comes from Joel's own key, `asked` or one that copies, shows or
//! lets in: never as this starts, and never for a list or a get the shell
//! makes on its own, which is answered as signed out until op has
//! answered here once. 1Password's prompt only ever answers what he just
//! did.
//!
//! Whatever op fails at while 1Password's Integrate with 1Password CLI is
//! off, the manager says it's refused, by the setting to turn on, since op
//! can't ask the app anything then and no prompt would ever come: as it
//! starts, and with every answer, the app's settings read each time, so
//! it's let in again the moment Joel turns it on.
//!
//! An audit (`audit`, asked by Joel alone) gives every login's password at
//! once, `{"passwords": [{"entry", "value"}]}`, from the cheapest op can
//! give them: its list of logins and passwords piped to one `op item get -
//! --reveal`, which reads every item named on its input in one run.

use std::io::{BufRead, Write};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use onepassword_manager::{
    account, entry, in_account, lets_its_command_in, passwords, period, reason, settings_file,
    value,
};
use serde_json::{Value, json};

/// What Joel reads while 1Password hasn't let the launcher in.
const LET_IN: &str = "1Password asks you to let the launcher in";

/// What Joel reads while 1Password keeps its command out, by the setting's
/// own name and where the app has it, so he can find it there.
const KEPT_OUT: &str =
    "1Password needs Integrate with 1Password CLI on, in Settings \u{203a} Developer";

fn main() {
    let mut args = std::env::args().skip(1);
    let mut op = "op".to_owned();
    while let Some(arg) = args.next() {
        if arg == "--op"
            && let Some(path) = args.next()
        {
            op = path;
        }
    }
    let config_home = std::env::var("XDG_CONFIG_HOME").ok();
    let home = std::env::var("HOME").ok();
    let mut manager = Manager {
        op,
        let_in: false,
        account: None,
        settings: settings_file(config_home.as_deref(), home.as_deref()),
    };
    let mut out = std::io::stdout().lock();
    let starting = if manager.kept_out() {
        json!({"state": "refused", "reason": KEPT_OUT})
    } else {
        json!({"state": "signed-out", "reason": LET_IN})
    };
    let _ = writeln!(out, "{starting}");
    let _ = out.flush();
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let Ok(asked) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let answer = manager.answer(&asked);
        let _ = writeln!(out, "{answer}");
        let _ = out.flush();
    }
}

struct Manager {
    op: String,
    /// Whether op has answered since this started, so its leave is live.
    let_in: bool,
    /// The account's id, once op has said it, for each entry's link.
    account: Option<String>,
    /// 1Password's settings file, read for whether it lets op in.
    settings: Option<String>,
}

/// How op failed: signed out for a leave not given or lapsed, refused
/// for anything else, with op's own words.
struct Failed {
    signed_out: bool,
    reason: String,
}

impl Manager {
    fn answer(&mut self, asked: &Value) -> Value {
        let id = asked.get("id").cloned().unwrap_or(Value::Null);
        let joels = asked.get("asked").and_then(Value::as_bool) == Some(true);
        let item = asked.get("entry").and_then(Value::as_str).unwrap_or("");
        let result = match asked.get("ask").and_then(Value::as_str) {
            Some("list") => self.list(joels),
            Some("get") => self.get(item, joels),
            Some("reveal") => self.reveal(item, asked.get("detail").and_then(Value::as_str)),
            Some("code") => self.code(item),
            Some("audit") => self.audit(joels),
            Some("let-in") => self
                .run(&["vault", "list", "--format", "json"])
                .map(|_| json!({})),
            other => Err(Failed {
                signed_out: false,
                reason: format!("1Password's manager can't answer {other:?}"),
            }),
        };
        let mut answer = match result {
            Ok(Value::Object(answer)) => answer,
            Ok(other) => {
                let mut wrapped = serde_json::Map::new();
                wrapped.insert("answer".to_owned(), other);
                wrapped
            }
            Err(_) if self.kept_out() => {
                let mut answer = serde_json::Map::new();
                answer.insert(
                    "error".to_owned(),
                    json!({"code": "refused", "message": KEPT_OUT}),
                );
                answer.insert("state".to_owned(), json!("refused"));
                answer.insert("reason".to_owned(), json!(KEPT_OUT));
                answer
            }
            Err(failed) => {
                let code = if failed.signed_out {
                    "signed-out"
                } else {
                    "refused"
                };
                let mut answer = serde_json::Map::new();
                answer.insert(
                    "error".to_owned(),
                    json!({"code": code, "message": failed.reason}),
                );
                if failed.signed_out {
                    answer.insert("state".to_owned(), json!("signed-out"));
                    answer.insert("reason".to_owned(), json!(failed.reason));
                }
                answer
            }
        };
        if self.let_in && !answer.contains_key("error") {
            answer.insert("state".to_owned(), json!("answering"));
        }
        answer.insert("id".to_owned(), id);
        Value::Object(answer)
    }

    /// Whether 1Password's settings say its command is kept out, read
    /// afresh, so the answer after Joel turns it on says he may unlock.
    /// A file not read says nothing, and op's own words stand.
    fn kept_out(&self) -> bool {
        self.settings
            .as_deref()
            .and_then(|file| std::fs::read_to_string(file).ok())
            .and_then(|settings| lets_its_command_in(&settings))
            == Some(false)
    }

    /// Whether op may run for this ask: Joel's own, or once op has let
    /// the launcher in and its leave is live.
    fn may(&self, joels: bool) -> Result<(), Failed> {
        if joels || self.let_in {
            Ok(())
        } else {
            Err(Failed {
                signed_out: true,
                reason: LET_IN.to_owned(),
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
        let entry = entry(&got).ok_or_else(|| Failed {
            signed_out: false,
            reason: format!("1Password gave no item for {item}"),
        })?;
        let account = self.account();
        Ok(json!({"entry": in_account(entry, account.as_deref())}))
    }

    /// One detail's value, read from the whole item op gives and the rest
    /// of it dropped as this returns.
    fn reveal(&mut self, item: &str, detail: Option<&str>) -> Result<Value, Failed> {
        let got = self.json(&["item", "get", item, "--format", "json"])?;
        let wanted = detail.unwrap_or("password");
        let value = value(&got, wanted).ok_or_else(|| Failed {
            signed_out: false,
            reason: format!("it has no {wanted}"),
        })?;
        Ok(json!({"value": value}))
    }

    /// Every login's password, for an audit, only as Joel's own ask: the
    /// logins and passwords op lists, each read in full by one op run fed
    /// that list, and nothing kept of what it said but each id and its
    /// password, which go in the answer and are dropped with it.
    fn audit(&mut self, joels: bool) -> Result<Value, Failed> {
        if !joels {
            return Err(Failed {
                signed_out: false,
                reason: "passwords are audited only when you ask".to_owned(),
            });
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
        serde_json::from_str(&said).map_err(|why| Failed {
            signed_out: false,
            reason: format!("1Password's answer couldn't be read: {why}"),
        })
    }

    /// Runs op, its answer on success; what it says on failure tells a
    /// leave not given from anything else, and is never logged, since an
    /// answer may carry a secret.
    fn run(&mut self, args: &[&str]) -> Result<String, Failed> {
        self.run_with(args, None)
    }

    /// Runs op as [`Manager::run`] does, given `input` on its own input,
    /// written as op reads it so neither waits on the other.
    fn run_with(&mut self, args: &[&str], input: Option<&str>) -> Result<String, Failed> {
        let unran = |why: std::io::Error| Failed {
            signed_out: false,
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
            self.let_in = true;
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
            self.let_in = false;
        }
        Err(Failed {
            signed_out,
            reason: if signed_out { LET_IN.to_owned() } else { said },
        })
    }
}
