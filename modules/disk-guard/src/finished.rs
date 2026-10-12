//! Whether a checkout's work is finished, as a predicate says: a command
//! run in the checkout, told its branch, head and remote, which prints
//! `{"finished": true}` or `{"finished": false}`. Anything else, a
//! failure or a timeout, means not finished, so a predicate that breaks
//! keeps what it was asked about.

use crate::commands::{self, Job, Ran};
use regex_lite::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub struct Checkout {
    /// `None` when its head is detached.
    pub branch: Option<String>,
    pub head: String,
    /// The branch's remote's URL, or origin's, and empty with neither.
    pub remote: String,
}

pub struct Predicate {
    pub remote: Regex,
    pub command: Vec<OsString>,
    pub timeout: Duration,
}

/// How long git is given to answer about a checkout.
const GIT_TIMEOUT: Duration = Duration::from_secs(10);

/// How long an unfinished answer is believed.
const RECHECK: f64 = 3_600.0;

/// Asks git, without taking the optional locks a status would, so a
/// checkout someone is working in sees nothing of it.
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let mut argv = vec![OsString::from("git"), OsString::from("-C"), dir.into()];
    argv.extend(args.iter().map(OsString::from));
    let job = Job {
        argv,
        cwd: None,
        env: vec![(OsString::from("GIT_OPTIONAL_LOCKS"), OsString::from("0"))],
        timeout: GIT_TIMEOUT,
        limit: 64 << 10,
    };
    match commands::run(&job).ok()? {
        Ran::Finished(output) if output.status == Some(0) => {
            Some(String::from_utf8(output.stdout).ok()?.trim().to_owned())
        }
        _ => None,
    }
}

/// A checkout's branch, head and remote, or `None` when it isn't one git
/// can read.
pub fn read(dir: &Path) -> Option<Checkout> {
    // A checkout with no commit yet fails this, having no head to name.
    let named = git(dir, &["rev-parse", "HEAD", "--symbolic-full-name", "HEAD"])?;
    let mut lines = named.lines();
    let head = lines.next()?.to_owned();
    let branch = lines
        .next()
        .and_then(|name| name.strip_prefix("refs/heads/"))
        .map(str::to_owned);
    let remote = branch
        .as_ref()
        .and_then(|branch| {
            git(
                dir,
                &["config", "--get", &format!("branch.{branch}.remote")],
            )
        })
        .unwrap_or_else(|| "origin".to_owned());
    let remote = git(dir, &["remote", "get-url", &remote]).unwrap_or_default();
    Some(Checkout {
        branch,
        head,
        remote,
    })
}

/// The first predicate for a checkout's remote.
pub fn predicate_for<'a>(predicates: &'a [Predicate], remote: &str) -> Option<&'a Predicate> {
    predicates
        .iter()
        .find(|predicate| predicate.remote.is_match(remote))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Answer {
    finished: bool,
}

/// Asks a predicate about a checkout.
pub fn ask(predicate: &Predicate, dir: &Path, checkout: &Checkout) -> bool {
    let told = |name: &str, value: &str| (OsString::from(name), OsString::from(value));
    let job = Job {
        argv: predicate.command.clone(),
        cwd: Some(dir.to_owned()),
        env: vec![
            told(
                "DISK_GUARD_BRANCH",
                checkout.branch.as_deref().unwrap_or(""),
            ),
            told("DISK_GUARD_HEAD", &checkout.head),
            told("DISK_GUARD_REMOTE", &checkout.remote),
        ],
        timeout: predicate.timeout,
        limit: 64 << 10,
    };
    match commands::run(&job) {
        Ok(Ran::Finished(output)) if output.status == Some(0) && !output.truncated => {
            serde_json::from_slice::<Answer>(&output.stdout).is_ok_and(|answer| answer.finished)
        }
        _ => false,
    }
}

/// Answers remembered per checkout: a finished head stays finished, and
/// an unfinished one is asked about again after an hour.
#[derive(Default)]
pub struct Answers {
    known: HashMap<PathBuf, (String, bool, f64)>,
}

impl Answers {
    pub fn finished(
        &mut self,
        dir: &Path,
        head: &str,
        now: f64,
        ask: impl FnOnce() -> bool,
    ) -> bool {
        if let Some((known, finished, at)) = self.known.get(dir)
            && known == head
            && (*finished || now - at < RECHECK)
        {
            return *finished;
        }
        let finished = ask();
        self.known
            .insert(dir.to_owned(), (head.to_owned(), finished, now));
        finished
    }
}
