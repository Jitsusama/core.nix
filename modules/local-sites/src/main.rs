//! local-sites --config FILE
//!
//! Joel's local sites, served on loopback alone, and each live page fed
//! by the daemon that holds its data. The config (config.rs), written by
//! the module, names where it listens and by which names, the folder of
//! documents sessions write, the folder the home is written into, the
//! folders Nix built that it serves at their own paths, and each feed: a
//! daemon's socket and the kinds and fields a page may read of it.
//!
//! A page reads a feed at `/feed/<name>`, a WebSocket over which it speaks
//! the daemon interface every daemon here speaks (docs/daemons.md), so a
//! page asks what the launcher asks and is told what it's told. The server
//! is no relay: it holds one watch on the daemon for each kind any page
//! reads and tells every page from it (upstream.rs), it lets a page read
//! and never act (gate.rs), and it admits only its own pages, by the name
//! a request reaches it by and the page a feed's handshake came from
//! (door.rs). An answer Joel presses on one of the sites' own pages is
//! the one thing a page sends that acts, checked and carried by answer.rs,
//! against times its looks write as clock.rs reads them.
//! A document a session wrote runs sandboxed, in an origin of
//! no one's, so it reads no feed and can't act as one of the sites' own
//! pages, which run only the scripts served beside them (http.rs). Files are
//! found by files.rs; one connection is http.rs.

mod answer;
mod clock;
mod config;
mod door;
mod files;
mod gate;
mod http;
mod page;
mod peer;
mod upstream;

use std::net::TcpListener;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::thread;

use terminal_kit::failure::{Exit, Failure};
use terminal_kit::flags;
use terminal_kit::help::Help;

use crate::answer::Answers;
use crate::config::Config;
use crate::door::Door;
use crate::http::Server;
use crate::peer::Processes;
use crate::upstream::Upstreams;

fn help() -> Help {
    Help::new(
        "local-sites",
        "Serves Joel's local sites, fed live by their daemons",
    )
    .example(
        "local-sites --config ~/.config/local-sites/local-sites.json",
        "serves what that config names",
    )
    .option(
        "--config FILE",
        "where it listens, what it serves and the feeds",
    )
    .exit(Exit::Failed, "it couldn't listen")
    .exit(Exit::Config, "the config couldn't be read")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        serve(&config_file(&context.args)?)
    })
}

/// The config file `args` name, as `--config FILE` or `--config=FILE`.
fn config_file(args: &[String]) -> Result<PathBuf, Failure> {
    let (file, rest) = match args {
        [] => {
            return Err(Failure::new(Exit::Usage, "local-sites needs its config")
                .try_this("local-sites --config FILE"));
        }
        [flag, rest @ ..] if flag == "--config" => match rest {
            [file, rest @ ..] if !file.starts_with("--") => (file.as_str(), rest),
            _ => return Err(missing()),
        },
        [joined, rest @ ..] if joined.starts_with("--config=") => {
            (&joined["--config=".len()..], rest)
        }
        [word, ..] => return Err(flags::unknown("flag", word, &["--config"])),
    };
    if file.is_empty() || file.starts_with("--") {
        return Err(missing());
    }
    if let [stray, ..] = rest {
        return Err(Failure::new(
            Exit::Usage,
            format!("local-sites takes nothing after its config, not {stray}"),
        )
        .try_this("local-sites --config FILE"));
    }
    Ok(PathBuf::from(file))
}

/// What `--config` without a file says.
fn missing() -> Failure {
    Failure::new(Exit::Usage, "--config names a file").try_this("local-sites --config FILE")
}

fn serve(file: &PathBuf) -> Result<(), Failure> {
    let unreadable = |why: String| {
        Failure::new(
            Exit::Config,
            format!("local-sites can't read {}", file.display()),
        )
        .because(why)
    };
    let text = std::fs::read_to_string(file).map_err(|why| unreadable(why.to_string()))?;
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
    let config = Config::from_json(&text, runtime.as_deref()).map_err(unreadable)?;
    let listener = TcpListener::bind(config.listen).map_err(|why| {
        Failure::new(
            Exit::Failed,
            format!("local-sites can't listen on {}", config.listen),
        )
        .because(why.to_string())
    })?;
    // Only a kernel that tells who holds a connection lets an answer be
    // known as the person's (peer.rs); elsewhere the pages answer in For You.
    let processes = Processes::here();
    if config.answers.is_some() && processes.is_none() {
        eprintln!("local-sites: takes no answers here, since it can't tell who presses one");
    }
    let answers = config
        .answers
        .as_ref()
        .zip(processes)
        .map(|(at, processes)| {
            let tell = Box::new(move |peer, server| processes.tell(peer, server));
            Answers::new(at.looks.clone(), at.node.clone(), tell)
        })
        .transpose()
        .map_err(|why| {
            Failure::new(Exit::Failed, "local-sites can't draw its answers' cookie")
                .because(why.to_string())
        })?;
    let server = Arc::new(Server {
        door: Door::new(&config.names, config.listen.port()),
        config,
        upstreams: Arc::new(Upstreams::default()),
        answers,
    });
    for stream in listener.incoming().flatten() {
        let server = Arc::clone(&server);
        thread::spawn(move || http::answer(&server, stream));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What `config_file` makes of `words`: the file, or the exit code.
    fn read(words: &[&str]) -> Result<PathBuf, u8> {
        let args: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
        config_file(&args).map_err(|failure| failure.code())
    }

    #[test]
    fn the_config_is_named_apart_or_joined() {
        assert_eq!(read(&["--config", "a.json"]), Ok(PathBuf::from("a.json")));
        assert_eq!(read(&["--config=a.json"]), Ok(PathBuf::from("a.json")));
    }

    #[test]
    fn a_config_named_wrongly_is_asked_wrongly() {
        for words in [
            &[][..],
            &["--config"],
            &["--config="],
            &["--config", "--verbose"],
            &["--config=--verbose"],
            &["--config", "a.json", "stray"],
            &["--confg", "a.json"],
        ] {
            assert_eq!(
                read(words),
                Err(64),
                "{words:?} should be refused as asked wrongly"
            );
        }
    }

    #[test]
    fn a_stray_word_is_named_as_one() {
        let args = ["--config", "a.json", "stray"].map(str::to_owned);
        let said = config_file(&args)
            .map(|_| ())
            .map_err(|failure| failure.json(false).to_string());
        let said = said.expect_err("a stray word is refused");
        assert!(
            said.contains("not stray"),
            "the refusal names the stray word: {said}"
        );
    }
}
