//! Main on a host that doesn't sign it. Root asks the host itself over TLS
//! which commit main is, with a token the account gives, so the answer is
//! the host's and a bad token is only refused. It reads that commit's
//! objects from the account's checkout through git's alternates, never its
//! configuration, and git checks them against the commit the host named.
//! It unpacks the flake's folder from them and builds it, after the account
//! has fetched the inputs only it can reach, each of which the store holds
//! to the hash its lock names.
//!
//! Where the host has a REST API, root reads the folder at main's head from
//! it instead, tree by tree, each checked against the id above it
//! (src/read.rs): a fetch into a large repository's checkout spends minutes
//! checking the new head's whole tree is there. When the API fails, main is
//! fetched as before.
//!
//! A fetch into a large repository's checkout takes minutes and gigabytes,
//! so the account may say when main may be fetched at all. Outside those
//! times, a head neither its checkout holds nor the API could read is left
//! for a later ask: the machine takes a commit of main the checkout
//! already holds, when the host says main holds it, or stays where it is,
//! with only its followed inputs moved on.
//!
//! In a repository whose main moves far more often than the flake's folder,
//! the account may also name the commit that last changed the folder. When
//! that commit's folder is the very tree last built, nothing is fetched or
//! built. That answer is the account's, not the host's, so it only ever
//! spares a fetch: what's built is still a commit the host named, and an
//! account that lies can at most hold the machine where it is, as one that
//! gives no token can.

use crate::config::{Config, Hosted};
use crate::git::{self, Repo};
use crate::read;
use crate::run::{self, Account, Failure};
use crate::session;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

/// The variable the locked input is given to Nix in, rather than in its
/// arguments.
const LOCKED: &str = "UPDATE_TO_MAIN_LOCKED";

/// Fetches a locked input into the store, by its lock.
const FETCH: &str =
    "(builtins.fetchTree (builtins.fromJSON (builtins.getEnv \"UPDATE_TO_MAIN_LOCKED\"))).outPath";

/// Asks the host for main's head, and makes sure root's copy can read it
/// from the account's checkout, fetching it there as the account when it
/// isn't there yet. When the folder is where it was at `last`, the commit
/// the machine last switched to, that's the head instead, unfetched.
pub fn head(config: &Config, hosted: &Hosted, last: Option<&str>) -> Result<String, Failure> {
    let account = run::account(&hosted.user)?;
    let main = Repo::main(config);
    git::make_copy(&main)?;
    let token = token(hosted, &account)?;
    let head = ask(config, &token)?;
    borrow(config, hosted, &account)?;
    if !git::has(&main, &head) {
        // Asked only once main must come from the checkout, so a day the
        // API answers runs nothing of the account's.
        let mut fetchable = None;
        if let Some(api) = &hosted.api {
            match read::read(config, hosted, api, &token, &head, last) {
                // The folder as it was last built is that build's, unbuilt.
                Ok(found) => match last {
                    Some(last) if folder(config, hosted, last).as_ref() == Some(&found.folder) => {
                        return Ok(last.to_owned());
                    }
                    _ => return Ok(head),
                },
                Err(failure) => {
                    let may = may_fetch(hosted, &account);
                    let then = if may {
                        "so main is fetched instead"
                    } else {
                        "and main mayn't be fetched now"
                    };
                    eprintln!("{}", fell_back(&failure, then));
                    fetchable = Some(may);
                }
            }
        }
        if let Some(last) = last
            && unmoved(config, hosted, &account, last)
        {
            return Ok(last.to_owned());
        }
        if !fetchable.unwrap_or_else(|| may_fetch(hosted, &account)) {
            return unfetched(config, hosted, &account, &head, last);
        }
        fetch(config, hosted, &account)?;
        if !git::has(&main, &head) {
            return Err(Failure::new(format!(
                "{} isn't in {} even after fetching main there",
                git::short(&head),
                hosted.checkout.display()
            )));
        }
    }
    Ok(head)
}

