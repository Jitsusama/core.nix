//! A token from the machine's GitHub broker, which holds the GitHub App's
//! key and mints tokens that can read and do nothing else. Its socket
//! takes `token` and a newline and answers one line, `ok <token> <when it
//! lapses>` or `no <why>` (github-broker's wire.rs). repos asks it alone,
//! rather than taking the broker's crate and the signing it carries.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

/// How long the broker has to answer: minting asks GitHub, which can take
/// a few seconds, and a refresh on a timer can wait for it.
const PATIENCE: Duration = Duration::from_secs(40);

/// A token from the broker at `socket`, or why there's none.
pub fn token(socket: &Path) -> Result<String, String> {
    let caller = UnixStream::connect(socket)
        .map_err(|why| format!("no GitHub broker answers at {}: {why}", socket.display()))?;
    caller
        .set_read_timeout(Some(PATIENCE))
        .map_err(|why| why.to_string())?;
    (&caller)
        .write_all(b"token\n")
        .map_err(|why| format!("the GitHub broker hung up: {why}"))?;
    let mut said = String::new();
    BufReader::new(&caller)
        .read_line(&mut said)
        .map_err(|why| format!("the GitHub broker didn't answer: {why}"))?;
    let said = said.trim_end();
    match said.split_once(' ') {
        Some(("ok", rest)) => match rest.split_once(' ') {
            Some((token, _lapses)) if !token.is_empty() => Ok(token.to_owned()),
            _ => Err(format!("the GitHub broker answered half a token: {rest}")),
        },
        Some(("no", why)) => Err(format!("the GitHub broker gave no token: {why}")),
        _ => Err(format!(
            "the GitHub broker answered something else: {said:?}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::os::unix::net::UnixListener;
    use std::path::PathBuf;
    use std::thread;

    use super::*;

    /// A stand-in broker answering one caller with `answer`, at a socket of
    /// the test's own, and what the caller asked.
    fn broker(name: &str, answer: &'static str) -> (PathBuf, thread::JoinHandle<String>) {
        let socket = std::env::temp_dir().join(format!("repos-{}-{name}.sock", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("the stand-in listens");
        let asked = thread::spawn(move || {
            let (mut caller, _) = listener.accept().expect("a caller");
            let mut asked = [0; 6];
            caller.read_exact(&mut asked).expect("the caller asks");
            caller.write_all(answer.as_bytes()).expect("it answers");
            String::from_utf8_lossy(&asked).into_owned()
        });
        (socket, asked)
    }

    #[test]
    fn the_broker_s_token_is_taken_without_when_it_lapses() {
        let (socket, asked) = broker("ok", "ok ghs_stand-in 2026-11-15T16:00:00Z\n");
        assert_eq!(token(&socket), Ok("ghs_stand-in".to_owned()));
        assert_eq!(asked.join().expect("the stand-in ends"), "token\n");
    }

    #[test]
    fn a_broker_refusing_says_why() {
        let (socket, _) = broker("no", "no the App isn't installed\n");
        assert_eq!(
            token(&socket),
            Err("the GitHub broker gave no token: the App isn't installed".to_owned())
        );
    }

    #[test]
    fn no_broker_at_the_socket_says_where_it_looked() {
        let socket = std::env::temp_dir().join(format!("repos-{}-none.sock", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let why = token(&socket).expect_err("nothing answers");
        assert!(
            why.starts_with(&format!("no GitHub broker answers at {}", socket.display())),
            "the failure didn't name the socket: {why}"
        );
    }
}
