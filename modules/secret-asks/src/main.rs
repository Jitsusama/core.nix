//! secret-asks: the release line's first experiment, one throwaway item
//! released to one fixed checker on Joel's yes in the launcher.
//!
//! - `secret-asks serve --socket SOCKET --asks DIR --vocabulary WORDS
//!   --item REF --op OP --git GIT --loginctl LOGINCTL [--account ACCOUNT]
//!   [--journal SOCKET]` serves each ask in DIR as a row in Asks, and
//!   answers release and refuse (src/daemon.rs says how), telling the
//!   journal at SOCKET, journald's own by default, how each ended.
//! - `secret-asks ask --item REF --tree PATH --why WORDS [--asks DIR]
//!   [--given NAME] [--session NAME] [--quest-id ID --quest-title TITLE]`
//!   puts up one ask for the checker, as the harness will for an agent.
//! - `secret-asks received` is the checker: it says how many characters
//!   each variable it was given holds, and never what they hold.

use std::path::PathBuf;
use std::process::ExitCode;

use secret_asks::ask::Ask;
use secret_asks::daemon::{self, Config};
use secret_asks::reading;
use secret_asks::words::Words;

const USAGE: &str = "usage: secret-asks serve --socket SOCKET --asks DIR --vocabulary WORDS \
--item REF --op OP --git GIT --loginctl LOGINCTL [--account ACCOUNT] [--journal SOCKET]
       secret-asks ask --item REF --tree PATH --why WORDS [--asks DIR] [--given NAME] \
[--session NAME] [--quest-id ID --quest-title TITLE]
       secret-asks received";

/// The value of a flag, if given.
fn named(args: &[String], flag: &str) -> Option<String> {
    let at = args.iter().position(|one| one == flag)?;
    args.get(at + 1).cloned()
}

/// The asks folder in the runtime directory, where the daemon watches.
fn default_asks() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR").map(|runtime| PathBuf::from(runtime).join("secret-asks"))
}

fn serve(args: &[String]) -> ExitCode {
    let given = (|| {
        Some((
            named(args, "--socket")?,
            named(args, "--asks")?,
            named(args, "--vocabulary")?,
            named(args, "--item")?,
            named(args, "--op")?,
            named(args, "--git")?,
            named(args, "--loginctl")?,
        ))
    })();
    let Some((socket, asks, vocabulary, item, op, git, loginctl)) = given else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let read = std::fs::read_to_string(&vocabulary).map_err(|why| why.to_string());
    let words =
        match read.and_then(|text| Words::from_vocabulary(&text).map_err(|why| why.to_string())) {
            Ok(words) => words,
            Err(why) => {
                eprintln!("secret-asks: can't read the vocabulary {vocabulary}: {why}");
                return ExitCode::FAILURE;
            }
        };
    let checker = match std::env::current_exe() {
        Ok(path) => path,
        Err(why) => {
            eprintln!("secret-asks: can't say where it runs from: {why}");
            return ExitCode::FAILURE;
        }
    };
    daemon::run(Config {
        socket: socket.into(),
        asks: asks.into(),
        item,
        account: named(args, "--account"),
        op: op.into(),
        git: git.into(),
        loginctl: loginctl.into(),
        checker,
        home: std::env::var("HOME").unwrap_or_default(),
        words,
        journal: named(args, "--journal").map_or_else(|| reading::JOURNAL.into(), PathBuf::from),
    })
}

/// Sixteen random bytes as 32 lowercase hex characters.
fn fresh_id() -> std::io::Result<String> {
    use std::fmt::Write;
    use std::io::Read;
    let mut bytes = [0_u8; 16];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    }))
}

fn ask(args: &[String]) -> ExitCode {
    let (Some(item), Some(tree), Some(why), Some(asks)) = (
        named(args, "--item"),
        named(args, "--tree"),
        named(args, "--why"),
        named(args, "--asks")
            .map(PathBuf::from)
            .or_else(default_asks),
    ) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let quest = named(args, "--quest-id").zip(named(args, "--quest-title"));
    let tree = std::path::absolute(&tree).unwrap_or_else(|_| tree.into());
    let now_ms = daemon::now_ms();
    let made = fresh_id().map(|id| Ask {
        id,
        asked_ms: now_ms,
        session: named(args, "--session").unwrap_or_else(|| "you".to_owned()),
        quest,
        item,
        why,
        argv: daemon::CHECKER
            .iter()
            .map(|word| (*word).to_owned())
            .collect(),
        tree: tree.display().to_string(),
        given: named(args, "--given").unwrap_or_else(|| "RELEASED".to_owned()),
        minutes: 1,
    });
    let written = made.and_then(|ask| {
        daemon::write_whole(&asks.join(format!("{}.ask.json", ask.id)), &ask.said())?;
        Ok(ask.id)
    });
    match written {
        Ok(id) => {
            println!("{id}");
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!(
                "secret-asks: can't put the ask in {}: {why}",
                asks.display()
            );
            ExitCode::FAILURE
        }
    }
}

/// The checker: each variable but the few every command is given, by
/// name and length only.
fn received() -> ExitCode {
    let mut any = false;
    for (name, value) in std::env::vars_os() {
        if ["PATH", "HOME", "LANG"].iter().any(|kept| name == *kept) {
            continue;
        }
        any = true;
        let length = value.to_string_lossy().chars().count();
        println!("{}: received {length} characters", name.to_string_lossy());
    }
    if !any {
        println!("received nothing");
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("serve") => serve(&args[1..]),
        Some("ask") => ask(&args[1..]),
        Some("received") => received(),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