/// Says why main isn't read from the host, with what the read said, such
/// as which answer curl failed on and its HTTP status, on one line, then
/// what follows: `then`, a fetch or none. On 11 and 12 November a read
/// ended with curl's exit 22 and nothing more, and the ten-minute fetch
/// that followed said no cause.
fn fell_back(failure: &Failure, then: &str) -> String {
    let mut why = failure.why.clone();
    if !failure.lines.is_empty() {
        why = format!("{why}: {}", failure.lines.join("; "));
    }
    format!("update-to-main: {why}, {then}")
}

/// Whether main may be fetched into the checkout now, by the account's
/// command: always, when it names none.
fn may_fetch(hosted: &Hosted, account: &Account) -> bool {
    hosted.may_fetch.as_ref().is_none_or(|command| {
        said(
            hosted,
            account,
            command,
            "asking whether main may be fetched now",
        )
        .is_ok()
    })
}

/// What the machine takes while main at `head` can't be read and mayn't be
/// fetched: the commit the account says its checkout holds, when root's
/// copy can read it, it descends from `last`, the host says main at `head`
/// holds it and its folder has moved; otherwise `last`, so only the
/// followed inputs move. A machine that has never switched has nothing to
/// stay on, so its ask fails.
fn unfetched(
    config: &Config,
    hosted: &Hosted,
    account: &Account,
    head: &str,
    last: Option<&str>,
) -> Result<String, Failure> {
    let Some(last) = last else {
        return Err(Failure::new(format!(
            "main at {} isn't in {}, and mayn't be fetched now",
            git::short(head),
            hosted.checkout.display()
        )));
    };
    let main = Repo::main(config);
    let taken = hosted
        .fetched
        .as_ref()
        .and_then(|command| {
            said(
                hosted,
                account,
                command,
                "asking what main the checkout holds",
            )
            .ok()
        })
        .filter(|commit| is_commit(commit) && commit != last && git::has(&main, commit))
        .filter(|commit| git::descends(&main, last, commit).unwrap_or(false))
        .filter(|commit| holds(config, hosted, commit, head).unwrap_or(false))
        .filter(|commit| folder(config, hosted, commit) != folder(config, hosted, last));
    if let Some(commit) = taken {
        eprintln!(
            "update-to-main: so it takes main at {}, which {} holds",
            git::short(&commit),
            hosted.checkout.display()
        );
        Ok(commit)
    } else {
        eprintln!(
            "update-to-main: so main stays at {} until a later ask",
            git::short(last)
        );
        Ok(last.to_owned())
    }
}

/// Unpacks the flake's folder at `commit`, fetches its private inputs as
/// the account, and gives the flake reference to build.
pub fn prepare(config: &Config, hosted: &Hosted, commit: &str) -> Result<String, Failure> {
    let account = run::account(&hosted.user)?;
    let folder = unpack(config, hosted, commit)?;
    if !hosted.inputs.is_empty() {
        let lock = folder.join("flake.lock");
        let text = fs::read_to_string(&lock)
            .map_err(|e| Failure::new(format!("{}: {e}", lock.display())))?;
        let lock: Value = serde_json::from_str(&text)
            .map_err(|e| Failure::new(format!("reading the flake's lock: {e}")))?;
        for name in &hosted.inputs {
            fetch_input(config, hosted, &account, name, locked(&lock, name)?)?;
        }
    }
    Ok(format!("path:{}#{}", folder.display(), config.output))
}

/// The folder's tree at `commit`, from root's copy, or from what a read of
/// it from the host found; none when neither has it.
pub fn folder(config: &Config, hosted: &Hosted, commit: &str) -> Option<String> {
    git::tree(&Repo::main(config), commit, &hosted.directory)
        .or_else(|| read::found(config, commit).map(|found| found.folder))
}

/// Whether `newer` descends from `older`: by root's copy when it holds
/// `newer`, and otherwise by the host's word when the folder was read from
/// it with `older` the commit last switched to.
pub fn descends(config: &Config, older: &str, newer: &str) -> Result<bool, Failure> {
    // A commit descends from itself, as git's merge-base says: main where it
    // was, with a followed input moved on, is that.
    if older == newer {
        return Ok(true);
    }
    let main = Repo::main(config);
    if git::has(&main, newer) {
        return git::descends(&main, older, newer);
    }
    match read::found(config, newer) {
        Some(found) if found.from.as_deref() == Some(older) => Ok(found.descends),
        _ => Ok(false),
    }
}

