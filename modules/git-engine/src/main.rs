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
//! repository between questions until the files it came from change, and
//! looks every half minute for packs that changed in a repository nobody
//! is asking about, closing them so a repack's old packs free their space.
//!
//! `git-engine --once DIR` answers once, its fields as facts, or as
//! `git-engine.answer/1` with `--json`, for trying it by hand. Its help and
//! refusals are the terminal kit's (docs/commands.md); serving, stdout is
//! the protocol's alone.

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
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

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

    /// Forgets each repository whose packs or commit-graph changed since
    /// it read them, closing the files it held. A prompt reads them again
    /// when it next asks; until then, a file git removed would otherwise
    /// keep its space for as long as this engine runs.
    fn let_go_of_changed(&mut self) {
        self.repos.retain(|_, repo| !repo.holds_changed_files());
    }
}

/// How often the engine looks for repositories whose files changed, so
/// that one no prompt asks about again lets go of them all the same.
const LOOK_EVERY: Duration = Duration::from_secs(30);

/// Each request read from `input`, ended by `\x1e` and without it, on a
/// thread of its own so the engine can do other work while none comes.
/// The channel closes at the end of the input or one cut short; a read
/// that fails is sent as the last thing on it.
fn requests(mut input: impl BufRead + Send + 'static) -> Receiver<std::io::Result<Vec<u8>>> {
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        loop {
            let mut request = Vec::new();
            match input.read_until(0x1e, &mut request) {
                Ok(0) => return,
                Ok(_) => match request.strip_suffix(&[0x1e]) {
                    Some(body) => {
                        if send.send(Ok(body.to_vec())).is_err() {
                            return;
                        }
                    }
                    None => return,
                },
                Err(error) => {
                    let _ = send.send(Err(error));
                    return;
                }
            }
        }
    });
    receive
}

fn serve(options: &Options) -> std::io::Result<()> {
    let mut engine = Engine::default();
    let incoming = requests(std::io::BufReader::new(std::io::stdin()));
    let stdout = std::io::stdout();
    let mut looked = Instant::now();
    loop {
        let wait = LOOK_EVERY.saturating_sub(looked.elapsed());
        let body = match incoming.recv_timeout(wait) {
            Ok(body) => Some(body?),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        };
        if looked.elapsed() >= LOOK_EVERY {
            engine.let_go_of_changed();
            looked = Instant::now();
        }
        let Some(body) = body else {
            continue;
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

use terminal_kit::{Exit, Failure, Help};

fn help() -> Help {
    Help::new(
        "git-engine",
        "answers the prompt's questions about a repository",
    )
    .example("git-engine --once .", "what the prompt would be told here")
    .example(
        "git-engine -s 1 -u 1",
        "serves gitstatusd's protocol on stdin, as the prompt starts it",
    )
    .command("--once DIR", "answers once for DIR, its fields as facts")
    .exit(Exit::Failed, "the prompt's pipe broke")
    .exit(Exit::NotFound, "DIR is in no repository, or isn't there")
}

fn main() -> std::process::ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        match context.args.split_first() {
            Some((once, rest)) if once == "--once" => once_for(context, rest),
            _ => {
                // gitstatus's flags each start with a dash, a value after
                // one included where it's a negative count, so a bare word
                // was never meant for the engine.
                if let Some(word) = stray(&context.args) {
                    return Err(Failure::new(Exit::Usage, format!("it takes no {word}"))
                        .try_this("git-engine --help"));
                }
                serve(&options(&context.args)).map_err(|error| {
                    Failure::new(Exit::Failed, "the prompt's pipe broke").because(error.to_string())
                })
            }
        }
    })
}

/// The first word that is neither a flag of gitstatusd's nor a value one
/// of them takes.
fn stray(args: &[String]) -> Option<&str> {
    let mut at = 0;
    while at < args.len() {
        let arg = args[at].as_str();
        match arg.strip_prefix('-') {
            Some("e" | "U" | "W" | "D") => {}
            Some(_) => at += 1,
            None => return Some(arg),
        }
        at += 1;
    }
    None
}

/// The fields of an answer that are words; the rest are counts.
const WORDS: [&str; 12] = [
    "workdir",
    "commit",
    "local_branch",
    "remote_branch",
    "remote_name",
    "remote_url",
    "action",
    "tag",
    "push_remote_name",
    "push_remote_url",
    "encoding",
    "summary",
];

