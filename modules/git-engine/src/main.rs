//! git-engine answers the prompt's questions about a repository: the
//! branch and commit, whether anything is staged, changed, conflicted or
//! untracked, and how far the branch is ahead of its upstream. It reads
//! git's files itself, refs in reftable as well as files, a sparse index,
//! packs through a multi-pack-index and the commit-graph, and starts no
//! program to do it.
//!
//! It speaks gitstatusd's protocol, so gitstatus's zsh plugin drives it
//! unchanged: requests arrive on stdin as `id\x1fdir\x1fno-diff\x1e`, and
//! each answer is `id\x1f1\x1f` and twenty-seven fields, or `id\x1f0` for a
//! folder in no repository, ended by `\x1e`. It keeps what it has read of a
//! repository between questions until the files it came from change.
//!
//! `git-engine --once DIR` answers once, one field to a line, for trying it
//! by hand.

mod attrs;
mod config;
mod fsmonitor;
mod graph;
mod hash;
mod index;
mod odb;
mod pattern;
mod refs;
mod reftable;
mod repo;
mod stamp;
mod status;
mod walks;
mod wildmatch;

use repo::{Answer, Options, Repo};
use status::Limits;
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

fn limit(value: Option<&str>, default: Option<usize>) -> Option<usize> {
    match value.and_then(|v| v.parse::<i64>().ok()) {
        Some(n) if n < 0 => None,
        Some(n) => usize::try_from(n).ok(),
        None => default,
    }
}

fn options(args: &[String]) -> Options {
    let mut values: HashMap<&str, &str> = HashMap::new();
    let mut switches = Vec::new();
    let mut at = 0;
    while at < args.len() {
        let arg = args[at].as_str();
        if let Some(flag) = arg.strip_prefix('-') {
            if matches!(flag, "e" | "U" | "W" | "D") {
                switches.push(flag);
            } else if let Some(value) = args.get(at + 1) {
                values.insert(flag, value);
                at += 1;
            }
        }
        at += 1;
    }
    Options {
        limits: Limits {
            staged: limit(values.get("s").copied(), Some(1)),
            unstaged: limit(values.get("u").copied(), Some(1)),
            conflicted: limit(values.get("c").copied(), Some(1)),
            untracked: limit(values.get("d").copied(), Some(1)),
        },
        max_index: limit(values.get("m").copied(), None),
        respect_untracked_setting: !switches.contains(&"U"),
    }
}

fn fields(answer: &Answer) -> Vec<String> {
    let counts = answer.counts.clone();
    let count = |pick: fn(&status::Counts) -> usize| {
        counts.as_ref().map_or("-1".into(), |c| pick(c).to_string())
    };
    vec![
        answer.workdir.clone(),
        answer.commit.clone(),
        answer.local_branch.clone(),
        answer.remote_branch.clone(),
        answer.remote_name.clone(),
        answer.remote_url.clone(),
        answer.action.clone(),
        answer.index_size.to_string(),
        count(|c| c.staged),
        count(|c| c.unstaged),
        count(|c| c.conflicted),
        count(|c| c.untracked),
        answer.ahead.to_string(),
        answer.behind.to_string(),
        answer.stashes.to_string(),
        answer.tag.clone(),
        count(|c| c.unstaged_deleted),
        count(|c| c.staged_new),
        count(|c| c.staged_deleted),
        answer.push_remote_name.clone(),
        answer.push_remote_url.clone(),
        answer.push_ahead.to_string(),
        answer.push_behind.to_string(),
        count(|c| c.skip_worktree),
        count(|c| c.assume_unchanged),
        answer.encoding.clone(),
        answer.summary.clone(),
    ]
}

/// The repositories asked about so far, by their git folder.
#[derive(Default)]
struct Engine {
    repos: HashMap<PathBuf, Repo>,
}

impl Engine {
    fn ask(&mut self, dir: &Path, options: &Options, diff: bool) -> Option<Answer> {
        let location = repo::discover(dir)?;
        let key = location.git_dir.clone();
        if self
            .repos
            .get(&key)
            .is_none_or(|repo| repo.location != location)
        {
            let repo = match Repo::open(location) {
                Ok(repo) => repo,
                Err(error) => {
                    eprintln!("git-engine: can't open {}: {error}", key.display());
                    return None;
                }
            };
            self.repos.insert(key.clone(), repo);
        }
        let repo = self.repos.get_mut(&key)?;
        match repo.answer(options, diff) {
            Ok(answer) => Some(answer),
            Err(error) => {
                eprintln!("git-engine: can't answer for {}: {error}", key.display());
                // Whatever went wrong may have been read part-way; start
                // this repository afresh next time.
                self.repos.remove(&key);
                None
            }
        }
    }
}

fn serve(options: &Options) -> std::io::Result<()> {
    let mut engine = Engine::default();
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let stdout = std::io::stdout();
    let mut request = Vec::new();
    loop {
        request.clear();
        if input.read_until(0x1e, &mut request)? == 0 {
            return Ok(());
        }
        let Some(body) = request.strip_suffix(&[0x1e]) else {
            return Ok(());
        };
        let mut parts = body.split(|&b| b == 0x1f);
        let id = parts.next().unwrap_or_default();
        let dir = parts.next().unwrap_or_default();
        let no_diff = parts.next().is_some_and(|flag| flag == b"1");
        let mut reply = id.to_vec();
        let answer = if dir.is_empty() {
            None
        } else {
            let dir = Path::new(std::ffi::OsStr::from_bytes(dir));
            engine.ask(dir, options, !no_diff)
        };
        match answer {
            Some(answer) => {
                reply.extend_from_slice(b"\x1f1");
                for field in fields(&answer) {
                    reply.push(0x1f);
                    reply.extend_from_slice(field.as_bytes());
                }
            }
            None => reply.extend_from_slice(b"\x1f0"),
        }
        reply.push(0x1e);
        let mut out = stdout.lock();
        out.write_all(&reply)?;
        out.flush()?;
    }
}

use std::os::unix::ffi::OsStrExt;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--once") {
        let dir = args
            .get(1)
            .map_or_else(|| PathBuf::from("."), PathBuf::from);
        let dir = std::fs::canonicalize(&dir).unwrap_or(dir);
        let options = options(&args[2.min(args.len())..]);
        let started = std::time::Instant::now();
        let answer = Engine::default().ask(&dir, &options, true);
        let took = started.elapsed();
        match answer {
            Some(answer) => {
                let names = [
                    "workdir",
                    "commit",
                    "local_branch",
                    "remote_branch",
                    "remote_name",
                    "remote_url",
                    "action",
                    "index_size",
                    "staged",
                    "unstaged",
                    "conflicted",
                    "untracked",
                    "ahead",
                    "behind",
                    "stashes",
                    "tag",
                    "unstaged_deleted",
                    "staged_new",
                    "staged_deleted",
                    "push_remote_name",
                    "push_remote_url",
                    "push_ahead",
                    "push_behind",
                    "skip_worktree",
                    "assume_unchanged",
                    "encoding",
                    "summary",
                ];
                for (name, value) in names.iter().zip(fields(&answer)) {
                    println!("{name}={value}");
                }
                eprintln!("answered in {took:?}");
            }
            None => {
                println!("norepo");
                std::process::exit(1);
            }
        }
        return;
    }
    if serve(&options(&args)).is_err() {
        std::process::exit(1);
    }
}