/// Whether main at `head` holds `commit`: by root's copy when it has
/// `head`, whose history it then has whole, and otherwise by the host's
/// word.
pub fn holds(config: &Config, hosted: &Hosted, commit: &str, head: &str) -> Result<bool, Failure> {
    let main = Repo::main(config);
    if git::has(&main, head) {
        return git::descends(&main, commit, head);
    }
    let Some(api) = &hosted.api else {
        return Err(Failure::new(format!(
            "root's copy hasn't got {}, and no host's API is configured to ask",
            git::short(head)
        )));
    };
    let account = run::account(&hosted.user)?;
    let token = token(hosted, &account)?;
    read::descends(config, api, &token, commit, head)
}

/// The file at `path` in the folder at `commit`, or none.
pub fn show(config: &Config, hosted: &Hosted, commit: &str, path: &str) -> Option<String> {
    let tree = folder(config, hosted, commit)?;
    git::show(&Repo::main(config), &tree, path)
}

fn token(hosted: &Hosted, account: &Account) -> Result<String, Failure> {
    let token = said(hosted, account, &hosted.token, "asking for a token")?;
    if token.is_empty() {
        return Err(Failure::new("asking for a token gave none"));
    }
    Ok(token)
}

/// Whether the flake's folder is the tree it was at `last`, by the commit
/// the account says last changed it on main. Anything short of that commit
/// being in the checkout with that very tree, a command that fails or one
/// that names something else, counts as moved, so main is fetched.
fn unmoved(config: &Config, hosted: &Hosted, account: &Account, last: &str) -> bool {
    let Some(command) = &hosted.changed else {
        return false;
    };
    let Ok(changed) = said(
        hosted,
        account,
        command,
        "asking what last changed the folder",
    ) else {
        return false;
    };
    if !is_commit(&changed) {
        return false;
    }
    let main = Repo::main(config);
    match (
        git::tree(&main, &changed, &hosted.directory),
        git::tree(&main, last, &hosted.directory),
    ) {
        (Some(changed), Some(last)) => changed == last,
        _ => false,
    }
}

/// The last line a command run as the account in its checkout prints.
fn said(
    hosted: &Hosted,
    account: &Account,
    command: &[String],
    what: &str,
) -> Result<String, Failure> {
    let (program, args) = command
        .split_first()
        .ok_or_else(|| Failure::new(format!("no command for {what}")))?;
    let mut command = run::command_as(program.as_ref(), account, &hosted.path, &hosted.checkout);
    command.args(args);
    let said = run::output(command, what, std::iter::empty::<&str>())?;
    Ok(said.lines().last().unwrap_or_default().trim().to_owned())
}

/// Whether `name` is a whole commit id, SHA-1's or SHA-256's, rather than
/// anything git would read as a name or an option.
pub fn is_commit(name: &str) -> bool {
    matches!(name.len(), 40 | 64) && name.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Main's head as the host names it. The token goes to git in its
/// environment, which only root reads, rather than in its arguments.
fn ask(config: &Config, token: &str) -> Result<String, Failure> {
    let mut command = git::git(&Repo::main(config));
    command
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "http.extraHeader")
        .env(
            "GIT_CONFIG_VALUE_0",
            format!("Authorization: Bearer {token}"),
        );
    let main = format!("refs/heads/{}", config.branch);
    let said = run::output(
        command,
        "asking the host for main",
        ["ls-remote", "--refs", &config.repository, &main],
    )?;
    said.lines()
        .find_map(|line| {
            let (commit, name) = line.split_once('\t')?;
            (name == main && is_commit(commit)).then(|| commit.to_owned())
        })
        .ok_or_else(|| Failure::new(format!("the host names no {main}")))
}

