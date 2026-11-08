//! The socket: a caller writes `token` and a newline, and reads back one
//! line, `ok <token> <when it lapses>` or `no <why>`.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::FromRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::time::Duration;

use crate::mint::{Minter, Token};

/// A caller has this long to ask and to read the answer, so one that
/// stalls can't hold up the next.
const PATIENCE: Duration = Duration::from_secs(5);

/// The socket systemd handed over (`LISTEN_FDS`, at descriptor 3) when it
/// started this for a caller, else a new one at `socket`.
pub fn listener(socket: &Path) -> Result<UnixListener, String> {
    let ours = std::env::var("LISTEN_PID").ok() == Some(std::process::id().to_string());
    if ours && std::env::var("LISTEN_FDS").ok().as_deref() == Some("1") {
        // SAFETY: systemd passes exactly one listening socket at descriptor
        // 3 when LISTEN_FDS is 1 and LISTEN_PID names this process, and
        // nothing else in this process owns it.
        return Ok(unsafe { UnixListener::from_raw_fd(3) });
    }
    let _ = std::fs::remove_file(socket);
    UnixListener::bind(socket).map_err(|why| format!("can't listen at {}: {why}", socket.display()))
}

/// Answers each caller in turn, for as long as the socket lasts.
pub fn serve(listener: &UnixListener, minter: &mut Minter) {
    for caller in listener.incoming().flatten() {
        if let Err(why) = answer(&caller, minter) {
            eprintln!("github-broker: a caller went before its answer: {why}");
        }
    }
}

fn answer(caller: &UnixStream, minter: &mut Minter) -> std::io::Result<()> {
    caller.set_read_timeout(Some(PATIENCE))?;
    caller.set_write_timeout(Some(PATIENCE))?;
    let mut asked = String::new();
    BufReader::new(caller.take(64)).read_line(&mut asked)?;
    let said = match asked.trim_end() {
        "token" => match minter.token() {
            Ok(token) => format!("ok {} {}\n", token.value, token.expires_at),
            Err(why) => {
                eprintln!("github-broker: {why}");
                format!("no {}\n", why.replace('\n', " "))
            }
        },
        other => format!("no {other:?} isn't something to ask; ask `token`\n"),
    };
    let mut caller = caller;
    caller.write_all(said.as_bytes())
}

/// Asks the broker at `socket` for a token.
pub fn ask(socket: &Path) -> Result<Token, String> {
    let caller = UnixStream::connect(socket)
        .map_err(|why| format!("no broker answers at {}: {why}", socket.display()))?;
    caller
        .set_read_timeout(Some(PATIENCE * 8))
        .map_err(|why| why.to_string())?;
    (&caller)
        .write_all(b"token\n")
        .map_err(|why| format!("the broker hung up: {why}"))?;
    let mut said = String::new();
    BufReader::new(&caller)
        .read_line(&mut said)
        .map_err(|why| format!("the broker didn't answer: {why}"))?;
    let said = said.trim_end();
    match said.split_once(' ') {
        Some(("ok", rest)) => match rest.split_once(' ') {
            Some((value, expires_at)) => Ok(Token {
                value: value.to_owned(),
                expires_at: expires_at.to_owned(),
            }),
            None => Err(format!("the broker answered half a token: {rest}")),
        },
        Some(("no", why)) => Err(why.to_owned()),
        _ => Err(format!("the broker answered something else: {said:?}")),
    }
}