/// Answers once for the folder `rest` names, `.` without one, as facts or
/// as `git-engine.answer/1`.
fn once_for(context: &terminal_kit::Context, rest: &[String]) -> Result<(), Failure> {
    let given = rest
        .first()
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    let Ok(dir) = std::fs::canonicalize(&given) else {
        return Err(Failure::new(
            Exit::NotFound,
            format!("there's no folder {}", given.display()),
        ));
    };
    let options = options(&rest[1.min(rest.len())..]);
    let started = Instant::now();
    let answer = Engine::default().ask(&dir, &options, true);
    let took = started.elapsed();
    let Some(answer) = answer else {
        return Err(Failure::new(
            Exit::NotFound,
            format!("{} is in no repository", dir.display()),
        ));
    };
    let said: Vec<(&str, String)> = NAMES.into_iter().zip(fields(&answer)).collect();
    let micros = u64::try_from(took.as_micros()).unwrap_or(u64::MAX);
    let written = if context.format().is_json() {
        let mut facts = serde_json::Map::new();
        for (name, value) in &said {
            let value = if WORDS.contains(name) {
                serde_json::Value::from(value.as_str())
            } else {
                // A count of -1 is one the index wasn't read for.
                value
                    .parse::<u64>()
                    .map_or(serde_json::Value::Null, serde_json::Value::from)
            };
            facts.insert((*name).to_owned(), value);
        }
        context.answer(&serde_json::json!({
            "schema": "git-engine.answer/1",
            "fields": facts,
            "took_us": micros,
        }))
    } else {
        let labels: Vec<String> = said
            .iter()
            .map(|(name, _)| name.replace('_', " "))
            .collect();
        let took = format!("{} ms", micros.div_ceil(1000));
        let facts: Vec<(&str, Option<&str>)> = labels
            .iter()
            .zip(&said)
            .map(|(label, (_, value))| {
                let value = match value.as_str() {
                    "" => None,
                    "-1" => Some("not counted"),
                    value => Some(value),
                };
                (label.as_str(), value)
            })
            .chain(std::iter::once(("took", Some(took.as_str()))))
            .collect();
        let lines = terminal_kit::layout::facts(&facts, "none", &context.out, context.room);
        context.say(&(lines.join("\n") + "\n"))
    };
    written
        .map_err(|error| Failure::new(Exit::Failed, "it couldn't write").because(error.to_string()))
}

/// Each field of an answer, by the name gitstatusd's protocol gives it.
const NAMES: [&str; 27] = [
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A repository with no commits yet, made afresh under `name`.
    fn repository(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("git-engine-main-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".git/objects/pack")).unwrap();
        std::fs::create_dir_all(dir.join(".git/refs/heads")).unwrap();
        std::fs::write(dir.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        dir
    }

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn takes_every_flag_gitstatus_gives_it() {
        let given = words("-s 1 -u -1 -c 1 -d 1 -m -1 -e -U -W -D -t 32 -v FATAL");
        assert_eq!(stray(&given), None, "gitstatus's own flags were refused");
    }

    #[test]
    fn refuses_a_bare_word() {
        assert_eq!(
            stray(&words("-s 1 -U here")),
            Some("here"),
            "a word no flag takes went unseen"
        );
    }

    #[test]
    fn lets_go_of_a_repository_whose_packs_changed() {
        let dir = repository("repacked");
        let mut engine = Engine::default();
        engine.ask(&dir, &options(&[]), false);
        assert_eq!(engine.repos.len(), 1);
        // What a repack does: writes a pack and removes the old ones.
        std::fs::write(dir.join(".git/objects/pack/pack-new.idx"), b"").unwrap();
        engine.let_go_of_changed();
        assert!(engine.repos.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn keeps_a_repository_nothing_changed_in() {
        let dir = repository("still");
        // Asked once its files have settled, since one just made may yet
        // change within the tick it was made in.
        crate::stamp::tests::ahead(3);
        let mut engine = Engine::default();
        engine.ask(&dir, &options(&[]), false);
        engine.let_go_of_changed();
        assert_eq!(engine.repos.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