/// Points root's copy at the checkout's objects, as the account finds them,
/// with the account's own git: a checkout may need a git of its own to be
/// read at all, as one marked with a repository extension the stock git
/// doesn't know is. Its objects are ordinary, so root's git reads them.
fn borrow(config: &Config, hosted: &Hosted, account: &Account) -> Result<(), Failure> {
    let mut command = run::command_as("git".as_ref(), account, &hosted.path, &hosted.checkout);
    command.env("GIT_CONFIG_NOSYSTEM", "1");
    let common = run::output(
        command,
        "finding the checkout's objects",
        ["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    let objects = Path::new(&common).join("objects");
    if !objects.is_absolute() || !objects.is_dir() {
        return Err(Failure::new(format!(
            "{} holds no objects",
            hosted.checkout.display()
        )));
    }
    let alternates = config.repository_copy().join("objects/info/alternates");
    fs::write(&alternates, format!("{}\n", objects.display()))
        .map_err(|e| Failure::new(format!("{}: {e}", alternates.display())))
}

/// Fetches main into the checkout as the account, with its own git and
/// logins, moving none of its branches. Main lands in a ref of the ask's
/// own, as the last ask's main is there: a git that checks only what a
/// fetch adds to the refs it moves then walks back to that, not through
/// every ref, which in a large repository took 2 minutes rather than 10 s.
fn fetch(config: &Config, hosted: &Hosted, account: &Account) -> Result<(), Failure> {
    let command = run::command_as("git".as_ref(), account, &hosted.path, &hosted.checkout);
    run::output(
        command,
        "fetching main into the checkout",
        [
            "fetch",
            "--quiet",
            "--no-tags",
            &config.repository,
            &format!("+refs/heads/{0}:refs/update-to-main/{0}", config.branch),
        ],
    )?;
    Ok(())
}

/// Unpacks the flake's folder at `commit` into a folder of its own, the
/// last one's taken away first. It archives the folder's own tree rather
/// than the commit with the folder as a pathspec: git reads a commit's
/// whole tree into an index first, for its attributes, which in a
/// large repository took 5 minutes and 4.9 GB for a folder of a few hundred
/// files.
fn unpack(config: &Config, hosted: &Hosted, commit: &str) -> Result<PathBuf, Failure> {
    let source = config.source();
    let _ = fs::remove_dir_all(&source);
    let into = source.join(commit);
    let folder = into.join(&hosted.directory);
    fs::create_dir_all(&folder).map_err(|e| Failure::new(format!("{}: {e}", folder.display())))?;
    let tree = self::folder(config, hosted, commit).ok_or_else(|| {
        Failure::new(format!(
            "root's copy has no folder for {}",
            git::short(commit)
        ))
    })?;
    let mut archive = git::git(&Repo::main(config));
    archive.args(["archive", "--format=tar", &tree]);
    let mut archive = archive
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Failure::new(format!("unpacking main: {e}")))?;
    let tar = archive
        .stdout
        .take()
        .ok_or_else(|| Failure::new("unpacking main: git gave nothing"))?;
    let mut unpack = run::command(&config.tools.tar, &config.state);
    unpack.arg("-x").arg("-C").arg(&folder).stdin(tar);
    let unpacked = run::output(unpack, "unpacking main", std::iter::empty::<&str>());
    let archived = archive
        .wait_with_output()
        .map_err(|e| Failure::new(format!("unpacking main: {e}")))?;
    if !archived.status.success() {
        return Err(Failure {
            why: format!("reading main's files failed ({})", archived.status),
            lines: run::tail(&String::from_utf8_lossy(&archived.stderr)),
        });
    }
    unpacked?;
    if !folder.join("flake.nix").is_file() {
        return Err(Failure::new(format!(
            "main at {} has no flake in {}",
            git::short(commit),
            if hosted.directory.is_empty() {
                "its top"
            } else {
                &hosted.directory
            }
        )));
    }
    Ok(folder)
}

/// An input's lock, by its name among the flake's own inputs.
pub fn locked<'a>(lock: &'a Value, name: &str) -> Result<&'a Value, Failure> {
    let missing = || Failure::new(format!("the flake's lock has no input {name}"));
    let root = lock["root"].as_str().ok_or_else(missing)?;
    let node = lock["nodes"][root]["inputs"][name]
        .as_str()
        .ok_or_else(missing)?;
    let locked = &lock["nodes"][node]["locked"];
    if locked.is_object() {
        Ok(locked)
    } else {
        Err(missing())
    }
}

