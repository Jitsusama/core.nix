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

use std::io::{BufRead, Write};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use onepassword_manager::{account, entry, in_account, period, reason, value};
use serde_json::{Value, json};

/// What Joel reads while 1Password hasn't let the launcher in.
const LET_IN: &str = "1Password asks you to let the launcher in";

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
    let mut manager = Manager {
        op,
        let_in: false,
        account: None,
    };
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", json!({"state": "signed-out", "reason": LET_IN}));
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
        let ran = Command::new(&self.op)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|why| Failed {
                signed_out: false,
                reason: format!("couldn't run 1Password's command: {why}"),
            })?;
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