/// Puts an input in the store as the account, by its lock: a path already
/// in the store is substituted, and anything else fetched to its hash.
fn fetch_input(
    config: &Config,
    hosted: &Hosted,
    account: &Account,
    name: &str,
    locked: &Value,
) -> Result<(), Failure> {
    let nix = config.tools.nix.as_os_str();
    let mut command = run::command_as(nix, account, &hosted.path, &hosted.checkout);
    let what = format!("fetching the input {name}");
    // An input's logins may be kept where only the account's login session
    // reaches, as a Mac's keychain is; the token and main's own fetch need
    // none of those, so they run as they are.
    let session = config.session.as_ref();
    let in_store = locked["type"] == "path"
        && locked["path"]
            .as_str()
            .is_some_and(|p| p.starts_with("/nix/store/"));
    if in_store {
        let path = locked["path"].as_str().unwrap_or_default();
        session::output(
            session,
            account,
            command,
            &what,
            [
                "--extra-experimental-features",
                "nix-command",
                "build",
                "--no-link",
                path,
            ],
        )?;
        return Ok(());
    }
    command.env(LOCKED, locked.to_string());
    session::output(
        session,
        account,
        command,
        &what,
        [
            "--extra-experimental-features",
            "nix-command flakes",
            "eval",
            "--raw",
            "--impure",
            "--expr",
            FETCH,
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lock() -> Value {
        serde_json::json!({
            "root": "root",
            "nodes": {
                "root": { "inputs": { "private": "private_2", "followed": ["a", "b"] } },
                "private_2": { "locked": { "type": "git", "url": "https://example.com/p", "rev": "abc" } },
            },
            "version": 7,
        })
    }

    #[test]
    fn an_input_is_found_by_the_name_the_flake_gives_it() {
        let lock = lock();
        assert_eq!(
            locked(&lock, "private").unwrap()["url"],
            "https://example.com/p"
        );
    }

    #[test]
    fn an_input_that_follows_another_or_isnt_there_isnt_found() {
        let lock = lock();
        assert!(locked(&lock, "followed").is_err());
        assert!(locked(&lock, "nowhere").is_err());
    }

    #[test]
    fn only_a_whole_commit_id_is_a_commit() {
        assert!(is_commit(&"a".repeat(40)));
        assert!(is_commit(&"0".repeat(64)));
        assert!(!is_commit("main"));
        assert!(!is_commit(&format!("--{}", "a".repeat(38))));
        assert!(!is_commit(&"a".repeat(41)));
    }

    #[test]
    fn a_read_that_failed_says_what_curl_said() {
        let failure = Failure {
            why: "reading main's folder from the host failed (exit status: 22)".to_owned(),
            lines: vec![
                "https://host/api/git/trees/abc answered 404".to_owned(),
                "curl: (22) The requested URL returned error: 404".to_owned(),
            ],
        };
        assert_eq!(
            fell_back(&failure, "so main is fetched instead"),
            "update-to-main: reading main's folder from the host failed (exit status: 22): \
             https://host/api/git/trees/abc answered 404; \
             curl: (22) The requested URL returned error: 404, so main is fetched instead",
            "the fallback should say what curl said, on one line"
        );
        assert_eq!(
            fell_back(
                &Failure::new("the host gave no answer"),
                "so main is fetched instead"
            ),
            "update-to-main: the host gave no answer, so main is fetched instead",
            "a failure with nothing said should read as before"
        );
        assert_eq!(
            fell_back(&failure, "and main mayn't be fetched now"),
            "update-to-main: reading main's folder from the host failed (exit status: 22): \
             https://host/api/git/trees/abc answered 404; \
             curl: (22) The requested URL returned error: 404, and main mayn't be fetched now",
            "a read that isn't followed by a fetch should say what curl said too"
        );
    }

    #[test]
    fn the_fetch_reads_the_variable_the_lock_is_given_in() {
        assert!(FETCH.contains(LOCKED));
    }
}
